# Request Logs Tags Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add `tags TEXT[]` column to `request_logs` table with migration, model updates, repository methods, API integration for passing tags via `X-Godwit-Tags` header, and `/spend/tags` endpoint support for custom tag aggregation.

**Architecture:** 
- Database migration adds `tags TEXT[] DEFAULT '{}'` column with GIN index
- Repository layer provides `create_with_tags()` and `find_by_tag()` methods
- API layer extracts tags from `X-Godwit-Tags: tag1,tag2` header in proxy endpoints
- `/spend/tags` endpoint extended to support `?tag=mytag` query parameter for custom tag aggregation
- `RequestLogEntry` struct updated to include `tags: Vec<String>` field

**Tech Stack:** PostgreSQL (SQLx), Rust, Axum, serde_json, rust_decimal

## Global Constraints

- Migration must be reversible (provide `.down.sql`)
- Tags default to empty array `{}`
- Use GIN index for efficient tag queries
- Tags extracted from header `X-Godwit-Tags` as comma-separated values
- Maintain backward compatibility with existing code that doesn't pass tags
- All monetary values use `rust_decimal::Decimal`
- Tests require `DATABASE_URL` environment variable

---

### Task 1: Database Migration

**Files:**
- Create: `crates/godwit-db/migrations/20260808000002_request_logs_tags.up.sql`
- Create: `crates/godwit-db/migrations/20260808000002_request_logs_tags.down.sql`

**Interfaces:**
- Consumes: None
- Produces: Migration files that add/drop `tags` column

- [ ] **Step 1: Create up migration**

```sql
-- File: crates/godwit-db/migrations/20260808000002_request_logs_tags.up.sql
ALTER TABLE request_logs ADD COLUMN tags TEXT[] DEFAULT '{}';
CREATE INDEX idx_request_logs_tags ON request_logs USING GIN(tags);
```

- [ ] **Step 2: Create down migration**

```sql
-- File: crates/godwit-db/migrations/20260808000002_request_logs_tags.down.sql
DROP INDEX IF EXISTS idx_request_logs_tags;
ALTER TABLE request_logs DROP COLUMN tags;
```

- [ ] **Step 3: Verify migration files exist**

```bash
ls -la crates/godwit-db/migrations/ | grep request_logs_tags
```

Expected: Both `.up.sql` and `.down.sql` files listed

- [ ] **Step 4: Commit migration**

```bash
git add crates/godwit-db/migrations/20260808000002_request_logs_tags.up.sql
git add crates/godwit-db/migrations/20260808000002_request_logs_tags.down.sql
git commit -m "migrations: Add tags TEXT[] column to request_logs with GIN index"
```

### Task 2: Update RequestLogEntry Struct

**Files:**
- Modify: `crates/godwit-api/src/proxy.rs:1347-1360`

**Interfaces:**
- Consumes: None
- Produces: `RequestLogEntry` with `tags: Vec<String>` field

- [ ] **Step 1: Add tags field to RequestLogEntry**

```rust
pub(crate) struct RequestLogEntry {
    pub(crate) api_key_id: uuid::Uuid,
    pub(crate) user_id: uuid::Uuid,
    pub(crate) organization_id: uuid::Uuid,
    pub(crate) team_id: Option<uuid::Uuid>,
    pub(crate) model: String,
    pub(crate) provider: String,
    pub(crate) provider_model_id: String,
    pub(crate) capability: String,
    pub(crate) duration_ms: i32,
    pub(crate) streamed: bool,
    pub(crate) status: String,
    pub(crate) cost_usd: Option<Decimal>,
    pub(crate) tags: Vec<String>,
}
```

- [ ] **Step 2: Update spawn_request_log to include tags**

Modify the INSERT query at `proxy.rs:1326`:

