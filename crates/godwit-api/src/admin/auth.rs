use axum::{
    extract::{ConnectInfo, Extension, Path, Query, State},
    http::header::{HeaderMap, HeaderValue, SET_COOKIE},
    middleware::from_fn_with_state,
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

use crate::middleware::cookie_csrf;
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

#[derive(Deserialize, Default)]
pub struct RefreshRequest {
    #[serde(default)]
    refresh_token: Option<String>,
}

#[derive(Deserialize, Default)]
pub struct LogoutRequest {
    #[serde(default)]
    refresh_token: Option<String>,
}

pub fn router(state: Arc<AppState>) -> Router<Arc<AppState>> {
    let cookie_routes = Router::new()
        .route("/auth/refresh", post(refresh))
        .route("/auth/logout", post(logout))
        .route_layer(from_fn_with_state(state.clone(), cookie_csrf));

    Router::new()
        .merge(cookie_routes)
        .route("/auth/login", post(login))
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

fn refresh_token_from_cookie(headers: &HeaderMap) -> Option<String> {
    crate::middleware::cookie_value(
        headers.get(axum::http::header::COOKIE),
        "godwit_refresh",
    )
}

/// Resolve the client IP for login rate limiting. When `trust_proxy` is set, reads
/// the first entry of `X-Forwarded-For`; otherwise the real peer address from
/// `ConnectInfo`; falls back to a sentinel for environments without either.
pub fn client_ip(
    headers: &HeaderMap,
    connect_info: Option<ConnectInfo<std::net::SocketAddr>>,
    trust_proxy: bool,
) -> String {
    if trust_proxy {
        if let Some(xff) = headers.get("x-forwarded-for") {
            let v = xff.to_str().unwrap_or("").trim();
            if let Some(first) = v.split(',').next() {
                return first.trim().to_string();
            }
        }
    }
    if let Some(ConnectInfo(addr)) = connect_info {
        return addr.ip().to_string();
    }
    "unknown".to_string()
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
    headers: HeaderMap,
    connect_info: Option<ConnectInfo<std::net::SocketAddr>>,
    Json(req): Json<LoginRequest>,
) -> Result<impl IntoResponse, crate::error::ApiError> {
    let ip = client_ip(&headers, connect_info, state.config.auth.trust_proxy);
    let user = match state.user_repo.get_by_email(&req.email).await {
        Ok(u) => u,
        Err(_) => {
            if let Some(retry_after) = state.login_limiter.attempt_allowed(&ip, true) {
                return Err(crate::error::ApiError::RateLimited(Some(retry_after)));
            }
            return Err(crate::error::ApiError::Unauthorized);
        }
    };
    let password_hash = user
        .password_hash
        .as_deref()
        .ok_or(crate::error::ApiError::Unauthorized)?;
    if !verify_password(&req.password, password_hash) {
        if let Some(retry_after) = state.login_limiter.attempt_allowed(&ip, true) {
            return Err(crate::error::ApiError::RateLimited(Some(retry_after)));
        }
        return Err(crate::error::ApiError::Unauthorized);
    }
    let (set_cookie_headers, body) = issue_token_pair(&state, &user).await?;
    Ok((set_cookie_headers, body))
}

async fn refresh(
    headers: HeaderMap,
    State(state): State<Arc<AppState>>,
    body: Option<Json<RefreshRequest>>,
) -> Result<impl IntoResponse, crate::error::ApiError> {
    let body_token = body.and_then(|b| b.refresh_token.clone());
    let token = refresh_token_from_cookie(&headers).or_else(|| body_token);
    let hash = hash_refresh_token(token.as_deref().ok_or(crate::error::ApiError::Unauthorized)?);
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
    headers: HeaderMap,
    State(state): State<Arc<AppState>>,
    body: Option<Json<LogoutRequest>>,
) -> Result<impl IntoResponse, crate::error::ApiError> {
    let body_token = body.and_then(|b| b.refresh_token.clone());
    let token = refresh_token_from_cookie(&headers).or_else(|| body_token);
    let mut headers = HeaderMap::new();
    headers.append(
        SET_COOKIE,
        HeaderValue::from_str("godwit_access=; HttpOnly; Path=/; Max-Age=0")
            .map_err(|_| crate::error::ApiError::Internal)?,
    );
    headers.append(
        SET_COOKIE,
        HeaderValue::from_str("godwit_refresh=; HttpOnly; Path=/api/v1/auth; Max-Age=0")
            .map_err(|_| crate::error::ApiError::Internal)?,
    );
    let refresh_token = token.ok_or(crate::error::ApiError::Unauthorized)?;
    let hash = hash_refresh_token(&refresh_token);
    state
        .refresh_token_repo
        .delete_by_hash(&hash)
        .await
        .map_err(crate::error::ApiError::Core)?;
    Ok((headers, Json(serde_json::json!({ "logged_out": true }))))
}

pub async fn me(
    State(state): State<Arc<AppState>>,
    Extension(claims): Extension<Claims>,
) -> Result<Json<serde_json::Value>, crate::error::ApiError> {
    let user_id = uuid::Uuid::parse_str(&claims.sub)
        .ok()
        .ok_or(crate::error::ApiError::Unauthorized)?;
    let user = state
        .user_repo
        .get_by_id(user_id)
        .await
        .map_err(|_| crate::error::ApiError::Unauthorized)?;
    Ok(Json(serde_json::json!({ "user": {
        "id": user.id,
        "email": user.email,
        "role": user.role,
        "organization_id": user.organization_id,
    }})))
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
    use axum::http::StatusCode;
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
                    login_max_attempts_per_minute: 10,
                    trust_proxy: false,
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
            login_limiter: crate::login_rate_limit::LoginLimiter::new(10),
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
        assert_eq!(req.refresh_token.as_deref(), Some("abc123"));
    }

    #[test]
    fn refresh_request_deserializes_without_body_field() {
        let req: RefreshRequest = serde_json::from_str(r#"{}"#).unwrap();
        assert_eq!(req.refresh_token, None);
    }

    #[test]
    fn logout_request_deserializes() {
        let json = r#"{"refresh_token":"abc123"}"#;
        let req: LogoutRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.refresh_token.as_deref(), Some("abc123"));
    }

    #[test]
    fn logout_request_deserializes_without_body_field() {
        let req: LogoutRequest = serde_json::from_str(r#"{}"#).unwrap();
        assert_eq!(req.refresh_token, None);
    }

    #[tokio::test]
    async fn logout_response_clears_both_cookies() {
        let pool = PgPool::connect(&std::env::var("DATABASE_URL").expect("DATABASE_URL set"))
            .await
            .expect("connect to test db");
        godwit_db::MIGRATOR.run(&pool).await.expect("run migrations");
        let state = test_state(pool).await;

        use axum::response::IntoResponse;
        use axum::Json as AxumJson;
        let req = LogoutRequest {
            refresh_token: Some("any-token".to_string()),
        };
        let res = logout(HeaderMap::new(), State(state), Some(AxumJson(req)))
            .await
            .expect("logout succeeds")
            .into_response();

        let cookie_strs: Vec<String> = res
            .headers()
            .get_all(SET_COOKIE)
            .iter()
            .map(|v| v.to_str().unwrap().to_string())
            .collect();

        assert!(
            cookie_strs.iter().any(|c| c == "godwit_access=; HttpOnly; Path=/; Max-Age=0"),
            "access clear cookie missing: {cookie_strs:?}"
        );
        assert!(
            cookie_strs.iter().any(|c| c == "godwit_refresh=; HttpOnly; Path=/api/v1/auth; Max-Age=0"),
            "refresh clear cookie missing: {cookie_strs:?}"
        );
    }

    async fn issue_cookie_for_user(
        state: &AppState,
        user: &User,
    ) -> String {
        let (headers, _body) = issue_token_pair(state, user).await.expect("issue tokens");
        let access = headers
            .get_all(SET_COOKIE)
            .iter()
            .map(|v| v.to_str().unwrap().to_string())
            .find(|c| c.starts_with("godwit_access="))
            .expect("godwit_access Set-Cookie present");
        access.split(';').next().unwrap().to_string()
    }

    async fn me_request(
        state: Arc<AppState>,
        cookie: Option<&str>,
    ) -> (StatusCode, serde_json::Value) {
        use axum::body::Body;
        use axum::http::Request;
        use tower::ServiceExt;

        let app = axum::Router::new()
            .nest("/api/v1", crate::admin::router(state.clone()))
            .with_state(state);
        let mut builder = Request::builder().uri("/api/v1/auth/me");
        if let Some(cookie) = cookie {
            builder = builder.header(axum::http::header::COOKIE, cookie);
        }
        let res = app
            .oneshot(builder.body(Body::empty()).unwrap())
            .await
            .expect("request succeeds");
        let status = res.status();
        let bytes = axum::body::to_bytes(res.into_body(), usize::MAX)
            .await
            .expect("read body");
        let json = serde_json::from_slice(&bytes).unwrap_or(serde_json::Value::Null);
        (status, json)
    }

    #[tokio::test]
    async fn me_with_valid_cookie_returns_current_user() {
        let pool = PgPool::connect(&std::env::var("DATABASE_URL").expect("DATABASE_URL set"))
            .await
            .expect("connect to test db");
        godwit_db::MIGRATOR.run(&pool).await.expect("run migrations");
        let state = test_state(pool).await;

        let org = state.org_repo.create("me-test-org", None).await.expect("create org");
        let unique = Uuid::new_v4().to_string();
        let user = state
            .user_repo
            .create(
                &format!("me-test-{unique}@example.com"),
                None,
                godwit_db::models::UserRole::OrgAdmin,
                Some(org.id),
            )
            .await
            .expect("create user");

        let cookie = issue_cookie_for_user(&state, &user).await;
        let (status, json) = me_request(state, Some(&cookie)).await;

        assert_eq!(status, StatusCode::OK);
        let user_json = json.get("user").expect("user key present");
        assert_eq!(user_json["id"], serde_json::json!(user.id));
        assert_eq!(user_json["email"], serde_json::json!(user.email));
        assert_eq!(user_json["role"], serde_json::json!(user.role));
        assert_eq!(
            user_json["organization_id"],
            serde_json::json!(user.organization_id)
        );
    }

    #[tokio::test]
    async fn me_without_auth_returns_401() {
        let pool = PgPool::connect(&std::env::var("DATABASE_URL").expect("DATABASE_URL set"))
            .await
            .expect("connect to test db");
        godwit_db::MIGRATOR.run(&pool).await.expect("run migrations");
        let state = test_state(pool).await;

        let (status, _json) = me_request(state, None).await;
        assert_eq!(status, StatusCode::UNAUTHORIZED);
    }
}
