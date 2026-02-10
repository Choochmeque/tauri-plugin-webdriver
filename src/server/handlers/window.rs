use std::sync::Arc;

use axum::extract::{Path, State};
use tauri::{Manager, Runtime};

use crate::server::response::{WebDriverErrorResponse, WebDriverResponse, WebDriverResult};
use crate::server::AppState;

/// GET /session/{session_id}/window - Get current window handle
pub async fn get_window_handle<R: Runtime>(
    State(state): State<Arc<AppState<R>>>,
    Path(session_id): Path<String>,
) -> WebDriverResult {
    let sessions = state.sessions.read().await;
    let _session = sessions
        .get(&session_id)
        .ok_or_else(|| WebDriverErrorResponse::invalid_session_id(&session_id))?;
    drop(sessions);

    // Return the first window's label as the current window handle
    if let Some((label, _)) = state.app.webview_windows().iter().next() {
        Ok(WebDriverResponse::success(label.clone()))
    } else {
        Err(WebDriverErrorResponse::no_such_window())
    }
}

/// GET /session/{session_id}/window/handles - Get all window handles
pub async fn get_window_handles<R: Runtime>(
    State(state): State<Arc<AppState<R>>>,
    Path(session_id): Path<String>,
) -> WebDriverResult {
    let sessions = state.sessions.read().await;
    let _session = sessions
        .get(&session_id)
        .ok_or_else(|| WebDriverErrorResponse::invalid_session_id(&session_id))?;
    drop(sessions);

    // Return all window labels as handles
    let handles: Vec<String> = state
        .app
        .webview_windows()
        .keys()
        .cloned()
        .collect();

    Ok(WebDriverResponse::success(handles))
}

/// DELETE /session/{session_id}/window - Close current window
pub async fn close_window<R: Runtime>(
    State(state): State<Arc<AppState<R>>>,
    Path(session_id): Path<String>,
) -> WebDriverResult {
    let sessions = state.sessions.read().await;
    let _session = sessions
        .get(&session_id)
        .ok_or_else(|| WebDriverErrorResponse::invalid_session_id(&session_id))?;
    drop(sessions);

    // Close the first window
    if let Some(window) = state.app.webview_windows().values().next().cloned() {
        window
            .close()
            .map_err(|e| WebDriverErrorResponse::unknown_error(&e.to_string()))?;

        // Return remaining window handles
        let handles: Vec<String> = state
            .app
            .webview_windows()
            .keys()
            .cloned()
            .collect();

        Ok(WebDriverResponse::success(handles))
    } else {
        Err(WebDriverErrorResponse::no_such_window())
    }
}
