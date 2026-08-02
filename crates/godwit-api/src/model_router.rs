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

        let model = if let Some(name) = profile_name {
            let mut found = None;
            for m in candidates {
                if let Ok(p) = profile_repo.get(m.provider_profile_id).await {
                    if p.name == name {
                        found = Some(m);
                        break;
                    }
                }
            }
            found.ok_or(PasteurError::NotFound)?
        } else if candidates.len() == 1 {
            candidates.into_iter().next().unwrap()
        } else if candidates.is_empty() {
            return Err(PasteurError::NotFound);
        } else {
            return Err(PasteurError::Validation(
                format!("ambiguous model '{}'; use 'profile_name/{}'", public_id, public_id)
            ));
        };

        let profile = profile_repo.get(model.provider_profile_id).await?;
        let protocol = Protocol(model.provider.clone());
        let adapter = self
            .registry
            .get(&protocol)
            .ok_or_else(|| PasteurError::Provider(format!("unknown protocol: {}", model.provider)))?;

        Ok(ResolvedModel {
            model,
            profile,
            adapter,
        })
    }
}
