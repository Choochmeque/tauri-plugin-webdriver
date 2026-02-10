use std::sync::Arc;

use axum::extract::{Path, State};
use axum::Json;
use serde::Deserialize;
use serde_json::json;
use tauri::{Manager, Runtime};

use crate::server::response::{WebDriverErrorResponse, WebDriverResponse, WebDriverResult};
use crate::server::AppState;
use crate::webdriver::locator::LocatorStrategy;

#[derive(Debug, Deserialize)]
pub struct FindElementRequest {
    pub using: String,
    pub value: String,
}

#[derive(Debug, Deserialize)]
pub struct SendKeysRequest {
    pub text: String,
}

/// POST /session/{session_id}/element - Find element
pub async fn find<R: Runtime>(
    State(state): State<Arc<AppState<R>>>,
    Path(session_id): Path<String>,
    Json(request): Json<FindElementRequest>,
) -> WebDriverResult {
    let mut sessions = state.sessions.write().await;
    let session = sessions
        .get_mut(&session_id)
        .ok_or_else(|| WebDriverErrorResponse::invalid_session_id(&session_id))?;

    let strategy = LocatorStrategy::from_string(&request.using)
        .ok_or_else(|| WebDriverErrorResponse::invalid_argument(&format!("Unknown locator strategy: {}", request.using)))?;

    // Store element reference and get ID
    let element_ref = session.elements.store();
    let js_code = strategy.to_find_js(&request.value, false, &element_ref.js_ref);

    // Execute JavaScript to find element
    if let Some(webview) = state.app.webview_windows().values().next() {
        webview
            .eval(&js_code)
            .map_err(|e: tauri::Error| WebDriverErrorResponse::javascript_error(&e.to_string()))?;
    }

    // Return element reference
    // Note: In a real implementation, we'd need to verify the element was found
    Ok(WebDriverResponse::success(json!({
        "element-6066-11e4-a52e-4f735466cecf": element_ref.id
    })))
}

/// POST /session/{session_id}/elements - Find multiple elements
pub async fn find_all<R: Runtime>(
    State(state): State<Arc<AppState<R>>>,
    Path(session_id): Path<String>,
    Json(request): Json<FindElementRequest>,
) -> WebDriverResult {
    let sessions = state.sessions.read().await;
    let _session = sessions
        .get(&session_id)
        .ok_or_else(|| WebDriverErrorResponse::invalid_session_id(&session_id))?;

    let _strategy = LocatorStrategy::from_string(&request.using)
        .ok_or_else(|| WebDriverErrorResponse::invalid_argument(&format!("Unknown locator strategy: {}", request.using)))?;

    // TODO: Implement finding multiple elements
    // For now, return empty array
    Ok(WebDriverResponse::success(Vec::<serde_json::Value>::new()))
}

/// POST /session/{session_id}/element/{element_id}/click - Click element
pub async fn click<R: Runtime>(
    State(state): State<Arc<AppState<R>>>,
    Path((session_id, element_id)): Path<(String, String)>,
) -> WebDriverResult {
    let sessions = state.sessions.read().await;
    let session = sessions
        .get(&session_id)
        .ok_or_else(|| WebDriverErrorResponse::invalid_session_id(&session_id))?;

    let element = session
        .elements
        .get(&element_id)
        .ok_or_else(|| WebDriverErrorResponse::no_such_element())?;

    let script = format!(
        r#"
        (function() {{
            var el = window.{};
            if (!el || !document.contains(el)) return {{ stale: true }};
            el.scrollIntoView({{ block: 'center', inline: 'center' }});
            el.click();
            return {{ success: true }};
        }})()
        "#,
        element.js_ref
    );

    if let Some(webview) = state.app.webview_windows().values().next() {
        webview
            .eval(&script)
            .map_err(|e: tauri::Error| WebDriverErrorResponse::javascript_error(&e.to_string()))?;
    }

    Ok(WebDriverResponse::null())
}

/// POST /session/{session_id}/element/{element_id}/clear - Clear element
pub async fn clear<R: Runtime>(
    State(state): State<Arc<AppState<R>>>,
    Path((session_id, element_id)): Path<(String, String)>,
) -> WebDriverResult {
    let sessions = state.sessions.read().await;
    let session = sessions
        .get(&session_id)
        .ok_or_else(|| WebDriverErrorResponse::invalid_session_id(&session_id))?;

    let element = session
        .elements
        .get(&element_id)
        .ok_or_else(|| WebDriverErrorResponse::no_such_element())?;

    let script = format!(
        r#"
        (function() {{
            var el = window.{};
            if (!el || !document.contains(el)) return {{ stale: true }};
            if (el.tagName === 'INPUT' || el.tagName === 'TEXTAREA') {{
                el.value = '';
                el.dispatchEvent(new Event('input', {{ bubbles: true }}));
                el.dispatchEvent(new Event('change', {{ bubbles: true }}));
            }}
            return {{ success: true }};
        }})()
        "#,
        element.js_ref
    );

    if let Some(webview) = state.app.webview_windows().values().next() {
        webview
            .eval(&script)
            .map_err(|e: tauri::Error| WebDriverErrorResponse::javascript_error(&e.to_string()))?;
    }

    Ok(WebDriverResponse::null())
}

