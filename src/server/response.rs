use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde::Serialize;
use serde_json::{json, Value};

/// W3C WebDriver success response
#[derive(Debug, Serialize)]
pub struct WebDriverResponse {
    pub value: Value,
}

impl WebDriverResponse {
    pub fn success<T: Serialize>(value: T) -> Self {
        Self {
            value: serde_json::to_value(value).unwrap_or(Value::Null),
        }
    }

    pub fn null() -> Self {
        Self { value: Value::Null }
    }
}

impl IntoResponse for WebDriverResponse {
    fn into_response(self) -> Response {
        (
            StatusCode::OK,
            [("Content-Type", "application/json; charset=utf-8")],
            Json(self),
        )
            .into_response()
    }
}

/// W3C WebDriver error response
#[derive(Debug)]
pub struct WebDriverErrorResponse {
    pub status: StatusCode,
    pub error: String,
    pub message: String,
    pub stacktrace: Option<String>,
}

impl WebDriverErrorResponse {
    pub fn new(status: StatusCode, error: &str, message: &str) -> Self {
        Self {
            status,
            error: error.to_string(),
            message: message.to_string(),
            stacktrace: None,
        }
    }

    pub fn invalid_session_id(session_id: &str) -> Self {
        Self::new(
            StatusCode::NOT_FOUND,
            "invalid session id",
            &format!("Session {} not found", session_id),
        )
    }

    pub fn no_such_element() -> Self {
        Self::new(
            StatusCode::NOT_FOUND,
            "no such element",
            "Unable to locate element",
        )
    }

    pub fn stale_element_reference() -> Self {
        Self::new(
            StatusCode::NOT_FOUND,
            "stale element reference",
            "Element is no longer attached to the DOM",
        )
    }

    pub fn no_such_window() -> Self {
        Self::new(
            StatusCode::NOT_FOUND,
            "no such window",
            "No window could be found",
        )
    }

    pub fn javascript_error(message: &str) -> Self {
        Self::new(StatusCode::INTERNAL_SERVER_ERROR, "javascript error", message)
    }

    pub fn unknown_error(message: &str) -> Self {
        Self::new(StatusCode::INTERNAL_SERVER_ERROR, "unknown error", message)
    }

    pub fn invalid_argument(message: &str) -> Self {
        Self::new(StatusCode::BAD_REQUEST, "invalid argument", message)
    }
}

impl IntoResponse for WebDriverErrorResponse {
    fn into_response(self) -> Response {
        let body = json!({
            "value": {
                "error": self.error,
                "message": self.message,
                "stacktrace": self.stacktrace.unwrap_or_default()
            }
        });

        (
            self.status,
            [("Content-Type", "application/json; charset=utf-8")],
            Json(body),
        )
            .into_response()
    }
}

/// Result type for WebDriver handlers
pub type WebDriverResult = Result<WebDriverResponse, WebDriverErrorResponse>;
