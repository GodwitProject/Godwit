# P2-C: Streaming & Paramètres Avancés Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Implement streaming Gemini normalisé, streaming normalization pour TOUS les providers, prompt caching (Anthropic+OpenAI+Gemini), et tous les paramètres avancés.

**Architecture:** 
- Streaming : `StreamTranslator` par provider (Gemini, OpenAI, Azure, llama.cpp, Ollama, vLLM, SGLang)
- Caching : `cache_control` (Anthropic), cache local TTL (OpenAI), `cachedContent` API (Gemini)
- Paramètres : DTO étendu + provider-specific translation

**Tech Stack:** Rust, serde, tokio (streams), dashmap (cache), reqwest

## Global Constraints

- Streaming Gemini : normalisé vers format OpenAI `chat.completion.chunk`
- Streaming normalization : TOUS les providers (OpenAI, Azure, llama.cpp, Ollama, vLLM, SGLang)
- Prompt caching : Anthropic (cache_control), OpenAI (local cache TTL), Gemini (cachedContent API)
- Paramètres : TOUS (`stop`, `logprobs`, `seed`, `n`, `presence_penalty`, `frequency_penalty`, `repetition_penalty`, `logit_bias`, `user`)
- `[DONE]` terminator obligatoire pour tous les streams
- Usage tracking : prompt/candidates/cached tokens pour tous les providers
- TTL cache par défaut : 3600s (1 heure), configurable
- Max cache size : 10000 entries, LRU eviction
- Use Decimal for all cost calculations (no floats)
- Follow existing code patterns in `godwit-providers/src/*.rs`, `godwit-api/src/proxy.rs`

---

## Task Decomposition

**Task 1:** Streaming Gemini + normalisation  
**Task 2:** Streaming translators (OpenAI, Azure)  
**Task 3:** Streaming translators (llama.cpp, Ollama)  
**Task 4:** Streaming translators (vLLM, SGLang)  
**Task 5:** Prompt caching Anthropic (cache_control)  
**Task 6:** Prompt caching OpenAI (local cache TTL)  
**Task 7:** Prompt caching Gemini (cachedContent API)  
**Task 8:** Paramètres avancés DTO (core)  
**Task 9:** Paramètres avancés providers (translation)  
**Task 10:** Integration streaming + cache + params  
**Task 11:** Config updates (cache TTL, max size)  
**Task 12:** Integration tests and documentation  

---

## Task Briefs (Summaries)

### Task 1: Streaming Gemini + Normalization
- **Files:** Create `crates/godwit-providers/src/gemini_stream.rs`, modify `gemini.rs`
- **Changes:** `GeminiStreamTranslator`, `chat_stream()` implementation, usage tracking
- **Tests:** Translation correcte, finish_reason mappé, usage extrait
- **Commit:** "feat: add Gemini streaming with OpenAI normalization"

### Task 2: Streaming Translators (OpenAI, Azure)
- **Files:** Create `crates/godwit-providers/src/stream_translators/openai.rs`, `azure.rs`
- **Changes:** `OpenAiStreamTranslator`, `AzureOpenAiStreamTranslator` implémentant `StreamTranslator`
- **Tests:** SSE parsing, delta extraction, [DONE] terminator
- **Commit:** "feat: add OpenAI/Azure stream translators"

### Task 3: Streaming Translators (llama.cpp, Ollama)
- **Files:** Create `crates/godwit-providers/src/stream_translators/llama_cpp.rs`, `ollama.rs`
- **Changes:** `LlamaCppStreamTranslator`, `OllamaStreamTranslator` (JSON lines parsing)
- **Tests:** Format-specific parsing, normalization
- **Commit:** "feat: add llama.cpp/Ollama stream translators"

### Task 4: Streaming Translators (vLLM, SGLang)
- **Files:** Create `crates/godwit-providers/src/stream_translators/vllm.rs`, `sglang.rs`
- **Changes:** `VllmStreamTranslator`, `SglangStreamTranslator`
- **Tests:** SSE parsing, normalization
- **Commit:** "feat: add vLLM/SGLang stream translators"

