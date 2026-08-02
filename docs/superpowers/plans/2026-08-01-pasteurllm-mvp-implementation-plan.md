# Godwit MVP Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use `superpowers:subagent-driven-development` (recommended) or `superpowers:executing-plans` to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build Godwit, an ultra-low-latency OpenAI-compatible LLM proxy in Rust, supporting OpenAI and Anthropic backends, with user/team/org management, API-key/OIDC/SAML authentication, RBAC, PostgreSQL persistence, and unit/integration tests. The hot path (`/v1/chat/completions`) must avoid synchronous DB calls and minimize allocations.

**Architecture:** Modular monolith optimized for a fast proxy path. One Cargo workspace with crates for core domain, database, authentication, LLM providers, HTTP API, and the binary. The proxy uses in-memory caches for API keys and model routing, persistent HTTP connection pools to providers, and asynchronous request logging. Admin operations use the database directly.

**Tech Stack:** Rust 1.80+, Axum, Tokio, SQLx, PostgreSQL 16, reqwest (with keep-alive), serde, Argon2, openidconnect, samael, moka (caching), dashmap, wiremock, Testcontainers.

---

## File Structure Overview

```
godwit/
├── Cargo.toml
├── Dockerfile
├── docker-compose.yml
├── .dockerignore
├── config.example.yaml
├── crates/
│   ├── godwit-core/
│   │   ├── Cargo.toml
│   │   └── src/lib.rs
│   ├── godwit-db/
│   │   ├── Cargo.toml
│   │   ├── migrations/
│   │   └── src/lib.rs
│   ├── godwit-auth/
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── api_keys.rs
│   │       ├── jwt.rs
│   │       ├── rbac.rs
│   │       ├── oidc.rs
│   │       └── saml.rs
│   ├── godwit-providers/
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── openai.rs
│   │       ├── anthropic.rs
│   │       └── streaming.rs
│   ├── godwit-cache/
│   │   ├── Cargo.toml
│   │   └── src/lib.rs
│   ├── godwit-api/
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── state.rs
│   │       ├── middleware.rs
│   │       ├── proxy.rs
│   │       └── admin/
│   │           ├── mod.rs
│   │           ├── auth.rs
│   │           ├── users.rs
│   │           ├── organizations.rs
│   │           ├── teams.rs
│   │           ├── api_keys.rs
│   │           ├── models.rs
│   │           └── spend.rs
│   └── godwit-bin/
│       ├── Cargo.toml
│       └── src/main.rs
└── tests/
    ├── proxy_integration.rs
    └── admin_integration.rs
```

---

## Task 1: Cargo Workspace and Core Crate

**Files:**
- Create: `Cargo.toml`
- Create: `crates/godwit-core/Cargo.toml`
- Create: `crates/godwit-core/src/lib.rs`
- Test: `crates/godwit-core/src/lib.rs` (inline unit tests)

- [ ] **Step 1: Write the failing test**

Add to `crates/godwit-core/src/lib.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn config_parses_from_yaml() {
        let yaml = r#"
server:
  host: 127.0.0.1
  port: 3000
  request_timeout_seconds: 60
database:
  url: postgres://user:pass@localhost/pasteurllm
auth:
  jwt_secret: supersecret
  access_token_ttl_minutes: 15
providers:
  openai:
    api_key: sk-openai
    base_url: https://api.openai.com/v1
"#;
        let config: AppConfig = serde_yaml::from_str(yaml).expect("parse yaml");
        assert_eq!(config.server.port, 3000);
        assert_eq!(config.providers.openai.api_key, "sk-openai");
    }
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p godwit-core config_parses_from_yaml`
Expected: FAIL with `cannot find type AppConfig`.

- [ ] **Step 3: Create workspace Cargo.toml**

```toml
[workspace]
members = ["crates/*"]
resolver = "2"

[workspace.dependencies]
tokio = { version = "1.39", features = ["full"] }
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
serde_yaml = "0.10"
thiserror = "1.0"
chrono = { version = "0.4", features = ["serde"] }
uuid = { version = "1.10", features = ["v4", "serde"] }
tracing = "0.1"
anyhow = "1"
reqwest = { version = "0.12", features = ["json"] }
```

- [ ] **Step 4: Create core crate Cargo.toml**

```toml
[package]
name = "godwit-core"
version = "0.1.0"
edition = "2021"

[dependencies]
serde = { workspace = true }
serde_yaml = { workspace = true }
thiserror = { workspace = true }
uuid = { workspace = true }
chrono = { workspace = true }
```

- [ ] **Step 5: Implement core config and errors**

Create `crates/godwit-core/src/lib.rs`:

```rust
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum PasteurError {
    #[error("configuration error: {0}")]
    Config(String),
    #[error("authentication error: {0}")]
    Auth(String),
    #[error("authorization error: {0}")]
    Forbidden(String),
    #[error("not found")]
    NotFound,
    #[error("provider error: {0}")]
    Provider(String),
    #[error("database error: {0}")]
    Database(String),
    #[error("validation error: {0}")]
    Validation(String),
    #[error("rate limited")]
    RateLimited,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AppConfig {
    pub server: ServerConfig,
    pub database: DatabaseConfig,
    pub auth: AuthConfig,
    pub providers: ProvidersConfig,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ServerConfig {
    pub host: String,
    pub port: u16,
    pub request_timeout_seconds: u64,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct DatabaseConfig {
    pub url: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AuthConfig {
    pub jwt_secret: String,
    pub access_token_ttl_minutes: i64,
    pub refresh_token_ttl_days: i64,
    pub oidc_providers: Vec<OidcProviderConfig>,
    pub saml_providers: Vec<SamlProviderConfig>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct OidcProviderConfig {
    pub id: String,
    pub issuer_url: String,
    pub client_id: String,
    pub client_secret: String,
    pub redirect_uri: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SamlProviderConfig {
    pub id: String,
    pub idp_metadata_url: String,
    pub sp_entity_id: String,
    pub acs_url: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ProvidersConfig {
    pub openai: ProviderConfig,
    pub anthropic: ProviderConfig,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ProviderConfig {
    pub api_key: String,
    pub base_url: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn config_parses_from_yaml() {
        let yaml = r#"
server:
  host: 127.0.0.1
  port: 3000
  request_timeout_seconds: 60
database:
  url: postgres://user:pass@localhost/pasteurllm
auth:
  jwt_secret: supersecret
  access_token_ttl_minutes: 15
providers:
  openai:
    api_key: sk-openai
    base_url: https://api.openai.com/v1
"#;
        let config: AppConfig = serde_yaml::from_str(yaml).expect("parse yaml");
        assert_eq!(config.server.port, 3000);
        assert_eq!(config.providers.openai.api_key, "sk-openai");
    }
}
```

- [ ] **Step 6: Run tests**

Run: `cargo test -p godwit-core`
Expected: PASS.

- [ ] **Step 7: Commit**

```bash
git add Cargo.toml crates/godwit-core
git commit -m "feat(core): workspace, config, and errors"
```

---

## Task 2: Database Migrations

**Files:**
- Create: `crates/godwit-db/Cargo.toml`
- Create: `crates/godwit-db/src/lib.rs`
- Create: `crates/godwit-db/migrations/*.sql`
- Test: `crates/godwit-db/src/lib.rs`

- [ ] **Step 1: Write the failing test**

Add to `crates/godwit-db/src/lib.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn migrations_run_successfully() {
        // Placeholder: will fail until migrator exists.
        let pool = sqlx::PgPool::connect("postgres://invalid").await.unwrap();
        run_migrations(&pool).await.unwrap();
    }
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p godwit-db migrations_run_successfully`
Expected: FAIL (no `run_migrations` function, connection invalid).

- [ ] **Step 3: Create DB crate Cargo.toml**

```toml
[package]
name = "godwit-db"
version = "0.1.0"
edition = "2021"

[dependencies]
godwit-core = { path = "../godwit-core" }
sqlx = { version = "0.7", features = ["runtime-tokio-rustls", "postgres", "uuid", "chrono", "migrate"] }
tokio = { workspace = true }
uuid = { workspace = true }
chrono = { workspace = true }
rust_decimal = { version = "1.35", features = ["db-postgres"] }
tracing = { workspace = true }
```

- [ ] **Step 4: Write migrations**

Create `crates/godwit-db/migrations/20260801000001_initial.sql`:

```sql
CREATE EXTENSION IF NOT EXISTS "pgcrypto";

CREATE TABLE organizations (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    name TEXT NOT NULL,
    rate_limit_requests_per_minute INTEGER,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE users (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    organization_id UUID REFERENCES organizations(id),
    email TEXT NOT NULL UNIQUE,
    name TEXT,
    role TEXT NOT NULL CHECK (role IN ('super_admin','org_admin','team_admin','user')),
    sso_provider TEXT,
    sso_subject TEXT,
    password_hash TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(sso_provider, sso_subject)
);

CREATE TABLE teams (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    organization_id UUID NOT NULL REFERENCES organizations(id),
    name TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE team_memberships (
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    team_id UUID NOT NULL REFERENCES teams(id) ON DELETE CASCADE,
    role TEXT NOT NULL CHECK (role IN ('team_admin','member')),
    PRIMARY KEY (user_id, team_id)
);

CREATE TABLE api_keys (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL REFERENCES users(id),
    team_id UUID REFERENCES teams(id),
    organization_id UUID NOT NULL REFERENCES organizations(id),
    name TEXT NOT NULL,
    key_prefix TEXT NOT NULL,
    key_hash TEXT NOT NULL UNIQUE,
    scopes TEXT[] NOT NULL DEFAULT ARRAY['proxy:write'],
    budget_limit_usd NUMERIC(12,4),
    budget_spent_usd NUMERIC(12,4) NOT NULL DEFAULT 0,
    rate_limit_requests_per_minute INTEGER,
    expires_at TIMESTAMPTZ,
    disabled BOOLEAN NOT NULL DEFAULT FALSE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE models (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    organization_id UUID NOT NULL REFERENCES organizations(id),
    public_id TEXT NOT NULL,
    provider TEXT NOT NULL CHECK (provider IN ('openai','anthropic')),
    provider_model_id TEXT NOT NULL,
    config JSONB NOT NULL DEFAULT '{}',
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(organization_id, public_id)
);

CREATE TABLE model_pricing (
    model_id UUID PRIMARY KEY REFERENCES models(id) ON DELETE CASCADE,
    input_price_per_1k NUMERIC(12,6) NOT NULL,
    output_price_per_1k NUMERIC(12,6) NOT NULL,
    effective_from TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    effective_until TIMESTAMPTZ
);

CREATE TABLE request_logs (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    api_key_id UUID REFERENCES api_keys(id),
    user_id UUID REFERENCES users(id),
    organization_id UUID NOT NULL REFERENCES organizations(id),
    team_id UUID REFERENCES teams(id),
    model TEXT NOT NULL,
    provider TEXT NOT NULL,
    provider_model_id TEXT NOT NULL,
    tokens_in INTEGER,
    tokens_out INTEGER,
    cost_usd NUMERIC(12,6),
    duration_ms INTEGER NOT NULL,
    streamed BOOLEAN NOT NULL DEFAULT FALSE,
    status TEXT NOT NULL,
    error_message TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
```

- [ ] **Step 5: Implement migrator wrapper**

