# P2-C: Streaming & Paramètres Avancés — Design Spec

**Date:** 2026-08-07  
**Auteur:** Thomas (avec opencode)  
**Statut:** En review  
**Release cible:** v1.4.0

---

## 1. Vue d'Ensemble

Cette spec couvre les fonctionnalités P2-C pour compléter la parité LiteLLM sur le streaming multi-provider, le prompt caching, et les paramètres avancés.

### 1.1 Contexte

P1 + P2-A + P2-B ont livré :
- ✅ Fallback/failover, usage tracking complet
- ✅ Tool-calling, multimodal, JSON Schema
- ✅ Batch API unifiée, moderation/rerank fallbacks

**Ce qui manque :**
- ❌ Streaming Gemini (actuellement `CapabilityNotSupported`)
- ❌ Streaming normalization (OpenAI/Azure/llama.cpp/Ollama/vLLM/SGLang non normalisés)
- ❌ Prompt caching (Anthropic `cache_control`, OpenAI auto, Gemini `cached_content`)
- ❌ Paramètres avancés (`stop`, `logprobs`, `seed`, `n`, penalties)

---

## 2. Fonctionnalités P2-C

### 2.1 Streaming Gemini Normalisé

**Actuel :** `CapabilityNotSupported`

**P2-C :** Support complet avec normalisation vers format OpenAI `chat.completion.chunk`

#### 2.1.1 Gemini Streaming API

Gemini utilise un endpoint différent :
```
POST https://generativelanguage.googleapis.com/v1beta/models/{model}:streamGenerateContent?alt=sse
```

**Response format (Gemini-native) :**
```json
{
  "candidates": [{
    "content": {
      "parts": [{"text": "Hello"}],
      "role": "model"
    },
    "finishReason": "STOP",
    "index": 0
  }],
  "usageMetadata": {
    "promptTokenCount": 10,
    "candidatesTokenCount": 5
  }
}
```

#### 2.1.2 Normalisation vers OpenAI Format

**Objectif :** Traduire chaque chunk Gemini → format OpenAI `chat.completion.chunk`

```rust
// crates/godwit-providers/src/gemini.rs

pub struct GeminiStreamTranslator {
    buffer: String,
    role_sent: bool,
    usage_reported: bool,
}

impl GeminiStreamTranslator {
    pub fn translate_chunk(&mut self, gemini_chunk: &GeminiResponse) -> Option<SseEvent> {
        // Extract text from parts
        let text = gemini_chunk.candidates.first()?
            .content.parts.first()?
            .text.as_ref()?;
        
        // Build OpenAI-style delta
        let delta = OpenAiDelta {
            role: if !self.role_sent { Some("assistant") } else { None },
            content: Some(text.clone()),
            tool_calls: None,
            finish_reason: match gemini_chunk.candidates.first()?.finishReason.as_deref() {
                Some("STOP") => Some("stop"),
                Some("MAX_TOKENS") => Some("length"),
                Some("SAFETY") => Some("content_filter"),
                _ => None,
            },
        };
        
        self.role_sent = true;
        
        Some(SseEvent {
            event: "chat.completion.chunk",
            data: serde_json::to_string(&delta).unwrap(),
        })
    }
}
```

#### 2.1.3 Usage Tracking

Gemini renvoie `usageMetadata` dans le **dernier chunk** seulement. Il faut bufferiser et extraire :

```rust
// Dans GeminiStreamTranslator
pub fn finalize(&mut self, last_chunk: &GeminiResponse) -> Option<UsageReport> {
    last_chunk.usageMetadata.as_ref().map(|usage| UsageReport {
        prompt_tokens: Some(usage.promptTokenCount as i32),
        completion_tokens: Some(usage.candidatesTokenCount as i32),
        cache_read_tokens: usage.cachedContentTokenCount.map(|c| c as i32),
        ..Default::default()
    })
}
```

---

### 2.2 Streaming Normalization (TOUS Providers)

**Actuel :** SSE brut relaié tel quel (pas de normalisation delta/finish_reason)

