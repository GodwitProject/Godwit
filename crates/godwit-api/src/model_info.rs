use axum::{
    extract::{Extension, Query, State},
    routing::get,
    Json, Router,
};
use godwit_auth::jwt::Claims;
use serde::Deserialize;
use std::sync::Arc;

use crate::{admin::require_super_admin, error::ApiError, model_router::DbModelRouter, state::AppState};

#[derive(Deserialize)]
pub struct ModelInfoQuery {
    id: String,
}

#[derive(serde::Serialize)]
pub struct ModelInfoResponse {
    pub id: String,
    pub provider_model_id: String,
    pub provider: String,
    pub capabilities: Vec<String>,
    pub pricing: ModelPricing,
    pub fallback_chain: Vec<String>,
    pub context_window: Option<i64>,
    pub max_output_tokens: Option<i64>,
}

#[derive(serde::Serialize)]
pub struct ModelPricing {
    pub input_price_per_million: Option<f64>,
    pub output_price_per_million: Option<f64>,
}

pub fn router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/v1/model/info", get(get_model_info))
}

async fn get_model_info(
    State(state): State<Arc<AppState>>,
    Extension(claims): Extension<Claims>,
    Query(query): Query<ModelInfoQuery>,
) -> Result<Json<ModelInfoResponse>, ApiError> {
    require_super_admin(&claims)?;
    
    let resolved = state
        .model_router
        .resolve(&query.id, godwit_core::Capability::Chat)
        .await
        .map_err(ApiError::Core)?;
    
    let fallback_chain = DbModelRouter::fallback_chain(&resolved.model);
    let pricing = extract_pricing(&resolved.model.pricing);
    let context_window = resolved.model.config.get("context_window").and_then(|v| v.as_i64());
    let max_output_tokens = resolved.model.config.get("max_output_tokens").and_then(|v| v.as_i64());
    
    Ok(Json(ModelInfoResponse {
        id: resolved.model.public_id.clone(),
        provider_model_id: resolved.model.provider_model_id.clone(),
        provider: resolved.profile.protocol.clone(),
        capabilities: resolved.model.capabilities.clone(),
        pricing,
        fallback_chain,
        context_window,
        max_output_tokens,
    }))
}

fn extract_pricing(pricing_json: &serde_json::Value) -> ModelPricing {
    let input_price = pricing_json
        .get("input_price_per_million")
        .and_then(|v| v.as_f64());
    let output_price = pricing_json
        .get("output_price_per_million")
        .and_then(|v| v.as_f64());
    
    ModelPricing {
        input_price_per_million: input_price,
        output_price_per_million: output_price,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn model_info_response_serializes() {
        let response = ModelInfoResponse {
            id: "gpt-4o".to_string(),
            provider_model_id: "gpt-4o-2024-08-06".to_string(),
            provider: "openai".to_string(),
            capabilities: vec!["chat".to_string(), "vision".to_string()],
            pricing: ModelPricing {
                input_price_per_million: Some(2.5),
                output_price_per_million: Some(10.0),
            },
            fallback_chain: vec!["gpt-4o-mini".to_string()],
            context_window: Some(128000),
            max_output_tokens: Some(32768),
        };
        
        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("\"id\":\"gpt-4o\""));
        assert!(json.contains("\"provider\":\"openai\""));
    }
}
