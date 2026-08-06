use dashmap::DashMap;
use godwit_auth::credentials::{decrypt_api_key, EncryptedSecret};
use godwit_core::{Capability, PasteurError, Protocol};
use godwit_db::models::{Model, ProviderProfile};
use godwit_db::repositories::{
    models::ModelRepository, provider_profiles::ProviderProfileRepository,
};
use godwit_providers::adapter::ResolvedProfile;
use godwit_providers::{Adapter, AdapterRegistry};
use sqlx::PgPool;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LoadBalanceStrategy {
    RoundRobin,
    LeastBusy,
    Latency,
}

impl LoadBalanceStrategy {
    pub fn from_config(config: &serde_json::Value) -> Option<Self> {
        config.get("load_balance")?.as_str().and_then(|s| match s {
            "round_robin" => Some(LoadBalanceStrategy::RoundRobin),
            "least_busy" => Some(LoadBalanceStrategy::LeastBusy),
            "latency" => Some(LoadBalanceStrategy::Latency),
            _ => None,
        })
    }
}

pub struct InFlightGuard {
    counter: Arc<AtomicUsize>,
}

impl Drop for InFlightGuard {
    fn drop(&mut self) {
        self.counter.fetch_sub(1, Ordering::SeqCst);
    }
}

pub struct ResolvedModel {
    pub model: Model,
    pub profile: ProviderProfile,
    pub resolved_credentials: ResolvedProfile,
    pub adapter: Arc<dyn Adapter>,
    #[allow(dead_code)]
    pub in_flight: Option<InFlightGuard>,
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
    round_robin_counter: AtomicUsize,
    in_flight: DashMap<Uuid, Arc<AtomicUsize>>,
    latency_ewma: DashMap<Uuid, std::sync::Mutex<f64>>,
}

impl DbModelRouter {
    pub fn new(pool: PgPool, registry: Arc<AdapterRegistry>, master_key: [u8; 32]) -> Self {
        Self {
            pool,
            registry,
            master_key,
            round_robin_counter: AtomicUsize::new(0),
            in_flight: DashMap::new(),
            latency_ewma: DashMap::new(),
        }
    }

    fn resolve_credentials(
        &self,
        profile: &ProviderProfile,
    ) -> Result<ResolvedProfile, PasteurError> {
        let base_url = profile.base_url.clone().ok_or_else(|| {
            PasteurError::Provider(format!(
                "provider profile '{}' has no base_url configured",
                profile.name
            ))
        })?;

        if profile.auth.is_null() || profile.auth == serde_json::json!({}) {
            return Ok(ResolvedProfile {
                base_url,
                api_key: None,
            });
        }

        let secret: EncryptedSecret = serde_json::from_value(profile.auth.clone())
            .map_err(|e| PasteurError::Provider(format!("malformed stored credentials: {e}")))?;
        let api_key = decrypt_api_key(&self.master_key, &secret).map_err(|e| {
            PasteurError::Provider(format!(
                "failed to decrypt stored provider credentials: {e}"
            ))
        })?;
        Ok(ResolvedProfile {
            base_url,
            api_key: Some(api_key),
        })
    }

    fn ensure_enabled(profile: ProviderProfile) -> Result<ProviderProfile, PasteurError> {
        if !profile.enabled {
            return Err(PasteurError::NotFound);
        }
        Ok(profile)
    }

    fn strategy_for(candidates: &[Model]) -> LoadBalanceStrategy {
        candidates
            .first()
            .and_then(|m| LoadBalanceStrategy::from_config(&m.config))
            .unwrap_or(LoadBalanceStrategy::RoundRobin)
    }

    fn start_in_flight(&self, model_id: Uuid) -> InFlightGuard {
        let counter = self
            .in_flight
            .entry(model_id)
            .or_insert_with(|| Arc::new(AtomicUsize::new(0)))
            .clone();
        counter.fetch_add(1, Ordering::SeqCst);
        InFlightGuard { counter }
    }

