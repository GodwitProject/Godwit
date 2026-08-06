use axum::{
    extract::{Path, Query, State},
    http::header::{HeaderMap, HeaderValue, SET_COOKIE},
    response::{IntoResponse, Redirect, Response},
    routing::{get, post},
    Json, Router,
};
use godwit_auth::{
    api_keys::verify_password,
    jwt::{issue, Claims},
    refresh_tokens::{generate_refresh_token, hash_refresh_token},
};
use godwit_db::models::User;
use serde::Deserialize;
use std::sync::Arc;

use crate::state::AppState;

#[derive(Deserialize)]
pub struct LoginRequest {
    email: String,
    password: String,
}

#[derive(Deserialize)]
pub struct OidcCallback {
    code: String,
    state: String,
}

#[derive(Deserialize)]
pub struct RefreshRequest {
    refresh_token: String,
}

#[derive(Deserialize)]
pub struct LogoutRequest {
    refresh_token: String,
}

pub fn router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/auth/login", post(login))
        .route("/auth/refresh", post(refresh))
        .route("/auth/logout", post(logout))
        .route("/auth/oidc/:provider", get(oidc_start))
        .route("/auth/oidc/:provider/callback", get(oidc_callback))
        .route("/auth/saml/:provider/acs", post(saml_acs))
}

/// Issues a fresh access token + refresh token pair for `user`, persisting the refresh
/// token's hash. Shared by login, the OIDC callback, and `/auth/refresh` so all three
/// issue tokens identically.
fn access_cookie(state: &AppState, token: &str) -> String {
    let secure = if state.config.auth.cookie_secure { "; Secure" } else { "" };
    let max_age = state.config.auth.access_token_ttl_minutes * 60;
    format!(
        "godwit_access={}; HttpOnly; Path=/; SameSite=Strict; Max-Age={}{}",
        token, max_age, secure
    )
}

fn refresh_cookie(state: &AppState, token: &str) -> String {
    let secure = if state.config.auth.cookie_secure { "; Secure" } else { "" };
    let max_age = state.config.auth.refresh_token_ttl_days * 86400;
    format!(
        "godwit_refresh={}; HttpOnly; Path=/api/v1/auth; SameSite=Strict; Max-Age={}{}",
        token, max_age, secure
    )
}

async fn issue_token_pair(
    state: &AppState,
    user: &User,
) -> Result<(HeaderMap, Json<serde_json::Value>), crate::error::ApiError> {
    let claims = Claims::new(user.id, user.organization_id.unwrap_or_default(), &user.role);
    let access_token = issue(
        &state.config.auth.jwt_secret,
        claims,
        chrono::Duration::minutes(state.config.auth.access_token_ttl_minutes),
    )
    .map_err(|_| crate::error::ApiError::Internal)?;

    let (refresh_plaintext, refresh_hash) = generate_refresh_token();
    let expires_at =
        chrono::Utc::now() + chrono::Duration::days(state.config.auth.refresh_token_ttl_days);
    state
        .refresh_token_repo
        .create(user.id, &refresh_hash, expires_at)
        .await
        .map_err(crate::error::ApiError::Core)?;

    let mut headers = HeaderMap::new();
    headers.append(
        SET_COOKIE,
        HeaderValue::from_str(&access_cookie(state, &access_token))
            .map_err(|_| crate::error::ApiError::Internal)?,
    );
    headers.append(
        SET_COOKIE,
        HeaderValue::from_str(&refresh_cookie(state, &refresh_plaintext))
            .map_err(|_| crate::error::ApiError::Internal)?,
    );

    let body = serde_json::json!({ "access_token": access_token, "refresh_token": refresh_plaintext });
    Ok((headers, Json(body)))
}

async fn login(
    State(state): State<Arc<AppState>>,
    Json(req): Json<LoginRequest>,
) -> Result<impl IntoResponse, crate::error::ApiError> {
    let user = state
        .user_repo
        .get_by_email(&req.email)
        .await
        .map_err(|_| crate::error::ApiError::Unauthorized)?;
    let password_hash = user
        .password_hash
        .as_deref()
        .ok_or(crate::error::ApiError::Unauthorized)?;
    if !verify_password(&req.password, password_hash) {
        return Err(crate::error::ApiError::Unauthorized);
    }
    let (headers, body) = issue_token_pair(&state, &user).await?;
    Ok((headers, body))
}

