# Admin Backend Completion Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Turn every stubbed admin endpoint (`teams`, `users/:id`, `spend`) into a real implementation, add organization create/update and team-membership management, add a refresh-token flow, and fix the double-nested admin routes this plan touches — all as the backend prerequisite for a future admin web UI.

**Architecture:** Same layering as the rest of the codebase: `godwit-db` repositories for persistence, `godwit-auth` for token generation, `godwit-api/src/admin/*` for HTTP handlers gated by `godwit_auth::rbac::Role`. A new global-vs-org-scoped RBAC convention is introduced: `super_admin` becomes unrestricted (optionally filterable by `organization_id`) on `users`/`teams`/`spend`, matching its existing global scope on `organizations`.

**Tech Stack:** Rust, Axum, SQLx/PostgreSQL, `sha2`/`hex` (new deps, for refresh-token hashing — argon2 doesn't support the equality lookup a refresh token needs, see Task 2).

## Global Constraints

- Spec: `docs/superpowers/specs/2026-08-03-admin-backend-completion-design.md` — every task traces to a section there.
- RBAC scoping model (spec §4): `super_admin` optional `organization_id` filter (omitted = all orgs); `org_admin` always forced to `claims.organization_id`; `team_admin`/`user` (spend only) always forced to `user_id = claims.user_id`.
- Role gates for `teams`/`users` endpoints stay `super_admin`/`org_admin` only (`Role::can_manage_users()`), unchanged from today — only the *scope within* that gate changes.
- Access-token lifetime stays 15 minutes; only a renewal mechanism is added.
- Toolchain: `export PATH="/usr/local/opt/rustup/bin:$PATH"` before any cargo command. DB tests need `DATABASE_URL="postgres://tmenard@localhost:5432/godwit"`.
- Follow existing conventions exactly: `PasteurError`/`ApiError::Core` for domain errors, `sqlx::query_as::<_, T>(...).map_err(|e| PasteurError::Database(e.to_string()))`, `RowNotFound => PasteurError::NotFound` on single-row fetches, `Role::from_str(&claims.role).ok_or(ApiError::Forbidden)?` for RBAC checks (see `crates/godwit-api/src/admin/organizations.rs` and `crates/godwit-api/tests/router_integration.rs` for the canonical patterns).

---

## Task 1: `refresh_tokens` Table & Repository

**Files:**
- Create: `crates/godwit-db/migrations/20260803000004_refresh_tokens.up.sql`
- Create: `crates/godwit-db/migrations/20260803000004_refresh_tokens.down.sql`
- Modify: `crates/godwit-db/src/models.rs` (add `RefreshToken`)
- Create: `crates/godwit-db/src/repositories/refresh_tokens.rs`
- Modify: `crates/godwit-db/src/repositories/mod.rs`

**Interfaces:**
- Produces: `godwit_db::models::RefreshToken { id: Uuid, user_id: Uuid, token_hash: String, expires_at: DateTime<Utc>, created_at: DateTime<Utc> }`, `RefreshTokenRepository::{new(pool), create(user_id, token_hash, expires_at) -> RefreshToken, get_by_hash(token_hash) -> RefreshToken, delete(id) -> (), delete_by_hash(token_hash) -> ()}`.

- [ ] **Step 1: Write the failing test**

Create `crates/godwit-db/src/repositories/refresh_tokens.rs` with tests first:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::repositories::users::UserRepository;
    use godwit_db::models::UserRole;
    use sqlx::PgPool;

    #[sqlx::test]
    async fn create_and_get_by_hash(pool: PgPool) {
        let users = UserRepository::new(pool.clone());
        let user = users
            .create("alice@example.com", None, UserRole::User, None)
            .await
            .expect("create user");

        let repo = RefreshTokenRepository::new(pool);
        let expires_at = chrono::Utc::now() + chrono::Duration::days(7);
        let created = repo
            .create(user.id, "hash-abc", expires_at)
            .await
            .expect("create refresh token");
        assert_eq!(created.user_id, user.id);
        assert_eq!(created.token_hash, "hash-abc");

        let fetched = repo.get_by_hash("hash-abc").await.expect("get by hash");
        assert_eq!(fetched.id, created.id);
    }

    #[sqlx::test]
    async fn get_by_hash_not_found(pool: PgPool) {
        let repo = RefreshTokenRepository::new(pool);
        let err = repo.get_by_hash("missing").await.unwrap_err();
        assert!(matches!(err, godwit_core::PasteurError::NotFound));
    }

    #[sqlx::test]
    async fn delete_by_hash_removes_it(pool: PgPool) {
        let users = UserRepository::new(pool.clone());
        let user = users
            .create("bob@example.com", None, UserRole::User, None)
            .await
            .expect("create user");
        let repo = RefreshTokenRepository::new(pool);
        let expires_at = chrono::Utc::now() + chrono::Duration::days(7);
        repo.create(user.id, "hash-to-delete", expires_at)
            .await
            .expect("create refresh token");

        repo.delete_by_hash("hash-to-delete").await.expect("delete");
        let err = repo.get_by_hash("hash-to-delete").await.unwrap_err();
        assert!(matches!(err, godwit_core::PasteurError::NotFound));
    }

    #[sqlx::test]
    async fn deleting_user_cascades_refresh_tokens(pool: PgPool) {
        let users = UserRepository::new(pool.clone());
        let user = users
            .create("carol@example.com", None, UserRole::User, None)
            .await
            .expect("create user");
        let repo = RefreshTokenRepository::new(pool.clone());
        let expires_at = chrono::Utc::now() + chrono::Duration::days(7);
        repo.create(user.id, "hash-cascade", expires_at)
            .await
            .expect("create refresh token");

        sqlx::query("DELETE FROM users WHERE id = $1")
            .bind(user.id)
            .execute(&pool)
            .await
            .expect("delete user");

        let err = repo.get_by_hash("hash-cascade").await.unwrap_err();
        assert!(matches!(err, godwit_core::PasteurError::NotFound));
    }
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `DATABASE_URL="postgres://tmenard@localhost:5432/godwit" cargo test -p godwit-db refresh_tokens`
Expected: FAIL to compile — no `refresh_tokens` table, no `RefreshTokenRepository`, no `RefreshToken` model.

- [ ] **Step 3: Write the migration**

Create `crates/godwit-db/migrations/20260803000004_refresh_tokens.up.sql`:

```sql
CREATE TABLE refresh_tokens (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    token_hash TEXT NOT NULL UNIQUE,
    expires_at TIMESTAMPTZ NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_refresh_tokens_token_hash ON refresh_tokens(token_hash);
```

Create `crates/godwit-db/migrations/20260803000004_refresh_tokens.down.sql`:

```sql
DROP TABLE refresh_tokens;
```

- [ ] **Step 4: Add the `RefreshToken` model**

In `crates/godwit-db/src/models.rs`, add:

```rust
#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct RefreshToken {
    pub id: Uuid,
    pub user_id: Uuid,
    pub token_hash: String,
    pub expires_at: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
}
```

- [ ] **Step 5: Implement `RefreshTokenRepository`**

Prepend to `crates/godwit-db/src/repositories/refresh_tokens.rs` (above the test module written in Step 1):

```rust
use crate::models::RefreshToken;
use chrono::{DateTime, Utc};
use godwit_core::PasteurError;
use sqlx::PgPool;
use uuid::Uuid;

pub struct RefreshTokenRepository {
    pool: PgPool,
}

impl RefreshTokenRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn create(
        &self,
        user_id: Uuid,
        token_hash: &str,
        expires_at: DateTime<Utc>,
    ) -> Result<RefreshToken, PasteurError> {
        sqlx::query_as::<_, RefreshToken>(
            "INSERT INTO refresh_tokens (user_id, token_hash, expires_at) VALUES ($1, $2, $3) RETURNING *"
        )
        .bind(user_id)
        .bind(token_hash)
        .bind(expires_at)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| PasteurError::Database(e.to_string()))
    }

    pub async fn get_by_hash(&self, token_hash: &str) -> Result<RefreshToken, PasteurError> {
        sqlx::query_as::<_, RefreshToken>("SELECT * FROM refresh_tokens WHERE token_hash = $1")
            .bind(token_hash)
            .fetch_one(&self.pool)
            .await
            .map_err(|e| match e {
                sqlx::Error::RowNotFound => PasteurError::NotFound,
                _ => PasteurError::Database(e.to_string()),
            })
    }

    pub async fn delete(&self, id: Uuid) -> Result<(), PasteurError> {
        sqlx::query("DELETE FROM refresh_tokens WHERE id = $1")
            .bind(id)
            .execute(&self.pool)
            .await
            .map_err(|e| PasteurError::Database(e.to_string()))?;
        Ok(())
    }

    pub async fn delete_by_hash(&self, token_hash: &str) -> Result<(), PasteurError> {
        sqlx::query("DELETE FROM refresh_tokens WHERE token_hash = $1")
            .bind(token_hash)
            .execute(&self.pool)
            .await
            .map_err(|e| PasteurError::Database(e.to_string()))?;
        Ok(())
    }
}
```

- [ ] **Step 6: Register the module**

Modify `crates/godwit-db/src/repositories/mod.rs`:

```rust
pub mod api_keys;
pub mod models;
pub mod organizations;
pub mod provider_profiles;
pub mod refresh_tokens;
pub mod teams;
pub mod team_memberships;
pub mod users;
```

(This adds `teams`/`team_memberships` module declarations now, ahead of Tasks 5-6 creating those files — harmless: an unresolved-module error would only appear once you build, and Tasks 5-6 land before anyone builds this crate again as part of this plan's sequential execution. If you're executing tasks out of order, only add `refresh_tokens` here for now and let Tasks 5-6 add their own lines.)

- [ ] **Step 7: Run tests to verify they pass**

Run: `DATABASE_URL="postgres://tmenard@localhost:5432/godwit" cargo test -p godwit-db refresh_tokens`
Expected: PASS (all 4 tests).

- [ ] **Step 8: Commit**

```bash
git add crates/godwit-db/migrations crates/godwit-db/src/models.rs crates/godwit-db/src/repositories/refresh_tokens.rs crates/godwit-db/src/repositories/mod.rs
git commit -m "feat(db): refresh_tokens table and repository"
```

---

## Task 2: Refresh Token Generation (`godwit-auth`)

**Files:**
- Modify: `crates/godwit-auth/Cargo.toml` (add `sha2`, `hex`)
- Create: `crates/godwit-auth/src/refresh_tokens.rs`
- Modify: `crates/godwit-auth/src/lib.rs`

**Interfaces:**
- Produces: `godwit_auth::refresh_tokens::{generate_refresh_token() -> (String, String), hash_refresh_token(token: &str) -> String}`.

**Why not reuse `api_keys.rs`'s Argon2 pattern:** Argon2 is salted and non-deterministic — hashing the same plaintext twice produces two different strings, so there's no way to look a token up by re-hashing it and comparing to a stored value (which is exactly what `POST /auth/refresh` needs to do). Argon2 makes sense for passwords and API keys (low iteration cost is fine, and `api_keys.rs` works around the lookup problem with a separate `key_prefix` column plus `get_by_prefix` + verify-each-candidate). A refresh token is already a 256-bit random value — it doesn't need slow hashing to resist brute force, and a fast deterministic digest (SHA-256) enables a direct equality lookup by `token_hash`, matching the schema built in Task 1.

- [ ] **Step 1: Write the failing test**

Create `crates/godwit-auth/src/refresh_tokens.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generate_produces_hashable_token() {
        let (plaintext, hash) = generate_refresh_token();
        assert!(!plaintext.is_empty());
        assert_eq!(hash_refresh_token(&plaintext), hash);
    }

    #[test]
    fn different_tokens_hash_differently() {
        let (_, hash_a) = generate_refresh_token();
        let (_, hash_b) = generate_refresh_token();
        assert_ne!(hash_a, hash_b);
    }

    #[test]
    fn hash_is_deterministic() {
        let (plaintext, _) = generate_refresh_token();
        assert_eq!(hash_refresh_token(&plaintext), hash_refresh_token(&plaintext));
    }
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p godwit-auth refresh_tokens`
Expected: FAIL — `cannot find function 'generate_refresh_token'`.

- [ ] **Step 3: Add dependencies**

Modify `crates/godwit-auth/Cargo.toml`, add to `[dependencies]`:

```toml
sha2 = "0.10"
hex = "0.4"
```

- [ ] **Step 4: Implement the module**

Prepend to `crates/godwit-auth/src/refresh_tokens.rs` (above the test module from Step 1):

```rust
use rand::rngs::OsRng;
use rand::RngCore;
use sha2::{Digest, Sha256};

pub fn generate_refresh_token() -> (String, String) {
    let mut bytes = [0u8; 32];
    OsRng.fill_bytes(&mut bytes);
    let plaintext = bs58::encode(&bytes).into_string();
    let hash = hash_refresh_token(&plaintext);
    (plaintext, hash)
}

pub fn hash_refresh_token(token: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(token.as_bytes());
    hex::encode(hasher.finalize())
}
```

- [ ] **Step 5: Wire up lib.rs**

Modify `crates/godwit-auth/src/lib.rs`:

```rust
pub mod api_keys;
pub mod credentials;
pub mod jwt;
pub mod oidc;
pub mod rbac;
pub mod refresh_tokens;
pub mod saml;
```

- [ ] **Step 6: Run tests to verify they pass**

Run: `cargo test -p godwit-auth refresh_tokens`
Expected: PASS (all 3 tests).

- [ ] **Step 7: Commit**

```bash
git add crates/godwit-auth/Cargo.toml crates/godwit-auth/src/refresh_tokens.rs crates/godwit-auth/src/lib.rs Cargo.lock
git commit -m "feat(auth): refresh token generation (SHA-256, deterministic lookup)"
```

---

## Task 3: Wire Refresh Tokens Into Auth Endpoints

**Files:**
- Modify: `crates/godwit-api/src/state.rs` (add `refresh_token_repo`)
- Modify: `crates/godwit-api/src/admin/auth.rs` (login/oidc share a helper; new refresh/logout endpoints)
- Modify: `crates/godwit-bin/src/main.rs` (wire the new repo)
- Modify: `crates/godwit-api/tests/router_integration.rs` (wire the new repo in `build_app`)

**Interfaces:**
- Consumes: `RefreshTokenRepository` (Task 1), `godwit_auth::refresh_tokens::{generate_refresh_token, hash_refresh_token}` (Task 2).
- Produces: `POST /auth/refresh` (body `{ refresh_token }` → `{ access_token, refresh_token }`), `POST /auth/logout` (body `{ refresh_token }` → `{ logged_out: true }`). `POST /auth/login` and the OIDC callback now return `{ access_token, refresh_token }` instead of just `access_token`.

- [ ] **Step 1: Write the failing test**

Add to `crates/godwit-api/src/admin/auth.rs`'s existing `#[cfg(test)] mod tests` block:

```rust
    #[test]
    fn refresh_request_deserializes() {
        let json = r#"{"refresh_token":"abc123"}"#;
        let req: RefreshRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.refresh_token, "abc123");
    }

    #[test]
    fn logout_request_deserializes() {
        let json = r#"{"refresh_token":"abc123"}"#;
        let req: LogoutRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.refresh_token, "abc123");
    }
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p godwit-api --lib admin::auth::tests`
Expected: FAIL to compile — `RefreshRequest`/`LogoutRequest` not found.

- [ ] **Step 3: Add `refresh_token_repo` to `AppState`**

Modify `crates/godwit-api/src/state.rs`:

```rust
use crate::model_router::DbModelRouter;
use godwit_cache::MemoryCache;
use godwit_core::AppConfig;
use godwit_db::models::ApiKey;
use godwit_db::repositories::{
    api_keys::ApiKeyRepository, organizations::OrganizationRepository,
    refresh_tokens::RefreshTokenRepository, users::UserRepository,
};
use godwit_providers::AdapterRegistry;
use sqlx::PgPool;
use std::sync::Arc;

pub struct AppState {
    pub config: AppConfig,
    pub pool: PgPool,
    pub adapter_registry: Arc<AdapterRegistry>,
    pub model_router: DbModelRouter,
    pub user_repo: UserRepository,
    pub org_repo: OrganizationRepository,
    pub api_key_repo: ApiKeyRepository,
    pub refresh_token_repo: RefreshTokenRepository,
    pub api_key_cache: MemoryCache<String, ApiKey>,
    pub credential_master_key: [u8; 32],
}
```

- [ ] **Step 4: Rewrite `auth.rs` to share token issuance and add refresh/logout**

Replace the whole non-test portion of `crates/godwit-api/src/admin/auth.rs`:

```rust
use axum::{
    extract::{Path, Query, State},
    response::{IntoResponse, Redirect, Response},
    routing::{get, post},
    Json, Router,
};
use godwit_auth::{
    api_keys::verify_password,
    jwt::{issue, Claims},
    refresh_tokens::{generate_refresh_token, hash_refresh_token},
};
use godwit_db::models::User;
use serde::Deserialize;
use std::sync::Arc;

use crate::state::AppState;

#[derive(Deserialize)]
pub struct LoginRequest {
    email: String,
    password: String,
}

#[derive(Deserialize)]
pub struct OidcCallback {
    code: String,
    state: String,
}

#[derive(Deserialize)]
pub struct RefreshRequest {
    refresh_token: String,
}

#[derive(Deserialize)]
pub struct LogoutRequest {
    refresh_token: String,
}

pub fn router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/auth/login", post(login))
        .route("/auth/refresh", post(refresh))
        .route("/auth/logout", post(logout))
        .route("/auth/oidc/:provider", get(oidc_start))
        .route("/auth/oidc/:provider/callback", get(oidc_callback))
        .route("/auth/saml/:provider/acs", post(saml_acs))
}

/// Issues a fresh access token + refresh token pair for `user`, persisting the refresh
/// token's hash. Shared by login, the OIDC callback, and `/auth/refresh` so all three
/// issue tokens identically.
async fn issue_token_pair(
    state: &AppState,
    user: &User,
) -> Result<serde_json::Value, crate::error::ApiError> {
    let claims = Claims::new(user.id, user.organization_id.unwrap_or_default(), &user.role);
    let access_token = issue(
        &state.config.auth.jwt_secret,
        claims,
        chrono::Duration::minutes(state.config.auth.access_token_ttl_minutes),
    )
    .map_err(|_| crate::error::ApiError::Internal)?;

    let (refresh_plaintext, refresh_hash) = generate_refresh_token();
    let expires_at =
        chrono::Utc::now() + chrono::Duration::days(state.config.auth.refresh_token_ttl_days);
    state
        .refresh_token_repo
        .create(user.id, &refresh_hash, expires_at)
        .await
        .map_err(crate::error::ApiError::Core)?;

    Ok(serde_json::json!({ "access_token": access_token, "refresh_token": refresh_plaintext }))
}

async fn login(
    State(state): State<Arc<AppState>>,
    Json(req): Json<LoginRequest>,
) -> Result<impl IntoResponse, crate::error::ApiError> {
    let user = state
        .user_repo
        .get_by_email(&req.email)
        .await
        .map_err(|_| crate::error::ApiError::Unauthorized)?;
    let password_hash = user
        .password_hash
        .as_deref()
        .ok_or(crate::error::ApiError::Unauthorized)?;
    if !verify_password(&req.password, password_hash) {
        return Err(crate::error::ApiError::Unauthorized);
    }
    Ok(Json(issue_token_pair(&state, &user).await?))
}

async fn refresh(
    State(state): State<Arc<AppState>>,
    Json(req): Json<RefreshRequest>,
) -> Result<impl IntoResponse, crate::error::ApiError> {
    let hash = hash_refresh_token(&req.refresh_token);
    let stored = state
        .refresh_token_repo
        .get_by_hash(&hash)
        .await
        .map_err(|_| crate::error::ApiError::Unauthorized)?;
    if stored.expires_at < chrono::Utc::now() {
        let _ = state.refresh_token_repo.delete(stored.id).await;
        return Err(crate::error::ApiError::Unauthorized);
    }
    let user = state
        .user_repo
        .get_by_id(stored.user_id)
        .await
        .map_err(|_| crate::error::ApiError::Unauthorized)?;
    // Rotate: the used refresh token is single-use.
    state
        .refresh_token_repo
        .delete(stored.id)
        .await
        .map_err(crate::error::ApiError::Core)?;
    Ok(Json(issue_token_pair(&state, &user).await?))
}

async fn logout(
    State(state): State<Arc<AppState>>,
    Json(req): Json<LogoutRequest>,
) -> Result<impl IntoResponse, crate::error::ApiError> {
    let hash = hash_refresh_token(&req.refresh_token);
    state
        .refresh_token_repo
        .delete_by_hash(&hash)
        .await
        .map_err(crate::error::ApiError::Core)?;
    Ok(Json(serde_json::json!({ "logged_out": true })))
}

async fn oidc_start(
    State(state): State<Arc<AppState>>,
    Path(provider_id): Path<String>,
) -> Result<impl IntoResponse, crate::error::ApiError> {
    let config = state
        .config
        .auth
        .oidc_providers
        .iter()
        .find(|p| p.id == provider_id)
        .cloned()
        .ok_or(crate::error::ApiError::NotFound)?;
    let client = godwit_auth::oidc::OidcClient::new(&config)
        .await
        .map_err(|_| crate::error::ApiError::Internal)?;
    let (url, _csrf, _nonce) = client.authorize_url(vec![
        "openid".to_string(),
        "email".to_string(),
        "profile".to_string(),
    ]);
    Ok(Redirect::temporary(url.as_str()))
}

async fn oidc_callback(
    State(state): State<Arc<AppState>>,
    Path(provider_id): Path<String>,
    Query(params): Query<OidcCallback>,
) -> Result<impl IntoResponse, crate::error::ApiError> {
    let config = state
        .config
        .auth
        .oidc_providers
        .iter()
        .find(|p| p.id == provider_id)
        .cloned()
        .ok_or(crate::error::ApiError::NotFound)?;
    let client = godwit_auth::oidc::OidcClient::new(&config)
        .await
        .map_err(|_| crate::error::ApiError::Internal)?;
    let (email, _subject, name) = client
        .exchange_code(&params.code, &params.state, "nonce")
        .await
        .map_err(|_| crate::error::ApiError::Unauthorized)?;
    let user = match state.user_repo.get_by_email(&email).await {
        Ok(u) => u,
        Err(_) => state
            .user_repo
            .create(&email, name.as_deref(), godwit_db::models::UserRole::User, None)
            .await
            .map_err(|_| crate::error::ApiError::Internal)?,
    };
    Ok(Json(issue_token_pair(&state, &user).await?))
}

async fn saml_acs(
    State(_state): State<Arc<AppState>>,
    Path(_provider_id): Path<String>,
) -> Result<Response, crate::error::ApiError> {
    Err(crate::error::ApiError::BadRequest(
        "SAML ACS requires XML signature validation; implement with real IdP metadata".to_string(),
    ))
}
```

- [ ] **Step 5: Wire `refresh_token_repo` into `main.rs`**

Modify `crates/godwit-bin/src/main.rs`: add the import and the `AppState` field.

```rust
use godwit_db::{
    connect,
    repositories::{
        api_keys::ApiKeyRepository, organizations::OrganizationRepository,
        refresh_tokens::RefreshTokenRepository, users::UserRepository,
    },
    run_migrations,
};
```

```rust
        api_key_repo: ApiKeyRepository::new(pool.clone()),
        refresh_token_repo: RefreshTokenRepository::new(pool.clone()),
        api_key_cache: MemoryCache::new(),
```

- [ ] **Step 6: Wire `refresh_token_repo` into the integration test harness**

Modify `crates/godwit-api/tests/router_integration.rs`'s `build_app`: add the same import and field as Step 5, using `godwit_db::repositories::refresh_tokens::RefreshTokenRepository::new(pool.clone())`.

- [ ] **Step 7: Run tests to verify they pass**

Run: `cargo check --workspace` (confirm `main.rs` and `router_integration.rs` compile with the new field), then `cargo test -p godwit-api --lib admin::auth::`.
Expected: `cargo check --workspace` passes cleanly; both new unit tests pass.

- [ ] **Step 8: Commit**

```bash
git add crates/godwit-api/src/state.rs crates/godwit-api/src/admin/auth.rs crates/godwit-bin/src/main.rs crates/godwit-api/tests/router_integration.rs
git commit -m "feat(api): refresh token flow (login/refresh/logout)"
```

---

## Task 4: Organizations — Create & Update

**Files:**
- Modify: `crates/godwit-db/src/repositories/organizations.rs`
- Modify: `crates/godwit-api/src/admin/organizations.rs`
- Modify: `crates/godwit-api/src/admin/mod.rs` (fix double-nesting)
- Modify: `crates/godwit-api/tests/router_integration.rs` (one call site of `.create("test-org")` gains an argument)

**Interfaces:**
- Consumes: nothing new.
- Produces: `OrganizationRepository::create(name, rate_limit_requests_per_minute: Option<i32>) -> Organization` (signature change — the old 1-arg form is gone), `OrganizationRepository::update(id, name: Option<&str>, rate_limit_requests_per_minute: Option<i32>) -> Organization`. `POST /organizations`, `PATCH /organizations/:id`.

**Note on the routing fix:** `admin/mod.rs` currently does `.nest("/organizations", organizations::router())`, but `organizations::router()` itself registers the path `/organizations` — nesting adds another prefix, so the real (broken) path today is `/api/v1/organizations/organizations`. This mirrors the exact bug already fixed for `models`/`provider_profiles` in the previous sub-project (fixed via `.merge(...)` instead of `.nest(...)`, since the inner router's routes already carry their full intended path). Fix it the same way here, since this task substantially extends `organizations.rs` and the bug means today's `GET /organizations` isn't even reachable at its documented path.

- [ ] **Step 1: Write the failing test**

Add to `crates/godwit-db/src/repositories/organizations.rs` a test module (create one if none exists):

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use sqlx::PgPool;

    #[sqlx::test]
    async fn create_with_rate_limit(pool: PgPool) {
        let repo = OrganizationRepository::new(pool);
        let org = repo
            .create("acme", Some(100))
            .await
            .expect("create org");
        assert_eq!(org.name, "acme");
        assert_eq!(org.rate_limit_requests_per_minute, Some(100));
    }

    #[sqlx::test]
    async fn create_without_rate_limit(pool: PgPool) {
        let repo = OrganizationRepository::new(pool);
        let org = repo.create("acme", None).await.expect("create org");
        assert_eq!(org.rate_limit_requests_per_minute, None);
    }

    #[sqlx::test]
    async fn update_changes_only_provided_fields(pool: PgPool) {
        let repo = OrganizationRepository::new(pool);
        let org = repo.create("acme", Some(100)).await.expect("create org");

        let renamed = repo
            .update(org.id, Some("acme-corp"), None)
            .await
            .expect("update name only");
        assert_eq!(renamed.name, "acme-corp");
        assert_eq!(renamed.rate_limit_requests_per_minute, Some(100));

        let rate_limited = repo
            .update(org.id, None, Some(50))
            .await
            .expect("update rate limit only");
        assert_eq!(rate_limited.name, "acme-corp");
        assert_eq!(rate_limited.rate_limit_requests_per_minute, Some(50));
    }
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `DATABASE_URL="postgres://tmenard@localhost:5432/godwit" cargo test -p godwit-db organizations:: 2>&1 | head -30`
Expected: FAIL to compile — `create` takes 1 argument, `update` doesn't exist.

- [ ] **Step 3: Rewrite `OrganizationRepository`**

Replace `crates/godwit-db/src/repositories/organizations.rs`'s non-test content:

```rust
use crate::models::Organization;
use godwit_core::PasteurError;
use sqlx::PgPool;
use uuid::Uuid;

pub struct OrganizationRepository {
    pool: PgPool,
}

impl OrganizationRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn create(
        &self,
        name: &str,
        rate_limit_requests_per_minute: Option<i32>,
    ) -> Result<Organization, PasteurError> {
        sqlx::query_as::<_, Organization>(
            "INSERT INTO organizations (name, rate_limit_requests_per_minute) VALUES ($1, $2) RETURNING *"
        )
        .bind(name)
        .bind(rate_limit_requests_per_minute)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| PasteurError::Database(e.to_string()))
    }

    pub async fn get_by_id(&self, id: Uuid) -> Result<Organization, PasteurError> {
        sqlx::query_as::<_, Organization>("SELECT * FROM organizations WHERE id = $1")
            .bind(id)
            .fetch_one(&self.pool)
            .await
            .map_err(|e| match e {
                sqlx::Error::RowNotFound => PasteurError::NotFound,
                _ => PasteurError::Database(e.to_string()),
            })
    }

    pub async fn list(&self) -> Result<Vec<Organization>, PasteurError> {
        sqlx::query_as::<_, Organization>("SELECT * FROM organizations ORDER BY name")
            .fetch_all(&self.pool)
            .await
            .map_err(|e| PasteurError::Database(e.to_string()))
    }

    pub async fn update(
        &self,
        id: Uuid,
        name: Option<&str>,
        rate_limit_requests_per_minute: Option<i32>,
    ) -> Result<Organization, PasteurError> {
        let current = self.get_by_id(id).await?;
        sqlx::query_as::<_, Organization>(
            "UPDATE organizations SET name = $2, rate_limit_requests_per_minute = $3 WHERE id = $1 RETURNING *"
        )
        .bind(id)
        .bind(name.unwrap_or(current.name.as_str()))
        .bind(rate_limit_requests_per_minute.or(current.rate_limit_requests_per_minute))
        .fetch_one(&self.pool)
        .await
        .map_err(|e| PasteurError::Database(e.to_string()))
    }
}
```

- [ ] **Step 4: Fix the one existing call site**

Modify `crates/godwit-api/tests/router_integration.rs:107` — change:

```rust
        .create("test-org")
```

to:

```rust
        .create("test-org", None)
```

- [ ] **Step 5: Run repository tests to verify they pass**

Run: `DATABASE_URL="postgres://tmenard@localhost:5432/godwit" cargo test -p godwit-db organizations::`
Expected: PASS (all 3 tests). `cargo check --workspace` will still fail at this point (admin/organizations.rs doesn't use the new signature yet) — expected, fixed next.

- [ ] **Step 6: Add the admin endpoints and fix routing**

Replace `crates/godwit-api/src/admin/organizations.rs`:

```rust
use axum::{
    extract::{Extension, Path, State},
    routing::{get, patch},
    Json, Router,
};
use godwit_auth::jwt::Claims;
use serde::Deserialize;
use std::sync::Arc;
use uuid::Uuid;

use crate::{admin::require_super_admin, error::ApiError, state::AppState};

pub fn router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/organizations", get(list_organizations).post(create_organization))
        .route("/organizations/:id", patch(update_organization))
}

async fn list_organizations(
    State(state): State<Arc<AppState>>,
    Extension(claims): Extension<Claims>,
) -> Result<Json<serde_json::Value>, ApiError> {
    require_super_admin(&claims)?;
    let orgs = state.org_repo.list().await.map_err(ApiError::Core)?;
    Ok(Json(serde_json::json!({ "data": orgs })))
}

#[derive(Deserialize)]
pub struct CreateOrganizationRequest {
    name: String,
    rate_limit_requests_per_minute: Option<i32>,
}

async fn create_organization(
    State(state): State<Arc<AppState>>,
    Extension(claims): Extension<Claims>,
    Json(req): Json<CreateOrganizationRequest>,
) -> Result<Json<serde_json::Value>, ApiError> {
    require_super_admin(&claims)?;
    let org = state
        .org_repo
        .create(&req.name, req.rate_limit_requests_per_minute)
        .await
        .map_err(ApiError::Core)?;
    Ok(Json(serde_json::json!({ "data": org })))
}

#[derive(Deserialize)]
pub struct UpdateOrganizationRequest {
    name: Option<String>,
    rate_limit_requests_per_minute: Option<i32>,
}

async fn update_organization(
    State(state): State<Arc<AppState>>,
    Extension(claims): Extension<Claims>,
    Path(id): Path<Uuid>,
    Json(req): Json<UpdateOrganizationRequest>,
) -> Result<Json<serde_json::Value>, ApiError> {
    require_super_admin(&claims)?;
    let org = state
        .org_repo
        .update(id, req.name.as_deref(), req.rate_limit_requests_per_minute)
        .await
        .map_err(ApiError::Core)?;
    Ok(Json(serde_json::json!({ "data": org })))
}
```

Note: `organizations.rs` previously checked the role inline (`if role != Role::SuperAdmin`) without a named helper. Rather than duplicating that check again, this rewrite imports the shared `pub(crate) require_super_admin` already defined in `admin/mod.rs` (`crate::admin::require_super_admin`) — the same one `models.rs` and `provider_profiles.rs` already use — instead of adding a third copy of the same three lines.

- [ ] **Step 7: Fix the double-nesting in `admin/mod.rs`**

Modify `crates/godwit-api/src/admin/mod.rs` — change:

```rust
        .nest("/organizations", organizations::router())
```

to:

```rust
        .merge(organizations::router())
```

- [ ] **Step 8: Run tests to verify they pass**

Run: `cargo check --workspace` (must pass cleanly), then `DATABASE_URL="postgres://tmenard@localhost:5432/godwit" cargo test --workspace 2>&1 | tail -60` (must show no regressions vs. the ~107-test baseline, plus the 3 new organization repository tests).

- [ ] **Step 9: Commit**

```bash
git add crates/godwit-db/src/repositories/organizations.rs crates/godwit-api/src/admin/organizations.rs crates/godwit-api/src/admin/mod.rs crates/godwit-api/tests/router_integration.rs
git commit -m "feat(api,db): organization create/update, fix double-nested route"
```

---

## Task 5: Teams — Model, Repository, and CRUD Endpoints

**Files:**
- Modify: `crates/godwit-db/src/models.rs` (add `Team`)
- Create: `crates/godwit-db/src/repositories/teams.rs`
- Modify: `crates/godwit-db/src/repositories/mod.rs` (if not already added in Task 1 Step 6)
- Modify: `crates/godwit-api/src/state.rs` (add `team_repo`)
- Modify: `crates/godwit-api/src/admin/teams.rs`
- Modify: `crates/godwit-api/src/admin/mod.rs` (fix double-nesting)
- Modify: `crates/godwit-bin/src/main.rs`, `crates/godwit-api/tests/router_integration.rs` (wire `team_repo`)

**Interfaces:**
- Produces: `godwit_db::models::Team { id: Uuid, organization_id: Uuid, name: String, created_at: DateTime<Utc> }`, `TeamRepository::{new(pool), create(organization_id, name) -> Team, get_by_id(id) -> Team, list_for_organization(organization_id) -> Vec<Team>, list_all() -> Vec<Team>, update(id, name) -> Team}`. `GET/POST /teams`, `PATCH /teams/:id`.

- [ ] **Step 1: Write the failing test**

Create `crates/godwit-db/src/repositories/teams.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::repositories::organizations::OrganizationRepository;
    use sqlx::PgPool;

    #[sqlx::test]
    async fn create_list_and_get_team(pool: PgPool) {
        let orgs = OrganizationRepository::new(pool.clone());
        let org = orgs.create("acme", None).await.expect("create org");

        let repo = TeamRepository::new(pool);
        let team = repo.create(org.id, "engineering").await.expect("create team");
        assert_eq!(team.organization_id, org.id);
        assert_eq!(team.name, "engineering");

        let fetched = repo.get_by_id(team.id).await.expect("get by id");
        assert_eq!(fetched.id, team.id);

        let listed = repo.list_for_organization(org.id).await.expect("list");
        assert_eq!(listed.len(), 1);
    }

    #[sqlx::test]
    async fn list_all_spans_organizations(pool: PgPool) {
        let orgs = OrganizationRepository::new(pool.clone());
        let org_a = orgs.create("acme-a", None).await.expect("create org a");
        let org_b = orgs.create("acme-b", None).await.expect("create org b");

        let repo = TeamRepository::new(pool);
        repo.create(org_a.id, "team-a").await.expect("create team a");
        repo.create(org_b.id, "team-b").await.expect("create team b");

        let all = repo.list_all().await.expect("list all");
        assert_eq!(all.len(), 2);
    }

    #[sqlx::test]
    async fn update_renames_team(pool: PgPool) {
        let orgs = OrganizationRepository::new(pool.clone());
        let org = orgs.create("acme", None).await.expect("create org");
        let repo = TeamRepository::new(pool);
        let team = repo.create(org.id, "old-name").await.expect("create team");

        let updated = repo.update(team.id, "new-name").await.expect("update team");
        assert_eq!(updated.name, "new-name");
    }
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `DATABASE_URL="postgres://tmenard@localhost:5432/godwit" cargo test -p godwit-db teams:: 2>&1 | head -20`
Expected: FAIL to compile — no `Team` model, no `TeamRepository`.

- [ ] **Step 3: Add the `Team` model**

In `crates/godwit-db/src/models.rs`, add:

```rust
#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct Team {
    pub id: Uuid,
    pub organization_id: Uuid,
    pub name: String,
    pub created_at: DateTime<Utc>,
}
```

- [ ] **Step 4: Implement `TeamRepository`**

Prepend to `crates/godwit-db/src/repositories/teams.rs` (above the test module from Step 1):

```rust
use crate::models::Team;
use godwit_core::PasteurError;
use sqlx::PgPool;
use uuid::Uuid;

pub struct TeamRepository {
    pool: PgPool,
}

impl TeamRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn create(&self, organization_id: Uuid, name: &str) -> Result<Team, PasteurError> {
        sqlx::query_as::<_, Team>(
            "INSERT INTO teams (organization_id, name) VALUES ($1, $2) RETURNING *",
        )
        .bind(organization_id)
        .bind(name)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| PasteurError::Database(e.to_string()))
    }

    pub async fn get_by_id(&self, id: Uuid) -> Result<Team, PasteurError> {
        sqlx::query_as::<_, Team>("SELECT * FROM teams WHERE id = $1")
            .bind(id)
            .fetch_one(&self.pool)
            .await
            .map_err(|e| match e {
                sqlx::Error::RowNotFound => PasteurError::NotFound,
                _ => PasteurError::Database(e.to_string()),
            })
    }

    pub async fn list_for_organization(&self, organization_id: Uuid) -> Result<Vec<Team>, PasteurError> {
        sqlx::query_as::<_, Team>("SELECT * FROM teams WHERE organization_id = $1 ORDER BY name")
            .bind(organization_id)
            .fetch_all(&self.pool)
            .await
            .map_err(|e| PasteurError::Database(e.to_string()))
    }

    pub async fn list_all(&self) -> Result<Vec<Team>, PasteurError> {
        sqlx::query_as::<_, Team>("SELECT * FROM teams ORDER BY organization_id, name")
            .fetch_all(&self.pool)
            .await
            .map_err(|e| PasteurError::Database(e.to_string()))
    }

    pub async fn update(&self, id: Uuid, name: &str) -> Result<Team, PasteurError> {
        sqlx::query_as::<_, Team>("UPDATE teams SET name = $2 WHERE id = $1 RETURNING *")
            .bind(id)
            .bind(name)
            .fetch_one(&self.pool)
            .await
            .map_err(|e| match e {
                sqlx::Error::RowNotFound => PasteurError::NotFound,
                _ => PasteurError::Database(e.to_string()),
            })
    }
}
```

- [ ] **Step 5: Register the module (if not already done)**

Check `crates/godwit-db/src/repositories/mod.rs` has `pub mod teams;`. Add it if Task 1 Step 6 wasn't applied yet.

- [ ] **Step 6: Run repository tests to verify they pass**

Run: `DATABASE_URL="postgres://tmenard@localhost:5432/godwit" cargo test -p godwit-db teams::`
Expected: PASS (all 3 tests).

- [ ] **Step 7: Add `team_repo` to `AppState`**

Modify `crates/godwit-api/src/state.rs` — add the import and field:

```rust
use godwit_db::repositories::{
    api_keys::ApiKeyRepository, organizations::OrganizationRepository,
    refresh_tokens::RefreshTokenRepository, teams::TeamRepository, users::UserRepository,
};
```

```rust
    pub org_repo: OrganizationRepository,
    pub team_repo: TeamRepository,
    pub api_key_repo: ApiKeyRepository,
