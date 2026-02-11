use std::sync::Arc;

use axum::extract::{Path, State};
use axum::Json;
use serde::Deserialize;
use serde_json::Value;
use tauri::Runtime;

use crate::server::response::{WebDriverErrorResponse, WebDriverResponse, WebDriverResult};
use crate::server::AppState;

#[derive(Debug, Deserialize)]
pub struct ExecuteScriptRequest {
    pub script: String,
    #[serde(default)]
    pub args: Vec<Value>,
}

/// POST `/session/{session_id}/execute/sync` - Execute synchronous script
pub async fn execute_sync<R: Runtime + 'static>(
    State(state): State<Arc<AppState<R>>>,
    Path(session_id): Path<String>,
    Json(request): Json<ExecuteScriptRequest>,
) -> WebDriverResult {
    let sessions = state.sessions.read().await;
    let session = sessions
        .get(&session_id)
        .ok_or_else(|| WebDriverErrorResponse::invalid_session_id(&session_id))?;
    let current_window = session.current_window.clone();
    drop(sessions);

    let executor = state.get_executor_for_window(&current_window)?;
    let result = executor
        .execute_script(&request.script, &request.args)
        .await?;
    Ok(WebDriverResponse::success(result))
}

/// POST `/session/{session_id}/execute/async` - Execute asynchronous script
pub async fn execute_async<R: Runtime + 'static>(
    State(state): State<Arc<AppState<R>>>,
    Path(session_id): Path<String>,
    Json(request): Json<ExecuteScriptRequest>,
) -> WebDriverResult {
    let sessions = state.sessions.read().await;
    let session = sessions
        .get(&session_id)
        .ok_or_else(|| WebDriverErrorResponse::invalid_session_id(&session_id))?;
    let timeout_ms = session.timeouts.script_ms;
    let current_window = session.current_window.clone();
    drop(sessions);

    let executor = state.get_executor_for_window(&current_window)?;
    let result = executor
        .execute_async_script(&request.script, &request.args, timeout_ms)
        .await?;
    Ok(WebDriverResponse::success(result))
}
