use axum::{
    extract::{Path, State},
    routing::{get, post},
    Json, Router,
};
use godwit_core::{ChatMessage, ChatContent};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use crate::state::AppState;
use crate::error::ApiError;

pub fn router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/v1/utils/token_counter", post(token_counter))
        .route("/v1/utils/model_info/:model_id", get(model_info))
        .route("/v1/utils/health", get(health))
}

#[derive(Debug, Deserialize)]
pub struct TokenCountRequest {
    pub model: String,
    pub messages: Vec<ChatMessage>,
}

#[derive(Debug, Serialize)]
pub struct TokenCountResponse {
    pub prompt_tokens: u32,
    pub model: String,
}

pub async fn token_counter(
    Json(req): Json<TokenCountRequest>,
) -> Result<Json<TokenCountResponse>, ApiError> {
    let prompt_tokens = count_tokens(&req.model, &req.messages);
    
    Ok(Json(TokenCountResponse {
        prompt_tokens,
        model: req.model,
    }))
}

fn count_tokens(_model: &str, messages: &[ChatMessage]) -> u32 {
    let mut total = 0u32;
    
    for msg in messages {
        total += 4;
        
        if !msg.role.is_empty() {
            total += msg.role.len() as u32;
        }
        
        if let Some(content) = &msg.content {
            for c in content {
                match c {
                    ChatContent::Text(text) => {
                        total += text.len() as u32 / 4;
                    }
                    ChatContent::Parts(parts) => {
                        for part in parts {
                            if let ChatContentPart::Text { text } = part {
                                total += text.len() as u32 / 4;
                            } else {
                                total += 100;
                            }
                        }
                    }
                }
            }
        }
        
        if let Some(name) = &msg.name {
            total += name.len() as u32 / 4;
        }
        
        if let Some(tool_calls) = &msg.tool_calls {
            for tc in tool_calls {
                total += tc.id.len() as u32 / 4;
                total += tc.function.name.len() as u32 / 4;
                total += tc.function.arguments.len() as u32 / 4;
            }
        }
        
        if let Some(tool_call_id) = &msg.tool_call_id {
            total += tool_call_id.len() as u32 / 4;
        }
    }
    
    total += 2;
    
    total
}

use godwit_core::ChatContentPart;

#[derive(Debug, Serialize)]
pub struct ModelInfo {
    pub id: String,
    pub provider: String,
    pub pricing: PricingInfo,
    pub capabilities: ModelCapabilities,
}

#[derive(Debug, Serialize)]
pub struct PricingInfo {
    pub input_cost_per_1k: f64,
    pub output_cost_per_1k: f64,
    pub cache_read_cost_per_1k: f64,
    pub cache_write_cost_per_1k: f64,
}

#[derive(Debug, Serialize)]
pub struct ModelCapabilities {
    pub supports_tool_calling: bool,
    pub supports_vision: bool,
    pub supports_streaming: bool,
    pub supports_prompt_cache: bool,
    pub max_tokens: u32,
}

pub async fn model_info(
    State(state): State<Arc<AppState>>,
    Path(model_id): Path<String>,
) -> Result<Json<ModelInfo>, ApiError> {
    let resolved = state
        .model_router
        .resolve(&model_id, godwit_core::Capability::Chat)
        .await
        .map_err(ApiError::Core)?;
    
    let pricing = extract_pricing(&resolved.model.pricing);
    let capabilities = extract_capabilities(&resolved.model);
    
    Ok(Json(ModelInfo {
        id: resolved.model.public_id.clone(),
        provider: resolved.profile.protocol.clone(),
        pricing,
        capabilities,
    }))
}

fn extract_pricing(pricing_json: &serde_json::Value) -> PricingInfo {
    let input_price = pricing_json
        .get("input_price_per_million")
        .and_then(|v| v.as_f64())
        .unwrap_or(0.0) / 1000.0;
    let output_price = pricing_json
        .get("output_price_per_million")
        .and_then(|v| v.as_f64())
        .unwrap_or(0.0) / 1000.0;
    
    PricingInfo {
        input_cost_per_1k: input_price,
        output_cost_per_1k: output_price,
        cache_read_cost_per_1k: 0.0,
        cache_write_cost_per_1k: 0.0,
    }
}

fn extract_capabilities(model: &godwit_db::models::Model) -> ModelCapabilities {
    let max_tokens = model
        .config
        .get("max_output_tokens")
        .and_then(|v| v.as_i64())
        .unwrap_or(4096) as u32;
    
    let supports_vision = model.capabilities.iter().any(|c| c == "vision");
    let supports_tool_calling = model.capabilities.iter().any(|c| c == "tool_calling" || c == "tools");
    let supports_streaming = true;
    let supports_prompt_cache = model.capabilities.iter().any(|c| c == "prompt_cache");
    
    ModelCapabilities {
        supports_tool_calling,
        supports_vision,
        supports_streaming,
        supports_prompt_cache,
        max_tokens,
    }
}

