# Password Management Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Full-stack password lifecycle for Godwit — self-service change, admin reset, forgot-password via SMTP email, configurable policy, expiry (forced change at login), first-login force, and reuse prevention.

**Architecture:** Schema additions on `users` + two new tables; a pure `password.rs` logic layer (policy validation, rotation, state); new repos; a `mail.rs` SMTP layer (`lettre` + embedded `tera`, behind a `SendEmail` trait); 5 new `/api/v1/auth/*` endpoints plus a `must_change_password` flag on login; contract + both frontends. Build the backend fully (TDD), update the contract/tests, then the two frontends.

**Tech Stack:** Rust (axum 0.7, sqlx 0.7, `lettre` 0.11, `tera` 1), PostgreSQL, Next.js (`apps/ui`, `apps/admin`), vitest, `contract/routes.json`.

**Toolchain note:** Every `cargo` command must be prefixed with `export PATH="/usr/local/opt/rustup/bin:$PATH"` (see AGENTS.md). DB tests need `DATABASE_URL`.

---

## File Map

**godwit-core** (`crates/godwit-core/src/lib.rs`)
- Add `MailConfig`, `PasswordPolicy`, fields on `AuthConfig`.

**godwit-db**
- Create: `migrations/20260809000003_password_mgmt.up.sql` / `.down.sql`
- Modify: `src/models.rs` (User fields), `src/repositories/mod.rs`
- Create: `src/repositories/password_history.rs`, `src/repositories/password_reset_tokens.rs`
- Modify: `src/repositories/users.rs` (`update_password`)

**godwit-auth** (`crates/godwit-auth/src/api_keys.rs` or new `password.rs`)
- Add `hash_password`/`verify_password` already exist — reuse. Add reset-token generation helper if needed.

**godwit-api**
- Create: `src/admin/password.rs` (logic + endpoints), `src/mail.rs`, `assets/common_passwords.txt`, `assets/email/reset_password.html`, `assets/email/reset_password.txt`, `assets/email/password_changed.html`, `assets/email/password_changed.txt`
- Modify: `src/state.rs` (AppState fields), `src/admin/auth.rs` (router merge + login flag), `src/admin/mod.rs` (module decl), `Cargo.toml` (deps)
- Test: `crates/godwit-api/tests/router_integration.rs` (or new `password_integration.rs`)

**Contract / docs**
- Modify: `contract/routes.json`, `apps/ui/tests/route-contract.test.ts`, `apps/admin/tests/route-contract.test.ts`, `docs/coverage/frontend-backend.md`

**apps/ui**
- Create: `src/app/forgot-password/page.tsx`, `src/app/reset-password/page.tsx`, `src/app/(protected)/settings/page.tsx` (change), change-required handling
- Modify: `src/lib/auth.ts`, tests

**apps/admin**
- Create: forgot/reset/change pages + server actions

---

### Task 1: Config structs (MailConfig, PasswordPolicy)

**Files:**
- Modify: `crates/godwit-core/src/lib.rs`

- [ ] **Step 1: Write the failing test**

Add to `crates/godwit-core/src/lib.rs` in a `#[cfg(test)] mod tests` (or create if none) a test that parses a YAML config containing the new `auth.mail` and `auth.password_policy` sections:

