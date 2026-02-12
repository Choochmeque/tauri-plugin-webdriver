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

use crate::platform::{PlatformExecutor, PrintOptions};
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
impl<R: Runtime + 'static> PlatformExecutor<R> for MacOSExecutor<R> {
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
            Ok(Err(_)) => Err(WebDriverErrorResponse::javascript_error(
                "Channel closed",
                None,
            )),
            Err(_) => Err(WebDriverErrorResponse::script_timeout()),
        }
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
