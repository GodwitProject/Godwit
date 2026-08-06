# Tool Calling, Multimodal & JSON Schema Guide

This guide covers the P2-A features implemented in Godwit: tool calling with MCP and web search, multimodal requests with images, and JSON Schema constrained generation.

## Table of Contents

- [Tool Calling](#tool-calling)
  - [MCP Tool Resolution](#mcp-tool-resolution)
  - [Web Search via SearXNG](#web-search-via-searxng)
  - [Agentic Loop](#agentic-loop)
  - [Parallel Tool Calls](#parallel-tool-calls)
- [Multimodal Requests](#multimodal-requests)
  - [Image URLs](#image-urls)
  - [Base64 Images](#base64-images)
  - [Backward Compatibility](#backward-compatibility)
  - [Provider Translation](#provider-translation)
- [JSON Schema](#json-schema)
  - [Schema Validation](#schema-validation)
  - [Guided Decoding](#guided-decoding)
  - [Strict Mode](#strict-mode)

---

## Tool Calling

Godwit supports tool calling with two resolution backends:

1. **MCP (Model Context Protocol)** - Local tool servers via subprocess
2. **Web Search** - SearXNG integration for web queries

### MCP Tool Resolution

Configure MCP servers in `config.yaml`:

```yaml
agentic:
  max_iterations: 4
  mcp_servers:
    - name: filesystem
      command: npx
      args:
        - "-y"
        - "@modelcontextprotocol/server-filesystem"
        - "/tmp"
      env: {}
  searxng:
    base_url: "http://localhost:8080"
```

Example tool call request:

```bash
curl -X POST http://localhost:3000/v1/chat/completions \
  -H "Authorization: Bearer sk-godwit-test" \
  -H "Content-Type: application/json" \
  -d '{
    "model": "claude-sonnet-4-20250514",
    "messages": [
      {
        "role": "user",
        "content": "Read the file at /tmp/test.txt"
      }
    ],
    "tools": [
      {
        "type": "function",
        "function": {
          "name": "mcp_file_read",
          "description": "Read a file from the filesystem",
          "parameters": {
            "type": "object",
            "properties": {
              "path": {"type": "string"}
            },
            "required": ["path"]
          }
        }
      }
    ],
    "tool_choice": "auto"
  }'
```

### Web Search via SearXNG

When a provider doesn't have native web search, Godwit routes to SearXNG:

```bash
curl -X POST http://localhost:3000/v1/chat/completions \
  -H "Authorization: Bearer sk-godwit-test" \
  -H "Content-Type: application/json" \
  -d '{
    "model": "gpt-4o",
    "messages": [
      {
        "role": "user",
        "content": "What is the weather today?"
      }
    ],
    "tools": [
      {
        "type": "function",
        "function": {
          "name": "web_search",
          "description": "Search the web for current information",
          "parameters": {
            "type": "object",
            "properties": {
              "query": {"type": "string"}
            },
            "required": ["query"]
          }
        }
      }
    ],
    "tool_choice": "auto"
  }'
```

### Agentic Loop

The agentic loop automatically iterates tool calls until completion:

- **Max iterations**: Configurable via `agentic.max_iterations` (default: 4)
- **Tool resolution**: MCP servers first, then web search fallback
- **Logging**: Each iteration is logged for debugging

```yaml
agentic:
  max_iterations: 8  # Override default
```

### Parallel Tool Calls

Enable parallel tool execution:

```bash
curl -X POST http://localhost:3000/v1/chat/completions \
  -H "Authorization: Bearer sk-godwit-test" \
  -H "Content-Type: application/json" \
  -d '{
    "model": "gpt-4o",
    "messages": [
      {
        "role": "user",
        "content": "What is the weather and time in Paris?"
      }
    ],
    "tools": [
      {
        "type": "function",
        "function": {
          "name": "get_weather",
          "description": "Get weather for a city",
          "parameters": {
            "type": "object",
            "properties": {
              "city": {"type": "string"}
            },
            "required": ["city"]
          }
        }
      },
      {
        "type": "function",
        "function": {
          "name": "get_time",
          "description": "Get current time for a timezone",
          "parameters": {
            "type": "object",
            "properties": {
              "timezone": {"type": "string"}
            },
            "required": ["timezone"]
          }
        }
      }
    ],
    "parallel_tool_calls": true
  }'
```

---

## Multimodal Requests

Godwit supports multimodal requests with images, with automatic provider translation.

### Image URLs

Reference images by URL:

```bash
curl -X POST http://localhost:3000/v1/chat/completions \
  -H "Authorization: Bearer sk-godwit-test" \
  -H "Content-Type: application/json" \
  -d '{
    "model": "gpt-4o",
    "messages": [
      {
        "role": "user",
        "content": [
          {"type": "text", "text": "What is in this image?"},
          {
            "type": "image_url",
            "image_url": {
              "url": "https://example.com/image.png",
              "detail": "high"
            }
          }
        ]
      }
    ]
  }'
```

### Base64 Images

Embed images as base64 data URLs:

```bash
curl -X POST http://localhost:3000/v1/chat/completions \
  -H "Authorization: Bearer sk-godwit-test" \
  -H "Content-Type: application/json" \
  -d '{
    "model": "claude-sonnet-4-20250514",
    "messages": [
      {
        "role": "user",
        "content": [
          {"type": "text", "text": "Describe this image"},
          {
            "type": "image_url",
            "image_url": {
              "url": "data:image/png;base64,iVBORw0KG..."
            }
          }
        ]
      }
    ]
  }'
```

### Backward Compatibility

String content is still supported for backward compatibility:

```bash
curl -X POST http://localhost:3000/v1/chat/completions \
  -H "Authorization: Bearer sk-godwit-test" \
  -H "Content-Type: application/json" \
  -d '{
    "model": "gpt-4o",
    "messages": [
      {
        "role": "user",
        "content": "Hello, world!"
      }
    ]
  }'
```

### Provider Translation

Godwit automatically translates multimodal requests to the provider's native format:

- **OpenAI** → Native `image_url` format
- **Anthropic** → `image` content blocks with base64 or URL
- **Gemini** → `inline_data` or `file_data` parts
- **vLLM/SGLang** → OpenAI-compatible format

---

## JSON Schema

Godwit supports JSON Schema constrained generation with post-response validation.

### Schema Validation

Enable JSON Schema response format:

```bash
curl -X POST http://localhost:3000/v1/chat/completions \
  -H "Authorization: Bearer sk-godwit-test" \
  -H "Content-Type: application/json" \
  -d '{
    "model": "gpt-4o",
    "messages": [
      {
        "role": "user",
        "content": "Generate a person object"
      }
    ],
    "response_format": {
      "type": "json_schema",
      "json_schema": {
        "name": "Person",
        "schema": {
          "type": "object",
          "properties": {
            "name": {"type": "string"},
            "age": {"type": "integer"},
            "email": {"type": "string", "format": "email"}
          },
          "required": ["name", "age", "email"]
        },
        "strict": true
      }
    }
  }'
```

### Guided Decoding

For vLLM and SGLang backends, JSON Schema enables guided decoding:

```bash
curl -X POST http://localhost:3000/v1/chat/completions \
  -H "Authorization: Bearer sk-godwit-test" \
  -H "Content-Type: application/json" \
  -d '{
    "model": "meta-llama/Llama-3.1-8B-Instruct",
    "messages": [
      {
        "role": "user",
        "content": "List 3 products with their prices"
      }
    ],
    "response_format": {
      "type": "json_schema",
      "json_schema": {
        "name": "ProductList",
        "schema": {
          "type": "array",
          "items": {
            "type": "object",
            "properties": {
              "product": {"type": "string"},
              "price": {"type": "number"}
            },
            "required": ["product", "price"]
          }
        },
        "strict": true
      }
    }
  }'
```

### Strict Mode

When `strict: true`, Godwit validates the response against the schema and rejects non-conforming outputs:

```yaml
response_format:
  type: json_schema
  json_schema:
    name: StrictObject
    schema:
      type: object
      additionalProperties: false
      properties:
        id:
          type: integer
        value:
          type: string
      required:
        - id
        - value
    strict: true  # Enforce validation
```

---

## Integration Tests

Run the integration tests (requires running server and database):

```bash
# Compile tests (no run)
cargo test --test tool_calling_integration --no-run
cargo test --test multimodal_integration --no-run
cargo test --test json_schema_integration --no-run

# Run with server (manually)
DATABASE_URL=postgres://user:pass@localhost:5432/godwit \
  cargo test --test tool_calling_integration -- --ignored

DATABASE_URL=postgres://user:pass@localhost:5432/godwit \
  cargo test --test multimodal_integration -- --ignored

DATABASE_URL=postgres://user:pass@localhost:5432/godwit \
  cargo test --test json_schema_integration -- --ignored
```

---

## Configuration Reference

Complete `config.yaml` example:

```yaml
server:
  host: 127.0.0.1
  port: 3000
  request_timeout_seconds: 60

database:
  url: postgres://user:pass@localhost:5432/godwit

auth:
  jwt_secret: supersecret
  access_token_ttl_minutes: 15
  refresh_token_ttl_days: 7
  oidc_providers: []
  saml_providers: []

agentic:
  max_iterations: 4
  mcp_servers:
    - name: filesystem
      command: npx
      args:
        - "-y"
        - "@modelcontextprotocol/server-filesystem"
        - "/tmp"
      env: {}
  searxng:
    base_url: "http://localhost:8080"

compat:
  openai_wire_streaming: false
```

---

## Error Handling

- **Tool resolution failures**: Logged and returned as tool call errors
- **Multimodal provider errors**: Translated to OpenAI-compatible error format
- **JSON Schema validation**: Returns 400 with validation error details

---

## See Also

- [API Reference](../crates/godwit-api/README.md)
- [Provider Implementation](../crates/godwit-providers/README.md)
- [Core Types](../crates/godwit-core/src/lib.rs)