**P2-C :** Normalisation pour TOUS les providers vers format OpenAI `chat.completion.chunk`

#### 2.2.1 Providers à Normaliser

| Provider | Format Actuel | Format Cible | Complexité |
|----------|---------------|--------------|------------|
| **OpenAI** | SSE brut | `chat.completion.chunk` | Faible |
| **Azure OpenAI** | SSE brut | `chat.completion.chunk` | Faible |
| **llama.cpp** | SSE brut | `chat.completion.chunk` | Moyenne |
| **Ollama** | SSE brut (JSON lines) | `chat.completion.chunk` | Moyenne |
| **vLLM** | SSE brut | `chat.completion.chunk` | Faible |
| **SGLang** | SSE brut | `chat.completion.chunk` | Faible |

#### 2.2.2 Architecture : Stream Translators

```rust
// crates/godwit-providers/src/streaming.rs (existant, à étendre)

pub trait StreamTranslator {
    fn translate(&mut self, raw_chunk: &str) -> Option<SseEvent>;
    fn finalize(&mut self) -> Option<SseEvent>;  // Pour finish_reason, usage
}

// Implémentations par provider
pub struct OpenAiStreamTranslator { ... }
pub struct AzureOpenAiStreamTranslator { ... }
pub struct LlamaCppStreamTranslator { ... }
pub struct OllamaStreamTranslator { ... }
pub struct VllmStreamTranslator { ... }
pub struct SglangStreamTranslator { ... }
```

#### 2.2.3 Exemple : Ollama Translation

Ollama utilise un format JSON lines différent :

```json
{"model":"llama3","created_at":"2024-01-01T00:00:00Z","message":{"role":"assistant","content":"Hello"},"done":false}
```

**Translation :**
```rust
// crates/godwit-providers/src/ollama.rs

pub struct OllamaStreamTranslator {
    role_sent: bool,
}

impl StreamTranslator for OllamaStreamTranslator {
    fn translate(&mut self, raw_line: &str) -> Option<SseEvent> {
        let ollama_chunk: OllamaChunk = serde_json::from_str(raw_line).ok()?;
        
        let delta = OpenAiDelta {
            role: if !self.role_sent { Some("assistant") } else { None },
            content: Some(ollama_chunk.message.content),
            tool_calls: None,
            finish_reason: if ollama_chunk.done { Some("stop") } else { None },
        };
        
        self.role_sent = true;
        
        Some(SseEvent {
            event: "chat.completion.chunk",
            data: serde_json::to_string(&delta).unwrap(),
        })
    }
    
    fn finalize(&mut self) -> Option<SseEvent> {
        // Send [DONE] terminator
        Some(SseEvent {
            event: "chat.completion.chunk",
            data: "[DONE]".to_string(),
        })
    }
}
```

---

### 2.3 Prompt Caching (Anthropic + OpenAI + Gemini)

**Objectif :** Support complet du caching pour réduire coûts et latence.

#### 2.3.1 Anthropic `cache_control`

**Déjà dans le DTO** (P2-A) :
```rust
pub enum ChatContent {
    Text {
        text: String,
        cache_control: Option<CacheControl>,
    },
    Image { ... }
}
```

**Implémentation :**
```rust
// crates/godwit-providers/src/anthropic.rs

let anthropic_content = content.iter().map(|c| match c {
    ChatContent::Text { text, cache_control } => {
        AnthropicContent::Text {
            text: text.clone(),
            cache_control: cache_control.as_ref().map(|cc| {
                AnthropicCacheControl::Ephemeral { type_: "ephemeral".to_string() }
            }),
        }
    }
    // ...
}).collect();
```

#### 2.3.2 OpenAI Automatic Caching

OpenAI ne supporte PAS `cache_control` explicite. Mais on peut tracker localement :

