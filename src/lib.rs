use tauri::{
    plugin::{Builder, TauriPlugin},
    Manager, Runtime,
};

#[cfg(desktop)]
mod desktop;
#[cfg(mobile)]
mod mobile;

mod error;
mod platform;
mod server;
mod webdriver;

pub use error::{Error, Result};

/// Default port for the `WebDriver` HTTP server
const DEFAULT_PORT: u16 = 4445;

/// Initializes the plugin.
#[must_use]
pub fn init<R: Runtime>() -> TauriPlugin<R> {
    Builder::new("webdriver")
        .setup(|app, api| {
            #[cfg(mobile)]
            let webdriver = mobile::init(app, api)?;
            #[cfg(desktop)]
            let webdriver = desktop::init(app, api);
            app.manage(webdriver);

            // Start the WebDriver HTTP server
            let app_handle = app.app_handle().clone();
            server::start(app_handle, DEFAULT_PORT);
            tracing::info!("WebDriver plugin initialized on port {DEFAULT_PORT}");

            Ok(())
        })
        .build()
}
