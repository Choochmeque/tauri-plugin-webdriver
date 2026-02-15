use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tauri::{Manager, Runtime, WebviewWindow};

use crate::mobile::Webdriver;
use crate::platform::{
    wrap_script_for_frame_context, FrameId, PlatformExecutor, PointerEventType, PrintOptions,
    WindowRect,
};
use crate::server::response::WebDriverErrorResponse;
use crate::webdriver::Timeouts;

/// iOS WKWebView executor using Tauri's mobile plugin bridge
#[derive(Clone)]
pub struct IOSExecutor<R: Runtime> {
    window: WebviewWindow<R>,
    timeouts: Timeouts,
    frame_context: Vec<FrameId>,
}

impl<R: Runtime> IOSExecutor<R> {
    pub fn new(window: WebviewWindow<R>, timeouts: Timeouts, frame_context: Vec<FrameId>) -> Self {
        Self {
            window,
            timeouts,
            frame_context,
        }
    }
}

// =============================================================================
// Plugin Method Arguments
// =============================================================================

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct EvaluateJsArgs {
    script: String,
    timeout_ms: u64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct AsyncScriptArgs {
    script: String,
    timeout_ms: u64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct TouchArgs {
    r#type: String,
    x: i32,
    y: i32,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct ScreenshotArgs {
    timeout_ms: u64,
}

// =============================================================================
// Plugin Method Responses
// =============================================================================

#[derive(Debug, Deserialize)]
struct JsResult {
    success: bool,
    value: Option<Value>,
    error: Option<String>,
}

#[derive(Debug, Deserialize)]
struct AlertResult {
    message: Option<String>,
    r#type: Option<String>,
    #[serde(rename = "defaultText")]
    default_text: Option<String>,
}

/// Register webview handlers on iOS (placeholder - no-op for now)
pub fn register_webview_handlers<R: Runtime>(_webview: &tauri::Webview<R>) {
    // On iOS, alert handling is done via the plugin's WKUIDelegate
    // which is set up during plugin initialization
    tracing::debug!("iOS webview handlers registered (via plugin)");
}

#[async_trait]
impl<R: Runtime + 'static> PlatformExecutor<R> for IOSExecutor<R> {
    fn window(&self) -> &WebviewWindow<R> {
        &self.window
    }

    async fn evaluate_js(&self, script: &str) -> Result<Value, WebDriverErrorResponse> {
        let wrapped_script = wrap_script_for_frame_context(script, &self.frame_context);

        let webdriver = self.window.app_handle().state::<Webdriver<R>>();

        let args = EvaluateJsArgs {
            script: wrapped_script,
            timeout_ms: self.timeouts.script_ms,
        };

        let result: JsResult = webdriver
            .0
            .run_mobile_plugin_async("evaluateJs", args)
            .await
            .map_err(|e| WebDriverErrorResponse::unknown_error(&e.to_string()))?;

        if result.success {
            // iOS returns the value directly (not JSON-encoded)
            let value = result.value.unwrap_or(Value::Null);

            Ok(serde_json::json!({
                "success": true,
                "value": value
            }))
        } else {
            Err(WebDriverErrorResponse::javascript_error(
                result.error.as_deref().unwrap_or("Unknown error"),
                None,
            ))
        }
    }

    async fn execute_async_script(
        &self,
        script: &str,
        args: &[Value],
    ) -> Result<Value, WebDriverErrorResponse> {
        let args_json = serde_json::to_string(args)
            .map_err(|e| WebDriverErrorResponse::invalid_argument(&e.to_string()))?;

        // Build wrapper that includes argument deserialization
        // Swift wraps this in a Promise and provides __done via callAsyncJavaScript
        let wrapper = format!(
            r"var ELEMENT_KEY = 'element-6066-11e4-a52e-4f735466cecf';
            function deserializeArg(arg) {{
                if (arg === null || arg === undefined) return arg;
                if (Array.isArray(arg)) return arg.map(deserializeArg);
                if (typeof arg === 'object') {{
                    if (arg[ELEMENT_KEY]) {{
                        var el = window['__wd_el_' + arg[ELEMENT_KEY].replace(/-/g, '')];
                        if (!el) throw new Error('stale element reference');
                        return el;
                    }}
                    var result = {{}};
                    for (var key in arg) {{
                        if (arg.hasOwnProperty(key)) result[key] = deserializeArg(arg[key]);
                    }}
                    return result;
                }}
                return arg;
            }}
            var __args = {args_json}.map(deserializeArg);
            __args.push(__done);
            (function() {{ {script} }}).apply(null, __args);"
        );

        let webdriver = self.window.app_handle().state::<Webdriver<R>>();

        let plugin_args = AsyncScriptArgs {
            script: wrapper,
            timeout_ms: self.timeouts.script_ms,
        };

        let result: JsResult = webdriver
            .0
            .run_mobile_plugin_async("executeAsyncScript", plugin_args)
            .await
            .map_err(|e| WebDriverErrorResponse::unknown_error(&e.to_string()))?;

        if result.success {
            // iOS returns the value directly (not JSON-encoded) via callAsyncJavaScript
            Ok(result.value.unwrap_or(Value::Null))
        } else {
            Err(WebDriverErrorResponse::javascript_error(
                result.error.as_deref().unwrap_or("Unknown error"),
                None,
            ))
        }
    }

    async fn take_screenshot(&self) -> Result<String, WebDriverErrorResponse> {
        let webdriver = self.window.app_handle().state::<Webdriver<R>>();

        let args = ScreenshotArgs {
            timeout_ms: self.timeouts.script_ms,
        };

        let result: JsResult = webdriver
            .0
            .run_mobile_plugin_async("takeScreenshot", args)
            .await
            .map_err(|e| WebDriverErrorResponse::unknown_error(&e.to_string()))?;

        if result.success {
            if let Some(Value::String(base64)) = result.value {
                Ok(base64)
            } else {
                Err(WebDriverErrorResponse::unknown_error(
                    "Screenshot returned invalid data",
                ))
            }
        } else {
            Err(WebDriverErrorResponse::unknown_error(
                result.error.as_deref().unwrap_or("Screenshot failed"),
            ))
        }
    }

    async fn take_element_screenshot(
        &self,
        js_var: &str,
    ) -> Result<String, WebDriverErrorResponse> {
        // Scroll element into view first
        let script = format!(
            r"(function() {{
                var el = window.{js_var};
                if (!el || !el.isConnected) {{
                    throw new Error('stale element reference');
                }}
                el.scrollIntoView({{ block: 'center', inline: 'center' }});
                return true;
            }})()"
        );
        self.evaluate_js(&script).await?;

        // Take full screenshot (element clipping can be added later)
        self.take_screenshot().await
    }

    async fn print_page(&self, options: PrintOptions) -> Result<String, WebDriverErrorResponse> {
        let webdriver = self.window.app_handle().state::<Webdriver<R>>();

        let result: JsResult = webdriver
            .0
            .run_mobile_plugin_async("printToPdf", options)
            .await
            .map_err(|e| WebDriverErrorResponse::unknown_error(&e.to_string()))?;

        if result.success {
            if let Some(Value::String(base64)) = result.value {
                Ok(base64)
            } else {
                Err(WebDriverErrorResponse::unknown_error(
                    "Print returned invalid data",
                ))
            }
        } else {
            Err(WebDriverErrorResponse::unknown_error(
                result.error.as_deref().unwrap_or("Print failed"),
            ))
        }
    }

    // Override pointer dispatch to use touch events on iOS
    async fn dispatch_pointer_event(
        &self,
        event_type: PointerEventType,
        x: i32,
        y: i32,
        _button: u32,
    ) -> Result<(), WebDriverErrorResponse> {
        let webdriver = self.window.app_handle().state::<Webdriver<R>>();

        let touch_type = match event_type {
            PointerEventType::Down => "down",
            PointerEventType::Up => "up",
            PointerEventType::Move => "move",
        };

        let args = TouchArgs {
            r#type: touch_type.to_string(),
            x,
            y,
        };

        let _result: Value = webdriver
            .0
            .run_mobile_plugin_async("dispatchTouch", args)
            .await
            .map_err(|e| WebDriverErrorResponse::unknown_error(&e.to_string()))?;

        Ok(())
    }

    // Alert handling via plugin
    async fn get_alert_text(&self) -> Result<String, WebDriverErrorResponse> {
        let webdriver = self.window.app_handle().state::<Webdriver<R>>();

        let result: AlertResult = webdriver
            .0
            .run_mobile_plugin_async("getAlertText", ())
            .await
            .map_err(|e| {
                if e.to_string().contains("no such alert") {
                    WebDriverErrorResponse::no_such_alert()
                } else {
                    WebDriverErrorResponse::unknown_error(&e.to_string())
                }
            })?;

        result
            .message
            .ok_or_else(WebDriverErrorResponse::no_such_alert)
    }

    async fn accept_alert(&self) -> Result<(), WebDriverErrorResponse> {
        let webdriver = self.window.app_handle().state::<Webdriver<R>>();

        let _result: Value = webdriver
            .0
            .run_mobile_plugin_async("acceptAlert", ())
            .await
            .map_err(|e| {
                if e.to_string().contains("no such alert") {
                    WebDriverErrorResponse::no_such_alert()
                } else {
                    WebDriverErrorResponse::unknown_error(&e.to_string())
                }
            })?;

        Ok(())
    }

    async fn dismiss_alert(&self) -> Result<(), WebDriverErrorResponse> {
        let webdriver = self.window.app_handle().state::<Webdriver<R>>();

        let _result: Value = webdriver
            .0
            .run_mobile_plugin_async("dismissAlert", ())
            .await
            .map_err(|e| {
                if e.to_string().contains("no such alert") {
                    WebDriverErrorResponse::no_such_alert()
                } else {
                    WebDriverErrorResponse::unknown_error(&e.to_string())
                }
            })?;

        Ok(())
    }

    async fn send_alert_text(&self, text: &str) -> Result<(), WebDriverErrorResponse> {
        let webdriver = self.window.app_handle().state::<Webdriver<R>>();

        #[derive(Serialize)]
        #[serde(rename_all = "camelCase")]
        struct SendAlertTextArgs {
            prompt_text: String,
        }

        let _result: Value = webdriver
            .0
            .run_mobile_plugin_async(
                "sendAlertText",
                SendAlertTextArgs {
                    prompt_text: text.to_string(),
                },
            )
            .await
            .map_err(|e| {
                let msg = e.to_string();
                if msg.contains("no such alert") {
                    WebDriverErrorResponse::no_such_alert()
                } else if msg.contains("not a prompt") {
                    WebDriverErrorResponse::element_not_interactable(
                        "User prompt is not a prompt dialog",
                    )
                } else {
                    WebDriverErrorResponse::unknown_error(&msg)
                }
            })?;

        Ok(())
    }

    // Cookies: Use default implementation from PlatformExecutor trait
    // (Tauri's window().cookies() APIs work on iOS)

    // =========================================================================
    // Window Management
    // =========================================================================

    async fn get_window_rect(&self) -> Result<WindowRect, WebDriverErrorResponse> {
        // Get viewport size from Swift plugin
        let webdriver = self.window.app_handle().state::<Webdriver<R>>();

        #[derive(Debug, Deserialize)]
        struct ViewportResult {
            width: u32,
            height: u32,
        }

        let result: ViewportResult = webdriver
            .0
            .run_mobile_plugin_async("getViewportSize", ())
            .await
            .map_err(|e| WebDriverErrorResponse::unknown_error(&e.to_string()))?;

        Ok(WindowRect {
            x: 0,
            y: 0,
            width: result.width,
            height: result.height,
        })
    }
}
