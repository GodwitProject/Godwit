# P1 Core Resilience & Usage Tracking Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Implement fallback/failover between providers and complete usage tracking for all providers to enable accurate cost tracking.

**Architecture:** 
- Fallback: Wrapper around provider calls with configurable model chains, retry logic, and comprehensive logging
- Usage tracking: Parse usage metadata from each provider's response format and normalize to `UsageReport`

**Tech Stack:** Rust, SQLx, reqwest, async/await, DashMap for concurrent state

## Global Constraints

- All providers must return `UsageReport` with accurate token counts (or estimates for image/audio)
- Fallback max 3 attempts to prevent infinite loops and cost explosions
- Fallback triggered only on 5xx/timeout/429, never on 4xx client errors
- All fallback attempts logged to `request_logs` with `attempt_number` and `fallback_triggered`
- Pricing required for all models (validation at creation time)
- Use `Decimal` for all cost calculations (no floats)
- Follow existing code patterns in `godwit-api/src/proxy.rs`, `godwit-providers/src/*.rs`

---

## File Structure

### New Files
- `crates/godwit-api/src/fallback.rs` — Fallback configuration and orchestration
- `crates/godwit-db/migrations/20260809000001_request_logs_fallback.up.sql` — Add fallback tracking columns
- `crates/godwit-db/migrations/20260809000001_request_logs_fallback.down.sql` — Rollback migration

### Modified Files
- `crates/godwit-api/src/proxy.rs` — Integrate fallback wrappers
- `crates/godwit-api/src/resilience.rs` — Add fallback config and helpers
- `crates/godwit-providers/src/anthropic.rs` — Parse usage in non-streaming
- `crates/godwit-providers/src/gemini.rs` — Parse usage in non-streaming
- `crates/godwit-providers/src/azure_openai.rs` — Parse usage (OpenAI format)
- `crates/godwit-providers/src/llama_cpp.rs` — Parse usage (OpenAI format)
- `crates/godwit-providers/src/ollama.rs` — Parse usage (OpenAI format)
- `crates/godwit-providers/src/vllm.rs` — Parse usage (OpenAI format)
- `crates/godwit-providers/src/sglang.rs` — Parse usage (OpenAI format)
- `crates/godwit-providers/src/openai.rs` — Add usage estimates for image/audio

### Test Files
- `crates/godwit-api/src/fallback.rs` (inline tests)
- `crates/godwit-providers/src/usage_tracking_tests.rs` (new test module)

---

## Task Decomposition

**Task 1:** Database migrations for fallback logging  
**Task 2:** Fallback configuration and orchestration core  
**Task 3:** Integrate fallback into proxy handlers  
**Task 4:** Anthropic & Gemini usage tracking  
**Task 5:** OpenAI-compatible providers usage tracking (Azure, llama.cpp, Ollama, vLLM, SGLang)  
**Task 6:** OpenAI image/audio usage estimates  
**Task 7:** Cost layer consolidation and validation  
**Task 8:** Integration tests and documentation  

---

### Task 1: Database Migrations for Fallback Logging

**Files:**
- Create: `crates/godwit-db/migrations/20260809000001_request_logs_fallback.up.sql`
- Create: `crates/godwit-db/migrations/20260809000001_request_logs_fallback.down.sql`

**Interfaces:**
- Consumes: None
- Produces: New columns `attempt_number INTEGER`, `fallback_triggered BOOLEAN` in `request_logs`

- [ ] **Step 1: Create up migration**

```sql
-- Add fallback tracking columns to request_logs
ALTER TABLE request_logs 
    ADD COLUMN attempt_number INTEGER DEFAULT 1,
    ADD COLUMN fallback_triggered BOOLEAN DEFAULT FALSE;

-- Index for querying fallback-heavy requests
CREATE INDEX idx_request_logs_fallback_triggered ON request_logs(fallback_triggered) 
    WHERE fallback_triggered = TRUE;
```

- [ ] **Step 2: Create down migration**

```sql
-- Rollback fallback tracking columns
DROP INDEX IF EXISTS idx_request_logs_fallback_triggered;
ALTER TABLE request_logs 
    DROP COLUMN IF EXISTS attempt_number,
    DROP COLUMN IF EXISTS fallback_triggered;
```

