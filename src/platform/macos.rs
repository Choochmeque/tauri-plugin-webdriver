use std::sync::Arc;

use async_trait::async_trait;
use base64::engine::general_purpose::STANDARD as BASE64_STANDARD;
use base64::Engine as _;
use block2::RcBlock;
use objc2::rc::Retained;
use objc2::runtime::{AnyObject, ProtocolObject};
use objc2::{define_class, msg_send, DefinedClass, MainThreadMarker, MainThreadOnly};
use objc2_app_kit::{NSBitmapImageFileType, NSBitmapImageRep, NSImage};
use objc2_foundation::{NSData, NSDictionary, NSError, NSObject, NSObjectProtocol, NSString};
use objc2_web_kit::{
    WKScriptMessage, WKScriptMessageHandler, WKSnapshotConfiguration, WKUserContentController,
    WKWebView,
};
use serde_json::Value;
use tauri::{Manager, Runtime, WebviewWindow};
use tokio::sync::oneshot;

use crate::platform::async_state::{AsyncScriptState, HANDLER_NAME};
use crate::platform::{wrap_script_for_frame_context, FrameId, PlatformExecutor, PrintOptions};
use crate::server::response::WebDriverErrorResponse;
use crate::webdriver::Timeouts;

/// macOS `WebView` executor using `WKWebView` native APIs
#[derive(Clone)]
pub struct MacOSExecutor<R: Runtime> {
    window: WebviewWindow<R>,
    timeouts: Timeouts,
    frame_context: Vec<FrameId>,
}

