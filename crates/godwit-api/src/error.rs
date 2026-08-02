use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use godwit_core::PasteurError;
use serde_json::json;

pub enum ApiError {
    Unauthorized,
    Forbidden,
    NotFound,
    BadRequest(String),
    Internal,
    Core(PasteurError),
}

impl From<PasteurError> for ApiError {
    fn from(err: PasteurError) -> Self {
        match err {
            PasteurError::NotFound => ApiError::NotFound,
            PasteurError::Auth(_) | PasteurError::Forbidden(_) => ApiError::Unauthorized,
            PasteurError::Validation(msg) => ApiError::BadRequest(msg),
            _ => ApiError::Core(err),
        }
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let (status, title, detail) = match &self {
            ApiError::Unauthorized => (
                StatusCode::UNAUTHORIZED,
                "Unauthorized",
                "Authentication required.",
            ),
            ApiError::Forbidden => (
                StatusCode::FORBIDDEN,
                "Forbidden",
                "Insufficient permissions.",
            ),
            ApiError::NotFound => (StatusCode::NOT_FOUND, "Not Found", "Resource not found."),
            ApiError::BadRequest(msg) => (StatusCode::BAD_REQUEST, "Bad Request", msg.as_str()),
            ApiError::Internal | ApiError::Core(_) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Internal Server Error",
                "An unexpected error occurred.",
            ),
        };
        let body = Json(json!({
            "type": format!("https://api.godwit.local/errors/{}", title.to_lowercase().replace(' ', "-")),
            "title": title,
            "status": status.as_u16(),
            "detail": detail,
            "instance": "/"
        }));
        (status, body).into_response()
    }
}
