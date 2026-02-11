use std::fmt::Write;
use std::sync::Arc;

use async_trait::async_trait;
use base64::engine::general_purpose::STANDARD as BASE64_STANDARD;
use base64::Engine as _;
use block2::RcBlock;
use objc2::runtime::AnyObject;
use objc2_app_kit::{NSBitmapImageFileType, NSBitmapImageRep, NSImage};
use objc2_foundation::{NSData, NSDictionary, NSError, NSString};
use objc2_web_kit::{WKSnapshotConfiguration, WKWebView};
use serde_json::Value;
use tauri::{Runtime, WebviewWindow};
use tokio::sync::oneshot;

use crate::platform::{
    Cookie, FrameId, PlatformExecutor, PointerEventType, PrintOptions, WindowRect,
};
use crate::server::response::WebDriverErrorResponse;
use crate::webdriver::Timeouts;

/// macOS `WebView` executor using `WKWebView` native APIs
#[derive(Clone)]
pub struct MacOSExecutor<R: Runtime> {
    window: WebviewWindow<R>,
    timeouts: Timeouts,
}

impl<R: Runtime> MacOSExecutor<R> {
    pub fn new(window: WebviewWindow<R>, timeouts: Timeouts) -> Self {
        Self { window, timeouts }
    }
}

