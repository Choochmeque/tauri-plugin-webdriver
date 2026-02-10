use std::sync::Arc;

use base64::engine::general_purpose::STANDARD as BASE64_STANDARD;
use base64::Engine as _;
use block2::RcBlock;
use objc2::runtime::AnyObject;
use objc2_app_kit::{NSBitmapImageFileType, NSBitmapImageRep, NSImage};
use objc2_foundation::{NSData, NSDictionary, NSError, NSString};
use objc2_web_kit::{WKSnapshotConfiguration, WKWebView};
use tauri::{Runtime, WebviewWindow};
use tokio::sync::oneshot;

use crate::server::response::WebDriverErrorResponse;

/// Executor for running JavaScript on WKWebView with result retrieval
#[derive(Clone)]
pub struct WebViewExecutor<R: Runtime> {
    window: WebviewWindow<R>,
}

impl<R: Runtime> WebViewExecutor<R> {
    pub fn new(window: WebviewWindow<R>) -> Self {
        Self { window }
    }

    /// Evaluate JavaScript using native WKWebView API and return the result
    pub async fn evaluate_js(
        &self,
        script: &str,
    ) -> Result<serde_json::Value, WebDriverErrorResponse> {
        let (tx, rx) = oneshot::channel();
        let script_owned = script.to_string();

        let result = self.window.with_webview(move |webview| {
            unsafe {
                let wk_webview: &WKWebView = &*webview.inner().cast();
                let ns_script = NSString::from_str(&script_owned);

                // Create completion handler block
                let tx = Arc::new(std::sync::Mutex::new(Some(tx)));
                let block = RcBlock::new(move |result: *mut AnyObject, error: *mut NSError| {
                    let response = if !error.is_null() {
                        let error_ref = &*error;
                        let description = error_ref.localizedDescription();
                        Err(description.to_string())
                    } else if result.is_null() {
                        Ok(serde_json::Value::Null)
                    } else {
                        // Convert NSObject to JSON value
                        let obj = &*result;
                        Ok(ns_object_to_json(obj))
                    };

                    if let Some(tx) = tx.lock().unwrap().take() {
                        let _ = tx.send(response);
                    }
                });

                wk_webview.evaluateJavaScript_completionHandler(&ns_script, Some(&block));
            }
        });

        if let Err(e) = result {
            return Err(WebDriverErrorResponse::javascript_error(&e.to_string()));
        }

        // Wait for result with timeout
        match tokio::time::timeout(std::time::Duration::from_secs(30), rx).await {
            Ok(Ok(Ok(value))) => Ok(serde_json::json!({
                "success": true,
                "value": value
            })),
            Ok(Ok(Err(error))) => Err(WebDriverErrorResponse::javascript_error(&error)),
            Ok(Err(_)) => Err(WebDriverErrorResponse::javascript_error("Channel closed")),
            Err(_) => Err(WebDriverErrorResponse::javascript_error("Script timeout")),
        }
    }

    /// Navigate to a URL
    pub async fn navigate(&self, url: &str) -> Result<(), WebDriverErrorResponse> {
        let script = format!(
            r#"window.location.href = '{}'; null;"#,
            url.replace('\\', "\\\\").replace('\'', "\\'")
        );
        self.evaluate_js(&script).await?;
        Ok(())
    }

    /// Get current URL
    pub async fn get_url(&self) -> Result<String, WebDriverErrorResponse> {
        let result = self.evaluate_js("window.location.href").await?;
        extract_string_value(&result)
    }

    /// Get page title
    pub async fn get_title(&self) -> Result<String, WebDriverErrorResponse> {
        let result = self.evaluate_js("document.title").await?;
        extract_string_value(&result)
    }