- [ ] **Step 3: Verify migrations compile**

```bash
export PATH="/usr/local/opt/rustup/bin:$PATH"
export DATABASE_URL="postgres://godwit:godwit@localhost:5432/godwit"
sqlx migrate run --database-url $DATABASE_URL
```

Expected: Migrations apply successfully

- [ ] **Step 4: Commit**

```bash
git add crates/godwit-db/migrations/
git commit -m "db: add request_logs fallback tracking columns
- attempt_number: which fallback attempt (1 = primary)
- fallback_triggered: whether this was a fallback attempt
- Index on fallback_triggered for analytics
"
```

---

### Task 2: Fallback Configuration and Orchestration Core

**Files:**
- Create: `crates/godwit-api/src/fallback.rs`
- Modify: `crates/godwit-api/src/resilience.rs`

**Interfaces:**
- Consumes: `RetryPolicy` from `resilience.rs`
- Produces: `FallbackConfig`, `get_fallback_chain()`, `call_with_fallback()`

- [ ] **Step 1: Define FallbackConfig struct**

```rust
// crates/godwit-api/src/fallback.rs
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

fn default_max_attempts() -> u32 { 3 }
fn default_timeout() -> Duration { Duration::from_secs(30) }

impl FallbackConfig {
    pub fn from_model_config(config: &serde_json::Value) -> Option<Self> {
        config.get("fallbacks")
            .and_then(|v| v.as_array())
            .map(|arr| {
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
```

- [ ] **Step 2: Implement fallback orchestration**

```rust
// crates/godwit-api/src/fallback.rs (continued)
use godwit_core::{ChatCompletionRequest, PasteurError};
use godwit_providers::adapter::{ProviderError, UsageReport};
use std::sync::Arc;
use crate::{
    model_router::{DbModelRouter, ResolvedModel},
    proxy::{call_chat, map_provider_error},
    state::AppState,
};
use axum::response::{IntoResponse, Response};

/// Result of a fallback attempt
pub struct FallbackResult {
    pub response: Response,
    pub usage: UsageReport,
    pub model_used: String,
    pub attempts_made: u32,
    pub fallback_triggered: bool,
}

/// Call chat with fallback chain
pub async fn call_chat_with_fallback(
    state: &Arc<AppState>,
    initial_model: &str,
    mut req: ChatCompletionRequest,
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
            Ok((response, usage)) => {
                return Ok(FallbackResult {
                    response,
                    usage,
                    model_used: model_ref.clone(),
                    attempts_made,
                    fallback_triggered: attempt_idx > 0,
                });
            }
            Err(e) => {
                // Log attempt failure
                log::warn!(
                    "Fallback attempt {} failed for model {}: {:?}",
                    attempts_made,
                    model_ref,
                    e
                );
                last_error = Some(e);
                
                // Check if we should continue
                if attempts_made >= 3 { // max_attempts from config
                    break;
                }
                
                // Check if error is retryable
                if !is_retryable_error(&last_error.unwrap()) {
                    break;
                }
            }
        }
    }
    
    // All attempts exhausted
    Err(last_error.unwrap_or(ProviderError::Provider(
        "All fallback attempts failed".to_string()
    )))
}

/// Get fallback chain for a model
async fn get_fallback_chain(state: &Arc<AppState>, model_ref: &str) -> Vec<String> {
    // First, try to get from model config
    if let Ok(model) = state.model_repo.get_by_public_id(model_ref).await {
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
) -> Result<(Response, UsageReport), ProviderError> {
    // Resolve model
    let resolved = state
        .model_router
        .resolve(model_ref, godwit_core::Capability::Chat)
        .await
        .map_err(|e| ProviderError::Provider(e.to_string()))?;
    
    // Call chat (existing function, needs to be exported from proxy.rs)
    call_chat(state, &resolved, req).await
}

/// Check if error is retryable (5xx, timeout, 429)
fn is_retryable_error(err: &ProviderError) -> bool {
    match err {
        ProviderError::Http { status, .. } => {
            *status >= 500 || *status == 429
        }
        ProviderError::Provider(msg) => {
            msg.contains("timeout") || msg.contains("timed out")
        }
        _ => false,
    }
}
```

