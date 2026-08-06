use axum::{
    extract::{Extension, Json, Path, Query, State},
    http::StatusCode,
    middleware,
    response::{IntoResponse, Response},
    routing::{get, post},
    Router,
};
use godwit_core::Capability;
use godwit_db::models::ApiKey;
use serde::Deserialize;
use serde_json::Value;
use std::sync::Arc;

use crate::state::AppState;

/// OpenAI-compatible `/v1/batches` pass-through.
///
/// Implements batch create (`POST /v1/batches`), list (`GET /v1/batches`), retrieve
/// (`GET /v1/batches/:id`) and cancel (`POST /v1/batches/:id/cancel`) by forwarding to the
/// resolved backend's `/batches` path. POST/list endpoints carry the upstream responses
/// verbatim; the backend profile is selected via a `model` ref (in the create body, or via
/// a `model` query parameter for the read-only routes).
pub fn router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/v1/batches", post(create_batch).get(list_batches))
        .route("/v1/batches/:id", get(get_batch))
        .route("/v1/batches/:id/cancel", post(cancel_batch))
        .route_layer(middleware::from_fn(crate::middleware::model_scope))
}

#[derive(Debug, Deserialize)]
pub struct ModelQuery {
    pub model: String,
}

async fn create_batch(
    State(state): State<Arc<AppState>>,
    Extension(_api_key): Extension<ApiKey>,
    Json(body): Json<Value>,
) -> Result<Response, crate::error::ApiError> {
    let model = extract_model(&body)?;
    let (response, _) = crate::proxy::forward_openai_passthrough(
        &state,
        reqwest::Method::POST,
        "batches",
        Some(body),
        &model,
        Capability::Chat,
    )
    .await?;
    Ok((StatusCode::CREATED, Json(response)).into_response())
}

async fn list_batches(
    State(state): State<Arc<AppState>>,
    Extension(_api_key): Extension<ApiKey>,
    Query(query): Query<ModelQuery>,
) -> Result<Response, crate::error::ApiError> {
    let (response, _) = crate::proxy::forward_openai_passthrough(
        &state,
        reqwest::Method::GET,
        "batches",
        None,
        &query.model,
        Capability::Chat,
    )
    .await?;
    Ok(Json(response).into_response())
}

async fn get_batch(
    State(state): State<Arc<AppState>>,
    Extension(_api_key): Extension<ApiKey>,
    Path(batch_id): Path<String>,
    Query(query): Query<ModelQuery>,
) -> Result<Response, crate::error::ApiError> {
    let path = format!("batches/{batch_id}");
    let (response, _) = crate::proxy::forward_openai_passthrough(
        &state,
        reqwest::Method::GET,
        &path,
        None,
        &query.model,
        Capability::Chat,
    )
    .await?;
    Ok(Json(response).into_response())
}

async fn cancel_batch(
    State(state): State<Arc<AppState>>,
    Extension(_api_key): Extension<ApiKey>,
    Path(batch_id): Path<String>,
    Query(query): Query<ModelQuery>,
) -> Result<Response, crate::error::ApiError> {
    let path = format!("batches/{batch_id}/cancel");
    let (response, _) = crate::proxy::forward_openai_passthrough(
        &state,
        reqwest::Method::POST,
        &path,
        None,
        &query.model,
        Capability::Chat,
    )
    .await?;
    Ok(Json(response).into_response())
}

fn extract_model(body: &Value) -> Result<String, crate::error::ApiError> {
    body.get("model")
        .and_then(|m| m.as_str())
        .map(|s| s.to_string())
        .ok_or_else(|| crate::error::ApiError::BadRequest("missing 'model' field".to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extract_model_from_body() {
        let body = serde_json::json!({ "model": "text-moderation-latest" });
        match extract_model(&body) {
            Ok(model) => assert_eq!(model, "text-moderation-latest"),
            Err(_) => panic!("expected model to be extracted"),
        }
    }

    #[test]
    fn extract_model_missing_returns_bad_request() {
        let body = serde_json::json!({ "messages": [] });
        match extract_model(&body) {
            Err(crate::error::ApiError::BadRequest(_)) => {}
            _ => panic!("expected BadRequest"),
        }
    }

    #[test]
    fn router_constructs_with_batch_routes() {
        let router = router();
        let _: axum::routing::Router<Arc<crate::state::AppState>> = router;
    }
}