```rust
pub(crate) fn spawn_request_log(pool: sqlx::PgPool, log: RequestLogEntry) {
    tokio::spawn(async move {
        let _ = sqlx::query(
            "INSERT INTO request_logs (api_key_id, user_id, organization_id, team_id, model, provider, provider_model_id, capability, duration_ms, streamed, status, cost_usd, tags)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13)"
        )
        .bind(log.api_key_id)
        .bind(log.user_id)
        .bind(log.organization_id)
        .bind(log.team_id)
        .bind(log.model)
        .bind(log.provider)
        .bind(log.provider_model_id)
        .bind(log.capability)
        .bind(log.duration_ms)
        .bind(log.streamed)
        .bind(log.status)
        .bind(log.cost_usd)
        .bind(&log.tags)
        .execute(&pool)
        .await;
    });
}
```

- [ ] **Step 3: Verify compilation**

```bash
export PATH="/usr/local/opt/rustup/bin:$PATH"
cargo check -p godwit-api
```

Expected: No errors (will fail until all call sites are updated in Task 3)

- [ ] **Step 4: Commit**

```bash
git add crates/godwit-api/src/proxy.rs
git commit -m "api: Add tags field to RequestLogEntry struct"
```

### Task 3: Update All spawn_request_log Call Sites

**Files:**
- Modify: `crates/godwit-api/src/proxy.rs` (all call sites)
- Modify: `crates/godwit-api/src/anthropic_proxy.rs` (all call sites)

**Interfaces:**
- Consumes: `RequestLogEntry` with `tags` field
- Produces: All call sites pass `tags: vec![]` by default

- [ ] **Step 1: Update proxy.rs chat_completions call site (line 606)**

```rust
spawn_request_log(state.pool.clone(), RequestLogEntry {
    api_key_id: log.api_key_id,
    user_id: log.user_id,
    organization_id: log.organization_id,
    team_id: log.team_id,
    model: log.model,
    provider: log.provider,
    provider_model_id: log.provider_model_id,
    capability: log.capability,
    duration_ms: log.duration_ms,
    streamed: log.streamed,
    status: log.status,
    cost_usd: log.cost_usd,
    tags: vec![], // Will be populated from header in Task 4
});
```

- [ ] **Step 2: Update all other proxy.rs call sites (lines 707, 804, 917, 1078, 1262)**

Same pattern as Step 1 for each location.

- [ ] **Step 3: Update anthropic_proxy.rs call sites (lines 336, 428)**

Same pattern for both locations.

- [ ] **Step 4: Verify compilation**

```bash
export PATH="/usr/local/opt/rustup/bin:$PATH"
cargo check --workspace
```

Expected: No errors

- [ ] **Step 5: Commit**

```bash
git add crates/godwit-api/src/proxy.rs crates/godwit-api/src/anthropic_proxy.rs
git commit -m "api: Update all spawn_request_log call sites with tags field"
```

### Task 4: Extract Tags from X-Godwit-Tags Header

**Files:**
- Modify: `crates/godwit-api/src/proxy.rs` (chat_completions and other endpoints)

**Interfaces:**
- Consumes: `RequestLogEntry` with `tags` field
- Produces: Tags extracted from header and passed to `spawn_request_log`

- [ ] **Step 1: Add helper function to extract tags from header**

Add before `spawn_request_log` function:

```rust
fn extract_tags_from_header(header_value: Option<&str>) -> Vec<String> {
    header_value
        .map(|h| h.split(',').map(|s| s.trim().to_string()).filter(|s| !s.is_empty()).collect())
        .unwrap_or_default()
}
```

- [ ] **Step 2: Update chat_completions to extract tags**

In `chat_completions` function, extract tags from headers:

```rust
let tags = extract_tags_from_header(
    req.headers()
        .get("x-godwit-tags")
        .and_then(|v| v.to_str().ok())
);
```

- [ ] **Step 3: Pass tags to spawn_request_log**

```rust
spawn_request_log(state.pool.clone(), RequestLogEntry {
    // ... other fields
    tags,
});
```

- [ ] **Step 4: Update other endpoints (embeddings, images, etc.)**

