use std::sync::Arc;

use async_trait::async_trait;
use serde_json::Value;
use tauri::{Runtime, WebviewWindow};
use tokio::sync::oneshot;
use webview2_com::Microsoft::Web::WebView2::Win32::ICoreWebView2ExecuteScriptCompletedHandler;
use windows::core::{HSTRING, PCWSTR};
use windows::Win32::System::Com::{CoInitializeEx, COINIT_APARTMENTTHREADED};

use crate::platform::{wrap_script_for_frame_context, FrameId, PlatformExecutor, PrintOptions};
use crate::server::response::WebDriverErrorResponse;
use crate::webdriver::Timeouts;

/// Windows `WebView2` executor
#[derive(Clone)]
pub struct WindowsExecutor<R: Runtime> {
    window: WebviewWindow<R>,
    timeouts: Timeouts,
    frame_context: Vec<FrameId>,
}

impl<R: Runtime> WindowsExecutor<R> {
    pub fn new(window: WebviewWindow<R>, timeouts: Timeouts, frame_context: Vec<FrameId>) -> Self {
        Self {
            window,
            timeouts,
            frame_context,
        }
    }
}

#[async_trait]
impl<R: Runtime + 'static> PlatformExecutor<R> for WindowsExecutor<R> {
    // =========================================================================
    // Window Access
    // =========================================================================

    fn window(&self) -> &WebviewWindow<R> {
        &self.window
    }

    fn timeouts(&self) -> &Timeouts {
        &self.timeouts
    }

    // =========================================================================
    // Core JavaScript Execution
    // =========================================================================

    async fn evaluate_js(&self, script: &str) -> Result<Value, WebDriverErrorResponse> {
        let (tx, rx) = oneshot::channel();
        let script_owned = wrap_script_for_frame_context(script, &self.frame_context);

        let result = self.window.with_webview(move |webview| unsafe {
            let _ = CoInitializeEx(None, COINIT_APARTMENTTHREADED);

            if let Ok(webview2) = webview.controller().CoreWebView2() {
                let script_hstring = HSTRING::from(&script_owned);

                let tx = Arc::new(std::sync::Mutex::new(Some(tx)));
                let handler: ICoreWebView2ExecuteScriptCompletedHandler =
                    ExecuteScriptHandler::new(tx).into();

                webview2
                    .ExecuteScript(PCWSTR(script_hstring.as_ptr()), &handler)
                    .ok();
            }
        });

        if let Err(e) = result {
            return Err(WebDriverErrorResponse::javascript_error(
                &e.to_string(),
                None,
            ));
        }

        let timeout = std::time::Duration::from_millis(self.timeouts.script_ms);
        match tokio::time::timeout(timeout, rx).await {
            Ok(Ok(Ok(value))) => Ok(serde_json::json!({
                "success": true,
                "value": value
            })),
            Ok(Ok(Err(error))) => Err(WebDriverErrorResponse::javascript_error(&error, None)),
            Ok(Err(_)) => Err(WebDriverErrorResponse::unknown_error("Channel closed")),
            Err(_) => Err(WebDriverErrorResponse::script_timeout()),
        }
    }

    // =========================================================================
    // Screenshots
    // =========================================================================

    async fn take_screenshot(&self) -> Result<String, WebDriverErrorResponse> {
        // Use JavaScript canvas-based screenshot for cross-platform compatibility
        let _script = r"(function() {
            return new Promise(function(resolve, reject) {
                try {
                    var canvas = document.createElement('canvas');
                    var ctx = canvas.getContext('2d');
                    canvas.width = document.documentElement.scrollWidth;
                    canvas.height = document.documentElement.scrollHeight;

                    // For simple pages, we can use html2canvas-like approach
                    // TODO: For now, return a placeholder - native WebView2 CapturePreview would be better
                    resolve('');
                } catch (e) {
                    reject(e.message);
                }
            });
        })()";

        // For Windows, we should use WebView2's CapturePreview API
        // This is a simplified implementation - full implementation would use native API
        let (tx, rx) = oneshot::channel();

        let result = self.window.with_webview(move |webview| {
            unsafe {
                if let Ok(_webview2) = webview.controller().CoreWebView2() {
                    // Create a memory stream for the image
                    // Note: This requires additional COM setup for IStream
                    // For now, return a placeholder

                    let tx = Arc::new(std::sync::Mutex::new(Some(tx)));
                    let handler = CapturePreviewHandler::new(tx);

                    // TODO: CapturePreview requires an IStream - simplified for now
                    // webview2.CapturePreview(COREWEBVIEW2_CAPTURE_PREVIEW_IMAGE_FORMAT_PNG, stream, &handler);

                    // For now, signal completion with empty result
                    if let Ok(mut guard) = handler.tx.lock() {
                        if let Some(tx) = guard.take() {
                            let _ = tx.send(Ok(String::new()));
                        }
                    };
                }
            }
        });

        if let Err(e) = result {
            return Err(WebDriverErrorResponse::unknown_error(&e.to_string()));
        }

        let timeout = std::time::Duration::from_millis(self.timeouts.script_ms);
        match tokio::time::timeout(timeout, rx).await {
            Ok(Ok(Ok(base64))) => {
                if base64.is_empty() {
                    // Fallback to JS-based screenshot
                    self.take_js_screenshot().await
                } else {
                    Ok(base64)
                }
            }
            Ok(Ok(Err(error))) => Err(WebDriverErrorResponse::unknown_error(&error)),
            Ok(Err(_)) => Err(WebDriverErrorResponse::unknown_error("Channel closed")),
            Err(_) => Err(WebDriverErrorResponse::script_timeout()),
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
                if (!el || !document.contains(el)) {{
                    throw new Error('stale element reference');
                }}
                el.scrollIntoView({{ block: 'center', inline: 'center' }});
                return true;
            }})()"
        );
        self.evaluate_js(&script).await?;

        // Take full screenshot and return (element clipping can be added later)
        self.take_screenshot().await
    }

    // =========================================================================
    // Alerts
    // =========================================================================

    async fn dismiss_alert(&self) -> Result<(), WebDriverErrorResponse> {
        // TODO: Implement native alert handling using WebView2's dialog event handlers
        Err(WebDriverErrorResponse::unsupported_operation(
            "Alert handling not yet implemented for Windows",
        ))
    }

    async fn accept_alert(&self) -> Result<(), WebDriverErrorResponse> {
        // TODO: Implement native alert handling using WebView2's dialog event handlers
        Err(WebDriverErrorResponse::unsupported_operation(
            "Alert handling not yet implemented for Windows",
        ))
    }

    async fn get_alert_text(&self) -> Result<String, WebDriverErrorResponse> {
        // TODO: Implement native alert handling using WebView2's dialog event handlers
        Err(WebDriverErrorResponse::unsupported_operation(
            "Alert handling not yet implemented for Windows",
        ))
    }

    async fn send_alert_text(&self, _text: &str) -> Result<(), WebDriverErrorResponse> {
        // TODO: Implement native alert handling using WebView2's dialog event handlers
        Err(WebDriverErrorResponse::unsupported_operation(
            "Alert handling not yet implemented for Windows",
        ))
    }

    // =========================================================================
    // Print
    // =========================================================================

    async fn print_page(&self, _options: PrintOptions) -> Result<String, WebDriverErrorResponse> {
        // TODO: Implement PDF printing using WebView2's PrintToPdf API
        Err(WebDriverErrorResponse::unsupported_operation(
            "PDF printing not yet implemented for Windows",
        ))
    }
}