- [ ] **Step 3: Export call_chat from proxy.rs**

```rust
// crates/godwit-api/src/proxy.rs - modify existing function
// Change from `async fn call_chat` to `pub(crate) async fn call_chat`
pub(crate) async fn call_chat(
    state: &Arc<AppState>,
    resolved: &ResolvedModel,
    req: ChatCompletionRequest,
) -> Result<(Response, Option<godwit_providers::adapter::UsageReport>), godwit_providers::adapter::ProviderError>
{
    // ... existing implementation ...
}
```

- [ ] **Step 4: Add fallback tests**

```rust
// crates/godwit-api/src/fallback.rs (test module)
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
```

- [ ] **Step 5: Run tests**

```bash
export PATH="/usr/local/opt/rustup/bin:$PATH"
cargo test -p godwit-api fallback::tests -- --nocapture
```

Expected: All tests pass

- [ ] **Step 6: Commit**

```bash
git add crates/godwit-api/src/fallback.rs crates/godwit-api/src/proxy.rs
git commit -m "feat: add fallback orchestration core

- FallbackConfig: configurable model chains with max attempts and timeout
- call_chat_with_fallback(): retry loop with logging
- is_retryable_error(): 5xx/timeout/429 only, not 4xx
- Tests for config parsing and error classification
"
```

---

### Task 3: Integrate Fallback into Proxy Handlers

**Files:**
- Modify: `crates/godwit-api/src/proxy.rs`

**Interfaces:**
- Consumes: `call_chat_with_fallback()` from `fallback.rs`
- Produces: Modified `chat_completions()` handler with fallback support

- [ ] **Step 1: Import fallback module**

```rust
// crates/godwit-api/src/proxy.rs - add at top
use crate::fallback::{call_chat_with_fallback, FallbackResult};
```

- [ ] **Step 2: Modify chat_completions handler**

```rust
// crates/godwit-api/src/proxy.rs - find chat_completions function
async fn chat_completions(
    Extension(api_key): Extension<ApiKey>,
    State(state): State<Arc<AppState>>,
    Json(mut req): Json<ChatCompletionRequest>,
) -> Result<Response, ApiError> {
    // ... existing rate limit and budget checks ...
    
    // Use fallback wrapper instead of direct call
    let fallback_result = call_chat_with_fallback(&state, &req.model, req.clone()).await
        .map_err(map_provider_error)?;
    
    // Log the request with fallback metadata
    state.request_log_repo.spawn_insert(
        &state.pool,
        api_key.id,
        api_key.organization_id,
        api_key.team_id,
        &fallback_result.model_used,
        &req,
        Some(&fallback_result.usage),
        fallback_result.attempts_made,
        fallback_result.fallback_triggered,
    ).await;
    
    Ok(fallback_result.response)
}
```

- [ ] **Step 3: Update RequestLogRepository to accept new params**

```rust
// crates/godwit-db/src/repositories/request_logs.rs
pub async fn spawn_insert(
    &self,
    pool: &PgPool,
    api_key_id: Uuid,
    org_id: Uuid,
    team_id: Option<Uuid>,
    model_used: &str,
    request: &ChatCompletionRequest,
    usage: Option<&UsageReport>,
    attempt_number: u32,
    fallback_triggered: bool,
) {
    // ... existing insert logic, add new columns ...
    sqlx::query!(
        r#"
        INSERT INTO request_logs 
            (api_key_id, organization_id, team_id, model_used, request_body, 
             usage_data, cost_usd, attempt_number, fallback_triggered, created_at)
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, NOW())
        "#,
        api_key_id,
        org_id,
        team_id,
        model_used,
        // ... rest of params
    )
    .execute(pool)
    .await?;
}
```

- [ ] **Step 4: Run compile check**

```bash
export PATH="/usr/local/opt/rustup/bin:$PATH"
cargo check --workspace
```

Expected: No errors

- [ ] **Step 5: Commit**

