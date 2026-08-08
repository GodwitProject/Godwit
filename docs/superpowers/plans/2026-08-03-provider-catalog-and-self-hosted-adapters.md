# Instance-Wide Provider Catalog & Self-Hosted Adapters Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Turn Godwit's per-organization provider catalog into an instance-wide, `super_admin`-managed catalog with encrypted, database-backed credentials; add wildcard passthrough routing; add four self-hosted adapters (vllm, sglang, llama.cpp, ollama); and expose the multimodal proxy routes (`/v1/embeddings`, `/v1/images/generations`, `/v1/images/edits`, `/v1/audio/speech`, `/v1/audio/transcriptions`) that already-implemented adapter capabilities have never had.

**Architecture:** `provider_profiles`/`models` drop `organization_id` and become global tables. Credentials move from static `config.yaml` to an AES-256-GCM-encrypted `auth` column, decrypted once per request by `DbModelRouter` into a stateless `ResolvedProfile` passed to every `Adapter` call. Organizations/teams are untouched — they keep scoping users, API keys, budgets, and `request_logs`.

**Tech Stack:** Rust, Axum, SQLx/PostgreSQL, `aes-gcm` (new dependency), `base64` (new dependency), existing `reqwest`/`wiremock`/`sqlx::test` patterns.

## Global Constraints

- Spec: `docs/superpowers/specs/2026-08-03-provider-catalog-and-self-hosted-adapters-design.md` — every task below traces to a section of that spec.
- No BYOK, no per-org/per-team credential scoping — the catalog and credentials are global to the instance (spec §1, §3).
- No adapters for azure_openai, bedrock, cohere, mistral, groq, together in this plan (spec §3 out-of-scope).
- The four self-hosted adapters implement only `Chat`, `chat_stream`, and `Embedding` — no image/audio capabilities, because vllm/sglang/llama.cpp/ollama don't expose such endpoints (spec §7).
- `ImageEdit` is implemented for the OpenAI adapter only; every other adapter returns `ProviderError::CapabilityNotSupported` for it (spec §8).
- Toolchain: `export PATH="/usr/local/opt/rustup/bin:$PATH"` before any `cargo` command (per `AGENTS.md`). DB tests need `DATABASE_URL` pointing at a reachable PostgreSQL 15+ instance.
- Follow existing repo conventions: `PasteurError` for domain errors mapped through `ApiError` (`crates/godwit-api/src/error.rs`), `sqlx::query_as::<_, T>` with `.map_err(|e| PasteurError::Database(e.to_string()))`, RBAC via `Role::from_str(&claims.role)` checked against `Extension<Claims>` (see `crates/godwit-api/src/admin/organizations.rs` for the canonical pattern).

---

## Task 1: `ImageEdit` Capability

**Files:**
- Modify: `crates/godwit-core/src/lib.rs` (the `Capability` enum, ~lines 122-166)
- Create: `crates/godwit-db/migrations/20260803000001_image_edit_capability.up.sql`
- Create: `crates/godwit-db/migrations/20260803000001_image_edit_capability.down.sql`
- Test: `crates/godwit-core/src/lib.rs` (inline), `crates/godwit-db/src/lib.rs` (inline, existing check-constraint test)

**Interfaces:**
- Produces: `godwit_core::Capability::ImageEdit`, `Capability::as_str() == "image_edit"`, `Capability::from_str("image_edit")`.

- [ ] **Step 1: Write the failing test**

In `crates/godwit-core/src/lib.rs`, extend the existing `capability_round_trips` and `capability_serde_roundtrip` tests to include the new variant:

```rust
    #[test]
    fn capability_round_trips() {
        let capabilities = [
            Capability::Chat,
            Capability::ImageGeneration,
            Capability::ImageEdit,
            Capability::VideoGeneration,
            Capability::AudioTts,
            Capability::AudioStt,
            Capability::Embedding,
        ];
        for cap in capabilities {
            let s = cap.as_str();
            assert_eq!(cap.to_string(), s);
            assert_eq!(Capability::from_str(s).unwrap(), cap);
        }
        assert!(Capability::from_str("unknown").is_err());
    }
```

Apply the same addition (`Capability::ImageEdit` in the array) to `capability_serde_roundtrip`.

- [ ] **Step 2: Run test to verify it fails**

Run: `export PATH="/usr/local/opt/rustup/bin:$PATH" && cargo test -p godwit-core capability_round_trips`
Expected: FAIL with `no variant named ImageEdit found for enum Capability`.

- [ ] **Step 3: Add the variant**

In `crates/godwit-core/src/lib.rs`, change:

```rust
pub enum Capability {
    Chat,
    ImageGeneration,
    VideoGeneration,
    AudioTts,
    AudioStt,
    Embedding,
}
```

to:

```rust
pub enum Capability {
    Chat,
    ImageGeneration,
    ImageEdit,
    VideoGeneration,
    AudioTts,
    AudioStt,
    Embedding,
}
```

Update `as_str`:

```rust
    pub fn as_str(&self) -> &'static str {
        match self {
            Capability::Chat => "chat",
            Capability::ImageGeneration => "image_generation",
            Capability::ImageEdit => "image_edit",
            Capability::VideoGeneration => "video_generation",
            Capability::AudioTts => "audio_tts",
            Capability::AudioStt => "audio_stt",
            Capability::Embedding => "embedding",
        }
    }
```

Update `FromStr`:

```rust
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "chat" => Ok(Self::Chat),
            "image_generation" => Ok(Self::ImageGeneration),
            "image_edit" => Ok(Self::ImageEdit),
            "video_generation" => Ok(Self::VideoGeneration),
            "audio_tts" => Ok(Self::AudioTts),
            "audio_stt" => Ok(Self::AudioStt),
            "embedding" => Ok(Self::Embedding),
            _ => Err(format!("unknown capability: {s}")),
        }
    }
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p godwit-core capability`
Expected: PASS (both `capability_round_trips` and `capability_serde_roundtrip`).

- [ ] **Step 5: Write the failing DB test**

In `crates/godwit-db/src/lib.rs`, find the existing test that rejects an invalid capability value (`models_capability_check_constraint_rejects_invalid_value` or similarly named — it inserts `capabilities = ARRAY['time_travel']` and asserts a constraint violation). Add a new test right after it asserting `image_edit` is now a legal value:

```rust
    #[sqlx::test(migrations = "./migrations")]
    async fn models_capabilities_check_constraint_accepts_image_edit(pool: PgPool) {
        let orgs = crate::repositories::organizations::OrganizationRepository::new(pool.clone());
        let org = orgs.create("test-org").await.expect("create org");
        let profiles =
            crate::repositories::provider_profiles::ProviderProfileRepository::new(pool.clone());
        let profile = profiles
            .create(org.id, "openai", "openai", None)
            .await
            .expect("create profile");

        let result = sqlx::query(
            "INSERT INTO models (organization_id, public_id, provider, provider_profile_id, provider_model_id, capabilities)
             VALUES ($1, 'edit-model', 'openai', $2, 'gpt-image-1', ARRAY['image_edit'])"
        )
        .bind(org.id)
        .bind(profile.id)
        .execute(&pool)
        .await;
        assert!(result.is_ok(), "image_edit should be a legal capability value, got: {:?}", result.err());
    }
```

- [ ] **Step 6: Run test to verify it fails**

Run: `DATABASE_URL="postgres://tmenard@localhost:5432/godwit" cargo test -p godwit-db models_capabilities_check_constraint_accepts_image_edit`
Expected: FAIL — `new row for relation "models" violates check constraint "chk_models_capabilities"`.

- [ ] **Step 7: Write the migration**

Create `crates/godwit-db/migrations/20260803000001_image_edit_capability.up.sql`:

```sql
ALTER TABLE models DROP CONSTRAINT chk_models_capabilities;

ALTER TABLE models ADD CONSTRAINT chk_models_capabilities
    CHECK (capabilities <@ ARRAY['chat','image_generation','image_edit','video_generation','audio_tts','audio_stt','embedding']);
```

Create `crates/godwit-db/migrations/20260803000001_image_edit_capability.down.sql`:

```sql
ALTER TABLE models DROP CONSTRAINT chk_models_capabilities;

ALTER TABLE models ADD CONSTRAINT chk_models_capabilities
    CHECK (capabilities <@ ARRAY['chat','image_generation','video_generation','audio_tts','audio_stt','embedding']);
```

- [ ] **Step 8: Run test to verify it passes**

Run: `DATABASE_URL="postgres://tmenard@localhost:5432/godwit" cargo test -p godwit-db models_capabilities`
Expected: PASS (both the existing rejection test and the new acceptance test).

- [ ] **Step 9: Commit**

```bash
git add crates/godwit-core/src/lib.rs crates/godwit-db/migrations crates/godwit-db/src/lib.rs
git commit -m "feat(core,db): add ImageEdit capability"
```

---

## Task 2: Credential Encryption Module

**Files:**
- Modify: `crates/godwit-auth/Cargo.toml` (add `aes-gcm`, `base64`)
- Create: `crates/godwit-auth/src/credentials.rs`
- Modify: `crates/godwit-auth/src/lib.rs` (add `pub mod credentials;`)

**Interfaces:**
- Produces: `godwit_auth::credentials::{EncryptedSecret, encrypt_api_key, decrypt_api_key, load_master_key_from_env}`.

- [ ] **Step 1: Write the failing test**

Create `crates/godwit-auth/src/credentials.rs` with tests first:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    fn test_key() -> [u8; 32] {
        [7u8; 32]
    }

    #[test]
    fn encrypt_then_decrypt_round_trips() {
        let key = test_key();
        let secret = encrypt_api_key(&key, "sk-real-provider-key");
        let plaintext = decrypt_api_key(&key, &secret).expect("decrypt");
        assert_eq!(plaintext, "sk-real-provider-key");
    }

    #[test]
    fn decrypt_fails_with_wrong_key() {
        let secret = encrypt_api_key(&test_key(), "sk-real-provider-key");
        let wrong_key = [9u8; 32];
        assert!(decrypt_api_key(&wrong_key, &secret).is_err());
    }

    #[test]
    fn decrypt_fails_with_tampered_ciphertext() {
        let key = test_key();
        let mut secret = encrypt_api_key(&key, "sk-real-provider-key");
        secret.ciphertext = "not-valid-base64-ciphertext!!".to_string();
        assert!(decrypt_api_key(&key, &secret).is_err());
    }

    #[test]
    fn load_master_key_from_env_decodes_base64() {
        std::env::set_var("TEST_CREDENTIAL_KEY", base64::Engine::encode(
            &base64::engine::general_purpose::STANDARD,
            [1u8; 32],
        ));
        let key = load_master_key_from_env("TEST_CREDENTIAL_KEY").expect("load key");
        assert_eq!(key, [1u8; 32]);
        std::env::remove_var("TEST_CREDENTIAL_KEY");
    }

    #[test]
    fn load_master_key_from_env_rejects_wrong_length() {
        std::env::set_var(
            "TEST_CREDENTIAL_KEY_SHORT",
            base64::Engine::encode(&base64::engine::general_purpose::STANDARD, [1u8; 16]),
        );
        assert!(load_master_key_from_env("TEST_CREDENTIAL_KEY_SHORT").is_err());
        std::env::remove_var("TEST_CREDENTIAL_KEY_SHORT");
    }
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p godwit-auth credentials`
Expected: FAIL — `unresolved import` / `cannot find function 'encrypt_api_key'` (the module has no implementation yet, only tests).

- [ ] **Step 3: Add dependencies**

Modify `crates/godwit-auth/Cargo.toml`, add to `[dependencies]`:

```toml
aes-gcm = "0.10"
base64 = "0.22"
```

- [ ] **Step 4: Implement the module**

Prepend to `crates/godwit-auth/src/credentials.rs` (above the `#[cfg(test)]` block already written):

```rust
use aes_gcm::aead::{rand_core::RngCore, Aead, KeyInit, OsRng};
use aes_gcm::{Aes256Gcm, Nonce};
use base64::{engine::general_purpose::STANDARD, Engine as _};
use godwit_core::PasteurError;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EncryptedSecret {
    pub ciphertext: String,
    pub nonce: String,
}

pub fn encrypt_api_key(master_key: &[u8; 32], plaintext: &str) -> EncryptedSecret {
    let cipher = Aes256Gcm::new(master_key.into());
    let mut nonce_bytes = [0u8; 12];
    OsRng.fill_bytes(&mut nonce_bytes);
    let nonce = Nonce::from_slice(&nonce_bytes);
    let ciphertext = cipher
        .encrypt(nonce, plaintext.as_bytes())
        .expect("AES-256-GCM encryption with a valid 12-byte nonce cannot fail");
    EncryptedSecret {
        ciphertext: STANDARD.encode(ciphertext),
        nonce: STANDARD.encode(nonce_bytes),
    }
}

pub fn decrypt_api_key(master_key: &[u8; 32], secret: &EncryptedSecret) -> Result<String, PasteurError> {
    let cipher = Aes256Gcm::new(master_key.into());
    let nonce_bytes = STANDARD
        .decode(&secret.nonce)
        .map_err(|e| PasteurError::Auth(format!("invalid credential nonce encoding: {e}")))?;
    let ciphertext = STANDARD
        .decode(&secret.ciphertext)
        .map_err(|e| PasteurError::Auth(format!("invalid credential ciphertext encoding: {e}")))?;
    if nonce_bytes.len() != 12 {
        return Err(PasteurError::Auth("invalid credential nonce length".to_string()));
    }
    let nonce = Nonce::from_slice(&nonce_bytes);
    let plaintext = cipher
        .decrypt(nonce, ciphertext.as_slice())
        .map_err(|_| PasteurError::Auth("failed to decrypt provider credential".to_string()))?;
    String::from_utf8(plaintext).map_err(|e| PasteurError::Auth(e.to_string()))
}

pub fn load_master_key_from_env(var: &str) -> Result<[u8; 32], PasteurError> {
    let encoded = std::env::var(var).map_err(|_| PasteurError::Config(format!("{var} is not set")))?;
    let bytes = STANDARD
        .decode(&encoded)
        .map_err(|e| PasteurError::Config(format!("{var} is not valid base64: {e}")))?;
    bytes
        .try_into()
        .map_err(|v: Vec<u8>| PasteurError::Config(format!("{var} must decode to exactly 32 bytes, got {}", v.len())))
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
pub mod saml;
```

- [ ] **Step 6: Run tests to verify they pass**

Run: `cargo test -p godwit-auth credentials`
Expected: PASS (all 5 tests).

- [ ] **Step 7: Commit**

```bash
git add crates/godwit-auth/Cargo.toml crates/godwit-auth/src/credentials.rs crates/godwit-auth/src/lib.rs Cargo.lock
git commit -m "feat(auth): AES-256-GCM credential encryption"
```

---

## Task 3: Instance-Wide Catalog Migration & Repositories

**Files:**
- Create: `crates/godwit-db/migrations/20260803000002_instance_wide_catalog.up.sql`
- Create: `crates/godwit-db/migrations/20260803000002_instance_wide_catalog.down.sql`
- Modify: `crates/godwit-db/src/models.rs` (`Model`, `ProviderProfile` structs)
- Modify: `crates/godwit-db/src/repositories/models.rs` (whole file)
- Modify: `crates/godwit-db/src/repositories/provider_profiles.rs` (whole file)
- Test: same files (existing `#[sqlx::test]` blocks, rewritten for the new signatures)

**Interfaces:**
- Consumes: `godwit_auth::credentials::EncryptedSecret` (Task 2), for `ProviderProfileRepository::set_auth`.
- Produces:
  - `ModelRepository::{create(public_id, provider, provider_profile_id, provider_model_id, capabilities) -> Model, list() -> Vec<Model>, get_by_public_id(public_id) -> Model, get(id) -> Model, update(id, ...) -> Model, delete(id) -> ()}`
  - `ProviderProfileRepository::{create(name, protocol, base_url, allow_wildcard) -> ProviderProfile, list() -> Vec<ProviderProfile>, get(id) -> ProviderProfile, get_by_name(name) -> ProviderProfile, update(id, base_url, allow_wildcard, enabled) -> ProviderProfile, set_auth(id, &EncryptedSecret) -> ProviderProfile}`

