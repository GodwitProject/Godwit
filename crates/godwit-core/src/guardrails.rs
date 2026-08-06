use crate::pii_masking::{PiiMasker, default_patterns};
use crate::{ChatCompletionRequest, ChatCompletionResponse};
use thiserror::Error;

#[derive(Debug, Clone)]
pub struct GuardrailsConfig {
    pub pii_enabled: bool,
    pub moderation_pre: bool,
    pub moderation_post: bool,
    pub block_on_moderation_failure: bool,
}

impl Default for GuardrailsConfig {
    fn default() -> Self {
        Self {
            pii_enabled: false,
            moderation_pre: false,
            moderation_post: false,
            block_on_moderation_failure: true,
        }
    }
}

pub struct GuardrailsOrchestrator {
    pii_masker: Option<PiiMasker>,
    config: GuardrailsConfig,
}

#[derive(Debug, Error)]
pub enum GuardrailsError {
    #[error("moderation failed: {0}")]
    Moderation(String),
    #[error("internal error: {0}")]
    Internal(String),
}

#[derive(Debug)]
pub enum PreCallResult {
    Allowed,
    Blocked(ModerationResult),
}

#[derive(Debug)]
pub enum PostCallResult {
    Allowed,
    Blocked(ModerationResult),
}

#[derive(Debug, Clone)]
pub struct ModerationResult {
    pub flagged: bool,
    pub categories: serde_json::Value,
}

impl GuardrailsOrchestrator {
    pub fn new(config: GuardrailsConfig) -> Self {
        let pii_masker = if config.pii_enabled {
            Some(PiiMasker::new(default_patterns()))
        } else {
            None
        };

        Self {
            pii_masker,
            config,
        }
    }

    pub async fn pre_call(
        &mut self,
        request: &mut ChatCompletionRequest,
        request_id: &str,
    ) -> Result<PreCallResult, GuardrailsError> {
        if self.config.pii_enabled {
            if let Some(masker) = &mut self.pii_masker {
                for msg in &mut request.messages {
                    if let Some(content_vec) = &mut msg.content {
                        for content in content_vec {
                            if let Some(text) = content.as_text() {
                                let masked = masker.mask(&text, request_id);
                                *content = crate::ChatContent::text(masked);
                            }
                        }
                    }
                }
            }
        }

        if self.config.moderation_pre {
            let combined_text = request
                .messages
                .iter()
                .filter_map(|m| m.content_as_text())
                .collect::<Vec<_>>()
                .join(" ");

            let mod_result = check_moderation(&combined_text, &request.model)
                .await
                .map_err(|e| GuardrailsError::Moderation(e.to_string()))?;

            if mod_result.flagged && self.config.block_on_moderation_failure {
                return Ok(PreCallResult::Blocked(mod_result));
            }
        }

        Ok(PreCallResult::Allowed)
    }

    pub async fn post_call(
        &mut self,
        response: &mut ChatCompletionResponse,
        request_id: &str,
    ) -> Result<PostCallResult, GuardrailsError> {
        if self.config.moderation_post {
            let response_text = response
                .choices
                .iter()
                .filter_map(|c| c.message.content_as_text())
                .collect::<Vec<_>>()
                .join(" ");

            let mod_result = check_moderation(&response_text, &response.model)
                .await
                .map_err(|e| GuardrailsError::Moderation(e.to_string()))?;

            if mod_result.flagged && self.config.block_on_moderation_failure {
                return Ok(PostCallResult::Blocked(mod_result));
            }
        }

        if self.config.pii_enabled {
            if let Some(masker) = &mut self.pii_masker {
                for choice in &mut response.choices {
                    if let Some(content_vec) = &mut choice.message.content {
                        for content in content_vec {
                            if let Some(text) = content.as_text() {
                                let unmasked = masker.unmask(&text, request_id);
                                *content = crate::ChatContent::text(unmasked);
                            }
                        }
                    }
                }
            }
        }

        Ok(PostCallResult::Allowed)
    }
}