```rust
// crates/godwit-api/src/prompt_cache.rs (nouveau fichier)

pub struct PromptCache {
    cache: DashMap<String, CachedPrompt>,
    ttl_secs: u64,
}

pub struct CachedPrompt {
    content_hash: String,
    cached_at: DateTime<Utc>,
    usage_count: u32,
}

impl PromptCache {
    pub fn check_cache(&self, messages: &[ChatMessage]) -> Option<CachedPrompt> {
        let hash = hash_messages(messages);
        self.cache.get(&hash).and_then(|entry| {
            if entry.cached_at + Duration::from_secs(self.ttl_secs) > Utc::now() {
                Some(entry.clone())
            } else {
                None
            }
        })
    }
    
    pub fn cache_prompt(&self, messages: &[ChatMessage]) {
        let hash = hash_messages(messages);
        self.cache.insert(hash, CachedPrompt {
            content_hash: hash,
            cached_at: Utc::now(),
            usage_count: 0,
        });
    }
}
```

**Usage :**
```rust
// Dans proxy.rs, avant d'appeler le provider
if let Some(_cached) = state.prompt_cache.check_cache(&req.messages) {
    // Skip API call, return cached response
    // (Or add header: X-Godwit-Cache: HIT)
} else {
    // Call provider, then cache
    let response = call_provider(...).await?;
    state.prompt_cache.cache_prompt(&req.messages);
}
```

#### 2.3.3 Gemini `cachedContent`

Gemini a une API de caching explicite :

```rust
// crates/godwit-providers/src/gemini.rs

// Step 1: Create cached content
pub async fn create_cached_content(
    &self,
    profile: &ResolvedProfile,
    model: &str,
    contents: Vec<GeminiContent>,
    ttl_secs: u32,
) -> Result<String, ProviderError> {
    let url = format!("{}/v1beta/cachedContents", profile.base_url);
    
    let body = serde_json::json!({
        "model": model,
        "contents": contents,
        "ttl": format!("{}s", ttl_secs),
    });
    
    let response = self.client.post(&url).json(&body).send().await?;
    let result: GeminiCachedContentResponse = response.json().await?;
    
    Ok(result.name)  // "cachedContents/{id}"
}

// Step 2: Use cached content in generation
pub async fn generate_with_cache(
    &self,
    profile: &ResolvedProfile,
    model: &str,
    cached_content_id: &str,
    prompt: &str,
) -> Result<GeminiResponse, ProviderError> {
    let url = format!("{}/v1beta/{}:generateContent", profile.base_url, model);
    
    let body = serde_json::json!({
        "cachedContent": format!("cachedContents/{}", cached_content_id),
        "contents": [{
            "role": "user",
            "parts": [{"text": prompt}]
        }]
    });
    
    // ... send request ...
}
```

---

### 2.4 Paramètres Avancés (TOUS)

**Actuel :** Seuls `temperature` et `max_tokens` sont supportés.

**P2-C :** Ajout de TOUS les paramètres OpenAI-standard.

#### 2.4.1 DTO Extensions

```rust
// crates/godwit-core/src/lib.rs

pub struct ChatCompletionRequest {
    // ... existing fields ...
    
    /// Stop sequences (max 4)
    #[serde(default)]
    pub stop: Option<Vec<String>>,
    
    /// Log probabilities
    #[serde(default)]
    pub logprobs: Option<bool>,
    #[serde(default)]
    pub top_logprobs: Option<u32>,
    
    /// Random seed for reproducibility
    #[serde(default)]
    pub seed: Option<i64>,
    
    /// Number of completions to generate
    #[serde(default)]
    pub n: Option<u32>,
    
    /// Penalties
    #[serde(default)]
    pub presence_penalty: Option<f32>,
    #[serde(default)]
    pub frequency_penalty: Option<f32>,
    #[serde(default)]
    pub repetition_penalty: Option<f32>,
    
    /// Logit bias
    #[serde(default)]
    pub logit_bias: Option<std::collections::HashMap<String, f32>>,
    
    /// User identifier (for abuse monitoring)
    #[serde(default)]
    pub user: Option<String>,
}
```

#### 2.4.2 Provider Translation

**OpenAI :** Pass-through direct (mêmes noms de champs)

