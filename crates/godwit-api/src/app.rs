use crate::{
    admin,
    agentic_loop::AgenticLoop,
    anthropic_proxy, circuit_breaker::CircuitBreakerRegistry, health, login_rate_limit::LoginLimiter,
    metrics_endpoint, moderation, model_router::DbModelRouter, proxy,
    rate_limit::RateLimiter, rerank, state::AppState, utils,
};
use axum::{middleware, routing::Router};
use godwit_cache::MemoryCache;
use godwit_core::{AppConfig, AuthConfig};
use godwit_mcp::McpRegistry;
use godwit_db::repositories::{
    api_keys::ApiKeyRepository, end_users::EndUsersRepository,
    organizations::OrganizationRepository, refresh_tokens::RefreshTokenRepository,
    team_memberships::TeamMembershipRepository, teams::TeamRepository, users::UserRepository,
};
use sqlx::PgPool;
use std::sync::Arc;

/// Assemble the full production router.
///
/// Integrates the provided state so the returned router has no remaining state
/// placeholder and can be served directly (mirrors the historical inline
/// assembly in `godwit-bin/src/main.rs`).
pub fn build_app(state: Arc<AppState>) -> Router {
    let proxy_router = proxy::router()
        .merge(anthropic_proxy::router())
        .merge(moderation::router())
        .merge(rerank::router())
        .route_layer(middleware::from_fn_with_state(
            state.clone(),
            crate::middleware::api_key_auth,
        ));

    Router::new()
        .merge(health::router())
        .merge(metrics_endpoint::router())
        .merge(utils::router())
        .merge(proxy_router)
        .nest("/api/v1", admin::router(state.clone()))
        .with_state(state)
}

fn base_auth() -> AuthConfig {
    AuthConfig {
        jwt_secret: "test-jwt-secret".to_string(),
        access_token_ttl_minutes: 15,
        refresh_token_ttl_days: 7,
        cookie_secure: false,
        allowed_cookie_origin: "".to_string(),
        login_max_attempts_per_minute: 10,
        trust_proxy: false,
        oidc_providers: vec![],
        saml_providers: vec![],
        mail: None,
        password_policy: godwit_core::PasswordPolicy::default(),
    }
}

/// Build an `AppState` from a pool and a default test config.
///
/// This is shared between the integration tests and any other in-process harness
/// that needs the real router wiring without duplicating the construction logic.
pub fn build_test_state(pool: PgPool) -> Arc<AppState> {
    build_test_state_with_auth(pool, base_auth())
}

/// Build an `AppState` from a pool and a caller-supplied auth config.
///
/// The login limiter capacity is derived from `auth.login_max_attempts_per_minute`
/// so tests that tune rate-limiting behave correctly.
pub fn build_test_state_with_auth(pool: PgPool, auth: AuthConfig) -> Arc<AppState> {
    use godwit_core::{DatabaseConfig, Protocol, ServerConfig};
    use godwit_providers::{
        anthropic::AnthropicAdapter, gemini::GeminiAdapter, llama_cpp::LlamaCppAdapter,
        ollama::OllamaAdapter, openai::OpenAiAdapter, sglang::SglangAdapter, vllm::VllmAdapter,
        AdapterRegistry,
    };

    let login_capacity = auth.login_max_attempts_per_minute.max(0) as u32;
    let config = AppConfig {
        server: ServerConfig {
            host: "127.0.0.1".to_string(),
            port: 0,
            request_timeout_seconds: 30,
        },
        database: DatabaseConfig {
            url: "postgres://unused".to_string(),
        },
        auth,
        agentic: godwit_core::AgenticConfig::default(),
        compat: None,
        circuit_breaker: None,
        moderation: godwit_core::ModerationConfig::default(),
        rerank: godwit_core::RerankConfig::default(),
        batch: godwit_core::BatchConfig::default(),
        cache: godwit_core::CacheConfig::default(),
        pii: godwit_core::PiiConfig::default(),
        moderation_pre: None,
        moderation_post: None,
        block_on_moderation_failure: None,
    };

    let mut registry = AdapterRegistry::new();
    registry.register(Protocol::openai(), Arc::new(OpenAiAdapter::new()));
    registry.register(Protocol::anthropic(), Arc::new(AnthropicAdapter::new()));
    registry.register(Protocol::gemini(), Arc::new(GeminiAdapter::new()));
    registry.register(Protocol::vllm(), Arc::new(VllmAdapter::new()));
    registry.register(Protocol::sglang(), Arc::new(SglangAdapter::new()));
    registry.register(Protocol::llama_cpp(), Arc::new(LlamaCppAdapter::new()));
    registry.register(Protocol::ollama(), Arc::new(OllamaAdapter::new()));
    let adapter_registry = Arc::new(registry);

    const MASTER_KEY: [u8; 32] = [42u8; 32];

    Arc::new(AppState {
        config,
        pool: pool.clone(),
        adapter_registry: adapter_registry.clone(),
        model_router: DbModelRouter::new(pool.clone(), adapter_registry, MASTER_KEY),
        mcp: Arc::new(McpRegistry::new()),
        searxng: None,
        searxng_profile: None,
        user_repo: UserRepository::new(pool.clone()),
        org_repo: OrganizationRepository::new(pool.clone()),
        team_repo: TeamRepository::new(pool.clone()),
        team_membership_repo: TeamMembershipRepository::new(pool.clone()),
        api_key_repo: ApiKeyRepository::new(pool.clone()),
        refresh_token_repo: RefreshTokenRepository::new(pool.clone()),
        end_user_repo: EndUsersRepository::new(pool.clone()),
        api_key_cache: MemoryCache::new(),
        credential_master_key: MASTER_KEY,
        rate_limiter: RateLimiter::new(),
        login_limiter: LoginLimiter::new(login_capacity),
        circuit_breaker_registry: Arc::new(CircuitBreakerRegistry::new(
            5,
            std::time::Duration::from_secs(60),
            3,
        )),
        agentic_loop: Arc::new(AgenticLoop::new(4, 120)),
        guardrails: Arc::new(tokio::sync::Mutex::new(
            godwit_core::guardrails::GuardrailsOrchestrator::new(
                godwit_core::guardrails::GuardrailsConfig::default(),
            ),
        )),
    })
}
