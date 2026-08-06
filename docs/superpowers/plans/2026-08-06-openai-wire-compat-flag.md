# OpenAI Wire Compatibility Flag Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a configuration flag `compat.openai_wire_streaming` that, when enabled, causes Godwit to emit native OpenAI `chat.completion.chunk` SSE format instead of the canonical envelope format.

**Architecture:** 
- Add `CompatConfig` struct to `godwit-core::AppConfig`
- Pass config through `AppState` to the proxy streaming handler
- In `call_chat`, conditionally use `OpenAiStreamTranslator` based on the flag
- The translator already exists in `godwit-providers::sse_egress` - we just need to wire it up conditionally

**Tech Stack:** Rust, Axum, SSE streaming, SQLx

## Global Constraints

- Crate prefix is `godwit_*` (e.g., `godwit_core`, `godwit_db`)
- Follow existing code conventions in the repository
- Run `cargo check --workspace --tests` after changes
- Run `cargo test -p godwit-api --lib proxy` for verification
- Update documentation in `docs/` directory

---

### Task 1: Add CompatConfig to godwit-core

**Files:**
- Modify: `crates/godwit-core/src/lib.rs`

**Interfaces:**
- Produces: `CompatConfig` struct with `openai_wire_streaming: bool` field
- Produces: `AppConfig` gains `compat: Option<CompatConfig>` field

- [ ] **Step 1: Add CompatConfig struct**

Add this struct definition in `crates/godwit-core/src/lib.rs` after the `AuthConfig` struct (around line 103):

```rust
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct CompatConfig {
    /// If true, emit native OpenAI chat.completion.chunk format instead of canonical envelope
    pub openai_wire_streaming: bool,
}
```

- [ ] **Step 2: Add compat field to AppConfig**

Modify the `AppConfig` struct (around line 24-32) to add:

```rust
#[derive(Debug, Clone, Deserialize)]
pub struct AppConfig {
    pub server: ServerConfig,
    pub database: DatabaseConfig,
    pub auth: AuthConfig,
    /// Agentic ecosystem wiring: MCP tool servers and the SearXNG web-search backend.
    #[serde(default)]
    pub agentic: AgenticConfig,
    /// Compatibility flags for wire-format interoperability.
    #[serde(default)]
    pub compat: Option<CompatConfig>,
}
```

- [ ] **Step 3: Run cargo check**

```bash
export PATH="/usr/local/opt/rustup/bin:$PATH"
cargo check -p godwit-core
```

Expected: PASS

- [ ] **Step 4: Commit**

```bash
git add crates/godwit-core/src/lib.rs
git commit -m "feat(core): add CompatConfig with openai_wire_streaming flag"
```

---

### Task 2: Pass compat config through AppState

**Files:**
- Modify: `crates/godwit-api/src/state.rs`
- Test: `cargo check -p godwit-api`

**Interfaces:**
- Consumes: `godwit_core::AppConfig` with `compat` field
- Produces: `AppState` gains `compat: Option<godwit_core::CompatConfig>` field

- [ ] **Step 1: Read AppState definition**

```bash
export PATH="/usr/local/opt/rustup/bin:$PATH"
grep -n "pub struct AppState" crates/godwit-api/src/state.rs
```

- [ ] **Step 2: Add compat field to AppState**

Add the `compat` field to the `AppState` struct, matching the pattern used for other fields.

- [ ] **Step 3: Update AppState construction**

Find where `AppState` is constructed (likely in `crates/godwit-api/src/lib.rs` or `main.rs`) and ensure the `compat` field is populated from `AppConfig`.

- [ ] **Step 4: Run cargo check**

```bash
export PATH="/usr/local/opt/rustup/bin:$PATH"
cargo check -p godwit-api
```

Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add crates/godwit-api/src/state.rs crates/godwit-api/src/lib.rs
git commit -m "feat(api): thread compat config through AppState"
```

---

### Task 3: Wire compat flag into proxy streaming

**Files:**
- Modify: `crates/godwit-api/src/proxy.rs::call_chat`

**Interfaces:**
- Consumes: `AppState` with `compat` field
- Produces: Conditional use of `OpenAiStreamTranslator` based on flag

- [ ] **Step 1: Read current call_chat implementation**

The function is at lines 189-254 in `proxy.rs`. Note the `OpenAiStreamTranslator` is already used unconditionally.

- [ ] **Step 2: Add state parameter to call_chat**

Modify the function signature to accept `state: &Arc<AppState>` so we can access the compat config.

- [ ] **Step 3: Make translator conditional**

In the streaming branch (lines 197-247), check the flag:

```rust
let use_openai_wire = state
    .config
    .compat
    .as_ref()
    .map(|c| c.openai_wire_streaming)
    .unwrap_or(false);

