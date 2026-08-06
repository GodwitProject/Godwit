# Gap G3 Complement: `/spend/tags` Endpoint Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add `/spend/tags` endpoint to return spend aggregated by `team_id` and `api_key_id` (used as implicit tags), following the LiteLLM `/spend/tags` pattern.

**Architecture:** 
- Create new `spend_tags.rs` module in `godwit-api/src/admin/`
- Implement two aggregation queries: `GROUP BY team_id` and `GROUP BY api_key_id`
- Apply RBAC scoping: super-admin sees all, org-admin sees own org, users see only their own data
- Return response in format: `{ "by_team": [...], "by_api_key": [...] }`

**Tech Stack:** Rust, Axum, SQLx, PostgreSQL

## Global Constraints

- Use `rust_decimal::Decimal` for all cost values
- Follow existing RBAC pattern from `spend.rs` (scope_spend_query function)
- Support `from`/`to` datetime filters and optional `organization_id` filter
- Response must match LiteLLM's `/spend/tags` structure (adapted for our implicit tags)
- All queries must use parameterized SQL to prevent injection
- Follow existing code style in `spend.rs` and `spend_logs.rs`

---

### Task 1: Create `/spend/tags` endpoint module

**Files:**
- Create: `crates/godwit-api/src/admin/spend_tags.rs`
- Modify: `crates/godwit-api/src/admin/mod.rs`

**Interfaces:**
- Consumes: `request_logs` table with columns: `team_id`, `api_key_id`, `cost_usd`, `created_at`, `organization_id`
- Produces: `GET /api/v1/spend/tags` endpoint

- [ ] **Step 1: Create `spend_tags.rs` with router and query structs**

Create new file `crates/godwit-api/src/admin/spend_tags.rs`:

```rust
use axum::{
    extract::{Extension, Query, State},
    routing::get,
    Json, Router,
};
use chrono::{DateTime, Utc};
use godwit_auth::{jwt::Claims, rbac::Role};
use rust_decimal::Decimal;
use serde::Deserialize;
use std::str::FromStr;
use std::sync::Arc;
use uuid::Uuid;

use crate::{error::ApiError, state::AppState};

pub fn router() -> Router<Arc<AppState>> {
    Router::new().route("/spend/tags", get(get_spend_tags))
}

#[derive(Debug, Deserialize)]
struct SpendTagsQuery {
    from: Option<DateTime<Utc>>,
    to: Option<DateTime<Utc>>,
    organization_id: Option<Uuid>,
}

#[derive(Debug, serde::Serialize)]
struct TeamSpend {
    team_id: Option<Uuid>,
    spend_usd: Decimal,
}

#[derive(Debug, serde::Serialize)]
struct ApiKeySpend {
    api_key_id: Option<Uuid>,
    spend_usd: Decimal,
}

#[derive(Debug, serde::Serialize)]
struct SpendTagsResponse {
    by_team: Vec<TeamSpend>,
    by_api_key: Vec<ApiKeySpend>,
}
```

- [ ] **Step 2: Implement RBAC scoping function**

Add the scoping function (mirrors `scope_spend_query` from `spend.rs`):

```rust
/// Applies RBAC scoping: super_admin gets what it asked for,
/// org_admin is forced to own org, team_admin/user see only their own data.
fn scope_spend_tags_query(
    claims: &Claims,
    query: SpendTagsQuery,
) -> (Option<Uuid>, Option<Uuid>, Option<Uuid>) {
    let role = Role::from_str(&claims.role);
    match role {
        Some(Role::SuperAdmin) => (query.organization_id, None, None),
        Some(Role::OrgAdmin) => (Some(claims.organization_id), None, None),
        _ => (Some(claims.organization_id), None, Some(claims.user_id)),
    }
}
```

- [ ] **Step 3: Implement `fetch_team_spend` function**