**Anthropic :**
- `stop` → `stop_sequences`
- `presence_penalty` → non supporté (ignorer)
- `frequency_penalty` → non supporté (ignorer)
- `seed` → non supporté (ignorer)
- `logprobs` → `top_k` (approximation)

**Gemini :**
- `stop` → `stopSequences`
- `temperature` → map to 0.0-2.0 scale
- `seed` → via `generationConfig.randomSeed`

**llama.cpp/Ollama/vLLM/SGLang :**
- La plupart des paramètres sont supportés nativement
- Mapping direct ou quasi-direct

---

## 3. Architecture

### 3.1 Diagramme de Flux — Streaming Normalization

```
┌─────────────┐
│   Client    │
│  (stream=true)│
└──────┬──────┘
       │
       ▼
┌─────────────────────────────────────────┐
│         proxy.rs                        │
│  chat_completions(stream=true)          │
│  ↓                                      │
│  Resolve model → provider               │
└──────────────┬──────────────────────────┘
               │
               ▼
┌─────────────────────────────────────────┐
│  Provider Adapter (ex: gemini.rs)       │
│  ┌─────────────────────────────────┐    │
│  │  Call provider streaming API    │    │
│  │  ↓                              │    │
│  │  Raw SSE stream (provider fmt)  │    │
│  │  ↓                              │    │
│  │  GeminiStreamTranslator         │    │
│  │  - Parse chunk                  │    │
│  │  - Translate to OpenAI format   │    │
│  │  - Emit SseEvent                │    │
│  └─────────────────────────────────┘    │
└──────────────┬──────────────────────────┘
               │
               ▼
┌─────────────────────────────────────────┐
│  Normalized SSE stream                  │
│  data: {"choices":[{"delta":{"role":"assistant"...}}]}  │
│  data: {"choices":[{"delta":{"content":"Hello"...}}]}   │
│  data: [DONE]                           │
└──────────────┬──────────────────────────┘
               │
               ▼
       ┌───────────────┐
       │    Client     │
       │ (reçoit du    │
       │  standard     │
       │  OpenAI)      │
       └───────────────┘
```

### 3.2 Fichiers à Créer/Modifier

**Nouveaux Fichiers :**
- `crates/godwit-providers/src/gemini_stream.rs` — Gemini streaming + translator
- `crates/godwit-providers/src/stream_translators/` — Directory avec translators par provider
  - `openai.rs`, `azure.rs`, `llama_cpp.rs`, `ollama.rs`, `vllm.rs`, `sglang.rs`
- `crates/godwit-api/src/prompt_cache.rs` — Prompt caching (OpenAI auto-cache)

**Fichiers à Modifier :**
- `crates/godwit-core/src/lib.rs` — Paramètres avancés dans `ChatCompletionRequest`
- `crates/godwit-providers/src/gemini.rs` — Streaming support + usage tracking
- `crates/godwit-providers/src/{openai,azure_openai,llama_cpp,ollama,vllm,sglang}.rs` — Stream translators
- `crates/godwit-providers/src/anthropic.rs` — `cache_control` support
- `crates/godwit-providers/src/adapter.rs` — Ajouter méthodes streaming avec translators

---

## 4. Tests

### 4.1 Tests Unitaires

```rust
// gemini_stream_tests.rs
#[test]
fn test_gemini_translator_role_sent_once() {
    // Assert: role only in first chunk
}

#[test]
fn test_gemini_translator_finish_reason_mapped() {
    // Assert: "STOP" → "stop", "MAX_TOKENS" → "length"
}

// stream_translator_tests.rs
#[test]
fn test_ollama_translator_json_lines() {
    // Assert: Ollama JSON lines → OpenAI chunks
}

#[test]
fn test_llama_cpp_translator_sse() {
    // Assert: llama.cpp SSE → OpenAI chunks
}

// prompt_cache_tests.rs
#[tokio::test]
async fn test_cache_hit() {
    // Assert: same messages → cache hit
}

#[tokio::test]
async fn test_cache_ttl_expired() {
    // Assert: after TTL → cache miss
}
```

### 4.2 Tests d'Intégration