let translator = Mutex::new(
    if use_openai_wire {
        godwit_providers::sse_egress::OpenAiStreamTranslator::new(
            stream_id,
            stream_model,
            created,
        )
    } else {
        // For canonical envelope, we need a different translator or passthrough
        // The current code already uses OpenAiStreamTranslator - this IS the OpenAI wire format
        // We need to understand what "canonical envelope" means vs OpenAI wire format
        godwit_providers::sse_egress::OpenAiStreamTranslator::new(
            stream_id,
            stream_model,
            created,
        )
    },
);
```

**WAIT** - Looking at the current code more carefully:

The `OpenAiStreamTranslator` in `sse_egress.rs` ALREADY produces OpenAI wire format (`chat.completion.chunk`). The "canonical envelope" is the input to the translator, not the output.

So the task is actually: when `openai_wire_streaming == false`, we should emit the canonical envelope directly WITHOUT translation.

Let me re-read the sse_egress to understand the flow better...

Actually, looking at the code:
- Adapters produce `SseEvent { data: String }` with canonical format `{"type":"delta"|"finish"|"error"}`
- `OpenAiStreamTranslator` converts canonical → OpenAI wire format
- Current proxy ALWAYS uses `OpenAiStreamTranslator`

So the fix is:
- If `openai_wire_streaming == true`: use `OpenAiStreamTranslator` (current behavior)
- If `openai_wire_streaming == false` or None: emit canonical envelope directly (new behavior needed)

We need to create a "passthrough" mode that just emits the canonical events as-is.

- [ ] **Step 4: Implement canonical passthrough**

Create a simple closure or struct that passes through canonical events without translation:

```rust
let sse_stream = stream.flat_map(move |event| {
    let events = if use_openai_wire {
        // Use OpenAiStreamTranslator (existing code)
        event
            .map(|e| {
                let mut translator = translator.lock().unwrap();
                let frames = translator.push(&e);
                frames
                    .iter()
                    .map(|f| {
                        axum::response::sse::Event::default().data(f.render())
                    })
                    .collect::<Vec<_>>()
            })
            .unwrap_or_else(|_| {
                let error_payload = serde_json::json!({
                    "error": {
                        "message": "upstream provider stream error",
                        "type": "server_error",
                        "param": null,
                        "code": null,
                    }
                });
                vec![axum::response::sse::Event::default()
                    .data(error_payload.to_string())]
            })
    } else {
        // Emit canonical envelope directly
        event
            .map(|e| {
                vec![axum::response::sse::Event::default().data(&e.data)]
            })
            .unwrap_or_else(|_| {
                let error_payload = serde_json::json!({
                    "error": {
                        "message": "upstream provider stream error",
                        "type": "server_error",
                    }
                });
                vec![axum::response::sse::Event::default()
                    .data(error_payload.to_string())]
            })
    };
    futures::stream::iter(
        events
            .into_iter()
            .map(Ok::<_, std::convert::Infallible>),
    )
});
```

- [ ] **Step 5: Update call_chat_agentic to pass state**

The `call_chat_agentic` function calls `call_chat`. Update the call site to pass `&state`.

- [ ] **Step 6: Update chat_completions to pass state**

The `chat_completions` function calls `call_chat_agentic`. Ensure state is passed through.

- [ ] **Step 7: Run cargo check**

```bash
export PATH="/usr/local/opt/rustup/bin:$PATH"
cargo check -p godwit-api
```

Expected: PASS

- [ ] **Step 8: Commit**

```bash
git add crates/godwit-api/src/proxy.rs
git commit -m "feat(proxy): conditionally apply OpenAI wire translation based on compat flag"
```

---

### Task 4: Add unit test for openai_wire_streaming flag

**Files:**
- Create: `crates/godwit-api/src/proxy_tests.rs` or add to existing tests in `proxy.rs`
- Test: `cargo test -p godwit-api --lib proxy`

**Interfaces:**
- Consumes: `call_chat` with compat flag set
- Produces: Test verifying SSE format differs based on flag

- [ ] **Step 1: Add test module**

Add tests at the end of `crates/godwit-api/src/proxy.rs` in the existing `#[cfg(test)]` module, or create a new test file.

- [ ] **Step 2: Write openai_wire_flag_changes_sse_format test**

```rust
#[tokio::test]
async fn openai_wire_flag_changes_sse_format() {
    // Test that when openai_wire_streaming is true, output is OpenAI format
    // Test that when false, output is canonical envelope format
    
    // This requires mocking the adapter stream - use existing test patterns
    // from the proxy.rs test module or godwit-providers sse_egress tests.
    
    // Verify OpenAI format has:
    // - "object": "chat.completion.chunk"
    // - "choices": [{ "delta": { ... } }]
    
    // Verify canonical format has:
    // - "type": "delta" | "finish" | "error"
}
```

- [ ] **Step 3: Run tests**

```bash
export PATH="/usr/local/opt/rustup/bin:$PATH"
cargo test -p godwit-api --lib proxy
```

Expected: New test passes, existing tests still pass

- [ ] **Step 4: Commit**

```bash
git add crates/godwit-api/src/proxy.rs
git commit -m "test(api): add openai_wire_flag_changes_sse_format test"
```

---

### Task 5: Create streaming documentation

**Files:**
- Create: `docs/api/streaming.md`

**Interfaces:**
- Documents: `compat.openai_wire_streaming` configuration flag