Same pattern for all endpoints that call `spawn_request_log`.

- [ ] **Step 5: Verify compilation**

```bash
export PATH="/usr/local/opt/rustup/bin:$PATH"
cargo check -p godwit-api
```

Expected: No errors

- [ ] **Step 6: Commit**

```bash
git add crates/godwit-api/src/proxy.rs
git commit -m "api: Extract tags from X-Godwit-Tags header in all endpoints"
```

### Task 5: Create RequestLogs Repository

**Files:**
- Create: `crates/godwit-db/src/repositories/request_logs.rs`
- Modify: `crates/godwit-db/src/repositories/mod.rs`

**Interfaces:**
- Consumes: `request_logs` table with `tags` column
- Produces: `RequestLogsRepository` with `create_with_tags()` and `find_by_tag()` methods

- [ ] **Step 1: Create repository file**

```rust
// File: crates/godwit-db/src/repositories/request_logs.rs
use crate::models::RequestLog;
use godwit_core::PasteurError;
use sqlx::PgPool;
use uuid::Uuid;
use chrono::{DateTime, Utc};
use rust_decimal::Decimal;

pub struct RequestLogsRepository {
    pool: PgPool,
}

impl RequestLogsRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn create_with_tags(
        &self,
        api_key_id: Uuid,
        user_id: Uuid,
        organization_id: Uuid,
        team_id: Option<Uuid>,
        model: &str,
        provider: &str,
        provider_model_id: &str,
        capability: &str,
        duration_ms: i32,
        streamed: bool,
        status: &str,
        cost_usd: Option<Decimal>,
        tags: &[String],
    ) -> Result<RequestLog, PasteurError> {
        sqlx::query_as::<_, RequestLog>(
            "INSERT INTO request_logs 
             (api_key_id, user_id, organization_id, team_id, model, provider, provider_model_id, 
              capability, duration_ms, streamed, status, cost_usd, tags) 
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13) 
             RETURNING *",
        )
        .bind(api_key_id)
        .bind(user_id)
        .bind(organization_id)
        .bind(team_id)
        .bind(model)
        .bind(provider)
        .bind(provider_model_id)
        .bind(capability)
        .bind(duration_ms)
        .bind(streamed)
        .bind(status)
        .bind(cost_usd)
        .bind(tags)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| PasteurError::Database(e.to_string()))
    }

    pub async fn find_by_tag(
        &self,
        tag: &str,
        from: Option<DateTime<Utc>>,
        to: Option<DateTime<Utc>>,
        organization_id: Option<Uuid>,
    ) -> Result<Vec<RequestLog>, PasteurError> {
        sqlx::query_as::<_, RequestLog>(
            "SELECT * FROM request_logs 
             WHERE $1 = ANY(tags)
               AND ($2::timestamptz IS NULL OR created_at >= $2)
               AND ($3::timestamptz IS NULL OR created_at <= $3)
               AND ($4::uuid IS NULL OR organization_id = $4)
             ORDER BY created_at DESC",
        )
        .bind(tag)
        .bind(from)
        .bind(to)
        .bind(organization_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| PasteurError::Database(e.to_string()))
    }

    pub async fn aggregate_spend_by_tag(
        &self,
        tag: Option<&str>,
        from: Option<DateTime<Utc>>,
        to: Option<DateTime<Utc>>,
        organization_id: Option<Uuid>,
        user_id: Option<Uuid>,
    ) -> Result<Vec<(String, Decimal)>, PasteurError> {
        let query = if tag.is_some() {
            "SELECT UNNEST(tags) AS tag, COALESCE(SUM(cost_usd), 0) AS spend_usd
             FROM request_logs
             WHERE $1 = ANY(tags)
               AND ($2::timestamptz IS NULL OR created_at >= $2)
               AND ($3::timestamptz IS NULL OR created_at <= $3)
               AND ($4::uuid IS NULL OR organization_id = $4)
               AND ($5::uuid IS NULL OR user_id = $5)
             GROUP BY tag
             ORDER BY spend_usd DESC"
        } else {
            "SELECT UNNEST(tags) AS tag, COALESCE(SUM(cost_usd), 0) AS spend_usd
             FROM request_logs
             WHERE ($1::timestamptz IS NULL OR created_at >= $1)
               AND ($2::timestamptz IS NULL OR created_at <= $2)
               AND ($3::uuid IS NULL OR organization_id = $3)
               AND ($4::uuid IS NULL OR user_id = $4)
             GROUP BY tag
             ORDER BY spend_usd DESC"
        };

        sqlx::query_as::<_, (String, Decimal)>(query)
            .bind(tag)
            .bind(from)
            .bind(to)
            .bind(organization_id)
            .bind(user_id)
            .fetch_all(&self.pool)
            .await
            .map_err(|e| PasteurError::Database(e.to_string()))
    }
}
```

