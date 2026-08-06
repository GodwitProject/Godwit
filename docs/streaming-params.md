# Streaming & Advanced Parameters Guide

This document covers streaming responses and advanced parameters support in Godwit for P2-C (Provider Protocol Compatibility).

## Table of Contents

- [Streaming](#streaming)
- [Advanced Parameters](#advanced-parameters)
- [Prompt Caching](#prompt-caching)
- [Configuration Reference](#configuration-reference)
- [Troubleshooting](#troubleshooting)

---

## Streaming

Godwit supports streaming responses across all providers with normalized SSE format.

### Basic Streaming Request

```bash
curl -X POST http://localhost:3000/v1/chat/completions \
  -H "Authorization: Bearer sk-godwit-test" \
  -H "Content-Type: application/json" \
  -d '{
    "model": "gpt-4o",
    "messages": [{"role": "user", "content": "Say hello"}],
    "stream": true
  }'
```

### Streaming Response Format

Godwit normalizes streaming responses to OpenAI-compatible format:

```
data: {"id":"chatcmpl-123","object":"chat.completion.chunk","created":1234567890,"model":"gpt-4o","choices":[{"index":0,"delta":{"role":"assistant","content":"Hello"},"finish_reason":null}]}

data: {"id":"chatcmpl-123","object":"chat.completion.chunk","created":1234567890,"model":"gpt-4o","choices":[{"index":0,"delta":{"content":" there"},"finish_reason":null}]}

data: [DONE]
```

### Provider-Specific Streaming

#### Gemini

```bash
curl -X POST http://localhost:3000/v1/chat/completions \
  -H "Authorization: Bearer sk-godwit-test" \
  -H "Content-Type: application/json" \
  -d '{
    "model": "gemini-pro",
    "messages": [{"role": "user", "content": "Explain quantum computing"}],
    "stream": true,
    "temperature": 0.7
  }'
```

#### Anthropic (Claude)

```bash
curl -X POST http://localhost:3000/v1/chat/completions \
  -H "Authorization: Bearer sk-godwit-test" \
  -H "Content-Type: application/json" \
  -d '{
    "model": "claude-3-5-sonnet-20241022",
    "messages": [{"role": "user", "content": "Write a poem"}],
    "stream": true
  }'
```

#### Self-hosted (vLLM, Ollama, llama.cpp, SGLang)

```bash
# vLLM
curl -X POST http://localhost:3000/v1/chat/completions \
  -H "Authorization: Bearer sk-godwit-test" \
  -d '{
    "model": "vllm-mistral-7b",
    "messages": [{"role": "user", "content": "Hello"}],
    "stream": true
  }'

# Ollama
curl -X POST http://localhost:3000/v1/chat/completions \
  -H "Authorization: Bearer sk-godwit-test" \
  -d '{
    "model": "ollama-llama3",
    "messages": [{"role": "user", "content": "Hello"}],
    "stream": true
  }'
```

---

## Advanced Parameters

Godwit supports 9 advanced parameters with provider-specific translation.

### Parameter Reference

| Parameter | Type | Description | Providers |
|-----------|------|-------------|-----------|
| `temperature` | float | Sampling temperature (0-2) | All |
| `max_tokens` | int | Maximum completion tokens | All |
| `top_p` | float | Nucleus sampling (0-1) | All |
| `top_k` | int | Top-k sampling | Gemini, vLLM, SGLang, Ollama |
| `frequency_penalty` | float | Frequency penalty (-2 to 2) | OpenAI, Azure, Anthropic |
| `presence_penalty` | float | Presence penalty (-2 to 2) | OpenAI, Azure, Anthropic |
| `repetition_penalty` | float | Repetition penalty (0-2) | Gemini, vLLM, SGLang |
| `stop` | string\|array | Stop sequences (max 4) | All |
| `seed` | int | Random seed for reproducibility | OpenAI, Azure, vLLM |
| `n` | int | Number of completions (1-10) | OpenAI, Azure |
| `logprobs` | bool | Return log probabilities | OpenAI, Azure |
| `top_logprobs` | int | Number of top logprobs | OpenAI, Azure |
| `logit_bias` | object | Token bias map | OpenAI, Azure |
| `user` | string | End-user identifier | All (tracking) |
| `parallel_tool_calls` | bool | Parallel tool execution | OpenAI, Anthropic |
| `response_format` | object | JSON schema enforcement | OpenAI, Anthropic |
| `reasoning.effort` | string | Reasoning effort level | OpenAI o1 |
| `reasoning.thinking` | object | Thinking budget | Anthropic |

### Examples

#### Temperature, Top-P, Top-K

```bash
curl -X POST http://localhost:3000/v1/chat/completions \
  -H "Authorization: Bearer sk-godwit-test" \
  -H "Content-Type: application/json" \
  -d '{
    "model": "gpt-4o",
    "messages": [{"role": "user", "content": "Write a creative story"}],
    "temperature": 0.9,
    "top_p": 0.95,
    "top_k": 50,
    "max_tokens": 200
  }'
```

#### Frequency & Presence Penalties

```bash
curl -X POST http://localhost:3000/v1/chat/completions \
  -H "Authorization: Bearer sk-godwit-test" \
  -H "Content-Type: application/json" \
  -d '{
    "model": "claude-3-5-sonnet-20241022",
    "messages": [{"role": "user", "content": "Generate diverse ideas"}],
    "frequency_penalty": 0.5,
    "presence_penalty": 0.3,
    "max_tokens": 150
  }'
```

#### Repetition Penalty

```bash
curl -X POST http://localhost:3000/v1/chat/completions \
  -H "Authorization: Bearer sk-godwit-test" \
  -H "Content-Type: application/json" \
  -d '{
    "model": "gemini-pro",
    "messages": [{"role": "user", "content": "Explain without repeating"}],
    "repetition_penalty": 1.2,
    "max_tokens": 100
  }'
```

#### Stop Sequences

```bash
# Single stop sequence
curl -X POST http://localhost:3000/v1/chat/completions \
  -H "Authorization: Bearer sk-godwit-test" \
  -d '{
    "model": "gpt-4o",
    "messages": [{"role": "user", "content": "Count to 10"}],
    "stop": "5",
    "max_tokens": 50
  }'

# Multiple stop sequences (max 4)
curl -X POST http://localhost:3000/v1/chat/completions \
  -H "Authorization: Bearer sk-godwit-test" \
  -d '{
    "model": "gpt-4o",
    "messages": [{"role": "user", "content": "Write a story"}],
    "stop": ["THE END", "STOP", "FIN"],
    "max_tokens": 200
  }'
```

#### Seed for Reproducibility

```bash
curl -X POST http://localhost:3000/v1/chat/completions \
  -H "Authorization: Bearer sk-godwit-test" \
  -d '{
    "model": "gpt-4o",
    "messages": [{"role": "user", "content": "Generate random text"}],
    "seed": 42,
    "temperature": 1.0,
    "max_tokens": 50
  }'
```

#### N Choices

```bash
curl -X POST http://localhost:3000/v1/chat/completions \
  -H "Authorization: Bearer sk-godwit-test" \
  -d '{
    "model": "gpt-4o",
    "messages": [{"role": "user", "content": "Say hello"}],
    "n": 3,
    "max_tokens": 20
  }'
```

#### Logprobs

```bash
curl -X POST http://localhost:3000/v1/chat/completions \
  -H "Authorization: Bearer sk-godwit-test" \
  -d '{
    "model": "gpt-4o",
    "messages": [{"role": "user", "content": "Hello"}],
    "logprobs": true,
    "top_logprobs": 5,
    "max_tokens": 20
  }'
```

#### Logit Bias

```bash
curl -X POST http://localhost:3000/v1/chat/completions \
  -H "Authorization: Bearer sk-godwit-test" \
  -d '{
    "model": "gpt-4o",
    "messages": [{"role": "user", "content": "Talk about pets"}],
    "logit_bias": {"1234": 5, "5678": -5},
    "max_tokens": 50
  }'
```

#### User Field (Tracking)

```bash
curl -X POST http://localhost:3000/v1/chat/completions \
  -H "Authorization: Bearer sk-godwit-test" \
  -d '{
    "model": "gpt-4o",
    "messages": [{"role": "user", "content": "Hello"}],
    "user": "end-user-123",
    "max_tokens": 50
  }'
```

#### Parallel Tool Calls

```bash
curl -X POST http://localhost:3000/v1/chat/completions \
  -H "Authorization: Bearer sk-godwit-test" \
  -d '{
    "model": "gpt-4o",
    "messages": [{"role": "user", "content": "Get weather and time"}],
    "tools": [...],
    "parallel_tool_calls": false,
    "max_tokens": 100
  }'
```

#### JSON Schema Response Format

```bash
curl -X POST http://localhost:3000/v1/chat/completions \
  -H "Authorization: Bearer sk-godwit-test" \
  -d '{
    "model": "gpt-4o",
    "messages": [{"role": "user", "content": "Generate a person"}],
    "response_format": {
      "type": "json_schema",
      "json_schema": {
        "name": "Person",
        "schema": {
          "type": "object",
          "properties": {
            "name": {"type": "string"},
            "age": {"type": "integer"}
          },
          "required": ["name", "age"]
        },
        "strict": true
      }
    },
    "max_tokens": 100
  }'
```

#### Reasoning Effort (OpenAI o1)

```bash
curl -X POST http://localhost:3000/v1/chat/completions \
  -H "Authorization: Bearer sk-godwit-test" \
  -d '{
    "model": "o1-preview",
    "messages": [{"role": "user", "content": "Solve this math problem"}],
    "reasoning": {
      "effort": "high"
    },
    "max_tokens": 1000
  }'
```

#### Thinking Budget (Anthropic)

```bash
curl -X POST http://localhost:3000/v1/chat/completions \
  -H "Authorization: Bearer sk-godwit-test" \
  -d '{
    "model": "claude-3-5-sonnet-20241022",
    "messages": [{"role": "user", "content": "Think carefully"}],
    "reasoning": {
      "thinking": {
        "type": "enabled",
        "budget_tokens": 1000
      }
    },
    "max_tokens": 500
  }'
```

---

## Prompt Caching

Godwit supports prompt caching for Anthropic, OpenAI, and Gemini providers.

### Enabling Cache Control

Add `cache_control` to messages:

```bash
curl -X POST http://localhost:3000/v1/chat/completions \
  -H "Authorization: Bearer sk-godwit-test" \
  -d '{
    "model": "claude-3-5-sonnet-20241022",
    "messages": [
      {
        "role": "system",
        "content": "You are a helpful assistant with context.",
        "cache_control": {"type": "ephemeral"}
      },
      {
        "role": "user",
        "content": "Hello"
      }
    ],
    "max_tokens": 100
  }'
```

### Cache Configuration

In `config.yaml`:

```yaml
cache:
  enabled: true
  ttl_secs: 3600
  max_size: 10000
```

---

## Configuration Reference

### Server Configuration

```yaml
server:
  host: "0.0.0.0"
  port: 3000
  request_timeout_seconds: 60
```

### Cache Configuration

```yaml
cache:
  enabled: true           # Enable/disable caching
  ttl_secs: 3600          # Cache entry TTL in seconds
  max_size: 10000         # Maximum cache entries
```

### Provider Configuration

```yaml
providers:
  - id: openai
    protocol: openai
    api_key: ${OPENAI_API_KEY}
    base_url: https://api.openai.com/v1
    
  - id: anthropic
    protocol: anthropic
    api_key: ${ANTHROPIC_API_KEY}
    base_url: https://api.anthropic.com/v1
    
  - id: gemini
    protocol: gemini
    api_key: ${GEMINI_API_KEY}
    base_url: https://generativelanguage.googleapis.com/v1beta
    
  - id: vllm
    protocol: vllm
    api_key: unused
    base_url: http://localhost:8000/v1
    
  - id: ollama
    protocol: ollama
    api_key: unused
    base_url: http://localhost:11434/v1
```

### Model Configuration

```yaml
models:
  - id: gpt-4o
    provider: openai
    upstream_name: gpt-4o
    capabilities: [chat]
    
  - id: claude-3-5-sonnet-20241022
    provider: anthropic
    upstream_name: claude-3-5-sonnet-20241022
    capabilities: [chat]
    
  - id: gemini-pro
    provider: gemini
    upstream_name: gemini-1.5-pro
    capabilities: [chat]
```

---

## Troubleshooting

### Streaming Issues

**Problem:** No streaming events received

**Solution:**
1. Verify `stream: true` in request
2. Check provider supports streaming
3. Ensure SSE parser is working (look for `data: ` prefix)

**Problem:** Streaming cuts off mid-response

**Solution:**
1. Increase `request_timeout_seconds` in config
2. Check network stability
3. Verify provider rate limits

### Parameter Issues

**Problem:** Parameter ignored or not applied

**Solution:**
1. Check provider supports the parameter (see table above)
2. Verify parameter is in valid range
3. Check logs for parameter translation warnings

**Problem:** Stop sequences not working

**Solution:**
1. Ensure max 4 stop sequences
2. Verify stop sequences are not empty
3. Check provider-specific limitations

### Caching Issues

**Problem:** Cache not being used

**Solution:**
1. Verify `cache.enabled: true` in config
2. Check `cache_control` field on messages
3. Verify cache TTL hasn't expired
4. Check cache isn't full (`max_size`)

**Problem:** Cache hit but stale content

**Solution:**
1. Reduce `cache.ttl_secs`
2. Implement cache invalidation logic
3. Use shorter TTL for dynamic content

### Integration Test Failures

**Problem:** Tests fail with connection refused

**Solution:**
```bash
# Start the server
cargo run --bin godwit

# Run tests (they are marked #[ignore] by default)
cargo test --test streaming_integration -- --ignored
cargo test --test cache_integration -- --ignored
cargo test --test params_integration -- --ignored
```

**Problem:** Tests fail with 401 Unauthorized

**Solution:**
1. Verify API keys in config
2. Check `Authorization` header format
3. Ensure test token is valid

### Provider-Specific Issues

**Gemini:**
- `top_k` only available on Gemini 1.5+
- Streaming uses different chunk format (normalized by Godwit)

**Anthropic:**
- `cache_control` requires Anthropic-specific message format
- System messages handled differently (normalized by Godwit)

**OpenAI:**
- `seed` parameter requires compatible model
- `logprobs` may increase latency

**Self-hosted (vLLM, Ollama, etc.):**
- Parameter support varies by backend
- Check upstream documentation for limitations

---

## Running Integration Tests

Integration tests are marked with `#[ignore]` and require a running server:

```bash
# Compile tests
cargo test --test streaming_integration --no-run
cargo test --test cache_integration --no-run
cargo test --test params_integration --no-run

# Run with server (requires DATABASE_URL)
DATABASE_URL=postgres://user:pass@localhost:5432/godwit \
  cargo test --test streaming_integration -- --ignored

# Run specific test
DATABASE_URL=postgres://user:pass@localhost:5432/godwit \
  cargo test --test params_integration params_temperature_top_p_top_k -- --ignored
```

---

## Cost Tracking

All cost calculations use `Decimal` for precision. Costs are tracked in:

- `godwit-db` schema (migrations on startup)
- Usage tracking integration tests

Example cost calculation:

```rust
use rust_decimal::Decimal;

let prompt_cost = Decimal::from(prompt_tokens) * price_per_prompt_token;
let completion_cost = Decimal::from(completion_tokens) * price_per_completion_token;
let total_cost = prompt_cost + completion_cost;
```

---

## Additional Resources

- [API Reference](./api/)
- [Configuration Guide](../config.example.yaml)
- [Provider Setup](./providers.md)