This task intentionally leaves `godwit-api`/`godwit-bin` non-compiling — their call sites (`model_router.rs`, `proxy.rs`, `admin/models.rs`, `main.rs`) are fixed in Tasks 12-13. Run tests scoped to `-p godwit-db` only.

- [ ] **Step 1: Write the failing tests**

Rewrite the test modules in both repository files to match the target (org-free) signatures. In `crates/godwit-db/src/repositories/provider_profiles.rs`, replace the entire `#[cfg(test)] mod tests` block with:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[sqlx::test]
    async fn create_and_list_provider_profiles(pool: PgPool) {
        let repo = ProviderProfileRepository::new(pool);
        let profile = repo
            .create("openai-default", "openai", Some("https://api.openai.com/v1"), false)
            .await
            .expect("create profile");
        assert_eq!(profile.name, "openai-default");
        assert_eq!(profile.protocol, "openai");
        assert_eq!(profile.base_url.as_deref(), Some("https://api.openai.com/v1"));
        assert!(!profile.allow_wildcard);

        let listed = repo.list().await.expect("list profiles");
        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0].id, profile.id);
    }

    #[sqlx::test]
    async fn get_profile_by_id(pool: PgPool) {
        let repo = ProviderProfileRepository::new(pool);
        let profile = repo.create("openai", "openai", None, false).await.expect("create profile");
        let fetched = repo.get(profile.id).await.expect("get profile");
        assert_eq!(fetched.id, profile.id);
    }

    #[sqlx::test]
    async fn get_profile_by_id_not_found(pool: PgPool) {
        let repo = ProviderProfileRepository::new(pool);
        let err = repo.get(uuid::Uuid::nil()).await.unwrap_err();
        assert!(matches!(err, PasteurError::NotFound));
    }

    #[sqlx::test]
    async fn get_profile_by_name(pool: PgPool) {
        let repo = ProviderProfileRepository::new(pool);
        let profile = repo
            .create("azure", "azure_openai", Some("https://azure.example.com"), true)
            .await
            .expect("create profile");
        let fetched = repo.get_by_name("azure").await.expect("get profile by name");
        assert_eq!(fetched.id, profile.id);
        assert_eq!(fetched.protocol, "azure_openai");
        assert!(fetched.allow_wildcard);
    }

    #[sqlx::test]
    async fn get_profile_by_name_not_found(pool: PgPool) {
        let repo = ProviderProfileRepository::new(pool);
        let err = repo.get_by_name("missing").await.unwrap_err();
        assert!(matches!(err, PasteurError::NotFound));
    }

    #[sqlx::test]
    async fn update_profile_fields(pool: PgPool) {
        let repo = ProviderProfileRepository::new(pool);
        let profile = repo.create("openai", "openai", None, false).await.expect("create profile");
        let updated = repo
            .update(profile.id, Some("https://new.example.com"), Some(true), Some(false))
            .await
            .expect("update profile");
        assert_eq!(updated.base_url.as_deref(), Some("https://new.example.com"));
        assert!(updated.allow_wildcard);
        assert!(!updated.enabled);
    }

    #[sqlx::test]
    async fn set_auth_stores_encrypted_secret(pool: PgPool) {
        let repo = ProviderProfileRepository::new(pool);
        let profile = repo.create("openai", "openai", None, false).await.expect("create profile");
        let secret = godwit_auth::credentials::encrypt_api_key(&[3u8; 32], "sk-test");
        let updated = repo.set_auth(profile.id, &secret).await.expect("set auth");
        let stored: godwit_auth::credentials::EncryptedSecret =
            serde_json::from_value(updated.auth.clone()).expect("deserialize stored auth");
        assert_eq!(stored.ciphertext, secret.ciphertext);
        assert_eq!(stored.nonce, secret.nonce);
    }
}
```

In `crates/godwit-db/src/repositories/models.rs`, replace the `#[cfg(test)]` block (if present at the bottom; there may be none yet — add one) with:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::repositories::provider_profiles::ProviderProfileRepository;

    #[sqlx::test]
    async fn create_list_and_get_model(pool: PgPool) {
        let profiles = ProviderProfileRepository::new(pool.clone());
        let profile = profiles.create("openai", "openai", None, false).await.expect("create profile");

        let models = ModelRepository::new(pool);
        let created = models
            .create("gpt-4o", "openai", profile.id, "gpt-4o", "chat")
            .await
            .expect("create model");
        assert_eq!(created.public_id, "gpt-4o");
        assert_eq!(created.provider_profile_id, profile.id);

        let listed = models.list().await.expect("list models");
        assert_eq!(listed.len(), 1);

        let fetched = models.get_by_public_id("gpt-4o").await.expect("get by public id");
        assert_eq!(fetched.id, created.id);
    }

    #[sqlx::test]
    async fn update_and_delete_model(pool: PgPool) {
        let profiles = ProviderProfileRepository::new(pool.clone());
        let profile = profiles.create("openai", "openai", None, false).await.expect("create profile");
        let models = ModelRepository::new(pool);
        let created = models
            .create("gpt-4o", "openai", profile.id, "gpt-4o", "chat")
            .await
            .expect("create model");

        let updated = models
            .update(created.id, Some("gpt-4o-renamed"), Some("chat,embedding"))
            .await
            .expect("update model");
        assert_eq!(updated.public_id, "gpt-4o-renamed");
        assert_eq!(updated.capabilities, vec!["chat".to_string(), "embedding".to_string()]);

        models.delete(created.id).await.expect("delete model");
        let err = models.get_by_public_id("gpt-4o-renamed").await.unwrap_err();
        assert!(matches!(err, PasteurError::NotFound));
    }
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `DATABASE_URL="postgres://tmenard@localhost:5432/godwit" cargo test -p godwit-db repositories:: 2>&1 | head -50`
Expected: FAIL to compile — `create` takes wrong number of arguments, `list` not found, `update`/`delete`/`set_auth` not found, `allow_wildcard` field not found on `ProviderProfile`.

- [ ] **Step 3: Write the migration**

Create `crates/godwit-db/migrations/20260803000002_instance_wide_catalog.up.sql`:

```sql
-- De-duplicate before dropping organization_id: no production tenants exist yet,
-- so we keep the first row per (name) / (provider_profile_id, public_id) and drop the rest.
DELETE FROM models m
USING models m2
WHERE m.provider_profile_id = m2.provider_profile_id
  AND m.public_id = m2.public_id
  AND m.ctid > m2.ctid;

DELETE FROM provider_profiles p
USING provider_profiles p2
WHERE p.name = p2.name
  AND p.ctid > p2.ctid
  AND EXISTS (SELECT 1 FROM provider_profiles p3 WHERE p3.name = p.name AND p3.id <> p.id);

-- Repoint any models whose profile was just deleted at the surviving profile with the same name.
UPDATE models m
SET provider_profile_id = surviving.id
FROM provider_profiles surviving
WHERE NOT EXISTS (SELECT 1 FROM provider_profiles pp WHERE pp.id = m.provider_profile_id)
  AND surviving.name = (SELECT name FROM provider_profiles WHERE id = m.provider_profile_id);

ALTER TABLE provider_profiles
    DROP COLUMN organization_id,
    ADD COLUMN allow_wildcard BOOLEAN NOT NULL DEFAULT false;
ALTER TABLE provider_profiles ADD CONSTRAINT provider_profiles_name_key UNIQUE (name);

ALTER TABLE models DROP COLUMN organization_id;
ALTER TABLE models ADD CONSTRAINT models_provider_profile_id_public_id_key UNIQUE (provider_profile_id, public_id);
```

Create `crates/godwit-db/migrations/20260803000002_instance_wide_catalog.down.sql`:

```sql
ALTER TABLE models DROP CONSTRAINT models_provider_profile_id_public_id_key;
ALTER TABLE models ADD COLUMN organization_id UUID REFERENCES organizations(id);

ALTER TABLE provider_profiles DROP CONSTRAINT provider_profiles_name_key;
ALTER TABLE provider_profiles
    DROP COLUMN allow_wildcard,
    ADD COLUMN organization_id UUID REFERENCES organizations(id);

-- Note: organization_id values are not recoverable after the up-migration's
-- de-duplication; this down-migration restores the columns as nullable only.
```

- [ ] **Step 4: Update the `Model` and `ProviderProfile` structs**

In `crates/godwit-db/src/models.rs`, change:

```rust
#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct Model {
    pub id: Uuid,
    pub organization_id: Uuid,
    pub public_id: String,
```

to:

```rust
#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct Model {
    pub id: Uuid,
    pub public_id: String,
```

And:

```rust
#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct ProviderProfile {
    pub id: Uuid,
    pub organization_id: Uuid,
    pub name: String,
    pub protocol: String,
    pub base_url: Option<String>,
    pub auth: serde_json::Value,
    pub config: serde_json::Value,
    pub enabled: bool,
    pub created_at: DateTime<Utc>,
}
```

to:

```rust
#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct ProviderProfile {
    pub id: Uuid,
    pub name: String,
    pub protocol: String,
    pub base_url: Option<String>,
    pub allow_wildcard: bool,
    pub auth: serde_json::Value,
    pub config: serde_json::Value,
    pub enabled: bool,
    pub created_at: DateTime<Utc>,
}
```

- [ ] **Step 5: Rewrite `ProviderProfileRepository`**

Replace the whole non-test portion of `crates/godwit-db/src/repositories/provider_profiles.rs`:

```rust
use crate::models::ProviderProfile;
use godwit_auth::credentials::EncryptedSecret;
use godwit_core::PasteurError;
use sqlx::PgPool;
use uuid::Uuid;

pub struct ProviderProfileRepository {
    pool: PgPool,
}

impl ProviderProfileRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn create(
        &self,
        name: &str,
        protocol: &str,
        base_url: Option<&str>,
        allow_wildcard: bool,
    ) -> Result<ProviderProfile, PasteurError> {
        sqlx::query_as::<_, ProviderProfile>(
            "INSERT INTO provider_profiles (name, protocol, base_url, allow_wildcard) VALUES ($1, $2, $3, $4) RETURNING *"
        )
        .bind(name)
        .bind(protocol)
        .bind(base_url)
        .bind(allow_wildcard)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| PasteurError::Database(e.to_string()))
    }

    pub async fn list(&self) -> Result<Vec<ProviderProfile>, PasteurError> {
        sqlx::query_as::<_, ProviderProfile>("SELECT * FROM provider_profiles ORDER BY name")
            .fetch_all(&self.pool)
            .await
            .map_err(|e| PasteurError::Database(e.to_string()))
    }

    pub async fn get(&self, id: Uuid) -> Result<ProviderProfile, PasteurError> {
        sqlx::query_as::<_, ProviderProfile>("SELECT * FROM provider_profiles WHERE id = $1")
            .bind(id)
            .fetch_one(&self.pool)
            .await
            .map_err(|e| match e {
                sqlx::Error::RowNotFound => PasteurError::NotFound,
                _ => PasteurError::Database(e.to_string()),
            })
    }

    pub async fn get_by_name(&self, name: &str) -> Result<ProviderProfile, PasteurError> {
        sqlx::query_as::<_, ProviderProfile>("SELECT * FROM provider_profiles WHERE name = $1")
            .bind(name)
            .fetch_one(&self.pool)
            .await
            .map_err(|e| match e {
                sqlx::Error::RowNotFound => PasteurError::NotFound,
                _ => PasteurError::Database(e.to_string()),
            })
    }

    pub async fn update(
        &self,
        id: Uuid,
        base_url: Option<&str>,
        allow_wildcard: Option<bool>,
        enabled: Option<bool>,
    ) -> Result<ProviderProfile, PasteurError> {
        let current = self.get(id).await?;
        sqlx::query_as::<_, ProviderProfile>(
            "UPDATE provider_profiles SET base_url = $2, allow_wildcard = $3, enabled = $4 WHERE id = $1 RETURNING *"
        )
        .bind(id)
        .bind(base_url.map(str::to_string).or(current.base_url))
        .bind(allow_wildcard.unwrap_or(current.allow_wildcard))
        .bind(enabled.unwrap_or(current.enabled))
        .fetch_one(&self.pool)
        .await
        .map_err(|e| PasteurError::Database(e.to_string()))
    }

    pub async fn set_auth(&self, id: Uuid, secret: &EncryptedSecret) -> Result<ProviderProfile, PasteurError> {
        let auth = serde_json::to_value(secret).map_err(|e| PasteurError::Validation(e.to_string()))?;
        sqlx::query_as::<_, ProviderProfile>(
            "UPDATE provider_profiles SET auth = $2 WHERE id = $1 RETURNING *"
        )
        .bind(id)
        .bind(auth)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| PasteurError::Database(e.to_string()))
    }
}
```

Add `godwit-auth = { path = "../godwit-auth" }` to `crates/godwit-db/Cargo.toml`'s `[dependencies]` (needed for the `EncryptedSecret` import above).

- [ ] **Step 6: Rewrite `ModelRepository`**

Replace the whole non-test portion of `crates/godwit-db/src/repositories/models.rs`:

```rust
use crate::models::Model;
use godwit_core::PasteurError;
use sqlx::PgPool;
use uuid::Uuid;

pub struct ModelRepository {
    pool: PgPool,
}

fn parse_capabilities(capabilities: &str) -> Vec<String> {
    let mut caps: Vec<String> = capabilities
        .split(',')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();
    if caps.is_empty() {
        caps.push("chat".to_string());
    }
    caps
}

impl ModelRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn create(
        &self,
        public_id: &str,
        provider: &str,
        provider_profile_id: Uuid,
        provider_model_id: &str,
        capabilities: &str,
    ) -> Result<Model, PasteurError> {
        sqlx::query_as::<_, Model>(
            "INSERT INTO models (public_id, provider, provider_profile_id, provider_model_id, capabilities) VALUES ($1, $2, $3, $4, $5) RETURNING *"
        )
        .bind(public_id)
        .bind(provider)
        .bind(provider_profile_id)
        .bind(provider_model_id)
        .bind(parse_capabilities(capabilities))
        .fetch_one(&self.pool)
        .await
        .map_err(|e| PasteurError::Database(e.to_string()))
    }

    pub async fn list(&self) -> Result<Vec<Model>, PasteurError> {
        sqlx::query_as::<_, Model>("SELECT * FROM models ORDER BY public_id")
            .fetch_all(&self.pool)
            .await
            .map_err(|e| PasteurError::Database(e.to_string()))
    }

    pub async fn get(&self, id: Uuid) -> Result<Model, PasteurError> {
        sqlx::query_as::<_, Model>("SELECT * FROM models WHERE id = $1")
            .bind(id)
            .fetch_one(&self.pool)
            .await
            .map_err(|e| match e {
                sqlx::Error::RowNotFound => PasteurError::NotFound,
                _ => PasteurError::Database(e.to_string()),
            })
    }

    pub async fn get_by_public_id(&self, public_id: &str) -> Result<Model, PasteurError> {
        sqlx::query_as::<_, Model>("SELECT * FROM models WHERE public_id = $1")
            .bind(public_id)
            .fetch_one(&self.pool)
            .await
            .map_err(|e| match e {
                sqlx::Error::RowNotFound => PasteurError::NotFound,
                _ => PasteurError::Database(e.to_string()),
            })
    }

    pub async fn get_by_profile_and_public_id(
        &self,
        provider_profile_id: Uuid,
        public_id: &str,
    ) -> Result<Model, PasteurError> {
        sqlx::query_as::<_, Model>(
            "SELECT * FROM models WHERE provider_profile_id = $1 AND public_id = $2",
        )
        .bind(provider_profile_id)
        .bind(public_id)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| match e {
            sqlx::Error::RowNotFound => PasteurError::NotFound,
            _ => PasteurError::Database(e.to_string()),
        })
    }

    pub async fn update(
        &self,
        id: Uuid,
        public_id: Option<&str>,
        capabilities: Option<&str>,
    ) -> Result<Model, PasteurError> {
        let current = self.get(id).await?;
        let new_public_id = public_id.unwrap_or(&current.public_id);
        let new_capabilities = capabilities
            .map(parse_capabilities)
            .unwrap_or(current.capabilities);
        sqlx::query_as::<_, Model>(
            "UPDATE models SET public_id = $2, capabilities = $3 WHERE id = $1 RETURNING *"
        )
        .bind(id)
        .bind(new_public_id)
        .bind(new_capabilities)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| PasteurError::Database(e.to_string()))
    }

    pub async fn delete(&self, id: Uuid) -> Result<(), PasteurError> {
        sqlx::query("DELETE FROM models WHERE id = $1")
            .bind(id)
            .execute(&self.pool)
            .await
            .map_err(|e| PasteurError::Database(e.to_string()))?;
        Ok(())
    }
}
```

