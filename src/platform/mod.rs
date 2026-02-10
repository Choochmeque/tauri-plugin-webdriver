mod executor;

pub use executor::*;

#[cfg(target_os = "macos")]
mod macos;

use std::sync::Arc;
use tauri::{Runtime, WebviewWindow};

/// Create a platform-specific executor for the given window
#[cfg(target_os = "macos")]
pub fn create_executor<R: Runtime + 'static>(
    window: WebviewWindow<R>,
) -> Arc<dyn PlatformExecutor> {
    Arc::new(macos::MacOSExecutor::new(window))
}