// =============================================================================
// Helper Methods
// =============================================================================

impl<R: Runtime + 'static> WindowsExecutor<R> {
    async fn take_js_screenshot(&self) -> Result<String, WebDriverErrorResponse> {
        // JavaScript-based screenshot using canvas
        let script = r"(function() {
            return new Promise(function(resolve, reject) {
                try {
                    // This is a simplified approach - full implementation would use html2canvas
                    var canvas = document.createElement('canvas');
                    var ctx = canvas.getContext('2d');
                    canvas.width = window.innerWidth;
                    canvas.height = window.innerHeight;

                    // Draw a white background
                    ctx.fillStyle = 'white';
                    ctx.fillRect(0, 0, canvas.width, canvas.height);

                    // For now, return base64 of blank canvas
                    // Full implementation would render DOM to canvas
                    var dataUrl = canvas.toDataURL('image/png');
                    resolve(dataUrl.replace('data:image/png;base64,', ''));
                } catch (e) {
                    reject(e.message);
                }
            });
        })()";

        let result = self.evaluate_js(script).await?;
        extract_string_value(&result)
    }
}

// =============================================================================
// COM Handlers
// =============================================================================

type ScriptResultSender = Arc<std::sync::Mutex<Option<oneshot::Sender<Result<Value, String>>>>>;
type CaptureResultSender = Arc<std::sync::Mutex<Option<oneshot::Sender<Result<String, String>>>>>;