```

- [ ] **Step 8: Rewrite `admin/teams.rs` with real GET/POST/PATCH**

Replace `crates/godwit-api/src/admin/teams.rs`:

```rust
use axum::{
    extract::{Extension, Path, Query, State},
    routing::{get, patch},
    Json, Router,
};
use godwit_auth::{jwt::Claims, rbac::Role};
use serde::Deserialize;
use std::sync::Arc;
use uuid::Uuid;

use crate::{error::ApiError, state::AppState};

pub fn router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/teams", get(list_teams).post(create_team))
        .route("/teams/:id", patch(update_team))
}

fn require_manage_users(claims: &Claims) -> Result<Role, ApiError> {
    let role = Role::from_str(&claims.role).ok_or(ApiError::Forbidden)?;
    if !role.can_manage_users() {
        return Err(ApiError::Forbidden);
    }
    Ok(role)
}

#[derive(Deserialize)]
pub struct ListTeamsQuery {
    organization_id: Option<Uuid>,
}

async fn list_teams(
    State(state): State<Arc<AppState>>,
    Extension(claims): Extension<Claims>,
    Query(query): Query<ListTeamsQuery>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let role = require_manage_users(&claims)?;
    let teams = if role == Role::SuperAdmin {
        match query.organization_id {
            Some(org_id) => state.team_repo.list_for_organization(org_id).await,
            None => state.team_repo.list_all().await,
        }
    } else {
        state.team_repo.list_for_organization(claims.organization_id).await
    }
    .map_err(ApiError::Core)?;
    Ok(Json(serde_json::json!({ "data": teams })))
}