- [ ] **Step 7: Run tests to verify they pass**

Run: `DATABASE_URL="postgres://tmenard@localhost:5432/godwit" cargo test -p godwit-db 2>&1 | tail -60`
Expected: PASS for all tests in `crates/godwit-db` (note: `cargo check --workspace` will still fail — `godwit-api`/`godwit-bin` are fixed in Tasks 12-13). Also update any other `#[sqlx::test]` blocks elsewhere in `godwit-db/src/lib.rs` that still call the old `create(organization_id, ...)` signatures (e.g. the migration-check-constraint tests from Task 1) — adjust them to the new argument order (no `organization_id`) so the whole crate compiles and passes.

- [ ] **Step 8: Commit**

```bash
git add crates/godwit-db crates/godwit-auth/Cargo.toml
git commit -m "feat(db): instance-wide provider_profiles and models"
```

---

## Task 4: `ResolvedProfile` & Stateless OpenAI Adapter

**Files:**
- Modify: `crates/godwit-providers/src/adapter.rs` (add `ResolvedProfile`, change trait signatures)
- Modify: `crates/godwit-providers/src/openai.rs` (whole file)

**Interfaces:**
- Produces: `godwit_providers::adapter::ResolvedProfile { base_url: String, api_key: Option<String> }` (custom redacted `Debug`), and the updated `Adapter` trait where every method takes `profile: &ResolvedProfile` instead of `profile: &ProviderProfile`.
- Consumes: nothing new (this task doesn't wire up decryption yet — that's Task 12).

This task changes a trait used by every adapter, so Anthropic (Task 5) and Gemini (Task 6) won't compile again until their own tasks land — same "crate-scoped tests only" caveat as Task 3, but here it's the whole `godwit-providers` crate: run `cargo test -p godwit-providers openai::` specifically, not the whole crate, until Tasks 5-6 land.

- [ ] **Step 1: Write the failing test**

In `crates/godwit-providers/src/openai.rs`, update the test helpers (`dummy_profile`, `dummy_model`) and one representative test to use `ResolvedProfile`:

```rust
    fn dummy_profile() -> crate::adapter::ResolvedProfile {
        crate::adapter::ResolvedProfile {
            base_url: "https://api.openai.com/v1".to_string(),
            api_key: Some("fake-key".to_string()),
        }
    }

    fn dummy_model() -> Model {
        Model {
            id: Uuid::nil(),
            public_id: "gpt-4o".to_string(),
            provider: "openai".to_string(),
            provider_profile_id: Uuid::nil(),
            provider_model_id: "gpt-4o".to_string(),
            capabilities: vec!["chat".to_string()],
            pricing: serde_json::json!({}),
            config: serde_json::json!({}),
            created_at: Utc::now(),
        }
    }
```

Update the `chat_returns_openai_shape` test's client construction to use the new base URL from `dummy_profile()` instead of a client built with `OpenAiAdapter::new(...)`:

```rust
    #[tokio::test]
    async fn chat_returns_openai_shape() {
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

        let client = OpenAiAdapter::new();
        let profile = crate::adapter::ResolvedProfile {
            base_url: server.uri(),
            api_key: Some("fake-key".to_string()),
        };
        let req = ChatCompletionRequest {
            model: "gpt-4o".to_string(),
            messages: vec![ChatMessage { role: "user".to_string(), content: "Hi".to_string() }],
            stream: Some(false),
            temperature: None,
            max_tokens: None,
        };
        let (resp, _usage) = client.chat(&profile, &dummy_model(), req).await.unwrap();
        let ProviderResponse::Chat(completion) = resp else { panic!("expected chat response") };
        assert_eq!(completion.choices[0].message.content, "Hello");
    }
```

Apply the equivalent change (drop the `new(api_key, base_url)` call, build `OpenAiAdapter::new()` with no args, build a `ResolvedProfile` pointing at `server.uri()`) to every other test in this file (`openai_image_generation`, `openai_image_generation_propagates_http_error`, `openai_audio_tts`, `openai_audio_stt`, `openai_embedding`).

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p godwit-providers --lib openai::tests::chat_returns_openai_shape`
Expected: FAIL to compile — `OpenAiAdapter::new()` takes 2 arguments, `ResolvedProfile` not found in `crate::adapter`.

- [ ] **Step 3: Add `ResolvedProfile` and update the trait**

In `crates/godwit-providers/src/adapter.rs`, add above the `Adapter` trait:

```rust
pub struct ResolvedProfile {
    pub base_url: String,
    pub api_key: Option<String>,
}

impl std::fmt::Debug for ResolvedProfile {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ResolvedProfile")
            .field("base_url", &self.base_url)
            .field("api_key", &self.api_key.as_ref().map(|_| "***redacted***"))
            .finish()
    }
}
```

Then, in the same file, replace every `profile: &ProviderProfile` parameter across all seven `Adapter` trait method signatures (`chat`, `chat_stream`, `image_generation`, `video_generation`, `audio_tts`, `audio_stt`, `embedding`) with `profile: &ResolvedProfile`. Remove the now-unused `use godwit_db::models::{Model, ProviderProfile};` import's `ProviderProfile` half — change it to `use godwit_db::models::Model;`.

- [ ] **Step 4: Rewrite the OpenAI adapter to be stateless**

In `crates/godwit-providers/src/openai.rs`, change the struct and constructor:

```rust
pub struct OpenAiProvider {
    client: Client,
}

pub type OpenAiAdapter = OpenAiProvider;

impl OpenAiProvider {
    pub fn new() -> Self {
        let client = Client::builder()
            .pool_max_idle_per_host(20)
            .pool_idle_timeout(std::time::Duration::from_secs(60))
            .timeout(std::time::Duration::from_secs(120))
            .build()
            .expect("build reqwest client");
        Self { client }
    }
}

impl Default for OpenAiProvider {
    fn default() -> Self {
        Self::new()
    }
}
```

Delete the `from_config(config: &ProviderConfig)` constructor and its `use godwit_core::{..., ProviderConfig}` import — `ProviderConfig`/`AppConfig.providers` is retired in Task 21.

Update the `use crate::adapter::{...}` import to bring in `ResolvedProfile` and drop `ProviderProfile`:

```rust
use crate::adapter::{Adapter, ProviderError, ProviderResponse, ResolvedProfile, SseEvent, UsageReport};
```

and change `use godwit_db::models::{Model, ProviderProfile};` to `use godwit_db::models::Model;`.

For every method, change the parameter from `_profile: &ProviderProfile` to `profile: &ResolvedProfile`, and replace every use of `self.base_url` with `profile.base_url` and every `self.api_key` with a conditional header. For example, `chat` becomes:

```rust
    async fn chat(
        &self,
        profile: &ResolvedProfile,
        _model: &Model,
        request: ChatCompletionRequest,
    ) -> Result<(ProviderResponse, UsageReport), ProviderError> {
        let url = format!("{}/chat/completions", profile.base_url);
        let mut req = self.client.post(&url).json(&request);
        if let Some(key) = &profile.api_key {
            req = req.header("Authorization", format!("Bearer {key}"));
        }
        let res = req.send().await.map_err(|e| ProviderError::Http {
            status: 0,
            message: e.to_string(),
        })?;
        if !res.status().is_success() {
            let status = res.status().as_u16();
            let text = res.text().await.unwrap_or_default();
            return Err(ProviderError::Http { status, message: text });
        }
        let body: ChatCompletionResponse = res
            .json()
            .await
            .map_err(|e| ProviderError::Serialization(e.to_string()))?;
        Ok((ProviderResponse::Chat(body), UsageReport::default()))
    }
```

Apply the same three changes (`_profile: &ProviderProfile` → `profile: &ResolvedProfile`; `format!("{}/...", self.base_url)` → `format!("{}/...", profile.base_url)`; unconditional `.header("Authorization", format!("Bearer {}", self.api_key))` → the `if let Some(key) = &profile.api_key { req = req.header(...) }` pattern above) to `chat_stream`, `image_generation`, `audio_tts`, and `audio_stt`. `video_generation` only needs the parameter rename (it already just returns `CapabilityNotSupported` and touches neither field). `embedding` gets the same treatment as `chat`.

- [ ] **Step 5: Run tests to verify they pass**

Run: `cargo test -p godwit-providers --lib openai::`
Expected: PASS (all `openai::tests::*`).

- [ ] **Step 6: Commit**

```bash
git add crates/godwit-providers/src/adapter.rs crates/godwit-providers/src/openai.rs
git commit -m "refactor(providers): stateless adapters via ResolvedProfile"
```

---

## Task 5: Stateless Anthropic Adapter

**Files:**
- Modify: `crates/godwit-providers/src/anthropic.rs` (whole file — same shape of change as Task 4)

**Interfaces:**
- Consumes: `ResolvedProfile` (Task 4).

- [ ] **Step 1: Write the failing test**

Update `dummy_profile()` in the test module the same way as Task 4:

```rust
    fn dummy_profile() -> crate::adapter::ResolvedProfile {
        crate::adapter::ResolvedProfile {
            base_url: "https://api.anthropic.com".to_string(),
            api_key: Some("fake-key".to_string()),
        }
    }
```

Update `dummy_model()` to drop `organization_id`:

```rust
    fn dummy_model() -> Model {
        Model {
            id: Uuid::nil(),
            public_id: "claude-sonnet".to_string(),
            provider: "anthropic".to_string(),
            provider_profile_id: Uuid::nil(),
            provider_model_id: "claude-3-5-sonnet-20241022".to_string(),
            capabilities: vec!["chat".to_string()],
            pricing: serde_json::json!({}),
            config: serde_json::json!({}),
            created_at: Utc::now(),
        }
    }