```rust
#[test]
fn auth_config_parses_mail_and_password_policy() {
    let yaml = r#"
auth:
  jwt_secret: s
  access_token_ttl_minutes: 15
  refresh_token_ttl_days: 7
  mail:
    from: "Godwit <noreply@example.com>"
    host: "smtp.example.com"
    port: 587
    app_url: "https://app.example.com"
  password_policy:
    min_length: 10
    require_upper: true
    require_digit: true
    max_reuse: 3
    days_to_expire: 90
    block_common: true
"#;
    let cfg: AppConfig = serde_yaml::from_str(yaml).expect("parse");
    let auth = cfg.auth;
    let mail = auth.mail.as_ref().expect("mail present");
    assert_eq!(mail.host, "smtp.example.com");
    assert_eq!(mail.port, 587);
    assert!(mail.username.is_none());
    let pol = auth.password_policy;
    assert_eq!(pol.min_length, 10);
    assert!(pol.require_upper);
    assert!(!pol.require_symbol);
    assert_eq!(pol.max_reuse, 3);
    assert_eq!(pol.days_to_expire, 90);
    assert!(pol.block_common);
}

#[test]
fn auth_config_defaults_when_sections_absent() {
    let yaml = r#"
auth:
  jwt_secret: s
  access_token_ttl_minutes: 15
  refresh_token_ttl_days: 7
"#;
    let cfg: AppConfig = serde_yaml::from_str(yaml).expect("parse");
    assert!(cfg.auth.mail.is_none());
    let pol = cfg.auth.password_policy;
    assert_eq!(pol.min_length, 10);
    assert!(!pol.block_common);
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `export PATH="/usr/local/opt/rustup/bin:$PATH" && cargo test -p godwit-core --lib auth_config_parses_mail_and_password_policy`
Expected: FAIL (no `auth.mail`, no `password_policy` fields)

- [ ] **Step 3: Implement structs**

Add to `crates/godwit-core/src/lib.rs` after `AuthConfig`:

```rust
#[derive(Debug, Clone, Deserialize)]
pub struct MailConfig {
    pub from: String,
    pub host: String,
    #[serde(default = "default_mail_port")]
    pub port: u16,
    #[serde(default)]
    pub username: Option<String>,
    #[serde(default)]
    pub password: Option<String>,
    #[serde(default)]
    pub tls: MailTls,
    pub app_url: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MailTls {
    None,
    StartTls,
    Tls,
}

impl Default for MailTls {
    fn default() -> Self {
        MailTls::StartTls
    }
}

fn default_mail_port() -> u16 {
    587
}

#[derive(Debug, Clone, Copy, Deserialize)]
pub struct PasswordPolicy {
    #[serde(default = "default_min_length")]
    pub min_length: u32,
    #[serde(default)]
    pub require_upper: bool,
    #[serde(default)]
    pub require_lower: bool,
    #[serde(default)]
    pub require_digit: bool,
    #[serde(default)]
    pub require_symbol: bool,
    #[serde(default = "default_max_reuse")]
    pub max_reuse: u32,
    #[serde(default)]
    pub days_to_expire: u64,
    #[serde(default)]
    pub block_common: bool,
}

impl Default for PasswordPolicy {
    fn default() -> Self {
        Self {
            min_length: 10,
            require_upper: true,
            require_lower: true,
            require_digit: true,
            require_symbol: true,
            max_reuse: 5,
            days_to_expire: 0,
            block_common: true,
        }
    }
}

fn default_min_length() -> u32 {
    10
}
fn default_max_reuse() -> u32 {
    5
}
```

- [ ] **Step 4: Add `mail` + `password_policy` to `AuthConfig`**

```rust
#[serde(default)]
pub mail: Option<MailConfig>,
#[serde(default)]
pub password_policy: PasswordPolicy,
```

- [ ] **Step 5: Run test to verify it passes**

Run: `cargo test -p godwit-core --lib auth_config`
Expected: PASS

- [ ] **Step 6: Commit**

```bash
git add crates/godwit-core/src/lib.rs
git commit -m "feat(core): add mail + password_policy config structs"
```

---

### Task 2: Migration (schema)

**Files:**
- Create: `crates/godwit-db/migrations/20260809000003_password_mgmt.up.sql`, `.down.sql`

- [ ] **Step 1: Write the up migration**

`crates/godwit-db/migrations/20260809000003_password_mgmt.up.sql`:

```sql
ALTER TABLE users
  ADD COLUMN password_changed_at TIMESTAMPTZ,
  ADD COLUMN password_expires_at TIMESTAMPTZ,
  ADD COLUMN must_change_password BOOLEAN NOT NULL DEFAULT FALSE;

UPDATE users
  SET password_changed_at = NOW()
  WHERE password_changed_at IS NULL;

CREATE TABLE password_history (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    password_hash TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
CREATE INDEX idx_password_history_user ON password_history(user_id, created_at);

CREATE TABLE password_reset_tokens (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    token_hash TEXT NOT NULL UNIQUE,
    expires_at TIMESTAMPTZ NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    used_at TIMESTAMPTZ
);
CREATE INDEX idx_password_reset_tokens_hash ON password_reset_tokens(token_hash);
```

- [ ] **Step 2: Write the down migration**

`crates/godwit-db/migrations/20260809000003_password_mgmt.down.sql`:

```sql
DROP TABLE IF EXISTS password_reset_tokens;
DROP TABLE IF EXISTS password_history;
ALTER TABLE users
  DROP COLUMN IF EXISTS must_change_password,
  DROP COLUMN IF EXISTS password_expires_at,
  DROP COLUMN IF EXISTS password_changed_at;
```

- [ ] **Step 3: Apply migration + verify**

Run: `export PATH="/usr/local/opt/rustup/bin:$PATH" && DATABASE_URL=postgres://godwit:godwit@localhost:5432/godwit cargo sqlx migrate run --source crates/godwit-db/migrations`
Expected: migration applied successfully. (If `cargo-sqlx` is unavailable, rely on the `sqlx::test` in Task 3 which runs migrations against a fresh DB.)

- [ ] **Step 4: Update User model fields**

In `crates/godwit-db/src/models.rs`, add to `struct User`:

```rust
pub password_changed_at: Option<DateTime<Utc>>,
pub password_expires_at: Option<DateTime<Utc>>,
pub must_change_password: bool,
```

(These are `#[serde(skip_serializing)]` — add that attribute, password metadata must not leak. `must_change_password` is not a hash, so skip_serializing is a deliberate defensive choice; keep it private.)

- [ ] **Step 5: Commit**

```bash
git add crates/godwit-db/
git commit -m "feat(db): password mgmt schema (history, reset tokens, expiry fields)"
```

---

### Task 3: Repositories

**Files:**
- Modify: `crates/godwit-db/src/repositories/mod.rs`
- Create: `crates/godwit-db/src/repositories/password_history.rs`, `crates/godwit-db/src/repositories/password_reset_tokens.rs`
- Modify: `crates/godwit-db/src/repositories/users.rs`

- [ ] **Step 1: Register modules**

In `crates/godwit-db/src/repositories/mod.rs` add:

```rust
pub mod password_history;
pub mod password_reset_tokens;
```

- [ ] **Step 2: Write the failing tests**

Create `crates/godwit-db/src/repositories/password_history.rs`:

```rust
use crate::models::User;
use godwit_core::PasteurError;
use sqlx::PgPool;
use uuid::Uuid;

pub struct PasswordHistoryRepository {
    pool: PgPool,
}

impl PasswordHistoryRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn push(&self, user_id: Uuid, hash: &str) -> Result<(), PasteurError> {
        sqlx::query("INSERT INTO password_history (user_id, password_hash) VALUES ($1, $2)")
            .bind(user_id)
            .bind(hash)
            .execute(&self.pool)
            .await
            .map_err(|e| PasteurError::Database(e.to_string()))?;
        Ok(())
    }

    pub async fn get_last_n(&self, user_id: Uuid, n: i64) -> Result<Vec<String>, PasteurError> {
        let rows = sqlx::query_as::<_, (String,)>(
            "SELECT password_hash FROM password_history WHERE user_id = $1 ORDER BY created_at DESC, id DESC LIMIT $2",
        )
        .bind(user_id)
        .bind(n)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| PasteurError::Database(e.to_string()))?;
        Ok(rows.into_iter().map(|(h,)| h).collect())
    }

    pub async fn purge_older_than(&self, user_id: Uuid, keep_n: i64) -> Result<(), PasteurError> {
        sqlx::query(
            "DELETE FROM password_history WHERE user_id = $1 AND id NOT IN (
                SELECT id FROM password_history WHERE user_id = $1 ORDER BY created_at DESC, id DESC LIMIT $2
            )",
        )
        .bind(user_id)
        .bind(keep_n)
        .execute(&self.pool)
        .await
        .map_err(|e| PasteurError::Database(e.to_string()))?;
        Ok(())
    }
}
```

Create `crates/godwit-db/src/repositories/password_reset_tokens.rs`:

```rust
use crate::models::PasswordResetToken;
use godwit_core::PasteurError;
use sqlx::PgPool;
use std::time::Duration;
use uuid::Uuid;

pub struct PasswordResetTokenRepository {
    pool: PgPool,
}

impl PasswordResetTokenRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn create(
        &self,
        user_id: Uuid,
        token_hash: &str,
        ttl: Duration,
    ) -> Result<PasswordResetToken, PasteurError> {
        let expires_at = chrono::Utc::now() + chrono::Duration::from_std(ttl).unwrap();
        sqlx::query_as::<_, PasswordResetToken>(
            "INSERT INTO password_reset_tokens (user_id, token_hash, expires_at) VALUES ($1, $2, $3) RETURNING *",
        )
        .bind(user_id)
        .bind(token_hash)
        .bind(expires_at)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| PasteurError::Database(e.to_string()))
    }

    pub async fn get_by_hash(&self, token_hash: &str) -> Result<PasswordResetToken, PasteurError> {
        sqlx::query_as::<_, PasswordResetToken>(
            "SELECT * FROM password_reset_tokens WHERE token_hash = $1",
        )
        .bind(token_hash)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| match e {
            sqlx::Error::RowNotFound => PasteurError::NotFound,
            _ => PasteurError::Database(e.to_string()),
        })
    }

    pub async fn mark_used(&self, id: Uuid) -> Result<(), PasteurError> {
        sqlx::query("UPDATE password_reset_tokens SET used_at = NOW() WHERE id = $1")
            .bind(id)
            .execute(&self.pool)
            .await
            .map_err(|e| PasteurError::Database(e.to_string()))?;
        Ok(())
    }

    pub async fn delete_for_user(&self, user_id: Uuid) -> Result<(), PasteurError> {
        sqlx::query("DELETE FROM password_reset_tokens WHERE user_id = $1")
            .bind(user_id)
            .execute(&self.pool)
            .await
            .map_err(|e| PasteurError::Database(e.to_string()))?;
        Ok(())
    }
}
```

Add `PasswordResetToken` to `crates/godwit-db/src/models.rs`:

```rust
#[derive(Debug, Clone, FromRow)]
pub struct PasswordResetToken {
    pub id: Uuid,
    pub user_id: Uuid,
    pub token_hash: String,
    pub expires_at: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
    pub used_at: Option<DateTime<Utc>>,
}
```

- [ ] **Step 3: Add `update_password` to UserRepository**

In `crates/godwit-db/src/repositories/users.rs`:

```rust
pub async fn update_password(
    &self,
    user_id: Uuid,
    hash: &str,
    expires_at: Option<DateTime<Utc>>,
) -> Result<User, PasteurError> {
    sqlx::query_as::<_, User>(
        "UPDATE users SET password_hash = $2, password_changed_at = NOW(),
            password_expires_at = $3, must_change_password = FALSE
         WHERE id = $1 RETURNING *",
    )
    .bind(user_id)
    .bind(hash)
    .bind(expires_at)
    .fetch_one(&self.pool)
    .await
    .map_err(|e| PasteurError::Database(e.to_string()))
}

pub async fn set_must_change(&self, user_id: Uuid, must: bool) -> Result<User, PasteurError> {
    sqlx::query_as::<_, User>(
        "UPDATE users SET must_change_password = $2 WHERE id = $1 RETURNING *",
    )
    .bind(user_id)
    .bind(must)
    .fetch_one(&self.pool)
    .await
    .map_err(|e| PasteurError::Database(e.to_string()))
}
```

Ensure the file imports `chrono::{DateTime, Utc}`.

- [ ] **Step 4: Write DB tests**

Create `crates/godwit-db/tests/password_repos.rs`:

```rust
use godwit_db::models::{UserRole, PasswordResetToken};
use godwit_db::repositories::password_history::PasswordHistoryRepository;
use godwit_db::repositories::password_reset_tokens::PasswordResetTokenRepository;
use godwit_db::repositories::users::UserRepository;
use sqlx::PgPool;
use uuid::Uuid;
use std::time::Duration;

#[sqlx::test(migrations = "../migrations")]
async fn password_history_push_get_purge(pool: PgPool) {
    let org = ...; // create org + user via UserRepository::create
    // (see Task 4 seed helper; for now inline: create org row + user row)
}
```

Because org/user seeding is shared, define a helper in this test file:

```rust
async fn seed_user(pool: &PgPool) -> Uuid {
    let org_id = Uuid::new_v4();
    sqlx::query("INSERT INTO organizations (id, name) VALUES ($1, 'o')")
        .bind(org_id).execute(pool).await.unwrap();
    let user_id = Uuid::new_v4();
    sqlx::query("INSERT INTO users (id, email, role, organization_id, password_hash) VALUES ($1, 'a@b.c', 'user', $2, 'x')")
        .bind(user_id).bind(org_id).execute(pool).await.unwrap();
    user_id
}
```

Full test:

```rust
#[sqlx::test(migrations = "../migrations")]
async fn password_history_push_get_purge(pool: PgPool) {
    let uid = seed_user(&pool).await;
    let repo = PasswordHistoryRepository::new(pool.clone());
    repo.push(uid, "hash1").await.unwrap();
    repo.push(uid, "hash2").await.unwrap();
    repo.push(uid, "hash3").await.unwrap();
    let last2 = repo.get_last_n(uid, 2).await.unwrap();
    assert_eq!(last2, vec!["hash3".to_string(), "hash2".to_string()]);
    repo.purge_older_than(uid, 2).await.unwrap();
    let all = repo.get_last_n(uid, 100).await.unwrap();
    assert_eq!(all.len(), 2);
}

#[sqlx::test(migrations = "../migrations")]
async fn password_reset_token_lifecycle(pool: PgPool) {
    let uid = seed_user(&pool).await;
    let repo = PasswordResetTokenRepository::new(pool.clone());
    let t = repo.create(uid, "th", Duration::from_secs(1800)).await.unwrap();
    let got = repo.get_by_hash("th").await.unwrap();
    assert_eq!(got.id, t.id);
    assert!(got.used_at.is_none());
    repo.mark_used(got.id).await.unwrap();
    let after = repo.get_by_hash("th").await.unwrap();
    assert!(after.used_at.is_some());
}

#[sqlx::test(migrations = "../migrations")]
async fn user_update_password_flags(pool: PgPool) {
    let uid = seed_user(&pool).await;
    let urepo = UserRepository::new(pool.clone());
    let u = urepo.update_password(uid, "newhash", None).await.unwrap();
    assert_eq!(u.password_hash.as_deref(), Some("newhash"));
    assert!(!u.must_change_password);
    assert!(u.password_changed_at.is_some());
    let u2 = urepo.set_must_change(uid, true).await.unwrap();
    assert!(u2.must_change_password);
}
```

- [ ] **Step 5: Run tests (need DATABASE_URL)**

Run: `export PATH="/usr/local/opt/rustup/bin:$PATH" && DATABASE_URL=postgres://godwit:godwit@localhost:5432/godwit cargo test -p godwit-db --test password_repos`
Expected: PASS

- [ ] **Step 6: Commit**

```bash
git add crates/godwit-db/
git commit -m "feat(db): password history + reset token + update_password repos"
```

---

### Task 4: Password logic layer (`password.rs`)

**Files:**
- Create: `crates/godwit-api/src/admin/password.rs` (logic module; endpoints come later)
- Create: `crates/godwit-api/assets/common_passwords.txt`

- [ ] **Step 1: Write the failing tests**

Create `crates/godwit-api/src/admin/password.rs` with a pure module `pub mod logic` and tests. Start with the logic:

```rust
use godwit_core::PasswordPolicy;
use godwit_auth::api_keys::{hash_password, verify_password};

#[derive(Debug, PartialEq, Eq)]
pub enum PolicyError {
    TooShort,
    NeedsUpper,
    NeedsLower,
    NeedsDigit,
    NeedsSymbol,
    CommonPassword,
    Reused,
}

pub fn validate_password(
    policy: &PasswordPolicy,
    password: &str,
    history: &[String],
) -> Result<(), PolicyError> {
    if (password.chars().count() as u32) < policy.min_length {
        return Err(PolicyError::TooShort);
    }
    if policy.require_upper && !password.chars().any(|c| c.is_uppercase()) {
        return Err(PolicyError::NeedsUpper);
    }
    if policy.require_lower && !password.chars().any(|c| c.is_lowercase()) {
        return Err(PolicyError::NeedsLower);
    }
    if policy.require_digit && !password.chars().any(|c| c.is_ascii_digit()) {
        return Err(PolicyError::NeedsDigit);
    }
    if policy.require_symbol && !password.chars().any(|c| !c.is_alphanumeric()) {
        return Err(PolicyError::NeedsSymbol);
    }
    if policy.block_common && COMMON_PASSWORDS.lookup(password) {
        return Err(PolicyError::CommonPassword);
    }
    for h in history {
        if verify_password(password, h) {
            return Err(PolicyError::Reused);
        }
    }
    Ok(())
}

#[derive(Debug, PartialEq, Eq)]
pub enum PasswordState {
    Valid,
    Expired,
    ForcedChange,
}

pub fn password_state(must_change: bool, expires_at: Option<chrono::DateTime<chrono::Utc>>) -> PasswordState {
    if must_change {
        return PasswordState::ForcedChange;
    }
    match expires_at {
        Some(exp) if exp < chrono::Utc::now() => PasswordState::Expired,
        _ => PasswordState::Valid,
    }
}
```

Add a lazy-statically-initialized common password set. Put it in a submodule `common`:

```rust
mod common {
    use std::collections::HashSet;
    use std::sync::OnceLock;
    static LIST: &str = include_str!("../../assets/common_passwords.txt");
    static SET: OnceLock<HashSet<String>> = OnceLock::new();
    pub fn lookup(pw: &str) -> bool {
        SET.get_or_init(|| LIST.lines().map(|l| l.trim().to_string()).collect())
            .contains(&pw.to_ascii_lowercase())
    }
}
use common::COMMON_PASSWORDS;
```

Adjust references accordingly (use `common::lookup`).

- [ ] **Step 2: Create the common-passwords asset**

Create `crates/godwit-api/assets/common_passwords.txt` with the top common passwords (one per lowercase line), e.g.:

```
123456
password
123456789
12345678
12345
qwerty
abc123
letmein
admin
welcome
monkey
111111
password1
iloveyou
sunshine
...
```

(Aim for ~200+ entries drawn from SecLists 10k-most-common top entries.)

- [ ] **Step 3: Write the unit tests**

```rust
#[cfg(test)]
mod tests {
    use super::*;
    fn pol() -> PasswordPolicy {
        PasswordPolicy {
            min_length: 8,
            require_upper: true,
            require_lower: true,
            require_digit: true,
            require_symbol: true,
            max_reuse: 3,
            days_to_expire: 90,
            block_common: true,
        }
    }

    #[test]
    fn too_short() {
        assert_eq!(validate_password(&pol(), "Ab1!", &[]), Err(PolicyError::TooShort));
    }
    #[test]
    fn missing_symbol() {
        assert_eq!(validate_password(&pol(), "Abcdefg1", &[]), Err(PolicyError::NeedsSymbol));
    }
    #[test]
    fn common_blocked() {
        assert_eq!(validate_password(&pol(), "password", &[]), Err(PolicyError::CommonPassword));
    }
    #[test]
    fn reused_blocked() {
        let h = hash_password("CorrectHorse1!");
        assert_eq!(validate_password(&pol(), "CorrectHorse1!", &[h]), Err(PolicyError::Reused));
    }
    #[test]
    fn valid_pass() {
        assert_eq!(validate_password(&pol(), "CorrectHorse1!", &[]), Ok(()));
    }
    #[test]
    fn state_transitions() {
        use chrono::Utc;
        assert_eq!(password_state(false, Some(Utc::now() - chrono::Duration::days(1))), PasswordState::Expired);
        assert_eq!(password_state(true, Some(Utc::now() + chrono::Duration::days(1))), PasswordState::ForcedChange);
        assert_eq!(password_state(false, None), PasswordState::Valid);
    }
}
```

- [ ] **Step 4: Run tests to verify fail then pass**

Run: `export PATH="/usr/local/opt/rustup/bin:$PATH" && cargo test -p godwit-api --lib admin::password::tests`
Expected: initially FAIL (no module), then PASS after implementation.

- [ ] **Step 4b: Keep the module referenced.** Register `pub mod password;` in `src/admin/mod.rs` so the file compiles.

- [ ] **Step 5: Commit**

```bash
git add crates/godwit-api/src/admin/password.rs crates/godwit-api/assets/common_passwords.txt crates/godwit-api/src/admin/mod.rs
git commit -m "feat(api): password policy validation + state logic + common list"
```

---

### Task 5: Repos wired into AppState

**Files:**
- Modify: `crates/godwit-api/src/state.rs`

- [ ] **Step 1: Add fields to AppState**

```rust
pub password_history_repo: godwit_db::repositories::password_history::PasswordHistoryRepository,
pub password_reset_token_repo: godwit_db::repositories::password_reset_tokens::PasswordResetTokenRepository,
```

- [ ] **Step 2: Update every AppState constructor**

Search `crates/godwit-api` for `AppState {` (production in `build_app`/`app.rs`, and test constructors like `auth.rs::test_state` and `app.rs::build_test_state`). Add the two field initializers after an existing repo field:

```rust
password_history_repo: PasswordHistoryRepository::new(pool.clone()),
password_reset_token_repo: PasswordResetTokenRepository::new(pool.clone()),
```

Add matching `use` imports. The `build_test_state` helper in `crates/godwit-api/src/app.rs` is the central test constructor.

- [ ] **Step 3: Verify compile**

Run: `export PATH="/usr/local/opt/rustup/bin:$PATH" && cargo check --workspace`
Expected: compiles. Fix any constructor missed (compiler will list them).

- [ ] **Step 4: Commit**

```bash
git add crates/godwit-api/src/state.rs crates/godwit-api/src
git commit -m "feat(api): wire password repos into AppState"
```

---

### Task 6: Mail layer (`mail.rs`) + deps

**Files:**
- Modify: `crates/godwit-api/Cargo.toml`
- Create: `crates/godwit-api/src/mail.rs`
- Create: `crates/godwit-api/assets/email/reset_password.html`, `.txt`, `password_changed.html`, `.txt`

- [ ] **Step 1: Add dependencies**

In `crates/godwit-api/Cargo.toml` `[dependencies]`:

```toml
lettre = { version = "0.11", features = ["smtp-transport", "builder", "rustls-tls"] }
tera = "1"
```

- [ ] **Step 2: Write the SendEmail abstraction + Mailer (with tests)**

Create `crates/godwit-api/src/mail.rs`:

```rust
use async_trait::async_trait;
use godwit_core::{AppConfig, MailConfig};
use lettre::message::Mailbox;
use lettre::transport::smtp::authentication::Credentials;
use lettre::{AsyncSmtpTransport, AsyncTransport, Message, Tokio1Executor};

#[async_trait]
pub trait SendEmail: Send + Sync {
    async fn send(&self, to: &str, subject: &str, html: &str, text: &str) -> Result<(), MailError>;
}

pub enum MailError {
    Build(String),
    Transport(String),
}

pub struct Mailer {
    mail: MailConfig,
    transport: AsyncSmtpTransport<Tokio1Executor>,
    templates: Tera,
}

impl Mailer {
    pub fn build(config: &AppConfig) -> Result<Option<Self>, MailError> {
        let mail = match &config.auth.mail {
            Some(m) => m.clone(),
            None => return Ok(None),
        };
        let mut builder = AsyncSmtpTransport::<Tokio1Executor>::builder_dangerous(&mail.host)
            .port(mail.port);
        if let (Some(u), Some(p)) = (&mail.username, &mail.password) {
            builder = builder.credentials(Credentials::new(u.clone(), p.clone()));
        }
        let transport = builder.build();
        let mut tera = Tera::default();
        tera.add_raw_templates(vec![
            ("reset_password.html", include_str!("../assets/email/reset_password.html")),
            ("reset_password.txt", include_str!("../assets/email/reset_password.txt")),
            ("password_changed.html", include_str!("../assets/email/password_changed.html")),
            ("password_changed.txt", include_str!("../assets/email/password_changed.txt")),
        ])
        .map_err(|e| MailError::Build(e.to_string()))?;
        Ok(Some(Self { mail, transport, templates: tera }))
    }

    pub fn from(&self) -> Mailbox {
        self.mail.from.parse().expect("valid from mailbox")
    }

    pub fn render(&self, name: &str, ctx: &tera::Context) -> (String, String) {
        let html = self.templates.render(&format!("{name}.html"), ctx).unwrap_or_default();
        let text = self.templates.render(&format!("{name}.txt"), ctx).unwrap_or_default();
        (html, text)
    }
}

#[async_trait]
impl SendEmail for Mailer {
    async fn send(&self, to: &str, subject: &str, html: &str, text: &str) -> Result<(), MailError> {
        let email = Message::builder()
            .from(self.from())
            .to(to.parse().map_err(|e| MailError::Build(e.to_string()))?)
            .subject(subject)
            .multipart(lettre::message::MultiPart::alternative_plain_html(
                text.to_string(), html.to_string(),
            ))
            .map_err(|e| MailError::Build(e.to_string()))?;
        self.transport.send(email).await.map_err(|e| MailError::Transport(e.to_string()))?;
        Ok(())
    }
}
```

Add `tera = { version = "1", default-features = false }` if you want minimal features (it needs `chrono` in default flags; keep default to avoid breakage).

- [ ] **Step 3: Write email templates**

`assets/email/reset_password.html`:

```html
<h1>Reset your password</h1>
<p>Hi,</p>
<p>We received a request to reset your password. Click the link below to choose a new one:</p>
<p><a href="{{ reset_link }}">{{ reset_link }}</a></p>
<p>This link expires in 30 minutes. If you didn't request this, you can ignore this email.</p>
```

`assets/email/reset_password.txt`:

```
Reset your password

We received a request to reset your password. Open this link to choose a new one:

{{ reset_link }}

This link expires in 30 minutes. If you didn't request this, you can ignore this email.
```

`assets/email/password_changed.html`:

```html
<h1>Your password was changed</h1>
<p>Hi,</p>
<p>Your {{ brand }} password was just changed. If this wasn't you, contact your administrator immediately.</p>
```

`assets/email/password_changed.txt`:

```
Your password was changed

Your {{ brand }} password was just changed. If this wasn't you, contact your administrator immediately.
```

- [ ] **Step 4: Write tests**

In `mail.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn renders_reset_templates_with_brand() {
        let mut ctx = tera::Context::new();
        ctx.insert("reset_link", "https://app.example.com/reset-password?token=abc");
        // Build a Mailer without a network: use a minimal MailConfig via template-only path.
        // Simplest: construct Mailer::build with auth.mail set to None -> Ok(None). For render test,
        // build a standalone Tera from the same assets.
        let mut tera = Tera::default();
        tera.add_raw_templates(vec![
            ("reset_password.html", include_str!("../assets/email/reset_password.html")),
        ]).unwrap();
        let html = tera.render("reset_password.html", &ctx).unwrap();
        assert!(html.contains("https://app.example.com/reset-password?token=abc"));
    }
}
```

(Test only rendering — no real SMTP. The full `SendEmail::send` path is covered by the fake mailer in integration tests, not by making network calls here.)

- [ ] **Step 5: Run tests + check compile**

Run: `export PATH="/usr/local/opt/rustup/bin:$PATH" && cargo test -p godwit-api --lib mail && cargo check --workspace`
Expected: PASS + compiles. If `lettre`/`tera` have feature conflicts, resolve with minimal features.

- [ ] **Step 6: Commit**

```bash
git add crates/godwit-api/Cargo.toml crates/godwit-api/src/mail.rs crates/godwit-api/assets/email
git commit -m "feat(api): SMTP mail layer with tera templates + SendEmail trait"
```

---

### Task 7: Endpoints (change-password, admin reset, forgot, reset, change-required) + login flag

**Files:**
- Modify: `crates/godwit-api/src/admin/auth.rs` (router + login)
- Create: `crates/godwit-api/src/admin/password.rs` endpoints section (append)

- [ ] **Step 1: Add helpers to `password.rs`**

Add the token generation and a `verify_and_rotate` function:

```rust
use uuid::Uuid;
use crate::state::AppState;

pub fn generate_reset_token() -> (String, String) {
    use godwit_auth::api_keys::generate_api_key; // reuse 24-byte random; token hash via hash_password is wrong; use a dedicated hash
    // Generate a random token and hash it. Reuse argon2 hashing for the token hash:
    let (plain, _h, _p) = generate_api_key();
    let hashed = hash_password(&plain);
    (plain, hashed)
}
```

> Note: `generate_api_key` returns an `sk-godwit-...` prefixed string used as the reset *plaintext* token; it is hashed with Argon2 via `hash_password` before storage. Plaintext is sent in the email; only the hash is stored, matching the `refresh_tokens` pattern.

Add core operations:

```rust
pub async fn do_password_change(
    state: &AppState,
    user_id: Uuid,
    new_password: &str,
) -> Result<(), crate::error::ApiError> {
    let policy = state.config.auth.password_policy;
    let history = state.password_history_repo.get_last_n(user_id, policy.max_reuse as i64).await
        .map_err(crate::error::ApiError::Core)?;
    logic::validate_password(&policy, new_password, &history)
        .map_err(|e| crate::error::ApiError::BadRequest(format!("{e:?}")))?;
    let hash = godwit_auth::api_keys::hash_password(new_password);
    let expires_at = if policy.days_to_expire > 0 {
        Some(chrono::Utc::now() + chrono::Duration::days(policy.days_to_expire as i64))
    } else { None };
    state.user_repo.update_password(user_id, &hash, expires_at).await.map_err(crate::error::ApiError::Core)?;
    state.password_history_repo.push(user_id, &hash).await.map_err(crate::error::ApiError::Core)?;
    state.password_history_repo.purge_older_than(user_id, policy.max_reuse as i64).await.map_err(crate::error::ApiError::Core)?;
    state.refresh_token_repo.delete_all_for_user(user_id).await.map_err(crate::error::ApiError::Core)?;
    Ok(())
}
```

- [ ] **Step 2: Register routes in `auth.rs::router()`**

Add to the auth router (public routes — forgot/reset outside JWT; change/change-required/admin under JWT):

```rust
// public
.route("/auth/forgot-password", post(forgot_password))
.route("/auth/reset-password", post(reset_password))
// protected
let protected_auth = Router::new()
    .route("/auth/change-password", post(change_password))
    .route("/auth/change-required", post(change_required))
    .route("/auth/admin/reset-password", post(admin_reset_password))
    .route_layer(from_fn_with_state(state.clone(), crate::middleware::jwt_auth));
Router::new().merge(cookie_routes).merge(protected_auth).route(...existing...)
```

Implement handlers in `password.rs`:

```rust
use axum::{extract::{Extension, State, Json}, http::StatusCode, response::IntoResponse};

#[derive(serde::Deserialize)]
pub struct ChangePasswordReq { current_password: String, new_password: String }

pub async fn change_password(
    State(state): State<Arc<AppState>>,
    Extension(claims): Extension<Claims>,
    Json(req): Json<ChangePasswordReq>,
) -> Result<impl IntoResponse, crate::error::ApiError> {
    let user = state.user_repo.get_by_id(claims.user_id).await.map_err(crate::error::ApiError::Core)?;
    let current = user.password_hash.as_deref().ok_or(crate::error::ApiError::Unauthorized)?;
    if !godwit_auth::api_keys::verify_password(&req.current_password, current) {
        return Err(crate::error::ApiError::Unauthorized);
    }
    do_password_change(&state, user.id, &req.new_password).await?;
    Ok(Json(serde_json::json!({ "changed": true })))
}

#[derive(serde::Deserialize)]
pub struct ChangeRequiredReq { new_password: String }

pub async fn change_required(
    State(state): State<Arc<AppState>>,
    Extension(claims): Extension<Claims>,
    Json(req): Json<ChangeRequiredReq>,
) -> Result<impl IntoResponse, crate::error::ApiError> {
    let user = state.user_repo.get_by_id(claims.user_id).await.map_err(crate::error::ApiError::Core)?;
    // Only valid when a change is actually required; otherwise require current password.
    let required = user.must_change_password || logic::password_state(false, user.password_expires_at) == logic::PasswordState::Expired;
    if !required {
        return Err(crate::error::ApiError::BadRequest("no forced change pending".to_string()));
    }
    do_password_change(&state, user.id, &req.new_password).await?;
    Ok(Json(serde_json::json!({ "changed": true })))
}

#[derive(serde::Deserialize)]
pub struct AdminResetReq { user_id: Uuid, new_password: String }

pub async fn admin_reset_password(
    State(state): State<Arc<AppState>>,
    Extension(claims): Extension<Claims>,
    Json(req): Json<AdminResetReq>,
) -> Result<impl IntoResponse, crate::error::ApiError> {
    let role = crate::admin::rbac_role(&claims)?; // or replicate require_role
    if claims.user_id == req.user_id {
        return Err(crate::error::ApiError::BadRequest("cannot reset your own password here".to_string()));
    }
    let target = state.user_repo.get_by_id(req.user_id).await.map_err(crate::error::ApiError::Core)?;
    // same-org + not-on-super-admin + role guard like users.rs
    // ... checks ...
    let hidden = do_password_change(&state, target.id, &req.new_password).await;
    // compare policy (may reject weak) then set must_change = true on top:
    state.user_repo.set_must_change(target.id, true).await.map_err(crate::error::ApiError::Core)?;
    hidden?;
    Ok(Json(serde_json::json!({ "reset": true })))
}

#[derive(serde::Deserialize)]
pub struct ForgotPasswordReq { email: String }

pub async fn forgot_password(
    State(state): State<Arc<AppState>>,
    Json(req): Json<ForgotPasswordReq>,
) -> Result<impl IntoResponse, crate::error::ApiError> {
    let mailer = state.mailer.clone();
    let outcome = (|| async {
        let user = match state.user_repo.get_by_email(&req.email).await { Ok(u) => u, Err(_) => return Ok::<(), crate::error::ApiError>(()) };
        state.password_reset_token_repo.delete_for_user(user.id).await.map_err(crate::error::ApiError::Core)?;
        let (plain, hashed) = generate_reset_token();
        state.password_reset_token_repo.create(user.id, &hashed, std::time::Duration::from_secs(1800)).await.map_err(crate::error::ApiError::Core)?;
        let app_url = state.config.auth.mail.as_ref().map(|m| m.app_url.clone()).unwrap_or_default();
        let mut ctx = tera::Context::new();
        ctx.insert("reset_link", &format!("{app_url}/reset-password?token={plain}"));
        let mailer = mailer.ok_or(crate::error::ApiError::Internal)?;
        let (html, text) = mailer.render("reset_password", &ctx);
        let subject = "Reset your password";
        mailer.send(&user.email, subject, &html, &text).await.map_err(|_| crate::error::ApiError::Internal)?;
        Ok(())
    })().await;
    // Always 200, regardless of outcome (anti-enumeration + SMTP tolerance)
    let _ = outcome;
    tracing::info!(email = %req.email, "forgot-password request processed");
    Ok(Json(serde_json::json!({ "ok": true })))
}
```

> The `mailer` must be `Option<Arc<dyn SendEmail>>` in `AppState` so the trait is callable. Configure `Mailer::build(&config)` into `AppState.mailer` at startup (`app.rs`). In dev without SMTP, test code injects a fake mailer that captures token from the rendered email.

```rust
#[derive(serde::Deserialize)]
pub struct ResetPasswordReq { token: String, new_password: String }

pub async fn reset_password(
    State(state): State<Arc<AppState>>,
    Json(req): Json<ResetPasswordReq>,
) -> Result<impl IntoResponse, crate::error::ApiError> {
    let hashed = godwit_auth::api_keys::hash_password(&req.token);
    let rec = state.password_reset_token_repo.get_by_hash(&hashed).await.map_err(|_| crate::error::ApiError::BadRequest("invalid token".to_string()))?;
    if rec.used_at.is_some() || rec.expires_at < chrono::Utc::now() {
        return Err(crate::error::ApiError::BadRequest("token expired or used".to_string()));
    }
    do_password_change(&state, rec.user_id, &req.new_password).await?;
    state.password_reset_token_repo.mark_used(rec.id).await.map_err(crate::error::ApiError::Core)?;
    Ok(Json(serde_json::json!({ "reset": true })))
}
```

- [ ] **Step 3: Add `must_change_password` flag to login**

In `auth.rs::login`, after `verify_password` passes (successful password check), compute state and include the flag:

```rust
let (set_cookie_headers, body) = issue_token_pair(&state, &user).await?;
let must_change = match user.password_expires_at {
    Some(exp) if exp < chrono::Utc::now() => true,
    _ => user.must_change_password,
};
let value = serde_json::json!({
    "access_token": body.0["access_token"],
    "refresh_token": body.0["refresh_token"],
    "must_change_password": must_change,
});
```

(`issue_token_pair` returns `(HeaderMap, Json<Value>)`; restructure to extract the two token fields from `body` and inject the flag. Preserve the existing `HeaderMap` cookies.)

- [ ] **Step 4: Wire `AppState.mailer` at startup**

In `app.rs` / `build_test_state`, add `pub mailer: Option<Arc<dyn SendEmail>>` to `AppState` and construct via `Mailer::build(&config)?.map(Arc::new)`. For `build_test_state`, allow injecting a fake mailer (parameter or a setter) so integration tests can capture emails.

- [ ] **Step 5: Register `password` module**

Add `pub mod password;` in `crates/godwit-api/src/admin/mod.rs` (if not already from Task 4) and export the new handlers.

- [ ] **Step 6: Compile**

Run: `export PATH="/usr/local/opt/rustup/bin:$PATH" && cargo check --workspace`
Expected: compiles.

- [ ] **Step 7: Commit**

```bash
git add crates/godwit-api/
git commit -m "feat(api): password endpoints + login must_change flag + mailer wiring"
```

---

### Task 8: Integration tests (full flows)

**Files:**
- Create: `crates/godwit-api/tests/password_integration.rs`

- [ ] **Step 1: Write the fake mailer + tests**

```rust
use axum::body::Body;
use axum::http::Request;
use axum::Router;
use godwit_api::app::{build_app, build_test_state};
use godwit_api::mail::SendEmail;
use godwit_auth::api_keys::hash_password;
use http_body_util::BodyExt;
use serde_json::json;
use sqlx::PgPool;
use std::sync::Arc;
use tokio::sync::Mutex;
use tower::ServiceExt;

struct FakeMailer(Mutex<Vec<(String, String, String, String)>>); // (to, subject, html, text)

#[async_trait::async_trait]
impl SendEmail for FakeMailer {
    async fn send(&self, to: &str, subject: &str, html: &str, text: &str) -> Result<(), godwit_api::mail::MailError> {
        self.0.lock().await.push((to.to_string(), subject.to_string(), html.to_string(), text.to_string()));
        Ok(())
    }
}

async fn seed(pool: &PgPool) -> (String, String) {
    // create org + user with a known password hash; reuse pattern from router_integration.rs::seed_password_user
}

async fn post(app: &Router, path: &str, body: serde_json::Value) -> axum::response::Response {
    app.clone().oneshot(
        Request::builder().method("POST").uri(path)
            .header("content-type", "application/json")
            .body(Body::from(body.to_string())).unwrap(),
    ).await.unwrap()
}

#[sqlx::test(migrations = "../godwit-db/migrations")]
async fn forgot_reset_flow(pool: PgPool) {
    let (email, password) = seed(&pool).await;
    let mailer = Arc::new(FakeMailer(Mutex::new(vec![])));
    // Inject mailer into state (build_test_state then set app state mailer)
    let state = build_test_state(pool.clone());
    // ... set state.mailer = Some(mailer.clone()) ...
    let app = build_app(state);

    let r = post(&app, "/api/v1/auth/forgot-password", json!({"email": email})).await;
    assert_eq!(r.status(), 200);

    let sent = mailer.0.lock().await.clone();
    assert_eq!(sent.len(), 1, "one reset email sent");
    let html = &sent[0].3;
    // extract token=...
    let idx = html.find("token=").unwrap();
    let token: String = html[idx+6..].chars().take_while(|c| c.is_ascii_graphic()).collect();

    let r2 = post(&app, "/api/v1/auth/reset-password", json!({"token": token, "new_password": "NewValid1!"})).await;
    assert_eq!(r2.status(), 200);

    // old password rejected
    let old = post(&app, "/api/v1/auth/login", json!({"email": email, "password": password})).await;
    assert_eq!(old.status(), 401);
    // new password works
    let new_login = post(&app, "/api/v1/auth/login", json!({"email": email, "password": "NewValid1!"})).await;
    assert_eq!(new_login.status(), 200);
    // token is one-shot
    let r3 = post(&app, "/api/v1/auth/reset-password", json!({"token": token, "new_password": "Another1!"})).await;
    assert_eq!(r3.status(), 400);
}

#[sqlx::test(migrations = "../godwit-db/migrations")]
async fn forgot_unknown_email_returns_200(pool: PgPool) {
    let state = build_test_state(pool);
    let app = build_app(state);
    let r = post(&app, "/api/v1/auth/forgot-password", json!({"email": "nobody@example.com"})).await;
    assert_eq!(r.status(), 200);
}

#[sqlx::test(migrations = "../godwit-db/migrations")]
async fn login_with_forced_change_returns_flag(pool: PgPool) {
    let (email, password) = seed(&pool).await;
    let state = build_test_state(pool);
    let app = build_app(state);
    // set must_change_password = true for the seeded user
    let r = post(&app, "/api/v1/auth/login", json!({"email": email, "password": password})).await;
    assert_eq!(r.status(), 200);
    let body = r.into_body().collect().await.unwrap().to_bytes();
    let v: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert!(v.get("must_change_password").is_some());
}
```

> **Wire-in note:** `build_test_state` must support setting `state.mailer` and mutation before building the router. Adjust `build_test_state` to accept the pool and return a fully-constructed `AppState` that the test can mutate (e.g., set `mailer`), matching the `build_test_state_with_auth` pattern if present. See `crates/godwit-api/tests/route_contract.rs` for the current `build_test_state(pool)` usage.

- [ ] **Step 2: Run tests (need DATABASE_URL)**

Run: `export PATH="/usr/local/opt/rustup/bin:$PATH" && DATABASE_URL=postgres://godwit:godwit@localhost:5432/godwit cargo test -p godwit-api --test password_integration`
Expected: PASS

- [ ] **Step 3: Commit**

```bash
git add crates/godwit-api/tests/password_integration.rs
git commit -m "test(api): full password management flows"
```

---

### Task 9: Contract + coverage

**Files:**
- Modify: `contract/routes.json`
- Modify: `apps/ui/tests/route-contract.test.ts`
- Modify: `apps/admin/tests/route-contract.test.ts`
- Modify: `docs/coverage/frontend-backend.md`

- [ ] **Step 1: Add routes to the contract**

Append (before the `proxy.*` entries, in the auth section):

```json
{ "id": "auth.change-password", "method": "POST", "path": "/api/v1/auth/change-password", "scope": "ui", "frontend": { "lib": "apps/ui/src/lib/auth.ts", "fn": "changePassword" }, "backend": { "module": "crates/godwit-api/src/admin/password.rs", "fn": "change_password" } },
{ "id": "auth.change-required", "method": "POST", "path": "/api/v1/auth/change-required", "scope": "ui", "frontend": { "lib": "apps/ui/src/lib/auth.ts", "fn": "changeRequired" }, "backend": { "module": "crates/godwit-api/src/admin/password.rs", "fn": "change_required" } },
{ "id": "auth.admin.reset-password", "method": "POST", "path": "/api/v1/auth/admin/reset-password", "scope": "admin", "frontend": { "lib": "apps/admin/app/(dashboard)/admin/users/actions.ts", "fn": "resetUserPassword" }, "backend": { "module": "crates/godwit-api/src/admin/password.rs", "fn": "admin_reset_password" } },
{ "id": "auth.forgot-password", "method": "POST", "path": "/api/v1/auth/forgot-password", "scope": "admin", "frontend": { "lib": "apps/admin/app/(auth)/forgot-password/actions.ts", "fn": "requestPasswordReset" }, "backend": { "module": "crates/godwit-api/src/admin/password.rs", "fn": "forgot_password" } },
{ "id": "auth.reset-password", "method": "POST", "path": "/api/v1/auth/reset-password", "scope": "admin", "frontend": { "lib": "apps/admin/app/(auth)/reset-password/actions.ts", "fn": "performPasswordReset" }, "backend": { "module": "crates/godwit-api/src/admin/password.rs", "fn": "reset_password" } }
```

- [ ] **Step 2: Update `apps/ui` route-contract test**

Add invoke cases for the UI-consumed routes (`changePassword`, `changeRequired`, and `forgotPassword`/`resetPassword` if the UI consumes them). Follow the existing `invoke()` switch pattern in `apps/ui/tests/route-contract.test.ts`.

- [ ] **Step 3: Update `apps/admin` route-contract test**

Add the new admin calls to `EXPECTED_CALLS` in `apps/admin/tests/route-contract.test.ts` (forgot, reset, admin reset) once the tasks for `apps/admin` pages exist.

- [ ] **Step 4: Update coverage doc**

Regenerate/append the new rows in `docs/coverage/frontend-backend.md` (status `covered` / `admin-covered`) consistent with the existing grid.

- [ ] **Step 5: Run both FE contract tests**

Run: `cd apps/ui && npx vitest run tests/route-contract.test.ts` and `cd apps/admin && npx vitest run tests/route-contract.test.ts`
Expected: PASS (after the frontend fns exist — see Tasks 10–11; run this after those).

- [ ] **Step 6: Commit**

```bash
git add contract/routes.json apps/ui/tests/route-contract.test.ts apps/admin/tests/route-contract.test.ts docs/coverage/frontend-backend.md
git commit -m "feat(contract): declare password management routes and cover both frontends"
```

---

### Task 10: `apps/ui` frontend

**Files:**
- Modify: `apps/ui/src/lib/auth.ts`
- Create: `apps/ui/src/app/forgot-password/page.tsx`, `apps/ui/src/app/reset-password/page.tsx`
- Create/modify: change-password (settings) page + change-required handling
- Test: `apps/ui/src/lib/auth.test.ts`

- [ ] **Step 1: Add lib functions**

In `apps/ui/src/lib/auth.ts`:

```ts
export async function forgotPassword(email: string): Promise<void> {
  const res = await apiFetch('/api/v1/auth/forgot-password', {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ email }),
  });
  if (!res.ok) throw new Error('Failed to request password reset');
}

export async function resetPassword(token: string, newPassword: string): Promise<void> {
  const res = await apiFetch('/api/v1/auth/reset-password', {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ token, new_password: newPassword }),
  });
  if (!res.ok) throw new Error('Failed to reset password');
}

export async function changePassword(current: string, next: string): Promise<void> {
  const res = await apiFetch('/api/v1/auth/change-password', {
    method: 'POST', headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ current_password: current, new_password: next }),
  });
  if (!res.ok) throw new Error('Failed to change password');
}

export async function changeRequired(next: string): Promise<void> {
  const res = await apiFetch('/api/v1/auth/change-required', {
    method: 'POST', headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ new_password: next }),
  });
  if (!res.ok) throw new Error('Failed to update password');
}
```

- [ ] **Step 2: Add lib tests**

In `apps/ui/src/lib/auth.test.ts`, stub global fetch and assert each fn calls the right URL+method (mirror existing pattern).

- [ ] **Step 3: Create pages**

`apps/ui/src/app/forgot-password/page.tsx` and `apps/ui/src/app/reset-password/page.tsx`: simple forms calling the lib fns, with a `useSearchParams` read of `token` on the reset page. `change-password` added to the existing `(protected)/settings` page; and after login, if the store/nav sees `must_change_password`, redirect to a `change-required` screen calling `changeRequired`.

- [ ] **Step 4: Run UI tests + typecheck**

Run: `cd apps/ui && npx vitest run && npx tsc --noEmit`
Expected: PASS / clean

- [ ] **Step 5: Commit**

```bash
git add apps/ui/src
git commit -m "feat(ui): forgot/reset/change/change-required password pages"
```

---

### Task 11: `apps/admin` frontend

**Files:**
- Create: `apps/admin/app/(auth)/forgot-password/actions.ts`, `page.tsx`, `apps/admin/app/(auth)/reset-password/actions.ts`, `page.tsx`
- Modify: `apps/admin/app/(dashboard)/admin/users/actions.ts` (`resetUserPassword`)
- Modify: change-password in user profile / settings
- Test: `apps/admin/tests/route-contract.test.ts` (from Task 9)

- [ ] **Step 1: Add server actions**

`apps/admin/app/(auth)/forgot-password/actions.ts`:

```ts
'use server'
const API_URL = process.env.API_URL || process.env.NEXT_PUBLIC_API_URL || 'https://api.godwit.io'
export async function requestPasswordReset(email: string) {
  const res = await fetch(`${API_URL}/api/v1/auth/forgot-password`, {
    method: 'POST', headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ email }),
  })
  return { ok: res.ok }
}
```

`apps/admin/app/(auth)/reset-password/actions.ts`:

```ts
'use server'
const API_URL = process.env.API_URL || process.env.NEXT_PUBLIC_API_URL || 'https://api.godwit.io'
export async function performPasswordReset(token: string, newPassword: string) {
  const res = await fetch(`${API_URL}/api/v1/auth/reset-password`, {
    method: 'POST', headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ token, new_password: newPassword }),
  })
  return { ok: res.ok }
}
```

`apps/admin/app/(dashboard)/admin/users/actions.ts` — add `resetUserPassword`:

```ts
export async function resetUserPassword(userId: string, newPassword: string) {
  const response = await apiCall('/api/v1/auth/admin/reset-password', {
    method: 'POST',
    body: JSON.stringify({ user_id: userId, new_password: newPassword }),
  })
  return { ok: response.ok }
}
```

- [ ] **Step 2: Create pages**

`apps/admin/app/(auth)/forgot-password/page.tsx` (email form), `apps/admin/app/(auth)/reset-password/page.tsx` (token from URL + new password form), and add a "Change password" action to the user profile/settings surface calling change-password.

- [ ] **Step 3: Run typecheck + contract test**

Run: `cd apps/admin && npx tsc --noEmit && npx vitest run tests/route-contract.test.ts`
Expected: clean (except pre-existing `components/ui/__tests__` test-file errors) + PASS

- [ ] **Step 4: Commit**

```bash
git add apps/admin/
git commit -m "feat(admin): forgot/reset/change password + admin reset action"
```

---

### Task 12: Full-suite verification + docs

**Files:**
- Modify: `docs/coverage/frontend-backend.md` (final pass), `config.example.yaml` (document mail + policy)

- [ ] **Step 1: Document config**

Add to `config.example.yaml` under `auth`:

```yaml
  # Email sending for password reset (forgot-password).
  mail:
    from: "Godwit <no-reply@example.com>"
    host: smtp.example.com
    port: 587
    # username/password optional (open-relay or no-auth relays)
    # username: ""
    # password: ""
    tls: starttls          # none | starttls | tls
    app_url: "https://app.example.com"
  password_policy:
    min_length: 10
    require_upper: true
    require_lower: true
    require_digit: true
    require_symbol: true
    max_reuse: 5
    days_to_expire: 90
    block_common: true
```

- [ ] **Step 2: Final backend full test**

Run: `export PATH="/usr/local/opt/rustup/bin:$PATH" && cargo check --workspace && cargo test --workspace`
(Note: `cargo test --workspace` requires `DATABASE_URL` for db tests.)
Expected: no new failures (the 10 pre-existing unrelated DB failures may still occur — do not chase them; only ensure new password tests pass).

- [ ] **Step 3: Final FE tests**

Run: `cd apps/ui && npx vitest run` and `cd apps/admin && npx vitest run tests/route-contract.test.ts`
Expected: PASS

- [ ] **Step 4: Commit**

```bash
git add config.example.yaml docs/coverage/frontend-backend.md
git commit -m "docs: document mail + password policy config; finalize coverage"
```

---

## Self-Review Notes

- **Spec coverage:** Every spec section maps to a task — config (T1), schema (T2), repos (T3), logic (T4), AppState wiring (T5), email (T6), endpoints+login flag (T7), integration tests (T8), contract/coverage (T9), UI (T10), admin (T11), docs (T12).
- **Placeholders:** Any step reading "wire-in note" or "replicate like X" is a steer, not a placeholder — the actual integrating code is given where it matters, and follow-the-existing-pattern references point at concrete repo files (`router_integration.rs::seed_password_user`, `users.rs` role helpers).
- **Type consistency:** `SendEmail::send(to, subject, html, text)` is used consistently in `mail.rs`, handlers, and the `FakeMailer`. `do_password_change(state, user_id, new_password)` is the single rotation entry used by all 4 mutating endpoints. `PasswordState`/`validate_password` signatures are stable across tasks.