```rust
async fn fetch_team_spend(
    pool: &sqlx::PgPool,
    from: Option<DateTime<Utc>>,
    to: Option<DateTime<Utc>>,
    organization_id: Option<Uuid>,
    user_id: Option<Uuid>,
) -> Result<Vec<TeamSpend>, sqlx::Error> {
    sqlx::query_as::<_, TeamSpend>(
        "SELECT team_id, COALESCE(SUM(cost_usd), 0) AS spend_usd
         FROM request_logs
         WHERE ($1::timestamptz IS NULL OR created_at >= $1)
           AND ($2::timestamptz IS NULL OR created_at <= $2)
           AND ($3::uuid IS NULL OR organization_id = $3)
           AND ($4::uuid IS NULL OR user_id = $4)
         GROUP BY team_id
         ORDER BY spend_usd DESC",
    )
    .bind(from)
    .bind(to)
    .bind(organization_id)
    .bind(user_id)
    .fetch_all(pool)
    .await
}
```

- [ ] **Step 4: Implement `fetch_api_key_spend` function**

```rust
async fn fetch_api_key_spend(
    pool: &sqlx::PgPool,
    from: Option<DateTime<Utc>>,
    to: Option<DateTime<Utc>>,
    organization_id: Option<Uuid>,
    user_id: Option<Uuid>,
) -> Result<Vec<ApiKeySpend>, sqlx::Error> {
    sqlx::query_as::<_, ApiKeySpend>(
        "SELECT api_key_id, COALESCE(SUM(cost_usd), 0) AS spend_usd
         FROM request_logs
         WHERE ($1::timestamptz IS NULL OR created_at >= $1)
           AND ($2::timestamptz IS NULL OR created_at <= $2)
           AND ($3::uuid IS NULL OR organization_id = $3)
           AND ($4::uuid IS NULL OR user_id = $4)
         GROUP BY api_key_id
         ORDER BY spend_usd DESC",
    )
    .bind(from)
    .bind(to)
    .bind(organization_id)
    .bind(user_id)
    .fetch_all(pool)
    .await
}
```

- [ ] **Step 5: Implement `get_spend_tags` handler**

```rust
async fn get_spend_tags(
    State(state): State<Arc<AppState>>,
    Extension(claims): Extension<Claims>,
    Query(query): Query<SpendTagsQuery>,
) -> Result<Json<SpendTagsResponse>, ApiError> {
    Role::from_str(&claims.role).ok_or(ApiError::Forbidden)?;
    
    let (organization_id, _team_id, user_id) = scope_spend_tags_query(&claims, query);
    
    let by_team = fetch_team_spend(&state.pool, query.from, query.to, organization_id, user_id)
        .await
        .map_err(|e| ApiError::Core(godwit_core::PasteurError::Database(e.to_string())))?;
    
    let by_api_key = fetch_api_key_spend(&state.pool, query.from, query.to, organization_id, user_id)
        .await
        .map_err(|e| ApiError::Core(godwit_core::PasteurError::Database(e.to_string())))?;
    
    Ok(Json(SpendTagsResponse { by_team, by_api_key }))
}
```

- [ ] **Step 6: Register module in `admin/mod.rs`**

Add at the top of `crates/godwit-api/src/admin/mod.rs`:
```rust
pub mod spend_tags;
```

Add to router merge (after `.merge(spend_logs::router())`):
```rust
.merge(spend_tags::router())
```

- [ ] **Step 7: Run check**

```bash
export PATH="/usr/local/opt/rustup/bin:$PATH"
cargo check -p godwit-api
```

Expected: Compiles successfully

- [ ] **Step 8: Commit**

```bash
git add crates/godwit-api/src/admin/spend_tags.rs crates/godwit-api/src/admin/mod.rs
git commit -m "G3.complement: Add /spend/tags endpoint for tag-based spend aggregation"
```

---

### Task 2: Add unit tests for serialization and RBAC scoping

**Files:**
- Modify: `crates/godwit-api/src/admin/spend_tags.rs` (add `#[cfg(test)]` module)

**Interfaces:**
- Consumes: `SpendTagsResponse`, `scope_spend_tags_query`
- Produces: Test coverage for response serialization and RBAC logic

