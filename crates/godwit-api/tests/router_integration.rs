//! In-process HTTP integration tests.
//!
//! Everything else in this workspace tests either a single function or a single adapter
//! against wiremock. Nothing exercised an actual axum route through the actual assembled
//! `Router` — the only route-level tests were `#[ignore]`d smoke tests needing a live
//! server on localhost:3000, so they had never run. That gap is why two Critical bugs
//! (`models.provider`'s CHECK constraint rejecting five of seven protocols, and adapters
//! never reading `provider_model_id`) plus a double-nested admin route survived 22
//! individual task reviews.
//!
//! These tests assemble the same `AppState` + `Router` that `godwit-bin`'s `main` does,
//! then drive it with `tower::ServiceExt::oneshot` — no TCP listener, no live server. A
//! `wiremock::MockServer` stands in for the upstream provider.

use axum::{
    body::Body,
    http::{Request, StatusCode},
    middleware, Router,
};
use godwit_api::{admin, model_router::DbModelRouter, proxy, state::AppState};
use godwit_cache::MemoryCache;
use godwit_core::{AppConfig, AuthConfig, DatabaseConfig, Protocol, ServerConfig};
use godwit_db::models::UserRole;
use godwit_db::repositories::{
    api_keys::ApiKeyRepository, models::ModelRepository, organizations::OrganizationRepository,
    provider_profiles::ProviderProfileRepository, refresh_tokens::RefreshTokenRepository,
    team_memberships::TeamMembershipRepository, teams::TeamRepository, users::UserRepository,
};
use godwit_providers::{
    anthropic::AnthropicAdapter, gemini::GeminiAdapter, llama_cpp::LlamaCppAdapter,
    ollama::OllamaAdapter, openai::OpenAiAdapter, sglang::SglangAdapter, vllm::VllmAdapter,
    AdapterRegistry,
};
use sqlx::PgPool;
use std::sync::Arc;
use tower::ServiceExt;
use uuid::Uuid;
use wiremock::{
    matchers::{method, path},
    Mock, MockServer, ResponseTemplate,
};

const JWT_SECRET: &str = "test-jwt-secret";
const MASTER_KEY: [u8; 32] = [42u8; 32];

fn test_config() -> AppConfig {
    AppConfig {
        server: ServerConfig {
            host: "127.0.0.1".to_string(),
            port: 0,
            request_timeout_seconds: 30,
        },
        database: DatabaseConfig {
            url: "postgres://unused".to_string(),
        },
        auth: AuthConfig {
            jwt_secret: JWT_SECRET.to_string(),
            access_token_ttl_minutes: 15,
            refresh_token_ttl_days: 7,
            oidc_providers: vec![],
            saml_providers: vec![],
        },
    }
}

fn test_registry() -> Arc<AdapterRegistry> {
    let mut registry = AdapterRegistry::new();
    registry.register(Protocol::openai(), Arc::new(OpenAiAdapter::new()));
    registry.register(Protocol::anthropic(), Arc::new(AnthropicAdapter::new()));
    registry.register(Protocol::gemini(), Arc::new(GeminiAdapter::new()));
    registry.register(Protocol::vllm(), Arc::new(VllmAdapter::new()));
    registry.register(Protocol::sglang(), Arc::new(SglangAdapter::new()));
    registry.register(Protocol::llama_cpp(), Arc::new(LlamaCppAdapter::new()));
    registry.register(Protocol::ollama(), Arc::new(OllamaAdapter::new()));
    Arc::new(registry)
}

/// Mirrors `godwit-bin/src/main.rs`'s state + router assembly, so these tests exercise the
/// real wiring (including the two auth middlewares and the `/api/v1` admin nesting) rather
/// than a hand-rolled approximation.
fn build_app(pool: PgPool) -> Router {
    let registry = test_registry();
    let state = Arc::new(AppState {
        config: test_config(),
        pool: pool.clone(),
        adapter_registry: registry.clone(),
        model_router: DbModelRouter::new(pool.clone(), registry, MASTER_KEY),
        user_repo: UserRepository::new(pool.clone()),
        org_repo: OrganizationRepository::new(pool.clone()),
        team_repo: TeamRepository::new(pool.clone()),
        team_membership_repo: TeamMembershipRepository::new(pool.clone()),
        api_key_repo: ApiKeyRepository::new(pool.clone()),
        refresh_token_repo: RefreshTokenRepository::new(pool.clone()),
        api_key_cache: MemoryCache::new(),
        credential_master_key: MASTER_KEY,
    });

    Router::new()
        .merge(proxy::router())
        .route_layer(middleware::from_fn_with_state(
            state.clone(),
            godwit_api::middleware::api_key_auth,
        ))
        .nest("/api/v1", admin::router(state.clone()))
        .with_state(state)
}

/// Creates an organization, a user, and a usable proxy API key; returns the plaintext key.
async fn seed_api_key(pool: &PgPool) -> String {
    let org = OrganizationRepository::new(pool.clone())
        .create("test-org", None)
        .await
        .expect("create org");
    let user = UserRepository::new(pool.clone())
        .create("proxy-user@example.com", None, UserRole::User, Some(org.id))
        .await
        .expect("create user");

    let (plaintext, hash, prefix) = godwit_auth::api_keys::generate_api_key();
    ApiKeyRepository::new(pool.clone())
        .create(
            user.id,
            org.id,
            "test-key",
            &prefix,
            &hash,
            &["chat".to_string()],
            None,
            None,
        )
        .await
        .expect("create api key");
    plaintext
}

/// Creates an organization and a user with a real (Argon2) password hash set, so the user
/// can authenticate through the real `POST /auth/login` endpoint. Returns `(email, password)`.
async fn seed_password_user(pool: &PgPool) -> (String, String) {
    let org = OrganizationRepository::new(pool.clone())
        .create("auth-test-org", None)
        .await
        .expect("create org");
    let email = "auth-user@example.com";
    let user = UserRepository::new(pool.clone())
        .create(email, None, UserRole::User, Some(org.id))
        .await
        .expect("create user");
    let password = "correct-horse-battery-staple";
    let hash = godwit_auth::api_keys::hash_password(password);
    sqlx::query("UPDATE users SET password_hash = $1 WHERE id = $2")
        .bind(&hash)
        .bind(user.id)
        .execute(pool)
        .await
        .expect("set password hash");
    (email.to_string(), password.to_string())
}

/// Issues an admin JWT for the given role without going through the login endpoint.
fn admin_token(role: &str) -> String {
    let claims = godwit_auth::jwt::Claims::new(Uuid::new_v4(), Uuid::new_v4(), role);
    godwit_auth::jwt::issue(JWT_SECRET, claims, chrono::Duration::minutes(15)).expect("issue jwt")
}

/// Issues an admin JWT for the given role, scoped to a specific organization — needed for
/// the teams RBAC tests, where the caller's `organization_id` (not just its role) determines
/// what it can see and modify.
fn admin_token_for_org(role: &str, organization_id: Uuid) -> String {
    let claims = godwit_auth::jwt::Claims::new(Uuid::new_v4(), organization_id, role);
    godwit_auth::jwt::issue(JWT_SECRET, claims, chrono::Duration::minutes(15)).expect("issue jwt")
}

/// Issues an admin JWT for a *specific* `user_id` — needed for the team membership RBAC
/// tests, where a `team_admin`'s authority comes from a `team_memberships` row keyed on
/// their exact `user_id`, not just their global role or org.
fn admin_token_for_user(role: &str, organization_id: Uuid, user_id: Uuid) -> String {
    let claims = godwit_auth::jwt::Claims::new(user_id, organization_id, role);
    godwit_auth::jwt::issue(JWT_SECRET, claims, chrono::Duration::minutes(15)).expect("issue jwt")
}

async fn body_json(response: axum::response::Response) -> serde_json::Value {
    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("read response body");
    if bytes.is_empty() {
        return serde_json::Value::Null;
    }
    serde_json::from_slice(&bytes).unwrap_or_else(|e| {
        panic!(
            "body was not JSON ({e}): {}",
            String::from_utf8_lossy(&bytes)
        )
    })
}

// ---------------------------------------------------------------------------------------
// 1. Wildcard chat completion, end to end.
// ---------------------------------------------------------------------------------------