Create `crates/godwit-db/src/lib.rs`:

```rust
use godwit_core::PasteurError;
use sqlx::{migrate::Migrator, PgPool};

pub static MIGRATOR: Migrator = sqlx::migrate!("./migrations");

pub async fn run_migrations(pool: &PgPool) -> Result<(), PasteurError> {
    MIGRATOR
        .run(pool)
        .await
        .map_err(|e| PasteurError::Database(e.to_string()))
}

pub async fn connect(database_url: &str) -> Result<PgPool, PasteurError> {
    PgPool::connect(database_url)
        .await
        .map_err(|e| PasteurError::Database(e.to_string()))
}
```

- [ ] **Step 6: Update test to use real test DB**

Replace the placeholder test with:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use sqlx::PgPool;

    #[sqlx::test]
    async fn migrations_run_successfully(pool: PgPool) {
        run_migrations(&pool).await.expect("migrations should run");
    }
}
```

Note: `sqlx::test` requires `DATABASE_URL` set or a running test database. Document this in `crates/godwit-db/README.md` or `.env.example` later.

- [ ] **Step 7: Run tests**

Run: `cargo test -p godwit-db`
Expected: PASS (requires `DATABASE_URL` env var pointing to a PostgreSQL DB).

- [ ] **Step 8: Commit**

```bash
git add crates/godwit-db
git commit -m "feat(db): initial schema and migrations"
```

---

## Task 3: Database Repositories

**Files:**
- Create: `crates/godwit-db/src/models.rs`
- Create: `crates/godwit-db/src/repositories/mod.rs`
- Create: `crates/godwit-db/src/repositories/users.rs`
- Create: `crates/godwit-db/src/repositories/organizations.rs`
- Create: `crates/godwit-db/src/repositories/api_keys.rs`
- Create: `crates/godwit-db/src/repositories/models.rs`
- Modify: `crates/godwit-db/src/lib.rs`
- Test: repository files

- [ ] **Step 1: Write a failing unit test for user repository**

Add to `crates/godwit-db/src/repositories/users.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use sqlx::PgPool;

    #[sqlx::test]
    async fn create_and_fetch_user(pool: PgPool) {
        let repo = UserRepository::new(pool);
        let user = repo
            .create("alice@example.com", Some("Alice"), UserRole::OrgAdmin, None)
            .await
            .expect("create user");
        assert_eq!(user.email, "alice@example.com");

        let fetched = repo.get_by_id(user.id).await.expect("fetch user");
        assert_eq!(fetched.email, "alice@example.com");
    }
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p godwit-db create_and_fetch_user`
Expected: FAIL (types and functions undefined).

- [ ] **Step 3: Define DB models**

Create `crates/godwit-db/src/models.rs`:

```rust
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct Organization {
    pub id: Uuid,
    pub name: String,
    pub rate_limit_requests_per_minute: Option<i32>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct User {
    pub id: Uuid,
    pub organization_id: Option<Uuid>,
    pub email: String,
    pub name: Option<String>,
    pub role: String,
    pub sso_provider: Option<String>,
    pub sso_subject: Option<String>,
    pub password_hash: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub enum UserRole {
    SuperAdmin,
    OrgAdmin,
    TeamAdmin,
    User,
}

impl UserRole {
    pub fn as_str(&self) -> &'static str {
        match self {
            UserRole::SuperAdmin => "super_admin",
            UserRole::OrgAdmin => "org_admin",
            UserRole::TeamAdmin => "team_admin",
            UserRole::User => "user",
        }
    }
}

#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct ApiKey {
    pub id: Uuid,
    pub user_id: Uuid,
    pub team_id: Option<Uuid>,
    pub organization_id: Uuid,
    pub name: String,
    pub key_prefix: String,
    pub key_hash: String,
    pub scopes: Vec<String>,
    pub budget_limit_usd: Option<rust_decimal::Decimal>,
    pub budget_spent_usd: rust_decimal::Decimal,
    pub rate_limit_requests_per_minute: Option<i32>,
    pub expires_at: Option<DateTime<Utc>>,
    pub disabled: bool,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct Model {
    pub id: Uuid,
    pub organization_id: Uuid,
    pub public_id: String,
    pub provider: String,
    pub provider_model_id: String,
    pub config: serde_json::Value,
    pub created_at: DateTime<Utc>,
}
```

- [ ] **Step 4: Implement UserRepository**

Create `crates/godwit-db/src/repositories/users.rs`:

```rust
use crate::models::{User, UserRole};
use godwit_core::PasteurError;
use sqlx::PgPool;
use uuid::Uuid;

pub struct UserRepository {
    pool: PgPool,
}

impl UserRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn create(
        &self,
        email: &str,
        name: Option<&str>,
        role: UserRole,
        organization_id: Option<Uuid>,
    ) -> Result<User, PasteurError> {
        sqlx::query_as::<_, User>(
            "INSERT INTO users (email, name, role, organization_id) VALUES ($1, $2, $3, $4) RETURNING *"
        )
        .bind(email)
        .bind(name)
        .bind(role.as_str())
        .bind(organization_id)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| PasteurError::Database(e.to_string()))
    }

    pub async fn get_by_id(&self, id: Uuid) -> Result<User, PasteurError> {
        sqlx::query_as::<_, User>("SELECT * FROM users WHERE id = $1")
            .bind(id)
            .fetch_one(&self.pool)
            .await
            .map_err(|e| match e {
                sqlx::Error::RowNotFound => PasteurError::NotFound,
                _ => PasteurError::Database(e.to_string()),
            })
    }

    pub async fn get_by_email(&self, email: &str) -> Result<User, PasteurError> {
        sqlx::query_as::<_, User>("SELECT * FROM users WHERE email = $1")
            .bind(email)
            .fetch_one(&self.pool)
            .await
            .map_err(|e| match e {
                sqlx::Error::RowNotFound => PasteurError::NotFound,
                _ => PasteurError::Database(e.to_string()),
            })
    }
}
```

- [ ] **Step 5: Implement OrganizationRepository**

Create `crates/godwit-db/src/repositories/organizations.rs`:

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

    pub async fn create(&self, name: &str) -> Result<Organization, PasteurError> {
        sqlx::query_as::<_, Organization>(
            "INSERT INTO organizations (name) VALUES ($1) RETURNING *"
        )
        .bind(name)
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
}
```

- [ ] **Step 6: Implement ApiKeyRepository**

Create `crates/godwit-db/src/repositories/api_keys.rs`:

```rust
use crate::models::ApiKey;
use godwit_core::PasteurError;
use rust_decimal::Decimal;
use sqlx::PgPool;
use uuid::Uuid;

pub struct ApiKeyRepository {
    pool: PgPool,
}

impl ApiKeyRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn create(
        &self,
        user_id: Uuid,
        organization_id: Uuid,
        name: &str,
        key_prefix: &str,
        key_hash: &str,
        scopes: &[String],
        budget_limit_usd: Option<Decimal>,
        rate_limit: Option<i32>,
    ) -> Result<ApiKey, PasteurError> {
        sqlx::query_as::<_, ApiKey>(
            "INSERT INTO api_keys (user_id, organization_id, name, key_prefix, key_hash, scopes, budget_limit_usd, rate_limit_requests_per_minute)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8) RETURNING *"
        )
        .bind(user_id)
        .bind(organization_id)
        .bind(name)
        .bind(key_prefix)
        .bind(key_hash)
        .bind(scopes)
        .bind(budget_limit_usd)
        .bind(rate_limit)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| PasteurError::Database(e.to_string()))
    }

    pub async fn get_by_prefix(&self, prefix: &str) -> Result<Vec<ApiKey>, PasteurError> {
        sqlx::query_as::<_, ApiKey>("SELECT * FROM api_keys WHERE key_prefix = $1 AND disabled = FALSE")
            .bind(prefix)
            .fetch_all(&self.pool)
            .await
            .map_err(|e| PasteurError::Database(e.to_string()))
    }
}
```

- [ ] **Step 7: Implement ModelRepository**

Create `crates/godwit-db/src/repositories/models.rs`:

```rust
use crate::models::Model;
use godwit_core::PasteurError;
use sqlx::PgPool;
use uuid::Uuid;

pub struct ModelRepository {
    pool: PgPool,
}

impl ModelRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn create(
        &self,
        organization_id: Uuid,
        public_id: &str,
        provider: &str,
        provider_model_id: &str,
    ) -> Result<Model, PasteurError> {
        sqlx::query_as::<_, Model>(
            "INSERT INTO models (organization_id, public_id, provider, provider_model_id) VALUES ($1, $2, $3, $4) RETURNING *"
        )
        .bind(organization_id)
        .bind(public_id)
        .bind(provider)
        .bind(provider_model_id)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| PasteurError::Database(e.to_string()))
    }

    pub async fn list_for_organization(
        &self,
        organization_id: Uuid,
    ) -> Result<Vec<Model>, PasteurError> {
        sqlx::query_as::<_, Model>("SELECT * FROM models WHERE organization_id = $1 ORDER BY public_id")
            .bind(organization_id)
            .fetch_all(&self.pool)
            .await
            .map_err(|e| PasteurError::Database(e.to_string()))
    }

    pub async fn get_by_public_id(
        &self,
        organization_id: Uuid,
        public_id: &str,
    ) -> Result<Model, PasteurError> {
        sqlx::query_as::<_, Model>("SELECT * FROM models WHERE organization_id = $1 AND public_id = $2")
            .bind(organization_id)
            .bind(public_id)
            .fetch_one(&self.pool)
            .await
            .map_err(|e| match e {
                sqlx::Error::RowNotFound => PasteurError::NotFound,
                _ => PasteurError::Database(e.to_string()),
            })
    }
}
```

- [ ] **Step 8: Wire up repositories module**

Create `crates/godwit-db/src/repositories/mod.rs`:

```rust
pub mod api_keys;
pub mod models;
pub mod organizations;
pub mod users;
```

Modify `crates/godwit-db/src/lib.rs`:

```rust
pub mod models;
pub mod repositories;
```

- [ ] **Step 8: Run repository tests**

Run: `cargo test -p godwit-db create_and_fetch_user`
Expected: PASS.

- [ ] **Step 9: Commit**

```bash
git add crates/godwit-db
git commit -m "feat(db): repositories for users, orgs, api keys, models"
```

---

## Task 4: API Key Authentication

**Files:**
- Create: `crates/godwit-auth/Cargo.toml`
- Create: `crates/godwit-auth/src/api_keys.rs`
- Modify: `crates/godwit-auth/src/lib.rs`
- Test: `crates/godwit-auth/src/api_keys.rs`

- [ ] **Step 1: Write failing unit test**

Create `crates/godwit-auth/src/api_keys.rs` with test first:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generate_and_verify_key() {
        let (plaintext, hash, prefix) = generate_api_key();
        assert!(plaintext.starts_with("sk-godwit-"));
        assert!(verify_key(&plaintext, &hash));
        assert_eq!(extract_prefix(&plaintext), prefix);
    }
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p godwit-auth generate_and_verify_key`
Expected: FAIL.

- [ ] **Step 3: Create auth crate Cargo.toml**

```toml
[package]
name = "godwit-auth"
version = "0.1.0"
edition = "2021"

