use std::sync::Arc;

use axum::{
    body::{Body, Bytes},
    extract::{Extension, Request, State},
    http::{header::AUTHORIZATION, HeaderValue, Method},
    http::{header::COOKIE, StatusCode},
    middleware::Next,
    response::Response,
};
use godwit_auth::{api_keys::verify_key, jwt::verify};
use godwit_db::models::ApiKey;

use crate::{error::ApiError, state::AppState};

pub fn extract_token(header: &str) -> Option<&str> {
    header.strip_prefix("Bearer ")
}

pub fn cookie_value<'a>(header: Option<&HeaderValue>, name: &str) -> Option<String> {
    header.and_then(|h| h.to_str().ok()).and_then(|cookies| {
        cookies.split(';').find_map(|part| {
            let mut kv = part.trim().splitn(2, '=');
            let k = kv.next()?.trim();
            let v = kv.next()?.trim();
            if k == name { Some(v.to_string()) } else { None }
        })
    })
}

pub async fn api_key_auth(
    State(state): State<Arc<AppState>>,
    mut req: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    let auth = req
        .headers()
        .get(AUTHORIZATION)
        .and_then(|h| h.to_str().ok())
        .and_then(extract_token)
        .ok_or(StatusCode::UNAUTHORIZED)?;

    // Fast path: cache lookup by raw key.
    if let Some(key) = state.api_key_cache.get(&auth.to_string()).await {
        if !key.disabled
            && key
                .expires_at
                .map(|e| e > chrono::Utc::now())
                .unwrap_or(true)
        {
            req.extensions_mut().insert(key);
            return Ok(next.run(req).await);
        }
    }

    // Fallback: database lookup by prefix.
    let prefix = godwit_auth::api_keys::extract_prefix(auth);
    let candidates = state
        .api_key_repo
        .get_by_prefix(&prefix)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let key = candidates
        .into_iter()
        .find(|k| verify_key(auth, &k.key_hash))
        .ok_or(StatusCode::UNAUTHORIZED)?;
    if key.disabled
        || key
            .expires_at
            .map(|e| e < chrono::Utc::now())
            .unwrap_or(false)
    {
        return Err(StatusCode::UNAUTHORIZED);
    }
    state
        .api_key_cache
        .insert(auth.to_string(), key.clone())
        .await;
    req.extensions_mut().insert(key);
    Ok(next.run(req).await)
}

pub async fn jwt_auth(
    State(state): State<Arc<AppState>>,
    mut req: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    // CSRF hardening: when an allowed cookie origin is configured, state-changing
    // requests must carry a matching Origin header. No-op in dev (empty origin).
    let allowed_origin = state.config.auth.allowed_cookie_origin.as_str();
    if !allowed_origin.is_empty() && is_state_changing(req.method()) {
        let origin_matches = req
            .headers()
            .get(axum::http::header::ORIGIN)
            .and_then(|h| h.to_str().ok())
            .map(|origin| origin == allowed_origin)
            .unwrap_or(false);
        if !origin_matches {
            return Err(StatusCode::FORBIDDEN);
        }
    }

    // 1. Bearer header (backward compatible)
    let token = req
        .headers()
        .get(AUTHORIZATION)
        .and_then(|h| h.to_str().ok())
        .and_then(extract_token)
        .map(str::to_string)
        // 2. httpOnly cookie fallback
        .or_else(|| cookie_value(req.headers().get(COOKIE), "godwit_access"));
    let auth = token.ok_or(StatusCode::UNAUTHORIZED)?;
    let claims =
        verify(&state.config.auth.jwt_secret, &auth).map_err(|_| StatusCode::UNAUTHORIZED)?;
    req.extensions_mut().insert(claims);
    Ok(next.run(req).await)
}

fn is_state_changing(method: &Method) -> bool {
    matches!(
        *method,
        Method::POST | Method::PUT | Method::PATCH | Method::DELETE
    )
}

async fn extract_model_from_body(body: Bytes) -> Option<String> {
    serde_json::from_slice::<serde_json::Value>(&body)
        .ok()
        .and_then(|v| v.get("model")?.as_str().map(|s| s.to_string()))
}

fn is_model_allowed(api_key: &ApiKey, model: &str) -> bool {
    api_key.allowed_models.is_empty() || api_key.allowed_models.iter().any(|m| m == model)
}

