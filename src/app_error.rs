use axum::http::StatusCode;
use axum::Json;
use axum::response::{IntoResponse, Response};
use log::{info};
use uuid::Uuid;
// use crate::storage::Storage;

#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error(transparent)]
    Any(#[from] anyhow::Error),

    #[error("time error: {0}")]
    TimeError(#[from] std::time::SystemTimeError),

    #[error("internal server error: {0}")]
    JsonError(#[from] serde_json::Error),

    #[error("http error: {0}")]
    HttpError(#[from] reqwest::Error),

    #[error("invalid integer: {0}")]
    TryFromIntError(#[from] std::num::TryFromIntError),

    #[error("generate error: {0}")]
    #[allow(dead_code)]
    GenerateError(String),

    #[error("io error: {0}")]
    IOError(#[from] std::io::Error),

    #[error("sensor error: {0}")]
    TemperatureSensorError(String),

    #[error("regex error: {0}")]
    RegexError(#[from] regex::Error),

    #[error("internal error: {0}")]
    InternalError(String),

    #[error("parse error: {0}")]
    ParseError(String),
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let uuid = Uuid::new_v4();
        let message: String = self.to_string();
        info!("error occurred {}: {}", uuid, message);
        let (http_status, code) = match &self {
            AppError::JsonError(_)        => (StatusCode::INTERNAL_SERVER_ERROR, 1),
            AppError::Any(_)              => (StatusCode::INTERNAL_SERVER_ERROR,   3),
            AppError::HttpError(e) if e.status().map(|s| s.is_client_error()).unwrap_or(false) => (StatusCode::BAD_REQUEST, 4),
            AppError::HttpError(_)        => (StatusCode::BAD_GATEWAY,             4),
            AppError::TryFromIntError(_)  => (StatusCode::BAD_REQUEST,             5),
            AppError::GenerateError(_)    => (StatusCode::BAD_REQUEST,             6),
            AppError::IOError(e) if e.kind() == std::io::ErrorKind::NotFound  => (StatusCode::NOT_FOUND, 7),
            AppError::IOError(_)          => (StatusCode::INTERNAL_SERVER_ERROR,   7),
            AppError::RegexError(_)       => (StatusCode::INTERNAL_SERVER_ERROR,   8),
            AppError::TemperatureSensorError(_)       => (StatusCode::INTERNAL_SERVER_ERROR,   9),
            AppError::InternalError(_)   => (StatusCode::INTERNAL_SERVER_ERROR, 10),
            AppError::TimeError(_)       => (StatusCode::INTERNAL_SERVER_ERROR, 11),
            AppError::ParseError(_)      => (StatusCode::INTERNAL_SERVER_ERROR, 12),

        };

        let body = serde_json::json!({
            "status": "error",
            "code": code,
            "uuid": uuid.to_string(),
        });
        (http_status, Json(body)).into_response()
    }
}

impl<T> From<std::sync::PoisonError<T>> for AppError {
    fn from(_: std::sync::PoisonError<T>) -> Self {
        AppError::InternalError("Mutex lock was poisoned".into())
    }
}