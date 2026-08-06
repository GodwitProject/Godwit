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

#[derive(Debug, Clone, Deserialize)]
pub struct AppConfig {
    pub server: ServerConfig,
    pub database: DatabaseConfig,
    pub auth: AuthConfig,
    /// Agentic ecosystem wiring: MCP tool servers and the SearXNG web-search backend.
    #[serde(default)]
    pub agentic: AgenticConfig,
}

/// The agentic ecosystem section of [`AppConfig`]: a list of MCP servers to expose as
/// tools, and an optional self-hosted SearXNG instance used to back `web_search` style
/// tool calls when the selected adapter has no native web search.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct AgenticConfig {
    #[serde(default)]
    pub mcp_servers: Vec<McpServerEntry>,
    #[serde(default)]
    pub searxng: Option<SearxngConfig>,
}

/// A single MCP server spawned as a subprocess speaking JSON-RPC over stdio.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct McpServerEntry {
    pub name: String,
    pub command: String,
    #[serde(default)]
    pub args: Vec<String>,
    /// Extra environment variables for the child process.
    #[serde(default)]
    pub env: std::collections::HashMap<String, String>,
}

/// Configuration for a self-hosted SearXNG web-search backend.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct SearxngConfig {
    pub base_url: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ServerConfig {
    pub host: String,
    pub port: u16,
    pub request_timeout_seconds: u64,
}

