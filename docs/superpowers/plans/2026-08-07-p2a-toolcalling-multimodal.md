# P2-A: Tool-Calling & Multimodal Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Implement tool-calling générique avec résolution automatique MCP + web search, support multimodal (images), et JSON Schema / structured output.

**Architecture:** 
- Tool-calling : Extension du DTO core + boucle agentic avec résolution MCP/SearXNG
- Multimodal : `ChatContent` enum (Text, Image) avec backward compat String → Vec
- JSON Schema : `response_format` avec validation post-réponse et guided decoding provider-specific

**Tech Stack:** Rust, serde, serde_json, jsonschema crate, reqwest, async/await

## Global Constraints

- Agentic loop max 4 itérations (configurable dans `AppConfig.agentic.max_iterations`)
- Tool resolution : MCP + web search (SearXNG) uniquement, pas de function calls non-enregistrés
- Multimodal : backward compat avec `ChatMessage.content: String` (converti en `Vec<ChatContent>`)
- JSON Schema : validation post-réponse OBLIGATOIRE (même avec guided decoding)
- Guided decoding : vLLM (`guided_json`), SGLang (`json_schema`), OpenAI (`strict: true`)
- Logging : chaque itération agentic loggée dans `request_logs` avec `tool_calls_count` et `iteration`
- Timeout par itération : 120s (configurable)
- Use `Decimal` for all cost calculations (no floats)
- Follow existing code patterns in `godwit-api/src/proxy.rs`, `godwit-providers/src/*.rs`

---

## Task Decomposition

**Task 1:** Database migrations for agentic tracking  
**Task 2:** Tool-calling DTO extensions (core)  
**Task 3:** Multimodal DTO extensions (core)  
**Task 4:** JSON Schema DTO extensions (core)  
**Task 5:** Agentic loop implementation  
**Task 6:** MCP tool resolution  
**Task 7:** Web search tool resolution (SearXNG)  
**Task 8:** Multimodal provider adapters (OpenAI, Anthropic, Gemini)  
**Task 9:** JSON Schema provider translation + validation  
**Task 10:** Integrate agentic loop into proxy handlers  
**Task 11:** Config updates (AppConfig.agentic.max_iterations)  
**Task 12:** Integration tests and documentation  

---

## Task Briefs (Summaries)

### Task 1: Database Migrations for Agentic Tracking
- **Files:** Create `crates/godwit-db/migrations/20260810000001_request_logs_agentic.{up,down}.sql`
- **Changes:** Add `tool_calls_count INTEGER`, `agentic_iteration INTEGER` to `request_logs`
- **Tests:** Run `sqlx migrate run`, verify columns exist
- **Commit:** "db: add request_logs agentic tracking columns"

### Task 2: Tool-Calling DTO Extensions (Core)
- **Files:** Modify `crates/godwit-core/src/lib.rs`
- **Changes:** Add `Tool`, `FunctionDefinition`, `ToolChoice`, `ToolCall`, `FunctionCall`; extend `ChatCompletionRequest` with `tools`, `tool_choice`, `parallel_tool_calls`; extend `ChatCompletionChoice` with `tool_calls`
- **Tests:** Serialization tests for Tool, ToolChoice
- **Commit:** "feat: add tool-calling DTO extensions"

### Task 3: Multimodal DTO Extensions (Core)
- **Files:** Modify `crates/godwit-core/src/lib.rs`
- **Changes:** Add `ChatContent` enum (Text, Image), `ImageUrl`, `ImageDetail`, `CacheControl`; change `ChatMessage.content` to `Option<Vec<ChatContent>>` with deserializer backward-compat
- **Tests:** Deserialization tests (String → Vec, Array → Vec)
- **Commit:** "feat: add multimodal DTO extensions"

### Task 4: JSON Schema DTO Extensions (Core)
- **Files:** Modify `crates/godwit-core/src/lib.rs`
- **Changes:** Add `ResponseFormat` enum (Text, JsonObject, JsonSchema), `JsonSchemaDefinition`; add `response_format` to `ChatCompletionRequest`
- **Tests:** Serialization tests for ResponseFormat::JsonSchema
- **Commit:** "feat: add JSON Schema DTO extensions"

### Task 5: Agentic Loop Implementation
- **Files:** Create `crates/godwit-api/src/agentic_loop.rs`
- **Changes:** `AgenticLoop` struct with `max_iterations`, `execute()` method, tool list building, iteration loop
- **Tests:** Max iterations exceeded, tool calls present → loop, no tool calls → return
- **Commit:** "feat: implement agentic loop with tool resolution"

