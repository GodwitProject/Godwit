use serde::{Deserialize, Serialize};
use std::time::Duration;

/// Server-side cached content for Gemini
#[derive(Debug, Clone)]
pub struct CachedContent {
    /// The cached content ID returned by Gemini API
    pub id: String,
    /// The model this cached content was created for
    pub model: String,
    /// Hash of the original messages for cache key matching
    pub messages_hash: u64,
    /// When this cached content was created
    pub created_at: std::time::Instant,
    /// Time-to-live for this cached content
    pub ttl: Duration,
}

impl CachedContent {
    /// Check if the cached content has expired
    pub fn is_expired(&self) -> bool {
        self.created_at.elapsed() > self.ttl
    }
}

/// Request to create cached content on Gemini servers
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateCachedContentRequest {
    /// The content to cache
    pub contents: Vec<serde_json::Value>,
    /// System instruction to include in the cache
    #[serde(skip_serializing_if = "Option::is_none")]
    pub system_instruction: Option<String>,
    /// Model to create the cache for
    pub model: String,
    /// Display name for the cached content (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub display_name: Option<String>,
    /// Time-to-live for the cached content
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ttl: Option<String>,
}

/// Response from Gemini's createCachedContent API
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateCachedContentResponse {
    /// The name of the cached content (format: cachedContents/{id})
    pub name: String,
    /// Display name if provided
    #[serde(default)]
    pub display_name: Option<String>,
    /// When the cached content was created
    pub create_time: Option<String>,
    /// When the cached content will expire
    pub expire_time: Option<String>,
    /// The model this cached content was created for
    pub model: String,
}

impl CreateCachedContentResponse {
    /// Extract the cached content ID from the name
    pub fn id(&self) -> String {
        self.name
            .strip_prefix("cachedContents/")
            .unwrap_or(&self.name)
            .to_string()
    }
}

/// Generate content request with cached content reference
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GenerateWithCacheRequest {
    /// The content to generate a response for (new content only)
    pub contents: Vec<serde_json::Value>,
    /// System instruction
    #[serde(skip_serializing_if = "Option::is_none")]
    pub system_instruction: Option<String>,
    /// Generation configuration
    #[serde(skip_serializing_if = "Option::is_none")]
    pub generation_config: Option<serde_json::Value>,
    /// Tools (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tools: Option<Vec<serde_json::Value>>,
    /// Reference to cached content
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cached_content: Option<String>,
}

/// Cache key for Gemini cached content
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct GeminiCacheKey {
    pub model: String,
    pub messages_hash: u64,
    pub system_instruction_hash: Option<u64>,
}

impl std::hash::Hash for GeminiCacheKey {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.model.hash(state);
        self.messages_hash.hash(state);
        self.system_instruction_hash.hash(state);
    }
}

impl GeminiCacheKey {
    /// Create a cache key from request parameters
    pub fn new(model: &str, messages: &[serde_json::Value], system_instruction: Option<&str>) -> Self {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        
        let mut messages_hasher = DefaultHasher::new();
        let messages_json = serde_json::to_string(messages).unwrap_or_default();
        messages_json.hash(&mut messages_hasher);
        
        let system_hash = system_instruction.map(|si| {
            let mut hasher = DefaultHasher::new();
            si.hash(&mut hasher);
            hasher.finish()
        });
        
        Self {
            model: model.to_string(),
            messages_hash: messages_hasher.finish(),
            system_instruction_hash: system_hash,
        }
    }
}
