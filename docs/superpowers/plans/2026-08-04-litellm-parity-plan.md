# LiteLLM Parity Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use `superpowers:subagent-driven-development` (recommended) or `superpowers:executing-plans` to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Atteindre la parité fonctionnelle backend avec LiteLLM en six sprints, en commençant par un DTO core riche qui débloque tool-calling, vision, structured output et reasoning, puis en ajoutant le pont Anthropic pour Claude Code, le scoping des clés API, la résilience proxy, le cost tracking et l’écosystème agentique (MCP, web search, SearXNG).

**Architecture:** Un DTO commun dans `godwit-core` sert de pivot entre les protocoles clients (OpenAI, Anthropic) et les adapters providers. Chaque adapter convertit du DTO commun vers son format natif. Godwit expose à la fois les endpoints OpenAI existants et un nouvel endpoint Anthropic natif.

**Tech Stack:** Rust 2021, axum 0.7, sqlx 0.7, reqwest 0.12, serde, tokio, chrono, uuid, thiserror, rust_decimal, dashmap (rate limiting), moka (cache). UI admin en Next.js 14.

## Global Constraints

- Pas de modification de l’auth SSO/SAML existante.
- Pas de refonte de la gestion org/team/budgets (déjà couverte côté admin).
- Les migrations SQL doivent rester dans `crates/godwit-db/migrations/`.
- Les tests SQLx nécessitent `DATABASE_URL=postgres://user:pass@localhost:5432/godwit`.
- `cargo check --workspace` et `cargo test --workspace` doivent passer après chaque tâche.
- Pas de commit automatique sans accord explicite de l’utilisateur.

---

## Sprint 1 — Core DTO big-bang

### Task 1.1: Add multimodal `ChatContent` to `godwit-core`

**Files:**
- Modify: `crates/godwit-core/src/lib.rs`
- Test: `crates/godwit-core/src/lib.rs` (tests module)

**Interfaces:**
- Consumes: existing `ChatMessage.content: String`.
- Produces: `ChatContent`, `ChatContentPart`, `ImageUrl`, helper `ChatContent::as_text()`.

- [ ] **Step 1: Define `ChatContent`, `ChatContentPart`, `ImageUrl`**

```rust
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(untagged)]
pub enum ChatContent {
    Text(String),
    Parts(Vec<ChatContentPart>),
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ChatContentPart {
    Text { text: String },
    ImageUrl { image_url: ImageUrl },
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ImageUrl {
    pub url: String,
    pub detail: Option<String>,
}
```

- [ ] **Step 2: Add helper methods on `ChatContent`**

```rust
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
```

- [ ] **Step 3: Change `ChatMessage.content` from `String` to `ChatContent`**

```rust
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ChatMessage {
    pub role: String,
    pub content: ChatContent,
    pub name: Option<String>,
}
```

- [ ] **Step 4: Add tests for `ChatContent` serialization and helpers**

```rust
#[test]
fn chat_content_text_roundtrips() {
    let content = ChatContent::text("hello");
    let json = serde_json::to_string(&content).unwrap();
    assert_eq!(json, "\"hello\"");
    let parsed: ChatContent = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed.as_text(), Some("hello"));
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
```

- [ ] **Step 5: Run tests**

```bash
export PATH="/usr/local/opt/rustup/bin:$PATH"
cargo test -p godwit-core
```

---

### Task 1.2: Add tool-calling types

**Files:**
- Modify: `crates/godwit-core/src/lib.rs`
- Test: `crates/godwit-core/src/lib.rs` (tests module)

**Interfaces:**
- Consumes: nothing.
- Produces: `Tool`, `FunctionDefinition`, `ToolChoice`, `FunctionName`, `ToolCall`, `FunctionCall`.

- [ ] **Step 1: Define tool types**

```rust
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
```

- [ ] **Step 2: Add `tool_calls` and `tool_call_id` to `ChatMessage`**

```rust
pub struct ChatMessage {
    pub role: String,
    pub content: ChatContent,
    pub name: Option<String>,
    pub tool_calls: Option<Vec<ToolCall>>,
    pub tool_call_id: Option<String>,
}
```