#[derive(Deserialize)]
pub struct CreateTeamRequest {
    name: String,
    organization_id: Option<Uuid>,
}

async fn create_team(
    State(state): State<Arc<AppState>>,
    Extension(claims): Extension<Claims>,
    Json(req): Json<CreateTeamRequest>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let role = require_manage_users(&claims)?;
    let organization_id = if role == Role::SuperAdmin {
        req.organization_id
            .ok_or_else(|| ApiError::BadRequest("organization_id is required".to_string()))?
    } else {
        claims.organization_id
    };
    let team = state
        .team_repo
        .create(organization_id, &req.name)
        .await
        .map_err(ApiError::Core)?;
    Ok(Json(serde_json::json!({ "data": team })))
}

#[derive(Deserialize)]
pub struct UpdateTeamRequest {
    name: String,
}

async fn update_team(
    State(state): State<Arc<AppState>>,
    Extension(claims): Extension<Claims>,
    Path(id): Path<Uuid>,
    Json(req): Json<UpdateTeamRequest>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let role = require_manage_users(&claims)?;
    let team = state.team_repo.get_by_id(id).await.map_err(ApiError::Core)?;
    if role != Role::SuperAdmin && team.organization_id != claims.organization_id {
        return Err(ApiError::Forbidden);
    }
    let updated = state.team_repo.update(id, &req.name).await.map_err(ApiError::Core)?;
    Ok(Json(serde_json::json!({ "data": updated })))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn create_team_request_deserializes_without_organization_id() {
        let json = r#"{"name":"engineering"}"#;
        let req: CreateTeamRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.name, "engineering");
        assert_eq!(req.organization_id, None);
    }
}
```

- [ ] **Step 9: Fix the double-nesting in `admin/mod.rs`**

Modify `crates/godwit-api/src/admin/mod.rs` — change:

```rust
        .nest("/teams", teams::router())
