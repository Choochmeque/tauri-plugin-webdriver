use std::sync::Arc;

use axum::extract::{Path, State};
use axum::Json;
use serde::Deserialize;
use tauri::Runtime;

use crate::platform::PointerEventType;
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
        #[serde(rename = "id")]
        _id: String,
        actions: Vec<KeyAction>,
    },
    #[serde(rename = "pointer")]
    Pointer {
        #[serde(rename = "id")]
        _id: String,
        actions: Vec<PointerAction>,
    },
    #[serde(rename = "wheel")]
    Wheel {
        #[serde(rename = "id")]
        _id: String,
        actions: Vec<WheelAction>,
    },
    #[serde(rename = "none")]
    None {
        #[serde(rename = "id")]
        _id: String,
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
pub enum WheelAction {
    #[serde(rename = "scroll")]
    Scroll {
        x: i32,
        y: i32,
        #[serde(rename = "deltaX")]
        delta_x: i32,
        #[serde(rename = "deltaY")]
        delta_y: i32,
        #[serde(default)]
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

/// Current pointer position for actions
struct PointerState {
    x: i32,
    y: i32,
}

/// POST `/session/{session_id}/actions` - Perform actions
#[allow(clippy::too_many_lines)]
pub async fn perform<R: Runtime + 'static>(
    State(state): State<Arc<AppState<R>>>,
    Path(session_id): Path<String>,
    Json(request): Json<ActionsRequest>,
) -> WebDriverResult {
    let sessions = state.sessions.read().await;
    let session = sessions
        .get(&session_id)
        .ok_or_else(|| WebDriverErrorResponse::invalid_session_id(&session_id))?;
    let current_window = session.current_window.clone();
    drop(sessions);

    let executor = state.get_executor_for_window(&current_window)?;
    let mut pointer_state = PointerState { x: 0, y: 0 };

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
                        PointerAction::PointerDown { button } => {
                            executor
                                .dispatch_pointer_event(
                                    PointerEventType::Down,
                                    pointer_state.x,
                                    pointer_state.y,
                                    *button,
                                )
                                .await?;
                        }
                        PointerAction::PointerUp { button } => {
                            executor
                                .dispatch_pointer_event(
                                    PointerEventType::Up,
                                    pointer_state.x,
                                    pointer_state.y,
                                    *button,
                                )
                                .await?;
                        }
                        PointerAction::PointerMove { x, y, duration } => {
                            pointer_state.x = *x;
                            pointer_state.y = *y;
                            if let Some(ms) = duration {
                                if *ms > 0 {
                                    tokio::time::sleep(std::time::Duration::from_millis(*ms)).await;
                                }
                            }
                            executor
                                .dispatch_pointer_event(
                                    PointerEventType::Move,
                                    pointer_state.x,
                                    pointer_state.y,
                                    0,
                                )
                                .await?;
                        }
                        PointerAction::Pause { duration } => {
                            if let Some(ms) = duration {
                                tokio::time::sleep(std::time::Duration::from_millis(*ms)).await;
                            }
                        }
                    }
                }
            }
            ActionSequence::Wheel { actions, .. } => {
                for action in actions {
                    match action {
                        WheelAction::Scroll {
                            x,
                            y,
                            delta_x,
                            delta_y,
                            duration,
                        } => {
                            if let Some(ms) = duration {
                                if *ms > 0 {
                                    tokio::time::sleep(std::time::Duration::from_millis(*ms)).await;
                                }
                            }
                            executor
                                .dispatch_scroll_event(*x, *y, *delta_x, *delta_y)
                                .await?;
                        }
                        WheelAction::Pause { duration } => {
                            if let Some(ms) = duration {
                                tokio::time::sleep(std::time::Duration::from_millis(*ms)).await;
                            }
                        }
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

    Ok(WebDriverResponse::null())
}

/// DELETE `/session/{session_id}/actions` - Release actions
pub async fn release<R: Runtime + 'static>(
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
