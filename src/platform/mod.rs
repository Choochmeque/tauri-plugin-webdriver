pub(crate) mod alert_state;
mod async_state;
mod executor;

pub use async_state::AsyncScriptState;
pub use executor::*;

#[cfg(target_os = "macos")]
mod macos;

#[cfg(target_os = "windows")]
mod windows;

#[cfg(target_os = "linux")]
mod linux;

use std::sync::Arc;
use tauri::{Runtime, WebviewWindow};

use crate::webdriver::Timeouts;

/// Create a platform-specific executor for the given window
#[cfg(target_os = "macos")]
pub fn create_executor<R: Runtime + 'static>(
    window: WebviewWindow<R>,
    timeouts: Timeouts,
    frame_context: Vec<FrameId>,
) -> Arc<dyn PlatformExecutor<R>> {
    Arc::new(macos::MacOSExecutor::new(window, timeouts, frame_context))
}

/// Create a platform-specific executor for the given window
#[cfg(target_os = "windows")]
pub fn create_executor<R: Runtime + 'static>(
    window: WebviewWindow<R>,
    timeouts: Timeouts,
    frame_context: Vec<FrameId>,
) -> Arc<dyn PlatformExecutor<R>> {
    Arc::new(windows::WindowsExecutor::new(
        window,
        timeouts,
        frame_context,
    ))
}

/// Create a platform-specific executor for the given window
#[cfg(target_os = "linux")]
pub fn create_executor<R: Runtime + 'static>(
    window: WebviewWindow<R>,
    timeouts: Timeouts,
    frame_context: Vec<FrameId>,
) -> Arc<dyn PlatformExecutor<R>> {
    Arc::new(linux::LinuxExecutor::new(window, timeouts, frame_context))
}

/// Register platform-specific webview handlers at webview creation time.
/// This is called from the plugin's `on_webview_ready` hook.
#[cfg(target_os = "windows")]
pub fn register_webview_handlers<R: Runtime>(webview: &tauri::Webview<R>) {
    windows::register_webview_handlers(webview);
}

/// Register platform-specific webview handlers at webview creation time.
/// This is called from the plugin's `on_webview_ready` hook.
#[cfg(target_os = "macos")]
pub fn register_webview_handlers<R: Runtime>(webview: &tauri::Webview<R>) {
    macos::register_webview_handlers(webview);
}

/// Register platform-specific webview handlers at webview creation time.
/// This is called from the plugin's `on_webview_ready` hook.
#[cfg(target_os = "linux")]
pub fn register_webview_handlers<R: Runtime>(webview: &tauri::Webview<R>) {
    linux::register_webview_handlers(webview);
}