```

to:

```rust
        .merge(teams::router())
```

- [ ] **Step 10: Wire `team_repo` into `main.rs` and the test harness**

In `crates/godwit-bin/src/main.rs`, add `teams::TeamRepository` to the `godwit_db::repositories::{...}` import and add `team_repo: TeamRepository::new(pool.clone()),` to the `AppState` literal. Do the same in `crates/godwit-api/tests/router_integration.rs`'s `build_app`.

- [ ] **Step 11: Run tests to verify they pass**

Run: `cargo check --workspace`, then `DATABASE_URL="postgres://tmenard@localhost:5432/godwit" cargo test --workspace 2>&1 | tail -60`.
Expected: both pass cleanly, no regressions.

- [ ] **Step 12: Commit**

```bash
git add crates/godwit-db/src/models.rs crates/godwit-db/src/repositories/teams.rs crates/godwit-db/src/repositories/mod.rs crates/godwit-api/src/state.rs crates/godwit-api/src/admin/teams.rs crates/godwit-api/src/admin/mod.rs crates/godwit-bin/src/main.rs crates/godwit-api/tests/router_integration.rs
git commit -m "feat(api,db): real teams CRUD, fix double-nested route"
```

---

## Task 6: Team Memberships

**Files:**
- Modify: `crates/godwit-db/src/models.rs` (add `TeamMembership`)
- Create: `crates/godwit-db/src/repositories/team_memberships.rs`
- Modify: `crates/godwit-db/src/repositories/mod.rs` (if not already added in Task 1 Step 6)
- Modify: `crates/godwit-api/src/state.rs` (add `team_membership_repo`)
- Modify: `crates/godwit-api/src/admin/teams.rs` (add member endpoints)
- Modify: `crates/godwit-bin/src/main.rs`, `crates/godwit-api/tests/router_integration.rs` (wire the new repo)

**Interfaces:**
- Consumes: `TeamRepository::get_by_id` (Task 5).
- Produces: `godwit_db::models::TeamMembership { user_id: Uuid, team_id: Uuid, role: String }`, `TeamMembershipRepository::{new(pool), add_member(team_id, user_id, role) -> TeamMembership, remove_member(team_id, user_id) -> (), get_membership(team_id, user_id) -> TeamMembership}`. `POST /teams/:id/members`, `DELETE /teams/:id/members/:user_id`.

- [ ] **Step 1: Write the failing test**

Create `crates/godwit-db/src/repositories/team_memberships.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::repositories::{
        organizations::OrganizationRepository, teams::TeamRepository, users::UserRepository,
    };
    use godwit_db::models::UserRole;
    use sqlx::PgPool;

    async fn seed(pool: &PgPool) -> (Uuid, Uuid) {
        let org = OrganizationRepository::new(pool.clone())
            .create("acme", None)
            .await
            .expect("create org");
        let team = TeamRepository::new(pool.clone())
            .create(org.id, "engineering")
            .await
            .expect("create team");
        let user = UserRepository::new(pool.clone())
            .create("dave@example.com", None, UserRole::User, Some(org.id))
            .await
            .expect("create user");
        (team.id, user.id)
    }

    #[sqlx::test]
    async fn add_and_get_membership(pool: PgPool) {
        let (team_id, user_id) = seed(&pool).await;
        let repo = TeamMembershipRepository::new(pool);
        let membership = repo
            .add_member(team_id, user_id, "member")
            .await
            .expect("add member");
        assert_eq!(membership.role, "member");

        let fetched = repo.get_membership(team_id, user_id).await.expect("get membership");
        assert_eq!(fetched.user_id, user_id);
    }

    #[sqlx::test]
    async fn add_member_upserts_role(pool: PgPool) {
        let (team_id, user_id) = seed(&pool).await;
        let repo = TeamMembershipRepository::new(pool);
        repo.add_member(team_id, user_id, "member").await.expect("add as member");
        let promoted = repo
            .add_member(team_id, user_id, "team_admin")
            .await
            .expect("re-add as team_admin");
        assert_eq!(promoted.role, "team_admin");
    }

    #[sqlx::test]
    async fn remove_member_deletes_row(pool: PgPool) {
        let (team_id, user_id) = seed(&pool).await;
        let repo = TeamMembershipRepository::new(pool);
        repo.add_member(team_id, user_id, "member").await.expect("add member");
        repo.remove_member(team_id, user_id).await.expect("remove member");
        let err = repo.get_membership(team_id, user_id).await.unwrap_err();
        assert!(matches!(err, godwit_core::PasteurError::NotFound));
    }

    #[sqlx::test]
    async fn get_membership_not_found(pool: PgPool) {
        let (team_id, user_id) = seed(&pool).await;
        let repo = TeamMembershipRepository::new(pool);
        let err = repo.get_membership(team_id, user_id).await.unwrap_err();
        assert!(matches!(err, godwit_core::PasteurError::NotFound));
    }
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `DATABASE_URL="postgres://tmenard@localhost:5432/godwit" cargo test -p godwit-db team_memberships:: 2>&1 | head -20`
Expected: FAIL to compile — no `TeamMembership` model, no `TeamMembershipRepository`.