```bash
git add crates/godwit-api/src/proxy.rs crates/godwit-db/src/repositories/request_logs.rs
git commit -m "feat: integrate fallback into chat_completions handler

- Use call_chat_with_fallback() wrapper
- Log attempt_number and fallback_triggered to request_logs
- Update RequestLogRepository::spawn_insert() signature
"
```

---

### Task 4: Anthropic & Gemini Usage Tracking

**Files:**
- Modify: `crates/godwit-providers/src/anthropic.rs`
- Modify: `crates/godwit-providers/src/gemini.rs`

**Interfaces:**
- Consumes: Existing response structures
- Produces: Accurate `UsageReport` for non-streaming calls

- [ ] **Step 1: Parse Anthropic usage in non-streaming**

```rust
// crates/godwit-providers/src/anthropic.rs - find chat() function
async fn chat(
    &self,
    profile: &ResolvedProfile,
    model: &Model,
    mut request: ChatCompletionRequest,
) -> Result<(ProviderResponse, UsageReport), ProviderError> {
    // ... existing request building and HTTP call ...
    
    let body: AnthropicResponse = res
        .json()
        .await
        .map_err(|e| ProviderError::Serialization(e.to_string()))?;
    
    // NEW: Parse usage from response
    let usage = UsageReport {
        prompt_tokens: Some(body.usage.input_tokens as i32),
        completion_tokens: Some(body.usage.output_tokens as i32),
        cache_read_tokens: body.usage.cache_read_input_tokens.map(|t| t as i32),
        cache_write_tokens: body.usage.cache_creation_input_tokens.map(|t| t as i32),
        ..Default::default()
    };
    
    Ok((ProviderResponse::Chat(convert_anthropic_to_godwit(body)?), usage))
}
```

- [ ] **Step 2: Verify Anthropic response structure has usage**

```rust
// crates/godwit-providers/src/anthropic.rs - check/add Usage struct
#[derive(Debug, Deserialize)]
pub struct AnthropicUsage {
    pub input_tokens: i32,
    pub output_tokens: i32,
    #[serde(default)]
    pub cache_read_input_tokens: Option<i32>,
    #[serde(default)]
    pub cache_creation_input_tokens: Option<i32>,
}

#[derive(Debug, Deserialize)]
pub struct AnthropicResponse {
    // ... existing fields ...
    pub usage: AnthropicUsage,
}
```

- [ ] **Step 3: Parse Gemini usage in non-streaming**

```rust
// crates/godwit-providers/src/gemini.rs - find chat() function
async fn chat(
    &self,
    profile: &ResolvedProfile,
    model: &Model,
    mut request: ChatCompletionRequest,
) -> Result<(ProviderResponse, UsageReport), ProviderError> {
    // ... existing request building and HTTP call ...
    
    let body: GeminiResponse = res
        .json()
        .await
        .map_err(|e| ProviderError::Serialization(e.to_string()))?;
    
    // NEW: Parse usageMetadata from response
    let usage = UsageReport {
        prompt_tokens: body.usage_metadata
            .as_ref()
            .map(|m| m.prompt_token_count as i32),
        completion_tokens: body.usage_metadata
            .as_ref()
            .map(|m| m.candidates_token_count as i32),
        cache_read_tokens: body.usage_metadata
            .as_ref()
            .and_then(|m| m.cached_content_token_count)
            .map(|c| c as i32),
        ..Default::default()
    };
    
    Ok((ProviderResponse::Chat(convert_gemini_to_godwit(body)?), usage))
}
```

- [ ] **Step 4: Verify Gemini response structure has usageMetadata**

```rust
// crates/godwit-providers/src/gemini.rs - check/add UsageMetadata struct
#[derive(Debug, Deserialize)]
pub struct GeminiUsageMetadata {
    pub prompt_token_count: i32,
    pub candidates_token_count: i32,
    #[serde(default)]
    pub cached_content_token_count: Option<i32>,
    #[serde(default)]
    pub total_token_count: Option<i32>,
}

#[derive(Debug, Deserialize)]
pub struct GeminiResponse {
    // ... existing fields ...
    #[serde(default)]
    pub usage_metadata: Option<GeminiUsageMetadata>,
}
```

- [ ] **Step 5: Add tests for usage parsing**

