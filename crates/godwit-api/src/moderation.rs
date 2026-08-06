use axum::{
    extract::{Extension, Json, State},
    middleware,
    response::{IntoResponse, Response},
    routing::post,
    Router,
};
use godwit_core::Capability;
use godwit_db::models::ApiKey;
use serde_json::Value;
use std::sync::Arc;

use crate::state::AppState;

/// OpenAI-compatible `/v1/moderations` pass-through.
///
/// Moderation is proxied to the same backend that serves the resolved model, using the
/// profile's base URL and credentials. The client's model ref is substituted with the
/// catalog row's upstream `provider_model_id` before forwarding, matching the chat
/// adapters. The upstream response is returned verbatim.
pub fn router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/v1/moderations", post(moderations))
        .route_layer(middleware::from_fn(crate::middleware::model_scope))
}

async fn moderations(
    State(state): State<Arc<AppState>>,
    Extension(_api_key): Extension<ApiKey>,
    Json(body): Json<Value>,
) -> Result<Response, crate::error::ApiError> {
    let model = extract_model(&body)?;
    let (response, _) = crate::proxy::forward_openai_passthrough(
        &state,
        reqwest::Method::POST,
        "moderations",
        Some(body),
        &model,
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
    fn router_constructs_with_moderation_route() {
        let _: axum::routing::Router<Arc<crate::state::AppState>> = router();
    }
}

