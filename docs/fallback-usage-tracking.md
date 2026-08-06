# Fallback & Usage Tracking Guide

## Fallback Configuration

Fallback allows Godwit to automatically retry requests with alternative providers when the primary provider fails.

### Configuration Example

```yaml
models:
  - public_id: gpt-4o
    provider_profile_id: uuid-openai
    provider_model_id: gpt-4o
    config:
      fallbacks:
        - anthropic/claude-sonnet-4-20250514
        - gemini/gemini-2.5-pro
      max_fallback_attempts: 3
      timeout_per_attempt_secs: 30
```

### Fallback Behavior

- **Triggered on**: 5xx errors, timeouts, and 429 (rate limit)
- **NOT triggered on**: 4xx client errors (invalid request, auth failure, etc.)
- **Max attempts**: Configurable per model (default: 3)
- **Timeout per attempt**: Configurable (default: 30 seconds)

### Fallback Chain

When a request fails with a retryable error:
1. Primary provider attempted first
2. If fails, first fallback in chain is tried
3. Continues down the chain until success or exhaustion
4. Last error returned if all attempts fail

### Monitoring

Fallback attempts are logged to `request_logs` table:
- `attempt_number`: Which attempt this was (1, 2, 3, ...)
- `fallback_triggered`: Boolean indicating if fallback was used

Query fallback statistics:
```sql
SELECT 
    model_id,
    COUNT(*) FILTER (WHERE fallback_triggered = true) as fallback_count,
    COUNT(*) as total_requests
FROM request_logs
GROUP BY model_id;
```

## Usage Tracking

All providers now report accurate usage information. Usage is extracted from provider responses and normalized to a common format.

### Chat Completions

**Tracked fields:**
- `prompt_tokens`: Input tokens sent to model
- `completion_tokens`: Output tokens generated
- `cache_read_tokens`: Tokens read from cache (if supported)
- `cache_write_tokens`: Tokens written to cache (if supported)

**Supported providers:**
- OpenAI (native)
- Anthropic (native)
- Gemini (native)
- Azure OpenAI (native)
- Ollama (estimated from response)
- llama.cpp (estimated from response)
- vLLM (native)
- SGLang (native)

### Embeddings

**Tracked fields:**
- `prompt_tokens`: Input tokens processed

**Supported providers:**
- OpenAI (native)
- Gemini (native)
- Azure OpenAI (native)
- Ollama (estimated)

### Image Generation

**Tracked fields:**
- `image_count`: Number of images generated

**Cost calculation:** Per-image pricing from model config

**Supported providers:**
- OpenAI DALL-E (native)
- Other providers via estimation

### Audio - Text-to-Speech (TTS)

**Tracked fields:**
- `tts_characters`: Number of characters synthesized

**Cost calculation:** Per-character pricing from model config

**Supported providers:**
- OpenAI (native)
- Other providers via estimation

### Audio - Speech-to-Text (STT)

**Tracked fields:**
- `audio_seconds`: Duration of audio processed (estimated)

**Cost calculation:** Per-second pricing from model config

**Supported providers:**
- OpenAI Whisper (native)
- Other providers via estimation

## Cost Calculation

Cost is calculated as:

```
Cost = (input_tokens * input_price + output_tokens * output_price) / 1_000_000
```

For non-token modalities:
- **Images**: `image_count * price_per_image`
- **TTS**: `characters * price_per_character`
- **STT**: `seconds * price_per_second`

### Pricing Configuration

Pricing is configured per model in the `models.pricing` JSONB column:

```yaml
models:
  - public_id: gpt-4o
    provider_profile_id: uuid-openai
    provider_model_id: gpt-4o
    pricing:
      input_price_usd_per_million: 5.00
      output_price_usd_per_million: 15.00
      cache_read_price_usd_per_million: 1.25
      cache_write_price_usd_per_million: 5.00
```

**Validation:** Pricing is required on model creation. Models without pricing will be rejected.

### Decimal Precision

All cost calculations use `Decimal` type (not floats) to avoid floating-point errors in financial calculations.

## Endpoints

### View Usage

```bash
curl http://localhost:3000/api/v1/spend \
  -H "Authorization: Bearer $ADMIN_KEY"
```

Returns aggregated spend across all requests.

### View Detailed Logs

```bash
curl "http://localhost:3000/api/v1/spend/logs?api_key_id=$KEY_ID" \
  -H "Authorization: Bearer $ADMIN_KEY"
```

Returns detailed request logs with usage and cost information.

### Filter by Tags

```bash
curl "http://localhost:3000/api/v1/spend/tags?tag=mytag" \
  -H "Authorization: Bearer $ADMIN_KEY"
```

Returns spend aggregated by custom tags.

## Error Classification

### Retryable Errors (trigger fallback)

- **5xx**: Server errors (provider down, internal error)
- **Timeout**: Request exceeded timeout_per_attempt_secs
- **429**: Rate limit exceeded

### Non-Retryable Errors (no fallback)

- **400**: Bad request (invalid JSON, missing fields)
- **401**: Authentication failure (invalid API key)
- **403**: Authorization failure (insufficient permissions)
- **404**: Not found (invalid model ID)
- **422**: Validation error (invalid parameters)

## Best Practices

### Fallback Configuration

1. **Order fallbacks by preference**: List most preferred (cost/performance) first
2. **Use diverse providers**: Don't fallback to similar providers (same region, same infrastructure)
3. **Set appropriate timeouts**: Balance between waiting for slow responses and failing fast
4. **Monitor fallback rates**: High fallback rates indicate primary provider issues

### Usage Tracking

1. **Verify pricing**: Ensure pricing is accurate for your use case
2. **Monitor anomalies**: Sudden spikes in usage may indicate bugs or abuse
3. **Use tags**: Tag requests for better cost attribution
4. **Review estimates**: Image/audio usage may be estimated; verify accuracy

### Cost Management

1. **Set budgets**: Use team and end-user budgets to prevent overspend
2. **Alert on thresholds**: Monitor spend and alert before budgets exceeded
3. **Optimize fallbacks**: Fallback to cheaper providers when possible
4. **Cache responses**: Reduce costs by caching repeated requests

## Troubleshooting

### Fallback Not Triggering

**Symptoms:** Request fails without trying fallback

**Causes:**
- Error is 4xx (client error) - fallback only triggers on 5xx/timeout/429
- Max attempts already reached
- Fallback chain not configured

**Solution:** Check `request_logs.attempt_number` and `fallback_triggered` columns

### Usage Shows Zero

**Symptoms:** Usage fields are null or zero

**Causes:**
- Provider doesn't return usage (some open-source models)
- Parsing error (unexpected JSON format)
- Streaming request (usage reported at end)

**Solution:** Check provider response format; may need to add custom parser

### Cost Calculation Incorrect

**Symptoms:** Spend doesn't match expected

**Causes:**
- Pricing not configured for model
- Pricing in wrong currency
- Decimal precision issues

**Solution:** Verify `models.pricing` JSONB column; ensure Decimal type used
