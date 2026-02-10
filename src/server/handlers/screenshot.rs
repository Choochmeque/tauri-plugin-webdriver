use std::sync::Arc;

use axum::extract::{Path, State};
use tauri::Runtime;

use crate::server::response::{WebDriverErrorResponse, WebDriverResponse, WebDriverResult};
use crate::server::AppState;

/// GET /session/{session_id}/screenshot - Take screenshot
pub async fn take<R: Runtime>(
    State(state): State<Arc<AppState<R>>>,
    Path(session_id): Path<String>,
) -> WebDriverResult {
    let sessions = state.sessions.read().await;
    let _session = sessions
        .get(&session_id)
        .ok_or_else(|| WebDriverErrorResponse::invalid_session_id(&session_id))?;

    // TODO: Implement screenshot via WKWebView takeSnapshot
    // For now, return empty base64 string
    Ok(WebDriverResponse::success(""))
}