- [ ] **Step 3: Add serialization tests**

```rust
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
```

- [ ] **Step 4: Run tests**

```bash
cargo test -p godwit-core
```

---

### Task 1.3: Add `response_format` and missing generation parameters

**Files:**
- Modify: `crates/godwit-core/src/lib.rs`
- Test: `crates/godwit-core/src/lib.rs`

**Interfaces:**
- Consumes: existing `ChatCompletionRequest`.
- Produces: `ResponseFormat`, `JsonSchema`, `Stop`, enriched `ChatCompletionRequest`.

- [ ] **Step 1: Add `ResponseFormat` and `JsonSchema`**

```rust
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
```

- [ ] **Step 2: Extend `ChatCompletionRequest`**

```rust
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
}
```

- [ ] **Step 3: Add serialization tests**

```rust
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
```

- [ ] **Step 4: Run tests**

```bash
cargo test -p godwit-core
```

---

### Task 1.4: Add reasoning, prompt caching and extended usage

**Files:**
- Modify: `crates/godwit-core/src/lib.rs`
- Modify: `crates/godwit-providers/src/adapter.rs`
- Test: both files

**Interfaces:**
- Consumes: `ChatCompletionRequest`, `ChatMessage`, `Usage`.
- Produces: `ReasoningConfig`, `ThinkingConfig`, `CacheControl`, extended `Usage`/`UsageReport`.

- [ ] **Step 1: Add reasoning and cache types in `godwit-core`**

```rust
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
```

- [ ] **Step 2: Add fields to `ChatMessage`, `ChatCompletionRequest`, `Usage`**

```rust
pub struct ChatMessage {
    pub role: String,
    pub content: ChatContent,
    pub name: Option<String>,
    pub tool_calls: Option<Vec<ToolCall>>,
    pub tool_call_id: Option<String>,
    pub cache_control: Option<CacheControl>,
}

pub struct ChatCompletionRequest {
    // ... existing fields ...
    pub reasoning: Option<ReasoningConfig>,
}

pub struct Usage {
    pub prompt_tokens: i32,
    pub completion_tokens: i32,
    pub total_tokens: i32,
    pub prompt_tokens_details: Option<TokenDetails>,
    pub completion_tokens_details: Option<TokenDetails>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct TokenDetails {
    pub cached_tokens: Option<i32>,
    pub audio_tokens: Option<i32>,
    pub image_tokens: Option<i32>,
}
```

- [ ] **Step 3: Extend `UsageReport` in `adapter.rs`**

```rust
#[derive(Debug, Clone, Default)]
pub struct UsageReport {
    pub prompt_tokens: Option<i32>,
    pub completion_tokens: Option<i32>,
    pub image_count: Option<i64>,
    pub audio_seconds: Option<f64>,
    pub video_seconds: Option<f64>,
    pub tts_characters: Option<i64>,
    pub embedding_tokens: Option<i64>,
    pub cache_read_tokens: Option<i32>,
    pub cache_write_tokens: Option<i32>,
}
```

- [ ] **Step 4: Add tests**

```rust
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
```

- [ ] **Step 5: Run tests**

```bash
cargo test -p godwit-core -p godwit-providers
```

---

### Task 1.5: Update `ChatCompletionChoice` and helpers

**Files:**
- Modify: `crates/godwit-core/src/lib.rs`

**Interfaces:**
- Consumes: `ChatMessage`.
- Produces: enriched `ChatCompletionChoice`.

- [ ] **Step 1: Add `logprobs` to `ChatCompletionChoice`**

```rust
pub struct ChatCompletionChoice {
    pub index: i32,
    pub message: ChatMessage,
    pub finish_reason: Option<String>,
    pub logprobs: Option<serde_json::Value>,
}
```

- [ ] **Step 2: Run core tests**

```bash
cargo test -p godwit-core
```

---

### Task 1.6: Fix compilation of `godwit-providers` adapters

