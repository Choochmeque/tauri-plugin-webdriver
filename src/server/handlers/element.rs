use std::sync::Arc;

use axum::extract::{Path, State};
use axum::Json;
use serde::Deserialize;
use serde_json::json;
use tauri::{Manager, Runtime};

#[cfg(target_os = "macos")]
use crate::platform::macos::WebViewExecutor;
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
pub async fn find<R: Runtime + 'static>(
    State(state): State<Arc<AppState<R>>>,
    Path(session_id): Path<String>,
    Json(request): Json<FindElementRequest>,
) -> WebDriverResult {
    let mut sessions = state.sessions.write().await;
    let session = sessions
        .get_mut(&session_id)
        .ok_or_else(|| WebDriverErrorResponse::invalid_session_id(&session_id))?;

    let strategy = LocatorStrategy::from_string(&request.using).ok_or_else(|| {
        WebDriverErrorResponse::invalid_argument(&format!(
            "Unknown locator strategy: {}",
            request.using
        ))
    })?;

    // Store element reference and get ID
    let element_ref = session.elements.store();
    let js_var = element_ref.js_ref.clone();
    let element_id = element_ref.id.clone();
    drop(sessions);

    let strategy_js = strategy.to_selector_js(&request.value);

    #[cfg(target_os = "macos")]
    {
        if let Some(window) = state.app.webview_windows().values().next().cloned() {
            let executor = WebViewExecutor::new(window);
            let found = executor.find_element(&strategy_js, &js_var).await?;
            if !found {
                return Err(WebDriverErrorResponse::no_such_element());
            }
        }
    }

    #[cfg(not(target_os = "macos"))]
    {
        let js_code = format!(
            r#"(function() {{
                var el = {};
                if (el) {{
                    window.{} = el;
                    return true;
                }}
                return false;
            }})()"#,
            strategy_js, js_var
        );
        if let Some(webview) = state.app.webview_windows().values().next() {
            webview
                .eval(&js_code)
                .map_err(|e: tauri::Error| WebDriverErrorResponse::javascript_error(&e.to_string()))?;
        }
    }

    Ok(WebDriverResponse::success(json!({
        "element-6066-11e4-a52e-4f735466cecf": element_id
    })))
}

/// POST /session/{session_id}/elements - Find multiple elements
pub async fn find_all<R: Runtime + 'static>(
    State(state): State<Arc<AppState<R>>>,
    Path(session_id): Path<String>,
    Json(request): Json<FindElementRequest>,
) -> WebDriverResult {
    let mut sessions = state.sessions.write().await;
    let session = sessions
        .get_mut(&session_id)
        .ok_or_else(|| WebDriverErrorResponse::invalid_session_id(&session_id))?;

    let strategy = LocatorStrategy::from_string(&request.using).ok_or_else(|| {
        WebDriverErrorResponse::invalid_argument(&format!(
            "Unknown locator strategy: {}",
            request.using
        ))
    })?;

    // Generate the JS code to find multiple elements
    let find_js = strategy.to_find_js(&request.value, true, "__wd_temp_els");
    drop(sessions);

    #[cfg(target_os = "macos")]
    {
        if let Some(window) = state.app.webview_windows().values().next().cloned() {
            let executor = WebViewExecutor::new(window);

            // First, find all elements and get count
            let count_script = format!(
                r#"
                var els = {};
                window.__wd_temp_els = els;
                return els ? els.length : 0;
                "#,
                strategy.to_selector_js_multiple(&request.value)
            );

            let result = executor.evaluate_js(&count_script).await?;
            let count = if let Some(success) = result.get("success").and_then(|v| v.as_bool()) {
                if success {
                    result.get("value").and_then(|v| v.as_u64()).unwrap_or(0) as usize
                } else {
                    0
                }
            } else {
                0
            };

            // Store each element reference
            let mut elements = Vec::new();
            let mut sessions = state.sessions.write().await;
            let session = sessions.get_mut(&session_id).ok_or_else(|| {
                WebDriverErrorResponse::invalid_session_id(&session_id)
            })?;

            for i in 0..count {
                let element_ref = session.elements.store();
                let js_var = element_ref.js_ref.clone();
                let element_id = element_ref.id.clone();

                // Store each element in its own global variable
                let store_script = format!(
                    "window.{} = window.__wd_temp_els[{}]; return true;",
                    js_var, i
                );
                let _ = executor.evaluate_js(&store_script).await;

                elements.push(json!({
                    "element-6066-11e4-a52e-4f735466cecf": element_id
                }));
            }

            return Ok(WebDriverResponse::success(elements));
        }
    }

    Ok(WebDriverResponse::success(Vec::<serde_json::Value>::new()))
}