#[async_trait]
impl<R: Runtime + 'static> PlatformExecutor for MacOSExecutor<R> {
    // =========================================================================
    // Core JavaScript Execution
    // =========================================================================

    async fn evaluate_js(&self, script: &str) -> Result<Value, WebDriverErrorResponse> {
        let (tx, rx) = oneshot::channel();
        let script_owned = script.to_string();

        let result = self.window.with_webview(move |webview| unsafe {
            let wk_webview: &WKWebView = &*webview.inner().cast();
            let ns_script = NSString::from_str(&script_owned);

            let tx = Arc::new(std::sync::Mutex::new(Some(tx)));
            let block = RcBlock::new(move |result: *mut AnyObject, error: *mut NSError| {
                let response = if !error.is_null() {
                    let error_ref = &*error;
                    let description = error_ref.localizedDescription();
                    Err(description.to_string())
                } else if result.is_null() {
                    Ok(Value::Null)
                } else {
                    let obj = &*result;
                    Ok(ns_object_to_json(obj))
                };

                if let Ok(mut guard) = tx.lock() {
                    if let Some(tx) = guard.take() {
                        let _ = tx.send(response);
                    }
                }
            });

            wk_webview.evaluateJavaScript_completionHandler(&ns_script, Some(&block));
        });

        if let Err(e) = result {
            return Err(WebDriverErrorResponse::javascript_error(&e.to_string()));
        }

        let timeout = std::time::Duration::from_millis(self.timeouts.script_ms);
        match tokio::time::timeout(timeout, rx).await {
            Ok(Ok(Ok(value))) => Ok(serde_json::json!({
                "success": true,
                "value": value
            })),
            Ok(Ok(Err(error))) => Err(WebDriverErrorResponse::javascript_error(&error)),
            Ok(Err(_)) => Err(WebDriverErrorResponse::javascript_error("Channel closed")),
            Err(_) => Err(WebDriverErrorResponse::script_timeout()),
        }
    }

    // =========================================================================
    // Document
    // =========================================================================

    async fn get_source(&self) -> Result<String, WebDriverErrorResponse> {
        let result = self
            .evaluate_js("document.documentElement.outerHTML")
            .await?;
        extract_string_value(&result)
    }

    // =========================================================================
    // Element Operations
    // =========================================================================

    // =========================================================================
    // Shadow DOM
    // =========================================================================

    // =========================================================================
    // Script Execution
    // =========================================================================

    async fn execute_async_script(
        &self,
        script: &str,
        args: &[Value],
    ) -> Result<Value, WebDriverErrorResponse> {
        let args_json = serde_json::to_string(args)
            .map_err(|e| WebDriverErrorResponse::invalid_argument(&e.to_string()))?;

        let wrapper = format!(
            r"new Promise(function(resolve, reject) {{
                try {{
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
                    var args = {args_json}.map(deserializeArg);
                    args.push(function(result) {{ resolve(result); }});
                    var fn = function() {{ {script} }};
                    fn.apply(null, args);
                }} catch (e) {{
                    reject(e);
                }}
            }})"
        );

        let result = self.evaluate_js(&wrapper).await?;

        if let Some(success) = result.get("success").and_then(Value::as_bool) {
            if success {
                return Ok(result.get("value").cloned().unwrap_or(Value::Null));
            } else if let Some(error) = result.get("error").and_then(Value::as_str) {
                return Err(WebDriverErrorResponse::javascript_error(error));
            }
        }

        Ok(Value::Null)
    }

    // =========================================================================
    // Screenshots
    // =========================================================================

    async fn take_screenshot(&self) -> Result<String, WebDriverErrorResponse> {
        let (tx, rx) = oneshot::channel();

        let result = self.window.with_webview(move |webview| unsafe {
            let wk_webview: &WKWebView = &*webview.inner().cast();
            let config = WKSnapshotConfiguration::new();

            let tx = Arc::new(std::sync::Mutex::new(Some(tx)));
            let block = RcBlock::new(move |image: *mut NSImage, error: *mut NSError| {
                let response = if !error.is_null() {
                    let error_ref = &*error;
                    let description = error_ref.localizedDescription();
                    Err(description.to_string())
                } else if image.is_null() {
                    Err("No image returned".to_string())
                } else {
                    let image_ref = &*image;
                    image_to_png_base64(image_ref)
                };

                if let Ok(mut guard) = tx.lock() {
                    if let Some(tx) = guard.take() {
                        let _ = tx.send(response);
                    }
                }
            });

            wk_webview.takeSnapshotWithConfiguration_completionHandler(Some(&config), &block);
        });

        if let Err(e) = result {
            return Err(WebDriverErrorResponse::unknown_error(&e.to_string()));
        }

        let timeout = std::time::Duration::from_millis(self.timeouts.script_ms);
        match tokio::time::timeout(timeout, rx).await {
            Ok(Ok(Ok(base64))) => Ok(base64),
            Ok(Ok(Err(error))) => Err(WebDriverErrorResponse::unknown_error(&error)),
            Ok(Err(_)) => Err(WebDriverErrorResponse::unknown_error("Channel closed")),
            Err(_) => Err(WebDriverErrorResponse::script_timeout()),
        }
    }

    async fn take_element_screenshot(
        &self,
        js_var: &str,
    ) -> Result<String, WebDriverErrorResponse> {
        // For element screenshots, we use JavaScript canvas approach
        let script = format!(
            r"(function() {{
                var el = window.{js_var};
                if (!el || !document.contains(el)) {{
                    throw new Error('stale element reference');
                }}

                // Use html2canvas-like approach if available, otherwise scroll into view
                el.scrollIntoView({{ block: 'center', inline: 'center' }});

                // Return element bounds for clipping
                var rect = el.getBoundingClientRect();
                return {{
                    x: rect.x,
                    y: rect.y,
                    width: rect.width,
                    height: rect.height
                }};
            }})()"
        );
        self.evaluate_js(&script).await?;

        // For now, take full screenshot - element clipping can be done in Phase 4
        // with proper WKSnapshotConfiguration rect clipping
        let (tx, rx) = oneshot::channel();

        let result = self.window.with_webview(move |webview| {
            unsafe {
                let wk_webview: &WKWebView = &*webview.inner().cast();
                let config = WKSnapshotConfiguration::new();

                // Set clip rect for element
                // Note: WKSnapshotConfiguration has afterScreenUpdates and rect properties
                // We'd set config.setRect(CGRect) here for proper element clipping

                let tx = Arc::new(std::sync::Mutex::new(Some(tx)));
                let block = RcBlock::new(move |image: *mut NSImage, error: *mut NSError| {
                    let response = if !error.is_null() {
                        let error_ref = &*error;
                        let description = error_ref.localizedDescription();
                        Err(description.to_string())
                    } else if image.is_null() {
                        Err("No image returned".to_string())
                    } else {
                        let image_ref = &*image;
                        image_to_png_base64(image_ref)
                    };

                    if let Ok(mut guard) = tx.lock() {
                        if let Some(tx) = guard.take() {
                            let _ = tx.send(response);
                        }
                    }
                });

                wk_webview.takeSnapshotWithConfiguration_completionHandler(Some(&config), &block);
            }
        });

        if let Err(e) = result {
            return Err(WebDriverErrorResponse::unknown_error(&e.to_string()));
        }

        let timeout = std::time::Duration::from_millis(self.timeouts.script_ms);
        match tokio::time::timeout(timeout, rx).await {
            Ok(Ok(Ok(base64))) => Ok(base64),
            Ok(Ok(Err(error))) => Err(WebDriverErrorResponse::unknown_error(&error)),
            Ok(Err(_)) => Err(WebDriverErrorResponse::unknown_error("Channel closed")),
            Err(_) => Err(WebDriverErrorResponse::script_timeout()),
        }
    }

    // =========================================================================
    // Actions (Keyboard/Pointer)
    // =========================================================================

    async fn dispatch_key_event(
        &self,
        key: &str,
        is_down: bool,
    ) -> Result<(), WebDriverErrorResponse> {
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
            "\u{E031}" => ("F1", "F1", 112),
            "\u{E032}" => ("F2", "F2", 113),
            "\u{E033}" => ("F3", "F3", 114),
            "\u{E034}" => ("F4", "F4", 115),
            "\u{E035}" => ("F5", "F5", 116),
            "\u{E036}" => ("F6", "F6", 117),
            "\u{E037}" => ("F7", "F7", 118),
            "\u{E038}" => ("F8", "F8", 119),
            "\u{E039}" => ("F9", "F9", 120),
            "\u{E03A}" => ("F10", "F10", 121),
            "\u{E03B}" => ("F11", "F11", 122),
            "\u{E03C}" => ("F12", "F12", 123),
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
                return self.dispatch_regular_key(key, &code, is_down).await;
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

    async fn dispatch_pointer_event(
        &self,
        event_type: PointerEventType,
        x: i32,
        y: i32,
        button: u32,
    ) -> Result<(), WebDriverErrorResponse> {
        let event_name = match event_type {
            PointerEventType::Down => "mousedown",
            PointerEventType::Up => "mouseup",
            PointerEventType::Move => "mousemove",
        };

        let buttons = if matches!(event_type, PointerEventType::Down) {
            1 << button
        } else {
            0
        };
        let script = format!(
            r"(function() {{
                var el = document.elementFromPoint({x}, {y});
                if (!el) el = document.body;

                var event = new MouseEvent('{event_name}', {{
                    bubbles: true,
                    cancelable: true,
                    clientX: {x},
                    clientY: {y},
                    button: {button},
                    buttons: {buttons}
                }});
                el.dispatchEvent(event);
                return true;
            }})()"
        );

        self.evaluate_js(&script).await?;
        Ok(())
    }

    async fn dispatch_scroll_event(
        &self,
        x: i32,
        y: i32,
        delta_x: i32,
        delta_y: i32,
    ) -> Result<(), WebDriverErrorResponse> {
        let script = format!(
            r"(function() {{
                var el = document.elementFromPoint({x}, {y});
                if (!el) el = document.body;

                var event = new WheelEvent('wheel', {{
                    bubbles: true,
                    cancelable: true,
                    clientX: {x},
                    clientY: {y},
                    deltaX: {delta_x},
                    deltaY: {delta_y},
                    deltaMode: 0
                }});
                el.dispatchEvent(event);

                // Also perform actual scroll
                window.scrollBy({delta_x}, {delta_y});
                return true;
            }})()"
        );

        self.evaluate_js(&script).await?;
        Ok(())
    }

    // =========================================================================
    // Window Management
    // =========================================================================

    async fn get_window_rect(&self) -> Result<WindowRect, WebDriverErrorResponse> {
        if let Ok(position) = self.window.outer_position() {
            if let Ok(size) = self.window.outer_size() {
                return Ok(WindowRect {
                    x: position.x,
                    y: position.y,
                    width: size.width,
                    height: size.height,
                });
            }
        }
        Ok(WindowRect::default())
    }

    async fn set_window_rect(
        &self,
        rect: WindowRect,
    ) -> Result<WindowRect, WebDriverErrorResponse> {
        use tauri::{PhysicalPosition, PhysicalSize};

        let _ = self
            .window
            .set_position(PhysicalPosition::new(rect.x, rect.y));
        let _ = self
            .window
            .set_size(PhysicalSize::new(rect.width, rect.height));

        self.get_window_rect().await
    }

    async fn maximize_window(&self) -> Result<WindowRect, WebDriverErrorResponse> {
        let _ = self.window.maximize();
        // Give it a moment to maximize
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        self.get_window_rect().await
    }

    async fn minimize_window(&self) -> Result<(), WebDriverErrorResponse> {
        let _ = self.window.minimize();
        Ok(())
    }

    async fn fullscreen_window(&self) -> Result<WindowRect, WebDriverErrorResponse> {
        let _ = self.window.set_fullscreen(true);
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        self.get_window_rect().await
    }

    // =========================================================================
    // Frames
    // =========================================================================

    async fn switch_to_frame(&self, id: FrameId) -> Result<(), WebDriverErrorResponse> {
        match id {
            FrameId::Top => {
                // Switch back to top-level context
                // TODO: This is a no-op for now as we don't track frame context
                Ok(())
            }
            FrameId::Index(index) => {
                let script = format!(
                    r"(function() {{
                        var frames = document.querySelectorAll('iframe, frame');
                        if ({index} >= frames.length) {{
                            throw new Error('no such frame');
                        }}
                        return true;
                    }})()"
                );
                self.evaluate_js(&script).await?;
                Ok(())
            }
            FrameId::Element(js_var) => {
                let script = format!(
                    r"(function() {{
                        var el = window.{js_var};
                        if (!el || !document.contains(el)) {{
                            throw new Error('stale element reference');
                        }}
                        if (el.tagName !== 'IFRAME' && el.tagName !== 'FRAME') {{
                            throw new Error('element is not a frame');
                        }}
                        return true;
                    }})()"
                );
                self.evaluate_js(&script).await?;
                Ok(())
            }
        }
    }

    async fn switch_to_parent_frame(&self) -> Result<(), WebDriverErrorResponse> {
        // TODO: No-op for now - frame context tracking would be needed
        Ok(())
    }

    // =========================================================================
    // Cookies
    // =========================================================================

    async fn get_all_cookies(&self) -> Result<Vec<Cookie>, WebDriverErrorResponse> {
        let script = r"(function() {
            var cookies = document.cookie.split(';');
            var result = [];
            for (var i = 0; i < cookies.length; i++) {
                var cookie = cookies[i].trim();
                if (cookie) {
                    var eqIndex = cookie.indexOf('=');
                    if (eqIndex > 0) {
                        result.push({
                            name: cookie.substring(0, eqIndex),
                            value: cookie.substring(eqIndex + 1)
                        });
                    }
                }
            }
            return result;
        })()";

        let result = self.evaluate_js(script).await?;

        if let Some(value) = result.get("value") {
            if let Some(arr) = value.as_array() {
                let cookies: Vec<Cookie> = arr
                    .iter()
                    .filter_map(|v| serde_json::from_value(v.clone()).ok())
                    .collect();
                return Ok(cookies);
            }
        }
        Ok(vec![])
    }

    async fn get_cookie(&self, name: &str) -> Result<Option<Cookie>, WebDriverErrorResponse> {
        let cookies = self.get_all_cookies().await?;
        Ok(cookies.into_iter().find(|c| c.name == name))
    }

    async fn add_cookie(&self, cookie: Cookie) -> Result<(), WebDriverErrorResponse> {
        let mut cookie_str = format!("{}={}", cookie.name, cookie.value);

        if let Some(path) = &cookie.path {
            let _ = write!(cookie_str, "; path={path}");
        }
        if let Some(domain) = &cookie.domain {
            let _ = write!(cookie_str, "; domain={domain}");
        }
        if cookie.secure {
            cookie_str.push_str("; secure");
        }
        if cookie.http_only {
            cookie_str.push_str("; httponly");
        }
        if let Some(expiry) = cookie.expiry {
            let _ = write!(cookie_str, "; expires={expiry}");
        }
        if let Some(same_site) = &cookie.same_site {
            let _ = write!(cookie_str, "; samesite={same_site}");
        }

        let escaped = cookie_str.replace('\'', "\\'");
        let script = format!(r"document.cookie = '{escaped}'; true");
        self.evaluate_js(&script).await?;
        Ok(())
    }

    async fn delete_cookie(&self, name: &str) -> Result<(), WebDriverErrorResponse> {
        let script = format!(
            r"document.cookie = '{}=; expires=Thu, 01 Jan 1970 00:00:00 GMT; path=/'; true",
            name.replace('\'', "\\'")
        );
        self.evaluate_js(&script).await?;
        Ok(())
    }

    async fn delete_all_cookies(&self) -> Result<(), WebDriverErrorResponse> {
        let cookies = self.get_all_cookies().await?;
        for cookie in cookies {
            self.delete_cookie(&cookie.name).await?;
        }
        Ok(())
    }

    // =========================================================================
    // Alerts
    // =========================================================================

    async fn dismiss_alert(&self) -> Result<(), WebDriverErrorResponse> {
        // TODO: Implement native alert handling with WKUIDelegate
        Err(WebDriverErrorResponse::unknown_error(
            "Alert handling not yet implemented - requires WKUIDelegate setup",
        ))
    }

    async fn accept_alert(&self) -> Result<(), WebDriverErrorResponse> {
        // TODO: Implement native alert handling with WKUIDelegate
        Err(WebDriverErrorResponse::unknown_error(
            "Alert handling not yet implemented - requires WKUIDelegate setup",
        ))
    }

    async fn get_alert_text(&self) -> Result<String, WebDriverErrorResponse> {
        // TODO: Implement native alert handling with WKUIDelegate
        Err(WebDriverErrorResponse::unknown_error(
            "Alert handling not yet implemented - requires WKUIDelegate setup",
        ))
    }

    async fn send_alert_text(&self, _text: &str) -> Result<(), WebDriverErrorResponse> {
        // TODO: Implement native alert handling with WKUIDelegate
        Err(WebDriverErrorResponse::unknown_error(
            "Alert handling not yet implemented - requires WKUIDelegate setup",
        ))
    }

    // =========================================================================
    // Print
    // =========================================================================

    async fn print_page(&self, _options: PrintOptions) -> Result<String, WebDriverErrorResponse> {
        // TODO: Implement PDF printing with WKWebView's createPDFWithConfiguration
        Err(WebDriverErrorResponse::unknown_error(
            "PDF printing not yet implemented",
        ))
    }
}