async fn check_moderation(text: &str, _model: &str) -> Result<ModerationResult, GuardrailsError> {
    if text.trim().is_empty() {
        return Ok(ModerationResult {
            flagged: false,
            categories: serde_json::json!({}),
        });
    }

    let client = reqwest::Client::new();
    let body = serde_json::json!({
        "model": "text-moderation-latest",
        "input": text,
    });

    let response = client
        .post("https://api.openai.com/v1/moderations")
        .header(reqwest::header::CONTENT_TYPE, "application/json")
        .json(&body)
        .send()
        .await
        .map_err(|e| GuardrailsError::Moderation(format!("request failed: {}", e)))?;

    if !response.status().is_success() {
        let status = response.status();
        let error_text = response.text().await.unwrap_or_default();
        return Err(GuardrailsError::Moderation(format!(
            "HTTP {}: {}",
            status, error_text
        )));
    }

    let value: serde_json::Value = response
        .json()
        .await
        .map_err(|e| GuardrailsError::Moderation(format!("parse failed: {}", e)))?;

    let results = value
        .get("results")
        .and_then(|r| r.as_array())
        .and_then(|arr| arr.first());

    let flagged = results
        .and_then(|r| r.get("flagged"))
        .and_then(|f| f.as_bool())
        .unwrap_or(false);

    let categories = results
        .and_then(|r| r.get("categories"))
        .cloned()
        .unwrap_or_else(|| serde_json::json!({}));

    Ok(ModerationResult { flagged, categories })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{ChatContent, ChatMessage};

    #[tokio::test]
    async fn test_pre_call_pii_masking() {
        let config = GuardrailsConfig {
            pii_enabled: true,
            moderation_pre: false,
            moderation_post: false,
            block_on_moderation_failure: true,
        };
        let mut orchestrator = GuardrailsOrchestrator::new(config);

        let mut request = ChatCompletionRequest {
            model: "test-model".to_string(),
            messages: vec![ChatMessage {
                role: "user".to_string(),
                content: Some(vec![ChatContent::text("Contact me at test@example.com")]),
                name: None,
                tool_calls: None,
                tool_call_id: None,
                cache_control: None,
            }],
            stream: None,
            temperature: None,
            max_tokens: None,
            top_p: None,
            top_k: None,
            frequency_penalty: None,
            presence_penalty: None,
            repetition_penalty: None,
            stop: None,
            seed: None,
            n: None,
            logprobs: None,
            top_logprobs: None,
            logit_bias: None,
            user: None,
            tools: None,
            tool_choice: None,
            parallel_tool_calls: None,
            response_format: None,
            reasoning: None,
        };

        let result = orchestrator.pre_call(&mut request, "test-req-1").await;
        assert!(result.is_ok());

        if let Some(content_vec) = &request.messages[0].content {
            if let Some(text) = content_vec[0].as_text() {
                assert!(text.contains("[EMAIL]"));
            }
        }
    }

    #[tokio::test]
    async fn test_post_call_pii_unmasking() {
        let config = GuardrailsConfig {
            pii_enabled: true,
            moderation_pre: false,
            moderation_post: false,
            block_on_moderation_failure: true,
        };
        let mut orchestrator = GuardrailsOrchestrator::new(config);

        let mut request = ChatCompletionRequest {
            model: "test-model".to_string(),
            messages: vec![ChatMessage {
                role: "user".to_string(),
                content: Some(vec![ChatContent::text("Email: test@example.com")]),
                name: None,
                tool_calls: None,
                tool_call_id: None,
                cache_control: None,
            }],
            stream: None,
            temperature: None,
            max_tokens: None,
            top_p: None,
            top_k: None,
            frequency_penalty: None,
            presence_penalty: None,
            repetition_penalty: None,
            stop: None,
            seed: None,
            n: None,
            logprobs: None,
            top_logprobs: None,
            logit_bias: None,
            user: None,
            tools: None,
            tool_choice: None,
            parallel_tool_calls: None,
            response_format: None,
            reasoning: None,
        };

        let _ = orchestrator.pre_call(&mut request, "test-req-2").await;

        let mut response = ChatCompletionResponse {
            id: "resp-1".to_string(),
            object: "chat.completion".to_string(),
            created: 1234567890,
            model: "test-model".to_string(),
            choices: vec![],
            usage: None,
        };

        let result = orchestrator.post_call(&mut response, "test-req-2").await;
        assert!(result.is_ok());
    }

    #[test]
    fn test_config_defaults() {
        let config = GuardrailsConfig::default();
        assert!(!config.pii_enabled);
        assert!(!config.moderation_pre);
        assert!(!config.moderation_post);
        assert!(config.block_on_moderation_failure);
    }

    #[test]
    fn test_orchestrator_creation() {
        let config = GuardrailsConfig::default();
        let orchestrator = GuardrailsOrchestrator::new(config);
        assert!(orchestrator.pii_masker.is_none());
    }

    #[test]
    fn test_orchestrator_with_pii_enabled() {
        let config = GuardrailsConfig {
            pii_enabled: true,
            ..Default::default()
        };
        let orchestrator = GuardrailsOrchestrator::new(config);
        assert!(orchestrator.pii_masker.is_some());
    }
}
