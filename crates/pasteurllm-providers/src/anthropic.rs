use crate::{Provider, ProviderError, ProviderResponse, SseEvent};
use async_trait::async_trait;
use futures::stream::BoxStream;
use pasteurllm_core::{
    ChatCompletionChoice, ChatCompletionRequest, ChatCompletionResponse, ChatMessage, Usage,
};
use reqwest::Client;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize)]
pub struct AnthropicMessage {
    pub role: String,
    pub content: String,
}

#[derive(Debug, Serialize)]
pub struct AnthropicRequest {
    pub model: String,
    pub max_tokens: i32,
    pub messages: Vec<AnthropicMessage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub system: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,
    pub stream: bool,
}

#[derive(Debug, Deserialize)]
pub struct ContentBlock {
    #[serde(rename = "type")]
    pub type_: String,
    pub text: String,
}

#[derive(Debug, Deserialize)]
pub struct AnthropicUsage {
    pub input_tokens: i32,
    pub output_tokens: i32,
}

#[derive(Debug, Deserialize)]
pub struct AnthropicResponse {
    pub id: String,
    pub model: String,
    pub content: Vec<ContentBlock>,
    pub usage: AnthropicUsage,
}

fn map_model(public_model: &str) -> String {
    match public_model {
        "claude-sonnet" => "claude-3-5-sonnet-20240620".to_string(),
        _ => public_model.to_string(),
    }
}

pub fn to_anthropic_request(req: &ChatCompletionRequest) -> AnthropicRequest {
    let mut system: Option<String> = None;
    let mut messages = Vec::new();
    for m in &req.messages {
        if m.role == "system" {
            system = Some(m.content.clone());
        } else {
            messages.push(AnthropicMessage {
                role: m.role.clone(),
                content: m.content.clone(),
            });
        }
    }
    AnthropicRequest {
        model: map_model(&req.model),
        max_tokens: req.max_tokens.unwrap_or(4096),
        messages,
        system,
        temperature: req.temperature,
        stream: req.stream.unwrap_or(false),
    }
}

pub fn to_openai_response(resp: AnthropicResponse, public_model: &str) -> ChatCompletionResponse {
    let text = resp
        .content
        .into_iter()
        .filter(|c| c.type_ == "text")
        .map(|c| c.text)
        .collect::<Vec<_>>()
        .join("");
    let usage = Usage {
        prompt_tokens: resp.usage.input_tokens,
        completion_tokens: resp.usage.output_tokens,
        total_tokens: resp.usage.input_tokens + resp.usage.output_tokens,
    };
    ChatCompletionResponse {
        id: resp.id,
        object: "chat.completion".to_string(),
        created: chrono::Utc::now().timestamp(),
        model: public_model.to_string(),
        choices: vec![ChatCompletionChoice {
            index: 0,
            message: ChatMessage {
                role: "assistant".to_string(),
                content: text,
            },
            finish_reason: Some("stop".to_string()),
        }],
        usage: Some(usage),
    }
}

pub struct AnthropicProvider {
    client: Client,
    api_key: String,
    base_url: String,
}

impl AnthropicProvider {
    pub fn new(api_key: &str, base_url: &str) -> Self {
        let client = Client::builder()
            .pool_max_idle_per_host(20)
            .pool_idle_timeout(std::time::Duration::from_secs(60))
            .timeout(std::time::Duration::from_secs(120))
            .build()
            .expect("build reqwest client");
        Self {
            client,
            api_key: api_key.to_string(),
            base_url: base_url.to_string(),
        }
    }
}

#[async_trait]
impl Provider for AnthropicProvider {
    async fn chat_completion(
        &self,
        request: ChatCompletionRequest,
    ) -> Result<ProviderResponse, ProviderError> {
        let url = format!("{}/messages", self.base_url);
        let anthropic_req = to_anthropic_request(&request);
        let res = self
            .client
            .post(&url)
            .header("x-api-key", self.api_key.clone())
            .header("anthropic-version", "2023-06-01")
            .json(&anthropic_req)
            .send()
            .await
            .map_err(|e| ProviderError::Http(e.to_string()))?;
        if !res.status().is_success() {
            return Err(ProviderError::Provider(res.text().await.unwrap_or_default()));
        }
        let anthropic_resp: AnthropicResponse = res
            .json()
            .await
            .map_err(|e| ProviderError::Serialization(e.to_string()))?;
        let openai_resp = to_openai_response(anthropic_resp, &request.model);
        Ok(ProviderResponse::Json(openai_resp))
    }

    async fn stream_chat_completion(
        &self,
        _request: ChatCompletionRequest,
    ) -> Result<BoxStream<'static, Result<SseEvent, ProviderError>>, ProviderError> {
        Err(ProviderError::NotImplemented)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pasteurllm_core::{ChatCompletionRequest, ChatMessage};

    #[test]
    fn openai_to_anthropic_request() {
        let req = ChatCompletionRequest {
            model: "claude-sonnet".to_string(),
            messages: vec![
                ChatMessage { role: "system".to_string(), content: "You are helpful".to_string() },
                ChatMessage { role: "user".to_string(), content: "Hello".to_string() },
            ],
            stream: Some(false),
            temperature: Some(0.7),
            max_tokens: Some(100),
        };
        let anthropic = to_anthropic_request(&req);
        assert_eq!(anthropic.model, "claude-3-5-sonnet-20240620");
        assert_eq!(anthropic.system, Some("You are helpful".to_string()));
        assert_eq!(anthropic.messages.len(), 1);
    }

    #[test]
    fn anthropic_response_to_openai() {
        let ar = AnthropicResponse {
            id: "msg-1".to_string(),
            model: "claude-3-5-sonnet-20240620".to_string(),
            content: vec![ContentBlock { text: "Hi there".to_string(), type_: "text".to_string() }],
            usage: AnthropicUsage { input_tokens: 1, output_tokens: 2 },
        };
        let openai = to_openai_response(ar, "claude-sonnet");
        assert_eq!(openai.choices[0].message.content, "Hi there");
        assert_eq!(openai.usage.unwrap().total_tokens, 3);
    }
}