**Files:**
- Modify: `crates/godwit-providers/src/openai.rs`
- Modify: `crates/godwit-providers/src/anthropic.rs`
- Modify: `crates/godwit-providers/src/azure_openai.rs`
- Modify: `crates/godwit-providers/src/gemini.rs`
- Modify: `crates/godwit-providers/src/bedrock.rs`
- Modify: `crates/godwit-providers/src/llama_cpp.rs`
- Modify: `crates/godwit-providers/src/ollama.rs`
- Modify: `crates/godwit-providers/src/vllm.rs`
- Modify: `crates/godwit-providers/src/sglang.rs`
- Modify: `crates/godwit-providers/src/openai_compatible.rs`

**Interfaces:**
- Consumes: new `ChatMessage`, `ChatContent`, `ChatCompletionRequest`.
- Produces: compiling adapters.

- [ ] **Step 1: Update OpenAI adapter**

Replace usages of `msg.content` (String) with `msg.content.as_text()` where text is required. Since OpenAI natively supports multimodal content, keep the original `ChatCompletionRequest` serialization; `ChatContent` serializes to OpenAI-compatible JSON.

```rust
// In openai.rs chat tests
ChatMessage {
    role: "user".into(),
    content: ChatContent::text("Hi"),
    name: None,
    tool_calls: None,
    tool_call_id: None,
    cache_control: None,
}
```

- [ ] **Step 2: Update Anthropic adapter**

```rust
// In AnthropicMessage construction
messages.push(AnthropicMessage {
    role: msg.role,
    content: msg.content.as_text().unwrap_or_default(),
});
```

Also ignore `tool_calls` and `tool_call_id` at this stage (handled in sprint 2).

- [ ] **Step 3: Update remaining adapters**

For each of `azure_openai.rs`, `gemini.rs`, `bedrock.rs`, `llama_cpp.rs`, `ollama.rs`, `vllm.rs`, `sglang.rs`, `openai_compatible.rs`:
- Replace `msg.content` access with `msg.content.as_text()`.
- Add missing struct fields with `..Default::default()` if using struct update syntax, or construct new `ChatMessage` literals with all fields.
- Keep behavior identical; advanced features are added in later sprints.

- [ ] **Step 4: Run workspace check**

```bash
export PATH="/usr/local/opt/rustup/bin:$PATH"
cargo check --workspace
```

---

### Task 1.7: Fix compilation of `godwit-api` and `godwit-bin`

**Files:**
- Modify: `crates/godwit-api/src/proxy.rs`
- Modify: `crates/godwit-api/src/model_router.rs` (tests)
- Modify: `crates/godwit-bin/src/main.rs` / `bootstrap.rs` if needed
- Modify: integration tests in `tests/` if they build `ChatMessage`

- [ ] **Step 1: Update `proxy.rs` test data**

All inline `ChatMessage` literals in `proxy.rs` tests must use `ChatContent::text(...)`.

- [ ] **Step 2: Update `model_router.rs` tests**

Same: replace `content: "...".into()` with `content: ChatContent::text("...")`.

- [ ] **Step 3: Run full workspace test**

```bash
export PATH="/usr/local/opt/rustup/bin:$PATH"
cargo test --workspace
```

If DB tests fail without `DATABASE_URL`, run at least:

```bash
cargo check --workspace
cargo test -p godwit-core -p godwit-providers
```

---

### Task 1.8: Verify UI admin types still compile

**Files:**
- Modify: `apps/admin/lib/types.ts` if it references backend chat types.

- [ ] **Step 1: Check admin build**

```bash
cd /home/thomas/work/Godwit/apps/admin
npm run build 2>&1 | head -50
```

The admin UI does not currently manipulate chat message content directly, so no changes are expected.

---

## Sprint 2 — Pont Anthropic natif + clés scopées aux modèles

### Task 2.1: Add Anthropic proxy module and `/v1/messages` routes

**Files:**
- Create: `crates/godwit-api/src/anthropic_proxy.rs`
- Modify: `crates/godwit-api/src/lib.rs` to mount the router

**Interfaces:**
- Consumes: `ChatCompletionRequest`, `ChatCompletionResponse`, `DbModelRouter`, `ApiKey`.
- Produces: Anthropic-compatible request/response types and conversion functions.

- [ ] **Step 1: Define Anthropic DTOs**

