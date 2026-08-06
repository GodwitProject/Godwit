use axum::{
    extract::{Extension, Json, State},
    middleware,
    response::Response,
    routing::post,
    Router,
};
use godwit_db::models::ApiKey;
use serde_json::Value;
use std::sync::Arc;

use crate::state::AppState;
use crate::moderation_fallback::{ModerationFallback, ModerationFallbackConfig};

/// OpenAI-compatible `/v1/moderations` with fallback chain (OpenAI → Azure → Self-hosted).
///
/// Moderation uses a configurable fallback chain. Each provider is tried with a timeout
/// (default 10s). The first successful response is returned in OpenAI format.
pub fn router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/v1/moderations", post(moderations))
        .route_layer(middleware::from_fn(crate::middleware::model_scope))
}

async fn moderations(
    State(state): State<Arc<AppState>>,
    Extension(api_key): Extension<ApiKey>,
    Json(body): Json<Value>,
) -> Result<Response, crate::error::ApiError> {
    let _model = extract_model(&body)?;
    
    let fallback = ModerationFallback::new(
        ModerationFallbackConfig::default()
    );
    
    fallback.execute(&state, &api_key, body).await
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

