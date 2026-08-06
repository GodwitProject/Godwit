# Model Aliasing

Model aliasing allows you to create friendly names (aliases) that resolve to existing models in the catalog. This is similar to LiteLLM's virtual model feature.

## Use Cases

- **Backward compatibility**: Keep old model names working when upgrading to new models
- **Simplified naming**: Use `gpt-4-turbo` instead of `gpt-4-1106-preview`
- **Unified endpoints**: Map multiple aliases to the same underlying model
- **Testing**: Alias `production-model` to different actual models during A/B testing

## Database Schema

```sql
CREATE TABLE model_aliases (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    alias TEXT NOT NULL UNIQUE,
    target_model_id UUID NOT NULL REFERENCES models(id) ON DELETE CASCADE,
    created_at TIMESTAMPTZ DEFAULT NOW()
);
CREATE INDEX idx_model_aliases_alias ON model_aliases(alias);
```

## Resolution Order

When a model reference is received, the router checks in this order:

1. **Alias lookup**: Is the entire model_ref an alias? If yes, resolve to target model
2. **Profile prefix**: Does it have a `profile/` prefix? Route to that profile
3. **Public ID**: Match against `models.public_id`
4. **Wildcard**: If profile allows wildcards, synthesize a model

## Admin API

### List all aliases

```bash
GET /api/v1/model-aliases
Authorization: Bearer <super_admin_jwt>
```

Response:
```json
{
  "data": [
    {
      "id": "uuid",
      "alias": "gpt-4-turbo",
      "target_model_id": "uuid",
      "created_at": "2026-08-08T00:00:00Z"
    }
  ]
}
```

### Create an alias

```bash
POST /api/v1/model-aliases
Authorization: Bearer <super_admin_jwt>
Content-Type: application/json

{
  "alias": "gpt-4-turbo",
  "target_model_id": "550e8400-e29b-41d4-a716-446655440000"
}
```

The `target_model_id` must reference an existing model in the catalog.

### Delete an alias

```bash
DELETE /api/v1/model-aliases/:id
Authorization: Bearer <super_admin_jwt>
```

Response:
```json
{
  "deleted": true
}
```

## Cascading Deletes

When a model is deleted, all aliases pointing to it are automatically deleted via `ON DELETE CASCADE`.

## Implementation Files

- Migration: `crates/godwit-db/migrations/20260808000001_model_aliases.up.sql`
- Model: `crates/godwit-db/src/models/model_alias.rs`
- Repository: `crates/godwit-db/src/repositories/model_aliases.rs`
- Router integration: `crates/godwit-api/src/model_router.rs` (resolve method)
- Admin endpoints: `crates/godwit-api/src/admin/model_aliases.rs`

## Tests

- `godwit-db`: 5 repository tests (create, get, list, delete, error cases)
- `godwit-api`: 2 router tests (alias resolution, non-existent target)
- `godwit-api`: 1 deserialization test for admin endpoint

## Example

```bash
# Create a model
curl -X POST http://localhost:3000/api/v1/models \
  -H "Authorization: Bearer $JWT" \
  -H "Content-Type: application/json" \
  -d '{
    "public_id": "gpt-4o",
    "provider": "openai",
    "provider_profile_id": "...",
    "provider_model_id": "gpt-4o",
    "capabilities": "chat"
  }'

# Create an alias
curl -X POST http://localhost:3000/api/v1/model-aliases \
  -H "Authorization: Bearer $JWT" \
  -H "Content-Type: application/json" \
  -d '{
    "alias": "gpt-4-turbo",
    "target_model_id": "..."
  }'

# Use the alias in chat
curl http://localhost:3000/v1/chat/completions \
  -H "Authorization: Bearer $API_KEY" \
  -H "Content-Type: application/json" \
  -d '{
    "model": "gpt-4-turbo",
    "messages": [{"role": "user", "content": "Hello"}]
  }'
```

The request will be routed to the `gpt-4o` model.
