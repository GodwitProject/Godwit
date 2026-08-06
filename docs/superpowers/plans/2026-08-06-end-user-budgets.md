# End-User Budgets Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Implement end-user budget management similar to team budgets (G5.4 deferred), allowing organizations to set spending limits per user.

**Architecture:** Follow existing team budget patterns: database table with budget_usd/max_budget_usd, repository for CRUD operations, API endpoints with RBAC, optional enforcement in middleware.

**Tech Stack:** Rust, SQLx, Axum, PostgreSQL, UUID, rust_decimal

## Global Constraints

- Crate prefix is `godwit_*` (e.g., `godwit_core`, `godwit_db`)
- Use SQLx compile-time checks; set `DATABASE_URL` before any build/test that touches godwit-db
- Follow existing code conventions from teams implementation
- Migration naming: `YYYYMMDDHHMMSS_feature_name.up.sql`
- Code style: no comments unless asked, concise, direct

---

### Task 1: Database Migration

**Files:**
- Create: `crates/godwit-db/migrations/20260807000001_end_users.up.sql`
- Create: `crates/godwit-db/migrations/20260807000001_end_users.down.sql`

- [ ] **Step 1: Create up migration**

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

- [ ] **Step 2: Create down migration**

```sql
DROP TABLE IF EXISTS end_users;
```

- [ ] **Step 3: Verify migration files created**

Run: `ls -la crates/godwit-db/migrations/ | grep end_users`

---

### Task 2: EndUser Model

**Files:**
- Modify: `crates/godwit-db/src/models.rs`

**Interfaces:**
- Produces: `EndUser` struct with fields matching DB schema

- [ ] **Step 1: Add EndUser struct to models.rs**

Add after Team struct (around line 127):

```rust
#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct EndUser {
    pub id: Uuid,
    pub organization_id: Uuid,
    pub user_id: Uuid,
    pub budget_usd: Option<rust_decimal::Decimal>,
    pub max_budget_usd: Option<rust_decimal::Decimal>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}
```

- [ ] **Step 2: Verify models compile**

Run: `export PATH="/usr/local/opt/rustup/bin:$PATH" && cargo check -p godwit-db`

---

### Task 3: EndUsers Repository

**Files:**
- Create: `crates/godwit-db/src/repositories/end_users.rs`
- Modify: `crates/godwit-db/src/repositories/mod.rs`

**Interfaces:**
- Consumes: `EndUser` model from `crate::models`
- Produces: `EndUsersRepository` with methods: create, get_by_user, list_by_org, update_budgets, delete

- [ ] **Step 1: Create repository file**

