use axum::{middleware, routing::Router};
use godwit_api::{admin, model_router::DbModelRouter, proxy, state::AppState};
use godwit_cache::MemoryCache;
use godwit_core::{AppConfig, Protocol};
use godwit_db::{
    connect,
    repositories::{
        api_keys::ApiKeyRepository, organizations::OrganizationRepository, users::UserRepository,
    },
    run_migrations,
};
use godwit_providers::{openai::OpenAiAdapter, AdapterRegistry};
use std::sync::Arc;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();

    let config: AppConfig = load_config()?;
    let pool = connect(&config.database.url).await?;
    run_migrations(&pool).await?;

    let mut registry = AdapterRegistry::new();
    registry.register(
        Protocol::openai(),
        Arc::new(OpenAiAdapter::new(
            &config.providers.openai.api_key,
            &config.providers.openai.base_url,
        )),
    );
    // Anthropic adapter will be added in Lot 3; for now register OpenAI for both
    // protocols to keep the workspace compiling.
    registry.register(
        Protocol::anthropic(),
        Arc::new(OpenAiAdapter::new(
            &config.providers.anthropic.api_key,
            &config.providers.anthropic.base_url,
        )),
    );

    let adapter_registry = Arc::new(registry);
    let state = Arc::new(AppState {
        config: config.clone(),
        pool: pool.clone(),
        adapter_registry: adapter_registry.clone(),
        model_router: DbModelRouter::new(pool.clone(), adapter_registry),
        user_repo: UserRepository::new(pool.clone()),
        org_repo: OrganizationRepository::new(pool.clone()),
        api_key_repo: ApiKeyRepository::new(pool.clone()),
        api_key_cache: MemoryCache::new(),
        model_cache: MemoryCache::new(),
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
