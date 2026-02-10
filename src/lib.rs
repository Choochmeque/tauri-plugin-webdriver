use tauri::{
    plugin::{Builder, TauriPlugin},
    Listener, Manager, Runtime,
};

pub use models::*;

#[cfg(desktop)]
mod desktop;
#[cfg(mobile)]
mod mobile;

mod commands;
mod error;
mod models;
#[cfg(target_os = "macos")]
mod platform;
mod server;
mod webdriver;

pub use error::{Error, Result};

#[cfg(desktop)]
use desktop::Webdriver;
#[cfg(mobile)]
use mobile::Webdriver;

/// Default port for the WebDriver HTTP server
const DEFAULT_PORT: u16 = 4445;

/// Extensions to [`tauri::App`], [`tauri::AppHandle`] and [`tauri::Window`] to access the webdriver APIs.
pub trait WebdriverExt<R: Runtime> {
    fn webdriver(&self) -> &Webdriver<R>;
}

impl<R: Runtime, T: Manager<R>> crate::WebdriverExt<R> for T {
    fn webdriver(&self) -> &Webdriver<R> {
        self.state::<Webdriver<R>>().inner()
    }
}

/// Payload for JavaScript result events
#[derive(Debug, Clone, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct WebDriverResultPayload {
    request_id: String,
    success: bool,
    #[serde(default)]
    value: serde_json::Value,
    #[serde(default)]
    error: Option<String>,
}

/// Initializes the plugin.
pub fn init<R: Runtime>() -> TauriPlugin<R> {
    Builder::new("webdriver")
        .invoke_handler(tauri::generate_handler![commands::ping])
        .setup(|app, api| {
            #[cfg(mobile)]
            let webdriver = mobile::init(app, api)?;
            #[cfg(desktop)]
            let webdriver = desktop::init(app, api)?;
            app.manage(webdriver);

            // Set up event listener for JavaScript results
            #[cfg(target_os = "macos")]
            {
                let app_handle = app.app_handle().clone();
                app_handle.listen("webdriver-result", move |event| {
                    if let Ok(payload) = serde_json::from_str::<WebDriverResultPayload>(event.payload()) {
                        let result = if payload.success {
                            serde_json::json!({
                                "success": true,
                                "value": payload.value
                            })
                        } else {
                            serde_json::json!({
                                "success": false,
                                "error": payload.error.unwrap_or_default()
                            })
                        };

                        // Send result to waiting handler
                        let request_id = payload.request_id;
                        let result_str = result.to_string();
                        tauri::async_runtime::spawn(async move {
                            platform::macos::handle_js_result(request_id, result_str).await;
                        });
                    }
                });
            }

            // Start the WebDriver HTTP server
            #[cfg(desktop)]
            {
                let app_handle = app.app_handle().clone();
                server::start(app_handle, DEFAULT_PORT);
                tracing::info!("WebDriver plugin initialized on port {}", DEFAULT_PORT);
            }

            Ok(())
        })
        .build()
}
