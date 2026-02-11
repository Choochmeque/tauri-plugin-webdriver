mod executor;

pub use executor::*;

#[cfg(target_os = "macos")]
mod macos;

#[cfg(target_os = "windows")]
mod windows;

use std::sync::Arc;
use tauri::{Runtime, WebviewWindow};

/// Create a platform-specific executor for the given window
#[cfg(target_os = "macos")]
pub fn create_executor<R: Runtime + 'static>(
    window: WebviewWindow<R>,
) -> Arc<dyn PlatformExecutor> {
    Arc::new(macos::MacOSExecutor::new(window))
}

/// Create a platform-specific executor for the given window
#[cfg(target_os = "windows")]
pub fn create_executor<R: Runtime + 'static>(
    window: WebviewWindow<R>,
) -> Arc<dyn PlatformExecutor> {
    Arc::new(windows::WindowsExecutor::new(window))
}