#[derive(Debug, Clone, Deserialize)]
pub struct DatabaseConfig {
    pub url: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct AuthConfig {
    pub jwt_secret: String,
    pub access_token_ttl_minutes: i64,
    pub refresh_token_ttl_days: i64,
    pub oidc_providers: Vec<OidcProviderConfig>,
    pub saml_providers: Vec<SamlProviderConfig>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct OidcProviderConfig {
    pub id: String,
    pub issuer_url: String,
    pub client_id: String,
    pub client_secret: String,
    pub redirect_uri: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SamlProviderConfig {
    pub id: String,
    pub idp_metadata_url: String,
    pub sp_entity_id: String,
    pub acs_url: String,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct ChatCompletionRequest {
    pub model: String,
    pub messages: Vec<ChatMessage>,
    pub stream: Option<bool>,
    pub temperature: Option<f32>,
    pub max_tokens: Option<i32>,
    pub top_p: Option<f32>,
    pub top_k: Option<i32>,
    pub frequency_penalty: Option<f32>,
    pub presence_penalty: Option<f32>,
    pub stop: Option<Stop>,
    pub seed: Option<i64>,
    pub n: Option<i32>,
    pub logprobs: Option<bool>,
    pub top_logprobs: Option<i32>,
    pub tools: Option<Vec<Tool>>,
    pub tool_choice: Option<ToolChoice>,
    pub response_format: Option<ResponseFormat>,
    pub reasoning: Option<ReasoningConfig>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "snake_case", tag = "type")]
pub enum ResponseFormat {
    Text,
    JsonObject,
    JsonSchema { json_schema: JsonSchema },
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct JsonSchema {
    pub name: String,
    pub schema: Option<serde_json::Value>,
    pub strict: Option<bool>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(untagged)]
pub enum Stop {
    String(String),
    Array(Vec<String>),
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ReasoningConfig {
    pub effort: Option<String>,
    pub thinking: Option<ThinkingConfig>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ThinkingConfig {
    pub r#type: String,
    pub budget_tokens: i32,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct CacheControl {
    pub r#type: String,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(untagged)]
pub enum ChatContent {
    Text(String),
    Parts(Vec<ChatContentPart>),
}

impl Default for ChatContent {
    fn default() -> Self {
        Self::Text(String::new())
    }
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ChatContentPart {
    Text { text: String },
    ImageUrl { image_url: ImageUrl },
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct ImageUrl {
    pub url: String,
    pub detail: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Tool {
    pub r#type: String,
    pub function: FunctionDefinition,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct FunctionDefinition {
    pub name: String,
    pub description: Option<String>,
    pub parameters: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ToolChoice {
    None,
    Auto,
    Required,
    Function { function: FunctionName },
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct FunctionName {
    pub name: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ToolCall {
    pub id: String,
    pub r#type: String,
    pub function: FunctionCall,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct FunctionCall {
    pub name: String,
    pub arguments: String,
}

impl ChatContent {
    pub fn text(value: impl Into<String>) -> Self {
        Self::Text(value.into())
    }

    pub fn as_text(&self) -> Option<String> {
        match self {
            ChatContent::Text(t) => Some(t.clone()),
            ChatContent::Parts(parts) => {
                let text: Vec<&str> = parts
                    .iter()
                    .filter_map(|p| match p {
                        ChatContentPart::Text { text } => Some(text.as_str()),
                        _ => None,
                    })
                    .collect();
                if text.is_empty() {
                    None
                } else {
                    Some(text.join(""))
                }
            }
        }
    }

    pub fn has_images(&self) -> bool {
        match self {
            ChatContent::Text(_) => false,
            ChatContent::Parts(parts) => parts.iter().any(|p| matches!(p, ChatContentPart::ImageUrl { .. })),
        }
    }
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct ChatMessage {
    pub role: String,
    pub content: ChatContent,
    pub name: Option<String>,
    pub tool_calls: Option<Vec<ToolCall>>,
    pub tool_call_id: Option<String>,
    pub cache_control: Option<CacheControl>,
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

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct ChatCompletionChoice {
    pub index: i32,
    pub message: ChatMessage,
    pub finish_reason: Option<String>,
    pub logprobs: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct Usage {
    pub prompt_tokens: i32,
    pub completion_tokens: i32,
    pub total_tokens: i32,
    pub prompt_tokens_details: Option<TokenDetails>,
    pub completion_tokens_details: Option<TokenDetails>,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct TokenDetails {
    pub cached_tokens: Option<i32>,
    pub audio_tokens: Option<i32>,
    pub image_tokens: Option<i32>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Capability {
    Chat,
    ImageGeneration,
    ImageEdit,
    VideoGeneration,
    AudioTts,
    AudioStt,
    Embedding,
}

impl Capability {
    pub fn as_str(&self) -> &'static str {
        match self {
            Capability::Chat => "chat",
            Capability::ImageGeneration => "image_generation",
            Capability::ImageEdit => "image_edit",
            Capability::VideoGeneration => "video_generation",
            Capability::AudioTts => "audio_tts",
            Capability::AudioStt => "audio_stt",
            Capability::Embedding => "embedding",
        }
    }
}

impl std::str::FromStr for Capability {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "chat" => Ok(Self::Chat),
            "image_generation" => Ok(Self::ImageGeneration),
            "image_edit" => Ok(Self::ImageEdit),
            "video_generation" => Ok(Self::VideoGeneration),
            "audio_tts" => Ok(Self::AudioTts),
            "audio_stt" => Ok(Self::AudioStt),
            "embedding" => Ok(Self::Embedding),
            _ => Err(format!("unknown capability: {s}")),
        }
    }
}

impl std::fmt::Display for Capability {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Deserialize, Serialize)]
pub struct Protocol(pub String);

impl Protocol {
    pub fn openai() -> Self {
        Self("openai".to_string())
    }
    pub fn anthropic() -> Self {
        Self("anthropic".to_string())
    }
    pub fn gemini() -> Self {
        Self("gemini".to_string())
    }
    pub fn ollama() -> Self {
        Self("ollama".to_string())
    }
    pub fn azure_openai() -> Self {
        Self("azure_openai".to_string())
    }
    pub fn bedrock() -> Self {
        Self("bedrock".to_string())
    }
    pub fn cohere() -> Self {
        Self("cohere".to_string())
    }
    pub fn mistral() -> Self {
        Self("mistral".to_string())
    }
    pub fn groq() -> Self {
        Self("groq".to_string())
    }
    pub fn together() -> Self {
        Self("together".to_string())
    }
    pub fn vllm() -> Self {
        Self("vllm".to_string())
    }
    pub fn sglang() -> Self {
        Self("sglang".to_string())
    }
    pub fn llama_cpp() -> Self {
        Self("llama_cpp".to_string())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::ops::Deref for Protocol {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        self.0.as_str()
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ImageGenerationRequest {
    pub model: String,
    pub prompt: String,
    pub n: Option<i32>,
    pub size: Option<String>,
    pub quality: Option<String>,
    pub style: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ImageGenerationResponse {
    pub created: i64,
    pub data: Vec<ImageData>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ImageData {
    pub url: Option<String>,
    pub b64_json: Option<String>,
    pub revised_prompt: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ImageEditRequest {
    pub model: String,
    pub prompt: String,
    pub n: Option<i32>,
    pub size: Option<String>,
    pub response_format: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AudioTtsRequest {
    pub model: String,
    pub input: String,
    pub voice: String,
    pub response_format: Option<String>,
}

/// Audio bytes are passed separately, not inside this DTO.
#[derive(Debug, Clone, Deserialize)]
pub struct AudioSttRequest {
    pub model: String,
    pub language: Option<String>,
    pub response_format: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AudioSttResponse {
    pub text: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct EmbeddingRequest {
    pub model: String,
    pub input: Vec<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct EmbeddingResponse {
    pub object: String,
    pub data: Vec<EmbeddingData>,
    pub model: String,
    pub usage: Usage,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct EmbeddingData {
    pub object: String,
    pub embedding: Vec<f32>,
    pub index: i32,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct VideoGenerationRequest {
    pub model: String,
    pub prompt: String,
    pub duration: Option<f32>,
    pub size: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct VideoGenerationResponse {
    pub created: i64,
    pub data: Vec<VideoData>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct VideoData {
    pub url: Option<String>,
    pub b64_json: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum MultimodalCapability {
    ImageGeneration,
    AudioTts,
    AudioStt,
    Embedding,
}

impl From<MultimodalCapability> for Capability {
    fn from(capability: MultimodalCapability) -> Self {
        match capability {
            MultimodalCapability::ImageGeneration => Capability::ImageGeneration,
            MultimodalCapability::AudioTts => Capability::AudioTts,
            MultimodalCapability::AudioStt => Capability::AudioStt,
            MultimodalCapability::Embedding => Capability::Embedding,
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct MultimodalRequest {
    pub model: String,
    pub capability: MultimodalCapability,
    pub body: serde_json::Value,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

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
"#;
        let config: AppConfig = serde_yaml::from_str(yaml).expect("parse yaml");
        assert_eq!(config.server.port, 3000);
    }

    #[test]
    fn capability_round_trips() {
        let capabilities = [
            Capability::Chat,
            Capability::ImageGeneration,
            Capability::ImageEdit,
            Capability::VideoGeneration,
            Capability::AudioTts,
            Capability::AudioStt,
            Capability::Embedding,
        ];
        for cap in capabilities {
            let s = cap.as_str();
            assert_eq!(cap.to_string(), s);
            assert_eq!(Capability::from_str(s).unwrap(), cap);
        }
        assert!(Capability::from_str("unknown").is_err());
    }

    #[test]
    fn capability_serde_roundtrip() {
        for cap in [
            Capability::Chat,
            Capability::ImageGeneration,
            Capability::ImageEdit,
            Capability::VideoGeneration,
            Capability::AudioTts,
            Capability::AudioStt,
            Capability::Embedding,
        ] {
            let serialized = serde_yaml::to_string(&cap).expect("serialize");
            let deserialized: Capability = serde_yaml::from_str(&serialized).expect("deserialize");
            assert_eq!(deserialized, cap);
            assert_eq!(deserialized.to_string(), cap.as_str());
        }
    }

    #[test]
    fn protocol_constructors_and_accessors() {
        let p = Protocol::sglang();
        assert_eq!(p.as_str(), "sglang");
        assert_eq!(&*p, "sglang");

        assert_eq!(Protocol::openai().as_str(), "openai");
        assert_eq!(Protocol::anthropic().as_str(), "anthropic");
        assert_eq!(Protocol::gemini().as_str(), "gemini");
        assert_eq!(Protocol::ollama().as_str(), "ollama");
        assert_eq!(Protocol::azure_openai().as_str(), "azure_openai");
        assert_eq!(Protocol::bedrock().as_str(), "bedrock");
        assert_eq!(Protocol::cohere().as_str(), "cohere");
        assert_eq!(Protocol::mistral().as_str(), "mistral");
        assert_eq!(Protocol::groq().as_str(), "groq");
        assert_eq!(Protocol::together().as_str(), "together");
        assert_eq!(Protocol::vllm().as_str(), "vllm");
        assert_eq!(Protocol::llama_cpp().as_str(), "llama_cpp");
    }

    #[test]
    fn image_generation_request_serde_roundtrip() {
        let req = ImageGenerationRequest {
            model: "dall-e-3".into(),
            prompt: "a cat in a hat".into(),
            n: Some(1),
            size: Some("1024x1024".into()),
            quality: Some("hd".into()),
            style: Some("vivid".into()),
        };

        let yaml = serde_yaml::to_string(&req).expect("serialize");
        let parsed: ImageGenerationRequest = serde_yaml::from_str(&yaml).expect("deserialize");

        assert_eq!(parsed.model, req.model);
        assert_eq!(parsed.prompt, req.prompt);
        assert_eq!(parsed.n, req.n);
        assert_eq!(parsed.size, req.size);
        assert_eq!(parsed.quality, req.quality);
        assert_eq!(parsed.style, req.style);
    }

    #[test]
    fn chat_content_text_roundtrips() {
        let content = ChatContent::text("hello");
        let json = serde_json::to_string(&content).unwrap();
        assert_eq!(json, "\"hello\"");
        let parsed: ChatContent = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.as_text(), Some("hello".to_string()));
    }

    #[test]
    fn chat_content_parts_roundtrip() {
        let content = ChatContent::Parts(vec![
            ChatContentPart::Text { text: "What ".into() },
            ChatContentPart::ImageUrl { image_url: ImageUrl { url: "https://example.com/img.png".into(), detail: Some("high".into()) } },
            ChatContentPart::Text { text: " is this?".into() },
        ]);
        let json = serde_json::to_string(&content).unwrap();
        let parsed: ChatContent = serde_json::from_str(&json).unwrap();
        assert!(parsed.has_images());
        assert!(parsed.as_text().unwrap().contains("What"));
    }

    #[test]
    fn tool_choice_function_serializes() {
        let tc = ToolChoice::Function { function: FunctionName { name: "get_weather".into() } };
        let json = serde_json::to_string(&tc).unwrap();
        assert!(json.contains("function"));
        assert!(json.contains("get_weather"));
    }

    #[test]
    fn tool_call_roundtrips() {
        let call = ToolCall {
            id: "call_1".into(),
            r#type: "function".into(),
            function: FunctionCall { name: "get_weather".into(), arguments: "{\"city\":\"Paris\"}".into() },
        };
        let json = serde_json::to_string(&call).unwrap();
        let parsed: ToolCall = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.function.name, "get_weather");
    }

    #[test]
    fn response_format_json_schema_serializes() {
        let rf = ResponseFormat::JsonSchema {
            json_schema: JsonSchema {
                name: "answer".into(),
                schema: Some(serde_json::json!({"type":"object"})),
                strict: Some(true),
            },
        };
        let json = serde_json::to_string(&rf).unwrap();
        assert!(json.contains("json_schema"));
    }

    #[test]
    fn stop_array_roundtrips() {
        let stop = Stop::Array(vec!["stop".into(), "halt".into()]);
        let json = serde_json::to_string(&stop).unwrap();
        let parsed: Stop = serde_json::from_str(&json).unwrap();
        match parsed {
            Stop::Array(arr) => assert_eq!(arr.len(), 2),
            _ => panic!("expected array"),
        }
    }

    #[test]
    fn usage_details_roundtrip() {
        let usage = Usage {
            prompt_tokens: 10,
            completion_tokens: 5,
            total_tokens: 15,
            prompt_tokens_details: Some(TokenDetails { cached_tokens: Some(4), audio_tokens: None, image_tokens: None }),
            completion_tokens_details: None,
        };
        let json = serde_json::to_string(&usage).unwrap();
        let parsed: Usage = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.prompt_tokens_details.unwrap().cached_tokens, Some(4));
    }
}