    /// Get page source
    pub async fn get_source(&self) -> Result<String, WebDriverErrorResponse> {
        let result = self
            .evaluate_js("document.documentElement.outerHTML")
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
            r#"(function() {{
                var el = {};
                if (el) {{
                    window.{} = el;
                    return true;
                }}
                return false;
            }})()"#,
            strategy_js, js_var
        );
        let result = self.evaluate_js(&script).await?;
        extract_bool_value(&result)
    }

    /// Get element text
    pub async fn get_element_text(&self, js_var: &str) -> Result<String, WebDriverErrorResponse> {
        let script = format!(
            r#"(function() {{
                var el = window.{};
                if (!el || !document.contains(el)) {{
                    throw new Error('stale element reference');
                }}
                return el.textContent || '';
            }})()"#,
            js_var
        );
        let result = self.evaluate_js(&script).await?;
        extract_string_value(&result)
    }

    /// Click element
    pub async fn click_element(&self, js_var: &str) -> Result<(), WebDriverErrorResponse> {
        let script = format!(
            r#"(function() {{
                var el = window.{};
                if (!el || !document.contains(el)) {{
                    throw new Error('stale element reference');
                }}
                el.scrollIntoView({{ block: 'center', inline: 'center' }});
                el.click();
                return true;
            }})()"#,
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
        let escaped = text.replace('\\', "\\\\").replace('`', "\\`").replace('$', "\\$");
        let script = format!(
            r#"(function() {{
                var el = window.{};
                if (!el || !document.contains(el)) {{
                    throw new Error('stale element reference');
                }}
                el.focus();

                if (el.tagName === 'INPUT' || el.tagName === 'TEXTAREA') {{
                    // For React controlled inputs, we need to use the native value setter
                    var nativeInputValueSetter = Object.getOwnPropertyDescriptor(
                        el.tagName === 'INPUT' ? window.HTMLInputElement.prototype : window.HTMLTextAreaElement.prototype,
                        'value'
                    ).set;

                    var newValue = el.value + `{}`;
                    nativeInputValueSetter.call(el, newValue);

                    // Dispatch InputEvent (more specific than Event)
                    var inputEvent = new InputEvent('input', {{
                        bubbles: true,
                        cancelable: true,
                        inputType: 'insertText',
                        data: `{}`
                    }});
                    el.dispatchEvent(inputEvent);

                    // Also dispatch change event
                    var changeEvent = new Event('change', {{ bubbles: true }});
                    el.dispatchEvent(changeEvent);
                }} else if (el.isContentEditable) {{
                    document.execCommand('insertText', false, `{}`);
                }}
                return true;
            }})()"#,
            js_var, escaped, escaped, escaped
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
            r#"(function() {{
                var el = window.{};
                if (!el || !document.contains(el)) {{
                    throw new Error('stale element reference');
                }}
                return el.getAttribute('{}');
            }})()"#,
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
            r#"(function() {{
                var el = window.{};
                if (!el || !document.contains(el)) {{
                    throw new Error('stale element reference');
                }}
                var style = window.getComputedStyle(el);
                return style.display !== 'none' && style.visibility !== 'hidden' && el.offsetParent !== null;
            }})()"#,
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
            r#"(function() {{
                var args = {};
                var fn = function() {{ {} }};
                return fn.apply(null, args);
            }})()"#,
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

    /// Execute asynchronous user script (with callback)
    pub async fn execute_async_script(
        &self,
        script: &str,
        args: &[serde_json::Value],
    ) -> Result<serde_json::Value, WebDriverErrorResponse> {
        let args_json = serde_json::to_string(args)
            .map_err(|e| WebDriverErrorResponse::invalid_argument(&e.to_string()))?;

        // For async scripts, we return a Promise and resolve it with the callback
        let wrapper = format!(
            r#"new Promise(function(resolve, reject) {{
                try {{
                    var args = {};
                    args.push(function(result) {{ resolve(result); }});
                    var fn = function() {{ {} }};
                    fn.apply(null, args);
                }} catch (e) {{
                    reject(e);
                }}
            }})"#,
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

    /// Get element tag name
    pub async fn get_element_tag_name(&self, js_var: &str) -> Result<String, WebDriverErrorResponse> {
        let script = format!(
            r#"(function() {{
                var el = window.{};
                if (!el || !document.contains(el)) {{
                    throw new Error('stale element reference');
                }}
                return el.tagName.toLowerCase();
            }})()"#,
            js_var
        );
        let result = self.evaluate_js(&script).await?;
        extract_string_value(&result)
    }

    /// Get element property
    pub async fn get_element_property(
        &self,
        js_var: &str,
        name: &str,
    ) -> Result<serde_json::Value, WebDriverErrorResponse> {
        let escaped_name = name.replace('\\', "\\\\").replace('\'', "\\'");
        let script = format!(
            r#"(function() {{
                var el = window.{};
                if (!el || !document.contains(el)) {{
                    throw new Error('stale element reference');
                }}
                return el['{}'];
            }})()"#,
            js_var, escaped_name
        );
        let result = self.evaluate_js(&script).await?;

        if let Some(success) = result.get("success").and_then(|v| v.as_bool()) {
            if success {
                return Ok(result.get("value").cloned().unwrap_or(serde_json::Value::Null));
            } else if let Some(error) = result.get("error").and_then(|v| v.as_str()) {
                return Err(WebDriverErrorResponse::javascript_error(error));
            }
        }
        Ok(serde_json::Value::Null)
    }

    /// Check if element is enabled
    pub async fn is_element_enabled(&self, js_var: &str) -> Result<bool, WebDriverErrorResponse> {
        let script = format!(
            r#"(function() {{
                var el = window.{};
                if (!el || !document.contains(el)) {{
                    throw new Error('stale element reference');
                }}
                return !el.disabled;
            }})()"#,
            js_var
        );
        let result = self.evaluate_js(&script).await?;
        extract_bool_value(&result)
    }

    /// Take screenshot using WKWebView's native snapshot API
    pub async fn take_screenshot(&self) -> Result<String, WebDriverErrorResponse> {
        let (tx, rx) = oneshot::channel();

        let result = self.window.with_webview(move |webview| {
            unsafe {
                let wk_webview: &WKWebView = &*webview.inner().cast();

                // Create snapshot configuration (nil = full page)
                let config = WKSnapshotConfiguration::new();

                // Create completion handler
                let tx = Arc::new(std::sync::Mutex::new(Some(tx)));
                let block = RcBlock::new(move |image: *mut NSImage, error: *mut NSError| {
                    let response = if !error.is_null() {
                        let error_ref = &*error;
                        let description = error_ref.localizedDescription();
                        Err(description.to_string())
                    } else if image.is_null() {
                        Err("No image returned".to_string())
                    } else {
                        // Convert NSImage to PNG data
                        let image_ref = &*image;
                        match image_to_png_base64(image_ref) {
                            Ok(base64) => Ok(base64),
                            Err(e) => Err(e),
                        }
                    };

                    if let Some(tx) = tx.lock().unwrap().take() {
                        let _ = tx.send(response);
                    }
                });

                wk_webview.takeSnapshotWithConfiguration_completionHandler(Some(&config), &block);
            }
        });

        if let Err(e) = result {
            return Err(WebDriverErrorResponse::unknown_error(&e.to_string()));
        }

        // Wait for result with timeout
        match tokio::time::timeout(std::time::Duration::from_secs(30), rx).await {
            Ok(Ok(Ok(base64))) => Ok(base64),
            Ok(Ok(Err(error))) => Err(WebDriverErrorResponse::unknown_error(&error)),
            Ok(Err(_)) => Err(WebDriverErrorResponse::unknown_error("Channel closed")),
            Err(_) => Err(WebDriverErrorResponse::unknown_error("Screenshot timeout")),
        }
    }

    /// Dispatch a key event (keydown/keyup)
    pub async fn dispatch_key_event(
        &self,
        key: &str,
        is_down: bool,
    ) -> Result<(), WebDriverErrorResponse> {
        // Map WebDriver key codes to JavaScript key values
        // WebDriver uses Unicode Private Use Area for special keys
        let (js_key, js_code, key_code) = match key {
            "\u{E007}" => ("Enter", "Enter", 13),           // Enter
            "\u{E003}" => ("Backspace", "Backspace", 8),    // Backspace
            "\u{E004}" => ("Tab", "Tab", 9),                // Tab
            "\u{E006}" => ("Enter", "NumpadEnter", 13),     // Return (numpad enter)
            "\u{E00C}" => ("Escape", "Escape", 27),         // Escape
            "\u{E00D}" => (" ", "Space", 32),               // Space
            "\u{E012}" => ("ArrowLeft", "ArrowLeft", 37),   // Left arrow
            "\u{E013}" => ("ArrowUp", "ArrowUp", 38),       // Up arrow
            "\u{E014}" => ("ArrowRight", "ArrowRight", 39), // Right arrow
            "\u{E015}" => ("ArrowDown", "ArrowDown", 40),   // Down arrow
            "\u{E017}" => ("Delete", "Delete", 46),         // Delete
            "\u{E031}" => ("F1", "F1", 112),                // F1
            "\u{E032}" => ("F2", "F2", 113),                // F2
            "\u{E033}" => ("F3", "F3", 114),                // F3
            "\u{E034}" => ("F4", "F4", 115),                // F4
            "\u{E035}" => ("F5", "F5", 116),                // F5
            "\u{E036}" => ("F6", "F6", 117),                // F6
            "\u{E037}" => ("F7", "F7", 118),                // F7
            "\u{E038}" => ("F8", "F8", 119),                // F8
            "\u{E039}" => ("F9", "F9", 120),                // F9
            "\u{E03A}" => ("F10", "F10", 121),              // F10
            "\u{E03B}" => ("F11", "F11", 122),              // F11
            "\u{E03C}" => ("F12", "F12", 123),              // F12
            "\u{E008}" => ("Shift", "ShiftLeft", 16),       // Shift
            "\u{E009}" => ("Control", "ControlLeft", 17),   // Control
            "\u{E00A}" => ("Alt", "AltLeft", 18),           // Alt
            "\u{E03D}" => ("Meta", "MetaLeft", 91),         // Meta/Command
            _ => {
                // Regular character key
                let ch = key.chars().next().unwrap_or(' ');
                let code = if ch.is_ascii_alphabetic() {
                    format!("Key{}", ch.to_ascii_uppercase())
                } else if ch.is_ascii_digit() {
                    format!("Digit{}", ch)
                } else {
                    key.to_string()
                };
                // Return a tuple with owned Strings that we handle specially
                return self.dispatch_regular_key(key, &code, is_down).await;
            }
        };

        let event_type = if is_down { "keydown" } else { "keyup" };
        let script = format!(
            r#"(function() {{
                var event = new KeyboardEvent('{}', {{
                    key: '{}',
                    code: '{}',
                    keyCode: {},
                    which: {},
                    bubbles: true,
                    cancelable: true
                }});
                var activeEl = document.activeElement || document.body;
                activeEl.dispatchEvent(event);
                return true;
            }})()"#,
            event_type, js_key, js_code, key_code, key_code
        );

        self.evaluate_js(&script).await?;
        Ok(())
    }

    /// Dispatch a regular character key event
    async fn dispatch_regular_key(
        &self,
        key: &str,
        code: &str,
        is_down: bool,
    ) -> Result<(), WebDriverErrorResponse> {
        let ch = key.chars().next().unwrap_or(' ');
        let key_code = ch as u32;
        let event_type = if is_down { "keydown" } else { "keyup" };

        let escaped_key = key.replace('\\', "\\\\").replace('\'', "\\'");
        let escaped_code = code.replace('\\', "\\\\").replace('\'', "\\'");

        let script = format!(
            r#"(function() {{
                var event = new KeyboardEvent('{}', {{
                    key: '{}',
                    code: '{}',
                    keyCode: {},
                    which: {},
                    bubbles: true,
                    cancelable: true
                }});
                var activeEl = document.activeElement || document.body;
                activeEl.dispatchEvent(event);
                return true;
            }})()"#,
            event_type, escaped_key, escaped_code, key_code, key_code
        );

        self.evaluate_js(&script).await?;
        Ok(())
    }

    /// Clear element content
    pub async fn clear_element(&self, js_var: &str) -> Result<(), WebDriverErrorResponse> {
        let script = format!(
            r#"(function() {{
                var el = window.{};
                if (!el || !document.contains(el)) {{
                    throw new Error('stale element reference');
                }}
                el.focus();
                if (el.tagName === 'INPUT' || el.tagName === 'TEXTAREA') {{
                    var nativeInputValueSetter = Object.getOwnPropertyDescriptor(
                        el.tagName === 'INPUT' ? window.HTMLInputElement.prototype : window.HTMLTextAreaElement.prototype,
                        'value'
                    ).set;
                    nativeInputValueSetter.call(el, '');
                    var inputEvent = new InputEvent('input', {{
                        bubbles: true,
                        cancelable: true,
                        inputType: 'deleteContentBackward'
                    }});
                    el.dispatchEvent(inputEvent);
                    var changeEvent = new Event('change', {{ bubbles: true }});
                    el.dispatchEvent(changeEvent);
                }} else if (el.isContentEditable) {{
                    el.innerHTML = '';
                }}
                return true;
            }})()"#,
            js_var
        );
        self.evaluate_js(&script).await?;
        Ok(())
    }
}

