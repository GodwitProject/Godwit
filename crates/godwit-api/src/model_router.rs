use godwit_core::{PasteurError, Protocol};
use godwit_db::models::{Model, ProviderProfile};
use godwit_db::repositories::{models::ModelRepository, provider_profiles::ProviderProfileRepository};
use godwit_providers::{Adapter, AdapterRegistry};
use sqlx::PgPool;
use std::sync::Arc;
use uuid::Uuid;

pub struct ResolvedModel {
    pub model: Model,
    pub profile: ProviderProfile,
    pub adapter: Arc<dyn Adapter>,
}

impl std::fmt::Debug for ResolvedModel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ResolvedModel")
            .field("model", &self.model)
            .field("profile", &self.profile)
            .field("adapter", &"<dyn Adapter>")
            .finish()
    }
}

pub struct DbModelRouter {
    pool: PgPool,
    registry: Arc<AdapterRegistry>,
}

impl DbModelRouter {
    pub fn new(pool: PgPool, registry: Arc<AdapterRegistry>) -> Self {
        Self { pool, registry }
    }

    pub async fn resolve(&self, organization_id: Uuid, model_ref: &str) -> Result<ResolvedModel, PasteurError> {
        let (profile_name, public_id) = if model_ref.contains('/') {
            let mut parts = model_ref.splitn(2, '/');
            (Some(parts.next().unwrap()), parts.next().unwrap())
        } else {
            (None, model_ref)
        };

        let model_repo = ModelRepository::new(self.pool.clone());
        let profile_repo = ProviderProfileRepository::new(self.pool.clone());

        let models = model_repo
            .list_for_organization(organization_id)
            .await?;

        let candidates: Vec<Model> = models
            .into_iter()
            .filter(|m| m.public_id == public_id)
            .collect();

        let (model, profile) = if let Some(name) = profile_name {
            let profile = profile_repo.get_by_name(organization_id, name).await?;
            let model = candidates
                .into_iter()
                .find(|m| m.provider_profile_id == profile.id)
                .ok_or(PasteurError::NotFound)?;
            (model, profile)
        } else if candidates.len() == 1 {
            let model = candidates.into_iter().next().unwrap();
            let profile = profile_repo.get(model.provider_profile_id).await?;
            (model, profile)
        } else if candidates.is_empty() {
            return Err(PasteurError::NotFound);
        } else {
            return Err(PasteurError::Validation(
                format!("ambiguous model '{}'; use 'profile_name/{}'", public_id, public_id)
            ));
        };
        let protocol = Protocol(profile.protocol.clone());
        let adapter = self
            .registry
            .get(&protocol)
            .ok_or_else(|| PasteurError::Provider(format!("unknown protocol: {}", profile.protocol)))?;

        Ok(ResolvedModel {
            model,
            profile,
            adapter,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use godwit_db::repositories::{
        models::ModelRepository, organizations::OrganizationRepository,
        provider_profiles::ProviderProfileRepository,
    };
    use godwit_providers::openai::OpenAiAdapter;
    use sqlx::PgPool;

    fn test_registry() -> Arc<AdapterRegistry> {
        let mut registry = AdapterRegistry::new();
        registry.register(
            Protocol::openai(),
            Arc::new(OpenAiAdapter::new("fake-key", "https://api.openai.com/v1")),
        );
        Arc::new(registry)
    }

    #[sqlx::test(migrations = "../godwit-db/migrations")]
    async fn bare_public_id_resolves_when_unique(pool: PgPool) {
        let orgs = OrganizationRepository::new(pool.clone());
        let org = orgs.create("test-org").await.expect("create org");

        let profiles = ProviderProfileRepository::new(pool.clone());
        let profile = profiles
            .create(org.id, "default", "openai", None)
            .await
            .expect("create profile");

        let models = ModelRepository::new(pool.clone());
        let model = models
            .create(org.id, "gpt-4o", "openai", profile.id, "gpt-4o")
            .await
            .expect("create model");

        let router = DbModelRouter::new(pool, test_registry());
        let resolved = router.resolve(org.id, "gpt-4o").await.expect("resolve");
        assert_eq!(resolved.model.id, model.id);
        assert_eq!(resolved.profile.id, profile.id);
    }

    #[sqlx::test(migrations = "../godwit-db/migrations")]
    async fn bare_public_id_ambiguous_when_duplicated(pool: PgPool) {
        let orgs = OrganizationRepository::new(pool.clone());
        let org = orgs.create("test-org").await.expect("create org");

        let profiles = ProviderProfileRepository::new(pool.clone());
        let profile_a = profiles
            .create(org.id, "openai", "openai", None)
            .await
            .expect("create profile a");
        let profile_b = profiles
            .create(org.id, "azure", "openai", None)
            .await
            .expect("create profile b");

        let models = ModelRepository::new(pool.clone());
        models
            .create(org.id, "gpt-4o", "openai", profile_a.id, "gpt-4o")
            .await
            .expect("create model a");
        models
            .create(org.id, "gpt-4o", "openai", profile_b.id, "gpt-4o")
            .await
            .expect("create model b");

        let router = DbModelRouter::new(pool, test_registry());
        let err = router.resolve(org.id, "gpt-4o").await.unwrap_err();
        assert!(
            matches!(err, PasteurError::Validation(_)),
            "expected ambiguous validation error, got {:?}",
            err
        );
    }

    #[sqlx::test(migrations = "../godwit-db/migrations")]
    async fn profile_prefix_selects_correct_model(pool: PgPool) {
        let orgs = OrganizationRepository::new(pool.clone());
        let org = orgs.create("test-org").await.expect("create org");

        let profiles = ProviderProfileRepository::new(pool.clone());
        let profile_a = profiles
            .create(org.id, "openai", "openai", None)
            .await
            .expect("create profile a");
        let profile_b = profiles
            .create(org.id, "azure", "openai", None)
            .await
            .expect("create profile b");

        let models = ModelRepository::new(pool.clone());
        models
            .create(org.id, "gpt-4o", "openai", profile_a.id, "gpt-4o")
            .await
            .expect("create model a");
        let model_b = models
            .create(org.id, "gpt-4o", "openai", profile_b.id, "gpt-4o")
            .await
            .expect("create model b");

        let router = DbModelRouter::new(pool, test_registry());
        let resolved = router
            .resolve(org.id, "azure/gpt-4o")
            .await
            .expect("resolve");
        assert_eq!(resolved.model.id, model_b.id);
        assert_eq!(resolved.profile.id, profile_b.id);
    }

    #[sqlx::test(migrations = "../godwit-db/migrations")]
    async fn unknown_public_id_returns_not_found(pool: PgPool) {
        let orgs = OrganizationRepository::new(pool.clone());
        let org = orgs.create("test-org").await.expect("create org");

        let router = DbModelRouter::new(pool, test_registry());
        let err = router.resolve(org.id, "unknown-model").await.unwrap_err();
        assert!(matches!(err, PasteurError::NotFound));
    }

    #[sqlx::test(migrations = "../godwit-db/migrations")]
    async fn unknown_profile_prefix_returns_not_found(pool: PgPool) {
        let orgs = OrganizationRepository::new(pool.clone());
        let org = orgs.create("test-org").await.expect("create org");

        let profiles = ProviderProfileRepository::new(pool.clone());
        let profile = profiles
            .create(org.id, "openai", "openai", None)
            .await
            .expect("create profile");

        let models = ModelRepository::new(pool.clone());
        models
            .create(org.id, "gpt-4o", "openai", profile.id, "gpt-4o")
            .await
            .expect("create model");

        let router = DbModelRouter::new(pool, test_registry());
        let err = router.resolve(org.id, "missing/gpt-4o").await.unwrap_err();
        assert!(matches!(err, PasteurError::NotFound));
    }
}