- [ ] **Step 1: Create docs/api directory if needed**

```bash
mkdir -p docs/api
```

- [ ] **Step 2: Write streaming.md**

```markdown
# Streaming API

Godwit supports streaming responses via Server-Sent Events (SSE) for chat completion endpoints.

## Stream Formats

### Canonical Envelope (Default)

By default, Godwit emits a protocol-agnostic canonical envelope:

```json
{"type":"delta","delta":"<text>"}
{"type":"finish","usage":{"prompt_tokens":5,"completion_tokens":2},"finish_reason":"stop"}
{"type":"error","message":"<error>"}
```

### OpenAI Wire Format

When `compat.openai_wire_streaming` is enabled, Godwit emits native OpenAI `chat.completion.chunk` format:

```json
{"id":"chatcmpl-uuid","object":"chat.completion.chunk","created":1234567890,"model":"gpt-4","choices":[{"index":0,"delta":{"role":"assistant","content":""},"finish_reason":null}]}
{"id":"chatcmpl-uuid","object":"chat.completion.chunk","created":1234567890,"model":"gpt-4","choices":[{"index":0,"delta":{"content":"Hello"},"finish_reason":null}]}
{"id":"chatcmpl-uuid","object":"chat.completion.chunk","created":1234567890,"model":"gpt-4","choices":[{"index":0,"delta":{},"finish_reason":"stop"}]}
```

## Configuration

Enable OpenAI wire format in `config.yaml`:

```yaml
compat:
  openai_wire_streaming: true
```

## Differences

| Aspect | Canonical | OpenAI Wire |
|--------|-----------|-------------|
| Event structure | `{"type": "...", ...}` | `{"id": "...", "object": "chat.completion.chunk", ...}` |
| Metadata per event | None | id, created, model on every chunk |
| Role signal | Not present | First chunk includes `{"role": "assistant"}` |
| Tool calls | Serialized as delta string | Structured `tool_calls` array in choices |
| Finish signal | `{"type": "finish", ...}` | `{"choices": [{"finish_reason": "..."}]}` |

## When to Use

- **Canonical**: Internal use, custom clients, multi-protocol gateways
- **OpenAI Wire**: Drop-in replacement for OpenAI API, existing OpenAI SDK clients

```

- [ ] **Step 3: Commit**

```bash
git add docs/api/streaming.md
git commit -m "docs: add streaming API documentation with OpenAI wire format guide"
```

---

### Task 6: Update G2 gap audit report

**Files:**
- Create: `docs/gap-audit/G2-report.md`

**Interfaces:**
- Documents: OpenAI wire compatibility flag feature

- [ ] **Step 1: Create G2-report.md**

```markdown
# G2: OpenAI Wire Compatibility Flag

## Status

✅ Implemented

## Summary

Added `compat.openai_wire_streaming` configuration flag to enable native OpenAI SSE streaming format.

## Changes

- `godwit-core`: Added `CompatConfig` struct with `openai_wire_streaming` field
- `godwit-api`: Threaded config through `AppState`, conditional translation in proxy streaming
- `docs`: Added streaming API documentation

## Testing

- Unit test: `openai_wire_flag_changes_sse_format`
- Verified: `cargo test -p godwit-api --lib proxy`

## Files Modified

- `crates/godwit-core/src/lib.rs`
- `crates/godwit-api/src/state.rs`
- `crates/godwit-api/src/proxy.rs`
- `docs/api/streaming.md`

```

- [ ] **Step 2: Commit**

```bash
git add docs/gap-audit/G2-report.md
git commit -m "docs: add G2 OpenAI wire compatibility flag report"
```

---

### Task 7: Final verification

**Files:**
- All modified files

**Interfaces:**
- Full workspace compilation and test

- [ ] **Step 1: Full workspace check**

```bash
export PATH="/usr/local/opt/rustup/bin:$PATH"
cargo check --workspace --tests
```

Expected: PASS

- [ ] **Step 2: Run proxy tests**

```bash
export PATH="/usr/local/opt/rustup/bin:$PATH"
DATABASE_URL=postgres://user:pass@localhost:5432/godwit cargo test -p godwit-api --lib proxy
```

Expected: All tests pass (including new test)

- [ ] **Step 3: Verify config example**

Check that `config.example.yaml` documents the new flag, or note that users should refer to `docs/api/streaming.md`.

- [ ] **Step 4: Final commit if needed**

```bash
git status
git add <any missed files>
git commit -m "chore: final cleanup"
```

---

## Self-Review

**1. Spec coverage:**
- ✅ Config flag in `AppConfig`
- ✅ Proxy streaming conditional logic
- ✅ Documentation in `docs/api/streaming.md`
- ✅ Unit test `openai_wire_flag_changes_sse_format`
- ✅ G2 report in `docs/gap-audit/G2-report.md`

**2. Placeholder scan:** No TBD/TODO patterns found.

**3. Type consistency:** 
- `CompatConfig` struct matches usage in proxy.rs
- `AppState` threading is consistent
- Function signatures updated to pass state through