- [ ] **Step 3: Add the `TeamMembership` model**

In `crates/godwit-db/src/models.rs`, add:

```rust
#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct TeamMembership {
    pub user_id: Uuid,
    pub team_id: Uuid,
    pub role: String,
}
```

- [ ] **Step 4: Implement `TeamMembershipRepository`**

Prepend to `crates/godwit-db/src/repositories/team_memberships.rs` (above the test module from Step 1):

```rust
use crate::models::TeamMembership;
use godwit_core::PasteurError;
use sqlx::PgPool;
use uuid::Uuid;

pub struct TeamMembershipRepository {
    pool: PgPool,
}

impl TeamMembershipRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn add_member(
        &self,
        team_id: Uuid,
        user_id: Uuid,
        role: &str,
    ) -> Result<TeamMembership, PasteurError> {
        sqlx::query_as::<_, TeamMembership>(
            "INSERT INTO team_memberships (user_id, team_id, role) VALUES ($1, $2, $3)
             ON CONFLICT (user_id, team_id) DO UPDATE SET role = EXCLUDED.role
             RETURNING *"
        )
        .bind(user_id)
        .bind(team_id)
        .bind(role)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| PasteurError::Database(e.to_string()))
    }

    pub async fn remove_member(&self, team_id: Uuid, user_id: Uuid) -> Result<(), PasteurError> {
        sqlx::query("DELETE FROM team_memberships WHERE team_id = $1 AND user_id = $2")
            .bind(team_id)
            .bind(user_id)
            .execute(&self.pool)
            .await
            .map_err(|e| PasteurError::Database(e.to_string()))?;
        Ok(())
    }

    pub async fn get_membership(&self, team_id: Uuid, user_id: Uuid) -> Result<TeamMembership, PasteurError> {
        sqlx::query_as::<_, TeamMembership>(
            "SELECT * FROM team_memberships WHERE team_id = $1 AND user_id = $2",
        )
        .bind(team_id)
        .bind(user_id)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| match e {
            sqlx::Error::RowNotFound => PasteurError::NotFound,
            _ => PasteurError::Database(e.to_string()),
        })
    }
}
```

- [ ] **Step 5: Register the module (if not already done)**

Check `crates/godwit-db/src/repositories/mod.rs` has `pub mod team_memberships;`. Add it if Task 1 Step 6 wasn't applied yet.

- [ ] **Step 6: Run repository tests to verify they pass**

Run: `DATABASE_URL="postgres://tmenard@localhost:5432/godwit" cargo test -p godwit-db team_memberships::`
Expected: PASS (all 4 tests).

- [ ] **Step 7: Add `team_membership_repo` to `AppState`**

Modify `crates/godwit-api/src/state.rs` — add `team_memberships::TeamMembershipRepository` to the import and add the field:

```rust
    pub team_repo: TeamRepository,
    pub team_membership_repo: TeamMembershipRepository,
```

- [ ] **Step 8: Add member endpoints to `admin/teams.rs`**

Add to `crates/godwit-api/src/admin/teams.rs` (in the non-test section, after `update_team`):

```rust
async fn require_team_manage(
    state: &AppState,
    claims: &Claims,
    team_id: Uuid,
) -> Result<(), ApiError> {
    let role = Role::from_str(&claims.role).ok_or(ApiError::Forbidden)?;
    if role == Role::SuperAdmin {
        return Ok(());
    }
    let team = state.team_repo.get_by_id(team_id).await.map_err(ApiError::Core)?;
    if role == Role::OrgAdmin && team.organization_id == claims.organization_id {
        return Ok(());
    }
    // A team_admin (or an org_admin of a *different* org, rejected above) must hold
    // team_admin membership for THIS specific team — not just the global role.
    match state
        .team_membership_repo
        .get_membership(team_id, claims.user_id)
        .await
    {
        Ok(membership) if membership.role == "team_admin" => Ok(()),
        _ => Err(ApiError::Forbidden),
    }
}

#[derive(Deserialize)]
pub struct AddMemberRequest {
    user_id: Uuid,
    role: String,
}

async fn add_member(
    State(state): State<Arc<AppState>>,
    Extension(claims): Extension<Claims>,
    Path(team_id): Path<Uuid>,
    Json(req): Json<AddMemberRequest>,
) -> Result<Json<serde_json::Value>, ApiError> {
    require_team_manage(&state, &claims, team_id).await?;
    if req.role != "team_admin" && req.role != "member" {
        return Err(ApiError::BadRequest(
            "role must be 'team_admin' or 'member'".to_string(),
        ));
    }
    let membership = state
        .team_membership_repo
        .add_member(team_id, req.user_id, &req.role)
        .await
        .map_err(ApiError::Core)?;
    Ok(Json(serde_json::json!({ "data": membership })))
}

async fn remove_member(
    State(state): State<Arc<AppState>>,
    Extension(claims): Extension<Claims>,
    Path((team_id, user_id)): Path<(Uuid, Uuid)>,
) -> Result<Json<serde_json::Value>, ApiError> {
    require_team_manage(&state, &claims, team_id).await?;
    state
        .team_membership_repo
        .remove_member(team_id, user_id)
        .await
        .map_err(ApiError::Core)?;
    Ok(Json(serde_json::json!({ "removed": true })))
}
```

Update `router()` in the same file:

```rust
pub fn router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/teams", get(list_teams).post(create_team))
        .route("/teams/:id", patch(update_team))
        .route("/teams/:id/members", post(add_member))
        .route("/teams/:id/members/:user_id", axum::routing::delete(remove_member))
}
```

Add `Deserialize` import for `Uuid` fields already present; no new imports beyond what Task 5 added.

- [ ] **Step 9: Wire `team_membership_repo` into `main.rs` and the test harness**

In `crates/godwit-bin/src/main.rs`, add `team_memberships::TeamMembershipRepository` to the import and `team_membership_repo: TeamMembershipRepository::new(pool.clone()),` to the `AppState` literal. Do the same in `crates/godwit-api/tests/router_integration.rs`'s `build_app`.

- [ ] **Step 10: Run tests to verify they pass**

Run: `cargo check --workspace`, then `DATABASE_URL="postgres://tmenard@localhost:5432/godwit" cargo test --workspace 2>&1 | tail -60`.
Expected: both pass cleanly, no regressions.

- [ ] **Step 11: Commit**

```bash
git add crates/godwit-db/src/models.rs crates/godwit-db/src/repositories/team_memberships.rs crates/godwit-db/src/repositories/mod.rs crates/godwit-api/src/state.rs crates/godwit-api/src/admin/teams.rs crates/godwit-bin/src/main.rs crates/godwit-api/tests/router_integration.rs
git commit -m "feat(api,db): team membership management"
```

---

## Task 7: User-Deletion Cascade Migration

**Files:**
- Create: `crates/godwit-db/migrations/20260803000005_user_delete_cascade.up.sql`
- Create: `crates/godwit-db/migrations/20260803000005_user_delete_cascade.down.sql`

**Interfaces:** none new — this is a schema-only change, verified by a migration test.

- [ ] **Step 1: Write the failing test**

Add to `crates/godwit-db/src/lib.rs`'s test module (following the pattern of the existing migration tests there):

```rust
    #[sqlx::test]
    async fn deleting_user_cascades_api_keys_and_nulls_request_logs(pool: PgPool) {
        use crate::repositories::{
            api_keys::ApiKeyRepository, organizations::OrganizationRepository, users::UserRepository,
        };
        use godwit_db::models::UserRole;

        let org = OrganizationRepository::new(pool.clone())
            .create("acme", None)
            .await
            .expect("create org");
        let user = UserRepository::new(pool.clone())
            .create("erin@example.com", None, UserRole::User, Some(org.id))
            .await
            .expect("create user");

        let (_, hash, prefix) = godwit_auth::api_keys::generate_api_key();
        let api_key = ApiKeyRepository::new(pool.clone())
            .create(user.id, org.id, "test-key", &prefix, &hash, &["chat".to_string()], None, None)
            .await
            .expect("create api key");

        sqlx::query(
            "INSERT INTO request_logs (api_key_id, user_id, organization_id, model, provider, provider_model_id, duration_ms, status)
             VALUES ($1, $2, $3, 'gpt-4o', 'openai', 'gpt-4o', 100, 'success')"
        )
        .bind(api_key.id)
        .bind(user.id)
        .bind(org.id)
        .execute(&pool)
        .await
        .expect("insert request log");

        sqlx::query("DELETE FROM users WHERE id = $1")
            .bind(user.id)
            .execute(&pool)
            .await
            .expect("delete user");

        let remaining_keys: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM api_keys WHERE id = $1")
            .bind(api_key.id)
            .fetch_one(&pool)
            .await
            .expect("count api_keys");
        assert_eq!(remaining_keys, 0, "api_keys row should cascade-delete with the user");

        let log_user_id: Option<uuid::Uuid> =
            sqlx::query_scalar("SELECT user_id FROM request_logs WHERE api_key_id = $1")
                .bind(api_key.id)
                .fetch_one(&pool)
                .await
                .expect("fetch request_logs row");
        assert_eq!(log_user_id, None, "request_logs.user_id should be nulled, not the row deleted");
    }
```

