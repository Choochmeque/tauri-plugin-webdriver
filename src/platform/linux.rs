use std::sync::Arc;

use async_trait::async_trait;
use base64::engine::general_purpose::STANDARD as BASE64_STANDARD;
use base64::Engine as _;
use glib::MainContext;
use javascriptcore::ValueExt;
use serde_json::Value;
use tauri::{Manager, Runtime, WebviewWindow};
use tokio::sync::oneshot;
use webkit2gtk::{PrintOperationExt, ScriptDialogType, UserContentManagerExt, WebViewExt};

use crate::platform::alert_state::{AlertStateManager, AlertType, PendingAlert};
use crate::platform::async_state::{AsyncScriptState, HANDLER_NAME};
use crate::platform::{wrap_script_for_frame_context, FrameId, PlatformExecutor, PrintOptions};
use crate::server::response::WebDriverErrorResponse;
use crate::webdriver::Timeouts;

/// Linux `WebKitGTK` executor
#[derive(Clone)]
pub struct LinuxExecutor<R: Runtime> {
    window: WebviewWindow<R>,
    timeouts: Timeouts,
    frame_context: Vec<FrameId>,
}

impl<R: Runtime> LinuxExecutor<R> {
    pub fn new(window: WebviewWindow<R>, timeouts: Timeouts, frame_context: Vec<FrameId>) -> Self {
        Self {
            window,
            timeouts,
            frame_context,
        }
    }
}