```rust
// crates/godwit-providers/src/usage_tracking_tests.rs (new file)
#[cfg(test)]
mod anthropic_usage_tests {
    use super::*;
    
    #[test]
    fn test_anthropic_usage_parsed() {
        let json = r#"{
            "usage": {
                "input_tokens": 100,
                "output_tokens": 50,
                "cache_read_input_tokens": 20,
                "cache_creation_input_tokens": 10
            }
        }"#;
        
        let response: AnthropicResponse = serde_json::from_str(json).unwrap();
        assert_eq!(response.usage.input_tokens, 100);
        assert_eq!(response.usage.output_tokens, 50);
    }
}

#[cfg(test)]
mod gemini_usage_tests {
    use super::*;
    
    #[test]
    fn test_gemini_usage_parsed() {
        let json = r#"{
            "usageMetadata": {
                "promptTokenCount": 100,
                "candidatesTokenCount": 50,
                "cachedContentTokenCount": 20
            }
        }"#;
        
        let response: GeminiResponse = serde_json::from_str(json).unwrap();
        assert_eq!(response.usage_metadata.unwrap().prompt_token_count, 100);
    }
}
```

- [ ] **Step 6: Run tests**

```bash
export PATH="/usr/local/opt/rustup/bin:$PATH"
cargo test -p godwit-providers usage_tracking_tests -- --nocapture
```

Expected: All tests pass

- [ ] **Step 7: Commit**

```bash
git add crates/godwit-providers/src/anthropic.rs crates/godwit-providers/src/gemini.rs crates/godwit-providers/src/usage_tracking_tests.rs
git commit -m "feat: parse usage in Anthropic and Gemini non-streaming

- Anthropic: input_tokens, output_tokens, cache tokens
- Gemini: prompt_token_count, candidates_token_count, cached_content
- Tests for JSON parsing
- Fixes: both providers returned UsageReport::default() before
"
```

---

### Task 5: OpenAI-Compatible Providers Usage Tracking

**Files:**
- Modify: `crates/godwit-providers/src/azure_openai.rs`
- Modify: `crates/godwit-providers/src/llama_cpp.rs`
- Modify: `crates/godwit-providers/src/ollama.rs`
- Modify: `crates/godwit-providers/src/vllm.rs`
- Modify: `crates/godwit-providers/src/sglang.rs`

**Interfaces:**
- Consumes: Existing response structures (OpenAI format)
- Produces: Accurate `UsageReport` for all OpenAI-compatible providers

- [ ] **Step 1: Verify OpenAI format usage structure**

All these providers return OpenAI-compatible responses with `usage` field:
```json
{
  "usage": {
    "prompt_tokens": 100,
    "completion_tokens": 50,
    "total_tokens": 150
  }
}
```

- [ ] **Step 2: Update azure_openai.rs chat()**

```rust
// crates/godwit-providers/src/azure_openai.rs - find chat() function
let body: ChatCompletionResponse = res.json().await?;
let usage = crate::usage::chat_usage_report(&body.usage);  // Already exists!
Ok((ProviderResponse::Chat(body), usage))
```

- [ ] **Step 3: Update llama_cpp.rs chat()**

```rust
// crates/godwit-providers/src/llama_cpp.rs
let body: ChatCompletionResponse = res.json().await?;
let usage = crate::usage::chat_usage_report(&body.usage);
Ok((ProviderResponse::Chat(body), usage))
```

- [ ] **Step 4: Update ollama.rs chat()**

```rust
// crates/godwit-providers/src/ollama.rs
let body: ChatCompletionResponse = res.json().await?;
let usage = crate::usage::chat_usage_report(&body.usage);
Ok((ProviderResponse::Chat(body), usage))
```

- [ ] **Step 5: Update vllm.rs chat()**

```rust
// crates/godwit-providers/src/vllm.rs
let body: ChatCompletionResponse = res.json().await?;
let usage = crate::usage::chat_usage_report(&body.usage);
Ok((ProviderResponse::Chat(body), usage))
```

- [ ] **Step 6: Update sglang.rs chat()**