mod handlers {
    #![allow(clippy::inline_always, clippy::ref_as_ptr)]

    use serde_json::Value;
    use webview2_com::Microsoft::Web::WebView2::Win32::{
        ICoreWebView2CapturePreviewCompletedHandler,
        ICoreWebView2CapturePreviewCompletedHandler_Impl,
        ICoreWebView2ExecuteScriptCompletedHandler,
        ICoreWebView2ExecuteScriptCompletedHandler_Impl,
    };
    use windows::core::implement;

    use super::{CaptureResultSender, ScriptResultSender};

    #[implement(ICoreWebView2ExecuteScriptCompletedHandler)]
    pub struct ExecuteScriptHandler {
        pub tx: ScriptResultSender,
    }

    impl ExecuteScriptHandler {
        pub fn new(tx: ScriptResultSender) -> Self {
            Self { tx }
        }
    }

    impl ICoreWebView2ExecuteScriptCompletedHandler_Impl for ExecuteScriptHandler_Impl {
        fn Invoke(
            &self,
            errorcode: windows::core::HRESULT,
            resultobjectasjson: &windows::core::PCWSTR,
        ) -> windows::core::Result<()> {
            let response = if errorcode.is_err() {
                Err(format!("Script execution failed: {errorcode:?}"))
            } else {
                let json_str = unsafe { resultobjectasjson.to_string().unwrap_or_default() };
                match serde_json::from_str(&json_str) {
                    Ok(value) => Ok(value),
                    Err(_) => Ok(Value::String(json_str)),
                }
            };

            if let Ok(mut guard) = self.tx.lock() {
                if let Some(tx) = guard.take() {
                    let _ = tx.send(response);
                }
            }
            Ok(())
        }
    }

    #[implement(ICoreWebView2CapturePreviewCompletedHandler)]
    pub struct CapturePreviewHandler {
        pub tx: CaptureResultSender,
    }

    impl CapturePreviewHandler {
        pub fn new(tx: CaptureResultSender) -> Self {
            Self { tx }
        }
    }

    impl ICoreWebView2CapturePreviewCompletedHandler_Impl for CapturePreviewHandler_Impl {
        fn Invoke(&self, errorcode: windows::core::HRESULT) -> windows::core::Result<()> {
            let response = if errorcode.is_err() {
                Err(format!("Capture preview failed: {errorcode:?}"))
            } else {
                // In a full implementation, we'd read the IStream here
                Ok(String::new())
            };

            if let Ok(mut guard) = self.tx.lock() {
                if let Some(tx) = guard.take() {
                    let _ = tx.send(response);
                }
            }
            Ok(())
        }
    }
}

use handlers::{CapturePreviewHandler, ExecuteScriptHandler};

/// Extract string value from JavaScript result
fn extract_string_value(result: &Value) -> Result<String, WebDriverErrorResponse> {
    if let Some(success) = result.get("success").and_then(Value::as_bool) {
        if success {
            if let Some(value) = result.get("value") {
                if let Some(s) = value.as_str() {
                    return Ok(s.to_string());
                }
                return Ok(value.to_string());
            }
        } else if let Some(error) = result.get("error").and_then(Value::as_str) {
            return Err(WebDriverErrorResponse::javascript_error(error, None));
        }
    }
    Ok(String::new())
}