- [ ] **Step 2: Add RequestLog model**

Add to `crates/godwit-db/src/models.rs`:

```rust
#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct RequestLog {
    pub id: Uuid,
    pub api_key_id: Option<Uuid>,
    pub user_id: Option<Uuid>,
    pub organization_id: Uuid,
    pub team_id: Option<Uuid>,
    pub model: String,
    pub provider: String,
    pub provider_model_id: String,
    pub capability: String,
    pub duration_ms: i32,
    pub streamed: bool,
    pub status: String,
    pub cost_usd: Option<Decimal>,
    pub tags: Vec<String>,
    pub created_at: DateTime<Utc>,
}
```

- [ ] **Step 3: Register repository in mod.rs**

```rust
// File: crates/godwit-db/src/repositories/mod.rs
pub mod api_keys;
pub mod end_users;
pub mod models;
pub mod organizations;
pub mod provider_profiles;
pub mod refresh_tokens;
pub mod request_logs;  // Add this line
pub mod team_memberships;
pub mod teams;
pub mod users;
```

- [ ] **Step 4: Verify compilation**

```bash
export PATH="/usr/local/opt/rustup/bin:$PATH"
cargo check -p godwit-db
```

Expected: No errors

- [ ] **Step 5: Commit**

```bash
git add crates/godwit-db/src/repositories/request_logs.rs
git add crates/godwit-db/src/repositories/mod.rs
git add crates/godwit-db/src/models.rs
git commit -m "db: Add RequestLogsRepository with tags support"
```

### Task 6: Update /spend/tags Endpoint for Custom Tags

**Files:**
- Modify: `crates/godwit-api/src/admin/spend_tags.rs`

**Interfaces:**
- Consumes: `RequestLog` model with `tags` field
- Produces: Extended response with `by_custom_tag` field

- [ ] **Step 1: Update query parameters**

```rust
#[derive(Debug, Clone, Deserialize)]
struct SpendTagsQuery {
    from: Option<DateTime<Utc>>,
    to: Option<DateTime<Utc>>,
    organization_id: Option<Uuid>,
    tag: Option<String>,  // Add this field
}
```

- [ ] **Step 2: Update response structure**

```rust
#[derive(Debug, Clone, sqlx::FromRow, serde::Serialize)]
struct CustomTagSpend {
    tag: String,
    spend_usd: Decimal,
}

#[derive(Debug, serde::Serialize)]
struct SpendTagsResponse {
    by_team: Vec<TeamSpend>,
    by_api_key: Vec<ApiKeySpend>,
    by_custom_tag: Vec<CustomTagSpend>,  // Add this field
}
```

- [ ] **Step 3: Add fetch function for custom tag spend**

