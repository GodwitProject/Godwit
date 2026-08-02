use godwit_cache::MemoryCache;
use godwit_core::AppConfig;
use godwit_db::models::{ApiKey, Model};
use godwit_db::repositories::{
    api_keys::ApiKeyRepository, organizations::OrganizationRepository, users::UserRepository,
};
use godwit_providers::AdapterRegistry;
use sqlx::PgPool;
use std::sync::Arc;

pub struct AppState {
    pub config: AppConfig,
    pub pool: PgPool,
    pub adapter_registry: Arc<AdapterRegistry>,
    pub user_repo: UserRepository,
    pub org_repo: OrganizationRepository,
    pub api_key_repo: ApiKeyRepository,
    pub api_key_cache: MemoryCache<String, ApiKey>,
    pub model_cache: MemoryCache<(uuid::Uuid, String), Model>,
}
