use std::sync::Arc;

use axum::extract::{Path, State};
use axum::Json;
use serde::Deserialize;
use tauri::{Manager, Runtime};

use crate::server::response::{WebDriverErrorResponse, WebDriverResponse, WebDriverResult};
use crate::server::AppState;

#[derive(Debug, Deserialize)]
pub struct NavigateRequest {
    pub url: String,
}

/// POST /session/{session_id}/url - Navigate to URL
pub async fn navigate<R: Runtime>(
    State(state): State<Arc<AppState<R>>>,
    Path(session_id): Path<String>,
    Json(request): Json<NavigateRequest>,
) -> WebDriverResult {
    let sessions = state.sessions.read().await;
    let session = sessions
        .get(&session_id)
        .ok_or_else(|| WebDriverErrorResponse::invalid_session_id(&session_id))?;

    // Execute navigation via JavaScript
    let script = format!(r#"window.location.href = '{}';"#, request.url.replace('\'', "\\'"));

    // TODO: Execute script via WKWebView
    // For now, use Tauri's webview eval
    if let Some(webview) = state.app.webview_windows().values().next() {
        webview
            .eval(&script)
            .map_err(|e: tauri::Error| WebDriverErrorResponse::javascript_error(&e.to_string()))?;
    }

    Ok(WebDriverResponse::null())
}

/// GET /session/{session_id}/url - Get current URL
pub async fn get_url<R: Runtime>(
    State(state): State<Arc<AppState<R>>>,
    Path(session_id): Path<String>,
) -> WebDriverResult {
    let sessions = state.sessions.read().await;
    let _session = sessions
        .get(&session_id)
        .ok_or_else(|| WebDriverErrorResponse::invalid_session_id(&session_id))?;

    // TODO: Get URL via evaluateJavaScript
    // For now, return placeholder
    Ok(WebDriverResponse::success("about:blank"))
}

/// GET /session/{session_id}/title - Get page title
pub async fn get_title<R: Runtime>(
    State(state): State<Arc<AppState<R>>>,
    Path(session_id): Path<String>,
) -> WebDriverResult {
    let sessions = state.sessions.read().await;
    let _session = sessions
        .get(&session_id)
        .ok_or_else(|| WebDriverErrorResponse::invalid_session_id(&session_id))?;

    // TODO: Get title via evaluateJavaScript
    Ok(WebDriverResponse::success(""))
}

/// POST /session/{session_id}/back - Navigate back
pub async fn back<R: Runtime>(
    State(state): State<Arc<AppState<R>>>,
    Path(session_id): Path<String>,
) -> WebDriverResult {
    let sessions = state.sessions.read().await;
    let _session = sessions
        .get(&session_id)
        .ok_or_else(|| WebDriverErrorResponse::invalid_session_id(&session_id))?;

    if let Some(webview) = state.app.webview_windows().values().next() {
        webview
            .eval("window.history.back();")
            .map_err(|e: tauri::Error| WebDriverErrorResponse::javascript_error(&e.to_string()))?;
    }

    Ok(WebDriverResponse::null())
}

/// POST /session/{session_id}/forward - Navigate forward
pub async fn forward<R: Runtime>(
    State(state): State<Arc<AppState<R>>>,
    Path(session_id): Path<String>,
) -> WebDriverResult {
    let sessions = state.sessions.read().await;
    let _session = sessions
        .get(&session_id)
        .ok_or_else(|| WebDriverErrorResponse::invalid_session_id(&session_id))?;

    if let Some(webview) = state.app.webview_windows().values().next() {
        webview
            .eval("window.history.forward();")
            .map_err(|e: tauri::Error| WebDriverErrorResponse::javascript_error(&e.to_string()))?;
    }

    Ok(WebDriverResponse::null())
}

/// POST /session/{session_id}/refresh - Refresh page
pub async fn refresh<R: Runtime>(
    State(state): State<Arc<AppState<R>>>,
    Path(session_id): Path<String>,
) -> WebDriverResult {
    let sessions = state.sessions.read().await;
    let _session = sessions
        .get(&session_id)
        .ok_or_else(|| WebDriverErrorResponse::invalid_session_id(&session_id))?;

    if let Some(webview) = state.app.webview_windows().values().next() {
        webview
            .eval("window.location.reload();")
            .map_err(|e: tauri::Error| WebDriverErrorResponse::javascript_error(&e.to_string()))?;
    }

    Ok(WebDriverResponse::null())
}