/// A wildcard-resolved request synthesises a `Model` whose `public_id` is the whole
/// `<profile>/<suffix>` ref and whose `provider_model_id` is just the suffix. Before the
/// fix, no adapter read `provider_model_id`, so the client's `request.model` — the full
/// prefixed string — went upstream verbatim and every wildcard call would have failed at
/// the provider. This is the regression guard for that, driven through the real route.
#[sqlx::test(migrations = "../godwit-db/migrations")]
async fn wildcard_chat_completion_sends_bare_upstream_model_id(pool: PgPool) {
    let upstream = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/chat/completions"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "id": "chatcmpl-abc",
            "object": "chat.completion",
            "created": 1,
            "model": "gpt-4o-mini-2024-07-18",
            "choices": [{
                "index": 0,
                "message": {"role": "assistant", "content": "Hello from upstream"},
                "finish_reason": "stop"
            }],
            "usage": {"prompt_tokens": 1, "completion_tokens": 2, "total_tokens": 3}
        })))
        .mount(&upstream)
        .await;

    // A wildcard-enabled profile pointing at the mock upstream. No models row exists.
    let profiles = ProviderProfileRepository::new(pool.clone());
    let profile = profiles
        .create("upstream", "openai", Some(&upstream.uri()), true)
        .await
        .expect("create wildcard profile");
    let secret = godwit_auth::credentials::encrypt_api_key(&MASTER_KEY, "sk-upstream");
    profiles
        .set_auth(profile.id, &secret)
        .await
        .expect("set auth");

    let api_key = seed_api_key(&pool).await;
    let app = build_app(pool);

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/v1/chat/completions")
                .header("Authorization", format!("Bearer {api_key}"))
                .header("Content-Type", "application/json")
                .body(Body::from(
                    serde_json::json!({
                        "model": "upstream/gpt-4o-mini-2024-07-18",
                        "messages": [{"role": "user", "content": "Hi"}],
                        "stream": false
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .expect("route response");

    let status = response.status();
    let body = body_json(response).await;
    assert_eq!(status, StatusCode::OK, "body={body}");
    assert_eq!(
        body["choices"][0]["message"]["content"],
        "Hello from upstream"
    );

    // The upstream must have received the *suffix* only.
    let received = upstream
        .received_requests()
        .await
        .expect("request recording enabled");
    assert_eq!(
        received.len(),
        1,
        "the upstream should have been called once"
    );
    let upstream_body: serde_json::Value =
        serde_json::from_slice(&received[0].body).expect("upstream body is JSON");
    assert_eq!(
        upstream_body["model"], "gpt-4o-mini-2024-07-18",
        "the adapter must send provider_model_id (the bare suffix) upstream"
    );
    assert_ne!(
        upstream_body["model"], "upstream/gpt-4o-mini-2024-07-18",
        "the profile-prefixed model_ref must never be forwarded upstream"
    );
    // The Authorization header must carry the decrypted profile credential, not the
    // caller's own Godwit API key.
    assert_eq!(
        received[0]
            .headers
            .get("authorization")
            .map(|v| v.to_str().unwrap()),
        Some("Bearer sk-upstream")
    );
}

/// The keyless counterpart (Important Fix 4): a self-hosted profile with no stored
/// credentials must serve traffic and send no Authorization header, rather than 500ing
/// with "no credentials configured".
#[sqlx::test(migrations = "../godwit-db/migrations")]
async fn keyless_self_hosted_profile_serves_chat_without_auth_header(pool: PgPool) {
    let upstream = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/chat/completions"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "id": "chatcmpl-local",
            "object": "chat.completion",
            "created": 1,
            "model": "meta-llama/Llama-3-70B-Instruct",
            "choices": [{
                "index": 0,
                "message": {"role": "assistant", "content": "Local reply"},
                "finish_reason": "stop"
            }],
            "usage": {"prompt_tokens": 1, "completion_tokens": 1, "total_tokens": 2}
        })))
        .mount(&upstream)
        .await;

    // No set_auth call: this profile deliberately has no credentials.
    ProviderProfileRepository::new(pool.clone())
        .create("local", "vllm", Some(&upstream.uri()), true)
        .await
        .expect("create keyless wildcard profile");

    let api_key = seed_api_key(&pool).await;
    let app = build_app(pool);

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/v1/chat/completions")
                .header("Authorization", format!("Bearer {api_key}"))
                .header("Content-Type", "application/json")
                .body(Body::from(
                    serde_json::json!({
                        "model": "local/meta-llama/Llama-3-70B-Instruct",
                        "messages": [{"role": "user", "content": "Hi"}],
                        "stream": false
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .expect("route response");

    let status = response.status();
    let body = body_json(response).await;
    assert_eq!(
        status,
        StatusCode::OK,
        "a profile with no credentials must be usable, body={body}"
    );

    let received = upstream
        .received_requests()
        .await
        .expect("request recording enabled");
    assert!(
        received[0].headers.get("authorization").is_none(),
        "no Authorization header should be sent when the profile has no api_key"
    );
    let upstream_body: serde_json::Value =
        serde_json::from_slice(&received[0].body).expect("upstream body is JSON");
    assert_eq!(upstream_body["model"], "meta-llama/Llama-3-70B-Instruct");
}

// ---------------------------------------------------------------------------------------
// 2. Admin model creation for a non-openai/anthropic protocol.
// ---------------------------------------------------------------------------------------

/// Catches regressions of BOTH Critical Fix 1 (a `models.provider` CHECK constraint that
/// only allowed 'openai'/'anthropic' would make this 500) and Important Fix 5 (a
/// double-nested admin router would make this 404 at `/api/v1/models`).
#[sqlx::test(migrations = "../godwit-db/migrations")]
async fn super_admin_can_create_a_vllm_backed_catalog_model(pool: PgPool) {
    let profile = ProviderProfileRepository::new(pool.clone())
        .create(
            "local-vllm",
            "vllm",
            Some("http://localhost:8000/v1"),
            false,
        )
        .await
        .expect("create profile");

    let token = admin_token("super_admin");
    let app = build_app(pool.clone());

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/models")
                .header("Authorization", format!("Bearer {token}"))
                .header("Content-Type", "application/json")
                .body(Body::from(
                    serde_json::json!({
                        "public_id": "llama-3-70b",
                        "provider": "vllm",
                        "provider_profile_id": profile.id,
                        "provider_model_id": "meta-llama/Llama-3-70B-Instruct",
                        "capabilities": "chat,embedding"
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .expect("route response");

    let status = response.status();
    let body = body_json(response).await;
    assert!(
        status.is_success(),
        "POST /api/v1/models must succeed for provider 'vllm', got {status}: {body}"
    );
    assert_eq!(body["data"]["public_id"], "llama-3-70b");
    assert_eq!(body["data"]["provider"], "vllm");
    assert_eq!(
        body["data"]["provider_model_id"],
        "meta-llama/Llama-3-70B-Instruct"
    );

    // The row really landed in the database.
    let models = ModelRepository::new(pool)
        .list()
        .await
        .expect("list models");
    assert_eq!(models.len(), 1);
    assert_eq!(models[0].provider, "vllm");
}

/// The documented admin paths must be the real ones. Before Fix 5, `models::router()` was
/// nested under an extra "/models", so the real paths were `/api/v1/models/models`.
#[sqlx::test(migrations = "../godwit-db/migrations")]
async fn admin_catalog_routes_are_not_double_nested(pool: PgPool) {
    let token = admin_token("super_admin");

    for uri in [
        "/api/v1/models",
        "/api/v1/provider-profiles",
        "/api/v1/teams",
    ] {
        let response = build_app(pool.clone())
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri(uri)
                    .header("Authorization", format!("Bearer {token}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .expect("route response");
        assert_eq!(
            response.status(),
            StatusCode::OK,
            "{uri} should be the real admin path"
        );
    }

    // ...and the doubled paths must NOT serve the listing. (They are not 404: the doubled
    // segment is swallowed by the `:id` route, which has no GET handler, so axum answers
    // 405. What matters is that they no longer *succeed*, which they did before the fix.)
    for uri in [
        "/api/v1/models/models",
        "/api/v1/provider-profiles/provider-profiles",
        "/api/v1/teams/teams",
    ] {
        let response = build_app(pool.clone())
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri(uri)
                    .header("Authorization", format!("Bearer {token}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .expect("route response");
        assert!(
            !response.status().is_success(),
            "{uri} should not serve the listing, got {}",
            response.status()
        );
    }
}

/// `/api/v1/models/:id` and `/api/v1/provider-profiles/:id` are reachable at their
/// documented (single-prefix) paths too.
#[sqlx::test(migrations = "../godwit-db/migrations")]
async fn admin_by_id_routes_are_reachable(pool: PgPool) {
    let profiles = ProviderProfileRepository::new(pool.clone());
    let profile = profiles
        .create(
            "local-vllm",
            "vllm",
            Some("http://localhost:8000/v1"),
            false,
        )
        .await
        .expect("create profile");
    let model = ModelRepository::new(pool.clone())
        .create(
            "llama-3-70b",
            "vllm",
            profile.id,
            "meta-llama/Llama-3-70B-Instruct",
            "chat",
        )
        .await
        .expect("create model");

    let token = admin_token("super_admin");

    let response = build_app(pool.clone())
        .oneshot(
            Request::builder()
                .method("PATCH")
                .uri(format!("/api/v1/provider-profiles/{}", profile.id))
                .header("Authorization", format!("Bearer {token}"))
                .header("Content-Type", "application/json")
                .body(Body::from(
                    serde_json::json!({"enabled": false}).to_string(),
                ))
                .unwrap(),
        )
        .await
        .expect("route response");
    let status = response.status();
    let body = body_json(response).await;
    assert!(status.is_success(), "got {status}: {body}");
    assert_eq!(body["enabled"], false);

    let response = build_app(pool.clone())
        .oneshot(
            Request::builder()
                .method("DELETE")
                .uri(format!("/api/v1/models/{}", model.id))
                .header("Authorization", format!("Bearer {token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .expect("route response");
    let status = response.status();
    let body = body_json(response).await;
    assert!(status.is_success(), "got {status}: {body}");
    assert_eq!(body["deleted"], true);
}

// ---------------------------------------------------------------------------------------
// Task 5: teams RBAC scoping through the real router.
//
// This is the "central" convention this whole plan establishes: `super_admin` sees
// everything unless it opts into a single org via `?organization_id=`; `org_admin` is
// always pinned to its own org no matter what the query string or request body says.
// ---------------------------------------------------------------------------------------

/// `super_admin`: omitting `organization_id` returns teams across every org; passing it
/// scopes the listing to just that one org.
#[sqlx::test(migrations = "../godwit-db/migrations")]
async fn super_admin_lists_all_teams_or_scopes_by_organization_id(pool: PgPool) {
    let orgs = OrganizationRepository::new(pool.clone());
    let org_a = orgs.create("org-a", None).await.expect("create org a");
    let org_b = orgs.create("org-b", None).await.expect("create org b");

    let teams = TeamRepository::new(pool.clone());
    let team_a = teams
        .create(org_a.id, "team-a")
        .await
        .expect("create team a");
    let team_b = teams
        .create(org_b.id, "team-b")
        .await
        .expect("create team b");

    let token = admin_token("super_admin");

    // No `organization_id`: every team, across both orgs.
    let response = build_app(pool.clone())
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/v1/teams")
                .header("Authorization", format!("Bearer {token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .expect("route response");
    assert_eq!(response.status(), StatusCode::OK);
    let body = body_json(response).await;
    let ids: Vec<String> = body["data"]
        .as_array()
        .expect("data array")
        .iter()
        .map(|t| t["id"].as_str().unwrap().to_string())
        .collect();
    assert_eq!(
        ids.len(),
        2,
        "super_admin with no organization_id must see teams across all orgs, got {body}"
    );
    assert!(ids.contains(&team_a.id.to_string()));
    assert!(ids.contains(&team_b.id.to_string()));

    // `organization_id=org_a`: only org A's team.
    let response = build_app(pool)
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(format!("/api/v1/teams?organization_id={}", org_a.id))
                .header("Authorization", format!("Bearer {token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .expect("route response");
    assert_eq!(response.status(), StatusCode::OK);
    let body = body_json(response).await;
    let ids: Vec<String> = body["data"]
        .as_array()
        .expect("data array")
        .iter()
        .map(|t| t["id"].as_str().unwrap().to_string())
        .collect();
    assert_eq!(
        ids,
        vec![team_a.id.to_string()],
        "organization_id must scope the listing to just that org, got {body}"
    );
}

/// `org_admin` is always scoped to its own org, even if it tries to peek at another org's
/// teams via `?organization_id=`.
#[sqlx::test(migrations = "../godwit-db/migrations")]
async fn org_admin_cannot_use_organization_id_to_see_another_orgs_teams(pool: PgPool) {
    let orgs = OrganizationRepository::new(pool.clone());
    let org_a = orgs.create("org-a", None).await.expect("create org a");
    let org_b = orgs.create("org-b", None).await.expect("create org b");

    let teams = TeamRepository::new(pool.clone());
    let team_a = teams
        .create(org_a.id, "team-a")
        .await
        .expect("create team a");
    teams
        .create(org_b.id, "team-b")
        .await
        .expect("create team b");

    let token = admin_token_for_org("org_admin", org_a.id);

    // Attempting to see org B's teams via the query param must be ignored.
    let response = build_app(pool)
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(format!("/api/v1/teams?organization_id={}", org_b.id))
                .header("Authorization", format!("Bearer {token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .expect("route response");
    assert_eq!(response.status(), StatusCode::OK);
    let body = body_json(response).await;
    let ids: Vec<String> = body["data"]
        .as_array()
        .expect("data array")
        .iter()
        .map(|t| t["id"].as_str().unwrap().to_string())
        .collect();
    assert_eq!(
        ids,
        vec![team_a.id.to_string()],
        "org_admin must only ever see its own org's teams, regardless of organization_id, got {body}"
    );
}

/// `org_admin` cannot rename a team belonging to a different org, even by guessing its ID.
#[sqlx::test(migrations = "../godwit-db/migrations")]
async fn org_admin_cannot_rename_a_team_in_another_org(pool: PgPool) {
    let orgs = OrganizationRepository::new(pool.clone());
    let org_a = orgs.create("org-a", None).await.expect("create org a");
    let org_b = orgs.create("org-b", None).await.expect("create org b");

    let teams = TeamRepository::new(pool.clone());
    let team_b = teams
        .create(org_b.id, "team-b")
        .await
        .expect("create team b");

    let token = admin_token_for_org("org_admin", org_a.id);

    let response = build_app(pool.clone())
        .oneshot(
            Request::builder()
                .method("PATCH")
                .uri(format!("/api/v1/teams/{}", team_b.id))
                .header("Authorization", format!("Bearer {token}"))
                .header("Content-Type", "application/json")
                .body(Body::from(
                    serde_json::json!({"name": "hijacked"}).to_string(),
                ))
                .unwrap(),
        )
        .await
        .expect("route response");
    assert_eq!(
        response.status(),
        StatusCode::FORBIDDEN,
        "org_admin must not be able to rename a team outside its own org"
    );

    // ...and the team's name is unchanged.
    let unchanged = teams.get_by_id(team_b.id).await.expect("fetch team b");
    assert_eq!(unchanged.name, "team-b");
}

/// `super_admin` must supply `organization_id` explicitly when creating a team — there is no
/// implicit org to fall back to, so a missing field is a 400, not a silently-chosen default.
#[sqlx::test(migrations = "../godwit-db/migrations")]
async fn super_admin_create_team_without_organization_id_is_bad_request(pool: PgPool) {
    let token = admin_token("super_admin");

    let response = build_app(pool.clone())
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/teams")
                .header("Authorization", format!("Bearer {token}"))
                .header("Content-Type", "application/json")
                .body(Body::from(
                    serde_json::json!({"name": "orphan-team"}).to_string(),
                ))
                .unwrap(),
        )
        .await
        .expect("route response");
    assert_eq!(
        response.status(),
        StatusCode::BAD_REQUEST,
        "super_admin must supply organization_id explicitly when creating a team"
    );

    let all = TeamRepository::new(pool)
        .list_all()
        .await
        .expect("list all teams");
    assert!(
        all.is_empty(),
        "a rejected POST must not create a team"
    );
}

// ---------------------------------------------------------------------------------------
// 3. RBAC rejection.
// ---------------------------------------------------------------------------------------

/// The instance-wide catalog is `super_admin`-only; `org_admin`/`team_admin`/`user` must be
/// rejected with 403 on every catalog endpoint.
#[sqlx::test(migrations = "../godwit-db/migrations")]
async fn non_super_admin_roles_are_forbidden_from_catalog_endpoints(pool: PgPool) {
    let profile = ProviderProfileRepository::new(pool.clone())
        .create(
            "local-vllm",
            "vllm",
            Some("http://localhost:8000/v1"),
            false,
        )
        .await
        .expect("create profile");

    let cases: Vec<(&str, &str, String, Option<serde_json::Value>)> = vec![
        (
            "GET",
            "org_admin",
            "/api/v1/provider-profiles".to_string(),
            None,
        ),
        ("GET", "team_admin", "/api/v1/models".to_string(), None),
        ("GET", "user", "/api/v1/models".to_string(), None),
        (
            "POST",
            "org_admin",
            "/api/v1/models".to_string(),
            Some(serde_json::json!({
                "public_id": "sneaky",
                "provider": "vllm",
                "provider_profile_id": profile.id,
                "provider_model_id": "x",
                "capabilities": "chat"
            })),
        ),
        (
            "POST",
            "team_admin",
            "/api/v1/provider-profiles".to_string(),
            Some(serde_json::json!({
                "name": "sneaky",
                "protocol": "openai",
                "base_url": "https://evil.example.com"
            })),
        ),
    ];

    for (http_method, role, uri, payload) in cases {
        let token = admin_token(role);
        let mut builder = Request::builder()
            .method(http_method)
            .uri(&uri)
            .header("Authorization", format!("Bearer {token}"));
        let body = match &payload {
            Some(json) => {
                builder = builder.header("Content-Type", "application/json");
                Body::from(json.to_string())
            }
            None => Body::empty(),
        };

        let response = build_app(pool.clone())
            .oneshot(builder.body(body).unwrap())
            .await
            .expect("route response");
        assert_eq!(
            response.status(),
            StatusCode::FORBIDDEN,
            "{role} must be forbidden from {http_method} {uri}"
        );
    }

    // Nothing was created by the rejected requests.
    let models = ModelRepository::new(pool.clone())
        .list()
        .await
        .expect("list models");
    assert!(
        models.is_empty(),
        "a forbidden POST must not create a model"
    );
    let profiles = ProviderProfileRepository::new(pool)
        .list()
        .await
        .expect("list profiles");
    assert_eq!(
        profiles.len(),
        1,
        "a forbidden POST must not create a profile"
    );
}

#[sqlx::test(migrations = "../godwit-db/migrations")]
async fn admin_catalog_endpoints_require_a_token(pool: PgPool) {
    let response = build_app(pool)
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/v1/models")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .expect("route response");
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

// ---------------------------------------------------------------------------------------
// Extra coverage for the remaining fixes, through real routes.
// ---------------------------------------------------------------------------------------

/// Important Fix 7: an unsupported capability must be a 400 with an explanation, not the
/// opaque 500 that `ApiError::Core` renders.
#[sqlx::test(migrations = "../godwit-db/migrations")]
async fn image_edit_against_a_vllm_model_returns_400_not_500(pool: PgPool) {
    let profiles = ProviderProfileRepository::new(pool.clone());
    let profile = profiles
        .create(
            "local-vllm",
            "vllm",
            Some("http://localhost:8000/v1"),
            false,
        )
        .await
        .expect("create profile");
    // The catalog row claims image_edit; the vllm adapter does not implement it, so the
    // rejection has to come from the adapter (not from the router's capability check).
    ModelRepository::new(pool.clone())
        .create("llama-edit", "vllm", profile.id, "llama-3", "image_edit")
        .await
        .expect("create model");

    let api_key = seed_api_key(&pool).await;
    let boundary = "TESTBOUNDARY";
    let multipart = format!(
        "--{b}\r\nContent-Disposition: form-data; name=\"model\"\r\n\r\nllama-edit\r\n\
         --{b}\r\nContent-Disposition: form-data; name=\"prompt\"\r\n\r\nadd a hat\r\n\
         --{b}\r\nContent-Disposition: form-data; name=\"image\"; filename=\"a.png\"\r\n\
         Content-Type: image/png\r\n\r\n\x01\x02\x03\r\n--{b}--\r\n",
        b = boundary
    );

    let response = build_app(pool)
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/v1/images/edits")
                .header("Authorization", format!("Bearer {api_key}"))
                .header(
                    "Content-Type",
                    format!("multipart/form-data; boundary={boundary}"),
                )
                .body(Body::from(multipart))
                .unwrap(),
        )
        .await
        .expect("route response");

    let status = response.status();
    let body = body_json(response).await;
    assert_eq!(
        status,
        StatusCode::BAD_REQUEST,
        "CapabilityNotSupported must surface as 400, got {status}: {body}"
    );
    assert!(
        body["detail"]
            .as_str()
            .unwrap_or_default()
            .contains("not supported"),
        "the 400 should explain what is unsupported, got: {body}"
    );
}

/// Important Fix 3: a disabled profile must neither serve traffic nor be advertised by
/// `GET /v1/models`.
#[sqlx::test(migrations = "../godwit-db/migrations")]
async fn disabled_profile_is_hidden_from_v1_models_and_rejected_by_chat(pool: PgPool) {
    let profiles = ProviderProfileRepository::new(pool.clone());
    let models = ModelRepository::new(pool.clone());

    let live = profiles
        .create("live", "openai", Some("https://api.openai.com/v1"), false)
        .await
        .expect("create live profile");
    models
        .create("visible-model", "openai", live.id, "gpt-4o", "chat")
        .await
        .expect("create visible model");

    let retired = profiles
        .create(
            "retired",
            "openai",
            Some("https://api.openai.com/v1"),
            false,
        )
        .await
        .expect("create retired profile");
    models
        .create("hidden-model", "openai", retired.id, "gpt-4o", "chat")
        .await
        .expect("create hidden model");
    profiles
        .update(retired.id, None, None, Some(false))
        .await
        .expect("disable profile");

    let api_key = seed_api_key(&pool).await;

    let response = build_app(pool.clone())
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/v1/models")
                .header("Authorization", format!("Bearer {api_key}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .expect("route response");
    assert_eq!(response.status(), StatusCode::OK);
    let body = body_json(response).await;
    let ids: Vec<&str> = body["data"]
        .as_array()
        .expect("data array")
        .iter()
        .map(|m| m["id"].as_str().unwrap())
        .collect();
    assert_eq!(
        ids,
        vec!["visible-model"],
        "models behind a disabled profile must not be advertised"
    );

    let response = build_app(pool)
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/v1/chat/completions")
                .header("Authorization", format!("Bearer {api_key}"))
                .header("Content-Type", "application/json")
                .body(Body::from(
                    serde_json::json!({
                        "model": "hidden-model",
                        "messages": [{"role": "user", "content": "Hi"}],
                        "stream": false
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .expect("route response");
    assert_eq!(
        response.status(),
        StatusCode::NOT_FOUND,
        "a disabled profile must behave as if it does not exist"
    );
}

/// Important Fix 6: an operator-side master-key misconfiguration must not be reported to
/// the caller as 401 ("your API key is invalid") — it is a 500.
#[sqlx::test(migrations = "../godwit-db/migrations")]
async fn undecryptable_provider_credentials_return_500_not_401(pool: PgPool) {
    let profiles = ProviderProfileRepository::new(pool.clone());
    let profile = profiles
        .create(
            "upstream",
            "openai",
            Some("https://api.openai.com/v1"),
            true,
        )
        .await
        .expect("create profile");
    // Encrypted under a DIFFERENT master key than the app's MASTER_KEY.
    let secret = godwit_auth::credentials::encrypt_api_key(&[7u8; 32], "sk-upstream");
    profiles
        .set_auth(profile.id, &secret)
        .await
        .expect("set auth");

    let api_key = seed_api_key(&pool).await;

    let response = build_app(pool)
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/v1/chat/completions")
                .header("Authorization", format!("Bearer {api_key}"))
                .header("Content-Type", "application/json")
                .body(Body::from(
                    serde_json::json!({
                        "model": "upstream/gpt-4o",
                        "messages": [{"role": "user", "content": "Hi"}],
                        "stream": false
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .expect("route response");

    assert_eq!(
        response.status(),
        StatusCode::INTERNAL_SERVER_ERROR,
        "a wrong CREDENTIAL_ENCRYPTION_KEY is the operator's problem, not the caller's"
    );
}

/// A catalog row mapping a friendly `public_id` onto a different upstream id must actually
/// translate on the wire — the non-wildcard half of Critical Fix 2.
#[sqlx::test(migrations = "../godwit-db/migrations")]
async fn catalog_model_translates_public_id_to_provider_model_id(pool: PgPool) {
    let upstream = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/chat/completions"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "id": "chatcmpl-xyz",
            "object": "chat.completion",
            "created": 1,
            "model": "gpt-4o-2024-08-06",
            "choices": [{
                "index": 0,
                "message": {"role": "assistant", "content": "ok"},
                "finish_reason": "stop"
            }],
            "usage": {"prompt_tokens": 1, "completion_tokens": 1, "total_tokens": 2}
        })))
        .mount(&upstream)
        .await;

    let profiles = ProviderProfileRepository::new(pool.clone());
    let profile = profiles
        .create("openai", "openai", Some(&upstream.uri()), false)
        .await
        .expect("create profile");
    let secret = godwit_auth::credentials::encrypt_api_key(&MASTER_KEY, "sk-upstream");
    profiles
        .set_auth(profile.id, &secret)
        .await
        .expect("set auth");
    ModelRepository::new(pool.clone())
        .create("my-4o", "openai", profile.id, "gpt-4o-2024-08-06", "chat")
        .await
        .expect("create model");

    let api_key = seed_api_key(&pool).await;

    let response = build_app(pool)
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/v1/chat/completions")
                .header("Authorization", format!("Bearer {api_key}"))
                .header("Content-Type", "application/json")
                .body(Body::from(
                    serde_json::json!({
                        "model": "my-4o",
                        "messages": [{"role": "user", "content": "Hi"}],
                        "stream": false
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .expect("route response");
    let status = response.status();
    let body = body_json(response).await;
    assert_eq!(status, StatusCode::OK, "body={body}");

    let received = upstream
        .received_requests()
        .await
        .expect("request recording enabled");
    let upstream_body: serde_json::Value =
        serde_json::from_slice(&received[0].body).expect("upstream body is JSON");
    assert_eq!(upstream_body["model"], "gpt-4o-2024-08-06");
    assert_ne!(upstream_body["model"], "my-4o");
}

// ---------------------------------------------------------------------------------------
// 4. Refresh token flow (Task 3): rotation, reuse rejection, expiry, idempotent logout.
// ---------------------------------------------------------------------------------------

/// Drives `login` -> `refresh` -> `logout` through the real router end to end. This is the
/// regression guard for the security properties that a unit test on request deserialization
/// cannot see: that `refresh` actually rotates (deletes the old row, not just issues a new
/// one alongside it), that the rotated-away token is truly dead (not merely superseded),
/// and that `logout` is safe to call twice.
#[sqlx::test(migrations = "../godwit-db/migrations")]
async fn refresh_token_rotates_rejects_reuse_and_logout_is_idempotent(pool: PgPool) {
    let (email, password) = seed_password_user(&pool).await;

    // 1. Login issues an access + refresh token pair.
    let response = build_app(pool.clone())
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/auth/login")
                .header("Content-Type", "application/json")
                .body(Body::from(
                    serde_json::json!({"email": email, "password": password}).to_string(),
                ))
                .unwrap(),
        )
        .await
        .expect("route response");
    assert_eq!(response.status(), StatusCode::OK);
    let body = body_json(response).await;
    let access_token_1 = body["access_token"]
        .as_str()
        .expect("access_token present")
        .to_string();
    let refresh_token_1 = body["refresh_token"]
        .as_str()
        .expect("refresh_token present")
        .to_string();
    assert!(!access_token_1.is_empty());
    assert!(!refresh_token_1.is_empty());

    // 2. Exchanging the refresh token for a new pair must rotate it: the new refresh token
    // has to differ from the one just spent, not just "return something".
    let response = build_app(pool.clone())
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/auth/refresh")
                .header("Content-Type", "application/json")
                .body(Body::from(
                    serde_json::json!({"refresh_token": refresh_token_1}).to_string(),
                ))
                .unwrap(),
        )
        .await
        .expect("route response");
    assert_eq!(response.status(), StatusCode::OK);
    let body = body_json(response).await;
    let refresh_token_2 = body["refresh_token"]
        .as_str()
        .expect("refresh_token present")
        .to_string();
    assert_ne!(
        refresh_token_2, refresh_token_1,
        "refresh must rotate to a brand-new refresh token, not reissue the same one"
    );

    // 3. The original (now-rotated-away) refresh token must be rejected: proof that it was
    // actually deleted, not merely superseded by the new one.
    let response = build_app(pool.clone())
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/auth/refresh")
                .header("Content-Type", "application/json")
                .body(Body::from(
                    serde_json::json!({"refresh_token": refresh_token_1}).to_string(),
                ))
                .unwrap(),
        )
        .await
        .expect("route response");
    assert_eq!(
        response.status(),
        StatusCode::UNAUTHORIZED,
        "a used-and-rotated refresh token must not be reusable"
    );

    // 4. Logout invalidates the current (still-valid) refresh token.
    let response = build_app(pool.clone())
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/auth/logout")
                .header("Content-Type", "application/json")
                .body(Body::from(
                    serde_json::json!({"refresh_token": refresh_token_2}).to_string(),
                ))
                .unwrap(),
        )
        .await
        .expect("route response");
    assert_eq!(response.status(), StatusCode::OK);
    let body = body_json(response).await;
    assert_eq!(body["logged_out"], true);

    // ...and calling logout again with the same (already-deleted) token is idempotent: no
    // error, same success response — not a 404/500 on the second call.
    let response = build_app(pool.clone())
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/auth/logout")
                .header("Content-Type", "application/json")
                .body(Body::from(
                    serde_json::json!({"refresh_token": refresh_token_2}).to_string(),
                ))
                .unwrap(),
        )
        .await
        .expect("route response");
    assert_eq!(
        response.status(),
        StatusCode::OK,
        "logging out a token that no longer exists must not error"
    );
    let body = body_json(response).await;
    assert_eq!(body["logged_out"], true);

    // 5. The logged-out token can no longer be used to refresh.
    let response = build_app(pool)
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/auth/refresh")
                .header("Content-Type", "application/json")
                .body(Body::from(
                    serde_json::json!({"refresh_token": refresh_token_2}).to_string(),
                ))
                .unwrap(),
        )
        .await
        .expect("route response");
    assert_eq!(
        response.status(),
        StatusCode::UNAUTHORIZED,
        "a logged-out refresh token must be rejected"
    );
}

/// An expired refresh token must be rejected by `/auth/refresh`, and cleaned up (deleted)
/// rather than left lying around. Constructed by inserting an already-expired row directly
/// through `RefreshTokenRepository`, bypassing normal issuance.
#[sqlx::test(migrations = "../godwit-db/migrations")]
async fn expired_refresh_token_is_rejected_and_cleaned_up(pool: PgPool) {
    let (email, _password) = seed_password_user(&pool).await;
    let user = UserRepository::new(pool.clone())
        .get_by_email(&email)
        .await
        .expect("fetch seeded user");

    let (plaintext, hash) = godwit_auth::refresh_tokens::generate_refresh_token();
    let expires_at = chrono::Utc::now() - chrono::Duration::days(1);
    RefreshTokenRepository::new(pool.clone())
        .create(user.id, &hash, expires_at)
        .await
        .expect("insert expired refresh token");

    let response = build_app(pool.clone())
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/auth/refresh")
                .header("Content-Type", "application/json")
                .body(Body::from(
                    serde_json::json!({"refresh_token": plaintext}).to_string(),
                ))
                .unwrap(),
        )
        .await
        .expect("route response");
    assert_eq!(
        response.status(),
        StatusCode::UNAUTHORIZED,
        "an expired refresh token must be rejected"
    );

    let err = RefreshTokenRepository::new(pool)
        .get_by_hash(&hash)
        .await
        .expect_err("an expired, rejected refresh token must have been deleted");
    assert!(matches!(err, godwit_core::PasteurError::NotFound));
}

// ---------------------------------------------------------------------------------------
// Task 6: team membership RBAC through the real router.
//
// `require_team_manage` is a three-way check: `super_admin` always allowed; `org_admin`
// allowed only for teams in its own org; anyone else (including a role literally named
// `team_admin`) allowed only if a `team_memberships` row says *they specifically* hold
// `team_admin` on *that specific team* — not merely their global role. These tests exercise
// all three branches, including the cross-team bypass attempt every prior admin-resource
// task's review has had to catch after the fact.
// ---------------------------------------------------------------------------------------

/// A `team_admin` membership holder for team A can add and then remove members of team A.
#[sqlx::test(migrations = "../godwit-db/migrations")]
async fn team_admin_can_manage_own_team_members(pool: PgPool) {
    let org = OrganizationRepository::new(pool.clone())
        .create("acme", None)
        .await
        .expect("create org");
    let team_a = TeamRepository::new(pool.clone())
        .create(org.id, "team-a")
        .await
        .expect("create team a");

    let users = UserRepository::new(pool.clone());
    let admin_user = users
        .create("team-a-admin@example.com", None, UserRole::User, Some(org.id))
        .await
        .expect("create team admin user");
    let target_user = users
        .create("target@example.com", None, UserRole::User, Some(org.id))
        .await
        .expect("create target user");

    // Seed admin_user as team_admin of team_a directly through the repository.
    TeamMembershipRepository::new(pool.clone())
        .add_member(team_a.id, admin_user.id, "team_admin")
        .await
        .expect("seed team_admin membership");

    let token = admin_token_for_user("team_admin", org.id, admin_user.id);

    // Add target_user as a plain member of team_a.
    let response = build_app(pool.clone())
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/api/v1/teams/{}/members", team_a.id))
                .header("Authorization", format!("Bearer {token}"))
                .header("Content-Type", "application/json")
                .body(Body::from(
                    serde_json::json!({"user_id": target_user.id, "role": "member"}).to_string(),
                ))
                .unwrap(),
        )
        .await
        .expect("route response");
    assert_eq!(
        response.status(),
        StatusCode::OK,
        "a team's own team_admin must be able to add a member to that team"
    );

    let membership = TeamMembershipRepository::new(pool.clone())
        .get_membership(team_a.id, target_user.id)
        .await
        .expect("membership must exist");
    assert_eq!(membership.role, "member");

    // Remove target_user from team_a.
    let response = build_app(pool.clone())
        .oneshot(
            Request::builder()
                .method("DELETE")
                .uri(format!("/api/v1/teams/{}/members/{}", team_a.id, target_user.id))
                .header("Authorization", format!("Bearer {token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .expect("route response");
    assert_eq!(
        response.status(),
        StatusCode::OK,
        "a team's own team_admin must be able to remove a member from that team"
    );

    let err = TeamMembershipRepository::new(pool)
        .get_membership(team_a.id, target_user.id)
        .await
        .expect_err("membership must have been removed");
    assert!(matches!(err, godwit_core::PasteurError::NotFound));
}

/// A `team_admin` of team A must not be able to manage team B's membership, even though both
/// teams belong to the same org — the membership check is scoped to the *specific* team, not
/// derived from the caller's global role.
#[sqlx::test(migrations = "../godwit-db/migrations")]
async fn team_admin_of_one_team_cannot_manage_another_teams_members(pool: PgPool) {
    let org = OrganizationRepository::new(pool.clone())
        .create("acme", None)
        .await
        .expect("create org");
    let teams = TeamRepository::new(pool.clone());
    let team_a = teams.create(org.id, "team-a").await.expect("create team a");
    let team_b = teams.create(org.id, "team-b").await.expect("create team b");

    let users = UserRepository::new(pool.clone());
    let admin_user = users
        .create("team-a-admin@example.com", None, UserRole::User, Some(org.id))
        .await
        .expect("create team admin user");
    let target_user = users
        .create("target@example.com", None, UserRole::User, Some(org.id))
        .await
        .expect("create target user");

    TeamMembershipRepository::new(pool.clone())
        .add_member(team_a.id, admin_user.id, "team_admin")
        .await
        .expect("seed team_admin membership on team_a only");

    let token = admin_token_for_user("team_admin", org.id, admin_user.id);

    // Attempt to add a member to team_b, which admin_user has no membership on.
    let response = build_app(pool.clone())
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/api/v1/teams/{}/members", team_b.id))
                .header("Authorization", format!("Bearer {token}"))
                .header("Content-Type", "application/json")
                .body(Body::from(
                    serde_json::json!({"user_id": target_user.id, "role": "member"}).to_string(),
                ))
                .unwrap(),
        )
        .await
        .expect("route response");
    assert_eq!(
        response.status(),
        StatusCode::FORBIDDEN,
        "a team_admin of team A must not be able to add members to team B"
    );

    let err = TeamMembershipRepository::new(pool.clone())
        .get_membership(team_b.id, target_user.id)
        .await
        .expect_err("no membership must have been created on team_b");
    assert!(matches!(err, godwit_core::PasteurError::NotFound));

    // Also seed target_user onto team_b directly, then confirm the same caller cannot remove it.
    TeamMembershipRepository::new(pool.clone())
        .add_member(team_b.id, target_user.id, "member")
        .await
        .expect("seed member on team_b");

    let response = build_app(pool.clone())
        .oneshot(
            Request::builder()
                .method("DELETE")
                .uri(format!("/api/v1/teams/{}/members/{}", team_b.id, target_user.id))
                .header("Authorization", format!("Bearer {token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .expect("route response");
    assert_eq!(
        response.status(),
        StatusCode::FORBIDDEN,
        "a team_admin of team A must not be able to remove members from team B"
    );

    let still_there = TeamMembershipRepository::new(pool)
        .get_membership(team_b.id, target_user.id)
        .await
        .expect("membership on team_b must remain untouched");
    assert_eq!(still_there.role, "member");
}

/// An `org_admin` can manage membership on any team within its own org, even without holding
/// any `team_admin` membership row itself.
#[sqlx::test(migrations = "../godwit-db/migrations")]
async fn org_admin_can_manage_any_teams_members_in_its_own_org(pool: PgPool) {
    let org = OrganizationRepository::new(pool.clone())
        .create("acme", None)
        .await
        .expect("create org");
    let team = TeamRepository::new(pool.clone())
        .create(org.id, "engineering")
        .await
        .expect("create team");
    let target_user = UserRepository::new(pool.clone())
        .create("target@example.com", None, UserRole::User, Some(org.id))
        .await
        .expect("create target user");

    let token = admin_token_for_org("org_admin", org.id);

    let response = build_app(pool.clone())
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/api/v1/teams/{}/members", team.id))
                .header("Authorization", format!("Bearer {token}"))
                .header("Content-Type", "application/json")
                .body(Body::from(
                    serde_json::json!({"user_id": target_user.id, "role": "team_admin"})
                        .to_string(),
                ))
                .unwrap(),
        )
        .await
        .expect("route response");
    assert_eq!(
        response.status(),
        StatusCode::OK,
        "org_admin must be able to manage any team's members within its own org"
    );

    let membership = TeamMembershipRepository::new(pool)
        .get_membership(team.id, target_user.id)
        .await
        .expect("membership must exist");
    assert_eq!(membership.role, "team_admin");
}

/// A plain `user` with no `team_admin` membership anywhere is forbidden from managing any
/// team's membership.
#[sqlx::test(migrations = "../godwit-db/migrations")]
async fn plain_user_without_any_team_admin_membership_is_forbidden(pool: PgPool) {
    let org = OrganizationRepository::new(pool.clone())
        .create("acme", None)
        .await
        .expect("create org");
    let team = TeamRepository::new(pool.clone())
        .create(org.id, "engineering")
        .await
        .expect("create team");
    let target_user = UserRepository::new(pool.clone())
        .create("target@example.com", None, UserRole::User, Some(org.id))
        .await
        .expect("create target user");

    let token = admin_token_for_org("user", org.id);

    let response = build_app(pool.clone())
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/api/v1/teams/{}/members", team.id))
                .header("Authorization", format!("Bearer {token}"))
                .header("Content-Type", "application/json")
                .body(Body::from(
                    serde_json::json!({"user_id": target_user.id, "role": "member"}).to_string(),
                ))
                .unwrap(),
        )
        .await
        .expect("route response");
    assert_eq!(
        response.status(),
        StatusCode::FORBIDDEN,
        "a plain user with no team_admin membership must be forbidden"
    );

    let err = TeamMembershipRepository::new(pool)
        .get_membership(team.id, target_user.id)
        .await
        .expect_err("no membership must have been created");
    assert!(matches!(err, godwit_core::PasteurError::NotFound));
}

/// `add_member` validates the `role` field against the allowed set before hitting the
/// database, so an invalid role is a clear 400 rather than the CHECK constraint's opaque
/// database error.
#[sqlx::test(migrations = "../godwit-db/migrations")]
async fn add_member_rejects_invalid_role_before_hitting_the_database(pool: PgPool) {
    let org = OrganizationRepository::new(pool.clone())
        .create("acme", None)
        .await
        .expect("create org");
    let team = TeamRepository::new(pool.clone())
        .create(org.id, "engineering")
        .await
        .expect("create team");
    let target_user = UserRepository::new(pool.clone())
        .create("target@example.com", None, UserRole::User, Some(org.id))
        .await
        .expect("create target user");

    let token = admin_token_for_org("super_admin", org.id);

    let response = build_app(pool.clone())
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/api/v1/teams/{}/members", team.id))
                .header("Authorization", format!("Bearer {token}"))
                .header("Content-Type", "application/json")
                .body(Body::from(
                    serde_json::json!({"user_id": target_user.id, "role": "owner"}).to_string(),
                ))
                .unwrap(),
        )
        .await
        .expect("route response");
    assert_eq!(
        response.status(),
        StatusCode::BAD_REQUEST,
        "an invalid role must be rejected with 400, not a database error"
    );

    let err = TeamMembershipRepository::new(pool)
        .get_membership(team.id, target_user.id)
        .await
        .expect_err("no membership must have been created for an invalid role");
    assert!(matches!(err, godwit_core::PasteurError::NotFound));
}

// ---------------------------------------------------------------------------------------
// 5. Users RBAC: org_admin must never be able to grant or self-assign super_admin.
// ---------------------------------------------------------------------------------------

/// `org_admin` may act on users within its own org (`check_same_org` allows this), but must
/// never be able to grant instance-wide `super_admin` privilege to one of them.
#[sqlx::test(migrations = "../godwit-db/migrations")]
async fn org_admin_cannot_promote_another_in_org_user_to_super_admin(pool: PgPool) {
    let org = OrganizationRepository::new(pool.clone())
        .create("acme", None)
        .await
        .expect("create org");
    let target = UserRepository::new(pool.clone())
        .create("target@example.com", None, UserRole::User, Some(org.id))
        .await
        .expect("create target user");

    let token = admin_token_for_org("org_admin", org.id);

    let response = build_app(pool.clone())
        .oneshot(
            Request::builder()
                .method("PATCH")
                .uri(format!("/api/v1/users/{}", target.id))
                .header("Authorization", format!("Bearer {token}"))
                .header("Content-Type", "application/json")
                .body(Body::from(
                    serde_json::json!({"role": "super_admin"}).to_string(),
                ))
                .unwrap(),
        )
        .await
        .expect("route response");
    assert_eq!(
        response.status(),
        StatusCode::FORBIDDEN,
        "org_admin must not be able to grant super_admin to another in-org user"
    );

    let unchanged = UserRepository::new(pool)
        .get_by_id(target.id)
        .await
        .expect("fetch target");
    assert_eq!(
        unchanged.role, "user",
        "the target's role must be unchanged after the rejected request"
    );
}

/// No caller — `org_admin` included — may change its own `role` via `PATCH /users/:id`,
/// mirroring the self-delete guard: self-role-change is a distinct footgun blocked the
/// same way, regardless of which role value was requested.
#[sqlx::test(migrations = "../godwit-db/migrations")]
async fn org_admin_cannot_change_its_own_role(pool: PgPool) {
    let org = OrganizationRepository::new(pool.clone())
        .create("acme", None)
        .await
        .expect("create org");
    let caller = UserRepository::new(pool.clone())
        .create("caller@example.com", None, UserRole::OrgAdmin, Some(org.id))
        .await
        .expect("create caller user");

    let token = admin_token_for_user("org_admin", org.id, caller.id);

    let response = build_app(pool.clone())
        .oneshot(
            Request::builder()
                .method("PATCH")
                .uri(format!("/api/v1/users/{}", caller.id))
                .header("Authorization", format!("Bearer {token}"))
                .header("Content-Type", "application/json")
                .body(Body::from(
                    serde_json::json!({"role": "org_admin"}).to_string(),
                ))
                .unwrap(),
        )
        .await
        .expect("route response");
    assert_eq!(
        response.status(),
        StatusCode::BAD_REQUEST,
        "a caller must never be able to change its own role, even to its current value"
    );
}

/// `org_admin` must never be able to create a brand-new `super_admin` user via
/// `POST /users` either — the same restriction as the `update_user` path applies to
/// `create_user`.
#[sqlx::test(migrations = "../godwit-db/migrations")]
async fn org_admin_cannot_create_a_super_admin_user(pool: PgPool) {
    let org = OrganizationRepository::new(pool.clone())
        .create("acme", None)
        .await
        .expect("create org");
    let token = admin_token_for_org("org_admin", org.id);

    let response = build_app(pool.clone())
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/users")
                .header("Authorization", format!("Bearer {token}"))
                .header("Content-Type", "application/json")
                .body(Body::from(
                    serde_json::json!({
                        "email": "new-super@example.com",
                        "role": "super_admin"
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .expect("route response");
    assert_eq!(
        response.status(),
        StatusCode::FORBIDDEN,
        "org_admin must not be able to create a super_admin user"
    );

    let created = UserRepository::new(pool)
        .get_by_email("new-super@example.com")
        .await;
    assert!(
        created.is_err(),
        "no super_admin user should have been created by an org_admin"
    );
}

/// Regression guard: the new restrictions only apply to non-`super_admin` callers.
/// `super_admin` must still be able to grant `super_admin` to another user via PATCH...
#[sqlx::test(migrations = "../godwit-db/migrations")]
async fn super_admin_can_still_grant_super_admin_role_via_patch(pool: PgPool) {
    let org = OrganizationRepository::new(pool.clone())
        .create("acme", None)
        .await
        .expect("create org");
    let target = UserRepository::new(pool.clone())
        .create("target@example.com", None, UserRole::User, Some(org.id))
        .await
        .expect("create target user");

    let token = admin_token("super_admin");

    let response = build_app(pool.clone())
        .oneshot(
            Request::builder()
                .method("PATCH")
                .uri(format!("/api/v1/users/{}", target.id))
                .header("Authorization", format!("Bearer {token}"))
                .header("Content-Type", "application/json")
                .body(Body::from(
                    serde_json::json!({"role": "super_admin"}).to_string(),
                ))
                .unwrap(),
        )
        .await
        .expect("route response");
    assert_eq!(response.status(), StatusCode::OK);
    let body = body_json(response).await;
    assert_eq!(body["data"]["role"], "super_admin");
}

/// ...and must still be able to create a brand-new `super_admin` user via POST.
///
/// `create_user` inserts into `claims.organization_id`, which must reference a real row
/// (the `users.organization_id` column has a `REFERENCES organizations(id)` FK), so the
/// token here is scoped to a real org rather than `admin_token`'s random one.
#[sqlx::test(migrations = "../godwit-db/migrations")]
async fn super_admin_can_still_create_a_super_admin_user(pool: PgPool) {
    let org = OrganizationRepository::new(pool.clone())
        .create("acme", None)
        .await
        .expect("create org");
    let token = admin_token_for_org("super_admin", org.id);

    let response = build_app(pool.clone())
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/users")
                .header("Authorization", format!("Bearer {token}"))
                .header("Content-Type", "application/json")
                .body(Body::from(
                    serde_json::json!({
                        "email": "new-super@example.com",
                        "role": "super_admin"
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .expect("route response");
    assert_eq!(response.status(), StatusCode::OK);
    let body = body_json(response).await;
    assert_eq!(body["data"]["role"], "super_admin");

    let created = UserRepository::new(pool)
        .get_by_email("new-super@example.com")
        .await
        .expect("fetch created user");
    assert_eq!(created.role, "super_admin");
}

// ---------------------------------------------------------------------------------------
// Task 10: end-to-end coverage tying Tasks 1-9 together through the real router.
// ---------------------------------------------------------------------------------------

/// Login -> refresh -> logout -> refresh-fails, driven end to end through the real router.
#[sqlx::test(migrations = "../godwit-db/migrations")]
async fn login_refresh_logout_flow(pool: PgPool) {
    let app = build_app(pool.clone());

    let user = UserRepository::new(pool.clone())
        .create("flow@example.com", None, UserRole::User, None)
        .await
        .expect("create user");
    sqlx::query("UPDATE users SET password_hash = $2 WHERE id = $1")
        .bind(user.id)
        .bind(godwit_auth::api_keys::hash_password("hunter2"))
        .execute(&pool)
        .await
        .expect("set password");

    let login_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/auth/login")
                .header("content-type", "application/json")
                .body(Body::from(
                    serde_json::json!({"email": "flow@example.com", "password": "hunter2"}).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(login_response.status(), StatusCode::OK);
    let login_body = body_json(login_response).await;
    let refresh_token = login_body["refresh_token"]
        .as_str()
        .expect("refresh_token present")
        .to_string();
    assert!(login_body["access_token"].as_str().is_some());

    // Refresh: exchanges the refresh token for a new pair.
    let refresh_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/auth/refresh")
                .header("content-type", "application/json")
                .body(Body::from(
                    serde_json::json!({"refresh_token": refresh_token}).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(refresh_response.status(), StatusCode::OK);
    let refresh_body = body_json(refresh_response).await;
    let rotated_refresh_token = refresh_body["refresh_token"]
        .as_str()
        .expect("rotated token present")
        .to_string();
    assert_ne!(
        rotated_refresh_token, refresh_token,
        "refresh token should rotate on use"
    );

    // The OLD refresh token is now invalid (single-use / rotated).
    let old_token_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/auth/refresh")
                .header("content-type", "application/json")
                .body(Body::from(
                    serde_json::json!({"refresh_token": refresh_token}).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(old_token_response.status(), StatusCode::UNAUTHORIZED);

    // Logout invalidates the rotated (current) refresh token.
    let logout_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/auth/logout")
                .header("content-type", "application/json")
                .body(Body::from(
                    serde_json::json!({"refresh_token": rotated_refresh_token}).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(logout_response.status(), StatusCode::OK);

    let post_logout_response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/auth/refresh")
                .header("content-type", "application/json")
                .body(Body::from(
                    serde_json::json!({"refresh_token": rotated_refresh_token}).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(post_logout_response.status(), StatusCode::UNAUTHORIZED);
}

/// A `super_admin` creates an organization, then a team inside it, then adds and removes a
/// member of that team — the full org/team/membership lifecycle through the real router.
#[sqlx::test(migrations = "../godwit-db/migrations")]
async fn super_admin_creates_org_team_and_manages_membership(pool: PgPool) {
    let app = build_app(pool.clone());
    let token = admin_token("super_admin");

    let create_org_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/organizations")
                .header("authorization", format!("Bearer {token}"))
                .header("content-type", "application/json")
                .body(Body::from(serde_json::json!({"name": "acme"}).to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(create_org_response.status(), StatusCode::OK);
    let org_id = body_json(create_org_response).await["data"]["id"]
        .as_str()
        .and_then(|s| Uuid::parse_str(s).ok())
        .expect("org id");

    let create_team_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/teams")
                .header("authorization", format!("Bearer {token}"))
                .header("content-type", "application/json")
                .body(Body::from(
                    serde_json::json!({"name": "engineering", "organization_id": org_id})
                        .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(create_team_response.status(), StatusCode::OK);
    let team_id = body_json(create_team_response).await["data"]["id"]
        .as_str()
        .and_then(|s| Uuid::parse_str(s).ok())
        .expect("team id");

    let member = UserRepository::new(pool.clone())
        .create("member@example.com", None, UserRole::User, Some(org_id))
        .await
        .expect("create member");

    let add_member_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/api/v1/teams/{team_id}/members"))
                .header("authorization", format!("Bearer {token}"))
                .header("content-type", "application/json")
                .body(Body::from(
                    serde_json::json!({"user_id": member.id, "role": "member"}).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(add_member_response.status(), StatusCode::OK);

    let remove_member_response = app
        .oneshot(
            Request::builder()
                .method("DELETE")
                .uri(format!("/api/v1/teams/{team_id}/members/{}", member.id))
                .header("authorization", format!("Bearer {token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(remove_member_response.status(), StatusCode::OK);
}

/// A `team_admin` of team A must be forbidden from managing team B's membership, even when
/// both teams belong to the same org — a second, end-to-end-flavored guard for the same
/// property `team_admin_of_one_team_cannot_manage_another_teams_members` already covers.
#[sqlx::test(migrations = "../godwit-db/migrations")]
async fn team_admin_cannot_manage_a_team_they_do_not_administer(pool: PgPool) {
    let app = build_app(pool.clone());
    let org = OrganizationRepository::new(pool.clone())
        .create("acme", None)
        .await
        .expect("create org");
    let team_a = godwit_db::repositories::teams::TeamRepository::new(pool.clone())
        .create(org.id, "team-a")
        .await
        .expect("create team a");
    let team_b = godwit_db::repositories::teams::TeamRepository::new(pool.clone())
        .create(org.id, "team-b")
        .await
        .expect("create team b");

    // A user who is team_admin of team_a, but not team_b. `team_memberships.user_id` has a
    // foreign key onto `users`, so (unlike the brief's literal `Uuid::new_v4()`) this has to
    // be a real, persisted user.
    let team_admin_user = UserRepository::new(pool.clone())
        .create("team-a-admin@example.com", None, UserRole::User, Some(org.id))
        .await
        .expect("create team admin user");
    let claims = godwit_auth::jwt::Claims::new(team_admin_user.id, org.id, "team_admin");
    godwit_db::repositories::team_memberships::TeamMembershipRepository::new(pool.clone())
        .add_member(team_a.id, claims.user_id, "team_admin")
        .await
        .expect("add as team_admin of team_a");
    let token =
        godwit_auth::jwt::issue(JWT_SECRET, claims, chrono::Duration::minutes(15)).expect("issue jwt");

    let other_user = UserRepository::new(pool.clone())
        .create("other@example.com", None, UserRole::User, Some(org.id))
        .await
        .expect("create other user");

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/api/v1/teams/{}/members", team_b.id))
                .header("authorization", format!("Bearer {token}"))
                .header("content-type", "application/json")
                .body(Body::from(
                    serde_json::json!({"user_id": other_user.id, "role": "member"}).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::FORBIDDEN);
}

/// Deleting a user via the real `DELETE /api/v1/users/:id` route actually removes the row
/// (the cascade migration from Task 7, exercised end to end rather than at the repository
/// layer).
#[sqlx::test(migrations = "../godwit-db/migrations")]
async fn deleting_a_user_via_the_api_cascades(pool: PgPool) {
    let app = build_app(pool.clone());
    let token = admin_token("super_admin");
    let org = OrganizationRepository::new(pool.clone())
        .create("acme", None)
        .await
        .expect("create org");
    let user = UserRepository::new(pool.clone())
        .create("todelete@example.com", None, UserRole::User, Some(org.id))
        .await
        .expect("create user");

    let response = app
        .oneshot(
            Request::builder()
                .method("DELETE")
                .uri(format!("/api/v1/users/{}", user.id))
                .header("authorization", format!("Bearer {token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let remaining: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM users WHERE id = $1")
        .bind(user.id)
        .fetch_one(&pool)
        .await
        .expect("count users");
    assert_eq!(remaining, 0);
}

/// A user may never delete their own account via the real route, regardless of role.
#[sqlx::test(migrations = "../godwit-db/migrations")]
async fn a_user_cannot_delete_their_own_account(pool: PgPool) {
    let app = build_app(pool.clone());
    let org = OrganizationRepository::new(pool.clone())
        .create("acme", None)
        .await
        .expect("create org");
    let self_user = UserRepository::new(pool.clone())
        .create("self@example.com", None, UserRole::SuperAdmin, Some(org.id))
        .await
        .expect("create self user");
    let claims = godwit_auth::jwt::Claims::new(self_user.id, org.id, "super_admin");
    let token =
        godwit_auth::jwt::issue(JWT_SECRET, claims, chrono::Duration::minutes(15)).expect("issue jwt");

    let response = app
        .oneshot(
            Request::builder()
                .method("DELETE")
                .uri(format!("/api/v1/users/{}", self_user.id))
                .header("authorization", format!("Bearer {token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

/// The `GET /api/v1/spend` route aggregates real `request_logs` rows and applies the RBAC
/// scoping model end to end: `super_admin` sees everything it asks for; a plain `user` only
/// ever sees its own row, regardless of query params it supplies.
#[sqlx::test(migrations = "../godwit-db/migrations")]
async fn spend_aggregates_request_logs_scoped_to_caller(pool: PgPool) {
    let org = OrganizationRepository::new(pool.clone())
        .create("acme", None)
        .await
        .expect("create org");
    let user_a = UserRepository::new(pool.clone())
        .create("a@example.com", None, UserRole::User, Some(org.id))
        .await
        .expect("create user a");
    let user_b = UserRepository::new(pool.clone())
        .create("b@example.com", None, UserRole::User, Some(org.id))
        .await
        .expect("create user b");

    for (user_id, cost, tokens_in, tokens_out) in [
        (user_a.id, "1.500000", 100, 50),
        (user_b.id, "2.500000", 200, 75),
    ] {
        sqlx::query(
            "INSERT INTO request_logs (user_id, organization_id, model, provider, provider_model_id, tokens_in, tokens_out, cost_usd, duration_ms, status)
             VALUES ($1, $2, 'gpt-4o', 'openai', 'gpt-4o', $3, $4, $5, 100, 'success')"
        )
        .bind(user_id)
        .bind(org.id)
        .bind(tokens_in)
        .bind(tokens_out)
        .bind(cost.parse::<rust_decimal::Decimal>().unwrap())
        .execute(&pool)
        .await
        .expect("insert request log");
    }

    let app = build_app(pool.clone());

    // super_admin sees both rows.
    let super_admin_token = admin_token("super_admin");
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(format!("/api/v1/spend?organization_id={}", org.id))
                .header("authorization", format!("Bearer {super_admin_token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body = body_json(response).await;
    assert_eq!(body["data"].as_array().unwrap().len(), 2);

    // A plain "user" only ever sees their own row, regardless of query params.
    let user_claims = godwit_auth::jwt::Claims::new(user_a.id, org.id, "user");
    let user_token = godwit_auth::jwt::issue(JWT_SECRET, user_claims, chrono::Duration::minutes(15))
        .expect("issue jwt");
    let response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(format!("/api/v1/spend?user_id={}", user_b.id)) // attempt to see user_b's spend
                .header("authorization", format!("Bearer {user_token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body = body_json(response).await;
    let rows = body["data"].as_array().unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0]["user_id"], user_a.id.to_string());
}