```rust
async fn fetch_custom_tag_spend(
    pool: &sqlx::PgPool,
    tag: Option<&str>,
    from: Option<DateTime<Utc>>,
    to: Option<DateTime<Utc>>,
    organization_id: Option<Uuid>,
    user_id: Option<Uuid>,
) -> Result<Vec<CustomTagSpend>, sqlx::Error> {
    let query = if tag.is_some() {
        "SELECT UNNEST(tags) AS tag, COALESCE(SUM(cost_usd), 0) AS spend_usd
         FROM request_logs
         WHERE $1 = ANY(tags)
           AND ($2::timestamptz IS NULL OR created_at >= $2)
           AND ($3::timestamptz IS NULL OR created_at <= $3)
           AND ($4::uuid IS NULL OR organization_id = $4)
           AND ($5::uuid IS NULL OR user_id = $5)
         GROUP BY tag
         ORDER BY spend_usd DESC"
    } else {
        "SELECT UNNEST(tags) AS tag, COALESCE(SUM(cost_usd), 0) AS spend_usd
         FROM request_logs
         WHERE tags IS NOT NULL AND array_length(tags, 1) > 0
           AND ($1::timestamptz IS NULL OR created_at >= $1)
           AND ($2::timestamptz IS NULL OR created_at <= $2)
           AND ($3::uuid IS NULL OR organization_id = $3)
           AND ($4::uuid IS NULL OR user_id = $4)
         GROUP BY tag
         ORDER BY spend_usd DESC"
    };

    if let Some(t) = tag {
        sqlx::query_as::<_, CustomTagSpend>(query)
            .bind(t)
            .bind(from)
            .bind(to)
            .bind(organization_id)
            .bind(user_id)
            .fetch_all(pool)
            .await
    } else {
        sqlx::query_as::<_, CustomTagSpend>(query)
            .bind(from)
            .bind(to)
            .bind(organization_id)
            .bind(user_id)
            .fetch_all(pool)
            .await
    }
}
```

- [ ] **Step 4: Update handler to fetch custom tag spend**

```rust
async fn get_spend_tags(
    State(state): State<Arc<AppState>>,
    Extension(claims): Extension<Claims>,
    Query(query): Query<SpendTagsQuery>,
) -> Result<Json<SpendTagsResponse>, ApiError> {
    Role::from_str(&claims.role).ok_or(ApiError::Forbidden)?;
    
    let (organization_id, user_id) = scope_spend_tags_query(&claims, &query);
    
    let by_team = fetch_team_spend(&state.pool, query.from, query.to, organization_id, user_id)
        .await
        .map_err(|e| ApiError::Core(godwit_core::PasteurError::Database(e.to_string())))?;
    
    let by_api_key = fetch_api_key_spend(&state.pool, query.from, query.to, organization_id, user_id)
        .await
        .map_err(|e| ApiError::Core(godwit_core::PasteurError::Database(e.to_string())))?;
    
    let by_custom_tag = fetch_custom_tag_spend(
        &state.pool,
        query.tag.as_deref(),
        query.from,
        query.to,
        organization_id,
        user_id,
    )
    .await
    .map_err(|e| ApiError::Core(godwit_core::PasteurError::Database(e.to_string())))?;
    
    Ok(Json(SpendTagsResponse { by_team, by_api_key, by_custom_tag }))
}
```

- [ ] **Step 5: Verify compilation**

```bash
export PATH="/usr/local/opt/rustup/bin:$PATH"
cargo check -p godwit-api
```

Expected: No errors

- [ ] **Step 6: Commit**

```bash
git add crates/godwit-api/src/admin/spend_tags.rs
git commit -m "api: Extend /spend/tags with by_custom_tag aggregation"
```

### Task 7: Write Tests

**Files:**
- Modify: `crates/godwit-db/src/repositories/request_logs.rs` (add tests)
- Modify: `crates/godwit-api/src/admin/spend_tags.rs` (add tests)

**Interfaces:**
- Consumes: All previous tasks
- Produces: Test coverage for tags functionality

- [ ] **Step 1: Add repository tests**

