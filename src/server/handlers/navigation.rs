use std::sync::Arc;

use axum::extract::{Path, State};
use axum::Json;
use serde::Deserialize;
use tauri::Runtime;

use crate::server::response::{WebDriverResponse, WebDriverResult};
use crate::server::AppState;

#[derive(Debug, Deserialize)]
pub struct NavigateRequest {
    pub url: String,
}

/// POST `/session/{session_id}/url` - Navigate to URL
pub async fn navigate<R: Runtime + 'static>(
    State(state): State<Arc<AppState<R>>>,
    Path(session_id): Path<String>,
    Json(request): Json<NavigateRequest>,
) -> WebDriverResult {
    let sessions = state.sessions.read().await;
    let session = sessions.get(&session_id)?;
    let current_window = session.current_window.clone();
    let timeouts = session.timeouts.clone();
    drop(sessions);

    let executor = state.get_executor_for_window(&current_window, timeouts)?;
    executor.navigate(&request.url).await?;

    Ok(WebDriverResponse::null())
}

/// GET `/session/{session_id}/url` - Get current URL
pub async fn get_url<R: Runtime + 'static>(
    State(state): State<Arc<AppState<R>>>,
    Path(session_id): Path<String>,
) -> WebDriverResult {
    let sessions = state.sessions.read().await;
    let session = sessions.get(&session_id)?;
    let current_window = session.current_window.clone();
    let timeouts = session.timeouts.clone();
    drop(sessions);

    let executor = state.get_executor_for_window(&current_window, timeouts)?;
    let url = executor.get_url().await?;
    Ok(WebDriverResponse::success(url))
}

/// GET `/session/{session_id}/title` - Get page title
pub async fn get_title<R: Runtime + 'static>(
    State(state): State<Arc<AppState<R>>>,
    Path(session_id): Path<String>,
) -> WebDriverResult {
    let sessions = state.sessions.read().await;
    let session = sessions.get(&session_id)?;
    let current_window = session.current_window.clone();
    let timeouts = session.timeouts.clone();
    drop(sessions);

    let executor = state.get_executor_for_window(&current_window, timeouts)?;
    let title = executor.get_title().await?;
    Ok(WebDriverResponse::success(title))
}

/// POST `/session/{session_id}/back` - Navigate back
pub async fn back<R: Runtime + 'static>(
    State(state): State<Arc<AppState<R>>>,
    Path(session_id): Path<String>,
) -> WebDriverResult {
    let sessions = state.sessions.read().await;
    let session = sessions.get(&session_id)?;
    let current_window = session.current_window.clone();
    let timeouts = session.timeouts.clone();
    drop(sessions);

    let executor = state.get_executor_for_window(&current_window, timeouts)?;
    executor.go_back().await?;
    Ok(WebDriverResponse::null())
}

/// POST `/session/{session_id}/forward` - Navigate forward
pub async fn forward<R: Runtime + 'static>(
    State(state): State<Arc<AppState<R>>>,
    Path(session_id): Path<String>,
) -> WebDriverResult {
    let sessions = state.sessions.read().await;
    let session = sessions.get(&session_id)?;
    let current_window = session.current_window.clone();
    let timeouts = session.timeouts.clone();
    drop(sessions);

    let executor = state.get_executor_for_window(&current_window, timeouts)?;
    executor.go_forward().await?;
    Ok(WebDriverResponse::null())
}

/// POST `/session/{session_id}/refresh` - Refresh page
pub async fn refresh<R: Runtime + 'static>(
    State(state): State<Arc<AppState<R>>>,
    Path(session_id): Path<String>,
) -> WebDriverResult {
    let sessions = state.sessions.read().await;
    let session = sessions.get(&session_id)?;
    let current_window = session.current_window.clone();
    let timeouts = session.timeouts.clone();
    drop(sessions);

    let executor = state.get_executor_for_window(&current_window, timeouts)?;
    executor.refresh().await?;
    Ok(WebDriverResponse::null())
}
