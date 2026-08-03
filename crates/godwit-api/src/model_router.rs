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
        // A profile always needs *some* endpoint to call.
        let base_url = profile
            .base_url
            .clone()
            .ok_or_else(|| PasteurError::Provider(format!("provider profile '{}' has no base_url configured", profile.name)))?;

        // No stored credentials is a legitimate configuration, not an error: self-hosted
        // engines (vllm/sglang/llama.cpp/ollama) are commonly deployed with no auth at all,
        // and every adapter only attaches an auth header `if let Some(key)`.
        if profile.auth.is_null() || profile.auth == serde_json::json!({}) {
            return Ok(ResolvedProfile { base_url, api_key: None });
        }

        // Credentials that ARE present but unreadable is a different story: that means an
        // operator-side misconfiguration (wrong CREDENTIAL_ENCRYPTION_KEY), DB corruption,
        // or tampering — a hard error, and specifically a *server-side* one. It must not be
        // reported as PasteurError::Auth, which `ApiError` maps to a 401 that would falsely
        // blame the caller's own API key.
        let secret: EncryptedSecret = serde_json::from_value(profile.auth.clone())
            .map_err(|e| PasteurError::Provider(format!("malformed stored credentials: {e}")))?;
        let api_key = decrypt_api_key(&self.master_key, &secret).map_err(|e| {
            PasteurError::Provider(format!("failed to decrypt stored provider credentials: {e}"))
        })?;
        Ok(ResolvedProfile { base_url, api_key: Some(api_key) })
    }

    /// An operator-disabled profile must behave as if it does not exist, for both direct
    /// (`profile_name/...`) and catalog-model-mediated lookups.
    fn ensure_enabled(profile: ProviderProfile) -> Result<ProviderProfile, PasteurError> {
        if !profile.enabled {
            return Err(PasteurError::NotFound);
        }
        Ok(profile)
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
            let profile = Self::ensure_enabled(profile_repo.get_by_name(name).await?)?;
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
                    let profile =
                        Self::ensure_enabled(profile_repo.get(model.provider_profile_id).await?)?;
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
        let profile = profiles
            .create("default", "openai", Some("https://api.openai.com/v1"), false)
            .await
            .expect("create profile");
        let secret = encrypt_api_key(&TEST_KEY, "sk-test-key");
        profiles.set_auth(profile.id, &secret).await.expect("set auth");

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
        let profile_b = profiles
            .create("azure", "openai", Some("https://api.openai.com/v1"), false)
            .await
            .expect("create profile b");
        let secret = encrypt_api_key(&TEST_KEY, "sk-test-key");
        profiles.set_auth(profile_b.id, &secret).await.expect("set auth");

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
        let profile = profiles
            .create("openai", "openai", Some("https://api.openai.com/v1"), true)
            .await
            .expect("create wildcard profile");
        let secret = encrypt_api_key(&TEST_KEY, "sk-test-key");
        profiles.set_auth(profile.id, &secret).await.expect("set auth");

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
    async fn resolve_errors_with_wrong_master_key(pool: PgPool) {
        let profiles = ProviderProfileRepository::new(pool.clone());
        let profile = profiles.create("openai", "openai", Some("https://api.openai.com/v1"), true).await.expect("create profile");
        let secret = encrypt_api_key(&TEST_KEY, "sk-real-key");
        profiles.set_auth(profile.id, &secret).await.expect("set auth");

        let wrong_key = [9u8; 32]; // different from TEST_KEY
        let router = DbModelRouter::new(pool, test_registry(), wrong_key);
        let err = router.resolve("openai/gpt-4o", Capability::Chat).await.unwrap_err();
        // Undecryptable *stored* credentials are a server-side misconfiguration, so this
        // must be Provider (-> 500), never Auth (-> 401, which would blame the caller's
        // own API key for the operator's wrong CREDENTIAL_ENCRYPTION_KEY).
        assert!(
            matches!(err, PasteurError::Provider(_)),
            "expected Provider error from decrypt failure, got {:?}",
            err
        );
    }

    #[sqlx::test(migrations = "../godwit-db/migrations")]
    async fn resolve_errors_with_malformed_stored_credentials(pool: PgPool) {
        let profiles = ProviderProfileRepository::new(pool.clone());
        let profile = profiles
            .create("openai", "openai", Some("https://api.openai.com/v1"), true)
            .await
            .expect("create profile");
        // `auth` present but not a valid EncryptedSecret.
        sqlx::query("UPDATE provider_profiles SET auth = $2 WHERE id = $1")
            .bind(profile.id)
            .bind(serde_json::json!({"garbage": true}))
            .execute(&pool)
            .await
            .expect("write malformed auth");

        let router = DbModelRouter::new(pool, test_registry(), TEST_KEY);
        let err = router.resolve("openai/gpt-4o", Capability::Chat).await.unwrap_err();
        assert!(matches!(err, PasteurError::Provider(_)), "got {:?}", err);
    }

    /// A profile with no stored credentials is a first-class configuration (self-hosted
    /// engines usually have no auth), not an error: `ResolvedProfile.api_key` is `Option`
    /// precisely so adapters can skip the auth header.
    #[sqlx::test(migrations = "../godwit-db/migrations")]
    async fn resolve_succeeds_with_no_credentials_and_yields_none_api_key(pool: PgPool) {
        let profiles = ProviderProfileRepository::new(pool.clone());
        profiles
            .create("local-vllm", "openai", Some("http://localhost:8000/v1"), true)
            .await
            .expect("create keyless profile");

        let router = DbModelRouter::new(pool, test_registry(), TEST_KEY);
        let resolved = router
            .resolve("local-vllm/meta-llama/Llama-3-70B", Capability::Chat)
            .await
            .expect("a keyless profile must resolve, not error");
        assert_eq!(resolved.resolved_credentials.base_url, "http://localhost:8000/v1");
        assert!(
            resolved.resolved_credentials.api_key.is_none(),
            "expected api_key None for a profile with no stored credentials"
        );
    }

    #[sqlx::test(migrations = "../godwit-db/migrations")]
    async fn resolve_errors_when_profile_has_no_base_url(pool: PgPool) {
        let profiles = ProviderProfileRepository::new(pool.clone());
        profiles
            .create("openai", "openai", None, true)
            .await
            .expect("create profile");

        let router = DbModelRouter::new(pool, test_registry(), TEST_KEY);
        let err = router.resolve("openai/gpt-4o", Capability::Chat).await.unwrap_err();
        assert!(matches!(err, PasteurError::Provider(_)), "got {:?}", err);
    }

    /// Fix: `provider_profiles.enabled` was never consulted, so a "disabled" profile kept
    /// serving traffic. A disabled profile must behave as if it does not exist.
    #[sqlx::test(migrations = "../godwit-db/migrations")]
    async fn disabled_profile_resolved_by_name_returns_not_found(pool: PgPool) {
        let profiles = ProviderProfileRepository::new(pool.clone());
        let profile = profiles
            .create("openai", "openai", Some("https://api.openai.com/v1"), true)
            .await
            .expect("create profile");
        let secret = encrypt_api_key(&TEST_KEY, "sk-test-key");
        profiles.set_auth(profile.id, &secret).await.expect("set auth");
        profiles
            .update(profile.id, None, None, Some(false))
            .await
            .expect("disable profile");

        let router = DbModelRouter::new(pool, test_registry(), TEST_KEY);
        let err = router.resolve("openai/gpt-4o", Capability::Chat).await.unwrap_err();
        assert!(matches!(err, PasteurError::NotFound), "got {:?}", err);
    }

    #[sqlx::test(migrations = "../godwit-db/migrations")]
    async fn disabled_profile_resolved_via_catalog_model_returns_not_found(pool: PgPool) {
        let profiles = ProviderProfileRepository::new(pool.clone());
        let profile = profiles
            .create("openai", "openai", Some("https://api.openai.com/v1"), false)
            .await
            .expect("create profile");
        let secret = encrypt_api_key(&TEST_KEY, "sk-test-key");
        profiles.set_auth(profile.id, &secret).await.expect("set auth");

        let models = ModelRepository::new(pool.clone());
        models
            .create("gpt-4o", "openai", profile.id, "gpt-4o", "chat")
            .await
            .expect("create model");

        // Sanity: resolvable while enabled.
        let router = DbModelRouter::new(pool.clone(), test_registry(), TEST_KEY);
        router.resolve("gpt-4o", Capability::Chat).await.expect("resolve while enabled");

        profiles
            .update(profile.id, None, None, Some(false))
            .await
            .expect("disable profile");

        let err = router.resolve("gpt-4o", Capability::Chat).await.unwrap_err();
        assert!(matches!(err, PasteurError::NotFound), "got {:?}", err);
    }

    #[sqlx::test(migrations = "../godwit-db/migrations")]
    async fn disabled_profile_rejects_prefixed_catalog_model(pool: PgPool) {
        let profiles = ProviderProfileRepository::new(pool.clone());
        let profile = profiles
            .create("openai", "openai", Some("https://api.openai.com/v1"), false)
            .await
            .expect("create profile");
        let models = ModelRepository::new(pool.clone());
        models
            .create("gpt-4o", "openai", profile.id, "gpt-4o", "chat")
            .await
            .expect("create model");
        profiles
            .update(profile.id, None, None, Some(false))
            .await
            .expect("disable profile");

        let router = DbModelRouter::new(pool, test_registry(), TEST_KEY);
        let err = router.resolve("openai/gpt-4o", Capability::Chat).await.unwrap_err();
        assert!(matches!(err, PasteurError::NotFound), "got {:?}", err);
    }
}