async fn refresh(
    State(state): State<Arc<AppState>>,
    Json(req): Json<RefreshRequest>,
) -> Result<impl IntoResponse, crate::error::ApiError> {
    let hash = hash_refresh_token(&req.refresh_token);
    let stored = state
        .refresh_token_repo
        .get_by_hash(&hash)
        .await
        .map_err(|_| crate::error::ApiError::Unauthorized)?;
    if stored.expires_at < chrono::Utc::now() {
        let _ = state.refresh_token_repo.delete(stored.id).await;
        return Err(crate::error::ApiError::Unauthorized);
    }
    let user = state
        .user_repo
        .get_by_id(stored.user_id)
        .await
        .map_err(|_| crate::error::ApiError::Unauthorized)?;
    // Rotate: the used refresh token is single-use.
    state
        .refresh_token_repo
        .delete(stored.id)
        .await
        .map_err(crate::error::ApiError::Core)?;
    let (headers, body) = issue_token_pair(&state, &user).await?;
    Ok((headers, body))
}

async fn logout(
    State(state): State<Arc<AppState>>,
    Json(req): Json<LogoutRequest>,
) -> Result<impl IntoResponse, crate::error::ApiError> {
    let hash = hash_refresh_token(&req.refresh_token);
    state
        .refresh_token_repo
        .delete_by_hash(&hash)
        .await
        .map_err(crate::error::ApiError::Core)?;
    Ok(Json(serde_json::json!({ "logged_out": true })))
}

async fn oidc_start(
    State(state): State<Arc<AppState>>,
    Path(provider_id): Path<String>,
) -> Result<impl IntoResponse, crate::error::ApiError> {
    let config = state
        .config
        .auth
        .oidc_providers
        .iter()
        .find(|p| p.id == provider_id)
        .cloned()
        .ok_or(crate::error::ApiError::NotFound)?;
    let client = godwit_auth::oidc::OidcClient::new(&config)
        .await
        .map_err(|_| crate::error::ApiError::Internal)?;
    let (url, _csrf, _nonce) = client.authorize_url(vec![
        "openid".to_string(),
        "email".to_string(),
        "profile".to_string(),
    ]);
    Ok(Redirect::temporary(url.as_str()))
}

async fn oidc_callback(
    State(state): State<Arc<AppState>>,
    Path(provider_id): Path<String>,
    Query(params): Query<OidcCallback>,
) -> Result<impl IntoResponse, crate::error::ApiError> {
    let config = state
        .config
        .auth
        .oidc_providers
        .iter()
        .find(|p| p.id == provider_id)
        .cloned()
        .ok_or(crate::error::ApiError::NotFound)?;
    let client = godwit_auth::oidc::OidcClient::new(&config)
        .await
        .map_err(|_| crate::error::ApiError::Internal)?;
    let (email, _subject, name) = client
        .exchange_code(&params.code, &params.state, "nonce")
        .await
        .map_err(|_| crate::error::ApiError::Unauthorized)?;
    let user = match state.user_repo.get_by_email(&email).await {
        Ok(u) => u,
        Err(_) => state
            .user_repo
            .create(
                &email,
                name.as_deref(),
                godwit_db::models::UserRole::User,
                None,
            )
            .await
            .map_err(|_| crate::error::ApiError::Internal)?,
    };
    Ok(issue_token_pair(&state, &user).await?)
}

