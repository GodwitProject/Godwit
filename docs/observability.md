# Observability

Godwit exposes comprehensive observability features including Prometheus metrics and utility endpoints for monitoring and debugging.

## Prometheus Metrics

Godwit exposes Prometheus-compatible metrics at `/metrics`.

### Metrics

#### `godwit_requests_total`

Total number of requests.

**Labels:**
- `model` - The model ID used
- `provider` - The provider name (e.g., `openai`, `anthropic`)
- `status` - Request status (e.g., `success`, `error`)

#### `godwit_request_duration_seconds`

Request latency histogram.

**Labels:**
- `model` - The model ID used
- `provider` - The provider name

**Buckets:** 0.01, 0.05, 0.1, 0.25, 0.5, 1.0, 2.5, 5.0, 10.0

#### `godwit_tokens_total`

Total tokens processed.

**Labels:**
- `type` - Token type (`input`, `output`, `cache`)
- `model` - The model ID

#### `godwit_cost_usd_total`

Total cost in USD.

**Labels:**
- `org` - Organization ID
- `team` - Team ID
- `api_key` - API key identifier

#### `godwit_active_requests`

Currently active requests.

**Labels:**
- `model` - The model ID
- `provider` - The provider name

### Example Prometheus Config

```yaml
scrape_configs:
  - job_name: 'godwit'
    static_configs:
      - targets: ['localhost:3000']
    metrics_path: '/metrics'
```

## Utility Endpoints

### Token Counter

`POST /v1/utils/token_counter`

Count tokens before sending a request.

**Request:**
```json
{
  "model": "gpt-4",
  "messages": [{"role": "user", "content": "Hello"}]
}
```

**Response:**
```json
{
  "prompt_tokens": 8,
  "model": "gpt-4"
}
```

### Model Info

`GET /v1/utils/model_info/:model_id`

Get pricing and capabilities for a model.

**Response:**
```json
{
  "id": "gpt-4",
  "provider": "openai",
  "pricing": {
    "input_cost_per_1k": 0.03,
    "output_cost_per_1k": 0.06
  },
  "capabilities": {
    "supports_tool_calling": true,
    "supports_vision": false,
    "supports_streaming": true,
    "max_tokens": 8192
  }
}
```

### Health

`GET /v1/utils/health`

Extended health check with provider status.

**Response:**
```json
{
  "status": "healthy",
  "version": "1.4.0",
  "uptime_secs": 3600,
  "database": "connected",
  "providers": [
    {"name": "openai", "status": "healthy", "latency_ms": 50},
    {"name": "anthropic", "status": "healthy", "latency_ms": 120}
  ]
}
```