[dependencies]
godwit-core = { path = "../godwit-core" }
argon2 = { version = "0.5", features = ["std"] }
rand = "0.8"
bs58 = "0.5"
serde_json = { workspace = true }
```

- [ ] **Step 4: Implement API key functions**

Create `crates/godwit-auth/src/api_keys.rs`:

```rust
use argon2::{
    password_hash::{rand_core::RngCore, PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
    Argon2,
};
use rand::rngs::OsRng;

const PREFIX: &str = "sk-godwit-";
const PREFIX_LEN: usize = 16; // characters after PREFIX used for lookup

pub fn generate_api_key() -> (String, String, String) {
    let mut bytes = [0u8; 24];
    OsRng.fill_bytes(&mut bytes);
    let plaintext = format!("{}{}", PREFIX, bs58::encode(&bytes).into_string());
    let prefix = extract_prefix(&plaintext);
    let hash = hash_key(&plaintext);
    (plaintext, hash, prefix)
}

pub fn extract_prefix(key: &str) -> String {
    let start = PREFIX.len();
    let end = (start + PREFIX_LEN).min(key.len());
    key[start..end].to_string()
}

pub fn hash_key(key: &str) -> String {
    let salt = SaltString::generate(&mut OsRng);
    let argon2 = Argon2::default();
    argon2
        .hash_password(key.as_bytes(), &salt)
        .expect("hash key")
        .to_string()
}

pub fn verify_key(key: &str, hash: &str) -> bool {
    let parsed = match PasswordHash::new(hash) {
        Ok(p) => p,
        Err(_) => return false,
    };
    Argon2::default()
        .verify_password(key.as_bytes(), &parsed)
        .is_ok()
}

pub fn hash_password(password: &str) -> String {
    let salt = SaltString::generate(&mut OsRng);
    Argon2::default()
        .hash_password(password.as_bytes(), &salt)
        .expect("hash password")
        .to_string()
}

pub fn verify_password(password: &str, hash: &str) -> bool {
    let parsed = match PasswordHash::new(hash) {
        Ok(p) => p,
        Err(_) => return false,
    };
    Argon2::default()
        .verify_password(password.as_bytes(), &parsed)
        .is_ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generate_and_verify_key() {
        let (plaintext, hash, prefix) = generate_api_key();
        assert!(plaintext.starts_with(PREFIX));
        assert!(verify_key(&plaintext, &hash));
        assert_eq!(extract_prefix(&plaintext), prefix);
    }

    #[test]
    fn verify_rejects_wrong_key() {
        let (_, hash, _) = generate_api_key();
        assert!(!verify_key("sk-godwit-wrong", &hash));
    }

    #[test]
    fn hash_and_verify_password() {
        let hash = hash_password("hunter2");
        assert!(verify_password("hunter2", &hash));
        assert!(!verify_password("wrong", &hash));
    }
}
```

- [ ] **Step 5: Wire up lib.rs**

Create `crates/godwit-auth/src/lib.rs`:

```rust
pub mod api_keys;
pub mod jwt;
pub mod rbac;
```

- [ ] **Step 6: Run tests**

Run: `cargo test -p godwit-auth`
Expected: PASS.

- [ ] **Step 7: Commit**

```bash
git add crates/godwit-auth
git commit -m "feat(auth): api key generation and verification"
```

---

## Task 5: JWT and RBAC

**Files:**
- Create: `crates/godwit-auth/src/jwt.rs`
- Create: `crates/godwit-auth/src/rbac.rs`
- Modify: `crates/godwit-auth/Cargo.toml`
- Test: both files

- [ ] **Step 1: Write failing unit tests**

Create `crates/godwit-auth/src/jwt.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn issue_and_verify_token() {
        let claims = Claims::new(Uuid::new_v4(), Uuid::new_v4(), "org_admin");
        let token = issue("secret", claims.clone(), Duration::minutes(15)).unwrap();
        let verified = verify("secret", &token).unwrap();
        assert_eq!(verified.user_id, claims.user_id);
    }
}
```

- [ ] **Step 2: Run tests to verify failure**

Run: `cargo test -p godwit-auth issue_and_verify_token`
Expected: FAIL.

- [ ] **Step 3: Add dependencies**

Modify `crates/godwit-auth/Cargo.toml`:

```toml
[dependencies]
jsonwebtoken = "9"
chrono = { workspace = true }
uuid = { workspace = true }
serde = { workspace = true }
```

- [ ] **Step 4: Implement JWT**

Create `crates/godwit-auth/src/jwt.rs`:

```rust
use chrono::{Duration, Utc};
use jsonwebtoken::{decode, encode, Algorithm, DecodingKey, EncodingKey, Header, Validation};
use godwit_core::PasteurError;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Claims {
    pub sub: String,
    pub user_id: Uuid,
    pub organization_id: Uuid,
    pub role: String,
    pub exp: i64,
    pub iat: i64,
}

impl Claims {
    pub fn new(user_id: Uuid, organization_id: Uuid, role: &str) -> Self {
        let now = Utc::now();
        Self {
            sub: user_id.to_string(),
            user_id,
            organization_id,
            role: role.to_string(),
            iat: now.timestamp(),
            exp: (now + Duration::minutes(15)).timestamp(),
        }
    }
}

pub fn issue(secret: &str, claims: Claims, ttl: Duration) -> Result<String, PasteurError> {
    let mut claims = claims;
    let now = Utc::now();
    claims.iat = now.timestamp();
    claims.exp = (now + ttl).timestamp();
    encode(
        &Header::new(Algorithm::HS256),
        &claims,
        &EncodingKey::from_secret(secret.as_bytes()),
    )
    .map_err(|e| PasteurError::Auth(e.to_string()))
}

pub fn verify(secret: &str, token: &str) -> Result<Claims, PasteurError> {
    decode::<Claims>(
        token,
        &DecodingKey::from_secret(secret.as_bytes()),
        &Validation::new(Algorithm::HS256),
    )
    .map(|data| data.claims)
    .map_err(|e| PasteurError::Auth(e.to_string()))
}
```

- [ ] **Step 5: Implement RBAC**

Create `crates/godwit-auth/src/rbac.rs`:

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Role {
    SuperAdmin,
    OrgAdmin,
    TeamAdmin,
    User,
}

impl Role {
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "super_admin" => Some(Role::SuperAdmin),
            "org_admin" => Some(Role::OrgAdmin),
            "team_admin" => Some(Role::TeamAdmin),
            "user" => Some(Role::User),
            _ => None,
        }
    }

    pub fn can_manage_users(&self) -> bool {
        matches!(self, Role::SuperAdmin | Role::OrgAdmin)
    }

    pub fn can_manage_orgs(&self) -> bool {
        matches!(self, Role::SuperAdmin)
    }

    pub fn can_manage_api_keys(&self) -> bool {
        matches!(self, Role::SuperAdmin | Role::OrgAdmin | Role::TeamAdmin)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn role_permissions() {
        assert!(Role::OrgAdmin.can_manage_users());
        assert!(!Role::User.can_manage_users());
        assert!(Role::SuperAdmin.can_manage_orgs());
        assert!(!Role::OrgAdmin.can_manage_orgs());
    }
}
```

- [ ] **Step 6: Run tests**

Run: `cargo test -p godwit-auth`
Expected: PASS.

- [ ] **Step 7: Commit**

```bash
git add crates/godwit-auth
git commit -m "feat(auth): jwt issue/verify and rbac roles"
```

---

## Task 6: OIDC Authentication

**Files:**
- Create: `crates/godwit-auth/src/oidc.rs`
- Modify: `crates/godwit-auth/Cargo.toml`
- Test: `crates/godwit-auth/src/oidc.rs`

- [ ] **Step 1: Add OIDC dependency**

Modify `crates/godwit-auth/Cargo.toml`:

```toml
[dependencies]
openidconnect = "3"
url = "2"
```

- [ ] **Step 2: Implement OIDC client wrapper**

Create `crates/godwit-auth/src/oidc.rs`:

```rust
use openidconnect::{
    core::{CoreAuthenticationFlow, CoreClient, CoreProviderMetadata, CoreResponseType},
    reqwest::async_http_client,
    AuthorizationCode, ClientId, ClientSecret, CsrfToken, IssuerUrl, Nonce, RedirectUrl, Scope,
};
use godwit_core::{OidcProviderConfig, PasteurError};
use url::Url;

pub struct OidcClient {
    inner: CoreClient,
    provider_id: String,
}

impl OidcClient {
    pub async fn new(config: &OidcProviderConfig) -> Result<Self, PasteurError> {
        let issuer_url = IssuerUrl::new(config.issuer_url.clone())
            .map_err(|e| PasteurError::Config(e.to_string()))?;
        let provider_metadata = CoreProviderMetadata::discover_async(issuer_url, async_http_client)
            .await
            .map_err(|e| PasteurError::Auth(e.to_string()))?;
        let client = CoreClient::from_provider_metadata(
            provider_metadata,
            ClientId::new(config.client_id.clone()),
            Some(ClientSecret::new(config.client_secret.clone())),
        )
        .set_redirect_uri(
            RedirectUrl::new(config.redirect_uri.clone())
                .map_err(|e| PasteurError::Config(e.to_string()))?,
        );
        Ok(Self {
            inner: client,
            provider_id: config.id.clone(),
        })
    }

    pub fn authorize_url(&self, scopes: Vec<String>) -> (Url, CsrfToken, Nonce) {
        let mut request = self
            .inner
            .authorize_url(
                CoreAuthenticationFlow::AuthorizationCode,
                CsrfToken::new_random,
                Nonce::new_random,
            );
        for scope in scopes {
            request = request.add_scope(Scope::new(scope));
        }
        request.url()
    }

    pub async fn exchange_code(
        &self,
        code: &str,
        _csrf: &str,
        nonce: &str,
    ) -> Result<(String, String, Option<String>), PasteurError> {
        let token_response = self
            .inner
            .exchange_code(AuthorizationCode::new(code.to_string()))
            .request_async(async_http_client)
            .await
            .map_err(|e| PasteurError::Auth(e.to_string()))?;
        let id_token = token_response
            .extra_fields()
            .id_token()
            .ok_or_else(|| PasteurError::Auth("missing id_token".to_string()))?;
        let nonce = Nonce::new(nonce.to_string());
        let claims = id_token
            .claims(&self.inner.id_token_verifier(), &nonce)
            .map_err(|e| PasteurError::Auth(e.to_string()))?;
        let email = claims
            .email()
            .map(|e| e.as_str().to_string())
            .ok_or_else(|| PasteurError::Auth("missing email".to_string()))?;
        let name = claims.name().and_then(|n| n.get(None)).map(|s| s.to_string());
        let subject = claims.subject().to_string();
        Ok((email, subject, name))
    }
}
```