```rust
#[derive(Debug, Deserialize)]
pub struct AnthropicMessagesRequest {
    pub model: String,
    pub messages: Vec<AnthropicMessage>,
    pub max_tokens: i32,
    #[serde(default)]
    pub stream: bool,
    #[serde(default)]
    pub system: Option<String>,
    pub temperature: Option<f32>,
    pub top_p: Option<f32>,
    pub top_k: Option<i32>,
    pub stop_sequences: Option<Vec<String>>,
    pub tools: Option<Vec<AnthropicTool>>,
    pub tool_choice: Option<AnthropicToolChoice>,
}

#[derive(Debug, Deserialize)]
pub struct AnthropicMessage {
    pub role: String,
    pub content: String,
}

#[derive(Debug, Deserialize)]
pub struct AnthropicTool {
    pub name: String,
    pub description: Option<String>,
    pub input_schema: Option<serde_json::Value>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AnthropicToolChoice {
    Auto,
    Any,
    Tool { name: String },
}

#[derive(Debug, Serialize)]
pub struct AnthropicMessagesResponse {
    pub id: String,
    pub r#type: String,
    pub role: String,
    pub model: String,
    pub content: Vec<AnthropicContentBlock>,
    pub stop_reason: Option<String>,
    pub usage: AnthropicUsage,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "snake_case", tag = "type")]
pub enum AnthropicContentBlock {
    Text { text: String },
    ToolUse { id: String, name: String, input: serde_json::Value },
}

#[derive(Debug, Serialize)]
pub struct AnthropicUsage {
    pub input_tokens: i32,
    pub output_tokens: i32,
}
```

- [ ] **Step 2: Implement conversion Anthropic → Core**

```rust
fn anthropic_message_to_core(msg: AnthropicMessage) -> ChatMessage {
    ChatMessage {
        role: msg.role,
        content: ChatContent::text(msg.content),
        name: None,
        tool_calls: None,
        tool_call_id: None,
        cache_control: None,
    }
}

fn anthropic_tool_to_core(tool: AnthropicTool) -> Tool {
    Tool {
        r#type: "function".into(),
        function: FunctionDefinition {
            name: tool.name,
            description: tool.description,
            parameters: tool.input_schema,
        },
    }
}

fn anthropic_request_to_core(req: AnthropicMessagesRequest) -> ChatCompletionRequest {
    let mut messages = Vec::new();
    if let Some(system) = req.system {
        messages.push(ChatMessage {
            role: "system".into(),
            content: ChatContent::text(system),
            name: None,
            tool_calls: None,
            tool_call_id: None,
            cache_control: None,
        });
    }
    messages.extend(req.messages.into_iter().map(anthropic_message_to_core));

    ChatCompletionRequest {
        model: req.model,
        messages,
        max_tokens: Some(req.max_tokens),
        temperature: req.temperature,
        top_p: req.top_p,
        top_k: req.top_k,
        stop: req.stop_sequences.map(Stop::Array),
        stream: Some(req.stream),
        tools: req.tools.map(|ts| ts.into_iter().map(anthropic_tool_to_core).collect()),
        tool_choice: req.tool_choice.map(|tc| match tc {
            AnthropicToolChoice::Auto => ToolChoice::Auto,
            AnthropicToolChoice::Any => ToolChoice::Required,
            AnthropicToolChoice::Tool { name } => ToolChoice::Function { function: FunctionName { name } },
        }),
        ..Default::default()
    }
}
```

- [ ] **Step 3: Wire route in `godwit-api/src/lib.rs`**

```rust
Router::new()
    .nest("/v1", proxy::router())
    .nest("/v1", anthropic_proxy::router())
```

- [ ] **Step 4: Add integration test with `reqwest`/`wiremock`**

POST `/v1/messages` with an Anthropic-shaped body and assert OpenAI-compatible upstream call is made.

---

### Task 2.2: Add `allowed_models` to API keys

**Files:**
- Create: `crates/godwit-db/migrations/<timestamp>_api_key_allowed_models.sql`
- Modify: `crates/godwit-db/src/models.rs`
- Modify: `crates/godwit-db/src/repositories/api_keys.rs`
- Modify: `crates/godwit-api/src/admin/api_keys.rs`

