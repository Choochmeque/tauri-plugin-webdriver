use std::collections::HashMap;

use tauri::{Runtime, WebviewWindow};
use tokio::sync::{oneshot, RwLock};

use crate::server::response::WebDriverErrorResponse;

/// Global storage for pending JavaScript evaluation results
pub static PENDING_RESULTS: std::sync::LazyLock<RwLock<HashMap<String, oneshot::Sender<String>>>> =
    std::sync::LazyLock::new(|| RwLock::new(HashMap::new()));

/// Handle incoming JavaScript result from the webview
pub async fn handle_js_result(request_id: String, result: String) {
    let mut pending = PENDING_RESULTS.write().await;
    if let Some(tx) = pending.remove(&request_id) {
        let _ = tx.send(result);
    }
}

/// Executor for running JavaScript on WKWebView with result retrieval
#[derive(Clone)]
pub struct WebViewExecutor<R: Runtime> {
    window: WebviewWindow<R>,
}

impl<R: Runtime> WebViewExecutor<R> {
    /// Create a new executor from a Tauri WebviewWindow
    pub fn new(window: WebviewWindow<R>) -> Self {
        Self { window }
    }

    /// Evaluate JavaScript and return the result
    /// Uses Tauri's IPC to get results back
    pub async fn evaluate_js(
        &self,
        script: &str,
    ) -> Result<serde_json::Value, WebDriverErrorResponse> {
        let request_id = uuid::Uuid::new_v4().to_string();
        let (tx, rx) = oneshot::channel();

        // Store the sender
        {
            let mut pending = PENDING_RESULTS.write().await;
            pending.insert(request_id.clone(), tx);
        }

        // Wrap script to send result back via Tauri event
        let wrapped_script = format!(
            r#"(async function() {{
                var requestId = '{}';
                try {{
                    var result = (function() {{ {} }})();
                    if (window.__TAURI__ && window.__TAURI__.event) {{
                        await window.__TAURI__.event.emit('webdriver-result', {{
                            requestId: requestId,
                            success: true,
                            value: result
                        }});
                    }}
                }} catch (e) {{
                    if (window.__TAURI__ && window.__TAURI__.event) {{
                        await window.__TAURI__.event.emit('webdriver-result', {{
                            requestId: requestId,
                            success: false,
                            error: e.message || String(e)
                        }});
                    }}
                }}
            }})()"#,
            request_id, script
        );

        // Execute the script
        self.window
            .eval(&wrapped_script)
            .map_err(|e| WebDriverErrorResponse::javascript_error(&e.to_string()))?;

        // Wait for result with timeout
        let result = tokio::time::timeout(std::time::Duration::from_secs(30), rx)
            .await
            .map_err(|_| {
                // Clean up on timeout
                let request_id = request_id.clone();
                tokio::spawn(async move {
                    let mut pending = PENDING_RESULTS.write().await;
                    pending.remove(&request_id);
                });
                WebDriverErrorResponse::javascript_error("Script timeout")
            })?
            .map_err(|_| WebDriverErrorResponse::javascript_error("Channel closed"))?;

        // Parse the result
        serde_json::from_str(&result)
            .map_err(|e| WebDriverErrorResponse::javascript_error(&e.to_string()))
    }

    /// Simple eval without waiting for result (fire and forget)
    pub fn eval_no_wait(&self, script: &str) -> Result<(), WebDriverErrorResponse> {
        self.window
            .eval(script)
            .map_err(|e| WebDriverErrorResponse::javascript_error(&e.to_string()))
    }

    /// Navigate to a URL
    pub async fn navigate(&self, url: &str) -> Result<(), WebDriverErrorResponse> {
        let script = format!(
            r#"window.location.href = '{}'; return true;"#,
            url.replace('\\', "\\\\").replace('\'', "\\'")
        );
        self.evaluate_js(&script).await?;
        Ok(())
    }

    /// Get current URL
    pub async fn get_url(&self) -> Result<String, WebDriverErrorResponse> {
        let result = self.evaluate_js("return window.location.href;").await?;
        extract_string_value(&result)
    }

    /// Get page title
    pub async fn get_title(&self) -> Result<String, WebDriverErrorResponse> {
        let result = self.evaluate_js("return document.title;").await?;
        extract_string_value(&result)
    }

    /// Get page source
    pub async fn get_source(&self) -> Result<String, WebDriverErrorResponse> {
        let result = self
            .evaluate_js("return document.documentElement.outerHTML;")
            .await?;
        extract_string_value(&result)
    }

    /// Find element and store reference
    pub async fn find_element(
        &self,
        strategy_js: &str,
        js_var: &str,
    ) -> Result<bool, WebDriverErrorResponse> {
        let script = format!(
            r#"
            var el = {};
            if (el) {{
                window.{} = el;
                return true;
            }}
            return false;
            "#,
            strategy_js, js_var
        );
        let result = self.evaluate_js(&script).await?;
        extract_bool_value(&result)
    }

    /// Get element text
    pub async fn get_element_text(&self, js_var: &str) -> Result<String, WebDriverErrorResponse> {
        let script = format!(
            r#"
            var el = window.{};
            if (!el || !document.contains(el)) {{
                throw new Error('stale element reference');
            }}
            return el.textContent || '';
            "#,
            js_var
        );
        let result = self.evaluate_js(&script).await?;
        extract_string_value(&result)
    }

    /// Click element
    pub async fn click_element(&self, js_var: &str) -> Result<(), WebDriverErrorResponse> {
        let script = format!(
            r#"
            var el = window.{};
            if (!el || !document.contains(el)) {{
                throw new Error('stale element reference');
            }}
            el.scrollIntoView({{ block: 'center', inline: 'center' }});
            el.click();
            return true;
            "#,
            js_var
        );
        self.evaluate_js(&script).await?;
        Ok(())
    }

    /// Send keys to element
    pub async fn send_keys_to_element(
        &self,
        js_var: &str,
        text: &str,
    ) -> Result<(), WebDriverErrorResponse> {
        let escaped = text.replace('\\', "\\\\").replace('`', "\\`");
        let script = format!(
            r#"
            var el = window.{};
            if (!el || !document.contains(el)) {{
                throw new Error('stale element reference');
            }}
            el.focus();
            if (el.tagName === 'INPUT' || el.tagName === 'TEXTAREA') {{
                el.value += `{}`;
                el.dispatchEvent(new Event('input', {{ bubbles: true }}));
                el.dispatchEvent(new Event('change', {{ bubbles: true }}));
            }} else if (el.isContentEditable) {{
                document.execCommand('insertText', false, `{}`);
            }}
            return true;
            "#,
            js_var, escaped, escaped
        );
        self.evaluate_js(&script).await?;
        Ok(())
    }

    /// Get element attribute
    pub async fn get_element_attribute(
        &self,
        js_var: &str,
        name: &str,
    ) -> Result<Option<String>, WebDriverErrorResponse> {
        let escaped_name = name.replace('\\', "\\\\").replace('\'', "\\'");
        let script = format!(
            r#"
            var el = window.{};
            if (!el || !document.contains(el)) {{
                throw new Error('stale element reference');
            }}
            return el.getAttribute('{}');
            "#,
            js_var, escaped_name
        );
        let result = self.evaluate_js(&script).await?;

        if let Some(value) = result.get("value") {
            if value.is_null() {
                return Ok(None);
            }
            if let Some(s) = value.as_str() {
                return Ok(Some(s.to_string()));
            }
        }
        Ok(None)
    }

    /// Check if element is displayed
    pub async fn is_element_displayed(&self, js_var: &str) -> Result<bool, WebDriverErrorResponse> {
        let script = format!(
            r#"
            var el = window.{};
            if (!el || !document.contains(el)) {{
                throw new Error('stale element reference');
            }}
            var style = window.getComputedStyle(el);
            return style.display !== 'none' && style.visibility !== 'hidden' && el.offsetParent !== null;
            "#,
            js_var
        );
        let result = self.evaluate_js(&script).await?;
        extract_bool_value(&result)
    }

    /// Execute user script
    pub async fn execute_script(
        &self,
        script: &str,
        args: &[serde_json::Value],
    ) -> Result<serde_json::Value, WebDriverErrorResponse> {
        let args_json = serde_json::to_string(args)
            .map_err(|e| WebDriverErrorResponse::invalid_argument(&e.to_string()))?;

        let wrapper = format!(
            r#"
            var args = {};
            var fn = function() {{ {} }};
            return fn.apply(null, args);
            "#,
            args_json, script
        );
        let result = self.evaluate_js(&wrapper).await?;

        if let Some(success) = result.get("success").and_then(|v| v.as_bool()) {
            if success {
                return Ok(result.get("value").cloned().unwrap_or(serde_json::Value::Null));
            } else if let Some(error) = result.get("error").and_then(|v| v.as_str()) {
                return Err(WebDriverErrorResponse::javascript_error(error));
            }
        }

        Ok(serde_json::Value::Null)
    }
}

/// Extract string value from JavaScript result
fn extract_string_value(result: &serde_json::Value) -> Result<String, WebDriverErrorResponse> {
    if let Some(success) = result.get("success").and_then(|v| v.as_bool()) {
        if success {
            if let Some(value) = result.get("value") {
                if let Some(s) = value.as_str() {
                    return Ok(s.to_string());
                }
                return Ok(value.to_string());
            }
        } else if let Some(error) = result.get("error").and_then(|v| v.as_str()) {
            return Err(WebDriverErrorResponse::javascript_error(error));
        }
    }
    Ok(String::new())
}

/// Extract boolean value from JavaScript result
fn extract_bool_value(result: &serde_json::Value) -> Result<bool, WebDriverErrorResponse> {
    if let Some(success) = result.get("success").and_then(|v| v.as_bool()) {
        if success {
            if let Some(value) = result.get("value").and_then(|v| v.as_bool()) {
                return Ok(value);
            }
        } else if let Some(error) = result.get("error").and_then(|v| v.as_str()) {
            return Err(WebDriverErrorResponse::javascript_error(error));
        }
    }
    Ok(false)
}