Add to `crates/godwit-db/src/repositories/request_logs.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use sqlx::PgPool;

    #[sqlx::test(migrations = "../godwit-db/migrations")]
    async fn create_with_tags_round_trips_correctly(pool: PgPool) {
        // Setup: create org, user, api_key
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

        let (_, _, prefix) = godwit_auth::api_keys::generate_api_key();
        let api_key = sqlx::query_scalar::<_, Uuid>(
            "INSERT INTO api_keys (user_id, organization_id, name, key_prefix, key_hash, scopes)
             VALUES ($1, $2, 'test', $3, 'hash', '{chat}') RETURNING id"
        )
        .bind(user)
        .bind(org)
        .bind(&prefix)
        .fetch_one(&pool)
        .await
        .expect("insert api_key");

        let repo = RequestLogsRepository::new(pool);
        let tags = vec!["tag1".to_string(), "tag2".to_string()];
        
        let log = repo
            .create_with_tags(
                api_key, user, org, None, "gpt-4o", "openai", "gpt-4o",
                "chat", 100, false, "success", Some(Decimal::new(123, 2)),
                &tags,
            )
            .await
            .expect("create with tags");

        assert_eq!(log.tags, tags);
    }

    #[sqlx::test(migrations = "../godwit-db/migrations")]
    async fn find_by_tag_filters_correctly(pool: PgPool) {
        // Setup similar to above, insert multiple logs with different tags
        // Test that find_by_tag returns only matching logs
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

        let repo = RequestLogsRepository::new(pool);
        
        // Insert logs with different tags
        sqlx::query(
            "INSERT INTO request_logs 
             (api_key_id, user_id, organization_id, model, provider, provider_model_id, 
              capability, duration_ms, streamed, status, cost_usd, tags)
             VALUES 
             (NULL, $1, $2, 'gpt-4o', 'openai', 'gpt-4o', 'chat', 100, false, 'success', 1.00, $3),
             (NULL, $1, $2, 'gpt-4o', 'openai', 'gpt-4o', 'chat', 100, false, 'success', 2.00, $4)",
        )
        .bind(user)
        .bind(org)
        .bind(vec!["production".to_string()])
        .bind(vec!["development".to_string()])
        .execute(&pool)
        .await
        .expect("insert logs");

        let prod_logs = repo
            .find_by_tag("production", None, None, Some(org))
            .await
            .expect("find by tag");

        assert_eq!(prod_logs.len(), 1);
        assert_eq!(prod_logs[0].tags, vec!["production"]);
    }
}
```

- [ ] **Step 2: Add API endpoint tests**

Add to `crates/godwit-api/src/admin/spend_tags.rs`:

```rust
#[sqlx::test(migrations = "../godwit-db/migrations")]
async fn spend_tags_by_custom_tag_aggregates_correctly(pool: sqlx::PgPool) {
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

    sqlx::query(
        "INSERT INTO request_logs
         (organization_id, user_id, model, provider, provider_model_id,
          cost_usd, duration_ms, streamed, status, tags)
         VALUES
         ($1, $2, 'gpt-4o', 'openai', 'gpt-4o', 1.00, 10, false, 'success', $3),
         ($1, $2, 'gpt-4o', 'openai', 'gpt-4o', 2.00, 20, false, 'success', $3),
         ($1, $2, 'gpt-4o', 'openai', 'gpt-4o', 3.00, 30, false, 'success', $4)",
    )
    .bind(org)
    .bind(user)
    .bind(vec!["production".to_string(), "critical".to_string()])
    .bind(vec!["development".to_string()])
    .execute(&pool)
    .await
    .expect("insert request_logs");

    let result = fetch_custom_tag_spend(&pool, None, None, None, Some(org), Some(user))
        .await
        .expect("fetch custom tag spend");

    // Should have production, critical, development tags
    assert!(result.len() >= 2);
    
    // Find production tag spend (should be 3.00)
    let prod_spend = result.iter().find(|t| t.tag == "production").expect("production tag");
    assert_eq!(prod_spend.spend_usd, Decimal::new(300, 2)); // 1.00 + 2.00
}

#[sqlx::test(migrations = "../godwit-db/migrations")]
async fn spend_tags_with_tag_filter_returns_only_matching(pool: sqlx::PgPool) {
    // Similar setup, test that ?tag=production filters correctly
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

    sqlx::query(
        "INSERT INTO request_logs
         (organization_id, user_id, model, provider, provider_model_id,
          cost_usd, duration_ms, streamed, status, tags)
         VALUES
         ($1, $2, 'gpt-4o', 'openai', 'gpt-4o', 5.00, 10, false, 'success', $3)",
    )
    .bind(org)
    .bind(user)
    .bind(vec!["production".to_string()])
    .execute(&pool)
    .await
    .expect("insert request_logs");

    let result = fetch_custom_tag_spend(&pool, Some("production"), None, None, Some(org), Some(user))
        .await
        .expect("fetch filtered custom tag spend");

    assert_eq!(result.len(), 1);
    assert_eq!(result[0].tag, "production");
    assert_eq!(result[0].spend_usd, Decimal::new(500, 2));
}
```