- [ ] **Step 1: Add test module at end of `spend_tags.rs`**

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    #[test]
    fn spend_tags_response_serializes_correctly() {
        let response = SpendTagsResponse {
            by_team: vec![
                TeamSpend {
                    team_id: Some(Uuid::nil()),
                    spend_usd: dec!(12.34),
                },
                TeamSpend {
                    team_id: None,
                    spend_usd: dec!(5.67),
                },
            ],
            by_api_key: vec![
                ApiKeySpend {
                    api_key_id: Some(Uuid::nil()),
                    spend_usd: dec!(56.78),
                },
            ],
        };
        
        let json = serde_json::to_string(&response).expect("serialize");
        assert!(json.contains("by_team"));
        assert!(json.contains("by_api_key"));
        assert!(json.contains("12.34"));
        assert!(json.contains("56.78"));
    }

    #[test]
    fn spend_tags_scope_forces_org_admin_to_own_org() {
        let claims = godwit_auth::jwt::Claims::new(
            uuid::Uuid::new_v4(),
            uuid::Uuid::new_v4(),
            "org_admin"
        );
        let requested = SpendTagsQuery {
            from: None,
            to: None,
            organization_id: Some(uuid::Uuid::new_v4()), // attempt to look at different org
        };
        let scoped = scope_spend_tags_query(&claims, requested);
        assert_eq!(scoped.0, Some(claims.organization_id));
    }

    #[test]
    fn spend_tags_scope_forces_user_to_self() {
        let claims = godwit_auth::jwt::Claims::new(
            uuid::Uuid::new_v4(),
            uuid::Uuid::new_v4(),
            "user"
        );
        let requested = SpendTagsQuery {
            from: None,
            to: None,
            organization_id: Some(uuid::Uuid::new_v4()),
        };
        let scoped = scope_spend_tags_query(&claims, requested);
        assert_eq!(scoped.0, Some(claims.organization_id));
        assert_eq!(scoped.2, Some(claims.user_id));
    }

    #[test]
    fn spend_tags_scope_leaves_super_admin_unscoped() {
        let claims = godwit_auth::jwt::Claims::new(
            uuid::Uuid::new_v4(),
            uuid::Uuid::new_v4(),
            "super_admin"
        );
        let org_id = uuid::Uuid::new_v4();
        let requested = SpendTagsQuery {
            from: None,
            to: None,
            organization_id: Some(org_id),
        };
        let scoped = scope_spend_tags_query(&claims, requested);
        assert_eq!(scoped.0, Some(org_id));
    }
}
```

- [ ] **Step 2: Run unit tests**

```bash
export PATH="/usr/local/opt/rustup/bin:$PATH"
cargo test -p godwit-api --lib admin::spend_tags::tests
```

Expected: All 4 tests pass

- [ ] **Step 3: Commit**

```bash
git add crates/godwit-api/src/admin/spend_tags.rs
git commit -m "G3.complement: Add unit tests for /spend/tags serialization and RBAC"
```

---

### Task 3: Add database integration tests

**Files:**
- Modify: `crates/godwit-api/src/admin/spend_tags.rs` (add `#[cfg(test)]` with sqlx tests)

**Interfaces:**
- Consumes: `fetch_team_spend`, `fetch_api_key_spend` functions
- Produces: DB-backed tests verifying aggregation correctness

- [ ] **Step 1: Add SQLx test for team aggregation**

Add to the test module in `spend_tags.rs`:

