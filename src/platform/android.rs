use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tauri::{Manager, Runtime, WebviewWindow};

use crate::mobile::Webdriver;
use crate::platform::{
    wrap_script_for_frame_context, FrameId, ModifierState, PlatformExecutor, PointerEventType,
    PrintOptions, WindowRect,
};
use crate::server::response::WebDriverErrorResponse;
use crate::webdriver::Timeouts;

/// Android WebView executor using Tauri's mobile plugin bridge
#[derive(Clone)]
pub struct AndroidExecutor<R: Runtime> {
    window: WebviewWindow<R>,
    timeouts: Timeouts,
    frame_context: Vec<FrameId>,
}

impl<R: Runtime> AndroidExecutor<R> {
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
    async_id: String,
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

/// Register webview handlers on Android (placeholder - no-op for now)
pub fn register_webview_handlers<R: Runtime>(_webview: &tauri::Webview<R>) {
    // On Android, alert handling is done via the plugin's WebChromeClient
    // which is set up during plugin initialization
    tracing::debug!("Android webview handlers registered (via plugin)");
}

#[async_trait]
impl<R: Runtime + 'static> PlatformExecutor<R> for AndroidExecutor<R> {
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
            // Parse the stringified JSON value from Android
            let value = if let Some(value_str) = result.value {
                if let Some(s) = value_str.as_str() {
                    // Android returns JSON as a string, parse it
                    serde_json::from_str(s).unwrap_or(value_str)
                } else {
                    value_str
                }
            } else {
                Value::Null
            };

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

        let async_id = uuid::Uuid::new_v4().to_string();

