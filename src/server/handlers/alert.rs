use std::sync::Arc;

use axum::extract::{Path, State};
use axum::Json;
use serde::Deserialize;
use tauri::Runtime;

use crate::server::response::{WebDriverErrorResponse, WebDriverResponse, WebDriverResult};
use crate::server::AppState;

#[derive(Debug, Deserialize)]
pub struct SendAlertTextRequest {
    pub text: String,
}

/// POST `/session/{session_id}/alert/dismiss` - Dismiss alert
pub async fn dismiss<R: Runtime + 'static>(
    State(state): State<Arc<AppState<R>>>,
    Path(session_id): Path<String>,
) -> WebDriverResult {
    let sessions = state.sessions.read().await;
    let session = sessions
        .get(&session_id)
        .ok_or_else(|| WebDriverErrorResponse::invalid_session_id(&session_id))?;
    let current_window = session.current_window.clone();
    drop(sessions);

    let executor = state.get_executor_for_window(&current_window)?;
    executor.dismiss_alert().await?;

    Ok(WebDriverResponse::null())
}

/// POST `/session/{session_id}/alert/accept` - Accept alert
pub async fn accept<R: Runtime + 'static>(
    State(state): State<Arc<AppState<R>>>,
    Path(session_id): Path<String>,
) -> WebDriverResult {
    let sessions = state.sessions.read().await;
    let session = sessions
        .get(&session_id)
        .ok_or_else(|| WebDriverErrorResponse::invalid_session_id(&session_id))?;
    let current_window = session.current_window.clone();
    drop(sessions);

    let executor = state.get_executor_for_window(&current_window)?;
    executor.accept_alert().await?;

    Ok(WebDriverResponse::null())
}

/// GET `/session/{session_id}/alert/text` - Get alert text
pub async fn get_text<R: Runtime + 'static>(
    State(state): State<Arc<AppState<R>>>,
    Path(session_id): Path<String>,
) -> WebDriverResult {
    let sessions = state.sessions.read().await;
    let session = sessions
        .get(&session_id)
        .ok_or_else(|| WebDriverErrorResponse::invalid_session_id(&session_id))?;
    let current_window = session.current_window.clone();
    drop(sessions);

    let executor = state.get_executor_for_window(&current_window)?;
    let text: String = executor.get_alert_text().await?;

    Ok(WebDriverResponse::success(text))
}

/// POST `/session/{session_id}/alert/text` - Send text to alert (for prompts)
pub async fn send_text<R: Runtime + 'static>(
    State(state): State<Arc<AppState<R>>>,
    Path(session_id): Path<String>,
    Json(request): Json<SendAlertTextRequest>,
) -> WebDriverResult {
    let sessions = state.sessions.read().await;
    let session = sessions
        .get(&session_id)
        .ok_or_else(|| WebDriverErrorResponse::invalid_session_id(&session_id))?;
    let current_window = session.current_window.clone();
    drop(sessions);

    let executor = state.get_executor_for_window(&current_window)?;
    executor.send_alert_text(&request.text).await?;

    Ok(WebDriverResponse::null())
}