```rust
// crates/godwit-providers/src/sglang.rs
let body: ChatCompletionResponse = res.json().await?;
let usage = crate::usage::chat_usage_report(&body.usage);
Ok((ProviderResponse::Chat(body), usage))
```

- [ ] **Step 7: Run compile check**

```bash
export PATH="/usr/local/opt/rustup/bin:$PATH"
cargo check --workspace
```

Expected: No errors

- [ ] **Step 8: Commit**

```bash
git add crates/godwit-providers/src/{azure_openai,llama_cpp,ollama,vllm,sglang}.rs
git commit -m "feat: parse usage in all OpenAI-compatible providers

- Azure OpenAI, llama.cpp, Ollama, vLLM, SGLang
- All use crate::usage::chat_usage_report(&body.usage)
- Fixes: all returned UsageReport::default() before
"
```

---

### Task 6: OpenAI Image/Audio Usage Estimates

**Files:**
- Modify: `crates/godwit-providers/src/openai.rs`

**Interfaces:**
- Consumes: Request and response structures
- Produces: Estimated `UsageReport` for image generation and audio

- [ ] **Step 1: Add usage estimate for image_generation()**

```rust
// crates/godwit-providers/src/openai.rs - image_generation() function
let body: ImageGenerationResponse = res.json().await?;

// Estimate usage: count images generated
let usage = UsageReport {
    image_count: Some(body.data.len() as i32),
    ..Default::default()
};

Ok((ProviderResponse::Image(body), usage))
```

- [ ] **Step 2: Add usage estimate for image_edit()**

```rust
// crates/godwit-providers/src/openai.rs - image_edit() function
let body: ImageGenerationResponse = res.json().await?;

let usage = UsageReport {
    image_count: Some(body.data.len() as i32),
    ..Default::default()
};

Ok((ProviderResponse::Image(body), usage))
```

- [ ] **Step 3: Add usage estimate for audio_tts()**

```rust
// crates/godwit-providers/src/openai.rs - audio_tts() function
let body: AudioTtsResponse = res.json().await?;

// Estimate: count characters in input
let usage = UsageReport {
    tts_characters: Some(request.input.chars().count() as i32),
    ..Default::default()
};

Ok((ProviderResponse::AudioTts(body), usage))
```

- [ ] **Step 4: Add usage estimate for audio_stt()**

```rust
// crates/godwit-providers/src/openai.rs - audio_stt() function
// Need to get audio duration from file
let duration_secs = get_audio_duration(&audio_bytes)?;

let usage = UsageReport {
    audio_seconds: Some(duration_secs),
    ..Default::default()
};

Ok((ProviderResponse::AudioStt(body), usage))

// Helper function
fn get_audio_duration(audio_bytes: &[u8]) -> Result<f64, ProviderError> {
    // Use hound or similar crate to parse WAV/MP3 headers
    // For now, return 0.0 as placeholder
    // TODO: Implement proper audio duration parsing
    Ok(0.0)
}
```

- [ ] **Step 5: Add test for image usage estimate**

```rust
// crates/godwit-providers/src/usage_tracking_tests.rs
#[test]
fn test_image_usage_estimate() {
    let usage = UsageReport {
        image_count: Some(4),
        ..Default::default()
    };
    
    assert_eq!(usage.image_count, Some(4));
}

#[test]
fn test_tts_usage_estimate() {
    let input = "Hello, world!";
    let usage = UsageReport {
        tts_characters: Some(input.chars().count() as i32),
        ..Default::default()
    };
    
    assert_eq!(usage.tts_characters, Some(13));
}
```

- [ ] **Step 6: Run tests**

```bash
export PATH="/usr/local/opt/rustup/bin:$PATH"
cargo test -p godwit-providers usage_tracking_tests -- --nocapture
```

Expected: All tests pass

- [ ] **Step 7: Commit**

```bash
git add crates/godwit-providers/src/openai.rs crates/godwit-providers/src/usage_tracking_tests.rs
git commit -m "feat: add usage estimates for OpenAI image and audio

- Image: count images generated
- TTS: count characters in input
- STT: placeholder for audio duration (TODO: proper parsing)
- Tests for estimates
"
```

---

### Task 7: Cost Layer Consolidation

