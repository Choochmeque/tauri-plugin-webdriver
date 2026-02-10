use std::sync::Arc;

use axum::extract::{Path, State};
use axum::Json;
use serde::Deserialize;
use serde_json::Value;
use tauri::{Manager, Runtime};

use crate::platform::WebViewExecutor;

#[cfg(target_os = "macos")]
use crate::server::response::{WebDriverErrorResponse, WebDriverResponse, WebDriverResult};
use crate::server::AppState;

#[derive(Debug, Deserialize)]
pub struct ExecuteScriptRequest {
    pub script: String,
    #[serde(default)]
    pub args: Vec<Value>,
}

/// POST /session/{session_id}/execute/sync - Execute synchronous script
pub async fn execute_sync<R: Runtime + 'static>(
    State(state): State<Arc<AppState<R>>>,
    Path(session_id): Path<String>,
    Json(request): Json<ExecuteScriptRequest>,
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
            let result = executor.execute_script(&request.script, &request.args).await?;
            return Ok(WebDriverResponse::success(result));
        }
    }

    #[cfg(not(target_os = "macos"))]
    {
        let args_json = serde_json::to_string(&request.args)
            .map_err(|e| WebDriverErrorResponse::invalid_argument(&e.to_string()))?;

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
    }

    Ok(WebDriverResponse::null())
}

/// POST /session/{session_id}/execute/async - Execute asynchronous script
pub async fn execute_async<R: Runtime + 'static>(
    State(state): State<Arc<AppState<R>>>,
    Path(session_id): Path<String>,
    Json(request): Json<ExecuteScriptRequest>,
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
            let result = executor.execute_async_script(&request.script, &request.args).await?;
            return Ok(WebDriverResponse::success(result));
        }
    }

    #[cfg(not(target_os = "macos"))]
    {
        let args_json = serde_json::to_string(&request.args)
            .map_err(|e| WebDriverErrorResponse::invalid_argument(&e.to_string()))?;

        let wrapper = format!(
            r#"
            (function() {{
                var args = {};
                var callback = function(result) {{
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
    }

    Ok(WebDriverResponse::null())
}
