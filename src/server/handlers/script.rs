use std::sync::Arc;

use axum::extract::{Path, State};
use axum::Json;
use serde::Deserialize;
use serde_json::Value;
use tauri::{Manager, Runtime};

use crate::server::response::{WebDriverErrorResponse, WebDriverResponse, WebDriverResult};
use crate::server::AppState;

#[derive(Debug, Deserialize)]
pub struct ExecuteScriptRequest {
    pub script: String,
    #[serde(default)]
    pub args: Vec<Value>,
}

/// POST /session/{session_id}/execute/sync - Execute synchronous script
pub async fn execute_sync<R: Runtime>(
    State(state): State<Arc<AppState<R>>>,
    Path(session_id): Path<String>,
    Json(request): Json<ExecuteScriptRequest>,
) -> WebDriverResult {
    let sessions = state.sessions.read().await;
    let _session = sessions
        .get(&session_id)
        .ok_or_else(|| WebDriverErrorResponse::invalid_session_id(&session_id))?;

    let args_json = serde_json::to_string(&request.args)
        .map_err(|e| WebDriverErrorResponse::invalid_argument(&e.to_string()))?;

    // Wrap user script in a function and pass arguments
    let wrapper = format!(
        r#"
        (function() {{
            var args = {};
            var fn = function() {{ {} }};
            return fn.apply(null, args);
        }})()
        "#,
        args_json, request.script
    );

    if let Some(webview) = state.app.webview_windows().values().next() {
        webview
            .eval(&wrapper)
            .map_err(|e: tauri::Error| WebDriverErrorResponse::javascript_error(&e.to_string()))?;
    }

    // TODO: Need async JS evaluation to get return value
    Ok(WebDriverResponse::null())
}

/// POST /session/{session_id}/execute/async - Execute asynchronous script
pub async fn execute_async<R: Runtime>(
    State(state): State<Arc<AppState<R>>>,
    Path(session_id): Path<String>,
    Json(request): Json<ExecuteScriptRequest>,
) -> WebDriverResult {
    let sessions = state.sessions.read().await;
    let _session = sessions
        .get(&session_id)
        .ok_or_else(|| WebDriverErrorResponse::invalid_session_id(&session_id))?;

    let args_json = serde_json::to_string(&request.args)
        .map_err(|e| WebDriverErrorResponse::invalid_argument(&e.to_string()))?;

    // For async scripts, the last argument is the callback
    let wrapper = format!(
        r#"
        (function() {{
            var args = {};
            var callback = function(result) {{
                // TODO: Send result back via message handler
                console.log('Async script result:', result);
            }};
            args.push(callback);
            var fn = function() {{ {} }};
            fn.apply(null, args);
        }})()
        "#,
        args_json, request.script
    );

    if let Some(webview) = state.app.webview_windows().values().next() {
        webview
            .eval(&wrapper)
            .map_err(|e: tauri::Error| WebDriverErrorResponse::javascript_error(&e.to_string()))?;
    }

    // TODO: Need to wait for callback
    Ok(WebDriverResponse::null())
}