```rust
use crate::models::EndUser;
use godwit_core::PasteurError;
use rust_decimal::Decimal;
use sqlx::PgPool;
use uuid::Uuid;

pub struct EndUsersRepository {
    pool: PgPool,
}

impl EndUsersRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn create(
        &self,
        organization_id: Uuid,
        user_id: Uuid,
        budget_usd: Option<Decimal>,
        max_budget_usd: Option<Decimal>,
    ) -> Result<EndUser, PasteurError> {
        sqlx::query_as::<_, EndUser>(
            "INSERT INTO end_users (organization_id, user_id, budget_usd, max_budget_usd) 
             VALUES ($1, $2, $3, $4) RETURNING *",
        )
        .bind(organization_id)
        .bind(user_id)
        .bind(budget_usd)
        .bind(max_budget_usd)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| PasteurError::Database(e.to_string()))
    }

    pub async fn get_by_user(
        &self,
        organization_id: Uuid,
        user_id: Uuid,
    ) -> Result<EndUser, PasteurError> {
        sqlx::query_as::<_, EndUser>(
            "SELECT * FROM end_users WHERE organization_id = $1 AND user_id = $2",
        )
        .bind(organization_id)
        .bind(user_id)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| match e {
            sqlx::Error::RowNotFound => PasteurError::NotFound,
            _ => PasteurError::Database(e.to_string()),
        })
    }

    pub async fn list_by_organization(
        &self,
        organization_id: Uuid,
    ) -> Result<Vec<EndUser>, PasteurError> {
        sqlx::query_as::<_, EndUser>(
            "SELECT * FROM end_users WHERE organization_id = $1 ORDER BY user_id",
        )
        .bind(organization_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| PasteurError::Database(e.to_string()))
    }

    pub async fn update_budgets(
        &self,
        organization_id: Uuid,
        user_id: Uuid,
        budget_usd: Option<Decimal>,
        max_budget_usd: Option<Decimal>,
    ) -> Result<EndUser, PasteurError> {
        sqlx::query_as::<_, EndUser>(
            "UPDATE end_users SET budget_usd = $3, max_budget_usd = $4, updated_at = NOW() 
             WHERE organization_id = $1 AND user_id = $2 RETURNING *",
        )
        .bind(organization_id)
        .bind(user_id)
        .bind(budget_usd)
        .bind(max_budget_usd)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| match e {
            sqlx::Error::RowNotFound => PasteurError::NotFound,
            _ => PasteurError::Database(e.to_string()),
        })
    }

    pub async fn delete(
        &self,
        organization_id: Uuid,
        user_id: Uuid,
    ) -> Result<(), PasteurError> {
        sqlx::query("DELETE FROM end_users WHERE organization_id = $1 AND user_id = $2")
            .bind(organization_id)
            .bind(user_id)
            .execute(&self.pool)
            .await
            .map_err(|e| PasteurError::Database(e.to_string()))?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::repositories::organizations::OrganizationRepository;
    use crate::repositories::users::UserRepository;
    use sqlx::PgPool;

    #[sqlx::test]
    async fn create_and_get_end_user(pool: PgPool) {
        let orgs = OrganizationRepository::new(pool.clone());
        let org = orgs.create("acme", None).await.expect("create org");
        
        let users = UserRepository::new(pool.clone());
        let user = users.create(org.id, "user@example.com", None, "user", None, None)
            .await.expect("create user");

        let repo = EndUsersRepository::new(pool);
        let end_user = repo.create(org.id, user.id, None, None).await.expect("create end_user");
        assert_eq!(end_user.organization_id, org.id);
        assert_eq!(end_user.user_id, user.id);

        let fetched = repo.get_by_user(org.id, user.id).await.expect("get by user");
        assert_eq!(fetched.id, end_user.id);
    }

    #[sqlx::test]
    async fn list_by_organization(pool: PgPool) {
        let orgs = OrganizationRepository::new(pool.clone());
        let org = orgs.create("acme", None).await.expect("create org");
        
        let users = UserRepository::new(pool.clone());
        let user1 = users.create(org.id, "user1@example.com", None, "user", None, None)
            .await.expect("create user1");
        let user2 = users.create(org.id, "user2@example.com", None, "user", None, None)
            .await.expect("create user2");

        let repo = EndUsersRepository::new(pool);
        repo.create(org.id, user1.id, None, None).await.expect("create end_user1");
        repo.create(org.id, user2.id, None, None).await.expect("create end_user2");

        let listed = repo.list_by_organization(org.id).await.expect("list");
        assert_eq!(listed.len(), 2);
    }

    #[sqlx::test]
    async fn update_budgets(pool: PgPool) {
        let orgs = OrganizationRepository::new(pool.clone());
        let org = orgs.create("acme", None).await.expect("create org");
        
        let users = UserRepository::new(pool.clone());
        let user = users.create(org.id, "user@example.com", None, "user", None, None)
            .await.expect("create user");

        let repo = EndUsersRepository::new(pool);
        let end_user = repo.create(org.id, user.id, None, None).await.expect("create end_user");

        let budget = rust_decimal::Decimal::from_str("100.00").unwrap();
        let max_budget = rust_decimal::Decimal::from_str("200.00").unwrap();
        let updated = repo.update_budgets(org.id, user.id, Some(budget), Some(max_budget))
            .await.expect("update budgets");
        
        assert_eq!(updated.budget_usd, Some(budget));
        assert_eq!(updated.max_budget_usd, Some(max_budget));
    }
}
```

- [ ] **Step 2: Update mod.rs to export repository**

Modify `crates/godwit-db/src/repositories/mod.rs`:

```rust
pub mod api_keys;
pub mod end_users;  // Add this line
pub mod models;
pub mod organizations;
pub mod provider_profiles;
pub mod refresh_tokens;
pub mod team_memberships;
pub mod teams;
pub mod users;
```