### Task 6: MCP Tool Resolution
- **Files:** Modify `crates/godwit-api/src/agentic_loop.rs`, use existing `McpRegistry`
- **Changes:** Integrate `McpRegistry::list_tools()` and `McpRegistry::call_tool()` into agentic loop
- **Tests:** MCP tool call returns result, tool result appended to conversation
- **Commit:** "feat: integrate MCP tool resolution into agentic loop"

### Task 7: Web Search Tool Resolution (SearXNG)
- **Files:** Create `crates/godwit-providers/src/web_search.rs`, modify `agentic_loop.rs`
- **Changes:** `SearxngClient` with `search(query)` method, inject `web_search` tool into tool list
- **Tests:** Web search returns results, results formatted as tool_result
- **Commit:** "feat: add SearXNG web search tool resolution"

### Task 8: Multimodal Provider Adapters
- **Files:** Modify `crates/godwit-providers/src/{openai,anthropic,gemini}.rs`
- **Changes:** Translate `ChatContent` → provider format (OpenAI: text/image_url, Anthropic: text/image with base64, Gemini: text/inlineData/fileData)
- **Tests:** Multimodal request → correct provider format, image base64 encoding
- **Commit:** "feat: add multimodal adapters for OpenAI/Anthropic/Gemini"

### Task 9: JSON Schema Provider Translation + Validation
- **Files:** Modify `crates/godwit-providers/src/{openai,anthropic,gemini,vllm,sglang,llama_cpp,ollama}.rs`, create `crates/godwit-api/src/response_validation.rs`
- **Changes:** Translate `ResponseFormat::JsonSchema` → provider-specific (OpenAI: `strict`, vLLM: `guided_json`, etc.); post-response validation with `jsonschema` crate
- **Tests:** Valid JSON passes, invalid JSON fails with error, provider translation correct
- **Commit:** "feat: add JSON Schema translation and validation"

### Task 10: Integrate Agentic Loop into Proxy Handlers
- **Files:** Modify `crates/godwit-api/src/proxy.rs`
- **Changes:** Replace direct `call_chat()` with `agentic_loop.execute()` when `tools` present; log `tool_calls_count` and `agentic_iteration`
- **Tests:** Tool-calling request → agentic loop triggered, logging correct
- **Commit:** "feat: integrate agentic loop into chat_completions handler"

### Task 11: Config Updates (AppConfig.agentic.max_iterations)
- **Files:** Modify `crates/godwit-core/src/lib.rs` (AppConfig), `crates/godwit-api/src/config.rs`
- **Changes:** Add `agentic: AgenticConfig { max_iterations: u32 }` to AppConfig, default 4
- **Tests:** Config parsing, default value, override from YAML
- **Commit:** "feat: add agentic.max_iterations config"

### Task 12: Integration Tests and Documentation
- **Files:** Create `tests/{tool_calling,multimodal,json_schema}_integration.rs`, create `docs/tool-calling-multimodal.md`
- **Changes:** Integration tests (ignored, require server+DB), documentation with examples
- **Tests:** Compile integration tests, manual run with server
- **Commit:** "docs: add P2-A integration tests and guide"

---

## Execution Notes

- **Task briefs:** Use `scripts/task-brief <plan-file> <task-N>` to generate detailed briefs with exact code snippets
- **Task reports:** Each implementer writes to `.superpowers/sdd/<plan-basename>/task-N-report.md`
- **Review package:** Use `scripts/review-package <plan-file> <base> <head>` after each task
- **Ledger:** Track progress in `.superpowers/sdd/<plan-basename>/progress.md`

---

## Success Criteria

- [ ] All 12 tasks complete and reviewed
- [ ] Tool-calling: MCP + web search resolution working
- [ ] Multimodal: Images in requests, correct provider translation
- [ ] JSON Schema: Validation working, guided decoding for vLLM/SGLang/OpenAI
- [ ] Agentic loop: Max 4 iterations, logging correct
- [ ] Integration tests compile (marked `#[ignore]`)
- [ ] Documentation complete

---

## Timeline Estimée

- Tasks 1-4 (DTO core): 2-3 jours
- Tasks 5-7 (Agentic loop): 2-3 jours
- Tasks 8-9 (Providers): 3-4 jours
- Tasks 10-12 (Integration): 2-3 jours
- **Total:** 9-13 jours