- [ ] **Step 1: Write migration**

```sql
ALTER TABLE api_keys ADD COLUMN allowed_models TEXT[] NOT NULL DEFAULT '{}';
```

- [ ] **Step 2: Add field to `ApiKey` model**

```rust
pub allowed_models: Vec<String>,
```

- [ ] **Step 3: Update repository create/get/list methods**

Bind and return `allowed_models`.

- [ ] **Step 4: Update admin handler `CreateApiKeyRequest`**

```rust
pub struct CreateApiKeyRequest {
    pub name: String,
    pub scopes: Vec<String>,
    pub allowed_models: Vec<String>,
}
```

---

### Task 2.3: Enforce model scope in proxy middleware

**Files:**
- Create or modify: `crates/godwit-api/src/middleware.rs` or new `crates/godwit-api/src/model_scope.rs`
- Modify: `crates/godwit-api/src/proxy.rs` and `crates/godwit-api/src/anthropic_proxy.rs` to apply middleware

- [ ] **Step 1: Implement extraction of model from request body**

```rust
async fn extract_model_from_body(body: Bytes) -> Option<String> {
    serde_json::from_slice::<serde_json::Value>(&body)
        .ok()
        .and_then(|v| v.get("model")?.as_str().map(|s| s.to_string()))
}
```

- [ ] **Step 2: Implement scope check**

```rust
fn is_model_allowed(api_key: &ApiKey, model: &str) -> bool {
    api_key.allowed_models.is_empty() || api_key.allowed_models.iter().any(|m| m == model)
}
```

- [ ] **Step 3: Return 403 if disallowed**

```rust
return Err(crate::error::ApiError::Forbidden);
```

---

### Task 2.4: Adapt admin UI for allowed models

**Files:**
- Modify: `apps/admin/app/(dashboard)/admin/api-keys/page.tsx`
- Modify: `apps/admin/app/(dashboard)/admin/api-keys/actions.ts`
- Modify: `apps/admin/lib/types.ts`

- [ ] **Step 1: Fetch model list from `/api/v1/models` admin endpoint**

If no admin endpoint exists, reuse the public `/v1/models` endpoint or add `GET /admin/models`.

- [ ] **Step 2: Add multi-select in create dialog**

```tsx
<select multiple name="allowed_models">
  {models.map((m) => <option key={m.id} value={m.public_id}>{m.public_id}</option>)}
</select>
```

- [ ] **Step 3: Display allowed models column**

```tsx
{ accessorKey: 'allowed_models', header: 'Allowed Models', cell: (info) => (info.getValue() as string[]).join(', ') }
```

---

## Sprint 3 — Résilience proxy

### Task 3.1: Implement retry wrapper

**Files:**
- Create: `crates/godwit-api/src/resilience.rs`

- [ ] **Step 1: Define `RetryPolicy`**

```rust
pub struct RetryPolicy {
    pub max_retries: u32,
    pub base_delay_ms: u64,
    pub max_delay_ms: u64,
    pub retryable_statuses: Vec<u16>,
}

impl Default for RetryPolicy {
    fn default() -> Self {
        Self {
            max_retries: 2,
            base_delay_ms: 500,
            max_delay_ms: 8000,
            retryable_statuses: vec![429, 502, 503, 504],
        }
    }
}
```

- [ ] **Step 2: Implement `with_retry` helper**

```rust
pub async fn with_retry<F, Fut, T>(policy: &RetryPolicy, f: F) -> Result<T, ProviderError>
where
    F: Fn() -> Fut,
    Fut: std::future::Future<Output = Result<T, ProviderError>>,
{
    let mut last_err = None;
    for attempt in 0..=policy.max_retries {
        match f().await {
            Ok(v) => return Ok(v),
            Err(e) => {
                let retryable = matches!(&e, ProviderError::Http { status, .. } if policy.retryable_statuses.contains(status));
                if !retryable || attempt == policy.max_retries {
                    return Err(e);
                }
                last_err = Some(e);
                let delay = std::cmp::min(
                    policy.base_delay_ms * 2u64.pow(attempt),
                    policy.max_delay_ms,
                );
                tokio::time::sleep(std::time::Duration::from_millis(delay)).await;
            }
        }
    }
    Err(last_err.expect("last error set if loop exits with Err"))
}
```