```bash
# Gemini streaming
curl -X POST http://localhost:3000/v1/chat/completions \
  -H "Authorization: Bearer $KEY" \
  -d '{"model": "gemini/gemini-pro", "messages": [{"role": "user", "content": "Hello"}], "stream": true}'

# Anthropic caching
curl -X POST http://localhost:3000/v1/chat/completions \
  -H "Authorization: Bearer $KEY" \
  -d '{
    "model": "anthropic/claude-3",
    "messages": [{
      "role": "user",
      "content": [
        {"type": "text", "text": "Long context...", "cache_control": {"type": "ephemeral"}}
      ]
    }]
  }'

# Paramètres avancés
curl -X POST http://localhost:3000/v1/chat/completions \
  -H "Authorization: Bearer $KEY" \
  -d '{
    "model": "gpt-4o",
    "messages": [...],
    "stop": ["END", "STOP"],
    "seed": 42,
    "logprobs": true,
    "top_logprobs": 3,
    "n": 2,
    "presence_penalty": 0.5,
    "frequency_penalty": 0.3
  }'
```

---

## 5. Critères de Succès

### 5.1 Streaming Gemini

- [ ] `chat_stream()` implémenté pour Gemini
- [ ] Chunks normalisés vers format OpenAI
- [ ] Usage tracking (prompt/candidates/cached tokens)
- [ ] Tests : translation correcte, finish_reason mappé

### 5.2 Streaming Normalization

- [ ] TOUS les providers (OpenAI, Azure, llama.cpp, Ollama, vLLM, SGLang) ont un `StreamTranslator`
- [ ] Chunks normalisés (role, content, tool_calls, finish_reason)
- [ ] `[DONE]` terminator envoyé
- [ ] Tests : chaque provider traduit correctement

### 5.3 Prompt Caching

- [ ] Anthropic : `cache_control` passé à l'API
- [ ] OpenAI : cache local avec TTL
- [ ] Gemini : `cachedContent` API supportée
- [ ] Tests : cache hit/miss, TTL expiration

### 5.4 Paramètres Avancés

- [ ] TOUS les paramètres dans le DTO
- [ ] Provider-specific translation (Anthropic, Gemini, etc.)
- [ ] Tests : paramètres passés correctement, validation

---

## 6. Risques & Mitigations

| Risque | Impact | Mitigation |
|--------|--------|------------|
| Streaming translator bug | Flux coupé ou mal formaté | Tests unitaires par translator, integration tests |
| Cache memory leak | RAM explosion | TTL strict, LRU eviction, max size |
| Paramètres non supportés | Erreur provider | Validation avant envoi, fallback silencieux |
| Gemini API changes | Breaking changes | Version pinning, feature flags |

---

## 7. Timeline Estimée

| Feature | Complexité | Temps |
|---------|------------|-------|
| Streaming Gemini + normalisation | Haute | 3-4 jours |
| Streaming translators (6 providers) | Moyenne | 3-4 jours |
| Prompt caching (Anthropic + OpenAI + Gemini) | Haute | 3-4 jours |
| Paramètres avancés (DTO + translation) | Faible | 1-2 jours |
| Tests + docs | Moyenne | 2-3 jours |
| **Total** | | **12-17 jours** |

---

## 8. Notes

- **Streaming normalization est CRITIQUE** — sans ça, les clients doivent parser 7 formats différents
- **Prompt caching réduit les coûts de 50-80%** sur les prompts récurrents (ex: system prompts longs)
- **Paramètres avancés** : `stop` et `seed` sont les plus utiles au quotidien
- **Gemini streaming** : API très différente (pas de SSE standard), besoin d'un translator robuste

---

## 9. Prochaines Étapes

1. ✅ Review de cette spec par l'utilisateur
2. ⏳ Créer le plan d'implémentation (writing-plans skill)
3. ⏳ Implémenter streaming Gemini + normalization
4. ⏳ Implémenter prompt caching
5. ⏳ Implémenter paramètres avancés
6. ⏳ Tests + docs
7. ⏳ Release v1.4.0