/// POST /session/{session_id}/element/{element_id}/value - Send keys to element
pub async fn send_keys<R: Runtime>(
    State(state): State<Arc<AppState<R>>>,
    Path((session_id, element_id)): Path<(String, String)>,
    Json(request): Json<SendKeysRequest>,
) -> WebDriverResult {
    let sessions = state.sessions.read().await;
    let session = sessions
        .get(&session_id)
        .ok_or_else(|| WebDriverErrorResponse::invalid_session_id(&session_id))?;

    let element = session
        .elements
        .get(&element_id)
        .ok_or_else(|| WebDriverErrorResponse::no_such_element())?;

    let escaped_text = request.text.replace('\\', "\\\\").replace('`', "\\`");

    let script = format!(
        r#"
        (function() {{
            var el = window.{};
            if (!el || !document.contains(el)) return {{ stale: true }};
            el.focus();
            if (el.tagName === 'INPUT' || el.tagName === 'TEXTAREA') {{
                el.value += `{}`;
                el.dispatchEvent(new Event('input', {{ bubbles: true }}));
                el.dispatchEvent(new Event('change', {{ bubbles: true }}));
            }} else if (el.isContentEditable) {{
                document.execCommand('insertText', false, `{}`);
            }}
            return {{ success: true }};
        }})()
        "#,
        element.js_ref, escaped_text, escaped_text
    );

    if let Some(webview) = state.app.webview_windows().values().next() {
        webview
            .eval(&script)
            .map_err(|e: tauri::Error| WebDriverErrorResponse::javascript_error(&e.to_string()))?;
    }

    Ok(WebDriverResponse::null())
}

/// GET /session/{session_id}/element/{element_id}/text - Get element text
pub async fn get_text<R: Runtime>(
    State(state): State<Arc<AppState<R>>>,
    Path((session_id, element_id)): Path<(String, String)>,
) -> WebDriverResult {
    let sessions = state.sessions.read().await;
    let session = sessions
        .get(&session_id)
        .ok_or_else(|| WebDriverErrorResponse::invalid_session_id(&session_id))?;

    let _element = session
        .elements
        .get(&element_id)
        .ok_or_else(|| WebDriverErrorResponse::no_such_element())?;

    // TODO: Need async JS evaluation to get result back
    // For now return empty string
    Ok(WebDriverResponse::success(""))
}

/// GET /session/{session_id}/element/{element_id}/name - Get element tag name
pub async fn get_tag_name<R: Runtime>(
    State(state): State<Arc<AppState<R>>>,
    Path((session_id, element_id)): Path<(String, String)>,
) -> WebDriverResult {
    let sessions = state.sessions.read().await;
    let session = sessions
        .get(&session_id)
        .ok_or_else(|| WebDriverErrorResponse::invalid_session_id(&session_id))?;

    let _element = session
        .elements
        .get(&element_id)
        .ok_or_else(|| WebDriverErrorResponse::no_such_element())?;

    // TODO: Need async JS evaluation
    Ok(WebDriverResponse::success(""))
}

/// GET /session/{session_id}/element/{element_id}/attribute/{name} - Get element attribute
pub async fn get_attribute<R: Runtime>(
    State(state): State<Arc<AppState<R>>>,
    Path((session_id, element_id, _name)): Path<(String, String, String)>,
) -> WebDriverResult {
    let sessions = state.sessions.read().await;
    let session = sessions
        .get(&session_id)
        .ok_or_else(|| WebDriverErrorResponse::invalid_session_id(&session_id))?;

    let _element = session
        .elements
        .get(&element_id)
        .ok_or_else(|| WebDriverErrorResponse::no_such_element())?;

    // TODO: Need async JS evaluation
    Ok(WebDriverResponse::null())
}

/// GET /session/{session_id}/element/{element_id}/property/{name} - Get element property
pub async fn get_property<R: Runtime>(
    State(state): State<Arc<AppState<R>>>,
    Path((session_id, element_id, _name)): Path<(String, String, String)>,
) -> WebDriverResult {
    let sessions = state.sessions.read().await;
    let session = sessions
        .get(&session_id)
        .ok_or_else(|| WebDriverErrorResponse::invalid_session_id(&session_id))?;

    let _element = session
        .elements
        .get(&element_id)
        .ok_or_else(|| WebDriverErrorResponse::no_such_element())?;

    Ok(WebDriverResponse::null())
}

/// GET /session/{session_id}/element/{element_id}/displayed - Is element displayed
pub async fn is_displayed<R: Runtime>(
    State(state): State<Arc<AppState<R>>>,
    Path((session_id, element_id)): Path<(String, String)>,
) -> WebDriverResult {
    let sessions = state.sessions.read().await;
    let session = sessions
        .get(&session_id)
        .ok_or_else(|| WebDriverErrorResponse::invalid_session_id(&session_id))?;

    let _element = session
        .elements
        .get(&element_id)
        .ok_or_else(|| WebDriverErrorResponse::no_such_element())?;

    // TODO: Need async JS evaluation
    Ok(WebDriverResponse::success(true))
}

/// GET /session/{session_id}/element/{element_id}/enabled - Is element enabled
pub async fn is_enabled<R: Runtime>(
    State(state): State<Arc<AppState<R>>>,
    Path((session_id, element_id)): Path<(String, String)>,
) -> WebDriverResult {
    let sessions = state.sessions.read().await;
    let session = sessions
        .get(&session_id)
        .ok_or_else(|| WebDriverErrorResponse::invalid_session_id(&session_id))?;

    let _element = session
        .elements
        .get(&element_id)
        .ok_or_else(|| WebDriverErrorResponse::no_such_element())?;

    // TODO: Need async JS evaluation
    Ok(WebDriverResponse::success(true))
}