/// Register `WebKitGTK` handlers at webview creation time.
/// This is called from the plugin's `on_webview_ready` hook to ensure
/// the script dialog handler is registered before any navigation completes.
pub fn register_webview_handlers<R: Runtime>(webview: &tauri::Webview<R>) {
    use crate::platform::alert_state::AlertResponse;
    use webkit2gtk::WebViewExt as _;

    // Get per-window alert state from the manager
    let manager = webview.app_handle().state::<AlertStateManager>();
    let alert_state = manager.get_or_create(webview.label());

    let _ = webview.with_webview(move |webview| {
        let webview = webview.inner().clone();
        let alert_state = alert_state.clone();

        // Connect to the script-dialog signal to intercept JS dialogs
        webview.connect_script_dialog(move |_webview, dialog| {
            let dialog_type = dialog.dialog_type();
            let message = dialog.message().map(|s| s.to_string()).unwrap_or_default();

            // Map WebKitGTK dialog type to our AlertType
            let alert_type = match dialog_type {
                ScriptDialogType::Alert => AlertType::Alert,
                ScriptDialogType::Confirm => AlertType::Confirm,
                ScriptDialogType::Prompt => AlertType::Prompt,
                ScriptDialogType::BeforeUnloadConfirm | _ => {
                    // BEFOREUNLOAD or unknown - let default behavior handle it
                    return false;
                }
            };

            let default_text = if alert_type == AlertType::Prompt {
                dialog
                    .prompt_get_default_text()
                    .map(|s| s.to_string())
                    .filter(|s| !s.is_empty())
            } else {
                None
            };

            tracing::debug!("Intercepted {:?} dialog: {}", alert_type, message);

            // Create channel for WebDriver response
            let (tx, rx) = std::sync::mpsc::channel::<AlertResponse>();
            alert_state.set_pending(PendingAlert {
                message: message.clone(),
                default_text: default_text.clone(),
                alert_type,
                responder: tx,
            });

            // Wait for WebDriver response with timeout
            let timeout = std::time::Duration::from_secs(30);
            let response = rx.recv_timeout(timeout);

            match response {
                Ok(AlertResponse {
                    accepted,
                    prompt_text,
                }) => {
                    if alert_type == AlertType::Confirm {
                        dialog.confirm_set_confirmed(accepted);
                    } else if alert_type == AlertType::Prompt && accepted {
                        // Only set text if accepted - when dismissed, not calling
                        // prompt_set_text() causes JavaScript to receive null
                        let text = prompt_text.or(default_text).unwrap_or_default();
                        dialog.prompt_set_text(&text);
                    }
                    // For Alert type, nothing special to set
                }
                Err(_) => {
                    // Timeout - auto-accept
                    if alert_type == AlertType::Confirm {
                        dialog.confirm_set_confirmed(true);
                    }
                }
            }

            // Return true to indicate we handled the dialog
            true
        });

        tracing::debug!("Registered script dialog handler for webview");
    });
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
        let script_owned = wrap_script_for_frame_context(script, &self.frame_context);

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
                if (!el || !el.isConnected) {{
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
    // Print
    // =========================================================================

    async fn print_page(&self, options: PrintOptions) -> Result<String, WebDriverErrorResponse> {
        let (tx, rx) = oneshot::channel::<Result<(), String>>();

        // Create temp directory for PDF output
        let temp_dir = tempfile::TempDir::new().map_err(|e| {
            WebDriverErrorResponse::unknown_error(&format!("Failed to create temp dir: {e}"))
        })?;
        let pdf_path = temp_dir.path().join("print.pdf");
        let pdf_path_clone = pdf_path.clone();

        // Extract options before moving into closure
        let orientation = options.orientation.clone();
        let page_width = options.page_width;
        let page_height = options.page_height;
        let margin_top = options.margin_top;
        let margin_bottom = options.margin_bottom;
        let margin_left = options.margin_left;
        let margin_right = options.margin_right;

        let result = self.window.with_webview(move |webview| {
            let webview = webview.inner().clone();

            // Create print operation
            let print_op = webkit2gtk::PrintOperation::new(&webview);

            // Create page setup
            let page_setup = gtk::PageSetup::new();

            // Page size (cm to points: 1 cm = 28.35 points)
            let width_points = page_width.unwrap_or(21.0) * 28.35;
            let height_points = page_height.unwrap_or(29.7) * 28.35;
            let paper_size = gtk::PaperSize::new_custom(
                "custom",
                "Custom",
                width_points,
                height_points,
                gtk::Unit::Points,
            );
            page_setup.set_paper_size(&paper_size);

            // Orientation
            if orientation.as_deref() == Some("landscape") {
                page_setup.set_orientation(gtk::PageOrientation::Landscape);
            } else {
                page_setup.set_orientation(gtk::PageOrientation::Portrait);
            }

            // Margins (cm to points)
            page_setup.set_top_margin(margin_top.unwrap_or(1.0) * 28.35, gtk::Unit::Points);
            page_setup.set_bottom_margin(margin_bottom.unwrap_or(1.0) * 28.35, gtk::Unit::Points);
            page_setup.set_left_margin(margin_left.unwrap_or(1.0) * 28.35, gtk::Unit::Points);
            page_setup.set_right_margin(margin_right.unwrap_or(1.0) * 28.35, gtk::Unit::Points);

            print_op.set_page_setup(&page_setup);

            // Print settings for PDF output
            let settings = gtk::PrintSettings::new();
            settings.set_printer("Print to File");
            settings.set(
                gtk::PRINT_SETTINGS_OUTPUT_URI,
                Some(&format!("file://{}", pdf_path_clone.display())),
            );
            settings.set(gtk::PRINT_SETTINGS_OUTPUT_FILE_FORMAT, Some("pdf"));

            print_op.set_print_settings(&settings);

            // Connect to finished signal
            let tx = Arc::new(std::sync::Mutex::new(Some(tx)));
            print_op.connect_finished(move |_op| {
                if let Ok(mut guard) = tx.lock() {
                    if let Some(tx) = guard.take() {
                        let _ = tx.send(Ok(()));
                    }
                }
            });

            // Run print operation (silent, no dialog)
            let () = print_op.print();
        });

        if let Err(e) = result {
            return Err(WebDriverErrorResponse::unknown_error(&e.to_string()));
        }

        // Wait for completion
        let timeout = std::time::Duration::from_millis(self.timeouts.script_ms);
        match tokio::time::timeout(timeout, rx).await {
            Ok(Ok(Ok(()))) => {}
            Ok(Ok(Err(error))) => {
                return Err(WebDriverErrorResponse::unknown_error(&error));
            }
            Ok(Err(_)) => {
                return Err(WebDriverErrorResponse::unknown_error("Channel closed"));
            }
            Err(_) => {
                return Err(WebDriverErrorResponse::script_timeout());
            }
        }

        // Read the PDF file
        let pdf_data = std::fs::read(&pdf_path).map_err(|e| {
            WebDriverErrorResponse::unknown_error(&format!("Failed to read PDF file: {e}"))
        })?;

        Ok(BASE64_STANDARD.encode(&pdf_data))
    }

    // =========================================================================
    // Async Script Execution
    // =========================================================================

    #[allow(clippy::too_many_lines)]
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
        let label = self.window.label().to_string();
        if !async_state.mark_handler_registered(&label) {
            let app_clone = app.clone();
            let handler_result = self.window.with_webview(move |webview| {
                let webview = webview.inner().clone();
                let state = app_clone.state::<AsyncScriptState>();
                let state_ptr = state.inner() as *const AsyncScriptState;

                // Get the UserContentManager and register our handler
                if let Some(manager) = webview.user_content_manager() {
                    // Register the script message handler
                    let _ = manager.register_script_message_handler(HANDLER_NAME);

                    // Connect to receive messages
                    // SAFETY: state_ptr is valid for the lifetime of the app
                    manager.connect_script_message_received(
                        Some(HANDLER_NAME),
                        move |_manager, result| {
                            let state = unsafe { &*state_ptr };

                            // Get the JavaScript value from the result
                            let js_value = result.js_value();
                            let Some(js_value) = js_value else {
                                return;
                            };

                            // Convert to JSON
                            let Some(json_str) = js_value.to_json(0) else {
                                return;
                            };

                            // Parse the message
                            let msg: Value = match serde_json::from_str(json_str.as_str()) {
                                Ok(v) => v,
                                Err(_) => return,
                            };

                            // Extract async ID
                            let async_id = match msg.get("id").and_then(Value::as_str) {
                                Some(id) => id.to_string(),
                                None => return,
                            };

                            // Check for error
                            if let Some(error) = msg.get("error").and_then(Value::as_str) {
                                if !error.is_empty() {
                                    state.complete(&async_id, Err(error.to_string()));
                                    return;
                                }
                            }

                            // Extract result
                            let result = msg.get("result").cloned().unwrap_or(Value::Null);
                            state.complete(&async_id, Ok(result));
                        },
                    );

                    tracing::debug!("Registered native message handler for webview");
                }
            });

            if let Err(e) = handler_result {
                return Err(WebDriverErrorResponse::unknown_error(&format!(
                    "Failed to register message handler: {e}"
                )));
            }
        }

        // Register pending operation
        let rx = async_state.register(async_id.clone());

        // Build wrapper with native postMessage (WebKitGTK uses same API as macOS WebKit)
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
            }})()"
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