#[derive(Debug, Serialize)]
pub struct HealthStatus {
    pub status: String,
    pub version: String,
    pub uptime_secs: u64,
    pub database: String,
    pub providers: Vec<ProviderStatus>,
}

#[derive(Debug, Serialize)]
pub struct ProviderStatus {
    pub name: String,
    pub status: String,
    pub latency_ms: Option<u32>,
}

static START_TIME: std::sync::OnceLock<u64> = std::sync::OnceLock::new();

fn get_start_time() -> u64 {
    *START_TIME.get_or_init(|| {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs()
    })
}

pub async fn health(
    State(state): State<Arc<AppState>>,
) -> Result<Json<HealthStatus>, ApiError> {
    let start_time = get_start_time();
    let uptime_secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs()
        - start_time;
    
    let database = check_database_health(&state.pool).await?;
    let providers = check_provider_health().await;
    
    Ok(Json(HealthStatus {
        status: "healthy".to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        uptime_secs,
        database,
        providers,
    }))
}

async fn check_database_health(pool: &sqlx::PgPool) -> Result<String, ApiError> {
    sqlx::query("SELECT 1")
        .fetch_one(pool)
        .await
        .map(|_| "connected".to_string())
        .map_err(|e| ApiError::Database(format!("database connection failed: {}", e)))
}

async fn check_provider_health() -> Vec<ProviderStatus> {
    vec![]
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn count_tokens_empty_messages() {
        let messages: Vec<ChatMessage> = vec![];
        let tokens = count_tokens("gpt-4", &messages);
        assert_eq!(tokens, 2);
    }
    
    #[test]
    fn count_tokens_simple_message() {
        let messages = vec![ChatMessage {
            role: "user".to_string(),
            content: Some(vec![ChatContent::Text("Hello, world!".to_string())]),
            name: None,
            tool_calls: None,
            tool_call_id: None,
            cache_control: None,
        }];
        let tokens = count_tokens("gpt-4", &messages);
        assert!(tokens > 0);
    }
    
    #[test]
    fn count_tokens_multiple_messages() {
        let messages = vec![
            ChatMessage {
                role: "system".to_string(),
                content: Some(vec![ChatContent::Text("You are helpful".to_string())]),
                name: None,
                tool_calls: None,
                tool_call_id: None,
                cache_control: None,
            },
            ChatMessage {
                role: "user".to_string(),
                content: Some(vec![ChatContent::Text("Hi".to_string())]),
                name: None,
                tool_calls: None,
                tool_call_id: None,
                cache_control: None,
            },
        ];
        let tokens = count_tokens("gpt-4", &messages);
        let single_tokens = count_tokens("gpt-4", &messages[..1]);
        assert!(tokens > single_tokens);
    }
    
    #[test]
    fn token_count_response_serializes() {
        let response = TokenCountResponse {
            prompt_tokens: 100,
            model: "gpt-4".to_string(),
        };
        
        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("\"prompt_tokens\":100"));
        assert!(json.contains("\"model\":\"gpt-4\""));
    }
    
    #[test]
    fn model_info_serializes() {
        let info = ModelInfo {
            id: "gpt-4o".to_string(),
            provider: "openai".to_string(),
            pricing: PricingInfo {
                input_cost_per_1k: 0.0025,
                output_cost_per_1k: 0.01,
                cache_read_cost_per_1k: 0.0,
                cache_write_cost_per_1k: 0.0,
            },
            capabilities: ModelCapabilities {
                supports_tool_calling: true,
                supports_vision: true,
                supports_streaming: true,
                supports_prompt_cache: false,
                max_tokens: 128000,
            },
        };
        
        let json = serde_json::to_string(&info).unwrap();
        assert!(json.contains("\"id\":\"gpt-4o\""));
        assert!(json.contains("\"provider\":\"openai\""));
    }
    
    #[test]
    fn health_status_serializes() {
        let status = HealthStatus {
            status: "healthy".to_string(),
            version: "0.1.0".to_string(),
            uptime_secs: 1000,
            database: "connected".to_string(),
            providers: vec![],
        };
        
        let json = serde_json::to_string(&status).unwrap();
        assert!(json.contains("\"status\":\"healthy\""));
        assert!(json.contains("\"version\":\"0.1.0\""));
    }
    
    #[test]
    fn provider_status_serializes() {
        let status = ProviderStatus {
            name: "openai".to_string(),
            status: "healthy".to_string(),
            latency_ms: Some(50),
        };
        
        let json = serde_json::to_string(&status).unwrap();
        assert!(json.contains("\"name\":\"openai\""));
        assert!(json.contains("\"latency_ms\":50"));
    }
}
