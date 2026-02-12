mod executor;

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
) -> Arc<dyn PlatformExecutor<R>> {
    Arc::new(macos::MacOSExecutor::new(window, timeouts))
}

/// Create a platform-specific executor for the given window
#[cfg(target_os = "windows")]
pub fn create_executor<R: Runtime + 'static>(
    window: WebviewWindow<R>,
    timeouts: Timeouts,
) -> Arc<dyn PlatformExecutor<R>> {
    Arc::new(windows::WindowsExecutor::new(window, timeouts))
}

/// Create a platform-specific executor for the given window
#[cfg(target_os = "linux")]
pub fn create_executor<R: Runtime + 'static>(
    window: WebviewWindow<R>,
    timeouts: Timeouts,
) -> Arc<dyn PlatformExecutor<R>> {
    Arc::new(linux::LinuxExecutor::new(window, timeouts))
}