### Task 5: Prompt Caching Anthropic
- **Files:** Modify `crates/godwit-providers/src/anthropic.rs`
- **Changes:** Pass `cache_control` to Anthropic API (`X-Cache-Control: ephemeral`)
- **Tests:** Cache control header present, tokens cached reported
- **Commit:** "feat: add Anthropic cache_control support"

### Task 6: Prompt Caching OpenAI
- **Files:** Create `crates/godwit-api/src/prompt_cache.rs`, modify `openai.rs`
- **Changes:** `PromptCache` struct (DashMap, TTL, LRU), cache hit/miss logic
- **Tests:** Cache hit on same messages, TTL expiration, LRU eviction
- **Commit:** "feat: add OpenAI local prompt caching"

### Task 7: Prompt Caching Gemini
- **Files:** Modify `crates/godwit-providers/src/gemini.rs`
- **Changes:** `create_cached_content()`, `generate_with_cache()` methods
- **Tests:** Cached content created, reused, TTL respected
- **Commit:** "feat: add Gemini cachedContent API support"

### Task 8: Paramètres Avancés DTO
- **Files:** Modify `crates/godwit-core/src/lib.rs`
- **Changes:** Add `stop`, `logprobs`, `top_logprobs`, `seed`, `n`, `presence_penalty`, `frequency_penalty`, `repetition_penalty`, `logit_bias`, `user` to `ChatCompletionRequest`
- **Tests:** Serialization, validation (max 4 stop sequences, n >= 1)
- **Commit:** "feat: add advanced params to ChatCompletionRequest DTO"

### Task 9: Paramètres Avancés Providers
- **Files:** Modify `crates/godwit-providers/src/{openai,anthropic,gemini,llama_cpp,ollama,vllm,sglang}.rs`
- **Changes:** Translate params to provider-specific format (ex: `stop` → `stop_sequences` for Anthropic)
- **Tests:** Params passed correctly, unsupported params ignored gracefully
- **Commit:** "feat: add advanced params translation for all providers"

### Task 10: Integration Streaming + Cache + Params
- **Files:** Modify `crates/godwit-api/src/proxy.rs`
- **Changes:** Wire up stream translators, prompt cache, params validation in `chat_completions()`
- **Tests:** End-to-end streaming with cache, params forwarded
- **Commit:** "feat: integrate streaming/cache/params in proxy handlers"

### Task 11: Config Updates
- **Files:** Modify `crates/godwit-core/src/lib.rs` (AppConfig), `config.example.yaml`
- **Changes:** Add `cache.ttl_secs`, `cache.max_size`, `cache.enabled` to AppConfig
- **Tests:** Config parsing, defaults correct, overrides work
- **Commit:** "feat: add prompt cache config options"

### Task 12: Integration Tests + Documentation
- **Files:** Create `tests/{streaming,cache,params}_integration.rs`, create `docs/streaming-params.md`
- **Changes:** Integration tests (ignored), documentation with examples
- **Tests:** Compile integration tests, manual run with server
- **Commit:** "docs: add P2-C integration tests and guide"

---

## Execution Notes

- **Task briefs:** Use `scripts/task-brief <plan-file> <task-N>` to generate detailed briefs with exact code snippets
- **Task reports:** Each implementer writes to `.superpowers/sdd/<plan-basename>/task-N-report.md`
- **Review package:** Use `scripts/review-package <plan-file> <base> <head>` after each task
- **Ledger:** Track progress in `.superpowers/sdd/<plan-basename>/progress.md`

---

## Success Criteria

- [ ] All 12 tasks complete and reviewed
- [ ] Streaming: Gemini + 6 providers normalized
- [ ] Caching: Anthropic/OpenAI/Gemini all working
- [ ] Params: All 9 params supported, provider translation correct
- [ ] Integration tests compile (marked `#[ignore]`)
- [ ] Documentation complete

---

## Timeline Estimée

- Tasks 1-4 (Streaming): 4-5 jours
- Tasks 5-7 (Caching): 3-4 jours
- Tasks 8-10 (Params + integration): 3-4 jours
- Tasks 11-12 (Config + docs): 1-2 jours
- **Total:** 11-15 jours
