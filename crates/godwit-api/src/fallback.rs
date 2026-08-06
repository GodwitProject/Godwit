use serde::Deserialize;
use std::time::Duration;

#[derive(Debug, Clone, Deserialize)]
pub struct FallbackConfig {
    /// Chain of model references to try (primary first, then fallbacks)
    pub models: Vec<String>,
    /// Maximum number of fallback attempts (not counting primary)
    #[serde(default = "default_max_attempts")]
    pub max_attempts: u32,
    /// Timeout per attempt
    #[serde(default = "default_timeout")]
    pub timeout_per_attempt: Duration,
}

fn default_max_attempts() -> u32 {
    3
}

fn default_timeout() -> Duration {
    Duration::from_secs(30)
}

impl FallbackConfig {
    pub fn from_model_config(config: &serde_json::Value) -> Option<Self> {
        config.get("fallbacks").and_then(|v| v.as_array()).map(|arr| {
            let models: Vec<String> = arr
                .iter()
                .filter_map(|v| v.as_str().map(|s| s.to_string()))
                .collect();

            let max_attempts = config
                .get("max_fallback_attempts")
                .and_then(|v| v.as_u64())
                .unwrap_or(3) as u32;

            let timeout_secs = config
                .get("timeout_per_attempt_secs")
                .and_then(|v| v.as_u64())
                .unwrap_or(30);

            FallbackConfig {
                models,
                max_attempts,
                timeout_per_attempt: Duration::from_secs(timeout_secs),
            }
        })
    }
}

use godwit_core::ChatCompletionRequest;
use godwit_db::repositories::models::ModelRepository;
use godwit_providers::adapter::{ProviderError, UsageReport};
use std::sync::Arc;
use crate::{
    proxy::call_chat,
    state::AppState,
};
use axum::response::Response;

/// Result of a fallback attempt
pub struct FallbackResult {
    pub response: Response,
    pub usage: UsageReport,
    pub model_used: String,
    pub model_pricing: serde_json::Value,
    pub attempts_made: u32,
    pub fallback_triggered: bool,
}

/// Call chat with fallback chain
pub async fn call_chat_with_fallback(
    state: &Arc<AppState>,
    initial_model: &str,
    req: ChatCompletionRequest,
) -> Result<FallbackResult, ProviderError> {
    // Get fallback chain from model config
    let fallback_chain = get_fallback_chain(state, initial_model).await;

    let mut attempts_made = 0u32;
    let mut last_error: Option<ProviderError> = None;

    for (attempt_idx, model_ref) in fallback_chain.iter().enumerate() {
        attempts_made = attempt_idx as u32 + 1;

        // Clone request for each attempt
        let req_clone = req.clone();

        match call_chat_attempt(state, model_ref, req_clone).await {
            Ok((response, usage, pricing)) => {
                return Ok(FallbackResult {
                    response,
                    usage,
                    model_used: model_ref.clone(),
                    model_pricing: pricing,
                    attempts_made,
                    fallback_triggered: attempt_idx > 0,
                });
            }
            Err(e) => {
                // Log attempt failure
                tracing::warn!(
                    "Fallback attempt {} failed for model {}: {:?}",
                    attempts_made,
                    model_ref,
                    e
                );

                // Check if error is retryable
                let retryable = is_retryable_error(&e);
                last_error = Some(e);

                // Check if we should continue
                if attempts_made >= fallback_chain.len() as u32 || !retryable {
                    break;
                }
            }
        }
    }

    // All attempts exhausted
    Err(last_error.unwrap_or(ProviderError::Provider(
        "All fallback attempts failed".to_string(),
    )))
}

/// Get fallback chain for a model
async fn get_fallback_chain(state: &Arc<AppState>, model_ref: &str) -> Vec<String> {
    // First, try to get from model config
    let model_repo = ModelRepository::new(state.pool.clone());
    if let Ok(model) = model_repo.get_by_public_id(model_ref).await {
        if let Some(config) = FallbackConfig::from_model_config(&model.config) {
            let mut chain = vec![model_ref.to_string()];
            chain.extend(config.models);
            return chain;
        }
    }

    // Fallback: just the primary model
    vec![model_ref.to_string()]
}

/// Single chat attempt (wrapper around existing call_chat)
async fn call_chat_attempt(
    state: &Arc<AppState>,
    model_ref: &str,
    req: ChatCompletionRequest,
) -> Result<(Response, UsageReport, serde_json::Value), ProviderError> {
    // Resolve model
    let resolved = state
        .model_router
        .resolve(model_ref, godwit_core::Capability::Chat)
        .await
        .map_err(|e| ProviderError::Provider(e.to_string()))?;

    // Call chat (existing function, needs to be exported from proxy.rs)
    let (resp, usage_opt) = call_chat(state, &resolved, req).await?;
    let usage = usage_opt.unwrap_or_default();
    Ok((resp, usage, resolved.model.pricing))
}

/// Check if error is retryable (5xx, timeout, 429)
fn is_retryable_error(err: &ProviderError) -> bool {
    match err {
        ProviderError::Http { status, .. } => *status >= 500 || *status == 429,
        ProviderError::Provider(msg) => msg.contains("timeout") || msg.contains("timed out"),
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fallback_config_from_json() {
        let config = serde_json::json!({
            "fallbacks": ["anthropic/claude-3", "gemini/gemini-pro"],
            "max_fallback_attempts": 2,
            "timeout_per_attempt_secs": 45
        });

        let fallback = FallbackConfig::from_model_config(&config).unwrap();
        assert_eq!(fallback.models.len(), 2);
        assert_eq!(fallback.max_attempts, 2);
        assert_eq!(fallback.timeout_per_attempt.as_secs(), 45);
    }

    #[test]
    fn test_is_retryable_error() {
        assert!(is_retryable_error(&ProviderError::Http {
            status: 503,
            message: "Service Unavailable".to_string()
        }));

        assert!(is_retryable_error(&ProviderError::Http {
            status: 429,
            message: "Rate Limited".to_string()
        }));

        assert!(!is_retryable_error(&ProviderError::Http {
            status: 400,
            message: "Bad Request".to_string()
        }));
    }
}