        // Build wrapper that includes argument deserialization and callback
        let wrapper = format!(
            r"(function() {{
                var ELEMENT_KEY = 'element-6066-11e4-a52e-4f735466cecf';
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
                try {{
                    (function() {{ {script} }}).apply(null, __args);
                }} catch (e) {{
                    __done(null, e.message || String(e));
                }}
            }})()"
        );

        let webdriver = self.window.app_handle().state::<Webdriver<R>>();

        let plugin_args = AsyncScriptArgs {
            async_id,
            script: wrapper,
            timeout_ms: self.timeouts.script_ms,
        };

        let result: JsResult = webdriver
            .0
            .run_mobile_plugin_async("executeAsyncScript", plugin_args)
            .await
            .map_err(|e| WebDriverErrorResponse::unknown_error(&e.to_string()))?;

        if result.success {
            let value = if let Some(value_str) = result.value {
                if let Some(s) = value_str.as_str() {
                    serde_json::from_str(s).unwrap_or(value_str)
                } else {
                    value_str
                }
            } else {
                Value::Null
            };
            Ok(value)
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

    // Override pointer dispatch to use native touch on Android
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

    // Override keyboard dispatch to use JavaScript-based approach on Android
    // (native key injection on Android is more complex)
    async fn dispatch_key_event(
        &self,
        key: &str,
        is_down: bool,
        modifiers: &ModifierState,
    ) -> Result<(), WebDriverErrorResponse> {
        // Use the default JavaScript-based implementation from executor.rs
        // by calling evaluate_js with the appropriate keyboard event dispatch
        let (js_key, js_code, key_code) = match key {
            "\u{E007}" => ("Enter", "Enter", 13),
            "\u{E003}" => ("Backspace", "Backspace", 8),
            "\u{E004}" => ("Tab", "Tab", 9),
            "\u{E006}" => ("Enter", "NumpadEnter", 13),
            "\u{E00C}" => ("Escape", "Escape", 27),
            "\u{E00D}" => (" ", "Space", 32),
            "\u{E012}" => ("ArrowLeft", "ArrowLeft", 37),
            "\u{E013}" => ("ArrowUp", "ArrowUp", 38),
            "\u{E014}" => ("ArrowRight", "ArrowRight", 39),
            "\u{E015}" => ("ArrowDown", "ArrowDown", 40),
            "\u{E017}" => ("Delete", "Delete", 46),
            "\u{E008}" => ("Shift", "ShiftLeft", 16),
            "\u{E009}" => ("Control", "ControlLeft", 17),
            "\u{E00A}" => ("Alt", "AltLeft", 18),
            "\u{E03D}" => ("Meta", "MetaLeft", 91),
            _ => {
                let ch = key.chars().next().unwrap_or(' ');
                let upper = ch.to_ascii_uppercase();
                let code = if ch.is_ascii_alphabetic() {
                    format!("Key{upper}")
                } else if ch.is_ascii_digit() {
                    format!("Digit{ch}")
                } else {
                    key.to_string()
                };
                return self
                    .dispatch_regular_key(key, &code, is_down, modifiers)
                    .await;
            }
        };

        let event_type = if is_down { "keydown" } else { "keyup" };

        let script = format!(
            r"(function() {{
                var event = new KeyboardEvent('{event_type}', {{
                    key: '{js_key}',
                    code: '{js_code}',
                    keyCode: {key_code},
                    which: {key_code},
                    bubbles: true,
                    cancelable: true
                }});
                var activeEl = document.activeElement || document.body;
                activeEl.dispatchEvent(event);
                return true;
            }})()"
        );

        self.evaluate_js(&script).await?;
        Ok(())
    }

    async fn dispatch_regular_key(
        &self,
        key: &str,
        code: &str,
        is_down: bool,
        modifiers: &ModifierState,
    ) -> Result<(), WebDriverErrorResponse> {
        let ch = key.chars().next().unwrap_or(' ');
        let key_code = ch as u32;
        let event_type = if is_down { "keydown" } else { "keyup" };

        let escaped_key = key.replace('\\', "\\\\").replace('\'', "\\'");
        let escaped_code = code.replace('\\', "\\\\").replace('\'', "\\'");

        let ctrl_key = modifiers.ctrl;
        let meta_key = modifiers.meta;
        let shift_key = modifiers.shift;
        let alt_key = modifiers.alt;

        let script = if is_down {
            format!(
                r"(function() {{
                    var activeEl = document.activeElement || document.body;

                    var keydownEvent = new KeyboardEvent('keydown', {{
                        key: '{escaped_key}',
                        code: '{escaped_code}',
                        keyCode: {key_code},
                        which: {key_code},
                        ctrlKey: {ctrl_key},
                        metaKey: {meta_key},
                        shiftKey: {shift_key},
                        altKey: {alt_key},
                        bubbles: true,
                        cancelable: true
                    }});
                    activeEl.dispatchEvent(keydownEvent);

                    if (!{ctrl_key} && !{meta_key} && !{alt_key}) {{
                        if (activeEl.tagName === 'INPUT' || activeEl.tagName === 'TEXTAREA') {{
                            var nativeInputValueSetter = Object.getOwnPropertyDescriptor(
                                activeEl.tagName === 'INPUT'
                                    ? window.HTMLInputElement.prototype
                                    : window.HTMLTextAreaElement.prototype,
                                'value'
                            ).set;

                            var newValue = activeEl.value + '{escaped_key}';
                            nativeInputValueSetter.call(activeEl, newValue);

                            var inputEvent = new InputEvent('input', {{
                                bubbles: true,
                                cancelable: true,
                                inputType: 'insertText',
                                data: '{escaped_key}'
                            }});
                            activeEl.dispatchEvent(inputEvent);
                        }}
                    }}

                    return true;
                }})()"
            )
        } else {
            format!(
                r"(function() {{
                    var activeEl = document.activeElement || document.body;
                    var event = new KeyboardEvent('{event_type}', {{
                        key: '{escaped_key}',
                        code: '{escaped_code}',
                        keyCode: {key_code},
                        which: {key_code},
                        ctrlKey: {ctrl_key},
                        metaKey: {meta_key},
                        shiftKey: {shift_key},
                        altKey: {alt_key},
                        bubbles: true,
                        cancelable: true
                    }});
                    activeEl.dispatchEvent(event);
                    return true;
                }})()"
            )
        };

        self.evaluate_js(&script).await?;
        Ok(())
    }

    // =========================================================================
    // Window Management
    // =========================================================================

    async fn get_window_rect(&self) -> Result<WindowRect, WebDriverErrorResponse> {
        // Get viewport size from Kotlin plugin
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
