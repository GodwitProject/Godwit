use axum::{middleware, routing::Router};
use godwit_api::{
    admin, anthropic_proxy, batch, health, moderation, model_router::DbModelRouter, proxy,
    rate_limit::RateLimiter, rerank, state::AppState,
};
use godwit_cache::MemoryCache;
use godwit_core::{AppConfig, Protocol};
use godwit_db::{
    connect,
    repositories::{
        api_keys::ApiKeyRepository, end_users::EndUsersRepository,
        organizations::OrganizationRepository, refresh_tokens::RefreshTokenRepository,
        team_memberships::TeamMembershipRepository, teams::TeamRepository, users::UserRepository,
    },
    run_migrations,
};
use godwit_providers::{
    anthropic::AnthropicAdapter, gemini::GeminiAdapter, llama_cpp::LlamaCppAdapter,
    ollama::OllamaAdapter, openai::OpenAiAdapter, sglang::SglangAdapter, vllm::VllmAdapter,
    AdapterRegistry,
};
use godwit_mcp::{
    config::{McpConfig, McpServerConfig},
    McpRegistry,
};
use std::sync::Arc;

mod bootstrap;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();

    let config: AppConfig = load_config()?;
    let pool = connect(&config.database.url).await?;
    run_migrations(&pool).await?;

    let master_key =
        godwit_auth::credentials::load_master_key_from_env("CREDENTIAL_ENCRYPTION_KEY")?;

    let legacy_providers = bootstrap::legacy_providers_from_env();
    bootstrap::bootstrap_provider_profiles(&pool, &master_key, &legacy_providers).await?;

    if let (Ok(email), Ok(password)) =
        (std::env::var("ADMIN_EMAIL"), std::env::var("ADMIN_PASSWORD"))
    {
        bootstrap::bootstrap_admin_user(&pool, &email, &password).await?;
    }

    let mut registry = AdapterRegistry::new();
    registry.register(Protocol::openai(), Arc::new(OpenAiAdapter::new()));
    registry.register(Protocol::anthropic(), Arc::new(AnthropicAdapter::new()));
    registry.register(Protocol::gemini(), Arc::new(GeminiAdapter::new()));
    registry.register(Protocol::vllm(), Arc::new(VllmAdapter::new()));
    registry.register(Protocol::sglang(), Arc::new(SglangAdapter::new()));
    registry.register(Protocol::llama_cpp(), Arc::new(LlamaCppAdapter::new()));
    registry.register(Protocol::ollama(), Arc::new(OllamaAdapter::new()));

    let adapter_registry = Arc::new(registry);

    // Agentic wiring: expose configured MCP servers as tools, and a SearXNG backend for
    // web-search tool calls when the selected adapter has no native web search.
    let mcp_registry = Arc::new(build_mcp_registry(&config));
    let (searxng, searxng_profile) = build_searxng(&config);

    let state = Arc::new(AppState {
        config: config.clone(),
        pool: pool.clone(),
        adapter_registry: adapter_registry.clone(),
        model_router: DbModelRouter::new(pool.clone(), adapter_registry, master_key),
        mcp: mcp_registry,
        searxng,
        searxng_profile,
        user_repo: UserRepository::new(pool.clone()),
        org_repo: OrganizationRepository::new(pool.clone()),
        team_repo: TeamRepository::new(pool.clone()),
        team_membership_repo: TeamMembershipRepository::new(pool.clone()),
        api_key_repo: ApiKeyRepository::new(pool.clone()),
        refresh_token_repo: RefreshTokenRepository::new(pool.clone()),
        end_user_repo: EndUsersRepository::new(pool.clone()),
        api_key_cache: MemoryCache::new(),
        credential_master_key: master_key,
        rate_limiter: RateLimiter::new(),
    });

    // `api_key_auth` is applied to the proxy router alone (via `route_layer` on its own
    // value) so admin routes — authenticated by `jwt_auth` inside `admin::router` — are
    // never subject to it. Applying it after merging the two routers would wrap both.
    // Health endpoints are registered before auth middleware so they don't require authentication.
    let proxy_router = proxy::router()
        .merge(anthropic_proxy::router())
        .merge(moderation::router())
        .merge(rerank::router())
        .merge(batch::router())
        .route_layer(middleware::from_fn_with_state(
            state.clone(),
            godwit_api::middleware::api_key_auth,
        ));

    let app = Router::new()
        .merge(health::router())
        .nest("/api/v1", admin::router(state.clone()))
        .merge(proxy_router)
        .with_state(state.clone());

    let listener =
        tokio::net::TcpListener::bind(format!("{}:{}", config.server.host, config.server.port))
            .await?;
    tracing::info!(
        "Godwit listening on {}:{}",
        config.server.host,
        config.server.port
    );
    axum::serve(listener, app).await?;
    Ok(())
}

fn load_config() -> anyhow::Result<AppConfig> {
    let path = std::env::var("CONFIG_PATH").unwrap_or_else(|_| "config.yaml".to_string());
    let file = std::fs::File::open(&path)?;
    let config: AppConfig = serde_yaml::from_reader(file)?;
    Ok(config)
}

/// Build an [`McpRegistry`] from the configured `mcp_servers`, establishing connections
/// lazily at first use. Returns an empty registry when none are configured.
fn build_mcp_registry(config: &AppConfig) -> McpRegistry {
    let servers: Vec<McpServerConfig> = config
        .agentic
        .mcp_servers
        .iter()
        .map(|s| McpServerConfig {
            name: s.name.clone(),
            command: s.command.clone(),
            args: s.args.clone(),
            env: s.env.clone(),
        })
        .collect();
    McpRegistry::from_config(&McpConfig::new(servers))
}

/// Build a [`SearxngProvider`] (and its resolved profile) when configured.
fn build_searxng(config: &AppConfig) -> (Option<godwit_providers::SearxngProvider>, Option<godwit_providers::adapter::ResolvedProfile>) {
    match &config.agentic.searxng {
        Some(s) => (
            Some(godwit_providers::SearxngProvider::new()),
            Some(godwit_providers::adapter::ResolvedProfile {
                base_url: s.base_url.clone(),
                api_key: None,
            }),
        ),
        None => (None, None),
    }
}