- [ ] **Step 2: Run test to verify it fails**

Run: `DATABASE_URL="postgres://tmenard@localhost:5432/godwit" cargo test -p godwit-db deleting_user_cascades`
Expected: FAIL — deleting the user raises a foreign-key violation (`api_keys` still references it with the default `NO ACTION`).

- [ ] **Step 3: Write the migration**

Create `crates/godwit-db/migrations/20260803000005_user_delete_cascade.up.sql`:

```sql
ALTER TABLE api_keys DROP CONSTRAINT api_keys_user_id_fkey;
ALTER TABLE api_keys ADD CONSTRAINT api_keys_user_id_fkey
    FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE;

ALTER TABLE request_logs DROP CONSTRAINT request_logs_user_id_fkey;
ALTER TABLE request_logs ADD CONSTRAINT request_logs_user_id_fkey
    FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE SET NULL;
```

Create `crates/godwit-db/migrations/20260803000005_user_delete_cascade.down.sql`:

```sql
ALTER TABLE api_keys DROP CONSTRAINT api_keys_user_id_fkey;
ALTER TABLE api_keys ADD CONSTRAINT api_keys_user_id_fkey
    FOREIGN KEY (user_id) REFERENCES users(id);

ALTER TABLE request_logs DROP CONSTRAINT request_logs_user_id_fkey;
ALTER TABLE request_logs ADD CONSTRAINT request_logs_user_id_fkey
    FOREIGN KEY (user_id) REFERENCES users(id);
```

(Constraint names confirmed against the live schema via `psql \d api_keys` / `\d request_logs` — both are Postgres' default auto-generated `<table>_<column>_fkey` names, since the original migration never named them explicitly.)

- [ ] **Step 4: Run test to verify it passes**

Run: `DATABASE_URL="postgres://tmenard@localhost:5432/godwit" cargo test -p godwit-db deleting_user_cascades`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add crates/godwit-db/migrations crates/godwit-db/src/lib.rs
git commit -m "feat(db): cascade api_keys, null request_logs.user_id on user delete"
```

---

## Task 8: Users — Get/Update/Delete & Global Scoping

**Files:**
- Modify: `crates/godwit-db/src/repositories/users.rs`
- Modify: `crates/godwit-api/src/admin/users.rs`
- Modify: `crates/godwit-api/src/admin/mod.rs` (fix double-nesting)

**Interfaces:**
- Consumes: user-delete cascade (Task 7).
- Produces: `UserRepository::{list_all() -> Vec<User>, update(id, name: Option<&str>, role: Option<&str>, organization_id: Option<Uuid>) -> User, delete(id) -> ()}` (new methods; `get_by_id`/`get_by_email`/`create`/`list_for_organization` unchanged). Real `GET /users/:id`, `PATCH /users/:id`, `DELETE /users/:id`; `GET /users` gains optional `?organization_id=` scoping for `super_admin`.

- [ ] **Step 1: Write the failing test**

Add to `crates/godwit-db/src/repositories/users.rs`'s existing `#[cfg(test)] mod tests` block:

```rust
    #[sqlx::test]
    async fn list_all_spans_organizations(pool: PgPool) {
        let repo = UserRepository::new(pool);
        repo.create("a@example.com", None, UserRole::User, None).await.expect("create a");
        repo.create("b@example.com", None, UserRole::User, None).await.expect("create b");
        let all = repo.list_all().await.expect("list all");
        assert_eq!(all.len(), 2);
    }

    #[sqlx::test]
    async fn update_changes_only_provided_fields(pool: PgPool) {
        let repo = UserRepository::new(pool);
        let user = repo
            .create("carol@example.com", Some("Carol"), UserRole::User, None)
            .await
            .expect("create user");

        let renamed = repo
            .update(user.id, Some("Carol R."), None, None)
            .await
            .expect("update name only");
        assert_eq!(renamed.name.as_deref(), Some("Carol R."));
        assert_eq!(renamed.role, "user");

        let promoted = repo
            .update(user.id, None, Some("org_admin"), None)
            .await
            .expect("update role only");
        assert_eq!(promoted.name.as_deref(), Some("Carol R."));
        assert_eq!(promoted.role, "org_admin");
    }

    #[sqlx::test]
    async fn delete_removes_the_row(pool: PgPool) {
        let repo = UserRepository::new(pool);
        let user = repo
            .create("dan@example.com", None, UserRole::User, None)
            .await
            .expect("create user");
        repo.delete(user.id).await.expect("delete user");
        let err = repo.get_by_id(user.id).await.unwrap_err();
        assert!(matches!(err, godwit_core::PasteurError::NotFound));
    }
```

- [ ] **Step 2: Run test to verify it fails**

Run: `DATABASE_URL="postgres://tmenard@localhost:5432/godwit" cargo test -p godwit-db repositories::users:: 2>&1 | head -20`
Expected: FAIL to compile — `list_all`/`update`/`delete` don't exist.

- [ ] **Step 3: Add the new repository methods**

Add to `crates/godwit-db/src/repositories/users.rs`'s `impl UserRepository` block (after `list_for_organization`):

```rust
    pub async fn list_all(&self) -> Result<Vec<User>, PasteurError> {
        sqlx::query_as::<_, User>("SELECT * FROM users ORDER BY organization_id, email")
            .fetch_all(&self.pool)
            .await
            .map_err(|e| PasteurError::Database(e.to_string()))
    }

    pub async fn update(
        &self,
        id: Uuid,
        name: Option<&str>,
        role: Option<&str>,
        organization_id: Option<Uuid>,
    ) -> Result<User, PasteurError> {
        let current = self.get_by_id(id).await?;
        sqlx::query_as::<_, User>(
            "UPDATE users SET name = $2, role = $3, organization_id = $4 WHERE id = $1 RETURNING *",
        )
        .bind(id)
        .bind(name.or(current.name.as_deref()))
        .bind(role.unwrap_or(current.role.as_str()))
        .bind(organization_id.or(current.organization_id))
        .fetch_one(&self.pool)
        .await
        .map_err(|e| PasteurError::Database(e.to_string()))
    }

    pub async fn delete(&self, id: Uuid) -> Result<(), PasteurError> {
        sqlx::query("DELETE FROM users WHERE id = $1")
            .bind(id)
            .execute(&self.pool)
            .await
            .map_err(|e| PasteurError::Database(e.to_string()))?;
        Ok(())
    }
```

- [ ] **Step 4: Run repository tests to verify they pass**

Run: `DATABASE_URL="postgres://tmenard@localhost:5432/godwit" cargo test -p godwit-db repositories::users::`
Expected: PASS (all 4 tests, including the pre-existing `create_and_fetch_user`).

- [ ] **Step 5: Rewrite `admin/users.rs`**

Replace `crates/godwit-api/src/admin/users.rs`'s non-test content:

```rust
use axum::{
    extract::{Extension, Path, Query, State},
    routing::get,
    Json, Router,
};
use godwit_auth::{jwt::Claims, rbac::Role};
use serde::Deserialize;
use std::sync::Arc;
use uuid::Uuid;

use crate::{error::ApiError, state::AppState};

#[derive(Deserialize)]
pub struct CreateUserRequest {
    email: String,
    name: Option<String>,
    role: String,
}

pub fn router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/users", get(list_users).post(create_user))
        .route(
            "/users/:id",
            get(get_user).patch(update_user).delete(delete_user),
        )
}

fn require_role(claims: &Claims, allowed: &[Role]) -> Result<Role, ApiError> {
    let role = Role::from_str(&claims.role).ok_or(ApiError::Forbidden)?;
    if !allowed.contains(&role) {
        return Err(ApiError::Forbidden);
    }
    Ok(role)
}

/// `org_admin` may only act on a user already in its own org; `super_admin` may act on anyone.
fn check_same_org(role: Role, claims: &Claims, target_org: Option<Uuid>) -> Result<(), ApiError> {
    if role != Role::SuperAdmin && target_org != Some(claims.organization_id) {
        return Err(ApiError::Forbidden);
    }
    Ok(())
}

#[derive(Deserialize)]
pub struct ListUsersQuery {
    organization_id: Option<Uuid>,
}

async fn list_users(
    State(state): State<Arc<AppState>>,
    Extension(claims): Extension<Claims>,
    Query(query): Query<ListUsersQuery>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let role = require_role(&claims, &[Role::SuperAdmin, Role::OrgAdmin])?;
    let users = if role == Role::SuperAdmin {
        match query.organization_id {
            Some(org_id) => state.user_repo.list_for_organization(org_id).await,
            None => state.user_repo.list_all().await,
        }
    } else {
        state.user_repo.list_for_organization(claims.organization_id).await
    }
    .map_err(ApiError::Core)?;
    Ok(Json(serde_json::json!({ "data": users })))
}

async fn create_user(
    State(state): State<Arc<AppState>>,
    Extension(claims): Extension<Claims>,
    Json(req): Json<CreateUserRequest>,
) -> Result<Json<serde_json::Value>, ApiError> {
    require_role(&claims, &[Role::SuperAdmin, Role::OrgAdmin])?;
    let role = godwit_db::models::UserRole::from_str(&req.role)
        .ok_or(ApiError::BadRequest("invalid role".to_string()))?;
    let org_id = claims.organization_id;
    let user = state
        .user_repo
        .create(&req.email, req.name.as_deref(), role, Some(org_id))
        .await
        .map_err(ApiError::Core)?;
    Ok(Json(serde_json::json!({ "data": user })))
}

async fn get_user(
    State(state): State<Arc<AppState>>,
    Extension(claims): Extension<Claims>,
    Path(id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let role = require_role(&claims, &[Role::SuperAdmin, Role::OrgAdmin])?;
    let user = state.user_repo.get_by_id(id).await.map_err(ApiError::Core)?;
    check_same_org(role, &claims, user.organization_id)?;
    Ok(Json(serde_json::json!({ "data": user })))
}

#[derive(Deserialize)]
pub struct UpdateUserRequest {
    name: Option<String>,
    role: Option<String>,
    organization_id: Option<Uuid>,
}

async fn update_user(
    State(state): State<Arc<AppState>>,
    Extension(claims): Extension<Claims>,
    Path(id): Path<Uuid>,
    Json(req): Json<UpdateUserRequest>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let role = require_role(&claims, &[Role::SuperAdmin, Role::OrgAdmin])?;
    let target = state.user_repo.get_by_id(id).await.map_err(ApiError::Core)?;
    check_same_org(role, &claims, target.organization_id)?;
    if req.organization_id.is_some() && role != Role::SuperAdmin {
        return Err(ApiError::Forbidden);
    }
    if let Some(ref role_str) = req.role {
        godwit_db::models::UserRole::from_str(role_str)
            .ok_or(ApiError::BadRequest("invalid role".to_string()))?;
    }
    let updated = state
        .user_repo
        .update(id, req.name.as_deref(), req.role.as_deref(), req.organization_id)
        .await
        .map_err(ApiError::Core)?;
    Ok(Json(serde_json::json!({ "data": updated })))
}

async fn delete_user(
    State(state): State<Arc<AppState>>,
    Extension(claims): Extension<Claims>,
    Path(id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let role = require_role(&claims, &[Role::SuperAdmin, Role::OrgAdmin])?;
    if claims.user_id == id {
        return Err(ApiError::BadRequest("cannot delete your own account".to_string()));
    }
    let target = state.user_repo.get_by_id(id).await.map_err(ApiError::Core)?;
    check_same_org(role, &claims, target.organization_id)?;
    state.user_repo.delete(id).await.map_err(ApiError::Core)?;
    Ok(Json(serde_json::json!({ "deleted": true })))
}
```

- [ ] **Step 6: Fix the double-nesting in `admin/mod.rs`**

Modify `crates/godwit-api/src/admin/mod.rs` — change:

```rust
        .nest("/users", users::router())
```

to:

```rust
        .merge(users::router())
```

- [ ] **Step 7: Run tests to verify they pass**

Run: `cargo check --workspace`, then `DATABASE_URL="postgres://tmenard@localhost:5432/godwit" cargo test --workspace 2>&1 | tail -60`.
Expected: both pass cleanly, no regressions.

- [ ] **Step 8: Commit**

```bash
git add crates/godwit-db/src/repositories/users.rs crates/godwit-api/src/admin/users.rs crates/godwit-api/src/admin/mod.rs
git commit -m "feat(api,db): real user get/update/delete, global super_admin scoping, fix double-nested route"
```

---

## Task 9: Real Spend Tracking

**Files:**
- Modify: `crates/godwit-api/src/admin/spend.rs`
- Modify: `crates/godwit-api/src/admin/mod.rs` (fix double-nesting)

**Interfaces:**
- Produces: `GET /spend?from=<RFC3339?>&to=<RFC3339?>&organization_id=<uuid?>&team_id=<uuid?>&user_id=<uuid?>`, scoped per the RBAC model (§4/§9 of the spec). `compute_cost` (existing, used by the proxy) is untouched.

**Note on the routing fix:** `admin/mod.rs` currently does `.nest("/spend", spend::router())`, and `spend::router()` registers `/spend` — the same double-nesting bug as Tasks 4/5/8, fixed the same way.

- [ ] **Step 1: Write the failing test**

Add to `crates/godwit-api/src/admin/spend.rs`'s existing `#[cfg(test)] mod tests` block (the file already has `cost_computation` — keep it, add these alongside):

```rust
    #[test]
    fn spend_query_deserializes_with_all_fields_optional() {
        let query: SpendQuery = serde_json::from_str("{}").expect("empty query");
        assert_eq!(query.organization_id, None);
        assert_eq!(query.team_id, None);
        assert_eq!(query.user_id, None);
        assert_eq!(query.from, None);
        assert_eq!(query.to, None);
    }

    #[test]
    fn spend_scope_forces_org_admin_to_own_org() {
        let claims = godwit_auth::jwt::Claims::new(uuid::Uuid::new_v4(), uuid::Uuid::new_v4(), "org_admin");
        let requested = SpendQuery {
            from: None,
            to: None,
            organization_id: Some(uuid::Uuid::new_v4()), // attempt to look at a different org
            team_id: None,
            user_id: None,
        };
        let scoped = scope_spend_query(&claims, requested);
        assert_eq!(scoped.0, Some(claims.organization_id));
    }

    #[test]
    fn spend_scope_forces_user_to_self() {
        let claims = godwit_auth::jwt::Claims::new(uuid::Uuid::new_v4(), uuid::Uuid::new_v4(), "user");
        let requested = SpendQuery {
            from: None,
            to: None,
            organization_id: Some(uuid::Uuid::new_v4()),
            team_id: Some(uuid::Uuid::new_v4()),
            user_id: Some(uuid::Uuid::new_v4()), // attempt to look at someone else's usage
        };
        let scoped = scope_spend_query(&claims, requested);
        assert_eq!(scoped.0, Some(claims.organization_id));
        assert_eq!(scoped.1, None);
        assert_eq!(scoped.2, Some(claims.user_id));
    }

    #[test]
    fn spend_scope_leaves_super_admin_unscoped() {
        let claims = godwit_auth::jwt::Claims::new(uuid::Uuid::new_v4(), uuid::Uuid::new_v4(), "super_admin");
        let org_id = uuid::Uuid::new_v4();
        let requested = SpendQuery { from: None, to: None, organization_id: Some(org_id), team_id: None, user_id: None };
        let scoped = scope_spend_query(&claims, requested);
        assert_eq!(scoped.0, Some(org_id));
    }
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p godwit-api --lib admin::spend:: 2>&1 | head -20`
Expected: FAIL to compile — `SpendQuery`, `scope_spend_query` don't exist.

- [ ] **Step 3: Rewrite `spend.rs`**

Replace `crates/godwit-api/src/admin/spend.rs`'s non-test content (keep `compute_cost` exactly as-is, it's used by `proxy.rs` and unrelated to this task):