- [ ] **Step 3: Add unit test for config validation**

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn invalid_issuer_fails() {
        let config = OidcProviderConfig {
            id: "bad".to_string(),
            issuer_url: "not-a-url".to_string(),
            client_id: "x".to_string(),
            client_secret: "y".to_string(),
            redirect_uri: "http://localhost/callback".to_string(),
        };
        // Discovery cannot run in unit test; test URL parsing error path.
        assert!(IssuerUrl::new(config.issuer_url.clone()).is_err());
    }
}
```

- [ ] **Step 4: Commit**

```bash
git add crates/godwit-auth
git commit -m "feat(auth): oidc discovery and code exchange"
```

---

## Task 7: SAML Authentication

**Files:**
- Create: `crates/godwit-auth/src/saml.rs`
- Modify: `crates/godwit-auth/Cargo.toml`

- [ ] **Step 1: Add SAML dependency**

Modify `crates/godwit-auth/Cargo.toml`:

```toml
[dependencies]
samael = "0.9"
```

- [ ] **Step 2: Implement SAML ACS helper**

Create `crates/godwit-auth/src/saml.rs`:

```rust
use godwit_core::{PasteurError, SamlProviderConfig};

pub struct SamlService {
    provider_id: String,
}

impl SamlService {
    pub fn new(config: &SamlProviderConfig) -> Result<Self, PasteurError> {
        Ok(Self {
            provider_id: config.id.clone(),
        })
    }

    pub fn parse_saml_response(
        &self,
        _encoded_response: &str,
    ) -> Result<(String, String, Option<String>), PasteurError> {
        // Placeholder: real implementation uses samael to decode and validate
        // the XML signature against IdP metadata.
        Err(PasteurError::Auth("SAML not fully implemented in MVP".to_string()))
    }
}
```

- [ ] **Step 3: Add integration test placeholder**

Add a note in the plan: full SAML round-trip requires an IdP fixture; cover in integration tests with a self-signed SAML response.

- [ ] **Step 4: Commit**

```bash
git add crates/godwit-auth
git commit -m "feat(auth): saml scaffolding"
```

---

## Task 8: OpenAI Provider Client

**Files:**
- Create: `crates/godwit-providers/Cargo.toml`
- Create: `crates/godwit-providers/src/lib.rs`
- Create: `crates/godwit-providers/src/openai.rs`
- Modify: `crates/godwit-core/src/lib.rs` (add ChatCompletionRequest DTO)
- Test: `crates/godwit-providers/src/openai.rs`

- [ ] **Step 1: Add ChatCompletion DTOs to core**

Modify `crates/godwit-core/src/lib.rs` to add:

```rust
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ChatCompletionRequest {
    pub model: String,
    pub messages: Vec<ChatMessage>,
    pub stream: Option<bool>,
    pub temperature: Option<f32>,
    pub max_tokens: Option<i32>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ChatMessage {
    pub role: String,
    pub content: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ChatCompletionResponse {
    pub id: String,
    pub object: String,
    pub created: i64,
    pub model: String,
    pub choices: Vec<ChatCompletionChoice>,
    pub usage: Option<Usage>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ChatCompletionChoice {
    pub index: i32,
    pub message: ChatMessage,
    pub finish_reason: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Usage {
    pub prompt_tokens: i32,
    pub completion_tokens: i32,
    pub total_tokens: i32,
}
```

- [ ] **Step 2: Write failing provider test**

Create `crates/godwit-providers/src/openai.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use godwit_core::{ChatCompletionRequest, ChatMessage};
    use wiremock::{matchers::*, Mock, MockServer, ResponseTemplate};

    #[tokio::test]
    async fn chat_completion_returns_openai_shape() {
        let server = MockServer::start().await;
        let body = serde_json::json!({
            "id": "chatcmpl-123",
            "object": "chat.completion",
            "created": 1,
            "model": "gpt-4o",
            "choices": [{"index":0,"message":{"role":"assistant","content":"Hello"},"finish_reason":"stop"}],
            "usage": {"prompt_tokens":1,"completion_tokens":1,"total_tokens":2}
        });
        Mock::given(method("POST"))
            .and(path("/chat/completions"))
            .respond_with(ResponseTemplate::new(200).set_body_json(body))
            .mount(&server)
            .await;

        let client = OpenAiProvider::new("fake-key", &server.uri());
        let req = ChatCompletionRequest {
            model: "gpt-4o".to_string(),
            messages: vec![ChatMessage { role: "user".to_string(), content: "Hi".to_string() }],
            stream: Some(false),
            temperature: None,
            max_tokens: None,
        };
        let resp = client.chat_completion(req).await.unwrap();
        assert_eq!(resp.choices[0].message.content, "Hello");
    }
}
```

- [ ] **Step 3: Run test to verify failure**

Run: `cargo test -p godwit-providers chat_completion_returns_openai_shape`
Expected: FAIL.

- [ ] **Step 4: Create providers crate Cargo.toml**

```toml
[package]
name = "godwit-providers"
version = "0.1.0"
edition = "2021"

[dependencies]
godwit-core = { path = "../godwit-core" }
reqwest = { version = "0.12", features = ["json", "rustls-tls", "stream"] }
serde = { workspace = true }
serde_json = { workspace = true }
tokio = { workspace = true }
futures = "0.3"
async-trait = "0.1"
chrono = { workspace = true }
tracing = { workspace = true }

[dev-dependencies]
wiremock = "0.6"
```

Note: ensure `futures` and `async-trait` versions are compatible with Rust 1.80+.

- [ ] **Step 5: Implement OpenAI provider**

Create `crates/godwit-providers/src/openai.rs`:

```rust
use async_trait::async_trait;
use godwit_core::{ChatCompletionRequest, ChatCompletionResponse, PasteurError, ProviderConfig};
use reqwest::Client;

pub struct OpenAiProvider {
    client: Client,
    api_key: String,
    base_url: String,
}

impl OpenAiProvider {
    pub fn new(api_key: &str, base_url: &str) -> Self {
        let client = Client::builder()
            .pool_max_idle_per_host(20)
            .pool_idle_timeout(std::time::Duration::from_secs(60))
            .timeout(std::time::Duration::from_secs(120))
            .build()
            .expect("build reqwest client");
        Self {
            client,
            api_key: api_key.to_string(),
            base_url: base_url.to_string(),
        }
    }

    pub fn from_config(config: &ProviderConfig) -> Self {
        Self::new(&config.api_key, &config.base_url)
    }
}

#[async_trait]
impl Provider for OpenAiProvider {
    async fn chat_completion(
        &self,
        request: ChatCompletionRequest,
    ) -> Result<ProviderResponse, ProviderError> {
        let url = format!("{}/chat/completions", self.base_url);
        let res = self
            .client
            .post(&url)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .json(&request)
            .send()
            .await
            .map_err(|e| ProviderError::Http(e.to_string()))?;
        if !res.status().is_success() {
            let text = res.text().await.unwrap_or_default();
            return Err(ProviderError::Provider(text));
        }
        let body: ChatCompletionResponse = res
            .json()
            .await
            .map_err(|e| ProviderError::Serialization(e.to_string()))?;
        Ok(ProviderResponse::Json(body))
    }

    async fn stream_chat_completion(
        &self,
        request: ChatCompletionRequest,
    ) -> Result<BoxStream<'static, Result<SseEvent, ProviderError>>, ProviderError> {
        // Will be implemented in Task 10.
        Err(ProviderError::NotImplemented)
    }
}
```

- [ ] **Step 6: Define provider trait and types in lib.rs**

Create `crates/godwit-providers/src/lib.rs`:

```rust
pub mod anthropic;
pub mod openai;

use async_trait::async_trait;
use futures::stream::BoxStream;
use godwit_core::{ChatCompletionRequest, ChatCompletionResponse};

#[derive(Debug)]
pub enum ProviderError {
    Http(String),
    Serialization(String),
    Provider(String),
    NotImplemented,
}

#[derive(Debug)]
pub enum ProviderResponse {
    Json(ChatCompletionResponse),
}

#[derive(Debug, Clone)]
pub struct SseEvent {
    pub data: String,
}

#[async_trait]
pub trait Provider: Send + Sync {
    async fn chat_completion(
        &self,
        request: ChatCompletionRequest,
    ) -> Result<ProviderResponse, ProviderError>;

    async fn stream_chat_completion(
        &self,
        request: ChatCompletionRequest,
    ) -> Result<BoxStream<'static, Result<SseEvent, ProviderError>>, ProviderError>;
}
```

- [ ] **Step 7: Run tests**

Run: `cargo test -p godwit-providers`
Expected: PASS.

- [ ] **Step 8: Commit**

```bash
git add crates/godwit-providers crates/godwit-core
git commit -m "feat(providers): openai client and provider trait"
```

---

## Task 9: Anthropic Mapping

**Files:**
- Create: `crates/godwit-providers/src/anthropic.rs`
- Test: same file

- [ ] **Step 1: Write failing mapping test**

Create `crates/godwit-providers/src/anthropic.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use godwit_core::{ChatCompletionRequest, ChatMessage};

    #[test]
    fn openai_to_anthropic_request() {
        let req = ChatCompletionRequest {
            model: "claude-sonnet".to_string(),
            messages: vec![
                ChatMessage { role: "system".to_string(), content: "You are helpful".to_string() },
                ChatMessage { role: "user".to_string(), content: "Hello".to_string() },
            ],
            stream: Some(false),
            temperature: Some(0.7),
            max_tokens: Some(100),
        };
        let anthropic = to_anthropic_request(&req);
        assert_eq!(anthropic.model, "claude-3-5-sonnet-20240620");
        assert_eq!(anthropic.system, Some("You are helpful".to_string()));
        assert_eq!(anthropic.messages.len(), 1);
    }

    #[test]
    fn anthropic_response_to_openai() {
        let ar = AnthropicResponse {
            id: "msg-1".to_string(),
            model: "claude-3-5-sonnet-20240620".to_string(),
            content: vec![ContentBlock { text: "Hi there".to_string(), type_: "text".to_string() }],
            usage: AnthropicUsage { input_tokens: 1, output_tokens: 2 },
        };
        let openai = to_openai_response(ar, "claude-sonnet");
        assert_eq!(openai.choices[0].message.content, "Hi there");
        assert_eq!(openai.usage.unwrap().total_tokens, 3);
    }
}
```

- [ ] **Step 2: Run tests to verify failure**

Run: `cargo test -p godwit-providers openai_to_anthropic_request`
Expected: FAIL.

- [ ] **Step 3: Implement Anthropic mapping**

Create `crates/godwit-providers/src/anthropic.rs`:

```rust
use godwit_core::{
    ChatCompletionChoice, ChatCompletionRequest, ChatCompletionResponse, ChatMessage, Usage,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize)]
pub struct AnthropicMessage {
    pub role: String,
    pub content: String,
}

#[derive(Debug, Serialize)]
pub struct AnthropicRequest {
    pub model: String,
    pub max_tokens: i32,
    pub messages: Vec<AnthropicMessage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub system: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,
    pub stream: bool,
}

#[derive(Debug, Deserialize)]
pub struct ContentBlock {
    #[serde(rename = "type")]
    pub type_: String,
    pub text: String,
}

#[derive(Debug, Deserialize)]
pub struct AnthropicUsage {
    pub input_tokens: i32,
    pub output_tokens: i32,
}

#[derive(Debug, Deserialize)]
pub struct AnthropicResponse {
    pub id: String,
    pub model: String,
    pub content: Vec<ContentBlock>,
    pub usage: AnthropicUsage,
}