```

Update the test that constructs the client to use `AnthropicAdapter::new()` (no args) and a `ResolvedProfile` pointing at the mock server's `uri()`, mirroring Task 4 Step 1's pattern exactly.

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p godwit-providers --lib anthropic::`
Expected: FAIL to compile — `AnthropicAdapter::new()` takes 2 arguments; trait impl signature mismatch (`&ProviderProfile` vs the trait's now-`&ResolvedProfile`).

- [ ] **Step 3: Rewrite the adapter to be stateless**

Apply the same struct/constructor change as Task 4 Step 4:

```rust
pub struct AnthropicProvider {
    client: Client,
}

pub type AnthropicAdapter = AnthropicProvider;

impl AnthropicProvider {
    pub fn new() -> Self {
        let client = Client::builder()
            .pool_max_idle_per_host(20)
            .pool_idle_timeout(std::time::Duration::from_secs(60))
            .timeout(std::time::Duration::from_secs(120))
            .build()
            .expect("build reqwest client");
        Self { client }
    }
}

impl Default for AnthropicProvider {
    fn default() -> Self {
        Self::new()
    }
}
```

Delete `from_config`. Update the `use crate::adapter::{...}` line to include `ResolvedProfile` and drop the `ProviderProfile` half of `use godwit_db::models::{Model, ProviderProfile};` → `use godwit_db::models::Model;`.

In the `chat` method (and `chat_stream`, since Anthropic streaming exists per the design's mention of SSE support), change `_profile: &ProviderProfile` to `profile: &ResolvedProfile`, replace `self.base_url` with `profile.base_url`, and replace the Anthropic-specific auth header — Anthropic uses `x-api-key`, not `Authorization: Bearer` — with:

```rust
        let mut req = self.client.post(&url).json(&anthropic_request);
        if let Some(key) = &profile.api_key {
            req = req.header("x-api-key", key).header("anthropic-version", "2023-06-01");
        }
```

(Match this against the adapter's existing header-building code — if it currently sets `anthropic-version` unconditionally elsewhere, keep that line outside the `if let` and only move the `x-api-key` header inside it.)

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test -p godwit-providers --lib anthropic::`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add crates/godwit-providers/src/anthropic.rs
git commit -m "refactor(providers): stateless Anthropic adapter"
```

---

## Task 6: Stateless Gemini Adapter

**Files:**
- Modify: `crates/godwit-providers/src/gemini.rs` (whole file — same shape of change as Tasks 4-5)

**Interfaces:**
- Consumes: `ResolvedProfile` (Task 4).

- [ ] **Step 1: Write the failing test**

Same pattern as Task 5 Step 1: update `dummy_profile()` to return `crate::adapter::ResolvedProfile { base_url: ..., api_key: Some("fake-key".to_string()) }`, drop `organization_id` from `dummy_model()`, and update the client-construction test to use `GeminiAdapter::new()` with no args plus a `ResolvedProfile` pointing at the mock server.

Note: Gemini's real API passes the key as a `?key=` query parameter, not a header — if the existing `chat`/`chat_request_url_includes_model_and_key` test asserts the URL contains the key, keep building the URL from `profile.api_key` (e.g. `format!("{}/models/{}:generateContent?key={}", profile.base_url, model.provider_model_id, profile.api_key.as_deref().unwrap_or_default())`), just sourced from `profile` instead of `self`.

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p godwit-providers --lib gemini::`
Expected: FAIL to compile, same class of error as Task 5 Step 2.

- [ ] **Step 3: Rewrite the adapter to be stateless**

Apply the same struct/constructor change as Tasks 4-5:

```rust
pub struct GeminiProvider {
    client: Client,
}

pub type GeminiAdapter = GeminiProvider;

impl GeminiProvider {
    pub fn new() -> Self {
        let client = Client::builder()
            .pool_max_idle_per_host(20)
            .pool_idle_timeout(std::time::Duration::from_secs(60))
            .timeout(std::time::Duration::from_secs(120))
            .build()
            .expect("build reqwest client");
        Self { client }
    }
}

impl Default for GeminiProvider {
    fn default() -> Self {
        Self::new()
    }
}
```

Delete `from_config`. Update imports the same way as Task 5 Step 3. In `chat`/`chat_stream`, change `_profile: &ProviderProfile` to `profile: &ResolvedProfile` and replace `self.base_url`/`self.api_key` with `profile.base_url`/`profile.api_key` wherever the URL or query string is built.

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test -p godwit-providers --lib gemini::`
Expected: PASS.

- [ ] **Step 5: Run the whole provider crate to confirm Tasks 4-6 are consistent**

Run: `cargo test -p godwit-providers --lib`
Expected: PASS for `openai::`, `anthropic::`, `gemini::`, `registry::`, `streaming::` (the `AdapterRegistry` registration tests in `registry.rs` construct adapters with `OpenAiAdapter::new("", "")` — update those two call sites to `OpenAiAdapter::new()`, since the crate won't compile otherwise).

- [ ] **Step 6: Commit**

```bash
git add crates/godwit-providers/src/gemini.rs crates/godwit-providers/src/registry.rs
git commit -m "refactor(providers): stateless Gemini adapter, fix registry tests"
```

---

## Task 7: New Adapter — vllm

**Files:**
- Create: `crates/godwit-providers/src/vllm.rs`
- Modify: `crates/godwit-providers/src/lib.rs` (add `pub mod vllm;`)

**Interfaces:**
- Consumes: `ResolvedProfile` (Task 4), `Adapter` trait (Task 4).
- Produces: `godwit_providers::vllm::VllmAdapter`.

- [ ] **Step 1: Write the failing test**

Create `crates/godwit-providers/src/vllm.rs`:

```rust
use crate::adapter::{Adapter, ProviderError, ProviderResponse, ResolvedProfile, SseEvent, UsageReport};
use crate::streaming::parse_sse_events;
use async_trait::async_trait;
use futures::stream::{self, BoxStream, StreamExt};
use godwit_core::{
    AudioSttRequest, AudioTtsRequest, Capability, ChatCompletionRequest, ChatCompletionResponse,
    EmbeddingRequest, EmbeddingResponse, ImageGenerationRequest, VideoGenerationRequest,
};
use godwit_db::models::Model;
use reqwest::Client;

pub struct VllmProvider {
    client: Client,
}

pub type VllmAdapter = VllmProvider;

impl VllmProvider {
    pub fn new() -> Self {
        let client = Client::builder()
            .pool_max_idle_per_host(20)
            .pool_idle_timeout(std::time::Duration::from_secs(60))
            .timeout(std::time::Duration::from_secs(120))
            .build()
            .expect("build reqwest client");
        Self { client }
    }
}

impl Default for VllmProvider {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Adapter for VllmProvider {
    fn supported_capabilities(&self) -> Vec<Capability> {
        vec![Capability::Chat, Capability::Embedding]
    }

    async fn chat(
        &self,
        profile: &ResolvedProfile,
        _model: &Model,
        request: ChatCompletionRequest,
    ) -> Result<(ProviderResponse, UsageReport), ProviderError> {
        let url = format!("{}/chat/completions", profile.base_url);
        let mut req = self.client.post(&url).json(&request);
        if let Some(key) = &profile.api_key {
            req = req.header("Authorization", format!("Bearer {key}"));
        }
        let res = req.send().await.map_err(|e| ProviderError::Http { status: 0, message: e.to_string() })?;
        if !res.status().is_success() {
            let status = res.status().as_u16();
            let text = res.text().await.unwrap_or_default();
            return Err(ProviderError::Http { status, message: text });
        }
        let body: ChatCompletionResponse = res
            .json()
            .await
            .map_err(|e| ProviderError::Serialization(e.to_string()))?;
        Ok((ProviderResponse::Chat(body), UsageReport::default()))
    }

    async fn chat_stream(
        &self,
        profile: &ResolvedProfile,
        _model: &Model,
        mut request: ChatCompletionRequest,
    ) -> Result<BoxStream<'static, Result<SseEvent, ProviderError>>, ProviderError> {
        request.stream = Some(true);
        let url = format!("{}/chat/completions", profile.base_url);
        let mut req = self.client.post(&url).json(&request);
        if let Some(key) = &profile.api_key {
            req = req.header("Authorization", format!("Bearer {key}"));
        }
        let res = req.send().await.map_err(|e| ProviderError::Http { status: 0, message: e.to_string() })?;
        if !res.status().is_success() {
            return Err(ProviderError::Http {
                status: res.status().as_u16(),
                message: res.text().await.unwrap_or_default(),
            });
        }
        let byte_stream = res.bytes_stream();
        let event_stream = byte_stream.flat_map(|bytes| {
            let text = bytes.map(|b| String::from_utf8_lossy(&b).to_string()).unwrap_or_default();
            stream::iter(parse_sse_events(&text).into_iter().map(Ok))
        });
        Ok(event_stream.boxed())
    }

    async fn image_generation(
        &self,
        _profile: &ResolvedProfile,
        _model: &Model,
        _request: ImageGenerationRequest,
    ) -> Result<(ProviderResponse, UsageReport), ProviderError> {
        Err(ProviderError::CapabilityNotSupported("image generation is not supported by vllm".to_string()))
    }

    async fn video_generation(
        &self,
        _profile: &ResolvedProfile,
        _model: &Model,
        _request: VideoGenerationRequest,
    ) -> Result<(ProviderResponse, UsageReport), ProviderError> {
        Err(ProviderError::CapabilityNotSupported("video generation is not supported by vllm".to_string()))
    }

    async fn audio_tts(
        &self,
        _profile: &ResolvedProfile,
        _model: &Model,
        _request: AudioTtsRequest,
    ) -> Result<(ProviderResponse, UsageReport), ProviderError> {
        Err(ProviderError::CapabilityNotSupported("audio TTS is not supported by vllm".to_string()))
    }

    async fn audio_stt(
        &self,
        _profile: &ResolvedProfile,
        _model: &Model,
        _request: AudioSttRequest,
        _file_bytes: Vec<u8>,
        _filename: String,
        _content_type: String,
    ) -> Result<(ProviderResponse, UsageReport), ProviderError> {
        Err(ProviderError::CapabilityNotSupported("audio STT is not supported by vllm".to_string()))
    }

    async fn embedding(
        &self,
        profile: &ResolvedProfile,
        _model: &Model,
        request: EmbeddingRequest,
    ) -> Result<(ProviderResponse, UsageReport), ProviderError> {
        let url = format!("{}/embeddings", profile.base_url);
        let mut req = self.client.post(&url).json(&request);
        if let Some(key) = &profile.api_key {
            req = req.header("Authorization", format!("Bearer {key}"));
        }
        let res = req.send().await.map_err(|e| ProviderError::Http { status: 0, message: e.to_string() })?;
        if !res.status().is_success() {
            return Err(ProviderError::Http {
                status: res.status().as_u16(),
                message: res.text().await.unwrap_or_default(),
            });
        }
        let body: EmbeddingResponse = res
            .json()
            .await
            .map_err(|e| ProviderError::Serialization(e.to_string()))?;
        Ok((
            ProviderResponse::Embedding(body.clone()),
            UsageReport { embedding_tokens: Some(body.usage.total_tokens as i64), ..Default::default() },
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use godwit_core::ChatMessage;
    use uuid::Uuid;
    use wiremock::{matchers::*, Mock, MockServer, ResponseTemplate};

    fn dummy_profile(base_url: String) -> ResolvedProfile {
        ResolvedProfile { base_url, api_key: None }
    }

    fn dummy_model() -> Model {
        Model {
            id: Uuid::nil(),
            public_id: "llama-3-70b".to_string(),
            provider: "vllm".to_string(),
            provider_profile_id: Uuid::nil(),
            provider_model_id: "meta-llama/Llama-3-70B-Instruct".to_string(),
            capabilities: vec!["chat".to_string()],
            pricing: serde_json::json!({}),
            config: serde_json::json!({}),
            created_at: Utc::now(),
        }
    }

    #[tokio::test]
    async fn chat_returns_openai_shape_without_auth_header() {
        let server = MockServer::start().await;
        let body = serde_json::json!({
            "id": "chatcmpl-1", "object": "chat.completion", "created": 1,
            "model": "meta-llama/Llama-3-70B-Instruct",
            "choices": [{"index":0,"message":{"role":"assistant","content":"Hi there"},"finish_reason":"stop"}],
            "usage": {"prompt_tokens":1,"completion_tokens":1,"total_tokens":2}
        });
        Mock::given(method("POST"))
            .and(path("/chat/completions"))
            .and(wiremock::matchers::header_exists("authorization").not())
            .respond_with(ResponseTemplate::new(200).set_body_json(body))
            .mount(&server)
            .await;

        let client = VllmAdapter::new();
        let profile = dummy_profile(server.uri());
        let req = ChatCompletionRequest {
            model: "llama-3-70b".to_string(),
            messages: vec![ChatMessage { role: "user".to_string(), content: "Hi".to_string() }],
            stream: Some(false),
            temperature: None,
            max_tokens: None,
        };
        let (resp, _usage) = client.chat(&profile, &dummy_model(), req).await.unwrap();
        let ProviderResponse::Chat(completion) = resp else { panic!("expected chat response") };
        assert_eq!(completion.choices[0].message.content, "Hi there");
    }

    #[tokio::test]
    async fn unsupported_capabilities_return_error() {
        let client = VllmAdapter::new();
        let profile = dummy_profile("http://localhost:8000/v1".to_string());
        let err = client
            .image_generation(&profile, &dummy_model(), ImageGenerationRequest {
                model: "llama-3-70b".to_string(),
                prompt: "a cat".to_string(),
                n: None,
                size: None,
                quality: None,
                style: None,
            })
            .await
            .unwrap_err();
        assert!(matches!(err, ProviderError::CapabilityNotSupported(_)));
    }
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p godwit-providers --lib vllm::`
Expected: FAIL — `unresolved module 'vllm'` (not registered in `lib.rs` yet).

- [ ] **Step 3: Register the module**

Modify `crates/godwit-providers/src/lib.rs`, add `pub mod vllm;` alongside the existing `pub mod anthropic; pub mod openai;` lines (keep alphabetical: `anthropic`, `gemini`, `openai`, `vllm`, ... as later tasks add more).

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test -p godwit-providers --lib vllm::`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add crates/godwit-providers/src/vllm.rs crates/godwit-providers/src/lib.rs
git commit -m "feat(providers): vllm adapter (chat, streaming, embedding)"
```

---

## Task 8: New Adapter — sglang

**Files:**
- Create: `crates/godwit-providers/src/sglang.rs`
- Modify: `crates/godwit-providers/src/lib.rs` (add `pub mod sglang;`)

**Interfaces:** identical shape to Task 7.

- [ ] **Step 1: Write the failing test**

Create `crates/godwit-providers/src/sglang.rs` with the exact same content as `crates/godwit-providers/src/vllm.rs` from Task 7, with these renames: `VllmProvider` → `SglangProvider`, `VllmAdapter` → `SglangAdapter`, error messages `"... is not supported by vllm"` → `"... is not supported by sglang"`, test model `provider: "vllm"` → `provider: "sglang"`, and the test server mock stays functionally identical (sglang's OpenAI-compatible server uses the same `/chat/completions`/`/embeddings` paths).

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p godwit-providers --lib sglang::`
Expected: FAIL — `unresolved module 'sglang'`.

- [ ] **Step 3: Register the module**

Modify `crates/godwit-providers/src/lib.rs`, add `pub mod sglang;`.

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test -p godwit-providers --lib sglang::`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add crates/godwit-providers/src/sglang.rs crates/godwit-providers/src/lib.rs
git commit -m "feat(providers): sglang adapter (chat, streaming, embedding)"
```

---

## Task 9: New Adapter — llama.cpp

**Files:**
- Create: `crates/godwit-providers/src/llama_cpp.rs`
- Modify: `crates/godwit-providers/src/lib.rs` (add `pub mod llama_cpp;`)

**Interfaces:** identical shape to Task 7.

- [ ] **Step 1: Write the failing test**

Create `crates/godwit-providers/src/llama_cpp.rs` with the same content as Task 7's `vllm.rs`, renamed: `VllmProvider` → `LlamaCppProvider`, `VllmAdapter` → `LlamaCppAdapter`, error messages → `"... is not supported by llama.cpp"`, test model `provider: "llama_cpp"`, test model id e.g. `"llama-3-8b-instruct.Q4_K_M.gguf"`.

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p godwit-providers --lib llama_cpp::`
Expected: FAIL — `unresolved module 'llama_cpp'`.

- [ ] **Step 3: Register the module**

Modify `crates/godwit-providers/src/lib.rs`, add `pub mod llama_cpp;`.

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test -p godwit-providers --lib llama_cpp::`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add crates/godwit-providers/src/llama_cpp.rs crates/godwit-providers/src/lib.rs
git commit -m "feat(providers): llama.cpp adapter (chat, streaming, embedding)"
```

---

## Task 10: New Adapter — ollama

**Files:**
- Create: `crates/godwit-providers/src/ollama.rs`
- Modify: `crates/godwit-providers/src/lib.rs` (add `pub mod ollama;`)

**Interfaces:** identical shape to Task 7.

- [ ] **Step 1: Write the failing test**

Create `crates/godwit-providers/src/ollama.rs` with the same content as Task 7's `vllm.rs`, renamed: `VllmProvider` → `OllamaProvider`, `VllmAdapter` → `OllamaAdapter`, error messages → `"... is not supported by ollama"`, test model `provider: "ollama"`, test model id e.g. `"llama3:70b"`. Ollama's OpenAI-compatible endpoints are also `/chat/completions` and `/embeddings` relative to `base_url` (typically `http://localhost:11434/v1`), so no path differences from vllm/sglang.

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p godwit-providers --lib ollama::`
Expected: FAIL — `unresolved module 'ollama'`.

- [ ] **Step 3: Register the module**

Modify `crates/godwit-providers/src/lib.rs`, add `pub mod ollama;`.

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test -p godwit-providers --lib ollama::`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add crates/godwit-providers/src/ollama.rs crates/godwit-providers/src/lib.rs
git commit -m "feat(providers): ollama adapter (chat, streaming, embedding)"
```

---

## Task 11: OpenAI `image_edit`

**Files:**
- Modify: `crates/godwit-providers/src/adapter.rs` (add `image_edit` to the `Adapter` trait, add `ImageEdit` request type)
- Modify: `crates/godwit-core/src/lib.rs` (add `ImageEditRequest` DTO)
- Modify: `crates/godwit-providers/src/openai.rs` (implement `image_edit`)
- Modify: `crates/godwit-providers/src/{anthropic,gemini,vllm,sglang,llama_cpp,ollama}.rs` (add `image_edit` returning `CapabilityNotSupported`)

**Interfaces:**
- Produces: `godwit_core::ImageEditRequest { model: String, prompt: String, n: Option<i32>, size: Option<String>, response_format: Option<String> }`, `Adapter::image_edit(&self, profile: &ResolvedProfile, model: &Model, request: ImageEditRequest, image_bytes: Vec<u8>, image_filename: String, mask_bytes: Option<Vec<u8>>) -> Result<(ProviderResponse, UsageReport), ProviderError>`.

- [ ] **Step 1: Write the failing test**

In `crates/godwit-providers/src/openai.rs`, add to the test module:

```rust
    #[tokio::test]
    async fn openai_image_edit() {
        let server = MockServer::start().await;
        let body = serde_json::json!({
            "created": 1,
            "data": [{"url": "https://example.com/edited.png", "b64_json": null, "revised_prompt": null}]
        });
        Mock::given(method("POST"))
            .and(path("/images/edits"))
            .respond_with(ResponseTemplate::new(200).set_body_json(body))
            .mount(&server)
            .await;

        let client = OpenAiAdapter::new();
        let profile = crate::adapter::ResolvedProfile {
            base_url: server.uri(),
            api_key: Some("fake-key".to_string()),
        };
        let req = godwit_core::ImageEditRequest {
            model: "gpt-image-1".to_string(),
            prompt: "add a hat".to_string(),
            n: Some(1),
            size: None,
            response_format: None,
        };
        let (resp, _usage) = client
            .image_edit(&profile, &dummy_model(), req, vec![1, 2, 3], "image.png".to_string(), None)
            .await
            .unwrap();
        let ProviderResponse::Image(image) = resp else { panic!("expected image response") };
        assert_eq!(image.data[0].url.as_deref(), Some("https://example.com/edited.png"));
    }
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p godwit-providers --lib openai::tests::openai_image_edit`
Expected: FAIL to compile — `ImageEditRequest` not found, `image_edit` not a method on `Adapter`.

- [ ] **Step 3: Add the DTO**

In `crates/godwit-core/src/lib.rs`, add near `ImageGenerationRequest`:

```rust
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ImageEditRequest {
    pub model: String,
    pub prompt: String,
    pub n: Option<i32>,
    pub size: Option<String>,
    pub response_format: Option<String>,
}
```

- [ ] **Step 4: Add `image_edit` to the `Adapter` trait**

In `crates/godwit-providers/src/adapter.rs`, add to the trait (after `image_generation`):

```rust
    async fn image_edit(
        &self,
        profile: &ResolvedProfile,
        model: &Model,
        request: godwit_core::ImageEditRequest,
        image_bytes: Vec<u8>,
        image_filename: String,
        mask_bytes: Option<Vec<u8>>,
    ) -> Result<(ProviderResponse, UsageReport), ProviderError>;
```

- [ ] **Step 5: Implement it for OpenAI**

In `crates/godwit-providers/src/openai.rs`, add after `image_generation`:

```rust
    async fn image_edit(
        &self,
        profile: &ResolvedProfile,
        _model: &Model,
        request: godwit_core::ImageEditRequest,
        image_bytes: Vec<u8>,
        image_filename: String,
        mask_bytes: Option<Vec<u8>>,
    ) -> Result<(ProviderResponse, UsageReport), ProviderError> {
        let url = format!("{}/images/edits", profile.base_url);
        let image_part = reqwest::multipart::Part::bytes(image_bytes)
            .file_name(image_filename)
            .mime_str("image/png")
            .map_err(|e| ProviderError::Provider(e.to_string()))?;
        let mut form = reqwest::multipart::Form::new()
            .part("image", image_part)
            .text("model", request.model)
            .text("prompt", request.prompt);
        if let Some(mask) = mask_bytes {
            let mask_part = reqwest::multipart::Part::bytes(mask)
                .file_name("mask.png")
                .mime_str("image/png")
                .map_err(|e| ProviderError::Provider(e.to_string()))?;
            form = form.part("mask", mask_part);
        }
        if let Some(n) = request.n {
            form = form.text("n", n.to_string());
        }
        if let Some(size) = request.size {
            form = form.text("size", size);
        }
        if let Some(response_format) = request.response_format {
            form = form.text("response_format", response_format);
        }
        let mut req = self.client.post(&url).multipart(form);
        if let Some(key) = &profile.api_key {
            req = req.header("Authorization", format!("Bearer {key}"));
        }
        let res = req.send().await.map_err(|e| ProviderError::Http { status: 0, message: e.to_string() })?;
        if !res.status().is_success() {
            return Err(ProviderError::Http {
                status: res.status().as_u16(),
                message: res.text().await.unwrap_or_default(),
            });
        }
        let body: godwit_core::ImageGenerationResponse = res
            .json()
            .await
            .map_err(|e| ProviderError::Serialization(e.to_string()))?;
        Ok((ProviderResponse::Image(body), UsageReport::default()))
    }
```

- [ ] **Step 6: Add `CapabilityNotSupported` stubs everywhere else**

Add this method to `crates/godwit-providers/src/anthropic.rs`, `gemini.rs`, `vllm.rs`, `sglang.rs`, `llama_cpp.rs`, `ollama.rs` (adjust the message's backend name per file):

```rust
    async fn image_edit(
        &self,
        _profile: &ResolvedProfile,
        _model: &Model,
        _request: godwit_core::ImageEditRequest,
        _image_bytes: Vec<u8>,
        _image_filename: String,
        _mask_bytes: Option<Vec<u8>>,
    ) -> Result<(ProviderResponse, UsageReport), ProviderError> {
        Err(ProviderError::CapabilityNotSupported("image edit is not supported by <backend>".to_string()))
    }
```

- [ ] **Step 7: Run tests to verify they pass**

Run: `cargo test -p godwit-providers --lib`
Expected: PASS across every adapter module (this is the first point since Task 6 where the whole `godwit-providers` crate compiles and passes as a whole).

- [ ] **Step 8: Commit**

```bash
git add crates/godwit-core/src/lib.rs crates/godwit-providers
git commit -m "feat(providers): OpenAI image_edit, CapabilityNotSupported elsewhere"
```

---

## Task 12: `DbModelRouter` Refactor

**Files:**
- Modify: `crates/godwit-api/src/model_router.rs` (whole file)

**Interfaces:**
- Consumes: `ProviderProfileRepository`/`ModelRepository` (Task 3), `godwit_auth::credentials::{decrypt_api_key, EncryptedSecret}` (Task 2), `ResolvedProfile` (Task 4).
- Produces: `DbModelRouter::new(pool, registry, master_key: [u8; 32])`, `DbModelRouter::resolve(&self, model_ref: &str, requested_capability: Capability) -> Result<ResolvedModel, PasteurError>`, `ResolvedModel { model: Model, profile: ProviderProfile, resolved_credentials: ResolvedProfile, adapter: Arc<dyn Adapter> }`.

This task still leaves `proxy.rs`/`admin/models.rs`/`main.rs` broken (they call the old `resolve(organization_id, model_ref)` signature and old repository methods) — fixed in Task 13. Test scoped to `-p godwit-api --lib model_router::`.

- [ ] **Step 1: Write the failing tests**

Replace the entire `#[cfg(test)] mod tests` block in `crates/godwit-api/src/model_router.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use godwit_auth::credentials::encrypt_api_key;
    use godwit_db::repositories::{models::ModelRepository, provider_profiles::ProviderProfileRepository};
    use godwit_providers::openai::OpenAiAdapter;
    use sqlx::PgPool;

    const TEST_KEY: [u8; 32] = [5u8; 32];

    fn test_registry() -> Arc<AdapterRegistry> {
        let mut registry = AdapterRegistry::new();
        registry.register(Protocol::openai(), Arc::new(OpenAiAdapter::new()));
        Arc::new(registry)
    }

    #[sqlx::test(migrations = "../godwit-db/migrations")]
    async fn bare_public_id_resolves_when_unique(pool: PgPool) {
        let profiles = ProviderProfileRepository::new(pool.clone());
        let profile = profiles.create("default", "openai", None, false).await.expect("create profile");

        let models = ModelRepository::new(pool.clone());
        let model = models
            .create("gpt-4o", "openai", profile.id, "gpt-4o", "chat")
            .await
            .expect("create model");

        let router = DbModelRouter::new(pool, test_registry(), TEST_KEY);
        let resolved = router.resolve("gpt-4o", Capability::Chat).await.expect("resolve");
        assert_eq!(resolved.model.id, model.id);
        assert_eq!(resolved.profile.id, profile.id);
    }

    #[sqlx::test(migrations = "../godwit-db/migrations")]
    async fn bare_public_id_ambiguous_when_duplicated(pool: PgPool) {
        let profiles = ProviderProfileRepository::new(pool.clone());
        let profile_a = profiles.create("openai", "openai", None, false).await.expect("create profile a");
        let profile_b = profiles.create("azure", "openai", None, false).await.expect("create profile b");

        let models = ModelRepository::new(pool.clone());
        models.create("gpt-4o", "openai", profile_a.id, "gpt-4o", "chat").await.expect("create model a");
        models.create("gpt-4o", "openai", profile_b.id, "gpt-4o", "chat").await.expect("create model b");

        let router = DbModelRouter::new(pool, test_registry(), TEST_KEY);
        let err = router.resolve("gpt-4o", Capability::Chat).await.unwrap_err();
        assert!(matches!(err, PasteurError::Validation(_)));
    }

    #[sqlx::test(migrations = "../godwit-db/migrations")]
    async fn profile_prefix_selects_correct_model(pool: PgPool) {
        let profiles = ProviderProfileRepository::new(pool.clone());
        let profile_a = profiles.create("openai", "openai", None, false).await.expect("create profile a");
        let profile_b = profiles.create("azure", "openai", None, false).await.expect("create profile b");

        let models = ModelRepository::new(pool.clone());
        models.create("gpt-4o", "openai", profile_a.id, "gpt-4o", "chat").await.expect("create model a");
        let model_b = models.create("gpt-4o", "openai", profile_b.id, "gpt-4o", "chat").await.expect("create model b");

        let router = DbModelRouter::new(pool, test_registry(), TEST_KEY);
        let resolved = router.resolve("azure/gpt-4o", Capability::Chat).await.expect("resolve");
        assert_eq!(resolved.model.id, model_b.id);
        assert_eq!(resolved.profile.id, profile_b.id);
    }

    #[sqlx::test(migrations = "../godwit-db/migrations")]
    async fn unknown_public_id_returns_not_found(pool: PgPool) {
        let router = DbModelRouter::new(pool, test_registry(), TEST_KEY);
        let err = router.resolve("unknown-model", Capability::Chat).await.unwrap_err();
        assert!(matches!(err, PasteurError::NotFound));
    }

    #[sqlx::test(migrations = "../godwit-db/migrations")]
    async fn unknown_profile_prefix_returns_not_found(pool: PgPool) {
        let profiles = ProviderProfileRepository::new(pool.clone());
        let profile = profiles.create("openai", "openai", None, false).await.expect("create profile");
        let models = ModelRepository::new(pool.clone());
        models.create("gpt-4o", "openai", profile.id, "gpt-4o", "chat").await.expect("create model");

        let router = DbModelRouter::new(pool, test_registry(), TEST_KEY);
        let err = router.resolve("missing/gpt-4o", Capability::Chat).await.unwrap_err();
        assert!(matches!(err, PasteurError::NotFound));
    }

    #[sqlx::test(migrations = "../godwit-db/migrations")]
    async fn wildcard_profile_synthesizes_model_when_catalog_misses(pool: PgPool) {
        let profiles = ProviderProfileRepository::new(pool.clone());
        let profile = profiles.create("openai", "openai", None, true).await.expect("create wildcard profile");

        let router = DbModelRouter::new(pool, test_registry(), TEST_KEY);
        let resolved = router.resolve("openai/gpt-4o-mini-anything", Capability::Chat).await.expect("resolve");
        assert_eq!(resolved.model.public_id, "openai/gpt-4o-mini-anything");
        assert_eq!(resolved.model.provider_model_id, "gpt-4o-mini-anything");
        assert!(resolved.model.has_capability(Capability::Chat));
    }

    #[sqlx::test(migrations = "../godwit-db/migrations")]
    async fn non_wildcard_profile_rejects_unknown_suffix(pool: PgPool) {
        let profiles = ProviderProfileRepository::new(pool.clone());
        profiles.create("openai", "openai", None, false).await.expect("create profile");

        let router = DbModelRouter::new(pool, test_registry(), TEST_KEY);
        let err = router.resolve("openai/anything", Capability::Chat).await.unwrap_err();
        assert!(matches!(err, PasteurError::NotFound));
    }

    #[sqlx::test(migrations = "../godwit-db/migrations")]
    async fn resolves_decrypted_credentials(pool: PgPool) {
        let profiles = ProviderProfileRepository::new(pool.clone());
        let profile = profiles.create("openai", "openai", Some("https://api.openai.com/v1"), true).await.expect("create profile");
        let secret = encrypt_api_key(&TEST_KEY, "sk-real-key");
        profiles.set_auth(profile.id, &secret).await.expect("set auth");

        let router = DbModelRouter::new(pool, test_registry(), TEST_KEY);
        let resolved = router.resolve("openai/gpt-4o", Capability::Chat).await.expect("resolve");
        assert_eq!(resolved.resolved_credentials.base_url, "https://api.openai.com/v1");
        assert_eq!(resolved.resolved_credentials.api_key.as_deref(), Some("sk-real-key"));
    }

    #[sqlx::test(migrations = "../godwit-db/migrations")]
    async fn resolve_errors_when_profile_has_no_credentials(pool: PgPool) {
        let profiles = ProviderProfileRepository::new(pool.clone());
        profiles.create("openai", "openai", Some("https://api.openai.com/v1"), true).await.expect("create profile");

        let router = DbModelRouter::new(pool, test_registry(), TEST_KEY);
        let err = router.resolve("openai/gpt-4o", Capability::Chat).await.unwrap_err();
        assert!(matches!(err, PasteurError::Provider(_)));
    }
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `DATABASE_URL="postgres://tmenard@localhost:5432/godwit" cargo test -p godwit-api --lib model_router:: 2>&1 | head -60`
Expected: FAIL to compile — `resolve` takes 2 arguments not matching (`organization_id: Uuid, model_ref: &str` vs `model_ref, Capability`), `DbModelRouter::new` takes 2 arguments not 3, `set_auth`/`resolved_credentials` not found.

- [ ] **Step 3: Rewrite `model_router.rs`**

Replace the whole non-test portion of `crates/godwit-api/src/model_router.rs`:

```rust
use godwit_auth::credentials::{decrypt_api_key, EncryptedSecret};
use godwit_core::{Capability, PasteurError, Protocol};
use godwit_db::models::{Model, ProviderProfile};
use godwit_db::repositories::{models::ModelRepository, provider_profiles::ProviderProfileRepository};
use godwit_providers::adapter::ResolvedProfile;
use godwit_providers::{Adapter, AdapterRegistry};
use sqlx::PgPool;
use std::sync::Arc;
use uuid::Uuid;

pub struct ResolvedModel {
    pub model: Model,
    pub profile: ProviderProfile,
    pub resolved_credentials: ResolvedProfile,
    pub adapter: Arc<dyn Adapter>,
}

impl std::fmt::Debug for ResolvedModel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ResolvedModel")
            .field("model", &self.model)
            .field("profile", &self.profile)
            .field("resolved_credentials", &self.resolved_credentials)
            .field("adapter", &"<dyn Adapter>")
            .finish()
    }
}

pub struct DbModelRouter {
    pool: PgPool,
    registry: Arc<AdapterRegistry>,
    master_key: [u8; 32],
}

impl DbModelRouter {
    pub fn new(pool: PgPool, registry: Arc<AdapterRegistry>, master_key: [u8; 32]) -> Self {
        Self { pool, registry, master_key }
    }

    fn resolve_credentials(&self, profile: &ProviderProfile) -> Result<ResolvedProfile, PasteurError> {
        let base_url = profile
            .base_url
            .clone()
            .ok_or_else(|| PasteurError::Provider(format!("provider profile '{}' has no base_url configured", profile.name)))?;
        if profile.auth.is_null() || profile.auth == serde_json::json!({}) {
            return Err(PasteurError::Provider(format!(
                "no credentials configured for protocol {}",
                profile.protocol
            )));
        }
        let secret: EncryptedSecret = serde_json::from_value(profile.auth.clone())
            .map_err(|e| PasteurError::Provider(format!("malformed stored credentials: {e}")))?;
        let api_key = decrypt_api_key(&self.master_key, &secret)?;
        Ok(ResolvedProfile { base_url, api_key: Some(api_key) })
    }

    pub async fn resolve(&self, model_ref: &str, requested_capability: Capability) -> Result<ResolvedModel, PasteurError> {
        let (profile_name, suffix) = if let Some((name, rest)) = model_ref.split_once('/') {
            (Some(name), rest)
        } else {
            (None, model_ref)
        };

        let model_repo = ModelRepository::new(self.pool.clone());
        let profile_repo = ProviderProfileRepository::new(self.pool.clone());

        let (model, profile) = if let Some(name) = profile_name {
            let profile = profile_repo.get_by_name(name).await?;
            match model_repo.get_by_profile_and_public_id(profile.id, suffix).await {
                Ok(model) => (model, profile),
                Err(PasteurError::NotFound) if profile.allow_wildcard => {
                    let model = Model {
                        id: Uuid::nil(),
                        public_id: model_ref.to_string(),
                        provider: profile.protocol.clone(),
                        provider_profile_id: profile.id,
                        provider_model_id: suffix.to_string(),
                        capabilities: vec![requested_capability.as_str().to_string()],
                        pricing: serde_json::json!({}),
                        config: serde_json::json!({}),
                        created_at: profile.created_at,
                    };
                    (model, profile)
                }
                Err(e) => return Err(e),
            }
        } else {
            let models = model_repo.list().await?;
            let candidates: Vec<Model> = models.into_iter().filter(|m| m.public_id == suffix).collect();
            match candidates.len() {
                0 => return Err(PasteurError::NotFound),
                1 => {
                    let model = candidates.into_iter().next().unwrap();
                    let profile = profile_repo.get(model.provider_profile_id).await?;
                    (model, profile)
                }
                _ => {
                    return Err(PasteurError::Validation(format!(
                        "ambiguous model '{suffix}'; use 'profile_name/{suffix}'"
                    )))
                }
            }
        };

        if !model.has_capability(requested_capability) {
            return Err(PasteurError::Validation(format!(
                "model {} does not support {}",
                model.public_id,
                requested_capability.as_str()
            )));
        }

        let resolved_credentials = self.resolve_credentials(&profile)?;
        let protocol = Protocol(profile.protocol.clone());
        let adapter = self
            .registry
            .get(&protocol)
            .ok_or_else(|| PasteurError::Provider(format!("unknown protocol: {}", profile.protocol)))?;

        Ok(ResolvedModel { model, profile, resolved_credentials, adapter })
    }
}
```

Note the added `has_capability` check right before credential resolution — this replaces the check that used to live in `proxy.rs` (`resolved.model.has_capability(Capability::Chat)`), since `resolve` now takes the requested capability directly and can validate it once for every route, not just chat. Task 13 removes the now-redundant check from `proxy.rs`.

- [ ] **Step 4: Run tests to verify they pass**

Run: `DATABASE_URL="postgres://tmenard@localhost:5432/godwit" cargo test -p godwit-api --lib model_router::`
Expected: PASS (all 9 tests). `cargo check --workspace` is still red until Task 13.

- [ ] **Step 5: Commit**

```bash
git add crates/godwit-api/src/model_router.rs
git commit -m "feat(api): instance-wide model resolution with wildcard passthrough"
```

---

## Task 13: Restore Compilation — `proxy.rs`, `admin/models.rs`, `main.rs`

**Files:**
- Modify: `crates/godwit-api/src/proxy.rs` (`chat_completions`, `list_models`)
- Modify: `crates/godwit-api/src/admin/models.rs` (`list_models`, RBAC check)
- Modify: `crates/godwit-api/src/admin/spend.rs` (`compute_cost` call site — still passes `Capability::Chat` explicitly, no change needed there, but verify)
- Modify: `crates/godwit-bin/src/main.rs` (registry + router construction, master key loading)
- Modify: `crates/godwit-api/src/state.rs` (drop the now-org-keyed `model_cache`)

**Interfaces:**
- Consumes: `DbModelRouter::new(pool, registry, master_key)` and `resolve(model_ref, capability)` (Task 12), stateless adapter constructors `XxxAdapter::new()` (Tasks 4-10), `godwit_auth::credentials::load_master_key_from_env` (Task 2).

This task restores `cargo check --workspace` to green — it's the integration point for everything so far.

- [ ] **Step 1: Fix `proxy.rs`**

In `crates/godwit-api/src/proxy.rs`, change the `resolve` call and drop the now-redundant capability check:

```rust
    let resolved = state
        .model_router
        .resolve(&req.model, Capability::Chat)
        .await?;

    let streamed = req.stream == Some(true);
    let (result, usage) = if streamed {
        let stream = resolved
            .adapter
            .chat_stream(&resolved.resolved_credentials, &resolved.model, req)
            .await
```

(Remove the preceding `if !resolved.model.has_capability(Capability::Chat) { ... }` block entirely — `resolve` now does this check.) Apply the same `&resolved.resolved_credentials` swap (replacing `&resolved.profile`) to the non-streaming `resolved.adapter.chat(...)` call further down.

Change `list_models` to drop the organization filter, since `models` is no longer org-scoped:

```rust
async fn list_models(
    State(state): State<Arc<AppState>>,
    Extension(_api_key): Extension<ApiKey>,
) -> Result<impl IntoResponse, crate::error::ApiError> {
    let repo = ModelRepository::new(state.pool.clone());
    let models = repo.list().await.map_err(crate::error::ApiError::Core)?;
    Ok((StatusCode::OK, Json(models_response(&models))))
}
```

(Drop the `model_cache` read/write block entirely — Step 5 below removes the cache field itself, since caching a global, rarely-changing catalog per-org no longer makes sense and the model list is now small enough to fetch directly. If a cache is wanted later, it can key by nothing (a single cached `Vec<Model>` with a short TTL) — out of scope here.)

- [ ] **Step 2: Fix `admin/models.rs`**

In `crates/godwit-api/src/admin/models.rs`, change the RBAC check and repository call:

```rust
async fn list_models(
    State(state): State<Arc<AppState>>,
    Extension(claims): Extension<Claims>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let role = Role::from_str(&claims.role).ok_or(ApiError::Forbidden)?;
    if role != Role::SuperAdmin {
        return Err(ApiError::Forbidden);
    }
    let repo = ModelRepository::new(state.pool.clone());
    let models = repo.list().await.map_err(ApiError::Core)?;
    Ok(Json(serde_json::json!({ "data": models })))
}
```

- [ ] **Step 3: Drop the org-keyed model cache**

In `crates/godwit-api/src/state.rs`, remove the `model_cache` field:

```rust
pub struct AppState {
    pub config: AppConfig,
    pub pool: PgPool,
    pub adapter_registry: Arc<AdapterRegistry>,
    pub model_router: DbModelRouter,
    pub user_repo: UserRepository,
    pub org_repo: OrganizationRepository,
    pub api_key_repo: ApiKeyRepository,
    pub api_key_cache: MemoryCache<String, ApiKey>,
}
```

Drop the now-unused `use godwit_db::models::{ApiKey, Model};` → `use godwit_db::models::ApiKey;`.

- [ ] **Step 4: Fix `main.rs`**

In `crates/godwit-bin/src/main.rs`, replace the registry construction and `AppState` assembly:

```rust
    let master_key = godwit_auth::credentials::load_master_key_from_env("CREDENTIAL_ENCRYPTION_KEY")?;

    let mut registry = AdapterRegistry::new();
    registry.register(Protocol::openai(), Arc::new(OpenAiAdapter::new()));
    registry.register(Protocol::anthropic(), Arc::new(AnthropicAdapter::new()));
    registry.register(Protocol::gemini(), Arc::new(GeminiAdapter::new()));
    registry.register(Protocol::vllm(), Arc::new(VllmAdapter::new()));
    registry.register(Protocol::sglang(), Arc::new(SglangAdapter::new()));
    registry.register(Protocol::llama_cpp(), Arc::new(LlamaCppAdapter::new()));
    registry.register(Protocol::ollama(), Arc::new(OllamaAdapter::new()));

    let adapter_registry = Arc::new(registry);
    let state = Arc::new(AppState {
        config: config.clone(),
        pool: pool.clone(),
        adapter_registry: adapter_registry.clone(),
        model_router: DbModelRouter::new(pool.clone(), adapter_registry, master_key),
        user_repo: UserRepository::new(pool.clone()),
        org_repo: OrganizationRepository::new(pool.clone()),
        api_key_repo: ApiKeyRepository::new(pool.clone()),
        api_key_cache: MemoryCache::new(),
    });
```

Update the `use godwit_providers::{...}` import to bring in the four new adapters:

```rust
use godwit_providers::{
    anthropic::AnthropicAdapter, gemini::GeminiAdapter, llama_cpp::LlamaCppAdapter,
    ollama::OllamaAdapter, openai::OpenAiAdapter, sglang::SglangAdapter, vllm::VllmAdapter,
    AdapterRegistry,
};
```

`anyhow::Error` already implements `From<PasteurError>`? Check: `load_master_key_from_env` returns `Result<[u8; 32], PasteurError>` and `main` returns `anyhow::Result<()>` — since `PasteurError` derives `thiserror::Error`, `?` on a `Result<_, PasteurError>` inside an `anyhow::Result` function works via `anyhow`'s blanket `From<E: std::error::Error>` impl, exactly like the existing `run_migrations(&pool).await?` call two lines above it.

- [ ] **Step 5: Run the full workspace check**

Run: `cargo check --workspace 2>&1 | tail -100`
Expected: PASS with no errors (warnings about unused `AppConfig.providers` are expected and resolved in Task 21).

- [ ] **Step 6: Run the full workspace test suite**

Run: `DATABASE_URL="postgres://tmenard@localhost:5432/godwit" cargo test --workspace 2>&1 | tail -150`
Expected: PASS. Fix any remaining call sites the compiler flags that this plan didn't anticipate (e.g. other admin modules referencing `Model`'s removed `organization_id` field) before moving on.

- [ ] **Step 7: Commit**

```bash
git add crates/godwit-api crates/godwit-bin
git commit -m "fix(api,bin): restore compilation for instance-wide catalog"
```

---

## Task 14: Admin API — `provider-profiles` CRUD

**Files:**
- Create: `crates/godwit-api/src/admin/provider_profiles.rs`
- Modify: `crates/godwit-api/src/admin/mod.rs` (register the new router)

**Interfaces:**
- Consumes: `ProviderProfileRepository` (Task 3), `godwit_auth::credentials::encrypt_api_key` (Task 2), `AppState.model_router`'s master key isn't directly exposed — add `pub credential_master_key: [u8; 32]` to `AppState` in this task (needed here and nowhere else yet, since `model_router` owns its own copy privately).
- Produces: `GET/POST /api/v1/provider-profiles`, `PATCH /api/v1/provider-profiles/{id}`.

- [ ] **Step 1: Write the failing test**

Create `crates/godwit-api/src/admin/provider_profiles.rs`:

```rust
use axum::{
    extract::{Extension, Path, State},
    routing::{get, patch, post},
    Json, Router,
};
use godwit_auth::{credentials::encrypt_api_key, jwt::Claims, rbac::Role};
use godwit_db::repositories::provider_profiles::ProviderProfileRepository;
use serde::Deserialize;
use std::sync::Arc;
use uuid::Uuid;

use crate::{error::ApiError, state::AppState};

pub fn router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/provider-profiles", get(list_profiles).post(create_profile))
        .route("/provider-profiles/:id", patch(update_profile))
}

fn require_super_admin(claims: &Claims) -> Result<(), ApiError> {
    let role = Role::from_str(&claims.role).ok_or(ApiError::Forbidden)?;
    if role != Role::SuperAdmin {
        return Err(ApiError::Forbidden);
    }
    Ok(())
}

fn profile_json(profile: &godwit_db::models::ProviderProfile) -> serde_json::Value {
    serde_json::json!({
        "id": profile.id,
        "name": profile.name,
        "protocol": profile.protocol,
        "base_url": profile.base_url,
        "allow_wildcard": profile.allow_wildcard,
        "enabled": profile.enabled,
        "has_credentials": !profile.auth.is_null() && profile.auth != serde_json::json!({}),
        "created_at": profile.created_at,
    })
}

async fn list_profiles(
    State(state): State<Arc<AppState>>,
    Extension(claims): Extension<Claims>,
) -> Result<Json<serde_json::Value>, ApiError> {
    require_super_admin(&claims)?;
    let repo = ProviderProfileRepository::new(state.pool.clone());
    let profiles = repo.list().await.map_err(ApiError::Core)?;
    Ok(Json(serde_json::json!({ "data": profiles.iter().map(profile_json).collect::<Vec<_>>() })))
}

#[derive(Debug, Deserialize)]
pub struct CreateProfileRequest {
    pub name: String,
    pub protocol: String,
    pub base_url: Option<String>,
    pub api_key: Option<String>,
    #[serde(default)]
    pub allow_wildcard: bool,
}

async fn create_profile(
    State(state): State<Arc<AppState>>,
    Extension(claims): Extension<Claims>,
    Json(req): Json<CreateProfileRequest>,
) -> Result<Json<serde_json::Value>, ApiError> {
    require_super_admin(&claims)?;
    let repo = ProviderProfileRepository::new(state.pool.clone());
    let profile = repo
        .create(&req.name, &req.protocol, req.base_url.as_deref(), req.allow_wildcard)
        .await
        .map_err(ApiError::Core)?;
    let profile = if let Some(api_key) = req.api_key {
        let secret = encrypt_api_key(&state.credential_master_key, &api_key);
        repo.set_auth(profile.id, &secret).await.map_err(ApiError::Core)?
    } else {
        profile
    };
    Ok(Json(profile_json(&profile)))
}

#[derive(Debug, Deserialize)]
pub struct UpdateProfileRequest {
    pub base_url: Option<String>,
    pub api_key: Option<String>,
    pub allow_wildcard: Option<bool>,
    pub enabled: Option<bool>,
}

async fn update_profile(
    State(state): State<Arc<AppState>>,
    Extension(claims): Extension<Claims>,
    Path(id): Path<Uuid>,
    Json(req): Json<UpdateProfileRequest>,
) -> Result<Json<serde_json::Value>, ApiError> {
    require_super_admin(&claims)?;
    let repo = ProviderProfileRepository::new(state.pool.clone());
    let profile = repo
        .update(id, req.base_url.as_deref(), req.allow_wildcard, req.enabled)
        .await
        .map_err(ApiError::Core)?;
    let profile = if let Some(api_key) = req.api_key {
        let secret = encrypt_api_key(&state.credential_master_key, &api_key);
        repo.set_auth(profile.id, &secret).await.map_err(ApiError::Core)?
    } else {
        profile
    };
    Ok(Json(profile_json(&profile)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn profile_json_never_includes_raw_auth() {
        let profile = godwit_db::models::ProviderProfile {
            id: Uuid::nil(),
            name: "openai".to_string(),
            protocol: "openai".to_string(),
            base_url: Some("https://api.openai.com/v1".to_string()),
            allow_wildcard: false,
            auth: serde_json::json!({"ciphertext": "abc", "nonce": "def"}),
            config: serde_json::json!({}),
            enabled: true,
            created_at: chrono::Utc::now(),
        };
        let json = profile_json(&profile);
        assert_eq!(json["has_credentials"], true);
        assert!(json.get("auth").is_none());
        assert!(json.get("ciphertext").is_none());
    }
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p godwit-api --lib admin::provider_profiles::`
Expected: FAIL to compile — `state.credential_master_key` doesn't exist on `AppState` yet.

- [ ] **Step 3: Add the master key to `AppState`**

In `crates/godwit-api/src/state.rs`, add the field:

```rust
pub struct AppState {
    pub config: AppConfig,
    pub pool: PgPool,
    pub adapter_registry: Arc<AdapterRegistry>,
    pub model_router: DbModelRouter,
    pub user_repo: UserRepository,
    pub org_repo: OrganizationRepository,
    pub api_key_repo: ApiKeyRepository,
    pub api_key_cache: MemoryCache<String, ApiKey>,
    pub credential_master_key: [u8; 32],
}
```

In `crates/godwit-bin/src/main.rs`, pass it through when building `AppState` (the `master_key` local variable already exists from Task 13 Step 4 — `DbModelRouter::new` consumes it by value, so compute it once and clone/copy it into both places since `[u8; 32]` is `Copy`):

```rust
        model_router: DbModelRouter::new(pool.clone(), adapter_registry, master_key),
        user_repo: UserRepository::new(pool.clone()),
        org_repo: OrganizationRepository::new(pool.clone()),
        api_key_repo: ApiKeyRepository::new(pool.clone()),
        api_key_cache: MemoryCache::new(),
        credential_master_key: master_key,
```

- [ ] **Step 4: Register the router**

In `crates/godwit-api/src/admin/mod.rs`, add the module and nest it:

```rust
pub mod api_keys;
pub mod auth;
pub mod models;
pub mod organizations;
pub mod provider_profiles;
pub mod spend;
pub mod teams;
pub mod users;

use crate::{middleware::jwt_auth, state::AppState};
use axum::{middleware, Router};
use std::sync::Arc;

pub fn router(state: Arc<AppState>) -> Router<Arc<AppState>> {
    let protected = Router::new()
        .nest("/users", users::router())
        .nest("/organizations", organizations::router())
        .nest("/teams", teams::router())
        .nest("/api-keys", api_keys::router())
        .nest("/models", models::router())
        .nest("/", provider_profiles::router())
        .nest("/spend", spend::router())
        .route_layer(middleware::from_fn_with_state(state, jwt_auth));

    Router::new().merge(auth::router()).merge(protected)
}
```

(`provider_profiles::router()` already defines its full paths — `/provider-profiles` and `/provider-profiles/:id` — so it's nested at `/`, matching the pattern `admin/models.rs` would use if it defined `/models/:id` itself; this keeps the URL exactly `/api/v1/provider-profiles`.)

- [ ] **Step 5: Run tests to verify they pass**

Run: `cargo test -p godwit-api --lib admin::provider_profiles::`
Expected: PASS.

- [ ] **Step 6: Run the whole workspace to catch any other `AppState` construction site**

Run: `cargo check --workspace`
Expected: PASS (check whether other test helpers construct `AppState` literally and need the new field — none currently do based on the existing test suite, but confirm).

- [ ] **Step 7: Commit**

```bash
git add crates/godwit-api/src/admin/provider_profiles.rs crates/godwit-api/src/admin/mod.rs crates/godwit-api/src/state.rs crates/godwit-bin/src/main.rs
git commit -m "feat(api): admin provider-profiles CRUD"
```

---

## Task 15: Admin API — `models` CRUD

**Files:**
- Modify: `crates/godwit-api/src/admin/models.rs` (add `POST`, `PATCH`, `DELETE`)

**Interfaces:**
- Consumes: `ModelRepository::{create, update, delete}` (Task 3).

- [ ] **Step 1: Write the failing test**

Add to `crates/godwit-api/src/admin/models.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn create_model_request_deserializes() {
        let json = serde_json::json!({
            "public_id": "gpt-4o",
            "provider": "openai",
            "provider_profile_id": Uuid::nil(),
            "provider_model_id": "gpt-4o",
            "capabilities": "chat,embedding"
        });
        let req: CreateModelRequest = serde_json::from_value(json).expect("deserialize");
        assert_eq!(req.public_id, "gpt-4o");
        assert_eq!(req.capabilities, "chat,embedding");
    }
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p godwit-api --lib admin::models::tests::create_model_request_deserializes`
Expected: FAIL — `CreateModelRequest` not found.

- [ ] **Step 3: Add the routes and handlers**

Replace the whole non-test portion of `crates/godwit-api/src/admin/models.rs`:

```rust
use axum::{
    extract::{Extension, Path, State},
    routing::{get, patch},
    Json, Router,
};
use godwit_auth::{jwt::Claims, rbac::Role};
use godwit_db::repositories::models::ModelRepository;
use serde::Deserialize;
use std::sync::Arc;
use uuid::Uuid;

use crate::{error::ApiError, state::AppState};

pub fn router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/models", get(list_models).post(create_model))
        .route("/models/:id", patch(update_model).delete(delete_model))
}

fn require_super_admin(claims: &Claims) -> Result<(), ApiError> {
    let role = Role::from_str(&claims.role).ok_or(ApiError::Forbidden)?;
    if role != Role::SuperAdmin {
        return Err(ApiError::Forbidden);
    }
    Ok(())
}

async fn list_models(
    State(state): State<Arc<AppState>>,
    Extension(claims): Extension<Claims>,
) -> Result<Json<serde_json::Value>, ApiError> {
    require_super_admin(&claims)?;
    let repo = ModelRepository::new(state.pool.clone());
    let models = repo.list().await.map_err(ApiError::Core)?;
    Ok(Json(serde_json::json!({ "data": models })))
}

#[derive(Debug, Deserialize)]
pub struct CreateModelRequest {
    pub public_id: String,
    pub provider: String,
    pub provider_profile_id: Uuid,
    pub provider_model_id: String,
    pub capabilities: String,
}

async fn create_model(
    State(state): State<Arc<AppState>>,
    Extension(claims): Extension<Claims>,
    Json(req): Json<CreateModelRequest>,
) -> Result<Json<serde_json::Value>, ApiError> {
    require_super_admin(&claims)?;
    let repo = ModelRepository::new(state.pool.clone());
    let model = repo
        .create(&req.public_id, &req.provider, req.provider_profile_id, &req.provider_model_id, &req.capabilities)
        .await
        .map_err(ApiError::Core)?;
    Ok(Json(serde_json::json!({ "data": model })))
}

#[derive(Debug, Deserialize)]
pub struct UpdateModelRequest {
    pub public_id: Option<String>,
    pub capabilities: Option<String>,
}

async fn update_model(
    State(state): State<Arc<AppState>>,
    Extension(claims): Extension<Claims>,
    Path(id): Path<Uuid>,
    Json(req): Json<UpdateModelRequest>,
) -> Result<Json<serde_json::Value>, ApiError> {
    require_super_admin(&claims)?;
    let repo = ModelRepository::new(state.pool.clone());
    let model = repo
        .update(id, req.public_id.as_deref(), req.capabilities.as_deref())
        .await
        .map_err(ApiError::Core)?;
    Ok(Json(serde_json::json!({ "data": model })))
}

async fn delete_model(
    State(state): State<Arc<AppState>>,
    Extension(claims): Extension<Claims>,
    Path(id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, ApiError> {
    require_super_admin(&claims)?;
    let repo = ModelRepository::new(state.pool.clone());
    repo.delete(id).await.map_err(ApiError::Core)?;
    Ok(Json(serde_json::json!({ "deleted": true })))
}
```

(Append the `#[cfg(test)] mod tests` block from Step 1 at the end of the file, unchanged.)

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test -p godwit-api --lib admin::models::`
Expected: PASS.

- [ ] **Step 5: Run the full workspace**

Run: `cargo check --workspace`
Expected: PASS.

- [ ] **Step 6: Commit**

```bash
git add crates/godwit-api/src/admin/models.rs
git commit -m "feat(api): admin models CRUD, restrict to super_admin"
```

---

## Task 16: Proxy Route — `POST /v1/embeddings`

**Files:**
- Modify: `crates/godwit-api/src/proxy.rs` (add route + handler)

**Interfaces:**
- Consumes: `DbModelRouter::resolve(model_ref, Capability::Embedding)` (Task 12), `Adapter::embedding` (existing on all 7 adapters).

Routes in `godwit-api` have no existing pattern for in-process HTTP testing (the codebase's only route-level tests are `#[ignore]`d, requiring a live server — see `tests/proxy_integration.rs`); coverage here follows that same convention (a live-server smoke test, added in Task 22) plus `cargo check`/`cargo test -p godwit-api` to confirm the handler compiles and doesn't break existing tests. This task has no isolated red/green cycle of its own for that reason — it wires up the route, then the whole crate's existing suite is the regression gate.

- [ ] **Step 1: Add the route and handler**

In `crates/godwit-api/src/proxy.rs`, add to `router()`:

```rust
pub fn router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/v1/models", get(list_models))
        .route("/v1/chat/completions", post(chat_completions))
        .route("/v1/embeddings", post(embeddings))
}
```

Add the handler (near `chat_completions`), reusing the same resolve → call adapter → log pattern:

```rust
async fn embeddings(
    State(state): State<Arc<AppState>>,
    Extension(api_key): Extension<ApiKey>,
    Json(req): Json<godwit_core::EmbeddingRequest>,
) -> Result<Response, crate::error::ApiError> {
    let start = std::time::Instant::now();
    let resolved = state
        .model_router
        .resolve(&req.model, Capability::Embedding)
        .await?;

    let (resp, usage) = resolved
        .adapter
        .embedding(&resolved.resolved_credentials, &resolved.model, req)
        .await
        .map_err(|e| crate::error::ApiError::Core(godwit_core::PasteurError::Provider(e.to_string())))?;
    let ProviderResponse::Embedding(body) = resp else {
        return Err(crate::error::ApiError::Core(godwit_core::PasteurError::Provider(
            "unexpected provider response variant".to_string(),
        )));
    };

    let log = RequestLogEntry {
        api_key_id: api_key.id,
        user_id: api_key.user_id,
        organization_id: api_key.organization_id,
        team_id: api_key.team_id,
        model: resolved.model.public_id.clone(),
        provider: resolved.model.provider.clone(),
        provider_model_id: resolved.model.provider_model_id.clone(),
        capability: Capability::Embedding.as_str().to_string(),
        duration_ms: start.elapsed().as_millis() as i32,
        streamed: false,
        status: "success".to_string(),
        cost_usd: None,
    };
    spawn_request_log(state.pool.clone(), log);

    Ok(Json(body).into_response())
}
```

This introduces a `spawn_request_log` helper — extract the existing inline `tokio::spawn(async move { ... insert into request_logs ... })` block from `chat_completions` into a shared function so both handlers use it:

```rust
fn spawn_request_log(pool: sqlx::PgPool, log: RequestLogEntry) {
    tokio::spawn(async move {
        let _ = sqlx::query(
            "INSERT INTO request_logs (api_key_id, user_id, organization_id, team_id, model, provider, provider_model_id, capability, duration_ms, streamed, status, cost_usd)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12)"
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
        .execute(&pool)
        .await;
    });
}
```

Replace `chat_completions`'s inline `tokio::spawn(...)` block with a call to `spawn_request_log(state.pool.clone(), log);`.

- [ ] **Step 2: Run tests to verify they pass**

Run: `cargo test -p godwit-api --lib proxy::`
Expected: PASS. Also run `cargo check --workspace` to confirm the extraction didn't break `chat_completions`.

- [ ] **Step 3: Commit**

```bash
git add crates/godwit-api/src/proxy.rs
git commit -m "feat(api): POST /v1/embeddings proxy route"
```

---

## Task 17: Proxy Route — `POST /v1/images/generations`

**Files:**
- Modify: `crates/godwit-api/src/proxy.rs` (add route + handler)

**Interfaces:**
- Consumes: `DbModelRouter::resolve(model_ref, Capability::ImageGeneration)`, `Adapter::image_generation`.

- [ ] **Step 1: Add the route**

In `crates/godwit-api/src/proxy.rs`, add `.route("/v1/images/generations", post(image_generations))` to `router()`.

- [ ] **Step 2: Add the handler**

```rust
async fn image_generations(
    State(state): State<Arc<AppState>>,
    Extension(api_key): Extension<ApiKey>,
    Json(req): Json<godwit_core::ImageGenerationRequest>,
) -> Result<Response, crate::error::ApiError> {
    let start = std::time::Instant::now();
    let resolved = state
        .model_router
        .resolve(&req.model, Capability::ImageGeneration)
        .await?;

    let (resp, _usage) = resolved
        .adapter
        .image_generation(&resolved.resolved_credentials, &resolved.model, req)
        .await
        .map_err(|e| crate::error::ApiError::Core(godwit_core::PasteurError::Provider(e.to_string())))?;
    let ProviderResponse::Image(body) = resp else {
        return Err(crate::error::ApiError::Core(godwit_core::PasteurError::Provider(
            "unexpected provider response variant".to_string(),
        )));
    };

    spawn_request_log(state.pool.clone(), RequestLogEntry {
        api_key_id: api_key.id,
        user_id: api_key.user_id,
        organization_id: api_key.organization_id,
        team_id: api_key.team_id,
        model: resolved.model.public_id.clone(),
        provider: resolved.model.provider.clone(),
        provider_model_id: resolved.model.provider_model_id.clone(),
        capability: Capability::ImageGeneration.as_str().to_string(),
        duration_ms: start.elapsed().as_millis() as i32,
        streamed: false,
        status: "success".to_string(),
        cost_usd: None,
    });

    Ok(Json(body).into_response())
}
```

- [ ] **Step 3: Run test to verify it compiles and the workspace is green**

Run: `cargo check --workspace`
Expected: PASS.

- [ ] **Step 4: Commit**

```bash
git add crates/godwit-api/src/proxy.rs
git commit -m "feat(api): POST /v1/images/generations proxy route"
```

---

## Task 18: Proxy Route — `POST /v1/audio/speech`

**Files:**
- Modify: `crates/godwit-api/src/proxy.rs` (add route + handler)

**Interfaces:**
- Consumes: `DbModelRouter::resolve(model_ref, Capability::AudioTts)`, `Adapter::audio_tts`.

- [ ] **Step 1: Add the route**

Add `.route("/v1/audio/speech", post(audio_speech))` to `router()`.

- [ ] **Step 2: Add the handler**

`audio_tts` returns `ProviderResponse::Bytes(Vec<u8>, String)` (raw audio bytes + content type), so this handler returns a raw byte response instead of JSON:

```rust
async fn audio_speech(
    State(state): State<Arc<AppState>>,
    Extension(api_key): Extension<ApiKey>,
    Json(req): Json<godwit_core::AudioTtsRequest>,
) -> Result<Response, crate::error::ApiError> {
    let start = std::time::Instant::now();
    let resolved = state
        .model_router
        .resolve(&req.model, Capability::AudioTts)
        .await?;

    let (resp, _usage) = resolved
        .adapter
        .audio_tts(&resolved.resolved_credentials, &resolved.model, req)
        .await
        .map_err(|e| crate::error::ApiError::Core(godwit_core::PasteurError::Provider(e.to_string())))?;
    let ProviderResponse::Bytes(bytes, content_type) = resp else {
        return Err(crate::error::ApiError::Core(godwit_core::PasteurError::Provider(
            "unexpected provider response variant".to_string(),
        )));
    };

    spawn_request_log(state.pool.clone(), RequestLogEntry {
        api_key_id: api_key.id,
        user_id: api_key.user_id,
        organization_id: api_key.organization_id,
        team_id: api_key.team_id,
        model: resolved.model.public_id.clone(),
        provider: resolved.model.provider.clone(),
        provider_model_id: resolved.model.provider_model_id.clone(),
        capability: Capability::AudioTts.as_str().to_string(),
        duration_ms: start.elapsed().as_millis() as i32,
        streamed: false,
        status: "success".to_string(),
        cost_usd: None,
    });

    Ok((
        [(axum::http::header::CONTENT_TYPE, content_type)],
        bytes,
    ).into_response())
}
```

- [ ] **Step 3: Run test to verify the workspace is green**

Run: `cargo check --workspace`
Expected: PASS.

- [ ] **Step 4: Commit**

```bash
git add crates/godwit-api/src/proxy.rs
git commit -m "feat(api): POST /v1/audio/speech proxy route"
```

---

## Task 19: Proxy Route — `POST /v1/audio/transcriptions`

**Files:**
- Modify: `crates/godwit-api/src/proxy.rs` (add route + multipart handler)
- Modify: `crates/godwit-api/Cargo.toml` (ensure `axum`'s `multipart` feature is enabled)

**Interfaces:**
- Consumes: `DbModelRouter::resolve(model_ref, Capability::AudioStt)`, `Adapter::audio_stt(profile, model, request, file_bytes, filename, content_type)`.

- [ ] **Step 1: Enable the multipart feature**

Check `crates/godwit-api/Cargo.toml`'s `axum` dependency line; if it doesn't already list `features = [..., "multipart"]`, add it:

```toml
axum = { version = "0.7", features = ["multipart"] }
```

(Match whatever version is already pinned — only add the feature.)

- [ ] **Step 2: Add the route**

Add `.route("/v1/audio/transcriptions", post(audio_transcriptions))` to `router()`.

- [ ] **Step 3: Add the multipart handler**

```rust
async fn audio_transcriptions(
    State(state): State<Arc<AppState>>,
    Extension(api_key): Extension<ApiKey>,
    mut multipart: axum::extract::Multipart,
) -> Result<Response, crate::error::ApiError> {
    let mut model_name: Option<String> = None;
    let mut language: Option<String> = None;
    let mut response_format: Option<String> = None;
    let mut file_bytes: Option<Vec<u8>> = None;
    let mut filename = "audio".to_string();
    let mut content_type = "application/octet-stream".to_string();

    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|e| crate::error::ApiError::BadRequest(e.to_string()))?
    {
        match field.name().unwrap_or_default() {
            "model" => model_name = Some(field.text().await.unwrap_or_default()),
            "language" => language = Some(field.text().await.unwrap_or_default()),
            "response_format" => response_format = Some(field.text().await.unwrap_or_default()),
            "file" => {
                filename = field.file_name().unwrap_or("audio").to_string();
                content_type = field.content_type().unwrap_or("application/octet-stream").to_string();
                file_bytes = Some(
                    field
                        .bytes()
                        .await
                        .map_err(|e| crate::error::ApiError::BadRequest(e.to_string()))?
                        .to_vec(),
                );
            }
            _ => {}
        }
    }

    let model_name = model_name.ok_or_else(|| crate::error::ApiError::BadRequest("missing 'model' field".to_string()))?;
    let file_bytes = file_bytes.ok_or_else(|| crate::error::ApiError::BadRequest("missing 'file' field".to_string()))?;

    let start = std::time::Instant::now();
    let resolved = state
        .model_router
        .resolve(&model_name, Capability::AudioStt)
        .await?;

    let req = godwit_core::AudioSttRequest { model: model_name, language, response_format };
    let (resp, _usage) = resolved
        .adapter
        .audio_stt(&resolved.resolved_credentials, &resolved.model, req, file_bytes, filename, content_type)
        .await
        .map_err(|e| crate::error::ApiError::Core(godwit_core::PasteurError::Provider(e.to_string())))?;
    let ProviderResponse::AudioStt(body) = resp else {
        return Err(crate::error::ApiError::Core(godwit_core::PasteurError::Provider(
            "unexpected provider response variant".to_string(),
        )));
    };

    spawn_request_log(state.pool.clone(), RequestLogEntry {
        api_key_id: api_key.id,
        user_id: api_key.user_id,
        organization_id: api_key.organization_id,
        team_id: api_key.team_id,
        model: resolved.model.public_id.clone(),
        provider: resolved.model.provider.clone(),
        provider_model_id: resolved.model.provider_model_id.clone(),
        capability: Capability::AudioStt.as_str().to_string(),
        duration_ms: start.elapsed().as_millis() as i32,
        streamed: false,
        status: "success".to_string(),
        cost_usd: None,
    });

    Ok(Json(body).into_response())
}
```

- [ ] **Step 4: Run test to verify the workspace is green**

Run: `cargo check --workspace`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add crates/godwit-api/src/proxy.rs crates/godwit-api/Cargo.toml
git commit -m "feat(api): POST /v1/audio/transcriptions proxy route"
```

---

## Task 20: Proxy Route — `POST /v1/images/edits`

**Files:**
- Modify: `crates/godwit-api/src/proxy.rs` (add route + multipart handler)

**Interfaces:**
- Consumes: `DbModelRouter::resolve(model_ref, Capability::ImageEdit)`, `Adapter::image_edit(profile, model, request, image_bytes, image_filename, mask_bytes)` (Task 11).

- [ ] **Step 1: Add the route**

Add `.route("/v1/images/edits", post(image_edits))` to `router()`.

- [ ] **Step 2: Add the multipart handler**

```rust
async fn image_edits(
    State(state): State<Arc<AppState>>,
    Extension(api_key): Extension<ApiKey>,
    mut multipart: axum::extract::Multipart,
) -> Result<Response, crate::error::ApiError> {
    let mut model_name: Option<String> = None;
    let mut prompt: Option<String> = None;
    let mut n: Option<i32> = None;
    let mut size: Option<String> = None;
    let mut response_format: Option<String> = None;
    let mut image_bytes: Option<Vec<u8>> = None;
    let mut image_filename = "image.png".to_string();
    let mut mask_bytes: Option<Vec<u8>> = None;

    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|e| crate::error::ApiError::BadRequest(e.to_string()))?
    {
        match field.name().unwrap_or_default() {
            "model" => model_name = Some(field.text().await.unwrap_or_default()),
            "prompt" => prompt = Some(field.text().await.unwrap_or_default()),
            "n" => n = field.text().await.ok().and_then(|s| s.parse().ok()),
            "size" => size = Some(field.text().await.unwrap_or_default()),
            "response_format" => response_format = Some(field.text().await.unwrap_or_default()),
            "image" => {
                image_filename = field.file_name().unwrap_or("image.png").to_string();
                image_bytes = Some(
                    field
                        .bytes()
                        .await
                        .map_err(|e| crate::error::ApiError::BadRequest(e.to_string()))?
                        .to_vec(),
                );
            }
            "mask" => {
                mask_bytes = Some(
                    field
                        .bytes()
                        .await
                        .map_err(|e| crate::error::ApiError::BadRequest(e.to_string()))?
                        .to_vec(),
                );
            }
            _ => {}
        }
    }

    let model_name = model_name.ok_or_else(|| crate::error::ApiError::BadRequest("missing 'model' field".to_string()))?;
    let prompt = prompt.ok_or_else(|| crate::error::ApiError::BadRequest("missing 'prompt' field".to_string()))?;
    let image_bytes = image_bytes.ok_or_else(|| crate::error::ApiError::BadRequest("missing 'image' field".to_string()))?;

    let start = std::time::Instant::now();
    let resolved = state
        .model_router
        .resolve(&model_name, Capability::ImageEdit)
        .await?;

    let req = godwit_core::ImageEditRequest { model: model_name, prompt, n, size, response_format };
    let (resp, _usage) = resolved
        .adapter
        .image_edit(&resolved.resolved_credentials, &resolved.model, req, image_bytes, image_filename, mask_bytes)
        .await
        .map_err(|e| crate::error::ApiError::Core(godwit_core::PasteurError::Provider(e.to_string())))?;
    let ProviderResponse::Image(body) = resp else {
        return Err(crate::error::ApiError::Core(godwit_core::PasteurError::Provider(
            "unexpected provider response variant".to_string(),
        )));
    };

    spawn_request_log(state.pool.clone(), RequestLogEntry {
        api_key_id: api_key.id,
        user_id: api_key.user_id,
        organization_id: api_key.organization_id,
        team_id: api_key.team_id,
        model: resolved.model.public_id.clone(),
        provider: resolved.model.provider.clone(),
        provider_model_id: resolved.model.provider_model_id.clone(),
        capability: Capability::ImageEdit.as_str().to_string(),
        duration_ms: start.elapsed().as_millis() as i32,
        streamed: false,
        status: "success".to_string(),
        cost_usd: None,
    });

    Ok(Json(body).into_response())
}
```

- [ ] **Step 3: Run test to verify the workspace is green**

Run: `cargo check --workspace`
Expected: PASS.

- [ ] **Step 4: Commit**

```bash
git add crates/godwit-api/src/proxy.rs
git commit -m "feat(api): POST /v1/images/edits proxy route"
```

---

## Task 21: Startup Bootstrap & Config Deprecation

**Files:**
- Modify: `crates/godwit-core/src/lib.rs` (remove `ProvidersConfig`/`ProviderConfig` from `AppConfig`, or keep `ProviderConfig` struct but drop it from `AppConfig` — see Step 3)
- Modify: `crates/godwit-bin/src/main.rs` (add bootstrap step)
- Modify: `config.example.yaml` (drop the `providers:` section)

**Interfaces:**
- Consumes: `ProviderProfileRepository::{list, create, set_auth}` (Task 3), `encrypt_api_key` (Task 2).

- [ ] **Step 1: Write the failing test**

In `crates/godwit-bin/src/main.rs` (or a new `crates/godwit-bin/src/bootstrap.rs` module — create the latter for testability, since `main.rs` itself has no test harness):

Create `crates/godwit-bin/src/bootstrap.rs`:

```rust
use godwit_auth::credentials::encrypt_api_key;
use godwit_db::repositories::provider_profiles::ProviderProfileRepository;
use sqlx::PgPool;

pub struct LegacyProviderConfig {
    pub name: &'static str,
    pub protocol: &'static str,
    pub base_url: String,
    pub api_key: String,
}

pub async fn bootstrap_provider_profiles(
    pool: &PgPool,
    master_key: &[u8; 32],
    legacy: &[LegacyProviderConfig],
) -> anyhow::Result<()> {
    let repo = ProviderProfileRepository::new(pool.clone());
    if !repo.list().await?.is_empty() {
        return Ok(());
    }
    for provider in legacy {
        let profile = repo
            .create(provider.name, provider.protocol, Some(&provider.base_url), false)
            .await?;
        let secret = encrypt_api_key(master_key, &provider.api_key);
        repo.set_auth(profile.id, &secret).await?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use sqlx::PgPool;

    #[sqlx::test(migrations = "../godwit-db/migrations")]
    async fn seeds_profiles_when_table_is_empty(pool: PgPool) {
        let legacy = vec![LegacyProviderConfig {
            name: "openai",
            protocol: "openai",
            base_url: "https://api.openai.com/v1".to_string(),
            api_key: "sk-legacy".to_string(),
        }];
        bootstrap_provider_profiles(&pool, &[9u8; 32], &legacy).await.expect("bootstrap");

        let repo = ProviderProfileRepository::new(pool);
        let profiles = repo.list().await.expect("list");
        assert_eq!(profiles.len(), 1);
        assert_eq!(profiles[0].name, "openai");
    }

    #[sqlx::test(migrations = "../godwit-db/migrations")]
    async fn does_nothing_when_profiles_already_exist(pool: PgPool) {
        let repo = ProviderProfileRepository::new(pool.clone());
        repo.create("existing", "openai", None, false).await.expect("create profile");

        let legacy = vec![LegacyProviderConfig {
            name: "openai",
            protocol: "openai",
            base_url: "https://api.openai.com/v1".to_string(),
            api_key: "sk-legacy".to_string(),
        }];
        bootstrap_provider_profiles(&pool, &[9u8; 32], &legacy).await.expect("bootstrap");

        let profiles = repo.list().await.expect("list");
        assert_eq!(profiles.len(), 1, "should not add legacy profiles when any profile already exists");
        assert_eq!(profiles[0].name, "existing");
    }
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `DATABASE_URL="postgres://tmenard@localhost:5432/godwit" cargo test -p godwit-bin bootstrap`
Expected: FAIL — `bootstrap.rs` isn't wired into `main.rs` as a module yet (`cannot find module 'bootstrap'`), since it's a new file with no `mod` declaration.

- [ ] **Step 3: Wire the module and call it from `main`**

Add `mod bootstrap;` at the top of `crates/godwit-bin/src/main.rs`. Remove `ProvidersConfig`/`ProviderConfig` from `AppConfig` in `crates/godwit-core/src/lib.rs`:

```rust
#[derive(Debug, Clone, Deserialize)]
pub struct AppConfig {
    pub server: ServerConfig,
    pub database: DatabaseConfig,
    pub auth: AuthConfig,
}
```

(Delete the `ProvidersConfig`/`ProviderConfig` structs entirely, and the `providers:` block from the `config_parses_from_yaml` test's YAML fixture in the same file.)

In `crates/godwit-bin/src/main.rs`, `let master_key = godwit_auth::credentials::load_master_key_from_env("CREDENTIAL_ENCRYPTION_KEY")?;` already exists from Task 13 Step 4, right before the `AdapterRegistry` is built. Insert the bootstrap call right after that existing line, reusing the same `master_key` binding (do not redeclare it) and before the registry construction:

```rust
    let legacy_providers = bootstrap::legacy_providers_from_env();
    bootstrap::bootstrap_provider_profiles(&pool, &master_key, &legacy_providers).await?;
```

Add `legacy_providers_from_env` to `crates/godwit-bin/src/bootstrap.rs`:

```rust
pub fn legacy_providers_from_env() -> Vec<LegacyProviderConfig> {
    let mut providers = Vec::new();
    if let Ok(key) = std::env::var("OPENAI_API_KEY") {
        providers.push(LegacyProviderConfig {
            name: "openai",
            protocol: "openai",
            base_url: std::env::var("OPENAI_BASE_URL").unwrap_or_else(|_| "https://api.openai.com/v1".to_string()),
            api_key: key,
        });
    }
    if let Ok(key) = std::env::var("ANTHROPIC_API_KEY") {
        providers.push(LegacyProviderConfig {
            name: "anthropic",
            protocol: "anthropic",
            base_url: std::env::var("ANTHROPIC_BASE_URL").unwrap_or_else(|_| "https://api.anthropic.com".to_string()),
            api_key: key,
        });
    }
    if let Ok(key) = std::env::var("GEMINI_API_KEY") {
        providers.push(LegacyProviderConfig {
            name: "gemini",
            protocol: "gemini",
            base_url: std::env::var("GEMINI_BASE_URL").unwrap_or_else(|_| "https://generativelanguage.googleapis.com".to_string()),
            api_key: key,
        });
    }
    providers
}
```

- [ ] **Step 4: Update `config.example.yaml`**

Remove the `providers:` section from `config.example.yaml`, since credentials now come from env vars read directly by the bootstrap (`OPENAI_API_KEY`, `ANTHROPIC_API_KEY`, `GEMINI_API_KEY`, and their `*_BASE_URL` overrides) and, after the first run, from `provider_profiles` in the database.

- [ ] **Step 5: Run tests to verify they pass**

Run: `DATABASE_URL="postgres://tmenard@localhost:5432/godwit" cargo test -p godwit-bin bootstrap`
Expected: PASS (both tests).

- [ ] **Step 6: Run the full workspace**

Run: `cargo check --workspace && DATABASE_URL="postgres://tmenard@localhost:5432/godwit" cargo test --workspace 2>&1 | tail -100`
Expected: PASS. Update the `config_parses_from_yaml` test in `godwit-core` if it still references the removed `providers:` YAML block (it must, since the struct no longer has that field — remove those assertions and the corresponding lines from the test's YAML fixture).

- [ ] **Step 7: Commit**

```bash
git add crates/godwit-core/src/lib.rs crates/godwit-bin config.example.yaml
git commit -m "feat(bin): bootstrap provider_profiles from env, drop static config"
```

---

## Task 22: Final Regression Pass

**Files:**
- Modify: `tests/proxy_integration.rs` (add smoke coverage notes for the new routes)
- Modify: `README.md` (update the provider/route list to match reality)

**Interfaces:** none new — this task only verifies and documents.

- [ ] **Step 1: Add integration smoke test stubs for the new routes**

In `tests/proxy_integration.rs`, following the existing `#[ignore]` pattern for `proxy_chat_completion_smoke`, add:

```rust
#[tokio::test]
#[ignore = "requires running server"]
async fn proxy_embeddings_smoke() {
    let client = reqwest::Client::new();
    let res = client
        .post("http://localhost:3000/v1/embeddings")
        .header("Authorization", "Bearer sk-godwit-test")
        .json(&serde_json::json!({"model": "text-embedding-3-small", "input": ["hello"]}))
        .send()
        .await
        .expect("request");
    assert!(res.status().is_success());
}

#[tokio::test]
#[ignore = "requires running server"]
async fn proxy_image_generations_smoke() {
    let client = reqwest::Client::new();
    let res = client
        .post("http://localhost:3000/v1/images/generations")
        .header("Authorization", "Bearer sk-godwit-test")
        .json(&serde_json::json!({"model": "gpt-image-1", "prompt": "a cat wearing a hat"}))
        .send()
        .await
        .expect("request");
    assert!(res.status().is_success());
}

#[tokio::test]
#[ignore = "requires running server"]
async fn proxy_audio_speech_smoke() {
    let client = reqwest::Client::new();
    let res = client
        .post("http://localhost:3000/v1/audio/speech")
        .header("Authorization", "Bearer sk-godwit-test")
        .json(&serde_json::json!({"model": "tts-1", "input": "hello world", "voice": "alloy"}))
        .send()
        .await
        .expect("request");
    assert!(res.status().is_success());
}

#[tokio::test]
#[ignore = "requires running server"]
async fn proxy_audio_transcriptions_smoke() {
    let form = reqwest::multipart::Form::new()
        .text("model", "whisper-1")
        .part("file", reqwest::multipart::Part::bytes(vec![0u8; 16]).file_name("clip.wav"));
    let client = reqwest::Client::new();
    let res = client
        .post("http://localhost:3000/v1/audio/transcriptions")
        .header("Authorization", "Bearer sk-godwit-test")
        .multipart(form)
        .send()
        .await
        .expect("request");
    assert!(res.status().is_success());
}

#[tokio::test]
#[ignore = "requires running server"]
async fn proxy_image_edits_smoke() {
    let form = reqwest::multipart::Form::new()
        .text("model", "gpt-image-1")
        .text("prompt", "add a hat")
        .part("image", reqwest::multipart::Part::bytes(vec![0u8; 16]).file_name("image.png"));
    let client = reqwest::Client::new();
    let res = client
        .post("http://localhost:3000/v1/images/edits")
        .header("Authorization", "Bearer sk-godwit-test")
        .multipart(form)
        .send()
        .await
        .expect("request");
    assert!(res.status().is_success());
}
```

- [ ] **Step 2: Run the full workspace test suite one more time**

Run: `export PATH="/usr/local/opt/rustup/bin:$PATH" && DATABASE_URL="postgres://tmenard@localhost:5432/godwit" cargo test --workspace 2>&1 | tail -200`
Expected: PASS, zero failures, across every crate (`godwit-core`, `godwit-db`, `godwit-auth`, `godwit-providers`, `godwit-cache`, `godwit-api`, `godwit-bin`, and the root integration-test package).

- [ ] **Step 3: Update `README.md`**

Update the "Status", "Architecture", "API", and "Configuration" sections of `README.md` to: list all 7 providers (OpenAI, Anthropic, Gemini, vllm, sglang, llama.cpp, ollama); document the new proxy routes (`/v1/embeddings`, `/v1/images/generations`, `/v1/images/edits`, `/v1/audio/speech`, `/v1/audio/transcriptions`); document `CREDENTIAL_ENCRYPTION_KEY` as a required env var; document the admin `provider-profiles` endpoints; remove references to `config.yaml`'s `providers:` section per Task 21.

- [ ] **Step 4: Commit**

```bash
git add tests/proxy_integration.rs README.md
git commit -m "test(integration): smoke coverage for new routes, update README"
```
