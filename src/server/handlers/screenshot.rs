use std::sync::Arc;

use axum::extract::{Path, State};
use tauri::{Manager, Runtime};

use crate::platform::WebViewExecutor;

#[cfg(target_os = "macos")]
use crate::server::response::{WebDriverErrorResponse, WebDriverResponse, WebDriverResult};
use crate::server::AppState;

/// GET /session/{session_id}/screenshot - Take screenshot
pub async fn take<R: Runtime + 'static>(
    State(state): State<Arc<AppState<R>>>,
    Path(session_id): Path<String>,
) -> WebDriverResult {
    let sessions = state.sessions.read().await;
    let _session = sessions
        .get(&session_id)
        .ok_or_else(|| WebDriverErrorResponse::invalid_session_id(&session_id))?;
    drop(sessions);

    #[cfg(target_os = "macos")]
    {
        if let Some(window) = state.app.webview_windows().values().next().cloned() {
            let executor = WebViewExecutor::new(window);
            let screenshot = executor.take_screenshot().await?;
            return Ok(WebDriverResponse::success(screenshot));
        }
    }

    // Screenshot not yet implemented for this platform
    Ok(WebDriverResponse::success(""))
}
