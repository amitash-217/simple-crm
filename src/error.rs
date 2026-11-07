use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde_json::json;

#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("not found")]
    NotFound,
    #[error("{0}")]
    BadRequest(String),
    #[error("authentication required")]
    Unauthorized,
    #[error("{0}")]
    Forbidden(String),
    #[error("{0}")]
    Conflict(String),
    #[error("database error: {0}")]
    Db(#[from] rusqlite::Error),
    #[error("internal error: {0}")]
    Internal(String),
}

/// Malformed or mistyped JSON bodies answer in the API's own error format.
impl From<axum::extract::rejection::JsonRejection> for AppError {
    fn from(rejection: axum::extract::rejection::JsonRejection) -> Self {
        AppError::BadRequest(rejection.body_text())
    }
}

impl From<axum::extract::rejection::QueryRejection> for AppError {
    fn from(rejection: axum::extract::rejection::QueryRejection) -> Self {
        AppError::BadRequest(rejection.body_text())
    }
}

impl AppError {
    fn status_and_code(&self) -> (StatusCode, &'static str) {
        match self {
            AppError::NotFound => (StatusCode::NOT_FOUND, "not_found"),
            AppError::BadRequest(_) => (StatusCode::BAD_REQUEST, "bad_request"),
            AppError::Unauthorized => (StatusCode::UNAUTHORIZED, "unauthorized"),
            AppError::Forbidden(_) => (StatusCode::FORBIDDEN, "forbidden"),
            AppError::Conflict(_) => (StatusCode::CONFLICT, "conflict"),
            AppError::Db(_) | AppError::Internal(_) => {
                (StatusCode::INTERNAL_SERVER_ERROR, "internal")
            }
        }
    }
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, code) = self.status_and_code();
        if status.is_server_error() {
            tracing::error!("{self}");
        }
        let message = if status.is_server_error() {
            "internal server error".to_string()
        } else {
            self.to_string()
        };
        (status, Json(json!({ "error": { "code": code, "message": message } }))).into_response()
    }
}