/// Convert NSImage to PNG and encode as base64
unsafe fn image_to_png_base64(image: &NSImage) -> Result<String, String> {
    // Get TIFF representation
    let tiff_data: Option<objc2::rc::Retained<NSData>> = image.TIFFRepresentation();
    let tiff_data = tiff_data.ok_or("Failed to get TIFF representation")?;

    // Create bitmap image rep from TIFF data
    let bitmap_rep = NSBitmapImageRep::imageRepWithData(&tiff_data)
        .ok_or("Failed to create bitmap image rep")?;

    // Convert to PNG with empty properties dictionary
    let empty_dict: objc2::rc::Retained<NSDictionary<NSString>> = NSDictionary::new();
    let png_data: Option<objc2::rc::Retained<NSData>> =
        bitmap_rep.representationUsingType_properties(NSBitmapImageFileType::PNG, &empty_dict);
    let png_data = png_data.ok_or("Failed to convert to PNG")?;

    // Get raw bytes and base64 encode
    let bytes = png_data.bytes();
    Ok(BASE64_STANDARD.encode(bytes))
}

/// Convert an NSObject to a JSON value
unsafe fn ns_object_to_json(obj: &AnyObject) -> serde_json::Value {
    use objc2_foundation::NSString as NSStr;

    let class = obj.class();
    let class_name = class.name();

    // Check for NSString
    if class_name.contains("String") {
        let ns_str: &NSStr = &*(obj as *const AnyObject as *const NSStr);
        return serde_json::Value::String(ns_str.to_string());
    }

    // Check for NSNumber (includes booleans)
    if class_name.contains("Number") || class_name.contains("Boolean") {
        // Use message sending to get values since NSNumber API varies
        use objc2::msg_send;
        use objc2::runtime::Bool;

        // Try to get as bool first (for __NSCFBoolean)
        if class_name.contains("Boolean") {
            let bool_val: Bool = msg_send![obj, boolValue];
            return serde_json::Value::Bool(bool_val.as_bool());
        }

        // Try as double
        let double_val: f64 = msg_send![obj, doubleValue];
        let int_val: i64 = msg_send![obj, longLongValue];

        // Check if it's an integer
        if (int_val as f64 - double_val).abs() < f64::EPSILON {
            return serde_json::Value::Number(serde_json::Number::from(int_val));
        } else if let Some(n) = serde_json::Number::from_f64(double_val) {
            return serde_json::Value::Number(n);
        }
        return serde_json::Value::Null;
    }

    // Check for NSArray
    if class_name.contains("Array") {
        use objc2::msg_send;

        let count: usize = msg_send![obj, count];
        let mut arr = Vec::new();
        for i in 0..count {
            let item: *mut AnyObject = msg_send![obj, objectAtIndex: i];
            if !item.is_null() {
                arr.push(ns_object_to_json(&*item));
            }
        }
        return serde_json::Value::Array(arr);
    }

    // Check for NSDictionary
    if class_name.contains("Dictionary") {
        use objc2::msg_send;

        let keys: *mut AnyObject = msg_send![obj, allKeys];
        if keys.is_null() {
            return serde_json::Value::Object(serde_json::Map::new());
        }

        let count: usize = msg_send![keys, count];
        let mut map = serde_json::Map::new();

        for i in 0..count {
            let key: *mut AnyObject = msg_send![keys, objectAtIndex: i];
            if key.is_null() {
                continue;
            }

            let key_class = (&*key).class().name();
            if !key_class.contains("String") {
                continue;
            }

            let ns_key: &NSStr = &*(key as *const AnyObject as *const NSStr);
            let key_str = ns_key.to_string();

            let val: *mut AnyObject = msg_send![obj, objectForKey: key];
            if !val.is_null() {
                map.insert(key_str, ns_object_to_json(&*val));
            }
        }
        return serde_json::Value::Object(map);
    }

    // Check for NSNull
    if class_name.contains("Null") {
        return serde_json::Value::Null;
    }

    // Default
    serde_json::Value::Null
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