async fn saml_acs(
    State(_state): State<Arc<AppState>>,
    Path(_provider_id): Path<String>,
) -> Result<Response, crate::error::ApiError> {
    Err(crate::error::ApiError::BadRequest(
        "SAML ACS requires XML signature validation; implement with real IdP metadata".to_string(),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::header::SET_COOKIE;
    use sqlx::PgPool;
    use uuid::Uuid;

    async fn test_state(pool: PgPool) -> Arc<AppState> {
        use crate::agentic_loop::AgenticLoop;
        use crate::circuit_breaker::CircuitBreakerRegistry;
        use crate::model_router::DbModelRouter;
        use godwit_cache::MemoryCache;
        use godwit_db::repositories::{
            api_keys::ApiKeyRepository, end_users::EndUsersRepository,
            organizations::OrganizationRepository, refresh_tokens::RefreshTokenRepository,
            team_memberships::TeamMembershipRepository, teams::TeamRepository, users::UserRepository,
        };
        use godwit_mcp::McpRegistry;
        use godwit_providers::{
            anthropic::AnthropicAdapter, gemini::GeminiAdapter, llama_cpp::LlamaCppAdapter,
            ollama::OllamaAdapter, openai::OpenAiAdapter, sglang::SglangAdapter, vllm::VllmAdapter,
            AdapterRegistry,
        };
        use godwit_core::{AgenticConfig, AppConfig, AuthConfig, DatabaseConfig, ServerConfig};

        let mut registry = AdapterRegistry::new();
        registry.register(godwit_core::Protocol::openai(), Arc::new(OpenAiAdapter::new()));
        registry.register(godwit_core::Protocol::anthropic(), Arc::new(AnthropicAdapter::new()));
        registry.register(godwit_core::Protocol::gemini(), Arc::new(GeminiAdapter::new()));
        registry.register(godwit_core::Protocol::vllm(), Arc::new(VllmAdapter::new()));
        registry.register(godwit_core::Protocol::sglang(), Arc::new(SglangAdapter::new()));
        registry.register(godwit_core::Protocol::llama_cpp(), Arc::new(LlamaCppAdapter::new()));
        registry.register(godwit_core::Protocol::ollama(), Arc::new(OllamaAdapter::new()));

        Arc::new(AppState {
            config: AppConfig {
                server: ServerConfig {
                    host: "127.0.0.1".to_string(),
                    port: 0,
                    request_timeout_seconds: 30,
                },
                database: DatabaseConfig {
                    url: "postgres://unused".to_string(),
                },
                auth: AuthConfig {
                    jwt_secret: "test-secret".to_string(),
                    access_token_ttl_minutes: 15,
                    refresh_token_ttl_days: 7,
                    cookie_secure: false,
                    allowed_cookie_origin: "".to_string(),
                    oidc_providers: vec![],
                    saml_providers: vec![],
                },
                agentic: AgenticConfig::default(),
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
            },
            pool: pool.clone(),
            adapter_registry: Arc::new(registry),
            model_router: DbModelRouter::new(pool.clone(), Arc::new(AdapterRegistry::new()), [42u8; 32]),
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
            credential_master_key: [42u8; 32],
            rate_limiter: crate::rate_limit::RateLimiter::new(),
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

    #[tokio::test]
    async fn issue_token_pair_sets_http_only_cookies() {
        let pool = PgPool::connect(&std::env::var("DATABASE_URL").expect("DATABASE_URL set"))
            .await
            .expect("connect to test db");
        godwit_db::MIGRATOR.run(&pool).await.expect("run migrations");
        let state = test_state(pool).await;

        let org = state.org_repo.create("cookie-test-org", None).await.expect("create org");
        let unique = Uuid::new_v4().to_string();
        let user = state
            .user_repo
            .create(
                &format!("cookie-test-{unique}@example.com"),
                None,
                godwit_db::models::UserRole::User,
                Some(org.id),
            )
            .await
            .expect("create user");

        let (headers, _body) = issue_token_pair(&state, &user).await.expect("issue tokens");

        let cookie_strs: Vec<String> = headers
            .get_all(SET_COOKIE)
            .iter()
            .map(|v| v.to_str().unwrap().to_string())
            .collect();

        let access = cookie_strs
            .iter()
            .find(|c| c.starts_with("godwit_access="))
            .expect("godwit_access Set-Cookie present");
        let refresh = cookie_strs
            .iter()
            .find(|c| c.starts_with("godwit_refresh="))
            .expect("godwit_refresh Set-Cookie present");

        assert!(access.contains("HttpOnly"), "access cookie missing HttpOnly: {access}");
        assert!(access.contains("SameSite=Strict"), "access cookie missing SameSite=Strict: {access}");
        assert!(refresh.contains("HttpOnly"), "refresh cookie missing HttpOnly: {refresh}");
        assert!(refresh.contains("SameSite=Strict"), "refresh cookie missing SameSite=Strict: {refresh}");
    }

    #[test]
    fn login_request_deserializes() {
        let json = r#"{"email":"a@b.com","password":"secret"}"#;
        let req: LoginRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.email, "a@b.com");
        assert_eq!(req.password, "secret");
    }

    #[test]
    fn refresh_request_deserializes() {
        let json = r#"{"refresh_token":"abc123"}"#;
        let req: RefreshRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.refresh_token, "abc123");
    }

    #[test]
    fn logout_request_deserializes() {
        let json = r#"{"refresh_token":"abc123"}"#;
        let req: LogoutRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.refresh_token, "abc123");
    }
}
