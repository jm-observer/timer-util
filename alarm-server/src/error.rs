use actix_web::{HttpResponse, ResponseError, http::StatusCode};
use serde_json::json;

#[derive(Debug)]
pub enum AppError {
    // Validation errors (400)
    Validation(String),
    // Not found errors (404)
    NotFound(String),
    // Internal server errors (500)
    Internal(String),
}

impl From<rusqlite::Error> for AppError {
    fn from(e: rusqlite::Error) -> Self {
        AppError::Internal(e.to_string())
    }
}

impl From<std::io::Error> for AppError {
    fn from(e: std::io::Error) -> Self {
        AppError::Internal(e.to_string())
    }
}

impl std::fmt::Display for AppError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AppError::Validation(msg) => write!(f, "{}", msg),
            AppError::NotFound(msg) => write!(f, "{}", msg),
            AppError::Internal(msg) => write!(f, "{}", msg),
        }
    }
}

impl ResponseError for AppError {
    fn status_code(&self) -> StatusCode {
        match self {
            AppError::Validation(_) => StatusCode::BAD_REQUEST,
            AppError::NotFound(_) => StatusCode::NOT_FOUND,
            AppError::Internal(_) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    fn error_response(&self) -> HttpResponse {
        let error_msg = match self {
            AppError::Validation(msg) => msg,
            AppError::NotFound(msg) => msg,
            AppError::Internal(msg) => msg,
        };
        HttpResponse::build(self.status_code()).json(json!({"error": error_msg}))
    }
}
