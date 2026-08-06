# End-User Budgets

## Overview

End-user budgets allow organizations to set spending limits per user, similar to team budgets (G5). This implements the G5.4 deferred feature for managing individual user spending within an organization.

## Database Schema

```sql
CREATE TABLE end_users (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    organization_id UUID NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    budget_usd NUMERIC(12,4) DEFAULT NULL,
    max_budget_usd NUMERIC(12,4) DEFAULT NULL,
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW(),
    UNIQUE(user_id, organization_id)
);
CREATE INDEX idx_end_users_org ON end_users(organization_id);
```

### Fields

- `id`: Unique identifier for the end-user budget record
- `organization_id`: Reference to the organization that owns this user
- `user_id`: Reference to the user being budgeted
- `budget_usd`: Current budget allocation in USD (nullable)
- `max_budget_usd`: Maximum budget cap in USD (nullable)
- `created_at`: Timestamp when the record was created
- `updated_at`: Timestamp when the record was last updated

## API Endpoints

All endpoints are under `/api/v1/end-users` and require authentication.

### List End-Users

```http
GET /api/v1/end-users?organization_id={uuid}
Authorization: Bearer {jwt_token}
```

**Query Parameters:**
- `organization_id` (optional): Filter by organization. Super admins can query any org; org admins are restricted to their own org.

**Response:**
```json
{
  "data": [
    {
      "id": "uuid",
      "organization_id": "uuid",
      "user_id": "uuid",
      "budget_usd": 100.00,
      "max_budget_usd": 200.00,
      "created_at": "2026-08-07T00:00:00Z",
      "updated_at": "2026-08-07T00:00:00Z"
    }
  ]
}
```

### Create End-User Budget

```http
POST /api/v1/end-users
Authorization: Bearer {jwt_token}
Content-Type: application/json

{
  "user_id": "uuid",
  "organization_id": "uuid",
  "budget_usd": 100.00,
  "max_budget_usd": 200.00
}
```

**Request Body:**
- `user_id` (required): The user to create a budget for
- `organization_id` (optional for super admins, ignored for org admins): Organization the user belongs to
- `budget_usd` (optional): Initial budget allocation
- `max_budget_usd` (optional): Maximum budget cap

**Response:**
```json
{
  "data": {
    "id": "uuid",
    "organization_id": "uuid",
    "user_id": "uuid",
    "budget_usd": 100.00,
    "max_budget_usd": 200.00,
    "created_at": "2026-08-07T00:00:00Z",
    "updated_at": "2026-08-07T00:00:00Z"
  }
}
```

### Get End-User Budget

```http
GET /api/v1/end-users/:user_id
Authorization: Bearer {jwt_token}
```

**Path Parameters:**
- `user_id`: The user whose budget to retrieve

**Response:**
```json
{
  "data": {
    "id": "uuid",
    "organization_id": "uuid",
    "user_id": "uuid",
    "budget_usd": 100.00,
    "max_budget_usd": 200.00,
    "created_at": "2026-08-07T00:00:00Z",
    "updated_at": "2026-08-07T00:00:00Z"
  }
}
```

### Update End-User Budget

```http
PATCH /api/v1/end-users/:user_id
Authorization: Bearer {jwt_token}
Content-Type: application/json

{
  "budget_usd": 150.00,
  "max_budget_usd": 250.00
}
```

**Path Parameters:**
- `user_id`: The user whose budget to update

**Request Body:**
- `budget_usd` (optional): New budget allocation
- `max_budget_usd` (optional): New maximum budget cap

**Response:**
```json
{
  "data": {
    "id": "uuid",
    "organization_id": "uuid",
    "user_id": "uuid",
    "budget_usd": 150.00,
    "max_budget_usd": 250.00,
    "created_at": "2026-08-07T00:00:00Z",
    "updated_at": "2026-08-07T00:00:00Z"
  }
}
```

### Delete End-User Budget

```http
DELETE /api/v1/end-users/:user_id
Authorization: Bearer {jwt_token}
```

**Path Parameters:**
- `user_id`: The user whose budget to delete

**Response:**
```json
{
  "deleted": true
}
```

## Authorization

### Role Requirements

- `super_admin`: Can manage end-user budgets across all organizations. Must supply `organization_id` explicitly when creating budgets for users in other organizations.
- `org_admin`: Can manage end-user budgets within their own organization only. The `organization_id` parameter is ignored and always defaults to the admin's organization.
- `team_admin` / `user`: No access to end-user budget endpoints (403 Forbidden).

### RBAC Behavior

1. **List endpoint**: Super admins see all budgets (or can filter by `organization_id`); org admins see only their organization's budgets.
2. **Create endpoint**: Super admins must provide `organization_id`; org admins use their own organization automatically.
3. **Get/Update/Delete endpoints**: Super admins can access any budget; org admins can only access budgets within their organization.
4. **Cross-org access**: Attempts by org admins to access budgets outside their organization return 403 Forbidden.

## Budget Enforcement

Budget enforcement is checked during API requests in the proxy handlers. If a user's cumulative spend (from `request_logs`) exceeds their `max_budget_usd`, the request is rejected with HTTP 429 (Too Many Requests).

### Enforcement Logic

The budget check happens in `crates/godwit-api/src/rate_limit.rs::check_end_user_budget()`:

```rust
pub async fn check_end_user_budget(
    pool: &PgPool,
    user_id: Uuid,
    org_id: Uuid,
) -> Result<(), ApiError> {
    // 1. Fetch end-user budget record
    let end_user = sqlx::query_as::<_, EndUser>(
        "SELECT * FROM end_users WHERE user_id = $1 AND organization_id = $2",
    )
    .bind(user_id)
    .bind(org_id)
    .fetch_optional(pool)
    .await?;
    
    // 2. No budget = no enforcement
    let Some(end_user) = end_user else { return Ok(()) };
    let Some(max_budget) = end_user.max_budget_usd else { return Ok(()) };
    
    // 3. Sum all request_logs for this user
    let spent: Decimal = sqlx::query_scalar(
        "SELECT COALESCE(SUM(cost_usd), 0) FROM request_logs WHERE user_id = $1 AND organization_id = $2",
    )
    .bind(user_id)
    .bind(org_id)
    .fetch_one(pool)
    .await?;
    
    // 4. Block if spent >= max_budget
    if spent >= max_budget {
        return Err(ApiError::BudgetExceeded);
    }
    
    Ok(())
}
```

### Integration Points

Budget enforcement is called in all proxy endpoints (`chat_completions`, `embeddings`, `image_generations`, `audio_speech`, `audio_transcriptions`, `image_edits`) after rate limit checks:

```rust
check_rate_limit(&state, &api_key, &model, estimated_tokens).await?;
check_user_budget(&state, &api_key).await?;  // ← Budget enforcement
```

### Response

When budget is exceeded:
- **HTTP Status**: 429 Too Many Requests
- **Error Type**: `https://api.godwit.local/errors/budget-exceeded`
- **Detail**: "End-user budget has been exceeded."

**Note:** Unlike rate limiting, budget exceeded does not include a `Retry-After` header since the budget does not automatically reset.

## Team Budget Enforcement

Teams can also have budget limits enforced at request time. When an API key is associated with a team, the gateway checks both the user's budget (if set) and the team's budget (if set) before processing the request.

### Team Budget Check Logic

The team budget check in `crates/godwit-api/src/rate_limit.rs::check_team_budget()`:

```rust
pub async fn check_team_budget(
    pool: &PgPool,
    team_id: Uuid,
) -> Result<(), ApiError> {
    // 1. Fetch team budget record
    let team = sqlx::query_as::<_, Team>(
        "SELECT * FROM teams WHERE id = $1",
    )
    .bind(team_id)
    .fetch_optional(pool)
    .await?;
    
    // 2. No team record = no enforcement
    let Some(team) = team else { return Ok(()) };
    let Some(max_budget) = team.max_budget_usd else { return Ok(()) };
    
    // 3. Sum all request_logs for this team
    let spent: Decimal = sqlx::query_scalar(
        "SELECT COALESCE(SUM(cost_usd), 0) FROM request_logs WHERE team_id = $1",
    )
    .bind(team_id)
    .fetch_one(pool)
    .await?;
    
    // 4. Block if spent >= max_budget
    if spent >= max_budget {
        return Err(ApiError::BudgetExceeded);
    }
    
    Ok(())
}
```

### Integration with User Budgets

When both user and team budgets are configured:
1. User budget is checked first (via `check_user_budget()`)
2. Team budget is checked second (via `check_team_budget()`)
3. Either budget being exceeded results in HTTP 429

The check order matters: a user with their own budget limit will be blocked before the team budget is evaluated, even if the team has remaining budget.

### Response

When team budget is exceeded:
- **HTTP Status**: 429 Too Many Requests
- **Error Type**: `https://api.godwit.local/errors/budget-exceeded`
- **Detail**: "End-user budget has been exceeded."

**Note:** The same error response is used for both user and team budget exceeded scenarios.

## Implementation Files

### Database
- Migration (up): `crates/godwit-db/migrations/20260807000001_end_users.up.sql`
- Migration (down): `crates/godwit-db/migrations/20260807000001_end_users.down.sql`

### Models
- `crates/godwit-db/src/models.rs`: `EndUser` struct

### Repositories
- `crates/godwit-db/src/repositories/end_users.rs`: `EndUsersRepository` with CRUD operations
- `crates/godwit-db/src/repositories/mod.rs`: Module export

### API
- `crates/godwit-api/src/admin/end_users.rs`: REST endpoints
- `crates/godwit-api/src/admin/mod.rs`: Router integration
- `crates/godwit-api/src/state.rs`: AppState field
- `crates/godwit-bin/src/main.rs`: Repository initialization

### Tests
- Repository tests: `crates/godwit-db/src/repositories/end_users.rs` (3 tests)
- API unit tests: `crates/godwit-api/src/admin/end_users.rs` (1 test)
- Integration tests: `crates/godwit-api/tests/router_integration.rs` (AppState update)

## Testing

### Unit Tests (Repository)

```bash
DATABASE_URL=postgres://user:pass@localhost:5432/godwit cargo test -p godwit-db repositories::end_users::tests
```

Tests cover:
- `create_and_get_end_user`: Create and retrieve a single end-user budget
- `list_by_organization`: List multiple end-users within an organization
- `update_budgets`: Update budget values

### Unit Tests (API)

```bash
cargo test -p godwit-api admin::end_users::tests
```

Tests cover:
- `create_end_user_request_deserializes_without_organization_id`: Request deserialization

### Integration Tests

```bash
DATABASE_URL=postgres://user:pass@localhost:5432/godwit cargo test -p godwit-api --test router_integration
```

## Related Features

- **Team Budgets (G5)**: Similar budget management for teams instead of individual users
- **Spend Logs**: Track actual spending against budgets
- **Rate Limiting**: Enforce budget limits in real-time

## Migration Notes

Running the migration creates the `end_users` table. Existing users are not automatically migrated; budgets must be created explicitly via the API.

```bash
# Migrations run automatically on server startup
cargo run --bin godwit
```

## Future Enhancements

1. **Budget periods**: Add support for monthly/weekly/daily budget cycles
2. **Notifications**: Alert users/admins when approaching budget limits
3. **Budget rollover**: Allow unused budget to roll over between periods
4. **Custom time windows**: Configurable budget tracking windows