**Files:**
- Modify: `crates/godwit-api/src/admin/spend.rs`
- Modify: `crates/godwit-db/src/repositories/models.rs`

**Interfaces:**
- Consumes: `compute_cost()` from `godwit-providers/src/usage.rs`
- Produces: Unified cost calculation across admin endpoints

- [ ] **Step 1: Verify compute_cost() is exported**

```rust
// crates/godwit-providers/src/lib.rs
pub use usage::{compute_cost, compute_chat_cost, compute_embedding_cost, /* etc */};
```

- [ ] **Step 2: Update spend.rs to use providers' compute_cost**

```rust
// crates/godwit-api/src/admin/spend.rs - remove duplicate compute_cost
// Replace local compute_cost() calls with:
use godwit_providers::compute_cost;

// In spend calculation functions:
let cost_usd = compute_cost(&model.pricing, capability, &usage);
```

- [ ] **Step 3: Add pricing validation to model creation**

```rust
// crates/godwit-db/src/repositories/models.rs - create() function
pub async fn create(
    &self,
    // ... params ...
    pricing: serde_json::Value,
) -> Result<Model, PasteurError> {
    // Validate pricing has required fields
    validate_pricing(&pricing)?;
    
    // ... existing create logic ...
}

fn validate_pricing(pricing: &serde_json::Value) -> Result<(), PasteurError> {
    let has_input_price = pricing.get("input_price_per_million").is_some();
    let has_output_price = pricing.get("output_price_per_million").is_some();
    
    if !has_input_price || !has_output_price {
        return Err(PasteurError::Validation(
            "pricing must include input_price_per_million and output_price_per_million".to_string()
        ));
    }
    
    Ok(())
}
```

- [ ] **Step 4: Add migration to backfill pricing for existing models**

```sql
-- crates/godwit-db/migrations/20260809000002_backfill_pricing.up.sql
-- Backfill default pricing for models without pricing
UPDATE models 
SET pricing = jsonb_build_object(
    'input_price_per_million', 0.0,
    'output_price_per_million', 0.0
)
WHERE pricing IS NULL OR pricing = '{}'::jsonb;

-- Add NOT NULL constraint after backfill
ALTER TABLE models 
ALTER COLUMN pricing SET DEFAULT '{}'::jsonb;
```

- [ ] **Step 5: Run compile check**

```bash
export PATH="/usr/local/opt/rustup/bin:$PATH"
cargo check --workspace
```

Expected: No errors

- [ ] **Step 6: Commit**

```bash
git add crates/godwit-api/src/admin/spend.rs crates/godwit-db/src/repositories/models.rs crates/godwit-db/migrations/20260809000002_backfill_pricing.*
git commit -m "feat: consolidate cost layer and add pricing validation

- Use godwit_providers::compute_cost() in admin/spend.rs
- validate_pricing() ensures required fields on model creation
- Migration backfills default pricing for existing models
- Fixes: duplicate cost logic between crates
"
```

---

### Task 8: Integration Tests and Documentation

**Files:**
- Create: `docs/fallback-usage-tracking.md`
- Modify: `CHANGELOG_LITELLM_PARITY.md` (or create new CHANGELOG section)

**Interfaces:**
- Consumes: All implemented features
- Produces: Documentation and integration tests

- [ ] **Step 1: Write integration test for fallback**

```rust
// tests/fallback_integration.rs (new file in tests/)
#[tokio::test]
#[ignore] // Requires running server and DB
async fn test_fallback_chain() {
    // Setup: Configure model with fallback chain
    // Mock: Primary provider returns 503
    // Assert: Fallback to secondary provider succeeds
    // Assert: request_logs shows fallback_triggered = true
}

#[tokio::test]
#[ignore]
async fn test_fallback_exhausted() {
    // Setup: Configure model with 2 fallbacks
    // Mock: All providers return 503
    // Assert: Last error returned
    // Assert: request_logs shows 3 attempts
}
```

- [ ] **Step 2: Write integration test for usage tracking**