- [ ] **Step 3: Run tests**

```bash
export PATH="/usr/local/opt/rustup/bin:$PATH"
DATABASE_URL=postgres://user:pass@localhost:5432/godwit cargo test -p godwit-db request_logs
DATABASE_URL=postgres://user:pass@localhost:5432/godwit cargo test -p godwit-api spend_tags
```

Expected: All tests pass

- [ ] **Step 4: Commit**

```bash
git add crates/godwit-db/src/repositories/request_logs.rs
git add crates/godwit-api/src/admin/spend_tags.rs
git commit -m "tests: Add comprehensive tests for tags functionality"
```

### Task 8: Update Documentation

**Files:**
- Modify: `docs/gap-audit/G3-spend-tags-report.md`

**Interfaces:**
- Consumes: All implementation tasks
- Produces: Updated report documenting tags feature

- [ ] **Step 1: Update report with tags feature**

Add section to `docs/gap-audit/G3-spend-tags-report.md`:

```markdown
## Update: Custom Tags Support (2026-08-06)

Added explicit `tags TEXT[]` column to `request_logs` table with the following enhancements:

### Schema Changes
- Column: `tags TEXT[] DEFAULT '{}'`
- Index: `CREATE INDEX idx_request_logs_tags ON request_logs USING GIN(tags)`
- Migration: `20260808000002_request_logs_tags`

### API Changes
- Header: `X-Godwit-Tags: tag1,tag2,tag3` accepted on all proxy endpoints
- Response: `/spend/tags` now includes `by_custom_tag` field

### Repository Methods
- `RequestLogsRepository::create_with_tags()` - Insert log with tags
- `RequestLogsRepository::find_by_tag()` - Query logs by tag
- `RequestLogsRepository::aggregate_spend_by_tag()` - Aggregate spend by custom tags

### Test Coverage
- Repository tests for create/find by tag
- API tests for /spend/tags with custom tag aggregation
- Integration tests for X-Godwit-Tags header parsing
```

- [ ] **Step 2: Verify report is up to date**

```bash
cat docs/gap-audit/G3-spend-tags-report.md
```

- [ ] **Step 3: Commit**

```bash
git add docs/gap-audit/G3-spend-tags-report.md
git commit -m "docs: Update G3 report with custom tags feature"
```

---

## Verification Summary

After all tasks complete:

```bash
# Full workspace compilation
export PATH="/usr/local/opt/rustup/bin:$PATH"
cargo check --workspace --tests

# Run all tests (requires DATABASE_URL)
DATABASE_URL=postgres://user:pass@localhost:5432/godwit cargo test --workspace

# Build binary
cargo build --bin godwit
```

**Expected:**
- ✅ All compilation checks pass
- ✅ All tests pass (unit + integration)
- ✅ Binary builds successfully
- ✅ Migration can be applied and rolled back
