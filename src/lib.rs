use tauri::{
    plugin::{Builder, TauriPlugin},
    Manager, Runtime,
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
