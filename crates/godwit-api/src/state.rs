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