impl<R: Runtime> MacOSExecutor<R> {
    pub fn new(window: WebviewWindow<R>, timeouts: Timeouts, frame_context: Vec<FrameId>) -> Self {
        Self {
            window,
            timeouts,
            frame_context,
        }
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
            Ok(Err(_)) => Err(WebDriverErrorResponse::unknown_error("Channel closed")),
            Err(_) => Err(WebDriverErrorResponse::script_timeout()),
        }
    }

    // =========================================================================
    // Async Script Execution (Native Handler)
    // =========================================================================

    async fn execute_async_script(
        &self,
        script: &str,
        args: &[Value],
    ) -> Result<Value, WebDriverErrorResponse> {
        let args_json = serde_json::to_string(args)
            .map_err(|e| WebDriverErrorResponse::invalid_argument(&e.to_string()))?;

        let async_id = uuid::Uuid::new_v4().to_string();

        // Get async state from Tauri's managed state
        let app = self.window.app_handle().clone();
        let async_state = app.state::<AsyncScriptState>();

        // Register native message handler if not already registered for this window
        let label = self.window.label();
        if !async_state.mark_handler_registered(label) {
            let app_clone = app.clone();
            let handler_result = self.window.with_webview(move |webview| unsafe {
                let wk_webview: &WKWebView = &*webview.inner().cast();
                let state = app_clone.state::<AsyncScriptState>();
                register_handler(wk_webview, state.inner());
            });

            if let Err(e) = handler_result {
                return Err(WebDriverErrorResponse::unknown_error(&format!(
                    "Failed to register message handler: {e}"
                )));
            }
        }

        // Register pending operation
        let rx = async_state.register(async_id.clone());

        // Build wrapper with native postMessage
        let wrapper = format!(
            r#"(function() {{
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
                var __done = function(r) {{
                    window.webkit.messageHandlers.{HANDLER_NAME}.postMessage({{
                        id: '{async_id}',
                        result: r,
                        error: null
                    }});
                }};
                var __args = {args_json}.map(deserializeArg);
                __args.push(__done);
                try {{
                    (function() {{ {script} }}).apply(null, __args);
                }} catch (e) {{
                    window.webkit.messageHandlers.{HANDLER_NAME}.postMessage({{
                        id: '{async_id}',
                        result: null,
                        error: e.message || String(e)
                    }});
                }}
            }})()"#
        );

        // Execute the wrapper (returns immediately)
        self.evaluate_js(&wrapper).await?;

        // Wait for result with timeout
        let timeout = std::time::Duration::from_millis(self.timeouts.script_ms);

        match tokio::time::timeout(timeout, rx).await {
            Ok(Ok(Ok(value))) => Ok(value),
            Ok(Ok(Err(error))) => Err(WebDriverErrorResponse::javascript_error(&error, None)),
            Ok(Err(_)) => Err(WebDriverErrorResponse::unknown_error("Channel closed")),
            Err(_) => {
                async_state.cancel(&async_id);
                Err(WebDriverErrorResponse::script_timeout())
            }
        }
    }

    // =========================================================================
    // Screenshots
    // =========================================================================

    async fn take_screenshot(&self) -> Result<String, WebDriverErrorResponse> {
        let (tx, rx) = oneshot::channel();

        let result = self.window.with_webview(move |webview| unsafe {
            let wk_webview: &WKWebView = &*webview.inner().cast();
            let mtm = MainThreadMarker::new_unchecked();
            let config = WKSnapshotConfiguration::new(mtm);

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
                let mtm = MainThreadMarker::new_unchecked();
                let config = WKSnapshotConfiguration::new(mtm);

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

    let bytes = png_data.to_vec();
    Ok(BASE64_STANDARD.encode(&bytes))
}

/// Convert an `NSObject` to a JSON value
pub(super) unsafe fn ns_object_to_json(obj: &AnyObject) -> Value {
    use objc2_foundation::NSString as NSStr;

    let class = obj.class();
    let class_name = class.name().to_str().unwrap_or("");

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

            let key_class = (&*key).class().name().to_str().unwrap_or("");
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

// =============================================================================
// Native Message Handler for Async Scripts
// =============================================================================

/// Instance variables for our message handler - stores pointer to AsyncScriptState
struct WebDriverMessageHandlerIvars {
    state_ptr: *const AsyncScriptState,
}

// SAFETY: The state pointer is valid for the lifetime of the app (managed by Tauri)
unsafe impl Send for WebDriverMessageHandlerIvars {}
unsafe impl Sync for WebDriverMessageHandlerIvars {}

define_class!(
    #[unsafe(super(NSObject))]
    #[thread_kind = MainThreadOnly]
    #[name = "WebDriverMessageHandler"]
    #[ivars = WebDriverMessageHandlerIvars]
    struct WebDriverMessageHandler;

    unsafe impl NSObjectProtocol for WebDriverMessageHandler {}

    unsafe impl WKScriptMessageHandler for WebDriverMessageHandler {
        #[unsafe(method(userContentController:didReceiveScriptMessage:))]
        fn user_content_controller_did_receive_script_message(
            &self,
            _controller: &WKUserContentController,
            message: &WKScriptMessage,
        ) {
            unsafe {
                let state_ptr = self.ivars().state_ptr;
                if state_ptr.is_null() {
                    tracing::error!("AsyncScriptState pointer is null");
                    return;
                }
                let state = &*state_ptr;

                let body = message.body();
                let class_name = body.class().name().to_str().unwrap_or("");

                if !class_name.contains("Dictionary") {
                    tracing::warn!("Unexpected message body type: {}", class_name);
                    return;
                }

                // Extract values from NSDictionary
                let id_key = NSString::from_str("id");
                let result_key = NSString::from_str("result");
                let error_key = NSString::from_str("error");

                let id_value: *mut AnyObject = msg_send![&*body, objectForKey: &*id_key];
                if id_value.is_null() {
                    tracing::warn!("Message missing 'id' field");
                    return;
                }

                let id_class = (*id_value).class().name().to_str().unwrap_or("");
                if !id_class.contains("String") {
                    tracing::warn!("Message 'id' is not a string");
                    return;
                }

                let id_ns: &NSString = &*id_value.cast::<NSString>();
                let async_id = id_ns.to_string();

                // Check for error
                let error_value: *mut AnyObject = msg_send![&*body, objectForKey: &*error_key];
                if !error_value.is_null() {
                    let error_class = (*error_value).class().name().to_str().unwrap_or("");
                    if error_class.contains("String") {
                        let error_ns: &NSString = &*error_value.cast::<NSString>();
                        let error_str = error_ns.to_string();
                        if !error_str.is_empty() {
                            state.complete(&async_id, Err(error_str));
                            return;
                        }
                    }
                }

                // Extract result
                let result_value: *mut AnyObject = msg_send![&*body, objectForKey: &*result_key];
                let json_result = if result_value.is_null() {
                    Value::Null
                } else {
                    ns_object_to_json(&*result_value)
                };

                state.complete(&async_id, Ok(json_result));
            }
        }
    }
);

impl WebDriverMessageHandler {
    fn new(mtm: MainThreadMarker, state: &AsyncScriptState) -> Retained<Self> {
        let this = Self::alloc(mtm);
        let this = this.set_ivars(WebDriverMessageHandlerIvars {
            state_ptr: state as *const AsyncScriptState,
        });
        unsafe { msg_send![super(this), init] }
    }
}

/// Register the message handler for a webview.
///
/// # Safety
/// Must be called on the main thread with a valid webview and state reference.
unsafe fn register_handler(webview: &WKWebView, state: &AsyncScriptState) {
    let config = webview.configuration();
    let controller = config.userContentController();

    let mtm = MainThreadMarker::new_unchecked();
    let handler = WebDriverMessageHandler::new(mtm, state);
    let handler_protocol = ProtocolObject::from_retained(handler);
    let name = NSString::from_str(HANDLER_NAME);

    controller.addScriptMessageHandler_name(&handler_protocol, &name);

    tracing::debug!("Registered native message handler for webview");
}
