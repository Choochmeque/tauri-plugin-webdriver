use std::sync::Arc;

use async_trait::async_trait;
use serde_json::Value;
use tauri::{Manager, Runtime, WebviewWindow};
use tokio::sync::oneshot;
use webview2_com::Microsoft::Web::WebView2::Win32::{
    ICoreWebView2, ICoreWebView2Deferral, ICoreWebView2ExecuteScriptCompletedHandler,
    ICoreWebView2ScriptDialogOpeningEventArgs, ICoreWebView2ScriptDialogOpeningEventHandler,
    ICoreWebView2WebMessageReceivedEventHandler, COREWEBVIEW2_SCRIPT_DIALOG_KIND_ALERT,
    COREWEBVIEW2_SCRIPT_DIALOG_KIND_CONFIRM, COREWEBVIEW2_SCRIPT_DIALOG_KIND_PROMPT,
};
use windows::core::{HSTRING, PCWSTR};
use windows::Win32::System::Com::{CoInitializeEx, COINIT_APARTMENTTHREADED};

use crate::platform::alert_state::{alert_state, AlertType, PendingAlert};
use crate::platform::async_state::{AsyncScriptState, HANDLER_NAME};
use crate::platform::{wrap_script_for_frame_context, FrameId, PlatformExecutor, PrintOptions};
use crate::server::response::WebDriverErrorResponse;
use crate::webdriver::Timeouts;

/// Wrapper for raw COM pointer to allow sending across threads.
/// SAFETY: The COM object must only be accessed from a COM-initialized thread.
struct SendableComPtr(*mut std::ffi::c_void);
unsafe impl Send for SendableComPtr {}

impl SendableComPtr {
    fn as_ptr(&self) -> *mut std::ffi::c_void {
        self.0
    }
}

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

/// Register WebView2 handlers at webview creation time.
/// This is called from the plugin's `on_webview_ready` hook to ensure
/// the script dialog handler is registered before any navigation completes.
pub fn register_webview_handlers<R: Runtime>(webview: &tauri::Webview<R>) {
    let _ = webview.with_webview(|webview| unsafe {
        let _ = CoInitializeEx(None, COINIT_APARTMENTTHREADED);

        if let Ok(webview2) = webview.controller().CoreWebView2() {
            register_script_dialog_handler(&webview2);
        }
    });
}

