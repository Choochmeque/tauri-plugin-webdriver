use std::sync::Arc;

use axum::extract::{Path, State};
use tauri::{Manager, Runtime};

#[cfg(target_os = "macos")]
use crate::platform::macos::WebViewExecutor;
use crate::server::response::{WebDriverErrorResponse, WebDriverResponse, WebDriverResult};
use crate::server::AppState;

/// GET /session/{session_id}/source - Get page source
pub async fn get_source<R: Runtime + 'static>(
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
            let source = executor.get_source().await?;
            return Ok(WebDriverResponse::success(source));
        }
    }

    Ok(WebDriverResponse::success(""))
}