/// POST /session/{session_id}/element/{element_id}/click - Click element
pub async fn click<R: Runtime + 'static>(
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

    let js_var = element.js_ref.clone();
    drop(sessions);

    #[cfg(target_os = "macos")]
    {
        if let Some(window) = state.app.webview_windows().values().next().cloned() {
            let executor = WebViewExecutor::new(window);
            executor.click_element(&js_var).await?;
        }
    }

    #[cfg(not(target_os = "macos"))]
    {
        let script = format!(
            r#"(function() {{
                var el = window.{};
                if (!el || !document.contains(el)) return {{ stale: true }};
                el.scrollIntoView({{ block: 'center', inline: 'center' }});
                el.click();
                return {{ success: true }};
            }})()"#,
            js_var
        );
        if let Some(webview) = state.app.webview_windows().values().next() {
            webview
                .eval(&script)
                .map_err(|e: tauri::Error| WebDriverErrorResponse::javascript_error(&e.to_string()))?;
        }
    }

    Ok(WebDriverResponse::null())
}

/// POST /session/{session_id}/element/{element_id}/clear - Clear element
pub async fn clear<R: Runtime + 'static>(
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

    let js_var = element.js_ref.clone();
    drop(sessions);

    let script = format!(
        r#"(function() {{
            var el = window.{};
            if (!el || !document.contains(el)) return {{ stale: true }};
            if (el.tagName === 'INPUT' || el.tagName === 'TEXTAREA') {{
                el.value = '';
                el.dispatchEvent(new Event('input', {{ bubbles: true }}));
                el.dispatchEvent(new Event('change', {{ bubbles: true }}));
            }}
            return {{ success: true }};
        }})()"#,
        js_var
    );

    if let Some(webview) = state.app.webview_windows().values().next() {
        webview
            .eval(&script)
            .map_err(|e: tauri::Error| WebDriverErrorResponse::javascript_error(&e.to_string()))?;
    }

    Ok(WebDriverResponse::null())
}

/// POST /session/{session_id}/element/{element_id}/value - Send keys to element
pub async fn send_keys<R: Runtime + 'static>(
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

    let js_var = element.js_ref.clone();
    drop(sessions);

    #[cfg(target_os = "macos")]
    {
        if let Some(window) = state.app.webview_windows().values().next().cloned() {
            let executor = WebViewExecutor::new(window);
            executor.send_keys_to_element(&js_var, &request.text).await?;
        }
    }

    #[cfg(not(target_os = "macos"))]
    {
        let escaped_text = request.text.replace('\\', "\\\\").replace('`', "\\`");
        let script = format!(
            r#"(function() {{
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
            }})()"#,
            js_var, escaped_text, escaped_text
        );
        if let Some(webview) = state.app.webview_windows().values().next() {
            webview
                .eval(&script)
                .map_err(|e: tauri::Error| WebDriverErrorResponse::javascript_error(&e.to_string()))?;
        }
    }

    Ok(WebDriverResponse::null())
}

/// GET /session/{session_id}/element/{element_id}/text - Get element text
pub async fn get_text<R: Runtime + 'static>(
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

    let js_var = element.js_ref.clone();
    drop(sessions);

    #[cfg(target_os = "macos")]
    {
        if let Some(window) = state.app.webview_windows().values().next().cloned() {
            let executor = WebViewExecutor::new(window);
            let text = executor.get_element_text(&js_var).await?;
            return Ok(WebDriverResponse::success(text));
        }
    }

    Ok(WebDriverResponse::success(""))
}

