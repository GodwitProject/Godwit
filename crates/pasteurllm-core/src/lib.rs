use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum PasteurError {
    #[error("configuration error: {0}")]
    Config(String),
    #[error("authentication error: {0}")]
    Auth(String),
    #[error("authorization error: {0}")]
    Forbidden(String),
    #[error("not found")]
    NotFound,
    #[error("provider error: {0}")]
    Provider(String),
    #[error("database error: {0}")]
    Database(String),
    #[error("validation error: {0}")]
    Validation(String),
    #[error("rate limited")]
    RateLimited,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AppConfig {
    pub server: ServerConfig,
    pub database: DatabaseConfig,
    pub auth: AuthConfig,
    pub providers: ProvidersConfig,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ServerConfig {
    pub host: String,
    pub port: u16,
    pub request_timeout_seconds: u64,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct DatabaseConfig {
    pub url: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AuthConfig {
    pub jwt_secret: String,
    pub access_token_ttl_minutes: i64,
    pub refresh_token_ttl_days: i64,
    pub oidc_providers: Vec<OidcProviderConfig>,
    pub saml_providers: Vec<SamlProviderConfig>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct OidcProviderConfig {
    pub id: String,
    pub issuer_url: String,
    pub client_id: String,
    pub client_secret: String,
    pub redirect_uri: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SamlProviderConfig {
    pub id: String,
    pub idp_metadata_url: String,
    pub sp_entity_id: String,
    pub acs_url: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ProvidersConfig {
    pub openai: ProviderConfig,
    pub anthropic: ProviderConfig,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ProviderConfig {
    pub api_key: String,
    pub base_url: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ChatCompletionRequest {
    pub model: String,
    pub messages: Vec<ChatMessage>,
    pub stream: Option<bool>,
    pub temperature: Option<f32>,
    pub max_tokens: Option<i32>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ChatMessage {
    pub role: String,
    pub content: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ChatCompletionResponse {
    pub id: String,
    pub object: String,
    pub created: i64,
    pub model: String,
    pub choices: Vec<ChatCompletionChoice>,
    pub usage: Option<Usage>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ChatCompletionChoice {
    pub index: i32,
    pub message: ChatMessage,
    pub finish_reason: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Usage {
    pub prompt_tokens: i32,
    pub completion_tokens: i32,
    pub total_tokens: i32,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn config_parses_from_yaml() {
        let yaml = r#"
server:
  host: 127.0.0.1
  port: 3000
  request_timeout_seconds: 60
database:
  url: postgres://user:pass@localhost/pasteurllm
auth:
  jwt_secret: supersecret
  access_token_ttl_minutes: 15
  refresh_token_ttl_days: 7
  oidc_providers: []
  saml_providers: []
providers:
  openai:
    api_key: sk-openai
    base_url: https://api.openai.com/v1
  anthropic:
    api_key: sk-anthropic
    base_url: https://api.anthropic.com/v1
"#;
        let config: AppConfig = serde_yaml::from_str(yaml).expect("parse yaml");
        assert_eq!(config.server.port, 3000);
        assert_eq!(config.providers.openai.api_key, "sk-openai");
    }
}
