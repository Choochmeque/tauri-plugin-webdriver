use objc2::rc::Retained;
use objc2::runtime::ProtocolObject;
use objc2::{define_class, msg_send, DefinedClass, MainThreadOnly};
use objc2_foundation::{NSObject, NSObjectProtocol, NSString};
use objc2_web_kit::{WKScriptMessage, WKScriptMessageHandler, WKUserContentController, WKWebView};
use serde_json::Value;

use super::async_state::{AsyncScriptState, HANDLER_NAME};
use super::macos::ns_object_to_json;

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

                let id_value: *mut objc2::runtime::AnyObject = msg_send![&*body, objectForKey: &*id_key];
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
                let error_value: *mut objc2::runtime::AnyObject = msg_send![&*body, objectForKey: &*error_key];
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
                let result_value: *mut objc2::runtime::AnyObject = msg_send![&*body, objectForKey: &*result_key];
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
    fn new(mtm: objc2::MainThreadMarker, state: &AsyncScriptState) -> Retained<Self> {
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
/// The state must outlive the webview (which is guaranteed when using Tauri's managed state).
pub unsafe fn register_handler(webview: &WKWebView, state: &AsyncScriptState) {
    let config = webview.configuration();
    let controller = config.userContentController();

    // SAFETY: We're running on the main thread (within with_webview callback)
    let mtm = objc2::MainThreadMarker::new_unchecked();
    let handler = WebDriverMessageHandler::new(mtm, state);
    let handler_protocol = ProtocolObject::from_retained(handler);
    let name = NSString::from_str(HANDLER_NAME);

    controller.addScriptMessageHandler_name(&handler_protocol, &name);

    tracing::debug!("Registered native message handler for webview");
}
