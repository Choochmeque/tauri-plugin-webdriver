use std::sync::Arc;

use axum::{
    routing::{delete, get, post},
    Router,
};
use tauri::Runtime;

use super::handlers;
use super::AppState;

/// Create the WebDriver router with all W3C WebDriver endpoints
pub fn create_router<R: Runtime + 'static>(state: Arc<AppState<R>>) -> Router {
    Router::new()
        // Status
        .route("/status", get(handlers::status::<R>))
        // Session management
        .route("/session", post(handlers::session::create::<R>))
        .route("/session/{session_id}", delete(handlers::session::delete::<R>))
        // Navigation
        .route(
            "/session/{session_id}/url",
            get(handlers::navigation::get_url::<R>).post(handlers::navigation::navigate::<R>),
        )
        .route("/session/{session_id}/title", get(handlers::navigation::get_title::<R>))
        .route("/session/{session_id}/back", post(handlers::navigation::back::<R>))
        .route("/session/{session_id}/forward", post(handlers::navigation::forward::<R>))
        .route("/session/{session_id}/refresh", post(handlers::navigation::refresh::<R>))
        // Elements
        .route("/session/{session_id}/element", post(handlers::element::find::<R>))
        .route("/session/{session_id}/elements", post(handlers::element::find_all::<R>))
        .route(
            "/session/{session_id}/element/{element_id}/click",
            post(handlers::element::click::<R>),
        )
        .route(
            "/session/{session_id}/element/{element_id}/clear",
            post(handlers::element::clear::<R>),
        )
        .route(
            "/session/{session_id}/element/{element_id}/value",
            post(handlers::element::send_keys::<R>),
        )
        .route(
            "/session/{session_id}/element/{element_id}/text",
            get(handlers::element::get_text::<R>),
        )
        .route(
            "/session/{session_id}/element/{element_id}/name",
            get(handlers::element::get_tag_name::<R>),
        )
        .route(
            "/session/{session_id}/element/{element_id}/attribute/{name}",
            get(handlers::element::get_attribute::<R>),
        )
        .route(
            "/session/{session_id}/element/{element_id}/property/{name}",
            get(handlers::element::get_property::<R>),
        )
        .route(
            "/session/{session_id}/element/{element_id}/displayed",
            get(handlers::element::is_displayed::<R>),
        )
        .route(
            "/session/{session_id}/element/{element_id}/enabled",
            get(handlers::element::is_enabled::<R>),
        )
        // Execute Script
        .route(
            "/session/{session_id}/execute/sync",
            post(handlers::script::execute_sync::<R>),
        )
        .route(
            "/session/{session_id}/execute/async",
            post(handlers::script::execute_async::<R>),
        )
        // Screenshot
        .route(
            "/session/{session_id}/screenshot",
            get(handlers::screenshot::take::<R>),
        )
        // Document
        .route(
            "/session/{session_id}/source",
            get(handlers::document::get_source::<R>),
        )
        .with_state(state)
}