pub async fn model_scope(
    Extension(api_key): Extension<ApiKey>,
    req: Request,
    next: Next,
) -> Result<Response, ApiError> {
    let (parts, body) = req.into_parts();
    let bytes = axum::body::to_bytes(body, usize::MAX)
        .await
        .map_err(|e| ApiError::BadRequest(e.to_string()))?;

    if let Some(model) = extract_model_from_body(bytes.clone()).await {
        if !is_model_allowed(&api_key, &model) {
            return Err(ApiError::Forbidden);
        }
    }

    let req = Request::from_parts(parts, Body::from(bytes));
    Ok(next.run(req).await)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extract_bearer_token() {
        assert_eq!(
            extract_token("Bearer sk-godwit-abc123"),
            Some("sk-godwit-abc123")
        );
        assert_eq!(extract_token("Basic abc"), None);
    }

    #[tokio::test]
    async fn extract_model_from_json_body() {
        let body = Bytes::from(r#"{"model":"gpt-4o","messages":[]}"#);
        assert_eq!(extract_model_from_body(body).await, Some("gpt-4o".into()));
    }

    #[tokio::test]
    async fn extract_model_returns_none_when_missing() {
        let body = Bytes::from(r#"{"messages":[]}"#);
        assert_eq!(extract_model_from_body(body).await, None);
    }

    #[tokio::test]
    async fn extract_model_returns_none_for_non_json() {
        let body = Bytes::from("not json");
        assert_eq!(extract_model_from_body(body).await, None);
    }

    fn api_key_with_allowed_models(models: &[String]) -> ApiKey {
        ApiKey {
            id: uuid::Uuid::new_v4(),
            user_id: uuid::Uuid::new_v4(),
            team_id: None,
            organization_id: uuid::Uuid::new_v4(),
            name: "test".to_string(),
            key_prefix: "prefix".to_string(),
            key_hash: "hash".to_string(),
            scopes: vec!["chat".to_string()],
            allowed_models: models.to_vec(),
            budget_limit_usd: None,
            budget_spent_usd: rust_decimal::Decimal::ZERO,
            rate_limit_requests_per_minute: None,
            rate_limit_tokens_per_minute: None,
            expires_at: None,
            disabled: false,
            created_at: chrono::Utc::now(),
        }
    }

    #[test]
    fn empty_allowed_models_allows_anything() {
        let key = api_key_with_allowed_models(&[]);
        assert!(is_model_allowed(&key, "gpt-4o"));
        assert!(is_model_allowed(&key, "claude-sonnet"));
    }

    #[test]
    fn allowed_models_blocks_missing_model() {
        let key = api_key_with_allowed_models(&["gpt-4o".to_string()]);
        assert!(is_model_allowed(&key, "gpt-4o"));
        assert!(!is_model_allowed(&key, "claude-sonnet"));
    }
}

#[cfg(test)]
mod auth_tests {
    use super::*;
    use axum::{
        body::Body,
        http::{header::COOKIE, Request},
        middleware,
    };
    use godwit_auth::jwt::{issue, Claims};
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

    fn valid_token() -> String {
        let claims = Claims::new(Uuid::new_v4(), Uuid::new_v4(), "org_admin");
        issue("test-secret", claims, chrono::Duration::minutes(15)).unwrap()
    }

    async fn check_auth(state: Arc<AppState>, req: Request<Body>) -> StatusCode {
        use tower::ServiceExt;
        let app = axum::Router::new()
            .route("/", axum::routing::get(|| async { "ok" }))
            .route("/", axum::routing::post(|| async { "ok" }))
            .layer(axum::middleware::from_fn_with_state(state, jwt_auth));
        app.oneshot(req).await.expect("request succeeds").status()
    }

    fn set_origin(mut state: Arc<AppState>, origin: &str) -> Arc<AppState> {
        Arc::get_mut(&mut state)
            .expect("unique Arc in test")
            .config
            .auth
            .allowed_cookie_origin = origin.to_string();
        state
    }

    #[tokio::test]
    async fn jwt_auth_accepts_cookie_token_without_bearer() {
        let pool = PgPool::connect(&std::env::var("DATABASE_URL").expect("DATABASE_URL set"))
            .await
            .expect("connect to test db");
        godwit_db::MIGRATOR.run(&pool).await.expect("run migrations");
        let state = test_state(pool).await;
        let token = valid_token();
        let req = Request::builder()
            .header(COOKIE, format!("godwit_access={token}; godwit_refresh=other"))
            .body(Body::empty())
            .unwrap();
        assert_eq!(check_auth(state, req).await, StatusCode::OK);
    }

    #[tokio::test]
    async fn jwt_auth_accepts_bearer_header() {
        let pool = PgPool::connect(&std::env::var("DATABASE_URL").expect("DATABASE_URL set"))
            .await
            .expect("connect to test db");
        godwit_db::MIGRATOR.run(&pool).await.expect("run migrations");
        let state = test_state(pool).await;
        let token = valid_token();
        let req = Request::builder()
            .header(axum::http::header::AUTHORIZATION, format!("Bearer {token}"))
            .body(Body::empty())
            .unwrap();
        assert_eq!(check_auth(state, req).await, StatusCode::OK);
    }

    #[tokio::test]
    async fn jwt_auth_rejects_missing_or_invalid_token() {
        let pool = PgPool::connect(&std::env::var("DATABASE_URL").expect("DATABASE_URL set"))
            .await
            .expect("connect to test db");
        godwit_db::MIGRATOR.run(&pool).await.expect("run migrations");
        let state = test_state(pool).await;

        let no_auth = Request::builder()
            .body(Body::empty())
            .unwrap();
        assert_eq!(check_auth(state.clone(), no_auth).await, StatusCode::UNAUTHORIZED);

        let invalid_cookie = Request::builder()
            .header(COOKIE, "godwit_access=not-a-jwt")
            .body(Body::empty())
            .unwrap();
        assert_eq!(check_auth(state.clone(), invalid_cookie).await, StatusCode::UNAUTHORIZED);

        let invalid_bearer = Request::builder()
            .header(axum::http::header::AUTHORIZATION, "Bearer not-a-jwt")
            .body(Body::empty())
            .unwrap();
        assert_eq!(check_auth(state, invalid_bearer).await, StatusCode::UNAUTHORIZED);
    }

    #[test]
    fn cookie_value_parses_named_cookie() {
        let header: HeaderValue = "foo=1; godwit_access=abc123; bar=2".parse().unwrap();
        assert_eq!(cookie_value(Some(&header), "godwit_access"), Some("abc123".to_string()));
        assert_eq!(cookie_value(Some(&header), "godwit_refresh"), None);
        assert_eq!(cookie_value(None, "godwit_access"), None);
        let other: HeaderValue = "godwit_refresh=r".parse().unwrap();
        assert_eq!(cookie_value(Some(&other), "godwit_access"), None);
    }

    #[test]
    fn state_changing_method_detection() {
        assert!(is_state_changing(&Method::POST));
        assert!(is_state_changing(&Method::PUT));
        assert!(is_state_changing(&Method::PATCH));
        assert!(is_state_changing(&Method::DELETE));
        assert!(!is_state_changing(&Method::GET));
        assert!(!is_state_changing(&Method::HEAD));
        assert!(!is_state_changing(&Method::OPTIONS));
    }

    #[tokio::test]
    async fn csrf_origin_check_suppresses_when_empty_in_dev() {
        let pool = PgPool::connect(&std::env::var("DATABASE_URL").expect("DATABASE_URL set"))
            .await
            .expect("connect to test db");
        godwit_db::MIGRATOR.run(&pool).await.expect("run migrations");
        let state = test_state(pool).await; // allowed_cookie_origin is ""
        let token = valid_token();
        let req = Request::builder()
            .method(Method::POST)
            .header(axum::http::header::AUTHORIZATION, format!("Bearer {token}"))
            .body(Body::empty())
            .unwrap();
        assert_eq!(check_auth(state, req).await, StatusCode::OK);
    }

    #[tokio::test]
    async fn csrf_state_changing_with_wrong_origin_rejected() {
        let pool = PgPool::connect(&std::env::var("DATABASE_URL").expect("DATABASE_URL set"))
            .await
            .expect("connect to test db");
        godwit_db::MIGRATOR.run(&pool).await.expect("run migrations");
        let state = set_origin(test_state(pool).await, "https://app.example.com");
        let token = valid_token();
        let wrong = Request::builder()
            .method(Method::POST)
            .header(axum::http::header::AUTHORIZATION, format!("Bearer {token}"))
            .header(axum::http::header::ORIGIN, "https://evil.example.com")
            .body(Body::empty())
            .unwrap();
        assert_eq!(check_auth(state.clone(), wrong).await, StatusCode::FORBIDDEN);

        let missing = Request::builder()
            .method(Method::POST)
            .header(axum::http::header::AUTHORIZATION, format!("Bearer {token}"))
            .body(Body::empty())
            .unwrap();
        assert_eq!(check_auth(state, missing).await, StatusCode::FORBIDDEN);
    }

    #[tokio::test]
    async fn csrf_state_changing_with_matching_origin_passes() {
        let pool = PgPool::connect(&std::env::var("DATABASE_URL").expect("DATABASE_URL set"))
            .await
            .expect("connect to test db");
        godwit_db::MIGRATOR.run(&pool).await.expect("run migrations");
        let state = set_origin(test_state(pool).await, "https://app.example.com");
        let token = valid_token();
        let req = Request::builder()
            .method(Method::POST)
            .header(axum::http::header::AUTHORIZATION, format!("Bearer {token}"))
            .header(axum::http::header::ORIGIN, "https://app.example.com")
            .body(Body::empty())
            .unwrap();
        assert_eq!(check_auth(state, req).await, StatusCode::OK);
    }
}
