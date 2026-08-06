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

/// `/v1/rerank` pass-through.
///
/// Re-ranking is forwarded to the resolved backend's `/rerank` endpoint, substituting the
/// client's model ref with the catalog row's upstream `provider_model_id`. The upstream
/// response is returned verbatim.
pub fn router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/v1/rerank", post(rerank))
        .route_layer(middleware::from_fn(crate::middleware::model_scope))
}

async fn rerank(
    State(state): State<Arc<AppState>>,
    Extension(_api_key): Extension<ApiKey>,
    Json(body): Json<Value>,
) -> Result<Response, crate::error::ApiError> {
    let model = extract_model(&body)?;
    let (response, _) = crate::proxy::forward_openai_passthrough(
        &state,
        reqwest::Method::POST,
        "rerank",
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
        let body = serde_json::json!({ "model": "rerank-model" });
        match extract_model(&body) {
            Ok(model) => assert_eq!(model, "rerank-model"),
            Err(_) => panic!("expected model to be extracted"),
        }
    }

    #[test]
    fn router_constructs_with_rerank_route() {
        let _: axum::routing::Router<Arc<crate::state::AppState>> = router();
    }
}