```rust
    #[sqlx::test(migrations = "../godwit-db/migrations")]
    async fn spend_tags_by_team_aggregates_correctly(pool: sqlx::PgPool) {
        let org = Uuid::new_v4();
        sqlx::query("INSERT INTO organizations (id, name) VALUES ($1, 'test-org')")
            .bind(org)
            .execute(&pool)
            .await
            .expect("insert org");

        let team_a = Uuid::new_v4();
        let team_b = Uuid::new_v4();
        sqlx::query("INSERT INTO teams (id, organization_id, name) VALUES ($1, $2, 'team-a'), ($3, $2, 'team-b')")
            .bind(team_a)
            .bind(org)
            .bind(team_b)
            .execute(&pool)
            .await
            .expect("insert teams");

        let user = Uuid::new_v4();
        sqlx::query("INSERT INTO users (id, organization_id, email, role) VALUES ($1, $2, 'test@example.com', 'user')")
            .bind(user)
            .bind(org)
            .execute(&pool)
            .await
            .expect("insert user");

        // Insert request_logs with different team assignments
        sqlx::query(
            "INSERT INTO request_logs
             (organization_id, team_id, user_id, model, provider, provider_model_id,
              tokens_in, tokens_out, cost_usd, duration_ms, streamed, status)
             VALUES
             ($1, $2, $6, 'gpt-4o', 'openai', 'gpt-4o', 100, 50, 1.50, 10, false, 'success'),
             ($1, $2, $6, 'gpt-4o', 'openai', 'gpt-4o', 200, 100, 2.50, 20, false, 'success'),
             ($1, $3, $6, 'gpt-4o', 'openai', 'gpt-4o', 300, 150, 3.00, 30, false, 'success')",
        )
        .bind(org)
        .bind(team_a)
        .bind(team_b)
        .bind(user)
        .execute(&pool)
        .await
        .expect("insert request_logs");

        let result = fetch_team_spend(&pool, None, None, Some(org), None)
            .await
            .expect("fetch team spend");

        assert_eq!(result.len(), 2);
        assert_eq!(result[0].team_id, Some(team_a));
        assert_eq!(result[0].spend_usd, dec!(4.00));
        assert_eq!(result[1].team_id, Some(team_b));
        assert_eq!(result[1].spend_usd, dec!(3.00));
    }
```

- [ ] **Step 2: Add SQLx test for api_key aggregation**

```rust
    #[sqlx::test(migrations = "../godwit-db/migrations")]
    async fn spend_tags_by_api_key_aggregates_correctly(pool: sqlx::PgPool) {
        let org = Uuid::new_v4();
        sqlx::query("INSERT INTO organizations (id, name) VALUES ($1, 'test-org')")
            .bind(org)
            .execute(&pool)
            .await
            .expect("insert org");

        let user = Uuid::new_v4();
        sqlx::query("INSERT INTO users (id, organization_id, email, role) VALUES ($1, $2, 'test@example.com', 'user')")
            .bind(user)
            .bind(org)
            .execute(&pool)
            .await
            .expect("insert user");

        let key_a = Uuid::new_v4();
        let key_b = Uuid::new_v4();
        sqlx::query(
            "INSERT INTO api_keys
             (id, user_id, organization_id, name, key_prefix, key_hash, scopes)
             VALUES
             ($1, $2, $3, 'key-a', 'prefix-a', 'hash-a', '{proxy:write}'),
             ($4, $2, $3, 'key-b', 'prefix-b', 'hash-b', '{proxy:write}')",
        )
        .bind(key_a)
        .bind(user)
        .bind(org)
        .bind(key_b)
        .execute(&pool)
        .await
        .expect("insert api_keys");

        sqlx::query(
            "INSERT INTO request_logs
             (organization_id, api_key_id, user_id, model, provider, provider_model_id,
              tokens_in, tokens_out, cost_usd, duration_ms, streamed, status)
             VALUES
             ($1, $2, $5, 'gpt-4o', 'openai', 'gpt-4o', 100, 50, 1.00, 10, false, 'success'),
             ($1, $2, $5, 'gpt-4o', 'openai', 'gpt-4o', 200, 100, 2.00, 20, false, 'success'),
             ($1, $3, $5, 'gpt-4o', 'openai', 'gpt-4o', 300, 150, 3.00, 30, false, 'success')",
        )
        .bind(org)
        .bind(key_a)
        .bind(key_b)
        .bind(user)
        .execute(&pool)
        .await
        .expect("insert request_logs");

        let result = fetch_api_key_spend(&pool, None, None, Some(org), None)
            .await
            .expect("fetch api_key spend");

        assert_eq!(result.len(), 2);
        // Results are ordered by spend_usd DESC
        assert_eq!(result[0].api_key_id, Some(key_b));
        assert_eq!(result[0].spend_usd, dec!(3.00));
        assert_eq!(result[1].api_key_id, Some(key_a));
        assert_eq!(result[1].spend_usd, dec!(3.00));
    }
```