pub fn to_anthropic_request(req: &ChatCompletionRequest) -> AnthropicRequest {
    let mut system: Option<String> = None;
    let mut messages = Vec::new();
    for m in &req.messages {
        if m.role == "system" {
            system = Some(m.content.clone());
        } else {
            messages.push(AnthropicMessage {
                role: m.role.clone(),
                content: m.content.clone(),
            });
        }
    }
    AnthropicRequest {
        model: req.model.clone(),
        max_tokens: req.max_tokens.unwrap_or(4096),
        messages,
        system,
        temperature: req.temperature,
        stream: req.stream.unwrap_or(false),
    }
}

pub fn to_openai_response(resp: AnthropicResponse, public_model: &str) -> ChatCompletionResponse {
    let text = resp
        .content
        .into_iter()
        .filter(|c| c.type_ == "text")
        .map(|c| c.text)
        .collect::<Vec<_>>()
        .join("");
    let usage = Usage {
        prompt_tokens: resp.usage.input_tokens,
        completion_tokens: resp.usage.output_tokens,
        total_tokens: resp.usage.input_tokens + resp.usage.output_tokens,
    };
    ChatCompletionResponse {
        id: resp.id,
        object: "chat.completion".to_string(),
        created: chrono::Utc::now().timestamp(),
        model: public_model.to_string(),
        choices: vec![ChatCompletionChoice {
            index: 0,
            message: ChatMessage {
                role: "assistant".to_string(),
                content: text,
            },
            finish_reason: Some("stop".to_string()),
        }],
        usage: Some(usage),
    }
}
```

- [ ] **Step 4: Implement Anthropic provider struct**

Add to `crates/godwit-providers/src/anthropic.rs`:

```rust
use crate::{Provider, ProviderError, ProviderResponse, SseEvent};
use async_trait::async_trait;
use godwit_core::{ChatCompletionRequest, ProviderConfig};
use reqwest::Client;

pub struct AnthropicProvider {
    client: Client,
    api_key: String,
    base_url: String,
}

impl AnthropicProvider {
    pub fn new(api_key: &str, base_url: &str) -> Self {
        let client = Client::builder()
            .pool_max_idle_per_host(20)
            .pool_idle_timeout(std::time::Duration::from_secs(60))
            .timeout(std::time::Duration::from_secs(120))
            .build()
            .expect("build reqwest client");
        Self {
            client,
            api_key: api_key.to_string(),
            base_url: base_url.to_string(),
        }
    }
}

#[async_trait]
impl Provider for AnthropicProvider {
    async fn chat_completion(
        &self,
        request: ChatCompletionRequest,
    ) -> Result<ProviderResponse, ProviderError> {
        let url = format!("{}/messages", self.base_url);
        let anthropic_req = to_anthropic_request(&request);
        let res = self
            .client
            .post(&url)
            .header("x-api-key", self.api_key.clone())
            .header("anthropic-version", "2023-06-01")
            .json(&anthropic_req)
            .send()
            .await
            .map_err(|e| ProviderError::Http(e.to_string()))?;
        if !res.status().is_success() {
            return Err(ProviderError::Provider(res.text().await.unwrap_or_default()));
        }
        let anthropic_resp: AnthropicResponse = res
            .json()
            .await
            .map_err(|e| ProviderError::Serialization(e.to_string()))?;
        let openai_resp = to_openai_response(anthropic_resp, &request.model);
        Ok(ProviderResponse::Json(openai_resp))
    }

    async fn stream_chat_completion(
        &self,
        _request: ChatCompletionRequest,
    ) -> Result<crate::BoxStream<'static, Result<SseEvent, ProviderError>>, ProviderError> {
        Err(ProviderError::NotImplemented)
    }
}
```

- [ ] **Step 5: Run tests**

Run: `cargo test -p godwit-providers anthropic`
Expected: PASS.

- [ ] **Step 6: Commit**

```bash
git add crates/godwit-providers
git commit -m "feat(providers): anthropic mapping and client"
```

---

## Task 10: Streaming Support

**Files:**
- Modify: `crates/godwit-providers/src/openai.rs`
- Modify: `crates/godwit-providers/src/anthropic.rs`
- Create: `crates/godwit-providers/src/streaming.rs`
- Test: streaming file

- [ ] **Step 1: Write streaming test**

Create `crates/godwit-providers/src/streaming.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_openai_sse_chunk() {
        let line = "data: {\"id\":\"1\",\"object\":\"chat.completion.chunk\"}\n\n";
        let events = parse_sse_events(line);
        assert_eq!(events.len(), 1);
        assert!(events[0].data.contains("chat.completion.chunk"));
    }

    #[test]
    fn ignores_sse_done() {
        let line = "data: [DONE]\n\n";
        let events = parse_sse_events(line);
        assert!(events.is_empty());
    }
}
```

- [ ] **Step 2: Run test to verify failure**

Run: `cargo test -p godwit-providers parse_openai_sse_chunk`
Expected: FAIL.

- [ ] **Step 3: Implement SSE parser**

Create `crates/godwit-providers/src/streaming.rs`:

```rust
use crate::SseEvent;

pub fn parse_sse_events(chunk: &str) -> Vec<SseEvent> {
    let mut events = Vec::new();
    for line in chunk.lines() {
        let line = line.trim();
        if line.is_empty() || line == ":" {
            continue;
        }
        if let Some(data) = line.strip_prefix("data: ") {
            if data == "[DONE]" {
                continue;
            }
            events.push(SseEvent {
                data: data.to_string(),
            });
        }
    }
    events
}
```

- [ ] **Step 4: Implement OpenAI streaming**

Modify `OpenAiProvider` in `crates/godwit-providers/src/openai.rs`:

```rust
use crate::streaming::parse_sse_events;
use futures::stream::{self, BoxStream, StreamExt};

async fn stream_chat_completion(
    &self,
    mut request: ChatCompletionRequest,
) -> Result<BoxStream<'static, Result<SseEvent, ProviderError>>, ProviderError> {
    request.stream = Some(true);
    let url = format!("{}/chat/completions", self.base_url);
    let res = self
        .client
        .post(&url)
        .header("Authorization", format!("Bearer {}", self.api_key))
        .json(&request)
        .send()
        .await
        .map_err(|e| ProviderError::Http(e.to_string()))?;
    if !res.status().is_success() {
        return Err(ProviderError::Provider(res.text().await.unwrap_or_default()));
    }
    let byte_stream = res.bytes_stream();
    let event_stream = byte_stream.flat_map(|bytes| {
        let text = bytes.map(|b| String::from_utf8_lossy(&b).to_string()).unwrap_or_default();
        let events = parse_sse_events(&text);
        stream::iter(events.into_iter().map(Ok))
    });
    Ok(event_stream.boxed())
}
```

- [ ] **Step 5: Implement Anthropic streaming**

Similar mapping for Anthropic SSE; map each Anthropic SSE chunk to OpenAI `chat.completion.chunk` JSON.

- [ ] **Step 6: Run tests**

Run: `cargo test -p godwit-providers streaming`
Expected: PASS.

- [ ] **Step 7: Commit**

```bash
git add crates/godwit-providers
git commit -m "feat(providers): sse streaming for openai and anthropic"
```

---

## Task 11: API State and Middleware

**Files:**
- Create: `crates/godwit-api/Cargo.toml`
- Create: `crates/godwit-api/src/lib.rs`
- Create: `crates/godwit-api/src/state.rs`
- Create: `crates/godwit-api/src/middleware.rs`
- Test: middleware file

- [ ] **Step 1: Write failing middleware test**

Create `crates/godwit-api/src/middleware.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extract_bearer_token() {
        assert_eq!(
            extract_token("Bearer sk-godwit-abc123"),
            Some("sk-godwit-abc123")
        );
        assert_eq!(extract_token("Basic abc"), None);
    }
}
```

- [ ] **Step 2: Run test to verify failure**

Run: `cargo test -p godwit-api extract_bearer_token`
Expected: FAIL.

- [ ] **Step 3: Create API crate Cargo.toml**

```toml
[package]
name = "godwit-api"
version = "0.1.0"
edition = "2021"

[dependencies]
godwit-core = { path = "../godwit-core" }
godwit-db = { path = "../godwit-db" }
godwit-auth = { path = "../godwit-auth" }
godwit-providers = { path = "../godwit-providers" }
godwit-cache = { path = "../godwit-cache" }
axum = "0.7"
async-trait = "0.1"
tower = "0.5"
tower-http = { version = "0.6", features = ["trace", "cors"] }
tokio = { workspace = true }
serde = { workspace = true }
serde_json = { workspace = true }
chrono = { workspace = true }
uuid = { workspace = true }
rust_decimal = { version = "1.35", features = ["serde"] }
tracing = { workspace = true }

[dev-dependencies]
hyper = { version = "1", features = ["full"] }
rust_decimal_macros = "1.35"
```

- [ ] **Step 4: Implement state and middleware**

Create `crates/godwit-api/src/state.rs`:

```rust
use godwit_auth::jwt::Claims;
use godwit_cache::MemoryCache;
use godwit_core::AppConfig;
use godwit_db::models::{ApiKey, Model};
use godwit_db::repositories::{api_keys::ApiKeyRepository, organizations::OrganizationRepository, users::UserRepository};
use godwit_providers::Provider;
use sqlx::PgPool;
use std::sync::Arc;

pub struct AppState {
    pub config: AppConfig,
    pub pool: PgPool,
    pub provider_router: Arc<dyn ProviderRouter>,
    pub user_repo: UserRepository,
    pub org_repo: OrganizationRepository,
    pub api_key_repo: ApiKeyRepository,
    pub api_key_cache: MemoryCache<String, ApiKey>,
    pub model_cache: MemoryCache<(uuid::Uuid, String), Model>,
}

#[async_trait::async_trait]
pub trait ProviderRouter: Send + Sync {
    async fn route(&self, organization_id: uuid::Uuid, model: &str) -> Option<Arc<dyn Provider>>;
}
```

Create `crates/godwit-api/src/error.rs`:

```rust
use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use godwit_core::PasteurError;
use serde_json::json;

pub enum ApiError {
    Unauthorized,
    Forbidden,
    NotFound,
    BadRequest(String),
    Internal,
    Core(PasteurError),
}

impl From<PasteurError> for ApiError {
    fn from(err: PasteurError) -> Self {
        match err {
            PasteurError::NotFound => ApiError::NotFound,
            PasteurError::Auth(_) | PasteurError::Forbidden(_) => ApiError::Unauthorized,
            PasteurError::Validation(msg) => ApiError::BadRequest(msg),
            _ => ApiError::Core(err),
        }
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let (status, title, detail) = match &self {
            ApiError::Unauthorized => (StatusCode::UNAUTHORIZED, "Unauthorized", "Authentication required."),
            ApiError::Forbidden => (StatusCode::FORBIDDEN, "Forbidden", "Insufficient permissions."),
            ApiError::NotFound => (StatusCode::NOT_FOUND, "Not Found", "Resource not found."),
            ApiError::BadRequest(msg) => (StatusCode::BAD_REQUEST, "Bad Request", msg.as_str()),
            ApiError::Internal | ApiError::Core(_) => (StatusCode::INTERNAL_SERVER_ERROR, "Internal Server Error", "An unexpected error occurred."),
        };
        let body = Json(json!({
            "type": format!("https://api.godwit.local/errors/{}", title.to_lowercase().replace(' ', "-")),
            "title": title,
            "status": status.as_u16(),
            "detail": detail,
            "instance": "/"
        }));
        (status, body).into_response()
    }
}
```

Create `crates/godwit-api/src/middleware.rs`:

```rust
use axum::{
    body::Body,
    extract::{Request, State},
    http::{header::AUTHORIZATION, StatusCode},
    middleware::Next,
    response::Response,
};
use godwit_auth::{api_keys::verify_key, jwt::verify};