    fn select_load_balanced(&self, candidates: &[Model], strategy: LoadBalanceStrategy) -> Model {
        match strategy {
            LoadBalanceStrategy::RoundRobin => {
                let idx = self.round_robin_counter.fetch_add(1, Ordering::SeqCst)
                    % candidates.len().max(1);
                candidates[idx].clone()
            }
            LoadBalanceStrategy::LeastBusy => candidates
                .iter()
                .min_by_key(|m| {
                    self.in_flight
                        .get(&m.id)
                        .map(|c| c.load(Ordering::SeqCst))
                        .unwrap_or(0)
                })
                .cloned()
                .unwrap_or_else(|| candidates[0].clone()),
            LoadBalanceStrategy::Latency => candidates
                .iter()
                .min_by(|a, b| {
                    let a_lat = self
                        .latency_ewma
                        .get(&a.id)
                        .map(|entry| {
                            let lat = *entry.lock().unwrap_or_else(|p| p.into_inner());
                            lat
                        })
                        .unwrap_or(f64::INFINITY);
                    let b_lat = self
                        .latency_ewma
                        .get(&b.id)
                        .map(|entry| {
                            let lat = *entry.lock().unwrap_or_else(|p| p.into_inner());
                            lat
                        })
                        .unwrap_or(f64::INFINITY);
                    a_lat.partial_cmp(&b_lat).unwrap_or(std::cmp::Ordering::Equal)
                })
                .cloned()
                .unwrap_or_else(|| candidates[0].clone()),
        }
    }

