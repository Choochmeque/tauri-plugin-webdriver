use std::sync::Arc;

use axum::extract::{Path, State};
use tauri::Runtime;

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

    let executor = state.get_executor()?;
    let source = executor.get_source().await?;
    Ok(WebDriverResponse::success(source))
}
