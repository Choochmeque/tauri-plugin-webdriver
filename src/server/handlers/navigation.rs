use std::sync::Arc;

use axum::extract::{Path, State};
use axum::Json;
use serde::Deserialize;
use tauri::{Manager, Runtime};

use crate::platform::WebViewExecutor;

#[cfg(target_os = "macos")]
use crate::server::response::{WebDriverErrorResponse, WebDriverResponse, WebDriverResult};
use crate::server::AppState;

#[derive(Debug, Deserialize)]
pub struct NavigateRequest {
    pub url: String,
}

/// POST /session/{session_id}/url - Navigate to URL
pub async fn navigate<R: Runtime + 'static>(
    State(state): State<Arc<AppState<R>>>,
    Path(session_id): Path<String>,
    Json(request): Json<NavigateRequest>,
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
            executor.navigate(&request.url).await?;
        }
    }

    #[cfg(not(target_os = "macos"))]
    {
        let script = format!(
            r#"window.location.href = '{}';"#,
            request.url.replace('\'', "\\'")
        );
        if let Some(webview) = state.app.webview_windows().values().next() {
            webview
                .eval(&script)
                .map_err(|e: tauri::Error| WebDriverErrorResponse::javascript_error(&e.to_string()))?;
        }
    }

    Ok(WebDriverResponse::null())
}

/// GET /session/{session_id}/url - Get current URL
pub async fn get_url<R: Runtime + 'static>(
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
            let url = executor.get_url().await?;
            return Ok(WebDriverResponse::success(url));
        }
    }

    Ok(WebDriverResponse::success("about:blank"))
}

/// GET /session/{session_id}/title - Get page title
pub async fn get_title<R: Runtime + 'static>(
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
            let title = executor.get_title().await?;
            return Ok(WebDriverResponse::success(title));
        }
    }

    Ok(WebDriverResponse::success(""))
}

/// POST /session/{session_id}/back - Navigate back
pub async fn back<R: Runtime + 'static>(
    State(state): State<Arc<AppState<R>>>,
    Path(session_id): Path<String>,
) -> WebDriverResult {
    let sessions = state.sessions.read().await;
    let _session = sessions
        .get(&session_id)
        .ok_or_else(|| WebDriverErrorResponse::invalid_session_id(&session_id))?;
    drop(sessions);

    if let Some(webview) = state.app.webview_windows().values().next() {
        webview
            .eval("window.history.back();")
            .map_err(|e: tauri::Error| WebDriverErrorResponse::javascript_error(&e.to_string()))?;
    }

    Ok(WebDriverResponse::null())
}

/// POST /session/{session_id}/forward - Navigate forward
pub async fn forward<R: Runtime + 'static>(
    State(state): State<Arc<AppState<R>>>,
    Path(session_id): Path<String>,
) -> WebDriverResult {
    let sessions = state.sessions.read().await;
    let _session = sessions
        .get(&session_id)
        .ok_or_else(|| WebDriverErrorResponse::invalid_session_id(&session_id))?;
    drop(sessions);

    if let Some(webview) = state.app.webview_windows().values().next() {
        webview
            .eval("window.history.forward();")
            .map_err(|e: tauri::Error| WebDriverErrorResponse::javascript_error(&e.to_string()))?;
    }

    Ok(WebDriverResponse::null())
}

/// POST /session/{session_id}/refresh - Refresh page
pub async fn refresh<R: Runtime + 'static>(
    State(state): State<Arc<AppState<R>>>,
    Path(session_id): Path<String>,
) -> WebDriverResult {
    let sessions = state.sessions.read().await;
    let _session = sessions
        .get(&session_id)
        .ok_or_else(|| WebDriverErrorResponse::invalid_session_id(&session_id))?;
    drop(sessions);

    if let Some(webview) = state.app.webview_windows().values().next() {
        webview
            .eval("window.location.reload();")
            .map_err(|e: tauri::Error| WebDriverErrorResponse::javascript_error(&e.to_string()))?;
    }

    Ok(WebDriverResponse::null())
}
