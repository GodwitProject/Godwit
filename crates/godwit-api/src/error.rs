use axum::{
    http::{header, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
use godwit_core::PasteurError;
use serde_json::json;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ApiError {
    #[error("unauthorized")]
    Unauthorized,
    #[error("forbidden")]
    Forbidden,
    #[error("not found")]
    NotFound,
    #[error("bad request: {0}")]
    BadRequest(String),
    #[error("rate limited")]
    RateLimited(Option<u64>),
    #[error("budget exceeded")]
    BudgetExceeded,
    #[error("internal error")]
    Internal,
    #[error("core error: {0}")]
    Core(PasteurError),
    #[error("database error: {0}")]
    Database(String),
    #[error("moderation blocked: {0:?}")]
    ModerationBlocked(Vec<String>),
    #[error("guardrails error: {0}")]
    GuardrailsError(#[from] godwit_core::guardrails::GuardrailsError),
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
            ApiError::ModerationBlocked(categories) => {
                let detail = format!("Content flagged by moderation: {:?}", categories);
                let body = Json(json!({
                    "type": "https://api.godwit.local/errors/moderation-blocked",
                    "title": "Moderation Blocked",
                    "status": 400,
                    "detail": detail,
                    "instance": "/",
                    "categories": categories
                }));
                return (StatusCode::BAD_REQUEST, body).into_response();
            }
            ApiError::Internal | ApiError::Core(_) | ApiError::Database(_) | ApiError::GuardrailsError(_) => (
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