#[async_trait]
impl<R: Runtime + 'static> PlatformExecutor<R> for WindowsExecutor<R> {
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
        if alert_state().respond(false, None) {
            Ok(())
        } else {
            Err(WebDriverErrorResponse::no_such_alert())
        }
    }

    async fn accept_alert(&self) -> Result<(), WebDriverErrorResponse> {
        let prompt_text = alert_state()
            .get_prompt_input()
            .or_else(|| alert_state().get_default_text());
        if alert_state().respond(true, prompt_text) {
            Ok(())
        } else {
            Err(WebDriverErrorResponse::no_such_alert())
        }
    }

    async fn get_alert_text(&self) -> Result<String, WebDriverErrorResponse> {
        match alert_state().get_message() {
            Some(msg) => Ok(msg),
            None => Err(WebDriverErrorResponse::no_such_alert()),
        }
    }

    async fn send_alert_text(&self, text: &str) -> Result<(), WebDriverErrorResponse> {
        match alert_state().get_alert_type() {
            None => Err(WebDriverErrorResponse::no_such_alert()),
            Some(AlertType::Prompt) => {
                if alert_state().set_prompt_input(text.to_string()) {
                    Ok(())
                } else {
                    Err(WebDriverErrorResponse::no_such_alert())
                }
            }
            Some(_) => Err(WebDriverErrorResponse::element_not_interactable(
                "User prompt is not a prompt dialog",
            )),
        }
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

    // =========================================================================
    // Async Script Execution
    // =========================================================================

    async fn execute_async_script(
        &self,
        script: &str,
        args: &[Value],
    ) -> Result<Value, WebDriverErrorResponse> {
        let args_json = serde_json::to_string(args)
            .map_err(|e| WebDriverErrorResponse::invalid_argument(&e.to_string()))?;

        let async_id = uuid::Uuid::new_v4().to_string();

        // Get async state and register this operation
        let app = self.window.app_handle().clone();
        let async_state = app.state::<AsyncScriptState>();
        let label = self.window.label().to_string();

        // Register handler if not already registered for this window
        if !async_state.mark_handler_registered(&label) {
            let app_clone = app.clone();
            let handler_result = self.window.with_webview(move |webview| unsafe {
                let _ = CoInitializeEx(None, COINIT_APARTMENTTHREADED);

                if let Ok(webview2) = webview.controller().CoreWebView2() {
                    let state = app_clone.state::<AsyncScriptState>();
                    register_message_handler(&webview2, state.inner());
                }
            });

            if let Err(e) = handler_result {
                return Err(WebDriverErrorResponse::unknown_error(&format!(
                    "Failed to register message handler: {e}"
                )));
            }
        }

        let rx = async_state.register(async_id.clone());

        // Build wrapper script using postMessage
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
                var __done = function(r) {{
                    window.chrome.webview.postMessage(JSON.stringify({{
                        handler: '{HANDLER_NAME}',
                        id: '{async_id}',
                        result: r,
                        error: null
                    }}));
                }};
                var __args = {args_json}.map(deserializeArg);
                __args.push(__done);
                try {{
                    (function() {{ {script} }}).apply(null, __args);
                }} catch (e) {{
                    window.chrome.webview.postMessage(JSON.stringify({{
                        handler: '{HANDLER_NAME}',
                        id: '{async_id}',
                        result: null,
                        error: e.message || String(e)
                    }}));
                }}
            }})()"
        );

        // Execute the wrapper (returns immediately)
        self.evaluate_js(&wrapper).await?;

        // Wait for result with timeout
        let timeout_ms = self.timeouts.script_ms;
        let timeout = std::time::Duration::from_millis(timeout_ms);

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
        ICoreWebView2, ICoreWebView2CapturePreviewCompletedHandler,
        ICoreWebView2CapturePreviewCompletedHandler_Impl, ICoreWebView2Deferral,
        ICoreWebView2ExecuteScriptCompletedHandler,
        ICoreWebView2ExecuteScriptCompletedHandler_Impl, ICoreWebView2ScriptDialogOpeningEventArgs,
        ICoreWebView2ScriptDialogOpeningEventHandler,
        ICoreWebView2ScriptDialogOpeningEventHandler_Impl,
        ICoreWebView2WebMessageReceivedEventArgs, ICoreWebView2WebMessageReceivedEventHandler,
        ICoreWebView2WebMessageReceivedEventHandler_Impl, COREWEBVIEW2_SCRIPT_DIALOG_KIND_ALERT,
        COREWEBVIEW2_SCRIPT_DIALOG_KIND_CONFIRM, COREWEBVIEW2_SCRIPT_DIALOG_KIND_PROMPT,
    };
    use windows::core::{implement, Interface};

    use super::{
        alert_state, AlertType, AsyncScriptState, CaptureResultSender, PendingAlert,
        ScriptResultSender, SendableComPtr, HANDLER_NAME,
    };
    use crate::platform::alert_state::AlertResponse;

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

    /// Handler for receiving web messages from JavaScript via postMessage
    #[implement(ICoreWebView2WebMessageReceivedEventHandler)]
    pub struct WebMessageReceivedHandler {
        state_ptr: *const AsyncScriptState,
    }

    // SAFETY: The state pointer is valid for the lifetime of the app (managed by Tauri)
    unsafe impl Send for WebMessageReceivedHandler {}
    unsafe impl Sync for WebMessageReceivedHandler {}

    impl WebMessageReceivedHandler {
        pub fn new(state: &AsyncScriptState) -> Self {
            Self {
                state_ptr: state as *const AsyncScriptState,
            }
        }
    }

    impl ICoreWebView2WebMessageReceivedEventHandler_Impl for WebMessageReceivedHandler_Impl {
        fn Invoke(
            &self,
            _sender: windows::core::Ref<'_, ICoreWebView2>,
            args: windows::core::Ref<'_, ICoreWebView2WebMessageReceivedEventArgs>,
        ) -> windows::core::Result<()> {
            unsafe {
                let state_ptr = self.state_ptr;
                if state_ptr.is_null() {
                    tracing::error!("AsyncScriptState pointer is null");
                    return Ok(());
                }
                let state = &*state_ptr;

                // Get the message as JSON string
                let Some(args_owned) = args.clone() else {
                    return Ok(());
                };
                let mut msg_ptr = windows::core::PWSTR::null();
                if args_owned.WebMessageAsJson(&raw mut msg_ptr).is_err() {
                    return Ok(()); // Failed to get message
                }
                let msg_text = msg_ptr.to_string().unwrap_or_default();

                // `WebMessageAsJson` returns the message as a JSON value.
                // Since JS sends `JSON.stringify({...})`, the message is a string,
                // so we get a JSON-encoded string (with extra quotes).
                // First parse to get the inner string, then parse that as our object.
                let inner_str: String = match serde_json::from_str(&msg_text) {
                    Ok(s) => s,
                    Err(_) => return Ok(()), // Not a JSON string
                };
                let msg: Value = match serde_json::from_str(&inner_str) {
                    Ok(v) => v,
                    Err(_) => return Ok(()), // Not our message format
                };

                // Check if this is our handler
                let handler = msg.get("handler").and_then(Value::as_str);
                if handler != Some(HANDLER_NAME) {
                    return Ok(()); // Not for us
                }

                // Extract async ID
                let Some(async_id) = msg.get("id").and_then(Value::as_str) else {
                    tracing::warn!("Message missing 'id' field");
                    return Ok(());
                };
                let async_id = async_id.to_string();

                // Check for error
                if let Some(error) = msg.get("error").and_then(Value::as_str) {
                    if !error.is_empty() {
                        state.complete(&async_id, Err(error.to_string()));
                        return Ok(());
                    }
                }

                // Extract result
                let result = msg.get("result").cloned().unwrap_or(Value::Null);
                state.complete(&async_id, Ok(result));
            }
            Ok(())
        }
    }

    /// Handler for intercepting JavaScript alert/confirm/prompt dialogs
    #[implement(ICoreWebView2ScriptDialogOpeningEventHandler)]
    pub struct ScriptDialogOpeningHandler;

    impl ScriptDialogOpeningHandler {
        pub fn new() -> Self {
            Self
        }
    }

    impl ICoreWebView2ScriptDialogOpeningEventHandler_Impl for ScriptDialogOpeningHandler_Impl {
        fn Invoke(
            &self,
            _sender: windows::core::Ref<'_, ICoreWebView2>,
            args: windows::core::Ref<'_, ICoreWebView2ScriptDialogOpeningEventArgs>,
        ) -> windows::core::Result<()> {
            unsafe {
                let Some(args) = args.clone() else {
                    return Ok(());
                };

                // Get dialog kind
                let mut kind = std::mem::zeroed();
                if args.Kind(&raw mut kind).is_err() {
                    tracing::error!("Failed to get script dialog kind");
                    return Ok(());
                }

                // Get message
                let mut message_ptr = windows::core::PWSTR::null();
                if args.Message(&raw mut message_ptr).is_err() {
                    tracing::error!("Failed to get script dialog message");
                    return Ok(());
                }
                let message = message_ptr.to_string().unwrap_or_default();

                // Get default text for prompts
                let mut default_text_ptr = windows::core::PWSTR::null();
                let default_text = if args.DefaultText(&raw mut default_text_ptr).is_ok() {
                    let text = default_text_ptr.to_string().unwrap_or_default();
                    if text.is_empty() {
                        None
                    } else {
                        Some(text)
                    }
                } else {
                    None
                };

                // Map WebView2 dialog kind to our AlertType
                let alert_type = if kind == COREWEBVIEW2_SCRIPT_DIALOG_KIND_ALERT {
                    AlertType::Alert
                } else if kind == COREWEBVIEW2_SCRIPT_DIALOG_KIND_CONFIRM {
                    AlertType::Confirm
                } else if kind == COREWEBVIEW2_SCRIPT_DIALOG_KIND_PROMPT {
                    AlertType::Prompt
                } else {
                    // BEFOREUNLOAD or unknown - just accept it
                    let _ = args.Accept();
                    return Ok(());
                };

                tracing::debug!("Intercepted {:?} dialog: {}", alert_type, message);

                // Get deferral to handle asynchronously (avoid blocking UI thread)
                let deferral = match args.GetDeferral() {
                    Ok(d) => d,
                    Err(e) => {
                        tracing::error!("Failed to get deferral: {e:?}");
                        let _ = args.Accept();
                        return Ok(());
                    }
                };

                // Create channel for WebDriver response
                let (tx, rx) = std::sync::mpsc::channel::<AlertResponse>();
                alert_state().set_pending(PendingAlert {
                    message: message.clone(),
                    default_text: default_text.clone(),
                    alert_type,
                    responder: tx,
                });

                // Wrap COM objects for thread transfer
                let args_ptr = SendableComPtr(args.into_raw());
                let deferral_ptr = SendableComPtr(deferral.into_raw());

                // Spawn thread to wait for WebDriver response (don't block UI thread)
                std::thread::spawn(move || {
                    let timeout = std::time::Duration::from_secs(30);
                    let response = rx.recv_timeout(timeout);

                    // Reconstruct COM objects from raw pointers
                    // SAFETY: These pointers came from valid COM objects and we're
                    // accessing them from a single thread
                    let args = unsafe {
                        ICoreWebView2ScriptDialogOpeningEventArgs::from_raw(args_ptr.as_ptr())
                    };
                    let deferral =
                        unsafe { ICoreWebView2Deferral::from_raw(deferral_ptr.as_ptr()) };

                    match response {
                        Ok(AlertResponse {
                            accepted,
                            prompt_text,
                        }) => {
                            if accepted {
                                // Set prompt text if provided
                                if let Some(text) = prompt_text {
                                    let result = windows::core::HSTRING::from(text.as_str());
                                    let _ =
                                        args.SetResultText(windows::core::PCWSTR(result.as_ptr()));
                                }
                                let _ = args.Accept();
                            }
                            // If not accepted, don't call Accept() - dialog returns false/null
                        }
                        Err(_) => {
                            // Timeout - auto-accept
                            let _ = args.Accept();
                        }
                    }

                    // Complete the deferral to let WebView2 continue
                    let _ = deferral.Complete();
                });
            }
            Ok(())
        }
    }
}

