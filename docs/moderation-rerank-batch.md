# Moderation, Rerank, and Batch API Guide

This guide covers the P2-B features implemented in Godwit: moderation fallback chain, rerank fallback chain, and batch API processing.

## Table of Contents

- [Moderation API](#moderation-api)
- [Rerank API](#rerank-api)
- [Batch API](#batch-api)
- [Configuration Reference](#configuration-reference)
- [Troubleshooting](#troubleshooting)

---

## Moderation API

The moderation endpoint provides OpenAI-compatible content moderation with automatic fallback across multiple providers.

### Endpoint

```
POST /v1/moderations
```

### Request Format

```json
{
  "model": "text-moderation-latest",
  "input": "Content to moderate"
}
```

### Response Format

```json
{
  "id": "modr-123",
  "model": "text-moderation-latest",
  "results": [
    {
      "flagged": false,
      "categories": {
        "hate": false,
        "hate/threatening": false,
        "self-harm": false,
        "sexual": false,
        "sexual/minors": false,
        "violence": false,
        "violence/graphic": false
      },
      "category_scores": {
        "hate": 0.0001,
        "hate/threatening": 0.00001,
        "self-harm": 0.00001,
        "sexual": 0.0002,
        "sexual/minors": 0.00001,
        "violence": 0.0003,
        "violence/graphic": 0.00001
      }
    }
  ]
}
```

### Fallback Chain

Moderation uses a configurable fallback chain. Providers are tried in order:

1. **OpenAI** (primary) - `https://api.openai.com/v1`
2. **Azure OpenAI** (secondary) - `https://your-resource.openai.azure.com/openai`
3. **Self-hosted** (tertiary) - `http://localhost:8000/v1`

If a provider fails (503, timeout, or error), the next provider in the chain is automatically tried.

### Example: cURL

```bash
curl -X POST http://localhost:3000/v1/moderations \
  -H "Authorization: Bearer sk-godwit-test" \
  -H "Content-Type: application/json" \
  -d '{
    "model": "text-moderation-latest",
    "input": "This is a test input for moderation"
  }'
```

### Example: Python

```python
import requests

response = requests.post(
    "http://localhost:3000/v1/moderations",
    headers={
        "Authorization": "Bearer sk-godwit-test",
        "Content-Type": "application/json"
    },
    json={
        "model": "text-moderation-latest",
        "input": "This is a test input for moderation"
    }
)

print(response.json())
```

---

## Rerank API

The rerank endpoint provides document re-ranking with fallback across multiple providers.

### Endpoint

```
POST /v1/rerank
```

### Request Format

```json
{
  "model": "rerank-english-v3.0",
  "query": "What is the capital of France?",
  "documents": [
    "Paris is the capital of France",
    "London is the capital of the UK",
    "Berlin is the capital of Germany"
  ]
}
```

### Response Format

```json
{
  "id": "rerank-123",
  "model": "rerank-english-v3.0",
  "results": [
    {
      "index": 0,
      "relevance_score": 0.98
    },
    {
      "index": 2,
      "relevance_score": 0.85
    },
    {
      "index": 1,
      "relevance_score": 0.72
    }
  ]
}
```

### Fallback Chain

Rerank uses a configurable fallback chain. Providers are tried in order:

1. **Cohere** (primary) - `https://api.cohere.ai/v1`
2. **Azure OpenAI** (secondary) - `https://your-resource.openai.azure.com/openai`
3. **Self-hosted** (tertiary) - `http://localhost:8000/v1`

### Example: cURL

```bash
curl -X POST http://localhost:3000/v1/rerank \
  -H "Authorization: Bearer sk-godwit-test" \
  -H "Content-Type: application/json" \
  -d '{
    "model": "rerank-english-v3.0",
    "query": "machine learning",
    "documents": [
      "Deep learning is a subset of machine learning",
      "Python is a programming language",
      "Machine learning algorithms learn from data"
    ]
  }'
```

### Example: Python

```python
import requests

response = requests.post(
    "http://localhost:3000/v1/rerank",
    headers={
        "Authorization": "Bearer sk-godwit-test",
        "Content-Type": "application/json"
    },
    json={
        "model": "rerank-english-v3.0",
        "query": "machine learning",
        "documents": [
            "Deep learning is a subset of machine learning",
            "Python is a programming language",
            "Machine learning algorithms learn from data"
        ]
    }
)

print(response.json())
```

---

## Batch API

The batch API allows submitting multiple requests for asynchronous processing. Godwit supports both native batch processing (OpenAI, Azure) and simulated batch processing (other providers).

### Endpoints

| Method | Endpoint | Description |
|--------|----------|-------------|
| POST | `/v1/batches` | Create a new batch |
| GET | `/v1/batches` | List batches |
| GET | `/v1/batches/:id` | Retrieve a batch |
| POST | `/v1/batches/:id/cancel` | Cancel a batch |

### Create Batch Request

```json
{
  "model": "gpt-4o",
  "input_file_id": "file-abc123",
  "endpoint": "/v1/chat/completions",
  "completion_window": "24h"
}
```

### Batch Response

```json
{
  "id": "batch_123",
  "object": "batch",
  "endpoint": "/v1/chat/completions",
  "errors": null,
  "input_file_id": "file-abc123",
  "completion_window": "24h",
  "status": "validating",
  "output_file_id": null,
  "error_file_id": null,
  "created_at": 1234567890,
  "in_progress_at": null,
  "expires_at": null,
  "finalizing_at": null,
  "completed_at": null,
  "failed_at": null,
  "expired_at": null,
  "cancelling_at": null,
  "cancelled_at": null,
  "request_counts": {
    "total": 0,
    "completed": 0,
    "failed": 0
  },
  "metadata": {}
}
```

### Batch Status Values

- `validating` - Batch is being validated
- `in_progress` - Batch is being processed
- `finalizing` - Batch is being finalized
- `completed` - Batch completed successfully
- `failed` - Batch failed
- `expired` - Batch expired
- `cancelling` - Batch is being cancelled
- `cancelled` - Batch was cancelled

### Native vs Simulated Batch Processing

**Native Batch (OpenAI, Azure):**
- Requests are forwarded directly to the provider's batch API
- Provider handles queuing, execution, and result storage
- Webhook notifications supported

**Simulated Batch (Anthropic, Gemini, vLLM, etc.):**
- Godwit processes requests sequentially with concurrency limits
- Retry logic with exponential backoff (max 2 retries)
- Cost tracking using Decimal precision
- Results aggregated and returned in OpenAI-compatible format

### Example: Create Batch (cURL)

```bash
curl -X POST http://localhost:3000/v1/batches \
  -H "Authorization: Bearer sk-godwit-test" \
  -H "Content-Type: application/json" \
  -d '{
    "model": "gpt-4o",
    "input_file_id": "file-abc123",
    "endpoint": "/v1/chat/completions",
    "completion_window": "24h"
  }'
```

### Example: Retrieve Batch (cURL)

```bash
curl -X GET "http://localhost:3000/v1/batches/batch_123?model=gpt-4o" \
  -H "Authorization: Bearer sk-godwit-test"
```

### Example: Cancel Batch (cURL)

```bash
curl -X POST "http://localhost:3000/v1/batches/batch_123/cancel?model=gpt-4o" \
  -H "Authorization: Bearer sk-godwit-test"
```

### Example: List Batches (cURL)

```bash
curl -X GET "http://localhost:3000/v1/batches?model=gpt-4o" \
  -H "Authorization: Bearer sk-godwit-test"
```

### Batch Processor Configuration

The batch processor has the following default settings:

- **Max concurrent requests:** 10
- **Max retries per request:** 2
- **Retry backoff:** Exponential (1s, 2s, 4s, ...)
- **Cost tracking:** Decimal precision (no floating point)

### Example: Python Batch Workflow

```python
import requests
import time

BASE_URL = "http://localhost:3000"
HEADERS = {"Authorization": "Bearer sk-godwit-test"}

# Create batch
create_resp = requests.post(
    f"{BASE_URL}/v1/batches",
    headers=HEADERS,
    json={
        "model": "gpt-4o",
        "input_file_id": "file-abc123",
        "endpoint": "/v1/chat/completions",
        "completion_window": "24h"
    }
)
batch = create_resp.json()
batch_id = batch["id"]

# Poll until complete
while True:
    status_resp = requests.get(
        f"{BASE_URL}/v1/batches/{batch_id}",
        headers=HEADERS,
        params={"model": "gpt-4o"}
    )
    batch_status = status_resp.json()
    
    if batch_status["status"] in ["completed", "failed", "cancelled"]:
        break
    
    time.sleep(5)

print(f"Batch {batch_id} finished with status: {batch_status['status']}")
```

---

## Configuration Reference

### YAML Configuration

Add the following sections to your `config.yaml`:

```yaml
moderation:
  provider_order:
    - openai
    - azure
    - self-hosted
  timeout_per_provider_secs: 10

rerank:
  provider_order:
    - cohere
    - azure
    - self-hosted
  timeout_per_provider_secs: 15

batch:
  max_concurrent_requests: 10
  max_retries: 2
  retry_base_delay_ms: 1000
```

### Provider Configuration

Configure providers in the `providers` section:

```yaml
providers:
  - id: openai
    type: openai
    base_url: https://api.openai.com/v1
    api_key: ${OPENAI_API_KEY}
    models:
      - text-moderation-latest
      - gpt-4o
      
  - id: azure
    type: azure
    base_url: https://your-resource.openai.azure.com/openai
    api_key: ${AZURE_API_KEY}
    api_version: "2024-02-15-preview"
    models:
      - text-moderation-latest
      - gpt-4o
      
  - id: cohere
    type: cohere
    base_url: https://api.cohere.ai/v1
    api_key: ${COHERE_API_KEY}
    models:
      - rerank-english-v3.0
      - rerank-multilingual-v3.0
      
  - id: self-hosted
    type: vllm
    base_url: http://localhost:8000/v1
    models:
      - moderation-model
      - rerank-model
```

### Environment Variables

```bash
# Moderation
MODERATION_PROVIDERS='[{"name":"openai","base_url":"https://api.openai.com/v1","api_key":"sk-xxx","model":"text-moderation-latest"}]'
MODERATION_TIMEOUT_PER_PROVIDER_SECS=10

# Rerank
RERANK_PROVIDERS='[{"name":"cohere","base_url":"https://api.cohere.ai/v1","api_key":"key-xxx","model":"rerank-english-v3.0"}]'
RERANK_TIMEOUT_PER_PROVIDER_SECS=15

# Batch
BATCH_MAX_CONCURRENT=10
BATCH_MAX_RETRIES=2
```

---

## Troubleshooting

### Moderation Issues

**Problem:** All moderation providers fail

**Symptoms:**
- 503 Service Unavailable response
- Error message: "All moderation providers failed"

**Solutions:**
1. Check provider API keys are configured correctly
2. Verify network connectivity to provider endpoints
3. Increase timeout per provider if providers are slow
4. Check provider status pages for outages

**Problem:** Moderation timeout

**Symptoms:**
- Request takes too long
- Timeout error after N seconds

**Solutions:**
1. Increase `moderation.timeout_per_provider_secs` in config
2. Check provider latency
3. Consider using a closer provider endpoint

### Rerank Issues

**Problem:** Rerank returns empty results

**Symptoms:**
- Response has `"results": []`

**Solutions:**
1. Verify documents array is not empty
2. Check query is not empty
3. Ensure model supports the document language

**Problem:** Rerank fallback not triggering

**Symptoms:**
- Primary provider fails but fallback doesn't activate

**Solutions:**
1. Check provider order in config
2. Verify fallback providers are configured
3. Check logs for fallback chain execution

### Batch Issues

**Problem:** Batch stuck in "validating" status

**Symptoms:**
- Batch status remains "validating" for extended period

**Solutions:**
1. Check input file exists and is accessible
2. Verify file format is valid JSONL
3. Check batch processor logs for errors

**Problem:** Batch requests failing after retries

**Symptoms:**
- High failure count in batch status
- Individual requests failing consistently

**Solutions:**
1. Check provider API limits and rate limits
2. Verify request format is correct
3. Reduce batch size if hitting provider limits
4. Check provider logs for specific errors

**Problem:** Batch cost tracking inaccurate

**Symptoms:**
- Costs don't match expected values
- Decimal precision issues

**Solutions:**
1. Verify model pricing is configured correctly
2. Check token counting logic
3. Ensure Decimal type is used (not float)

### General Issues

**Problem:** 401 Unauthorized

**Solutions:**
1. Verify API key is correct
2. Check API key has not expired
3. Ensure Authorization header format is `Bearer <key>`

**Problem:** 404 Not Found

**Solutions:**
1. Verify endpoint path is correct
2. Check model is configured in provider
3. Ensure provider is reachable

**Problem:** 500 Internal Server Error

**Solutions:**
1. Check Godwit logs for stack trace
2. Verify database connection
3. Check provider configuration

### Logs and Debugging

Enable debug logging:

```bash
RUST_LOG=debug cargo run --bin godwit
```

Key log messages to watch for:

- `Attempting moderation with provider X (Y/Z)` - Fallback chain execution
- `Attempting rerank with provider X (Y/Z)` - Fallback chain execution
- `Batch processing started` - Batch processor initialization
- `Batch item X succeeded/failed` - Individual batch item results

### Integration Tests

Run integration tests (requires running server and database):

```bash
# Start server
DATABASE_URL=postgres://user:pass@localhost:5432/godwit cargo run --bin godwit

# In another terminal, run tests
cargo test --test moderation_integration -- --ignored
cargo test --test rerank_integration -- --ignored
cargo test --test batch_integration -- --ignored
```

---

## API Compatibility

### OpenAI Parity

Godwit aims for OpenAI API compatibility:

| Endpoint | OpenAI | Godwit | Notes |
|----------|--------|--------|-------|
| `/v1/moderations` | ✅ | ✅ | Fallback chain added |
| `/v1/rerank` | ❌ | ✅ | Cohere-compatible format |
| `/v1/batches` | ✅ | ✅ | Simulated for non-OpenAI providers |

### Response Normalization

All providers return responses in a normalized format:

- **Moderation:** OpenAI format (id, model, results)
- **Rerank:** Cohere format (id, model, results with index and relevance_score)
- **Batch:** OpenAI format (id, object, status, request_counts, etc.)

---

## Performance Considerations

### Moderation

- Timeout per provider: 10 seconds (configurable)
- Fallback adds latency only if primary fails
- Response size: ~500 bytes for single input

### Rerank

- Timeout per provider: 15 seconds (configurable)
- Latency scales with document count
- Response size: ~100 bytes per document

### Batch

- Max concurrent requests: 10 (configurable)
- Retry delay: Exponential backoff (1s, 2s, 4s)
- Cost tracking: Decimal precision (no float errors)

---

## Security Considerations

1. **API Keys:** Store provider API keys in environment variables or secrets manager
2. **Rate Limiting:** Configure rate limits per API key to prevent abuse
3. **Input Validation:** All inputs are validated before forwarding to providers
4. **Logging:** Sensitive data (API keys, tokens) is not logged

---

## Support

For issues or questions:

1. Check the [troubleshooting section](#troubleshooting)
2. Review Godwit logs with `RUST_LOG=debug`
3. Open an issue on GitHub