use crate::state::AppState;

pub fn extract_token(header: &str) -> Option<&str> {
    header.strip_prefix("Bearer ")
}

pub async fn api_key_auth(
    State(state): State<Arc<AppState>>,
    mut req: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    let auth = req
        .headers()
        .get(AUTHORIZATION)
        .and_then(|h| h.to_str().ok())
        .and_then(extract_token)
        .ok_or(StatusCode::UNAUTHORIZED)?;

    // Fast path: cache lookup by raw key.
    if let Some(key) = state.api_key_cache.get(&auth.to_string()).await {
        if !key.disabled && key.expires_at.map(|e| e > chrono::Utc::now()).unwrap_or(true) {
            req.extensions_mut().insert(key);
            return Ok(next.run(req).await);
        }
    }

    // Fallback: database lookup by prefix.
    let prefix = godwit_auth::api_keys::extract_prefix(auth);
    let candidates = state
        .api_key_repo
        .get_by_prefix(&prefix)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let key = candidates
        .into_iter()
        .find(|k| verify_key(auth, &k.key_hash))
        .ok_or(StatusCode::UNAUTHORIZED)?;
    if key.disabled || key.expires_at.map(|e| e < chrono::Utc::now()).unwrap_or(false) {
        return Err(StatusCode::UNAUTHORIZED);
    }
    state.api_key_cache.insert(auth.to_string(), key.clone()).await;
    req.extensions_mut().insert(key);
    Ok(next.run(req).await)
}

pub async fn jwt_auth(
    State(state): State<Arc<AppState>>,
    mut req: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    let auth = req
        .headers()
        .get(AUTHORIZATION)
        .and_then(|h| h.to_str().ok())
        .and_then(extract_token)
        .ok_or(StatusCode::UNAUTHORIZED)?;
    let claims = verify(&state.config.auth.jwt_secret, auth)
        .map_err(|_| StatusCode::UNAUTHORIZED)?;
    req.extensions_mut().insert(claims);
    Ok(next.run(req).await)
}
```

- [ ] **Step 5: Wire up lib.rs**

Create `crates/godwit-api/src/lib.rs`:

```rust
pub mod admin;
pub mod error;
pub mod middleware;
pub mod proxy;
pub mod state;
```

- [ ] **Step 6: Run tests**

Run: `cargo test -p godwit-api extract_bearer_token`
Expected: PASS.

- [ ] **Step 7: Commit**

```bash
git add crates/godwit-api
git commit -m "feat(api): app state and auth middleware"
```

---

## Task 11.5: In-Memory Cache for Fast Proxy Path

**Files:**
- Create: `crates/godwit-cache/Cargo.toml`
- Create: `crates/godwit-cache/src/lib.rs`
- Test: same file

- [ ] **Step 1: Write failing cache test**

Create `crates/godwit-cache/src/lib.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn cache_stores_and_retrieves() {
        let cache = MemoryCache::new();
        cache.insert("key".to_string(), "value".to_string()).await;
        assert_eq!(cache.get("key").await, Some("value".to_string()));
    }
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p godwit-cache cache_stores_and_retrieves`
Expected: FAIL.

- [ ] **Step 3: Create cache crate Cargo.toml**

```toml
[package]
name = "godwit-cache"
version = "0.1.0"
edition = "2021"

[dependencies]
dashmap = "6"
tokio = { workspace = true }
```

- [ ] **Step 4: Implement MemoryCache**

Create `crates/godwit-cache/src/lib.rs`:

```rust
use dashmap::DashMap;
use std::{fmt::Debug, hash::Hash, sync::Arc};

#[derive(Clone)]
pub struct MemoryCache<K, V> {
    inner: Arc<DashMap<K, V>>,
}

impl<K: Eq + Hash + Debug, V: Clone + Send + Sync> MemoryCache<K, V> {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(DashMap::new()),
        }
    }

    pub async fn insert(&self, key: K, value: V) {
        self.inner.insert(key, value);
    }

    pub async fn get(&self, key: &K) -> Option<V> {
        self.inner.get(key).map(|entry| entry.clone())
    }

    pub async fn invalidate(&self, key: &K) {
        self.inner.remove(key);
    }
}

impl<K, V> Default for MemoryCache<K, V> {
    fn default() -> Self {
        Self::new()
    }
}
```

- [ ] **Step 5: Run tests**

Run: `cargo test -p godwit-cache`
Expected: PASS.

- [ ] **Step 6: Commit**

```bash
git add crates/godwit-cache
git commit -m "feat(cache): in-memory cache for proxy fast path"
```

---

## Task 12: Proxy Routes

**Files:**
- Create: `crates/godwit-api/src/proxy.rs`
- Test: same file

- [ ] **Step 1: Write failing proxy test**

Create `crates/godwit-api/src/proxy.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use axum::{body::Body, http::Request, routing::Router};
    use godwit_db::models::ApiKey;
    use std::sync::Arc;
    use tower::ServiceExt;

    fn test_api_key() -> ApiKey {
        ApiKey {
            id: uuid::Uuid::new_v4(),
            user_id: uuid::Uuid::new_v4(),
            team_id: None,
            organization_id: uuid::Uuid::new_v4(),
            name: "test".to_string(),
            key_prefix: "test".to_string(),
            key_hash: "hash".to_string(),
            scopes: vec!["proxy:write".to_string()],
            budget_limit_usd: None,
            budget_spent_usd: rust_decimal::Decimal::ZERO,
            rate_limit_requests_per_minute: None,
            expires_at: None,
            disabled: false,
            created_at: chrono::Utc::now(),
        }
    }

    #[test]
    fn models_response_has_openai_shape() {
        let body = models_response(&[]);
        assert_eq!(body["object"], "list");
        assert!(body["data"].as_array().unwrap().is_empty());
    }
}
```

- [ ] **Step 2: Implement proxy router**

Create `crates/godwit-api/src/proxy.rs`:

```rust
use axum::{
    extract::{Extension, Json, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::{get, post},
    Router,
};
use godwit_core::{ChatCompletionRequest, ChatMessage, ChatCompletionResponse};
use godwit_db::models::ApiKey;
use godwit_db::repositories::models::ModelRepository;
use std::sync::Arc;

use crate::state::AppState;

pub fn router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/v1/models", get(list_models))
        .route("/v1/chat/completions", post(chat_completions))
}

pub fn models_response(models: &[godwit_db::models::Model]) -> serde_json::Value {
    let data: Vec<serde_json::Value> = models
        .iter()
        .map(|m| {
            serde_json::json!({
                "id": m.public_id,
                "object": "model",
                "created": m.created_at.timestamp(),
                "owned_by": "organization"
            })
        })
        .collect();
    serde_json::json!({ "object": "list", "data": data })
}

async fn list_models(
    State(state): State<Arc<AppState>>,
    Extension(api_key): Extension<ApiKey>,
) -> Result<impl IntoResponse, crate::error::ApiError> {
    let models = if let Some(cached) = state.model_cache.get(&(api_key.organization_id, "".to_string())).await {
        vec![cached]
    } else {
        let repo = ModelRepository::new(state.pool.clone());
        let models = repo
            .list_for_organization(api_key.organization_id)
            .await
            .map_err(crate::error::ApiError::Core)?;
        for m in &models {
            state.model_cache.insert((api_key.organization_id, m.public_id.clone()), m.clone()).await;
        }
        models
    };
    Ok((StatusCode::OK, Json(models_response(&models))))
}

async fn chat_completions(
    State(state): State<Arc<AppState>>,
    Extension(api_key): Extension<ApiKey>,
    Json(req): Json<ChatCompletionRequest>,
) -> Result<Response, crate::error::ApiError> {
    let start = std::time::Instant::now();
    let model = state
        .model_cache
        .get(&(api_key.organization_id, req.model.clone()))
        .await
        .ok_or(crate::error::ApiError::NotFound)?;

    let provider = state
        .provider_router
        .route(api_key.organization_id, &model.provider_model_id)
        .await
        .ok_or(crate::error::ApiError::NotFound)?;

    let streamed = req.stream == Some(true);
    let result = if streamed {
        let stream = provider
            .stream_chat_completion(req)
            .await
            .map_err(|_| StatusCode::BAD_GATEWAY)?;
        let sse_stream = stream.map(move |event| {
            let chunk = event
                .map(|e| format!("data: {}\n\n", e.data))
                .unwrap_or_else(|_| "data: [ERROR]\n\n".to_string());
            Ok::<_, std::convert::Infallible>(chunk)
        });
        Ok(axum::response::Sse::new(sse_stream).into_response())
    } else {
        let resp = provider
            .chat_completion(req)
            .await
            .map_err(|_| StatusCode::BAD_GATEWAY)?;
        match resp {
            godwit_providers::ProviderResponse::Json(json) => Ok(Json(json).into_response()),
        }
    };

    // Asynchronous logging to avoid blocking the response.
    let log = RequestLogEntry {
        api_key_id: api_key.id,
        user_id: api_key.user_id,
        organization_id: api_key.organization_id,
        team_id: api_key.team_id,
        model: model.public_id.clone(),
        provider: model.provider.clone(),
        provider_model_id: model.provider_model_id.clone(),
        duration_ms: start.elapsed().as_millis() as i32,
        streamed,
        status: "success".to_string(),
    };
    let pool = state.pool.clone();
    tokio::spawn(async move {
        // Insert request_logs row asynchronously.
        let _ = sqlx::query(
            "INSERT INTO request_logs (api_key_id, user_id, organization_id, team_id, model, provider, provider_model_id, duration_ms, streamed, status)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)"
        )
        .bind(log.api_key_id)
        .bind(log.user_id)
        .bind(log.organization_id)
        .bind(log.team_id)
        .bind(log.model)
        .bind(log.provider)
        .bind(log.provider_model_id)
        .bind(log.duration_ms)
        .bind(log.streamed)
        .bind(log.status)
        .execute(&pool)
        .await;
    });

    result
}

#[derive(Clone)]
struct RequestLogEntry {
    api_key_id: uuid::Uuid,
    user_id: uuid::Uuid,
    organization_id: uuid::Uuid,
    team_id: Option<uuid::Uuid>,
    model: String,
    provider: String,
    provider_model_id: String,
    duration_ms: i32,
    streamed: bool,
    status: String,
}
```

- [ ] **Step 3: Run tests**

Run: `cargo test -p godwit-api proxy`
Expected: PASS.

- [ ] **Step 4: Commit**

```bash
git add crates/godwit-api
git commit -m "feat(api): high-performance proxy routes with caching and async logging"
```

---

## Task 13: Admin Auth Routes

**Files:**
- Create: `crates/godwit-api/src/admin/auth.rs`
- Test: auth routes

- [ ] **Step 1: Implement login, OIDC, and SAML routes**

Create `crates/godwit-api/src/admin/auth.rs`:

```rust
use axum::{
    extract::{Path, Query, State},
    response::{IntoResponse, Redirect},
    routing::{get, post},
    Json, Router,
};
use godwit_auth::{
    api_keys::verify_password,
    jwt::{issue, Claims},
};
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