- [ ] **Step 3: Verify repository compiles**

Run: `export PATH="/usr/local/opt/rustup/bin:$PATH" && cargo check -p godwit-db`

- [ ] **Step 4: Run repository tests (requires DATABASE_URL)**

Run: `export PATH="/usr/local/opt/rustup/bin:$PATH" && DATABASE_URL=postgres://user:pass@localhost:5432/godwit cargo test -p godwit-db repositories::end_users::tests`

---

### Task 4: API Endpoints

**Files:**
- Create: `crates/godwit-api/src/admin/end_users.rs`
- Modify: `crates/godwit-api/src/admin/mod.rs`

**Interfaces:**
- Consumes: `EndUsersRepository` from `godwit_db::repositories`
- Produces: Router with endpoints: GET /api/v1/end-users, POST /api/v1/end-users, GET /api/v1/end-users/:user_id, PATCH /api/v1/end-users/:user_id, DELETE /api/v1/end-users/:user_id

- [ ] **Step 1: Create API endpoints file**

```rust
use axum::{
    extract::{Extension, Path, Query, State},
    routing::{delete, get, patch, post},
    Json, Router,
};
use godwit_auth::{jwt::Claims, rbac::Role};
use godwit_db::repositories::end_users::EndUsersRepository;
use serde::Deserialize;
use std::sync::Arc;
use uuid::Uuid;

use crate::{error::ApiError, state::AppState};

pub fn router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/end-users", get(list_end_users).post(create_end_user))
        .route(
            "/end-users/:user_id",
            get(get_end_user).patch(update_end_user).delete(delete_end_user),
        )
}

fn require_manage_users(claims: &Claims) -> Result<Role, ApiError> {
    let role = Role::from_str(&claims.role).ok_or(ApiError::Forbidden)?;
    if !role.can_manage_users() {
        return Err(ApiError::Forbidden);
    }
    Ok(role)
}

#[derive(Deserialize)]
pub struct ListEndUsersQuery {
    organization_id: Option<Uuid>,
}

async fn list_end_users(
    State(state): State<Arc<AppState>>,
    Extension(claims): Extension<Claims>,
    Query(query): Query<ListEndUsersQuery>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let role = require_manage_users(&claims)?;
    let repo = EndUsersRepository::new(state.pool.clone());
    
    let end_users = if role == Role::SuperAdmin {
        match query.organization_id {
            Some(org_id) => repo.list_by_organization(org_id).await,
            None => repo.list_by_organization(claims.organization_id).await,
        }
    } else {
        repo.list_by_organization(claims.organization_id).await
    }
    .map_err(ApiError::Core)?;
    
    Ok(Json(serde_json::json!({ "data": end_users })))
}

async fn get_end_user(
    State(state): State<Arc<AppState>>,
    Extension(claims): Extension<Claims>,
    Path(user_id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let role = require_manage_users(&claims)?;
    let repo = EndUsersRepository::new(state.pool.clone());
    
    let organization_id = if role == Role::SuperAdmin {
        // Super admin can query any org's end-user
        // For simplicity, use claims org_id unless specified in query
        claims.organization_id
    } else {
        claims.organization_id
    };
    
    let end_user = repo
        .get_by_user(organization_id, user_id)
        .await
        .map_err(ApiError::Core)?;
    
    Ok(Json(serde_json::json!({ "data": end_user })))
}

#[derive(Deserialize)]
pub struct CreateEndUserRequest {
    user_id: Uuid,
    organization_id: Option<Uuid>,
    budget_usd: Option<rust_decimal::Decimal>,
    max_budget_usd: Option<rust_decimal::Decimal>,
}

async fn create_end_user(
    State(state): State<Arc<AppState>>,
    Extension(claims): Extension<Claims>,
    Json(req): Json<CreateEndUserRequest>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let role = require_manage_users(&claims)?;
    let repo = EndUsersRepository::new(state.pool.clone());
    
    let organization_id = if role == Role::SuperAdmin {
        req.organization_id
            .ok_or_else(|| ApiError::BadRequest("organization_id is required".to_string()))?
    } else {
        claims.organization_id
    };
    
    let end_user = repo
        .create(organization_id, req.user_id, req.budget_usd, req.max_budget_usd)
        .await
        .map_err(ApiError::Core)?;
    
    Ok(Json(serde_json::json!({ "data": end_user })))
}

#[derive(Deserialize)]
pub struct UpdateEndUserRequest {
    budget_usd: Option<rust_decimal::Decimal>,
    max_budget_usd: Option<rust_decimal::Decimal>,
}

async fn update_end_user(
    State(state): State<Arc<AppState>>,
    Extension(claims): Extension<Claims>,
    Path(user_id): Path<Uuid>,
    Json(req): Json<UpdateEndUserRequest>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let role = require_manage_users(&claims)?;
    let repo = EndUsersRepository::new(state.pool.clone());
    
    let organization_id = claims.organization_id;
    
    // Check if end_user exists
    let end_user = repo
        .get_by_user(organization_id, user_id)
        .await
        .map_err(ApiError::Core)?;
    
    // If not super admin, verify org match
    if role != Role::SuperAdmin && end_user.organization_id != claims.organization_id {
        return Err(ApiError::Forbidden);
    }
    
    let updated = repo
        .update_budgets(organization_id, user_id, req.budget_usd, req.max_budget_usd)
        .await
        .map_err(ApiError::Core)?;
    
    Ok(Json(serde_json::json!({ "data": updated })))
}

async fn delete_end_user(
    State(state): State<Arc<AppState>>,
    Extension(claims): Extension<Claims>,
    Path(user_id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let role = require_manage_users(&claims)?;
    let repo = EndUsersRepository::new(state.pool.clone());
    
    let organization_id = claims.organization_id;
    
    // Check if end_user exists
    let end_user = repo
        .get_by_user(organization_id, user_id)
        .await
        .map_err(ApiError::Core)?;
    
    // If not super admin, verify org match
    if role != Role::SuperAdmin && end_user.organization_id != claims.organization_id {
        return Err(ApiError::Forbidden);
    }
    
    repo.delete(organization_id, user_id)
        .await
        .map_err(ApiError::Core)?;
    
    Ok(Json(serde_json::json!({ "deleted": true })))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn create_end_user_request_deserializes_without_organization_id() {
        let json = r#"{"user_id":"550e8400-e29b-41d4-a716-446655440000"}"#;
        let req: CreateEndUserRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.user_id.to_string(), "550e8400-e29b-41d4-a716-446655440000");
        assert_eq!(req.organization_id, None);
    }
}
```