- [ ] **Step 3: Add test for datetime filtering**

```rust
    #[sqlx::test(migrations = "../godwit-db/migrations")]
    async fn spend_tags_respects_from_to_filters(pool: sqlx::PgPool) {
        let org = Uuid::new_v4();
        sqlx::query("INSERT INTO organizations (id, name) VALUES ($1, 'test-org')")
            .bind(org)
            .execute(&pool)
            .await
            .expect("insert org");

        let user = Uuid::new_v4();
        sqlx::query("INSERT INTO users (id, organization_id, email, role) VALUES ($1, $2, 'test@example.com', 'user')")
            .bind(user)
            .bind(org)
            .execute(&pool)
            .await
            .expect("insert user");

        let now = Utc::now();
        let yesterday = now - chrono::Duration::days(1);
        let tomorrow = now + chrono::Duration::days(1);

        sqlx::query(
            "INSERT INTO request_logs
             (organization_id, user_id, model, provider, provider_model_id,
              cost_usd, duration_ms, streamed, status, created_at)
             VALUES
             ($1, $2, 'gpt-4o', 'openai', 'gpt-4o', 1.00, 10, false, 'success', $3),
             ($1, $2, 'gpt-4o', 'openai', 'gpt-4o', 2.00, 20, false, 'success', $4),
             ($1, $2, 'gpt-4o', 'openai', 'gpt-4o', 4.00, 30, false, 'success', $5)",
        )
        .bind(org)
        .bind(user)
        .bind(yesterday)
        .bind(now)
        .bind(tomorrow)
        .execute(&pool)
        .await
        .expect("insert request_logs");

        // Query only today and future
        let result = fetch_team_spend(&pool, Some(now), Some(tomorrow), Some(org), Some(user))
            .await
            .expect("fetch filtered spend");

        assert_eq!(result.len(), 1);
        assert_eq!(result[0].spend_usd, dec!(6.00)); // 2.00 + 4.00
    }
```

- [ ] **Step 4: Run database tests**

```bash
export PATH="/usr/local/opt/rustup/bin:$PATH"
DATABASE_URL=postgres://user:pass@localhost:5432/godwit cargo test -p godwit-api --lib admin::spend_tags
```

Expected: All tests pass (requires running PostgreSQL)

- [ ] **Step 5: Commit**

```bash
git add crates/godwit-api/src/admin/spend_tags.rs
git commit -m "G3.complement: Add database integration tests for /spend/tags"
```

---

### Task 4: Update G3 report documentation

**Files:**
- Create: `docs/gap-audit/G3-spend-tags-report.md`

**Interfaces:**
- Documents the `/spend/tags` implementation as complement to G3

- [ ] **Step 1: Create report file**

Create `docs/gap-audit/G3-spend-tags-report.md`:

```markdown
# G3 Complement: `/spend/tags` Endpoint Report

**Status:** Complete

**Date:** 2026-08-06

## Summary

Implemented `/spend/tags` endpoint to return spend aggregated by implicit tags (`team_id` and `api_key_id`), following LiteLLM's `/spend/tags` pattern.

## Files Modified

1. `crates/godwit-api/src/admin/spend_tags.rs` — New module with endpoint implementation
2. `crates/godwit-api/src/admin/mod.rs` — Registered `spend_tags` module

## Implementation Details

### Endpoint

- **Path:** `GET /api/v1/spend/tags`
- **Query Parameters:**
  - `from: DateTime<Utc>` (optional) — Start of time range
  - `to: DateTime<Utc>` (optional) — End of time range
  - `organization_id: Uuid` (optional) — Filter by organization (super-admin only)

### Response Format

```json
{
  "by_team": [
    { "team_id": "...", "spend_usd": "12.34" },
    { "team_id": null, "spend_usd": "5.67" }
  ],
  "by_api_key": [
    { "api_key_id": "...", "spend_usd": "56.78" },
    { "api_key_id": null, "spend_usd": "1.23" }
  ]
}
```

### RBAC

- **Super Admin:** Sees all data, can filter by `organization_id`
- **Org Admin:** Sees only own organization's data
- **User/Team Admin:** Sees only own data (filtered by `user_id`)

### SQL Queries

Two aggregation queries:
1. `SELECT team_id, SUM(cost_usd) FROM request_logs GROUP BY team_id`
2. `SELECT api_key_id, SUM(cost_usd) FROM request_logs GROUP BY api_key_id`

Both queries respect `from`/`to` datetime filters and organization/user scoping.

## Test Coverage

### Unit Tests (4)
- `spend_tags_response_serializes_correctly` — JSON serialization
- `spend_tags_scope_forces_org_admin_to_own_org` — RBAC org scoping
- `spend_tags_scope_forces_user_to_self` — RBAC user scoping
- `spend_tags_scope_leaves_super_admin_unscoped` — RBAC super-admin passthrough

### Database Tests (3)
- `spend_tags_by_team_aggregates_correctly` — Team aggregation
- `spend_tags_by_api_key_aggregates_correctly` — API key aggregation
- `spend_tags_respects_from_to_filters` — Datetime filtering

**Total:** 7 tests

## Verification Commands

```bash
# Compile check
export PATH="/usr/local/opt/rustup/bin:$PATH"
cargo check -p godwit-api

# Unit tests
cargo test -p godwit-api --lib admin::spend_tags::tests

# Database tests (requires PostgreSQL)
DATABASE_URL=postgres://user:pass@localhost:5432/godwit cargo test -p godwit-api --lib admin::spend_tags
```

## Design Decision: Implicit Tags

Per G3 spec, `request_logs` table does not have a `tags` column. Following **Option B** from the gap analysis:

- Use `team_id` and `api_key_id` as implicit tags
- This avoids schema migration and backfill requirements
- Aligns with existing RBAC model (users belong to teams, API keys belong to users)
- LiteLLM compatibility: response structure matches `/spend/tags` semantics

## Future Enhancements (Not in MVP)

- Add explicit `tags TEXT[]` column to `request_logs` if use cases require arbitrary tagging
- Support for custom tag extraction from request metadata
- Caching layer for frequently-accessed tag aggregations

## Commits

1. `G3.complement: Add /spend/tags endpoint for tag-based spend aggregation`
2. `G3.complement: Add unit tests for /spend/tags serialization and RBAC`
3. `G3.complement: Add database integration tests for /spend/tags`
```

- [ ] **Step 2: Commit**

```bash
git add docs/gap-audit/G3-spend-tags-report.md
git commit -m "G3.complement: Add /spend/tags implementation report"
```

---

### Task 5: Final verification

**Files:**
- None (verification only)

- [ ] **Step 1: Full workspace check**

```bash
export PATH="/usr/local/opt/rustup/bin:$PATH"
cargo check --workspace --tests
```

Expected: No compilation errors

- [ ] **Step 2: Run all godwit-api tests**

```bash
export PATH="/usr/local/opt/rustup/bin:$PATH"
cargo test -p godwit-api --lib
```

Expected: All tests pass (including new spend_tags tests)

- [ ] **Step 3: Verify endpoint is registered**

Check that `admin/mod.rs` includes:
```rust
pub mod spend_tags;
```
and
```rust
.merge(spend_tags::router())
```

---

## Summary of Commits

1. `G3.complement: Add /spend/tags endpoint for tag-based spend aggregation`
2. `G3.complement: Add unit tests for /spend/tags serialization and RBAC`
3. `G3.complement: Add database integration tests for /spend/tags`
4. `G3.complement: Add /spend/tags implementation report`