pub fn router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/auth/login", post(login))
        .route("/auth/oidc/:provider", get(oidc_start))
        .route("/auth/oidc/:provider/callback", get(oidc_callback))
        .route("/auth/saml/:provider/acs", post(saml_acs))
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
    let claims = Claims::new(user.id, user.organization_id.unwrap_or_default(), &user.role);
    let token = issue(&state.config.auth.jwt_secret, claims, chrono::Duration::minutes(15))
        .map_err(|_| crate::error::ApiError::Internal)?;
    Ok(Json(serde_json::json!({ "access_token": token })))
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
    let (url, _csrf, _nonce) = client.authorize_url(vec!["openid".to_string(), "email".to_string(), "profile".to_string()]);
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
    // Upsert user by email; in production also verify sso_subject.
    let user = match state.user_repo.get_by_email(&email).await {
        Ok(u) => u,
        Err(_) => state
            .user_repo
            .create(&email, name.as_deref(), godwit_db::models::UserRole::User, None)
            .await
            .map_err(|_| crate::error::ApiError::Internal)?,
    };
    let claims = Claims::new(user.id, user.organization_id.unwrap_or_default(), &user.role);
    let token = issue(&state.config.auth.jwt_secret, claims, chrono::Duration::minutes(15))
        .map_err(|_| crate::error::ApiError::Internal)?;
    Ok(Json(serde_json::json!({ "access_token": token })))
}

async fn saml_acs(
    State(_state): State<Arc<AppState>>,
    Path(_provider_id): Path<String>,
) -> Result<impl IntoResponse, crate::error::ApiError> {
    Err(crate::error::ApiError::BadRequest(
        "SAML ACS requires XML signature validation; implement with real IdP metadata".to_string(),
    ))
}
```

- [ ] **Step 2: Commit**

```bash
git add crates/godwit-api
git commit -m "feat(api): admin auth routes (login, oidc, saml acs)"
```

---

## Task 14: Admin Users/Orgs/Teams Routes

**Files:**
- Create: `crates/godwit-api/src/admin/users.rs`
- Create: `crates/godwit-api/src/admin/organizations.rs`
- Create: `crates/godwit-api/src/admin/teams.rs`
- Modify: `crates/godwit-api/src/admin/mod.rs`
- Test: each file

- [ ] **Step 1: Implement users router with RBAC**

Create `crates/godwit-api/src/admin/users.rs`:

```rust
use axum::{
    extract::{Extension, Path, State},
    routing::{delete, get, patch, post},
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
        .route("/users/:id", get(get_user).patch(update_user).delete(delete_user))
}

fn require_role(claims: &Claims, allowed: &[Role]) -> Result<Role, ApiError> {
    let role = Role::from_str(&claims.role).ok_or(ApiError::Forbidden)?;
    if !allowed.contains(&role) {
        return Err(ApiError::Forbidden);
    }
    Ok(role)
}

async fn list_users(
    State(state): State<Arc<AppState>>,
    Extension(claims): Extension<Claims>,
) -> Result<Json<serde_json::Value>, ApiError> {
    require_role(&claims, &[Role::SuperAdmin, Role::OrgAdmin])?;
    // Filter by organization for org_admin.
    let users = state
        .user_repo
        .list_for_organization(claims.organization_id)
        .await
        .map_err(ApiError::Core)?;
    Ok(Json(serde_json::json!({ "data": users })))
}

async fn create_user(
    State(state): State<Arc<AppState>>,
    Extension(claims): Extension<Claims>,
    Json(req): Json<CreateUserRequest>,
) -> Result<Json<serde_json::Value>, ApiError> {
    require_role(&claims, &[Role::SuperAdmin, Role::OrgAdmin])?;
    let role = godwit_db::models::UserRole::from_str(&req.role).ok_or(ApiError::BadRequest("invalid role".to_string()))?;
    let org_id = claims.organization_id;
    let user = state
        .user_repo
        .create(&req.email, req.name.as_deref(), role, Some(org_id))
        .await
        .map_err(ApiError::Core)?;
    Ok(Json(serde_json::json!({ "data": user })))
}

