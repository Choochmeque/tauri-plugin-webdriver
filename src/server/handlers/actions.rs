use std::sync::Arc;

use axum::extract::{Path, State};
use axum::Json;
use serde::Deserialize;
use tauri::{Manager, Runtime};

#[cfg(target_os = "macos")]
use crate::platform::macos::WebViewExecutor;
use crate::server::response::{WebDriverErrorResponse, WebDriverResponse, WebDriverResult};
use crate::server::AppState;

#[derive(Debug, Deserialize)]
pub struct ActionsRequest {
    pub actions: Vec<ActionSequence>,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "type")]
pub enum ActionSequence {
    #[serde(rename = "key")]
    Key {
        id: String,
        actions: Vec<KeyAction>,
    },
    #[serde(rename = "pointer")]
    Pointer {
        id: String,
        actions: Vec<PointerAction>,
    },
    #[serde(rename = "none")]
    None {
        id: String,
        actions: Vec<PauseAction>,
    },
}

#[derive(Debug, Deserialize)]
#[serde(tag = "type")]
pub enum KeyAction {
    #[serde(rename = "keyDown")]
    KeyDown { value: String },
    #[serde(rename = "keyUp")]
    KeyUp { value: String },
    #[serde(rename = "pause")]
    Pause { duration: Option<u64> },
}

#[derive(Debug, Deserialize)]
#[serde(tag = "type")]
pub enum PointerAction {
    #[serde(rename = "pointerDown")]
    PointerDown { button: u32 },
    #[serde(rename = "pointerUp")]
    PointerUp { button: u32 },
    #[serde(rename = "pointerMove")]
    PointerMove {
        x: i32,
        y: i32,
        duration: Option<u64>,
    },
    #[serde(rename = "pause")]
    Pause { duration: Option<u64> },
}

#[derive(Debug, Deserialize)]
#[serde(tag = "type")]
pub enum PauseAction {
    #[serde(rename = "pause")]
    Pause { duration: Option<u64> },
}

/// POST /session/{session_id}/actions - Perform actions
pub async fn perform<R: Runtime + 'static>(
    State(state): State<Arc<AppState<R>>>,
    Path(session_id): Path<String>,
    Json(request): Json<ActionsRequest>,
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

            for action_seq in &request.actions {
                match action_seq {
                    ActionSequence::Key { actions, .. } => {
                        for action in actions {
                            match action {
                                KeyAction::KeyDown { value } => {
                                    executor.dispatch_key_event(value, true).await?;
                                }
                                KeyAction::KeyUp { value } => {
                                    executor.dispatch_key_event(value, false).await?;
                                }
                                KeyAction::Pause { duration } => {
                                    if let Some(ms) = duration {
                                        tokio::time::sleep(std::time::Duration::from_millis(*ms)).await;
                                    }
                                }
                            }
                        }
                    }
                    ActionSequence::Pointer { actions, .. } => {
                        for action in actions {
                            match action {
                                PointerAction::Pause { duration } => {
                                    if let Some(ms) = duration {
                                        tokio::time::sleep(std::time::Duration::from_millis(*ms)).await;
                                    }
                                }
                                // TODO: Implement pointer actions
                                _ => {}
                            }
                        }
                    }
                    ActionSequence::None { actions, .. } => {
                        for action in actions {
                            match action {
                                PauseAction::Pause { duration } => {
                                    if let Some(ms) = duration {
                                        tokio::time::sleep(std::time::Duration::from_millis(*ms)).await;
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    Ok(WebDriverResponse::null())
}

/// DELETE /session/{session_id}/actions - Release actions
pub async fn release<R: Runtime>(
    State(state): State<Arc<AppState<R>>>,
    Path(session_id): Path<String>,
) -> WebDriverResult {
    let sessions = state.sessions.read().await;
    let _session = sessions
        .get(&session_id)
        .ok_or_else(|| WebDriverErrorResponse::invalid_session_id(&session_id))?;

    // Release all pressed keys/buttons (no-op for now)
    Ok(WebDriverResponse::null())
}