- [ ] **Step 2: Update admin/mod.rs to include end_users router**

Modify `crates/godwit-api/src/admin/mod.rs`:

```rust
pub mod api_keys;
pub mod auth;
pub mod end_users;  // Add this line
pub mod models;
pub mod organizations;
pub mod provider_profiles;
pub mod spend;
pub mod spend_logs;
pub mod stats;
pub mod teams;
pub mod users;
```

And in the router function, add:
```rust
.merge(end_users::router())
```

- [ ] **Step 3: Verify API compiles**

Run: `export PATH="/usr/local/opt/rustup/bin:$PATH" && cargo check -p godwit-api`

- [ ] **Step 4: Run API tests**

Run: `export PATH="/usr/local/opt/rustup/bin:$PATH" && cargo test -p godwit-api admin::end_users::tests`

---

### Task 5: Update AppState

**Files:**
- Modify: `crates/godwit-api/src/state.rs`

**Interfaces:**
- Produces: `end_user_repo: EndUsersRepository` field in AppState

- [ ] **Step 1: Check current AppState structure**

Read: `crates/godwit-api/src/state.rs`

- [ ] **Step 2: Add end_user_repo to AppState**

Add field similar to team_repo:
```rust
pub end_user_repo: EndUsersRepository,
```

- [ ] **Step 3: Initialize in AppState::new**

Add initialization:
```rust
end_user_repo: EndUsersRepository::new(pool.clone()),
```

- [ ] **Step 4: Add import**

```rust
use godwit_db::repositories::end_users::EndUsersRepository;
```

- [ ] **Step 5: Verify compiles**