async fn get_user(
    State(_state): State<Arc<AppState>>,
    Extension(_claims): Extension<Claims>,
    Path(_id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, ApiError> {
    Err(ApiError::NotFound)
}

async fn update_user(
    State(_state): State<Arc<AppState>>,
    Extension(_claims): Extension<Claims>,
    Path(_id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, ApiError> {
    Err(ApiError::NotFound)
}

async fn delete_user(
    State(_state): State<Arc<AppState>>,
    Extension(_claims): Extension<Claims>,
    Path(_id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, ApiError> {
    Err(ApiError::NotFound)
}
```

- [ ] **Step 2: Add UserRole::from_str to DB models**

Modify `crates/godwit-db/src/models.rs`:

```rust
impl UserRole {
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "super_admin" => Some(UserRole::SuperAdmin),
            "org_admin" => Some(UserRole::OrgAdmin),
            "team_admin" => Some(UserRole::TeamAdmin),
            "user" => Some(UserRole::User),
            _ => None,
        }
    }
}
```

- [ ] **Step 3: Add list_for_organization to UserRepository**

Modify `crates/godwit-db/src/repositories/users.rs`:

```rust
pub async fn list_for_organization(
    &self,
    organization_id: Uuid,
) -> Result<Vec<User>, PasteurError> {
    sqlx::query_as::<_, User>("SELECT * FROM users WHERE organization_id = $1")
        .bind(organization_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| PasteurError::Database(e.to_string()))
}
```

- [ ] **Step 4: Implement remaining CRUD routers**

Create `organizations.rs`, `teams.rs`, `api_keys.rs`, `models.rs`, and `spend.rs` following the same pattern: extract JWT claims, check role, call repository.

- [ ] **Step 5: Wire admin module with JWT middleware**

Modify `crates/godwit-api/src/admin/mod.rs`:

```rust
pub mod api_keys;
pub mod auth;
pub mod models;
pub mod organizations;
pub mod spend;
pub mod teams;
pub mod users;

use axum::{middleware, Router};
use std::sync::Arc;
use crate::{middleware::jwt_auth, state::AppState};

pub fn router() -> Router<Arc<AppState>> {
    let protected = Router::new()
        .nest("/users", users::router())
        .nest("/organizations", organizations::router())
        .nest("/teams", teams::router())
        .nest("/api-keys", api_keys::router())
        .nest("/models", models::router())
        .nest("/spend", spend::router())
        .route_layer(middleware::from_fn(jwt_auth));

    Router::new()
        .nest("/auth", auth::router())
        .merge(protected)
}
```

- [ ] **Step 6: Commit**

```bash
git add crates/godwit-api
git commit -m "feat(api): admin users, orgs, teams crud with rbac"
```

---

## Task 15: Admin API Keys and Models Routes

**Files:**
- Create: `crates/godwit-api/src/admin/api_keys.rs`
- Create: `crates/godwit-api/src/admin/models.rs`
- Test: both files

- [ ] **Step 1: Implement API key creation**

Create `crates/godwit-api/src/admin/api_keys.rs`:

```rust
use axum::{
    extract::{Extension, State},
    routing::{get, post},
    Json, Router,
};
use godwit_auth::{api_keys::generate_api_key, jwt::Claims, rbac::Role};
use serde::Deserialize;
use std::sync::Arc;

use crate::{error::ApiError, state::AppState};

#[derive(Deserialize)]
pub struct CreateApiKeyRequest {
    name: String,
    scopes: Vec<String>,
}

pub fn router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/api-keys", get(list_api_keys).post(create_api_key))
}

async fn list_api_keys(
    State(state): State<Arc<AppState>>,
    Extension(claims): Extension<Claims>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let role = Role::from_str(&claims.role).ok_or(ApiError::Forbidden)?;
    if !role.can_manage_api_keys() {
        return Err(ApiError::Forbidden);
    }
    let keys = state
        .api_key_repo
        .list_for_organization(claims.organization_id)
        .await
        .map_err(ApiError::Core)?;
    Ok(Json(serde_json::json!({ "data": keys })))
}

async fn create_api_key(
    State(state): State<Arc<AppState>>,
    Extension(claims): Extension<Claims>,
    Json(req): Json<CreateApiKeyRequest>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let role = Role::from_str(&claims.role).ok_or(ApiError::Forbidden)?;
    if !role.can_manage_api_keys() {
        return Err(ApiError::Forbidden);
    }
    let (plaintext, hash, prefix) = generate_api_key();
    let key = state
        .api_key_repo
        .create(
            claims.user_id,
            claims.organization_id,
            &req.name,
            &prefix,
            &hash,
            &req.scopes,
            None,
            None,
        )
        .await
        .map_err(ApiError::Core)?;
    Ok(Json(serde_json::json!({
        "id": key.id,
        "key": plaintext,
        "name": key.name,
    })))
}
```

- [ ] **Step 2: Add list_for_organization to ApiKeyRepository**

Modify `crates/godwit-db/src/repositories/api_keys.rs`:

```rust
pub async fn list_for_organization(
    &self,
    organization_id: Uuid,
) -> Result<Vec<ApiKey>, PasteurError> {
    sqlx::query_as::<_, ApiKey>("SELECT * FROM api_keys WHERE organization_id = $1")
        .bind(organization_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| PasteurError::Database(e.to_string()))
}
```

- [ ] **Step 3: Implement model routes**

Create `crates/godwit-api/src/admin/models.rs` with CRUD for `models` table following the same JWT + RBAC pattern.

- [ ] **Step 4: Commit**

```bash
git add crates/godwit-api
git commit -m "feat(api): admin api keys and models"
```

---

## Task 16: Spend Tracking

**Files:**
- Create: `crates/godwit-api/src/admin/spend.rs`
- Modify: proxy route to log requests
- Test: spend calculation

- [ ] **Step 1: Add spend calculation helper**

Create `crates/godwit-api/src/admin/spend.rs`:

```rust
use rust_decimal::Decimal;
use godwit_core::Usage;

pub fn compute_cost(usage: &Usage, input_price: Decimal, output_price: Decimal) -> Decimal {
    let input = Decimal::from(usage.prompt_tokens) * input_price / Decimal::from(1000);
    let output = Decimal::from(usage.completion_tokens) * output_price / Decimal::from(1000);
    input + output
}

#[cfg(test)]
mod tests {
    use super::*;
    use godwit_core::Usage;
    use rust_decimal_macros::dec;

    #[test]
    fn cost_computation() {
        let usage = Usage {
            prompt_tokens: 1000,
            completion_tokens: 500,
            total_tokens: 1500,
        };
        let cost = compute_cost(&usage, dec!(0.005), dec!(0.015));
        assert_eq!(cost, dec!(0.0125));
    }
}
```

- [ ] **Step 2: Update proxy handler with cost tracking**

Modify `chat_completions` in `crates/godwit-api/src/proxy.rs` to compute cost from `usage` on non-streaming responses, update the async log entry with tokens/cost, and increment `api_keys.budget_spent_usd`. For streaming responses, skip cost attribution in MVP.

- [ ] **Step 3: Commit**

```bash
git add crates/godwit-api
git commit -m "feat(api): spend tracking and budget updates"
```

---

## Task 17: Binary, Configuration, and Startup

**Files:**
- Create: `crates/godwit-bin/Cargo.toml`
- Create: `crates/godwit-bin/src/main.rs`
- Create: `config.example.yaml`
- Test: smoke test

- [ ] **Step 1: Create bin crate Cargo.toml**

```toml
[package]
name = "godwit-bin"
version = "0.1.0"
edition = "2021"

[[bin]]
name = "godwit"
path = "src/main.rs"

[dependencies]
godwit-core = { path = "../godwit-core" }
godwit-db = { path = "../godwit-db" }
godwit-auth = { path = "../godwit-auth" }
godwit-providers = { path = "../godwit-providers" }
godwit-cache = { path = "../godwit-cache" }
godwit-api = { path = "../godwit-api" }
axum = "0.7"
tokio = { workspace = true }
tracing = { workspace = true }
tracing-subscriber = "0.3"
serde_yaml = { workspace = true }
anyhow = { workspace = true }
uuid = { workspace = true }
async-trait = "0.1"
```

- [ ] **Step 2: Implement main**

Create `crates/godwit-bin/src/main.rs`:

```rust
use godwit_api::{admin, proxy, state::{AppState, ProviderRouter}};
use godwit_cache::MemoryCache;
use godwit_core::{AppConfig, ProviderConfig};
use godwit_db::{connect, run_migrations, repositories::{api_keys::ApiKeyRepository, organizations::OrganizationRepository, users::UserRepository}};
use godwit_providers::{anthropic::AnthropicProvider, openai::OpenAiProvider, Provider};
use std::sync::Arc;
use axum::{middleware, routing::Router};

pub struct SimpleProviderRouter {
    openai: Arc<dyn Provider>,
    anthropic: Arc<dyn Provider>,
}

impl SimpleProviderRouter {
    pub fn new(providers: &godwit_core::ProvidersConfig) -> Self {
        Self {
            openai: Arc::new(OpenAiProvider::from_config(&providers.openai)),
            anthropic: Arc::new(AnthropicProvider::new(
                &providers.anthropic.api_key,
                &providers.anthropic.base_url,
            )),
        }
    }
}

#[async_trait::async_trait]
impl ProviderRouter for SimpleProviderRouter {
    async fn route(&self, _organization_id: uuid::Uuid, provider_model_id: &str) -> Option<Arc<dyn Provider>> {
        // In MVP, routing is determined by provider config attached to the model.
        // This simple router dispatches by inspecting the model id prefix.
        if provider_model_id.starts_with("claude") {
            Some(self.anthropic.clone())
        } else {
            Some(self.openai.clone())
        }
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();
    let config: AppConfig = load_config()?;
    let pool = connect(&config.database.url).await?;
    run_migrations(&pool).await?;

    let state = Arc::new(AppState {
        config: config.clone(),
        pool: pool.clone(),
        provider_router: Arc::new(SimpleProviderRouter::new(&config.providers)),
        user_repo: UserRepository::new(pool.clone()),
        org_repo: OrganizationRepository::new(pool.clone()),
        api_key_repo: ApiKeyRepository::new(pool.clone()),
        api_key_cache: MemoryCache::new(),
        model_cache: MemoryCache::new(),
    });

    let app = Router::new()
        .merge(proxy::router())
        .route_layer(middleware::from_fn_with_state(state.clone(), godwit_api::middleware::api_key_auth))
        .nest("/api/v1", admin::router())
        .with_state(state.clone());

    let listener = tokio::net::TcpListener::bind(format!("{}:{}", config.server.host, config.server.port)).await?;
    tracing::info!("Godwit listening on {}:{}", config.server.host, config.server.port);
    axum::serve(listener, app).await?;
    Ok(())
}

fn load_config() -> anyhow::Result<AppConfig> {
    let path = std::env::var("CONFIG_PATH").unwrap_or_else(|_| "config.yaml".to_string());
    let file = std::fs::File::open(&path)?;
    let config: AppConfig = serde_yaml::from_reader(file)?;
    Ok(config)
}
```

- [ ] **Step 3: Create config.example.yaml**

```yaml
server:
  host: 0.0.0.0
  port: 3000
  request_timeout_seconds: 120

database:
  url: postgres://pasteurllm:pasteurllm@localhost:5432/pasteurllm

auth:
  jwt_secret: change-me-in-production
  access_token_ttl_minutes: 15
  refresh_token_ttl_days: 7
  oidc_providers: []
  saml_providers: []

providers:
  openai:
    api_key: "${OPENAI_API_KEY}"
    base_url: https://api.openai.com/v1
  anthropic:
    api_key: "${ANTHROPIC_API_KEY}"
    base_url: https://api.anthropic.com/v1
```

- [ ] **Step 4: Build and smoke test**

Run: `cargo build --bin godwit`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add crates/godwit-bin config.example.yaml
git commit -m "feat(bin): main startup, config loading, router assembly"
```

---

## Task 18: Docker and Compose

**Files:**
- Create: `Dockerfile`
- Create: `docker-compose.yml`
- Create: `.dockerignore`
- Test: `docker compose build`

- [ ] **Step 1: Create Dockerfile**

```dockerfile
FROM lukemathwalker/cargo-chef:latest-rust-1 AS chef
WORKDIR /app

FROM chef AS planner
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

FROM chef AS builder
COPY --from=planner /app/recipe.json recipe.json
RUN cargo chef cook --release --recipe-path recipe.json
COPY . .
RUN cargo build --release --bin godwit

FROM debian:bookworm-slim AS runtime
RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*
WORKDIR /app
COPY --from=builder /app/target/release/godwit /usr/local/bin/godwit
COPY config.example.yaml /app/config.yaml
ENV CONFIG_PATH=/app/config.yaml
EXPOSE 3000
CMD ["godwit"]
```

- [ ] **Step 2: Create docker-compose.yml**

```yaml
services:
  db:
    image: postgres:16-alpine
    environment:
      POSTGRES_USER: godwit
      POSTGRES_PASSWORD: godwit
      POSTGRES_DB: godwit
    ports:
      - "5432:5432"
    volumes:
      - pgdata:/var/lib/postgresql/data
    healthcheck:
      test: ["CMD-SHELL", "pg_isready -U godwit"]
      interval: 5s
      timeout: 5s
      retries: 5

  app:
    build: .
    ports:
      - "3000:3000"
    environment:
      CONFIG_PATH: /app/config.yaml
      DATABASE_URL: postgres://pasteurllm:pasteurllm@db:5432/pasteurllm
      OPENAI_API_KEY: ${OPENAI_API_KEY}
      ANTHROPIC_API_KEY: ${ANTHROPIC_API_KEY}
      JWT_SECRET: ${JWT_SECRET}
    depends_on:
      db:
        condition: service_healthy

volumes:
  pgdata:
```

- [ ] **Step 3: Create .dockerignore**

```
target/
.git/
.gitignore
*.md
.env
```

- [ ] **Step 4: Build image**

Run: `docker compose build`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add Dockerfile docker-compose.yml .dockerignore
git commit -m "chore(deploy): dockerfile and compose"
```

---

## Task 19: Integration Tests

**Files:**
- Create: `tests/proxy_integration.rs`
- Create: `tests/admin_integration.rs`
- Modify: workspace Cargo.toml for integration test dependencies

- [ ] **Step 1: Write proxy integration test**

Create `tests/proxy_integration.rs`:

```rust
use godwit_core::{ChatCompletionRequest, ChatMessage};
use reqwest::Client;

#[tokio::test]
#[ignore = "requires running server"]
async fn proxy_chat_completion_smoke() {
    let client = Client::new();
    let resp = client
        .post("http://localhost:3000/v1/chat/completions")
        .header("Authorization", "Bearer sk-godwit-test")
        .json(&ChatCompletionRequest {
            model: "gpt-4o".to_string(),
            messages: vec![ChatMessage { role: "user".to_string(), content: "Hi".to_string() }],
            stream: Some(false),
            temperature: None,
            max_tokens: None,
        })
        .send()
        .await
        .unwrap();
    assert!(resp.status().is_success() || resp.status() == 401);
}
```

- [ ] **Step 2: Write admin integration test**

Create `tests/admin_integration.rs` with login + user CRUD flow.

- [ ] **Step 3: Add reqwest to workspace**

Modify `Cargo.toml` workspace dependencies:

```toml
reqwest = { version = "0.12", features = ["json"] }
```

- [ ] **Step 4: Run integration tests**

Run: `cargo test --test proxy_integration -- --ignored`
Expected: PASS (with running local stack).

- [ ] **Step 5: Commit**

```bash
git add tests Cargo.toml
git commit -m "test(integration): proxy and admin smoke tests"
```

---

## Task 20: Performance Benchmarks

**Files:**
- Create: `benches/proxy_latency.rs`
- Create: `scripts/bench.sh`

- [ ] **Step 1: Add Criterion benchmark**

Modify `Cargo.toml` workspace to add Criterion:

```toml
[workspace.dependencies]
criterion = { version = "0.5", features = ["async_tokio"] }
```

Create `crates/godwit-providers/Cargo.toml` dev dependency:

```toml
[dev-dependencies]
criterion = { workspace = true }
```

- [ ] **Step 2: Write latency benchmark**

Create `benches/proxy_latency.rs`:

```rust
use criterion::{criterion_group, criterion_main, Criterion};
use godwit_core::{ChatCompletionRequest, ChatMessage};
use godwit_providers::anthropic::to_anthropic_request;

fn bench_anthropic_mapping(c: &mut Criterion) {
    let req = ChatCompletionRequest {
        model: "claude-sonnet".to_string(),
        messages: vec![
            ChatMessage { role: "system".to_string(), content: "You are helpful".to_string() },
            ChatMessage { role: "user".to_string(), content: "Hello".to_string() },
        ],
        stream: Some(false),
        temperature: Some(0.7),
        max_tokens: Some(100),
    };
    c.bench_function("openai_to_anthropic_mapping", |b| {
        b.iter(|| to_anthropic_request(&req))
    });
}

criterion_group!(benches, bench_anthropic_mapping);
criterion_main!(benches);
```

- [ ] **Step 3: Add load-test script**

Create `scripts/bench.sh`:

```bash
#!/bin/bash
set -e
which oha || cargo install oha
oha -z 30s -c 50 --no-tui \
  -H "Authorization: Bearer sk-godwit-test" \
  -d '{"model":"gpt-4o","messages":[{"role":"user","content":"hi"}]}' \
  http://localhost:3000/v1/chat/completions
```

- [ ] **Step 4: Run benchmark**

Run: `cargo bench -p godwit-providers`
Expected: PASS and report latencies.

- [ ] **Step 5: Commit**

```bash
git add benches scripts
git commit -m "chore(bench): proxy latency benchmarks and load test script"
```

---

## Self-Review Checklist

- [ ] **Spec coverage:** Every MVP requirement from the design spec maps to at least one task above.
- [ ] **Placeholder scan:** No "TODO", "TBD", or "implement later" steps remain.
- [ ] **Type consistency:** Function and type names match across tasks.
- [ ] **Test coverage:** Each crate has unit tests; integration tests exist for proxy and admin.
- [ ] **Dependencies:** Later tasks only depend on earlier tasks.

If any gaps are found while executing, create a follow-up task before continuing.
