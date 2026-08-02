use axum::{middleware, routing::Router};
use godwit_api::{
    admin, proxy,
    state::{AppState, ProviderRouter},
};
use godwit_cache::MemoryCache;
use godwit_core::AppConfig;
use godwit_db::{
    connect,
    repositories::{
        api_keys::ApiKeyRepository, organizations::OrganizationRepository, users::UserRepository,
    },
    run_migrations,
};
use godwit_providers::{anthropic::AnthropicProvider, openai::OpenAiProvider, Provider};
use std::sync::Arc;

pub struct SimpleProviderRouter {
    openai: Arc<dyn Provider>,
    anthropic: Arc<dyn Provider>,
}

impl SimpleProviderRouter {
    pub fn new(providers: &godwit_core::ProvidersConfig) -> Self {
        Self {
            openai: Arc::new(OpenAiProvider::from_config(&providers.openai)),
            anthropic: Arc::new(AnthropicProvider::new(
                &providers.anthropic.api_key,
                &providers.anthropic.base_url,
            )),
        }
    }
}

#[async_trait::async_trait]
impl ProviderRouter for SimpleProviderRouter {
    async fn route(
        &self,
        _organization_id: uuid::Uuid,
        provider_model_id: &str,
    ) -> Option<Arc<dyn Provider>> {
        if provider_model_id.starts_with("claude") {
            Some(self.anthropic.clone())
        } else {
            Some(self.openai.clone())
        }
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();

    let config: AppConfig = load_config()?;
    let pool = connect(&config.database.url).await?;
    run_migrations(&pool).await?;

    let state = Arc::new(AppState {
        config: config.clone(),
        pool: pool.clone(),
        provider_router: Arc::new(SimpleProviderRouter::new(&config.providers)),
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