Run: `export PATH="/usr/local/opt/rustup/bin:$PATH" && cargo check -p godwit-api`

---

### Task 6: Budget Enforcement (Optional)

**Files:**
- Modify: `crates/godwit-api/src/middleware/mod.rs` or `crates/godwit-api/src/middleware/rate_limit.rs`

**Interfaces:**
- Consumes: `EndUsersRepository` to check budget before allowing requests

- [ ] **Step 1: Review existing middleware**

Read: `crates/godwit-api/src/middleware/` files to understand rate limiting structure

- [ ] **Step 2: Add budget check logic**

In the appropriate middleware (likely where rate limiting or spend tracking occurs), add:

```rust
// Check end-user budget if user_id is present
if let Some(user_id) = get_user_id_from_request(...) {
    if let Ok(end_user) = state.end_user_repo.get_by_user(org_id, user_id).await {
        if let Some(budget) = end_user.budget_usd {
            // Compare against spent amount
            // Return 429 or 402 if exceeded
        }
    }
}
```

- [ ] **Step 3: Verify compiles**

Run: `export PATH="/usr/local/opt/rustup/bin:$PATH" && cargo check -p godwit-api`

---

### Task 7: Documentation

**Files:**
- Create: `docs/end-user-budgets.md`

- [ ] **Step 1: Create documentation file**

```markdown
# End-User Budgets

## Overview

End-user budgets allow organizations to set spending limits per user, similar to team budgets (G5).

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
```

## API Endpoints

### List End-Users
```
GET /api/v1/end-users?organization_id={uuid}
```

### Create End-User Budget
```
POST /api/v1/end-users
Content-Type: application/json

{
  "user_id": "uuid",
  "organization_id": "uuid",  // optional, defaults to caller's org
  "budget_usd": 100.00,
  "max_budget_usd": 200.00
}
```

### Get End-User Budget
```
GET /api/v1/end-users/:user_id
```

### Update End-User Budget
```
PATCH /api/v1/end-users/:user_id
Content-Type: application/json

{
  "budget_usd": 150.00,
  "max_budget_usd": 250.00
}
```

### Delete End-User Budget
```
DELETE /api/v1/end-users/:user_id
```

## Authorization

- `super_admin`: Can manage end-users across all organizations
- `org_admin`: Can manage end-users within their organization
- Other roles: No access

## Budget Enforcement

Budget enforcement is checked during API requests. If a user's spent amount exceeds their `budget_usd`, the request is rejected with HTTP 429 (Too Many Requests) or 402 (Payment Required).

## Implementation Files

- Migration: `crates/godwit-db/migrations/20260807000001_end_users.up.sql`
- Model: `crates/godwit-db/src/models.rs:EndUser`
- Repository: `crates/godwit-db/src/repositories/end_users.rs`
- API: `crates/godwit-api/src/admin/end_users.rs`
```

- [ ] **Step 2: Verify documentation is complete**

---

### Task 8: Final Verification

- [ ] **Step 1: Full workspace compile check**

Run: `export PATH="/usr/local/opt/rustup/bin:$PATH" && cargo check --workspace --tests`

- [ ] **Step 2: Run all godwit-db tests**

Run: `export PATH="/usr/local/opt/rustup/bin:$PATH" && DATABASE_URL=postgres://user:pass@localhost:5432/godwit cargo test -p godwit-db`

- [ ] **Step 3: Run all godwit-api tests**

Run: `export PATH="/usr/local/opt/rustup/bin:$PATH" && DATABASE_URL=postgres://user:pass@localhost:5432/godwit cargo test -p godwit-api`

- [ ] **Step 4: Create git commit**

```bash
git add .
git commit -m "feat: implement end-user budgets (G5.4 deferred)

- Add end_users table with budget_usd/max_budget_usd
- Add EndUser model and EndUsersRepository
- Add REST endpoints: GET/POST/PATCH/DELETE /api/v1/end-users
- Add repository tests and API unit tests
- Document in docs/end-user-budgets.md"
```

---

## Test Summary

Expected test counts:
- godwit-db repository tests: 3 tests (create_and_get, list_by_organization, update_budgets)
- godwit-api unit tests: 1 test (deserialize without organization_id)
- Integration tests: Manual or with running server
