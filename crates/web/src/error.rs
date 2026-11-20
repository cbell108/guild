//! Mapping of domain and store errors onto the `ApiError` JSON body and HTTP status.

use api::{ApiError, ErrorCode};
use axum::Json;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use hangouts_core::DomainError;
use store::StoreError;

/// An error that renders as `{"error":{"code","message"}}` with the matching status.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WebError {
    /// Machine-readable code.
    pub code: ErrorCode,
    /// User-facing message.
    pub message: String,
}

/// Result type of every handler.
pub type WebResult<T> = Result<T, WebError>;

impl WebError {
    /// Builds an error.
    pub fn new(code: ErrorCode, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
        }
    }

    /// 400.
    pub fn bad_request(message: impl Into<String>) -> Self {
        Self::new(ErrorCode::BadRequest, message)
    }

    /// 401.
    pub fn unauthorized() -> Self {
        Self::new(ErrorCode::Unauthorized, "Please log in first.")
    }

    /// 403.
    pub fn forbidden(message: impl Into<String>) -> Self {
        Self::new(ErrorCode::Forbidden, message)
    }

    /// 404.
    pub fn not_found(what: &str) -> Self {
        Self::new(ErrorCode::NotFound, format!("{what} not found."))
    }

    /// HTTP status for the code.
    pub const fn status(&self) -> StatusCode {
        match self.code {
            ErrorCode::BadRequest | ErrorCode::InvalidToken => StatusCode::BAD_REQUEST,
            ErrorCode::Unauthorized => StatusCode::UNAUTHORIZED,
            ErrorCode::Forbidden => StatusCode::FORBIDDEN,
            ErrorCode::NotFound => StatusCode::NOT_FOUND,
            ErrorCode::Conflict => StatusCode::CONFLICT,
            ErrorCode::LlmUnavailable => StatusCode::SERVICE_UNAVAILABLE,
            ErrorCode::Internal => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}

impl IntoResponse for WebError {
    fn into_response(self) -> Response {
        (self.status(), Json(ApiError::new(self.code, self.message))).into_response()
    }
}

impl From<DomainError> for WebError {
    fn from(err: DomainError) -> Self {
        Self::bad_request(err.to_string())
    }
}

impl From<hangouts_core::TimeError> for WebError {
    fn from(err: hangouts_core::TimeError) -> Self {
        Self::bad_request(err.to_string())
    }
}

impl From<StoreError> for WebError {
    fn from(err: StoreError) -> Self {
        match err {
            StoreError::NotFound => Self::new(ErrorCode::NotFound, "Not found."),
            StoreError::Conflict(msg) => Self::new(ErrorCode::Conflict, msg),
            StoreError::Invalid(msg) => Self::bad_request(msg),
            other => {
                tracing::error!("store error: {other}");
                Self::new(ErrorCode::Internal, "Something went wrong on our side.")
            }
        }
    }
}
