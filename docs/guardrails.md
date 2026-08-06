# Guardrails

Godwit provides comprehensive guardrails features including PII masking, content moderation, and budget alerting to ensure safe and cost-controlled API usage.

## PII Masking

Automatically detect and mask personally identifiable information in requests and responses.

### Configuration

```yaml
pii:
  enabled: true
  mask_request: true
  mask_response: true
  patterns:
    - name: email
      pattern: "[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\\.[a-zA-Z]{2,}"
      replacement: "[EMAIL]"
      enabled: true
    - name: phone
      pattern: "\\+?[\\d\\s-()]{10,}"
      replacement: "[PHONE]"
      enabled: true
```

### Default Patterns

- Email addresses
- Phone numbers (10+ digits)
- Credit card numbers (16 digits with optional separators)
- Social Security Numbers (XXX-XX-XXXX)

### How It Works

1. **Pre-call**: Request messages are scanned and PII replaced with placeholders
2. **Provider call**: Masked content sent to provider
3. **Post-call**: Response scanned; if it references masked data, originals restored

### Example

**Request:**
```
"My email is test@example.com"
```

**Sent to provider:**
```
"My email is [EMAIL]"
```

## Moderation Guardrails

Block toxic content before and after provider calls.

### Configuration

```yaml
guardrails:
  moderation_pre: true
  moderation_post: true
  block_on_moderation_failure: true
```

### Pre-Call Moderation

Checks request content before sending to provider. If flagged:
- Returns 400 `ModerationBlocked` error
- Does not charge for the request
- Logs the event

### Post-Call Moderation

Checks provider response before returning. If flagged:
- Returns 400 `ModerationBlocked` error
- Does not return toxic content to user
- Logs the event

## Budget Alerting

Send webhooks when spending approaches budget limits.

### Configuration

```yaml
alerting:
  enabled: true
  check_interval_secs: 300
  webhooks:
    - org_id: "xxx"
      budget_threshold_percent: 80
      webhook_url: "https://hooks.slack.com/xxx"
    - org_id: "xxx"
      budget_threshold_percent: 100
      webhook_url: "https://api.example.com/alerts"
```

### Alert Events

- `budget_80`: Spend >= 80% of budget (warning)
- `budget_100`: Spend >= 100% of budget (critical)

### Payload

```json
{
  "event_type": "budget_80",
  "org_id": "xxx",
  "current_spend": 800.00,
  "budget": 1000.00,
  "threshold_percent": 80,
  "timestamp": "2026-08-07T12:00:00Z"
}
```

### Retry Logic

Failed webhooks are retried with exponential backoff:
- Attempt 1: immediate
- Attempt 2: 30 seconds
- Attempt 3: 2 minutes
- Attempt 4: 10 minutes
- Attempt 5: 1 hour
- After 5 failures: marked as failed, no more retries
