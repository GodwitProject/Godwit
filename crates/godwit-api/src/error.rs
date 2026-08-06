use axum::{
    http::{header, StatusCode},
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
    RateLimited(Option<u64>),
    BudgetExceeded,
    Internal,
    Core(PasteurError),
}

impl From<PasteurError> for ApiError {
    fn from(err: PasteurError) -> Self {
        match err {
            PasteurError::NotFound => ApiError::NotFound,
            PasteurError::Auth(_) | PasteurError::Forbidden(_) => ApiError::Unauthorized,
            PasteurError::Validation(msg) => ApiError::BadRequest(msg),
            PasteurError::RateLimited => ApiError::RateLimited(None),
            _ => ApiError::Core(err),
        }
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let (status, title, detail, retry_after) = match &self {
            ApiError::Unauthorized => (
                StatusCode::UNAUTHORIZED,
                "Unauthorized",
                "Authentication required.",
                None,
            ),
            ApiError::Forbidden => (
                StatusCode::FORBIDDEN,
                "Forbidden",
                "Insufficient permissions.",
                None,
            ),
            ApiError::NotFound => (
                StatusCode::NOT_FOUND,
                "Not Found",
                "Resource not found.",
                None,
            ),
            ApiError::BadRequest(msg) => (
                StatusCode::BAD_REQUEST,
                "Bad Request",
                msg.as_str(),
                None,
            ),
            ApiError::RateLimited(retry_after) => (
                StatusCode::TOO_MANY_REQUESTS,
                "Too Many Requests",
                "Rate limit exceeded.",
                *retry_after,
            ),
            ApiError::BudgetExceeded => (
                StatusCode::TOO_MANY_REQUESTS,
                "Budget Exceeded",
                "End-user budget has been exceeded.",
                None,
            ),
            ApiError::Internal | ApiError::Core(_) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Internal Server Error",
                "An unexpected error occurred.",
                None,
            ),
        };
        let body = Json(json!({
            "type": format!("https://api.godwit.local/errors/{}", title.to_lowercase().replace(' ', "-")),
            "title": title,
            "status": status.as_u16(),
            "detail": detail,
            "instance": "/"
        }));
        if let Some(seconds) = retry_after {
            (
                status,
                [(header::RETRY_AFTER, seconds.to_string())],
                body,
            )
                .into_response()
        } else {
            (status, body).into_response()
        }
    }
}