```rust
// tests/usage_tracking_integration.rs
#[tokio::test]
#[ignore]
async fn test_anthropic_usage_tracked() {
    // Make chat request to Anthropic
    // Assert: UsageReport has prompt_tokens, completion_tokens
    // Assert: /spend/logs shows correct cost
}

#[tokio::test]
#[ignore]
async fn test_gemini_usage_tracked() {
    // Make chat request to Gemini
    // Assert: UsageReport has prompt_tokens, completion_tokens
}
```

- [ ] **Step 3: Write documentation**

```markdown
# Fallback & Usage Tracking Guide

## Fallback Configuration

```yaml
models:
  - public_id: gpt-4o
    provider_profile_id: uuid-openai
    provider_model_id: gpt-4o
    config:
      fallbacks:
        - anthropic/claude-sonnet-4-20250514
        - gemini/gemini-2.5-pro
      max_fallback_attempts: 3
      timeout_per_attempt_secs: 30
```

## Usage Tracking

All providers now report usage:
- **Chat**: prompt_tokens, completion_tokens, cache tokens
- **Embeddings**: embedding_tokens
- **Images**: image_count
- **Audio TTS**: tts_characters
- **Audio STT**: audio_seconds (estimated)

## Cost Calculation

Cost = (input_tokens * input_price + output_tokens * output_price) / 1_000_000

Pricing configured per model in `models.pricing` JSONB column.
```

- [ ] **Step 4: Update changelog**

```markdown
## [v1.1.0] - 2026-08-XX

### Added
- Fallback/failover between providers with configurable chains
- Usage tracking for all providers (Anthropic, Gemini, Azure, llama.cpp, Ollama, vLLM, SGLang)
- Usage estimates for image generation and audio
- Cost layer consolidation
- request_logs.attempt_number and fallback_triggered columns

### Changed
- All providers now return accurate UsageReport (was UsageReport::default() for 7 providers)
- Pricing validation on model creation

### Fixed
- Anthropic non-streaming usage not tracked
- Gemini non-streaming usage not tracked
- OpenAI-compatible providers usage not tracked
```

- [ ] **Step 5: Run integration tests (if DB available)**

```bash
export PATH="/usr/local/opt/rustup/bin:$PATH"
export DATABASE_URL="postgres://godwit:godwit@localhost:5432/godwit"
cargo test --test fallback_integration -- --nocapture
cargo test --test usage_tracking_integration -- --nocapture
```

- [ ] **Step 6: Commit**

```bash
git add docs/fallback-usage-tracking.md tests/*.rs CHANGELOG_LITELLM_PARITY.md
git commit -m "docs: add fallback & usage tracking guide and integration tests

- Integration tests for fallback chain and exhaustion
- Integration tests for usage tracking per provider
- Documentation: configuration, usage tracking, cost calculation
- Changelog update for v1.1.0
"
```

---

## Testing Strategy

### Unit Tests (Tasks 2, 4, 5, 6)
- Fallback config parsing
- Error classification (retryable vs non-retryable)
- Usage JSON parsing per provider
- Cost calculation accuracy

### Integration Tests (Task 8)
- Fallback chain execution (requires mock providers)
- Usage tracking end-to-end
- Cost aggregation in /spend endpoints

### Manual Testing
```bash
# Test fallback
curl -X POST http://localhost:3000/v1/chat/completions \
  -H "Authorization: Bearer $KEY" \
  -d '{"model": "gpt-4o", "messages": [{"role": "user", "content": "test"}]}'

# Check request_logs for fallback metadata
curl http://localhost:3000/api/v1/spend/logs?api_key_id=$KEY_ID

# Verify usage tracking
curl http://localhost:3000/api/v1/spend
```

---

## Rollback Plan

If issues arise:
1. Revert migrations: `sqlx migrate revert --database-url $DATABASE_URL`
2. Revert code: `git revert <fallback commits>`
3. Fallback is opt-in via model config — can disable without code changes

---

## Success Criteria

- [ ] All 9 providers return accurate UsageReport
- [ ] Fallback configurable per model
- [ ] Fallback triggered only on 5xx/timeout/429
- [ ] request_logs shows attempt_number and fallback_triggered
- [ ] Cost calculations match pricing * usage
- [ ] All unit tests pass
- [ ] Integration tests pass (with DB)
- [ ] Documentation complete
