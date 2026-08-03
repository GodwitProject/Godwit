use axum::{middleware, routing::Router};
use godwit_api::{admin, model_router::DbModelRouter, proxy, state::AppState};
use godwit_cache::MemoryCache;
use godwit_core::{AppConfig, Protocol};
use godwit_db::{
    connect,
    repositories::{
        api_keys::ApiKeyRepository, organizations::OrganizationRepository,
        refresh_tokens::RefreshTokenRepository, team_memberships::TeamMembershipRepository,
        teams::TeamRepository, users::UserRepository,
    },
    run_migrations,
};
use godwit_providers::{
    anthropic::AnthropicAdapter, gemini::GeminiAdapter, llama_cpp::LlamaCppAdapter,
    ollama::OllamaAdapter, openai::OpenAiAdapter, sglang::SglangAdapter, vllm::VllmAdapter,
    AdapterRegistry,
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

    let mut registry = AdapterRegistry::new();
    registry.register(Protocol::openai(), Arc::new(OpenAiAdapter::new()));
    registry.register(Protocol::anthropic(), Arc::new(AnthropicAdapter::new()));
    registry.register(Protocol::gemini(), Arc::new(GeminiAdapter::new()));
    registry.register(Protocol::vllm(), Arc::new(VllmAdapter::new()));
    registry.register(Protocol::sglang(), Arc::new(SglangAdapter::new()));
    registry.register(Protocol::llama_cpp(), Arc::new(LlamaCppAdapter::new()));
    registry.register(Protocol::ollama(), Arc::new(OllamaAdapter::new()));

    let adapter_registry = Arc::new(registry);
    let state = Arc::new(AppState {
        config: config.clone(),
        pool: pool.clone(),
        adapter_registry: adapter_registry.clone(),
        model_router: DbModelRouter::new(pool.clone(), adapter_registry, master_key),
        user_repo: UserRepository::new(pool.clone()),
        org_repo: OrganizationRepository::new(pool.clone()),
        team_repo: TeamRepository::new(pool.clone()),
        team_membership_repo: TeamMembershipRepository::new(pool.clone()),
        api_key_repo: ApiKeyRepository::new(pool.clone()),
        refresh_token_repo: RefreshTokenRepository::new(pool.clone()),
        api_key_cache: MemoryCache::new(),
        credential_master_key: master_key,
    });

    let app = Router::new()
        .merge(proxy::router())
        .route_layer(middleware::from_fn_with_state(
            state.clone(),
            godwit_api::middleware::api_key_auth,
        ))
        .nest("/api/v1", admin::router(state.clone()))
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