```rust
use axum::{
    extract::{Extension, Query, State},
    routing::get,
    Json, Router,
};
use chrono::{DateTime, Utc};
use godwit_auth::{jwt::Claims, rbac::Role};
use godwit_core::Capability;
use godwit_db::models::Model;
use godwit_providers::UsageReport;
use rust_decimal::Decimal;
use serde::Deserialize;
use std::str::FromStr;
use std::sync::Arc;
use uuid::Uuid;

use crate::{error::ApiError, state::AppState};

const PRICING_INPUT_PER_1K: &str = "input_per_1k";
const PRICING_OUTPUT_PER_1K: &str = "output_per_1k";

pub fn router() -> Router<Arc<AppState>> {
    Router::new().route("/spend", get(get_spend))
}

#[derive(Debug, Deserialize, PartialEq)]
pub struct SpendQuery {
    from: Option<DateTime<Utc>>,
    to: Option<DateTime<Utc>>,
    organization_id: Option<Uuid>,
    team_id: Option<Uuid>,
    user_id: Option<Uuid>,
}

/// Applies the RBAC scoping model (spec §4/§9): `super_admin` gets whatever it asked
/// for; `org_admin` is always forced to its own org (but may still filter by team/user
/// within it); `team_admin`/`user` are always forced to their own usage only, with any
/// org/team/user filter the caller passed ignored.
fn scope_spend_query(
    claims: &Claims,
    query: SpendQuery,
) -> (Option<Uuid>, Option<Uuid>, Option<Uuid>) {
    let role = Role::from_str(&claims.role);
    match role {
        Some(Role::SuperAdmin) => (query.organization_id, query.team_id, query.user_id),
        Some(Role::OrgAdmin) => (Some(claims.organization_id), query.team_id, query.user_id),
        _ => (Some(claims.organization_id), None, Some(claims.user_id)),
    }
}

#[derive(Debug, sqlx::FromRow, serde::Serialize)]
struct SpendRow {
    organization_id: Uuid,
    team_id: Option<Uuid>,
    user_id: Option<Uuid>,
    total_cost_usd: Decimal,
    request_count: i64,
    tokens_in: i64,
    tokens_out: i64,
}

async fn get_spend(
    State(state): State<Arc<AppState>>,
    Extension(claims): Extension<Claims>,
    Query(query): Query<SpendQuery>,
) -> Result<Json<serde_json::Value>, ApiError> {
    Role::from_str(&claims.role).ok_or(ApiError::Forbidden)?;
    let from = query.from;
    let to = query.to;
    let (organization_id, team_id, user_id) = scope_spend_query(&claims, query);

    let rows = sqlx::query_as::<_, SpendRow>(
        "SELECT organization_id, team_id, user_id,
                COALESCE(SUM(cost_usd), 0) AS total_cost_usd,
                COUNT(*) AS request_count,
                COALESCE(SUM(tokens_in), 0) AS tokens_in,
                COALESCE(SUM(tokens_out), 0) AS tokens_out
         FROM request_logs
         WHERE ($1::timestamptz IS NULL OR created_at >= $1)
           AND ($2::timestamptz IS NULL OR created_at <= $2)
           AND ($3::uuid IS NULL OR organization_id = $3)
           AND ($4::uuid IS NULL OR team_id = $4)
           AND ($5::uuid IS NULL OR user_id = $5)
         GROUP BY organization_id, team_id, user_id"
    )
    .bind(from)
    .bind(to)
    .bind(organization_id)
    .bind(team_id)
    .bind(user_id)
    .fetch_all(&state.pool)
    .await
    .map_err(|e| ApiError::Core(godwit_core::PasteurError::Database(e.to_string())))?;

    Ok(Json(serde_json::json!({ "data": rows })))
}

pub fn compute_cost(model: &Model, capability: Capability, usage: &UsageReport) -> Option<Decimal> {
    let pricing = model.pricing.as_object()?;
    match capability {
        Capability::Chat => {
            let input_price = pricing.get(PRICING_INPUT_PER_1K)?;
            let output_price = pricing.get(PRICING_OUTPUT_PER_1K)?;
            let input_rate = Decimal::from_str(input_price.as_str()?)
                .inspect_err(|e| tracing::warn!(%e, "malformed input_per_1k pricing"))
                .ok()?;
            let output_rate = Decimal::from_str(output_price.as_str()?)
                .inspect_err(|e| tracing::warn!(%e, "malformed output_per_1k pricing"))
                .ok()?;
            let input =
                Decimal::from(usage.prompt_tokens.unwrap_or(0)) * input_rate / Decimal::from(1000);
            let output = Decimal::from(usage.completion_tokens.unwrap_or(0)) * output_rate
                / Decimal::from(1000);
            Some(input + output)
        }
        _ => {
            tracing::warn!(
                capability = %capability,
                "cost computation not supported for capability"
            );
            None
        }
    }
}
```

