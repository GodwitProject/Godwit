use pasteurllm_auth::jwt::Claims;
use pasteurllm_cache::MemoryCache;
use pasteurllm_core::AppConfig;
use pasteurllm_db::models::{ApiKey, Model};
use pasteurllm_db::repositories::{
    api_keys::ApiKeyRepository, organizations::OrganizationRepository, users::UserRepository,
};
use pasteurllm_providers::Provider;
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
