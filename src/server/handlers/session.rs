use std::sync::Arc;

use axum::extract::{Path, State};
use axum::Json;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use tauri::Runtime;

use crate::server::response::{WebDriverErrorResponse, WebDriverResponse, WebDriverResult};
use crate::server::AppState;

/// Wait for a window to become available, polling with timeout
async fn wait_for_window<R: Runtime>(
    state: &AppState<R>,
    timeout_ms: u64,
) -> Result<String, WebDriverErrorResponse> {
    let start = std::time::Instant::now();
    let timeout = std::time::Duration::from_millis(timeout_ms);
    let poll_interval = std::time::Duration::from_millis(100);

    loop {
        let window_labels = state.get_window_labels();
        if let Some(label) = window_labels.first().cloned() {
            return Ok(label);
        }

        if start.elapsed() >= timeout {
            return Err(WebDriverErrorResponse::no_such_window());
        }

        tokio::time::sleep(poll_interval).await;
    }
}

#[derive(Debug, Deserialize)]
pub struct CreateSessionRequest {
    pub capabilities: Capabilities,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Capabilities {
    #[serde(default)]
    pub always_match: Value,
    #[serde(default)]
    pub first_match: Vec<Value>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionResponse {
    pub session_id: String,
    pub capabilities: Value,
}

/// POST /session - Create a new session
pub async fn create<R: Runtime>(
    State(state): State<Arc<AppState<R>>>,
    Json(request): Json<CreateSessionRequest>,
) -> WebDriverResult {
    // Wait for a window to become available (up to 10 seconds)
    let initial_window = wait_for_window(&state, 10_000).await?;

    let mut sessions = state.sessions.write().await;

    // Create session with capabilities and initial window
    let session = sessions.create(request.capabilities.always_match.clone(), initial_window);

    let response = SessionResponse {
        session_id: session.id.clone(),
        capabilities: json!({
            "browserName": "tauri",
            "browserVersion": "2.0",
            "platformName": std::env::consts::OS,
            "acceptInsecureCerts": false,
            "pageLoadStrategy": "normal",
            "timeouts": {
                "implicit": session.timeouts.implicit_ms,
                "pageLoad": session.timeouts.page_load_ms,
                "script": session.timeouts.script_ms
            }
        }),
    };

    Ok(WebDriverResponse::success(response))
}

/// DELETE /session/{session_id} - Delete a session
pub async fn delete<R: Runtime>(
    State(state): State<Arc<AppState<R>>>,
    Path(session_id): Path<String>,
) -> WebDriverResult {
    let mut sessions = state.sessions.write().await;

    if sessions.delete(&session_id) {
        Ok(WebDriverResponse::null())
    } else {
        Err(WebDriverErrorResponse::invalid_session_id(&session_id))
    }
}