/// GET /session/{session_id}/element/{element_id}/name - Get element tag name
pub async fn get_tag_name<R: Runtime + 'static>(
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

    let js_var = element.js_ref.clone();
    drop(sessions);

    #[cfg(target_os = "macos")]
    {
        if let Some(window) = state.app.webview_windows().values().next().cloned() {
            let executor = WebViewExecutor::new(window);
            let tag_name = executor.get_element_tag_name(&js_var).await?;
            return Ok(WebDriverResponse::success(tag_name));
        }
    }

    Ok(WebDriverResponse::success(""))
}

/// GET /session/{session_id}/element/{element_id}/attribute/{name} - Get element attribute
pub async fn get_attribute<R: Runtime + 'static>(
    State(state): State<Arc<AppState<R>>>,
    Path((session_id, element_id, name)): Path<(String, String, String)>,
) -> WebDriverResult {
    let sessions = state.sessions.read().await;
    let session = sessions
        .get(&session_id)
        .ok_or_else(|| WebDriverErrorResponse::invalid_session_id(&session_id))?;

    let element = session
        .elements
        .get(&element_id)
        .ok_or_else(|| WebDriverErrorResponse::no_such_element())?;

    let js_var = element.js_ref.clone();
    drop(sessions);

    #[cfg(target_os = "macos")]
    {
        if let Some(window) = state.app.webview_windows().values().next().cloned() {
            let executor = WebViewExecutor::new(window);
            let attr = executor.get_element_attribute(&js_var, &name).await?;
            return Ok(WebDriverResponse::success(attr));
        }
    }

    Ok(WebDriverResponse::null())
}

/// GET /session/{session_id}/element/{element_id}/property/{name} - Get element property
pub async fn get_property<R: Runtime + 'static>(
    State(state): State<Arc<AppState<R>>>,
    Path((session_id, element_id, name)): Path<(String, String, String)>,
) -> WebDriverResult {
    let sessions = state.sessions.read().await;
    let session = sessions
        .get(&session_id)
        .ok_or_else(|| WebDriverErrorResponse::invalid_session_id(&session_id))?;

    let element = session
        .elements
        .get(&element_id)
        .ok_or_else(|| WebDriverErrorResponse::no_such_element())?;

    let js_var = element.js_ref.clone();
    drop(sessions);

    #[cfg(target_os = "macos")]
    {
        if let Some(window) = state.app.webview_windows().values().next().cloned() {
            let executor = WebViewExecutor::new(window);
            let prop = executor.get_element_property(&js_var, &name).await?;
            return Ok(WebDriverResponse::success(prop));
        }
    }

    Ok(WebDriverResponse::null())
}

/// GET /session/{session_id}/element/{element_id}/displayed - Is element displayed
pub async fn is_displayed<R: Runtime + 'static>(
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

    let js_var = element.js_ref.clone();
    drop(sessions);

    #[cfg(target_os = "macos")]
    {
        if let Some(window) = state.app.webview_windows().values().next().cloned() {
            let executor = WebViewExecutor::new(window);
            let displayed = executor.is_element_displayed(&js_var).await?;
            return Ok(WebDriverResponse::success(displayed));
        }
    }

    Ok(WebDriverResponse::success(true))
}

/// GET /session/{session_id}/element/{element_id}/enabled - Is element enabled
pub async fn is_enabled<R: Runtime + 'static>(
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

    let js_var = element.js_ref.clone();
    drop(sessions);

    #[cfg(target_os = "macos")]
    {
        if let Some(window) = state.app.webview_windows().values().next().cloned() {
            let executor = WebViewExecutor::new(window);
            let enabled = executor.is_element_enabled(&js_var).await?;
            return Ok(WebDriverResponse::success(enabled));
        }
    }

    Ok(WebDriverResponse::success(true))
}