### Task 3.2: Implement fallback / failover

**Files:**
- Modify: `crates/godwit-api/src/model_router.rs`
- Modify: `crates/godwit-api/src/proxy.rs`

- [ ] **Step 1: Read fallback chain from `models.config`**

```json
{ "fallbacks": ["gpt-4o-backup", "claude-sonnet-backup"] }
```

- [ ] **Step 2: On failure, attempt each fallback in order**

```rust
for fallback_id in fallback_chain {
    match state.model_router.resolve(fallback_id, capability).await {
        Ok(resolved) => return call_adapter(resolved, req).await,
        Err(_) => continue,
    }
}
```

### Task 3.3: Implement load balancing

**Files:**
- Modify: `crates/godwit-api/src/model_router.rs`

- [ ] **Step 1: Replace ambiguous error with strategy selection**

```rust
pub enum LoadBalanceStrategy {
    RoundRobin,
    LeastBusy,
    Latency,
}
```

- [ ] **Step 2: Add shared state for counters / latency tracking**

Use `std::sync::atomic::AtomicUsize` or `dashmap` in `DbModelRouter`.

### Task 3.4: Implement rate limiting

**Files:**
- Create: `crates/godwit-api/src/rate_limit.rs`
- Modify: `crates/godwit-api/src/proxy.rs`
- Modify: `crates/godwit-api/src/anthropic_proxy.rs`

- [ ] **Step 1: Add token bucket with `dashmap`**

```rust
pub struct RateLimiter {
    buckets: DashMap<(Uuid, String), Mutex<TokenBucket>>,
}
```

- [ ] **Step 2: Check and consume tokens before adapter call**

Return `PasteurError::RateLimited` (HTTP 429) when exhausted.

---

## Sprint 4 — Usage & cost tracking

### Task 4.1: Extract real usage in chat adapters

**Files:**
- Modify: `crates/godwit-providers/src/openai.rs`
- Modify: `crates/godwit-providers/src/azure_openai.rs`
- Modify: `crates/godwit-providers/src/llama_cpp.rs`
- Modify: `crates/godwit-providers/src/ollama.rs`
- Modify: `crates/godwit-providers/src/vllm.rs`
- Modify: `crates/godwit-providers/src/sglang.rs`

- [ ] **Step 1: Parse `usage` field from JSON response**

```rust
let usage_report = UsageReport {
    prompt_tokens: body.usage.as_ref().map(|u| u.prompt_tokens),
    completion_tokens: body.usage.as_ref().map(|u| u.completion_tokens),
    ..Default::default()
};
```

### Task 4.2: Implement cost calculation

**Files:**
- Modify: `crates/godwit-providers/src/usage.rs`

- [ ] **Step 1: Define pricing schema helpers**

```rust
pub fn compute_chat_cost(pricing: &serde_json::Value, usage: &UsageReport) -> Option<Decimal> {
    let input_price = pricing.get("input_price_per_million")?.as_f64()?;
    let output_price = pricing.get("output_price_per_million")?.as_f64()?;
    let input_tokens = usage.prompt_tokens? as f64;
    let output_tokens = usage.completion_tokens? as f64;
    let cost = (input_tokens * input_price + output_tokens * output_price) / 1_000_000.0;
    Decimal::from_f64(cost)
}
```

- [ ] **Step 2: Wire `compute_cost` in `proxy.rs`**

Replace placeholder with `compute_cost(&resolved.model.pricing, &usage)`.

---

## Sprint 5 — Capacités manquantes

### Task 5.1: Implement Gemini streaming

**Files:**
- Modify: `crates/godwit-providers/src/gemini.rs`

- [ ] **Step 1: Implement `chat_stream` using Gemini server-sent events**

Return a `BoxStream` of normalized `SseEvent` similar to Anthropic.

### Task 5.2: Normalize OpenAI/Azure/local SSE streams

