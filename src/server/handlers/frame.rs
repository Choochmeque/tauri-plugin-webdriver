use std::sync::Arc;

use axum::extract::{Path, State};
use axum::Json;
use serde::Deserialize;
use serde_json::Value;
use tauri::Runtime;

use crate::platform::FrameId;
use crate::server::response::{WebDriverErrorResponse, WebDriverResponse, WebDriverResult};
use crate::server::AppState;

#[derive(Debug, Deserialize)]
pub struct SwitchFrameRequest {
    pub id: Value,
}

/// POST `/session/{session_id}/frame` - Switch to frame
pub async fn switch_to_frame<R: Runtime + 'static>(
    State(state): State<Arc<AppState<R>>>,
    Path(session_id): Path<String>,
    Json(request): Json<SwitchFrameRequest>,
) -> WebDriverResult {
    let sessions = state.sessions.read().await;
    let session = sessions.get(&session_id)?;

    // Parse the frame ID
    let frame_id = match &request.id {
        Value::Null => FrameId::Top,
        Value::Number(n) => {
            let index = n.as_u64().ok_or_else(|| {
                WebDriverErrorResponse::invalid_argument(
                    "Frame index must be a non-negative integer",
                )
            })?;
            let index = u32::try_from(index)
                .map_err(|_| WebDriverErrorResponse::invalid_argument("Frame index too large"))?;
            FrameId::Index(index)
        }
        Value::Object(obj) => {
            // W3C element reference format
            if let Some(element_id) = obj.get("element-6066-11e4-a52e-4f735466cecf") {
                let element_id = element_id.as_str().ok_or_else(|| {
                    WebDriverErrorResponse::invalid_argument("Element reference must be a string")
                })?;

                // Look up the element's js_var
                let element = session
                    .elements
                    .get(element_id)
                    .ok_or_else(WebDriverErrorResponse::no_such_element)?;

                FrameId::Element(element.js_ref.clone())
            } else {
                return Err(WebDriverErrorResponse::invalid_argument(
                    "Invalid frame identifier object",
                ));
            }
        }
        _ => {
            return Err(WebDriverErrorResponse::invalid_argument(
                "Frame ID must be null, a number, or an element reference",
            ));
        }
    };

    let current_window = session.current_window.clone();
    let timeouts = session.timeouts.clone();
    drop(sessions);

    let executor = state.get_executor_for_window(&current_window, timeouts)?;
    executor.switch_to_frame(frame_id).await?;

    Ok(WebDriverResponse::null())
}

/// POST `/session/{session_id}/frame/parent` - Switch to parent frame
pub async fn switch_to_parent_frame<R: Runtime + 'static>(
    State(state): State<Arc<AppState<R>>>,
    Path(session_id): Path<String>,
) -> WebDriverResult {
    let sessions = state.sessions.read().await;
    let session = sessions.get(&session_id)?;
    let current_window = session.current_window.clone();
    let timeouts = session.timeouts.clone();
    drop(sessions);

    let executor = state.get_executor_for_window(&current_window, timeouts)?;
    executor.switch_to_parent_frame().await?;

    Ok(WebDriverResponse::null())
}