    pub fn fallback_chain(model: &Model) -> Vec<String> {
        model
            .config
            .get("fallbacks")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().map(|s| s.to_string()))
                    .collect()
            })
            .unwrap_or_default()
    }

    pub fn record_latency(&self, model_id: Uuid, duration_ms: i32) {
        let alpha = 0.2;
        let entry = self
            .latency_ewma
            .entry(model_id)
            .or_insert_with(|| std::sync::Mutex::new(duration_ms as f64));
        if let Ok(mut ewma) = entry.lock() {
            *ewma = alpha * duration_ms as f64 + (1.0 - alpha) * *ewma;
        };
    }

    pub async fn resolve(
        &self,
        model_ref: &str,
        requested_capability: Capability,
    ) -> Result<ResolvedModel, PasteurError> {
        let (profile_name, suffix) = if let Some((name, rest)) = model_ref.split_once('/') {
            (Some(name), rest)
        } else {
            (None, model_ref)
        };

        let model_repo = ModelRepository::new(self.pool.clone());
        let profile_repo = ProviderProfileRepository::new(self.pool.clone());

        let (model, profile) = if let Some(name) = profile_name {
            let profile = Self::ensure_enabled(profile_repo.get_by_name(name).await?)?;
            match model_repo
                .get_by_profile_and_public_id(profile.id, suffix)
                .await
            {
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
            let candidates: Vec<Model> = models
                .into_iter()
                .filter(|m| m.public_id == suffix)
                .collect();
            match candidates.len() {
                0 => return Err(PasteurError::NotFound),
                1 => {
                    let model = candidates.into_iter().next().unwrap();
                    let profile =
                        Self::ensure_enabled(profile_repo.get(model.provider_profile_id).await?)?;
                    (model, profile)
                }
                _ => {
                    let strategy = Self::strategy_for(&candidates);
                    let model = self.select_load_balanced(&candidates, strategy);
                    let profile =
                        Self::ensure_enabled(profile_repo.get(model.provider_profile_id).await?)?;
                    (model, profile)
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

        let in_flight = if model.id.is_nil() {
            None
        } else {
            Some(self.start_in_flight(model.id))
        };

        let resolved_credentials = self.resolve_credentials(&profile)?;
        let protocol = Protocol(profile.protocol.clone());
        let adapter = self.registry.get(&protocol).ok_or_else(|| {
            PasteurError::Provider(format!("unknown protocol: {}", profile.protocol))
        })?;

        Ok(ResolvedModel {
            model,
            profile,
            resolved_credentials,
            adapter,
            in_flight,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use godwit_auth::credentials::encrypt_api_key;
    use godwit_db::repositories::{
        models::ModelRepository, provider_profiles::ProviderProfileRepository,
    };
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
            .create(
                "default",
                "openai",
                Some("https://api.openai.com/v1"),
                false,
            )
            .await
            .expect("create profile");
        let secret = encrypt_api_key(&TEST_KEY, "sk-test-key");
        profiles
            .set_auth(profile.id, &secret)
            .await
            .expect("set auth");

        let models = ModelRepository::new(pool.clone());
        let model = models
            .create("gpt-4o", "openai", profile.id, "gpt-4o", "chat")
            .await
            .expect("create model");

        let router = DbModelRouter::new(pool, test_registry(), TEST_KEY);
        let resolved = router
            .resolve("gpt-4o", Capability::Chat)
            .await
            .expect("resolve");
        assert_eq!(resolved.model.id, model.id);
        assert_eq!(resolved.profile.id, profile.id);
    }

    #[sqlx::test(migrations = "../godwit-db/migrations")]
    async fn bare_public_id_load_balances_when_duplicated(pool: PgPool) {
        let profiles = ProviderProfileRepository::new(pool.clone());
        let profile_a = profiles
            .create("openai", "openai", Some("https://api.openai.com/v1"), false)
            .await
            .expect("create profile a");
        let profile_b = profiles
            .create("azure", "openai", Some("https://api.openai.com/v1"), false)
            .await
            .expect("create profile b");

        let models = ModelRepository::new(pool.clone());
        let model_a = models
            .create("gpt-4o", "openai", profile_a.id, "gpt-4o", "chat")
            .await
            .expect("create model a");
        models
            .create("gpt-4o", "openai", profile_b.id, "gpt-4o", "chat")
            .await
            .expect("create model b");

        let router = DbModelRouter::new(pool, test_registry(), TEST_KEY);
        let resolved = router
            .resolve("gpt-4o", Capability::Chat)
            .await
            .expect("resolve");
        // Round-robin starts at index 0 and should select the first created model.
        assert_eq!(resolved.model.id, model_a.id);
    }

    #[sqlx::test(migrations = "../godwit-db/migrations")]
    async fn load_balance_least_busy_prefers_idle_model(pool: PgPool) {
        let profiles = ProviderProfileRepository::new(pool.clone());
        let profile_a = profiles
            .create("openai", "openai", Some("https://api.openai.com/v1"), false)
            .await
            .expect("create profile a");
        let profile_b = profiles
            .create("azure", "openai", Some("https://api.openai.com/v1"), false)
            .await
            .expect("create profile b");

        let models = ModelRepository::new(pool.clone());
        let model_a = models
            .create("gpt-4o", "openai", profile_a.id, "gpt-4o", "chat")
            .await
            .expect("create model a");
        let model_b = models
            .create("gpt-4o", "openai", profile_b.id, "gpt-4o", "chat")
            .await
            .expect("create model b");

        sqlx::query(
            "UPDATE models SET config = $2 WHERE id = $1",
        )
        .bind(model_a.id)
        .bind(serde_json::json!({ "load_balance": "least_busy" }))
        .execute(&pool)
        .await
        .expect("update model a config");
        sqlx::query(
            "UPDATE models SET config = $2 WHERE id = $1",
        )
        .bind(model_b.id)
        .bind(serde_json::json!({ "load_balance": "least_busy" }))
        .execute(&pool)
        .await
        .expect("update model b config");

        let router = DbModelRouter::new(pool, test_registry(), TEST_KEY);

        // Start one in-flight request for model_a.
        let _guard = router.start_in_flight(model_a.id);

        let resolved = router
            .resolve("gpt-4o", Capability::Chat)
            .await
            .expect("resolve");
        assert_eq!(resolved.model.id, model_b.id);
    }

    #[sqlx::test(migrations = "../godwit-db/migrations")]
    async fn load_balance_latency_prefers_lower_latency(pool: PgPool) {
        let profiles = ProviderProfileRepository::new(pool.clone());
        let profile_a = profiles
            .create("openai", "openai", Some("https://api.openai.com/v1"), false)
            .await
            .expect("create profile a");
        let profile_b = profiles
            .create("azure", "openai", Some("https://api.openai.com/v1"), false)
            .await
            .expect("create profile b");

        let models = ModelRepository::new(pool.clone());
        let model_a = models
            .create("gpt-4o", "openai", profile_a.id, "gpt-4o", "chat")
            .await
            .expect("create model a");
        let model_b = models
            .create("gpt-4o", "openai", profile_b.id, "gpt-4o", "chat")
            .await
            .expect("create model b");

        sqlx::query("UPDATE models SET config = $2 WHERE id = $1")
            .bind(model_a.id)
            .bind(serde_json::json!({ "load_balance": "latency" }))
            .execute(&pool)
            .await
            .expect("update model a config");
        sqlx::query("UPDATE models SET config = $2 WHERE id = $1")
            .bind(model_b.id)
            .bind(serde_json::json!({ "load_balance": "latency" }))
            .execute(&pool)
            .await
            .expect("update model b config");

        let router = DbModelRouter::new(pool, test_registry(), TEST_KEY);
        router.record_latency(model_a.id, 1000);
        router.record_latency(model_b.id, 100);

        let resolved = router
            .resolve("gpt-4o", Capability::Chat)
            .await
            .expect("resolve");
        assert_eq!(resolved.model.id, model_b.id);
    }

    #[sqlx::test(migrations = "../godwit-db/migrations")]
    async fn fallback_chain_read_from_config(pool: PgPool) {
        let profiles = ProviderProfileRepository::new(pool.clone());
        let profile = profiles
            .create("openai", "openai", Some("https://api.openai.com/v1"), false)
            .await
            .expect("create profile");
        let models = ModelRepository::new(pool.clone());
        let model = models
            .create("gpt-4o", "openai", profile.id, "gpt-4o", "chat")
            .await
            .expect("create model");

        sqlx::query("UPDATE models SET config = $2 WHERE id = $1")
            .bind(model.id)
            .bind(serde_json::json!({ "fallbacks": ["gpt-4o-backup", "claude-sonnet-backup"] }))
            .execute(&pool)
            .await
            .expect("update config");

        let router = DbModelRouter::new(pool, test_registry(), TEST_KEY);
        let resolved = router.resolve("gpt-4o", Capability::Chat).await.expect("resolve");
        assert_eq!(
            DbModelRouter::fallback_chain(&resolved.model),
            vec!["gpt-4o-backup", "claude-sonnet-backup"]
        );
    }

    #[sqlx::test(migrations = "../godwit-db/migrations")]
    async fn profile_prefix_selects_correct_model(pool: PgPool) {
        let profiles = ProviderProfileRepository::new(pool.clone());
        let profile_a = profiles
            .create("openai", "openai", None, false)
            .await
            .expect("create profile a");
        let profile_b = profiles
            .create("azure", "openai", Some("https://api.openai.com/v1"), false)
            .await
            .expect("create profile b");
        let secret = encrypt_api_key(&TEST_KEY, "sk-test-key");
        profiles
            .set_auth(profile_b.id, &secret)
            .await
            .expect("set auth");

        let models = ModelRepository::new(pool.clone());
        models
            .create("gpt-4o", "openai", profile_a.id, "gpt-4o", "chat")
            .await
            .expect("create model a");
        let model_b = models
            .create("gpt-4o", "openai", profile_b.id, "gpt-4o", "chat")
            .await
            .expect("create model b");

        let router = DbModelRouter::new(pool, test_registry(), TEST_KEY);
        let resolved = router
            .resolve("azure/gpt-4o", Capability::Chat)
            .await
            .expect("resolve");
        assert_eq!(resolved.model.id, model_b.id);
        assert_eq!(resolved.profile.id, profile_b.id);
    }

    #[sqlx::test(migrations = "../godwit-db/migrations")]
    async fn unknown_public_id_returns_not_found(pool: PgPool) {
        let router = DbModelRouter::new(pool, test_registry(), TEST_KEY);
        let err = router
            .resolve("unknown-model", Capability::Chat)
            .await
            .unwrap_err();
        assert!(matches!(err, PasteurError::NotFound));
    }

    #[sqlx::test(migrations = "../godwit-db/migrations")]
    async fn unknown_profile_prefix_returns_not_found(pool: PgPool) {
        let profiles = ProviderProfileRepository::new(pool.clone());
        let profile = profiles
            .create("openai", "openai", None, false)
            .await
            .expect("create profile");
        let models = ModelRepository::new(pool.clone());
        models
            .create("gpt-4o", "openai", profile.id, "gpt-4o", "chat")
            .await
            .expect("create model");

        let router = DbModelRouter::new(pool, test_registry(), TEST_KEY);
        let err = router
            .resolve("missing/gpt-4o", Capability::Chat)
            .await
            .unwrap_err();
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
        profiles
            .set_auth(profile.id, &secret)
            .await
            .expect("set auth");

        let router = DbModelRouter::new(pool, test_registry(), TEST_KEY);
        let resolved = router
            .resolve("openai/gpt-4o-mini-anything", Capability::Chat)
            .await
            .expect("resolve");
        assert_eq!(resolved.model.public_id, "openai/gpt-4o-mini-anything");
        assert_eq!(resolved.model.provider_model_id, "gpt-4o-mini-anything");
        assert!(resolved.model.has_capability(Capability::Chat));
    }

    #[sqlx::test(migrations = "../godwit-db/migrations")]
    async fn non_wildcard_profile_rejects_unknown_suffix(pool: PgPool) {
        let profiles = ProviderProfileRepository::new(pool.clone());
        profiles
            .create("openai", "openai", None, false)
            .await
            .expect("create profile");

        let router = DbModelRouter::new(pool, test_registry(), TEST_KEY);
        let err = router
            .resolve("openai/anything", Capability::Chat)
            .await
            .unwrap_err();
        assert!(matches!(err, PasteurError::NotFound));
    }

    #[sqlx::test(migrations = "../godwit-db/migrations")]
    async fn resolves_decrypted_credentials(pool: PgPool) {
        let profiles = ProviderProfileRepository::new(pool.clone());
        let profile = profiles
            .create("openai", "openai", Some("https://api.openai.com/v1"), true)
            .await
            .expect("create profile");
        let secret = encrypt_api_key(&TEST_KEY, "sk-real-key");
        profiles
            .set_auth(profile.id, &secret)
            .await
            .expect("set auth");

        let router = DbModelRouter::new(pool, test_registry(), TEST_KEY);
        let resolved = router
            .resolve("openai/gpt-4o", Capability::Chat)
            .await
            .expect("resolve");
        assert_eq!(
            resolved.resolved_credentials.base_url,
            "https://api.openai.com/v1"
        );
        assert_eq!(
            resolved.resolved_credentials.api_key.as_deref(),
            Some("sk-real-key")
        );
    }

    #[sqlx::test(migrations = "../godwit-db/migrations")]
    async fn resolve_errors_with_wrong_master_key(pool: PgPool) {
        let profiles = ProviderProfileRepository::new(pool.clone());
        let profile = profiles
            .create("openai", "openai", Some("https://api.openai.com/v1"), true)
            .await
            .expect("create profile");
        let secret = encrypt_api_key(&TEST_KEY, "sk-real-key");
        profiles
            .set_auth(profile.id, &secret)
            .await
            .expect("set auth");

        let wrong_key = [9u8; 32];
        let router = DbModelRouter::new(pool, test_registry(), wrong_key);
        let err = router
            .resolve("openai/gpt-4o", Capability::Chat)
            .await
            .unwrap_err();
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
        sqlx::query("UPDATE provider_profiles SET auth = $2 WHERE id = $1")
            .bind(profile.id)
            .bind(serde_json::json!({"garbage": true}))
            .execute(&pool)
            .await
            .expect("write malformed auth");

        let router = DbModelRouter::new(pool, test_registry(), TEST_KEY);
        let err = router
            .resolve("openai/gpt-4o", Capability::Chat)
            .await
            .unwrap_err();
        assert!(matches!(err, PasteurError::Provider(_)), "got {:?}", err);
    }

    #[sqlx::test(migrations = "../godwit-db/migrations")]
    async fn resolve_succeeds_with_no_credentials_and_yields_none_api_key(pool: PgPool) {
        let profiles = ProviderProfileRepository::new(pool.clone());
        profiles
            .create(
                "local-vllm",
                "openai",
                Some("http://localhost:8000/v1"),
                true,
            )
            .await
            .expect("create keyless profile");

        let router = DbModelRouter::new(pool, test_registry(), TEST_KEY);
        let resolved = router
            .resolve("local-vllm/meta-llama/Llama-3-70B", Capability::Chat)
            .await
            .expect("a keyless profile must resolve, not error");
        assert_eq!(
            resolved.resolved_credentials.base_url,
            "http://localhost:8000/v1"
        );
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
        let err = router
            .resolve("openai/gpt-4o", Capability::Chat)
            .await
            .unwrap_err();
        assert!(matches!(err, PasteurError::Provider(_)), "got {:?}", err);
    }

    #[sqlx::test(migrations = "../godwit-db/migrations")]
    async fn disabled_profile_resolved_by_name_returns_not_found(pool: PgPool) {
        let profiles = ProviderProfileRepository::new(pool.clone());
        let profile = profiles
            .create("openai", "openai", Some("https://api.openai.com/v1"), true)
            .await
            .expect("create profile");
        let secret = encrypt_api_key(&TEST_KEY, "sk-test-key");
        profiles
            .set_auth(profile.id, &secret)
            .await
            .expect("set auth");
        profiles
            .update(profile.id, None, None, Some(false))
            .await
            .expect("disable profile");

        let router = DbModelRouter::new(pool, test_registry(), TEST_KEY);
        let err = router
            .resolve("openai/gpt-4o", Capability::Chat)
            .await
            .unwrap_err();
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
        profiles
            .set_auth(profile.id, &secret)
            .await
            .expect("set auth");

        let models = ModelRepository::new(pool.clone());
        models
            .create("gpt-4o", "openai", profile.id, "gpt-4o", "chat")
            .await
            .expect("create model");

        let router = DbModelRouter::new(pool.clone(), test_registry(), TEST_KEY);
        router
            .resolve("gpt-4o", Capability::Chat)
            .await
            .expect("resolve while enabled");

        profiles
            .update(profile.id, None, None, Some(false))
            .await
            .expect("disable profile");

        let err = router
            .resolve("gpt-4o", Capability::Chat)
            .await
            .unwrap_err();
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
        let err = router
            .resolve("openai/gpt-4o", Capability::Chat)
            .await
            .unwrap_err();
        assert!(matches!(err, PasteurError::NotFound), "got {:?}", err);
    }
}
