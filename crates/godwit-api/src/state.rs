use crate::{circuit_breaker::CircuitBreakerRegistry, model_router::DbModelRouter, rate_limit::RateLimiter};
use godwit_cache::MemoryCache;
use godwit_core::AppConfig;
use godwit_db::models::ApiKey;
use godwit_db::repositories::{
    api_keys::ApiKeyRepository, end_users::EndUsersRepository,
    organizations::OrganizationRepository, refresh_tokens::RefreshTokenRepository,
    team_memberships::TeamMembershipRepository, teams::TeamRepository, users::UserRepository,
};
use godwit_mcp::McpRegistry;
use godwit_providers::adapter::ResolvedProfile;
use godwit_providers::{AdapterRegistry, SearxngProvider};
use sqlx::PgPool;
use std::sync::Arc;

pub struct AppState {
    pub config: AppConfig,
    pub pool: PgPool,
    pub adapter_registry: Arc<AdapterRegistry>,
    pub model_router: DbModelRouter,
    /// MCP tool registry; empty when no `mcp_servers` are configured.
    pub mcp: Arc<McpRegistry>,
    /// SearXNG web-search backend; present only when configured.
    pub searxng: Option<SearxngProvider>,
    /// Resolved profile for the SearXNG backend (its base URL).
    pub searxng_profile: Option<ResolvedProfile>,
    pub user_repo: UserRepository,
    pub org_repo: OrganizationRepository,
    pub team_repo: TeamRepository,
    pub team_membership_repo: TeamMembershipRepository,
    pub api_key_repo: ApiKeyRepository,
    pub refresh_token_repo: RefreshTokenRepository,
    pub end_user_repo: EndUsersRepository,
    pub api_key_cache: MemoryCache<String, ApiKey>,
    pub credential_master_key: [u8; 32],
    pub rate_limiter: RateLimiter,
    pub circuit_breaker_registry: Arc<CircuitBreakerRegistry>,
}