**Files:**
- Modify: `crates/godwit-providers/src/streaming.rs`
- Modify: `crates/godwit-providers/src/openai.rs`, `azure_openai.rs`, `llama_cpp.rs`, `ollama.rs`, `vllm.rs`, `sglang.rs`, `openai_compatible.rs`

- [ ] **Step 1: Parse `delta.content`, `delta.tool_calls`, `finish_reason`**

Emit normalized events: `{ "type": "delta", "delta": "..." }`, `{ "type": "finish", "usage": {...} }`.

### Task 5.3: Embeddings for Anthropic, Gemini, Bedrock

**Files:**
- Modify: `crates/godwit-providers/src/anthropic.rs`
- Modify: `crates/godwit-providers/src/gemini.rs`
- Modify: `crates/godwit-providers/src/bedrock.rs`

- [ ] **Step 1: Implement `embedding` adapter method**

Use each provider’s embedding endpoint and normalize to `EmbeddingResponse`.

### Task 5.4: Audio/image generation for non-OpenAI providers

**Files:**
- Modify relevant provider adapters.

- [ ] **Step 1: Implement where supported**

Azure DALL-E, Gemini image generation, Bedrock Titan/Stable Diffusion.

### Task 5.5: Moderation, rerank and batch endpoints

**Files:**
- Create: `crates/godwit-api/src/moderation.rs`, `rerank.rs`, `batch.rs`

- [ ] **Step 1: Add `/v1/moderations`, `/v1/rerank`, `/v1/batches` routes**

Delegate to provider adapters or implement fallback logic.

---

## Sprint 6 — MCP, web search & SearXNG

### Task 6.1: Web search passthrough

**Files:**
- Modify: `crates/godwit-providers/src/openai.rs`
- Modify: `crates/godwit-providers/src/anthropic.rs`
- Modify: `crates/godwit-providers/src/gemini.rs`

- [ ] **Step 1: Detect native web search tools**

If `tools` contains `web_search`, `web_search_20250305`, or `google_search`, pass them through to the provider.

### Task 6.2: SearXNG search provider

**Files:**
- Create: `crates/godwit-providers/src/searxng.rs`
- Modify: `crates/godwit-providers/src/lib.rs`

- [ ] **Step 1: Define `SearxngProvider` adapter**

```rust
pub struct SearxngProvider { client: Client }
```

- [ ] **Step 2: Implement search call**

```rust
async fn search(&self, profile: &ResolvedProfile, query: &str) -> Result<Vec<SearchResult>, ProviderError> {
    let url = format!("{}/search?q={}&format=json", profile.base_url, urlencoding::encode(query));
    // parse JSON
}
```

- [ ] **Step 3: Expose as tool or fallback**

When a model does not support native web search, replace the tool call with a SearXNG call and inject results as a tool message.

### Task 6.3: MCP client

**Files:**
- Create: `crates/godwit-mcp/` (new crate) or `crates/godwit-providers/src/mcp_client.rs`

- [ ] **Step 1: Add MCP transport (stdio / SSE)**

- [ ] **Step 2: Register MCP servers from config**

```yaml
mcp_servers:
  - name: filesystem
    command: npx
    args: ["-y", "@modelcontextprotocol/server-filesystem", "/tmp"]
```

- [ ] **Step 3: Expose MCP tools in chat requests**

Convert MCP tool definitions into `godwit_core::Tool` and route tool calls to the MCP server.

### Task 6.4: MCP server (optional)

**Files:**
- Create: `crates/godwit-mcp/src/server.rs`

- [ ] **Step 1: Expose Godwit models as MCP resources/tools**

Allow external MCP clients to call Godwit models through the MCP protocol.

---

## Verification

After each task:

```bash
export PATH="/usr/local/opt/rustup/bin:$PATH"
cargo check --workspace
```

After each sprint:

```bash
export PATH="/usr/local/opt/rustup/bin:$PATH"
DATABASE_URL=postgres://user:pass@localhost:5432/godwit cargo test --workspace
```

For UI changes:

```bash
cd /home/thomas/work/Godwit/apps/admin
npm run build
```