use handlers::{
    CapturePreviewHandler, ExecuteScriptHandler, ScriptDialogOpeningHandler,
    WebMessageReceivedHandler,
};

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

// =============================================================================
// Native Message Handler Registration
// =============================================================================

/// Register the `WebMessage` handler for a webview.
///
/// # Safety
/// Must be called from a COM-initialized thread with a valid webview.
unsafe fn register_message_handler(webview: &ICoreWebView2, state: &AsyncScriptState) {
    let handler: ICoreWebView2WebMessageReceivedEventHandler =
        WebMessageReceivedHandler::new(state).into();

    // We don't need to store the token since we never remove the handler
    let mut token = std::mem::zeroed();
    if let Err(e) = webview.add_WebMessageReceived(&handler, &raw mut token) {
        tracing::error!("Failed to register WebMessageReceived handler: {e:?}");
    } else {
        tracing::debug!("Registered native message handler for webview");
    }
}

/// Register the `ScriptDialogOpening` handler for a webview.
///
/// This intercepts JavaScript alert/confirm/prompt dialogs and stores them
/// for `WebDriver` to handle.
///
/// # Safety
/// Must be called from a COM-initialized thread with a valid webview.
unsafe fn register_script_dialog_handler(webview: &ICoreWebView2) {
    // Disable default script dialogs so ScriptDialogOpening event fires
    if let Ok(settings) = webview.Settings() {
        if let Err(e) = settings.SetAreDefaultScriptDialogsEnabled(false) {
            tracing::error!("Failed to disable default script dialogs: {e:?}");
            return;
        }
    } else {
        tracing::error!("Failed to get webview settings");
        return;
    }

    let handler: ICoreWebView2ScriptDialogOpeningEventHandler =
        ScriptDialogOpeningHandler::new().into();

    let mut token = std::mem::zeroed();
    if let Err(e) = webview.add_ScriptDialogOpening(&handler, &raw mut token) {
        tracing::error!("Failed to register ScriptDialogOpening handler: {e:?}");
    } else {
        tracing::debug!("Registered script dialog handler for webview");
    }

    // Prevent handler from being dropped - leak it to keep the COM ref alive
    std::mem::forget(handler);
}