(Append the `#[cfg(test)] mod tests` block — the existing `cost_computation` test plus the four new ones from Step 1 — at the end of the file, unchanged from what's there today plus the additions.)

- [ ] **Step 4: Fix the double-nesting in `admin/mod.rs`**

Modify `crates/godwit-api/src/admin/mod.rs` — change:

```rust
        .nest("/spend", spend::router())
```

to:

```rust
        .merge(spend::router())
```

- [ ] **Step 5: Run tests to verify they pass**

Run: `cargo test -p godwit-api --lib admin::spend::`
Expected: PASS (5 tests: the pre-existing `cost_computation` plus the 4 new ones).

Then run: `cargo check --workspace` and `DATABASE_URL="postgres://tmenard@localhost:5432/godwit" cargo test --workspace 2>&1 | tail -60`.
Expected: both pass cleanly, no regressions.

- [ ] **Step 6: Commit**

```bash
git add crates/godwit-api/src/admin/spend.rs crates/godwit-api/src/admin/mod.rs
git commit -m "feat(api): real spend aggregation with date/org/team/user scoping"
```

---

## Task 10: End-to-End Integration Coverage

**Files:**
- Modify: `crates/godwit-api/tests/router_integration.rs`

**Interfaces:** none new — this task only adds tests exercising Tasks 1-9 through the real, assembled router (the same `build_app`/`tower::ServiceExt::oneshot` pattern already established in this file).

- [ ] **Step 1: Add the login → refresh → logout → refresh-fails flow test**

Add to `crates/godwit-api/tests/router_integration.rs`:

```rust
#[sqlx::test(migrations = "../godwit-db/migrations")]
async fn login_refresh_logout_flow(pool: PgPool) {
    let app = build_app(pool.clone());

    let user = UserRepository::new(pool.clone())
        .create("flow@example.com", None, UserRole::User, None)
        .await
        .expect("create user");
    sqlx::query("UPDATE users SET password_hash = $2 WHERE id = $1")
        .bind(user.id)
        .bind(godwit_auth::api_keys::hash_password("hunter2"))
        .execute(&pool)
        .await
        .expect("set password");

    let login_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/auth/login")
                .header("content-type", "application/json")
                .body(Body::from(
                    serde_json::json!({"email": "flow@example.com", "password": "hunter2"}).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(login_response.status(), StatusCode::OK);
    let login_body = body_json(login_response).await;
    let refresh_token = login_body["refresh_token"].as_str().expect("refresh_token present").to_string();
    assert!(login_body["access_token"].as_str().is_some());

    // Refresh: exchanges the refresh token for a new pair.
    let refresh_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/auth/refresh")
                .header("content-type", "application/json")
                .body(Body::from(serde_json::json!({"refresh_token": refresh_token}).to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(refresh_response.status(), StatusCode::OK);
    let refresh_body = body_json(refresh_response).await;
    let rotated_refresh_token = refresh_body["refresh_token"].as_str().expect("rotated token present").to_string();
    assert_ne!(rotated_refresh_token, refresh_token, "refresh token should rotate on use");

    // The OLD refresh token is now invalid (single-use / rotated).
    let old_token_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/auth/refresh")
                .header("content-type", "application/json")
                .body(Body::from(serde_json::json!({"refresh_token": refresh_token}).to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(old_token_response.status(), StatusCode::UNAUTHORIZED);

    // Logout invalidates the rotated (current) refresh token.
    let logout_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/auth/logout")
                .header("content-type", "application/json")
                .body(Body::from(serde_json::json!({"refresh_token": rotated_refresh_token}).to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(logout_response.status(), StatusCode::OK);

    let post_logout_response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/auth/refresh")
                .header("content-type", "application/json")
                .body(Body::from(serde_json::json!({"refresh_token": rotated_refresh_token}).to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(post_logout_response.status(), StatusCode::UNAUTHORIZED);
}
```

- [ ] **Step 2: Add the organization + team + membership flow test**

```rust
#[sqlx::test(migrations = "../godwit-db/migrations")]
async fn super_admin_creates_org_team_and_manages_membership(pool: PgPool) {
    let app = build_app(pool.clone());
    let token = admin_token("super_admin");

    let create_org_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/organizations")
                .header("authorization", format!("Bearer {token}"))
                .header("content-type", "application/json")
                .body(Body::from(serde_json::json!({"name": "acme"}).to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(create_org_response.status(), StatusCode::OK);
    let org_id = body_json(create_org_response).await["data"]["id"]
        .as_str()
        .and_then(|s| Uuid::parse_str(s).ok())
        .expect("org id");

    let create_team_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/teams")
                .header("authorization", format!("Bearer {token}"))
                .header("content-type", "application/json")
                .body(Body::from(
                    serde_json::json!({"name": "engineering", "organization_id": org_id}).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(create_team_response.status(), StatusCode::OK);
    let team_id = body_json(create_team_response).await["data"]["id"]
        .as_str()
        .and_then(|s| Uuid::parse_str(s).ok())
        .expect("team id");

    let member = UserRepository::new(pool.clone())
        .create("member@example.com", None, UserRole::User, Some(org_id))
        .await
        .expect("create member");

    let add_member_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/api/v1/teams/{team_id}/members"))
                .header("authorization", format!("Bearer {token}"))
                .header("content-type", "application/json")
                .body(Body::from(
                    serde_json::json!({"user_id": member.id, "role": "member"}).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(add_member_response.status(), StatusCode::OK);

    let remove_member_response = app
        .oneshot(
            Request::builder()
                .method("DELETE")
                .uri(format!("/api/v1/teams/{team_id}/members/{}", member.id))
                .header("authorization", format!("Bearer {token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(remove_member_response.status(), StatusCode::OK);
}

#[sqlx::test(migrations = "../godwit-db/migrations")]
async fn team_admin_cannot_manage_a_team_they_do_not_administer(pool: PgPool) {
    let app = build_app(pool.clone());
    let org = OrganizationRepository::new(pool.clone()).create("acme", None).await.expect("create org");
    let team_a = godwit_db::repositories::teams::TeamRepository::new(pool.clone())
        .create(org.id, "team-a")
        .await
        .expect("create team a");
    let team_b = godwit_db::repositories::teams::TeamRepository::new(pool.clone())
        .create(org.id, "team-b")
        .await
        .expect("create team b");

    // A user who is team_admin of team_a, but not team_b.
    let claims = godwit_auth::jwt::Claims::new(Uuid::new_v4(), org.id, "team_admin");
    godwit_db::repositories::team_memberships::TeamMembershipRepository::new(pool.clone())
        .add_member(team_a.id, claims.user_id, "team_admin")
        .await
        .expect("add as team_admin of team_a");
    let token = godwit_auth::jwt::issue(JWT_SECRET, claims, chrono::Duration::minutes(15)).expect("issue jwt");

    let other_user = UserRepository::new(pool.clone())
        .create("other@example.com", None, UserRole::User, Some(org.id))
        .await
        .expect("create other user");

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/api/v1/teams/{}/members", team_b.id))
                .header("authorization", format!("Bearer {token}"))
                .header("content-type", "application/json")
                .body(Body::from(
                    serde_json::json!({"user_id": other_user.id, "role": "member"}).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::FORBIDDEN);
}
```

- [ ] **Step 3: Add the user-deletion-cascade end-to-end test**

```rust
#[sqlx::test(migrations = "../godwit-db/migrations")]
async fn deleting_a_user_via_the_api_cascades(pool: PgPool) {
    let app = build_app(pool.clone());
    let token = admin_token("super_admin");
    let org = OrganizationRepository::new(pool.clone()).create("acme", None).await.expect("create org");
    let user = UserRepository::new(pool.clone())
        .create("todelete@example.com", None, UserRole::User, Some(org.id))
        .await
        .expect("create user");

    let response = app
        .oneshot(
            Request::builder()
                .method("DELETE")
                .uri(format!("/api/v1/users/{}", user.id))
                .header("authorization", format!("Bearer {token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let remaining: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM users WHERE id = $1")
        .bind(user.id)
        .fetch_one(&pool)
        .await
        .expect("count users");
    assert_eq!(remaining, 0);
}

#[sqlx::test(migrations = "../godwit-db/migrations")]
async fn a_user_cannot_delete_their_own_account(pool: PgPool) {
    let app = build_app(pool.clone());
    let org = OrganizationRepository::new(pool.clone()).create("acme", None).await.expect("create org");
    let self_user = UserRepository::new(pool.clone())
        .create("self@example.com", None, UserRole::SuperAdmin, Some(org.id))
        .await
        .expect("create self user");
    let claims = godwit_auth::jwt::Claims::new(self_user.id, org.id, "super_admin");
    let token = godwit_auth::jwt::issue(JWT_SECRET, claims, chrono::Duration::minutes(15)).expect("issue jwt");

    let response = app
        .oneshot(
            Request::builder()
                .method("DELETE")
                .uri(format!("/api/v1/users/{}", self_user.id))
                .header("authorization", format!("Bearer {token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}
```

- [ ] **Step 4: Add the spend aggregation end-to-end test**

```rust
#[sqlx::test(migrations = "../godwit-db/migrations")]
async fn spend_aggregates_request_logs_scoped_to_caller(pool: PgPool) {
    let org = OrganizationRepository::new(pool.clone()).create("acme", None).await.expect("create org");
    let user_a = UserRepository::new(pool.clone())
        .create("a@example.com", None, UserRole::User, Some(org.id))
        .await
        .expect("create user a");
    let user_b = UserRepository::new(pool.clone())
        .create("b@example.com", None, UserRole::User, Some(org.id))
        .await
        .expect("create user b");

    for (user_id, cost, tokens_in, tokens_out) in [(user_a.id, "1.500000", 100, 50), (user_b.id, "2.500000", 200, 75)] {
        sqlx::query(
            "INSERT INTO request_logs (user_id, organization_id, model, provider, provider_model_id, tokens_in, tokens_out, cost_usd, duration_ms, status)
             VALUES ($1, $2, 'gpt-4o', 'openai', 'gpt-4o', $3, $4, $5, 100, 'success')"
        )
        .bind(user_id)
        .bind(org.id)
        .bind(tokens_in)
        .bind(tokens_out)
        .bind(cost.parse::<rust_decimal::Decimal>().unwrap())
        .execute(&pool)
        .await
        .expect("insert request log");
    }

    let app = build_app(pool.clone());

    // super_admin sees both rows.
    let super_admin_token = admin_token("super_admin");
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(format!("/api/v1/spend?organization_id={}", org.id))
                .header("authorization", format!("Bearer {super_admin_token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body = body_json(response).await;
    assert_eq!(body["data"].as_array().unwrap().len(), 2);

    // A plain "user" only ever sees their own row, regardless of query params.
    let user_claims = godwit_auth::jwt::Claims::new(user_a.id, org.id, "user");
    let user_token = godwit_auth::jwt::issue(JWT_SECRET, user_claims, chrono::Duration::minutes(15)).expect("issue jwt");
    let response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(format!("/api/v1/spend?user_id={}", user_b.id)) // attempt to see user_b's spend
                .header("authorization", format!("Bearer {user_token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body = body_json(response).await;
    let rows = body["data"].as_array().unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0]["user_id"], user_a.id.to_string());
}
```

- [ ] **Step 5: Run the full test suite**

Run: `cargo check --workspace`, then `DATABASE_URL="postgres://tmenard@localhost:5432/godwit" cargo test --workspace 2>&1 | tail -80`.
Expected: all pass, no regressions. Double-check the actual pass/fail counts before writing them anywhere — several tasks in the previous sub-project had reports that mis-stated or under-ran this exact check.

- [ ] **Step 6: Commit**

```bash
git add crates/godwit-api/tests/router_integration.rs
git commit -m "test(api): end-to-end coverage for refresh tokens, teams, user deletion, spend"
```

---

## Task 11: Final Documentation Pass

**Files:**
- Modify: `README.md`
- Modify: `config.example.yaml` (if `access_token_ttl_minutes`/`refresh_token_ttl_days` aren't already documented — check first, they likely already are)

**Interfaces:** none new — documentation only.

- [ ] **Step 1: Update `README.md`**

Update the "API" section to document: `POST /api/v1/auth/refresh`, `POST /api/v1/auth/logout`; `POST`/`PATCH /api/v1/organizations[/:id]`; the real `GET/POST/PATCH /api/v1/teams[/:id]` plus `POST`/`DELETE /api/v1/teams/:id/members[/:user_id]`; `GET/PATCH/DELETE /api/v1/users/:id`; the `?organization_id=` scoping behavior for `super_admin` on `users`/`teams`/`spend`; the real `GET /api/v1/spend` query parameters and response shape. Note that `POST /auth/login` and the OIDC callback now return both `access_token` and `refresh_token`.

- [ ] **Step 2: Verify `config.example.yaml` already documents the token TTL fields**

Run: `grep -n "access_token_ttl_minutes\|refresh_token_ttl_days" config.example.yaml`
Expected: both already present (they were added to `AuthConfig` during the original MVP scaffolding but never used until this plan). If either is missing, add it with the values used elsewhere in the file (`access_token_ttl_minutes: 15`, `refresh_token_ttl_days: 7`).

- [ ] **Step 3: Run the full test suite one more time**

Run: `cargo check --workspace`, then `DATABASE_URL="postgres://tmenard@localhost:5432/godwit" cargo test --workspace 2>&1 | tail -80`.
Expected: all pass, matching Task 10 Step 5's count exactly (no code changed in this task).

- [ ] **Step 4: Commit**

```bash
git add README.md config.example.yaml
git commit -m "docs: document refresh tokens, org/team/user CRUD, and spend endpoints"
```
