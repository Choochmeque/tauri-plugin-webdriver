use std::sync::Arc;

use async_trait::async_trait;
use glib::MainContext;
use javascriptcore::ValueExt;
use serde_json::Value;
use tauri::{Runtime, WebviewWindow};
use tokio::sync::oneshot;
use webkit2gtk::WebViewExt;

use crate::platform::{PlatformExecutor, PrintOptions};
use crate::server::response::WebDriverErrorResponse;
use crate::webdriver::Timeouts;

/// Linux `WebKitGTK` executor
#[derive(Clone)]
pub struct LinuxExecutor<R: Runtime> {
    window: WebviewWindow<R>,
    timeouts: Timeouts,
}

impl<R: Runtime> LinuxExecutor<R> {
    pub fn new(window: WebviewWindow<R>, timeouts: Timeouts) -> Self {
        Self { window, timeouts }
    }
}

#[async_trait]
impl<R: Runtime + 'static> PlatformExecutor<R> for LinuxExecutor<R> {
    // =========================================================================
    // Window Access
    // =========================================================================

    fn window(&self) -> &WebviewWindow<R> {
        &self.window
    }

    // =========================================================================
    // Core JavaScript Execution
    // =========================================================================

    async fn evaluate_js(&self, script: &str) -> Result<Value, WebDriverErrorResponse> {
        let (tx, rx) = oneshot::channel();
        let script_owned = script.to_string();

        let result = self.window.with_webview(move |webview| {
            let webview = webview.inner().clone();
            let tx = Arc::new(std::sync::Mutex::new(Some(tx)));

            // Use glib main context to spawn the async future
            let ctx = MainContext::default();
            ctx.spawn_local(async move {
                let result = webview
                    .evaluate_javascript_future(&script_owned, None, None)
                    .await;
                let response: Result<Value, String> = match result {
                    Ok(js_value) => {
                        if let Some(json_str) = js_value.to_json(0) {
                            match serde_json::from_str::<Value>(json_str.as_str()) {
                                Ok(value) => Ok(value),
                                Err(_) => Ok(Value::String(json_str.to_string())),
                            }
                        } else {
                            Ok(Value::Null)
                        }
                    }
                    Err(e) => Err(e.to_string()),
                };

                if let Ok(mut guard) = tx.lock() {
                    if let Some(tx) = guard.take() {
                        let _ = tx.send(response);
                    }
                }
            });
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
        // Use JavaScript canvas-based screenshot
        let script = r"(function() {
            return new Promise(function(resolve, reject) {
                try {
                    var canvas = document.createElement('canvas');
                    var ctx = canvas.getContext('2d');
                    canvas.width = window.innerWidth;
                    canvas.height = window.innerHeight;

                    ctx.fillStyle = 'white';
                    ctx.fillRect(0, 0, canvas.width, canvas.height);

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

    async fn take_element_screenshot(
        &self,
        js_var: &str,
    ) -> Result<String, WebDriverErrorResponse> {
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

        self.take_screenshot().await
    }

    // =========================================================================
    // Alerts
    // =========================================================================

    async fn dismiss_alert(&self) -> Result<(), WebDriverErrorResponse> {
        // TODO: Implement native alert handling using WebKitGTK's dialog event handlers
        Err(WebDriverErrorResponse::unsupported_operation(
            "Alert handling not yet implemented for Linux",
        ))
    }

    async fn accept_alert(&self) -> Result<(), WebDriverErrorResponse> {
        // TODO: Implement native alert handling using WebKitGTK's dialog event handlers
        Err(WebDriverErrorResponse::unsupported_operation(
            "Alert handling not yet implemented for Linux",
        ))
    }

    async fn get_alert_text(&self) -> Result<String, WebDriverErrorResponse> {
        // TODO: Implement native alert handling using WebKitGTK's dialog event handlers
        Err(WebDriverErrorResponse::unsupported_operation(
            "Alert handling not yet implemented for Linux",
        ))
    }

    async fn send_alert_text(&self, _text: &str) -> Result<(), WebDriverErrorResponse> {
        // TODO: Implement native alert handling using WebKitGTK's dialog event handlers
        Err(WebDriverErrorResponse::unsupported_operation(
            "Alert handling not yet implemented for Linux",
        ))
    }

    // =========================================================================
    // Print
    // =========================================================================

    async fn print_page(&self, _options: PrintOptions) -> Result<String, WebDriverErrorResponse> {
        // TODO: Implement PDF printing using WebKitGTK's print operation
        Err(WebDriverErrorResponse::unsupported_operation(
            "PDF printing not yet implemented for Linux",
        ))
    }
}

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