// =============================================================================
// Helper Methods
// =============================================================================

impl<R: Runtime + 'static> MacOSExecutor<R> {
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
            r"(function() {{
                var event = new KeyboardEvent('{event_type}', {{
                    key: '{escaped_key}',
                    code: '{escaped_code}',
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
}

// =============================================================================
// Utility Functions
// =============================================================================

/// Convert `NSImage` to PNG and encode as base64
unsafe fn image_to_png_base64(image: &NSImage) -> Result<String, String> {
    let tiff_data: Option<objc2::rc::Retained<NSData>> = image.TIFFRepresentation();
    let tiff_data = tiff_data.ok_or("Failed to get TIFF representation")?;

    let bitmap_rep = NSBitmapImageRep::imageRepWithData(&tiff_data)
        .ok_or("Failed to create bitmap image rep")?;

    let empty_dict: objc2::rc::Retained<NSDictionary<NSString>> = NSDictionary::new();
    let png_data: Option<objc2::rc::Retained<NSData>> =
        bitmap_rep.representationUsingType_properties(NSBitmapImageFileType::PNG, &empty_dict);
    let png_data = png_data.ok_or("Failed to convert to PNG")?;

    let bytes = png_data.bytes();
    Ok(BASE64_STANDARD.encode(bytes))
}

/// Convert an `NSObject` to a JSON value
unsafe fn ns_object_to_json(obj: &AnyObject) -> Value {
    use objc2_foundation::NSString as NSStr;

    let class = obj.class();
    let class_name = class.name();

    if class_name.contains("String") {
        let ns_str: &NSStr = &*std::ptr::from_ref::<AnyObject>(obj).cast::<NSStr>();
        return Value::String(ns_str.to_string());
    }

    if class_name.contains("Number") || class_name.contains("Boolean") {
        use objc2::msg_send;
        use objc2::runtime::Bool;

        if class_name.contains("Boolean") {
            let bool_val: Bool = msg_send![obj, boolValue];
            return Value::Bool(bool_val.as_bool());
        }

        let double_val: f64 = msg_send![obj, doubleValue];
        let int_val: i64 = msg_send![obj, longLongValue];

        #[allow(clippy::cast_precision_loss)]
        if (int_val as f64 - double_val).abs() < f64::EPSILON {
            return Value::Number(serde_json::Number::from(int_val));
        } else if let Some(n) = serde_json::Number::from_f64(double_val) {
            return Value::Number(n);
        }
        return Value::Null;
    }

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
        return Value::Array(arr);
    }

    if class_name.contains("Dictionary") {
        use objc2::msg_send;

        let keys: *mut AnyObject = msg_send![obj, allKeys];
        if keys.is_null() {
            return Value::Object(serde_json::Map::new());
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

            let ns_key: &NSStr = &*key.cast_const().cast::<NSStr>();
            let key_str = ns_key.to_string();

            let val: *mut AnyObject = msg_send![obj, objectForKey: key];
            if !val.is_null() {
                map.insert(key_str, ns_object_to_json(&*val));
            }
        }
        return Value::Object(map);
    }

    if class_name.contains("Null") {
        return Value::Null;
    }

    Value::Null
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
            return Err(WebDriverErrorResponse::javascript_error(error));
        }
    }
    Ok(String::new())
}
