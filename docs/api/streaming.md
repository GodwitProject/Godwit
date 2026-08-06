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
