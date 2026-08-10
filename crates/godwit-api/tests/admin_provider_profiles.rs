use axum::{body::Body, http::Request, Router};
use godwit_db::models::UserRole;
use godwit_db::repositories::{
    models::ModelRepository, organizations::OrganizationRepository,
    provider_profiles::ProviderProfileRepository, users::UserRepository,
};
use sqlx::PgPool;
use tower::ServiceExt;
use uuid::Uuid;

const JWT_SECRET: &str = "test-jwt-secret";

fn build_app(pool: PgPool) -> Router {
    let state = godwit_api::app::build_test_state(pool);
    godwit_api::app::build_app(state)
}

fn super_admin_token() -> String {
    let claims = godwit_auth::jwt::Claims::new(Uuid::new_v4(), Uuid::new_v4(), "super_admin");
    godwit_auth::jwt::issue(JWT_SECRET, claims, chrono::Duration::minutes(15)).expect("issue jwt")
}

async fn seed_super_admin(pool: &PgPool) {
    let org = OrganizationRepository::new(pool.clone())
        .create("test-org", None)
        .await
        .unwrap();
    UserRepository::new(pool.clone())
        .create("admin@example.com", None, UserRole::SuperAdmin, Some(org.id))
        .await
        .unwrap();
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

#[sqlx::test(migrations = "../godwit-db/migrations")]
async fn delete_provider_profile_ok(pool: PgPool) {
    seed_super_admin(&pool).await;
    let app = build_app(pool.clone());
    let token = super_admin_token();

    let create_req = Request::builder()
        .method("POST")
        .uri("/api/v1/provider-profiles")
        .header("authorization", format!("Bearer {token}"))
        .header("content-type", "application/json")
        .body(Body::from(
            serde_json::json!({
                "name": "openai",
                "protocol": "openai",
                "base_url": "https://api.openai.com/v1",
                "allow_wildcard": false,
            })
            .to_string(),
        ))
        .unwrap();
    let create_res = app.clone().oneshot(create_req).await.unwrap();
    assert_eq!(create_res.status(), 200);

    let body = body_json(create_res).await;
    let id = body["id"].as_str().unwrap();

    let delete_req = Request::builder()
        .method("DELETE")
        .uri(format!("/api/v1/provider-profiles/{id}"))
        .header("authorization", format!("Bearer {token}"))
        .body(Body::empty())
        .unwrap();
    let delete_res = app.oneshot(delete_req).await.unwrap();
    assert_eq!(delete_res.status(), 200);

    let body = body_json(delete_res).await;
    assert_eq!(body["deleted"], true);
}

#[sqlx::test(migrations = "../godwit-db/migrations")]
async fn delete_provider_profile_blocked_by_models(pool: PgPool) {
    seed_super_admin(&pool).await;
    let app = build_app(pool.clone());
    let token = super_admin_token();

    let repo = ProviderProfileRepository::new(pool.clone());
    let profile = repo.create("openai", "openai", None, false).await.unwrap();

    let models = ModelRepository::new(pool.clone());
    models
        .create(
            "gpt-4o",
            "openai",
            profile.id,
            "gpt-4o",
            "chat",
            serde_json::json!({
                "input_price_per_million": 5.0,
                "output_price_per_million": 15.0,
            }),
        )
        .await
        .unwrap();

    let req = Request::builder()
        .method("DELETE")
        .uri(format!("/api/v1/provider-profiles/{}", profile.id))
        .header("authorization", format!("Bearer {token}"))
        .body(Body::empty())
        .unwrap();
    let res = app.oneshot(req).await.unwrap();
    assert_eq!(res.status(), 400);
}

#[sqlx::test(migrations = "../godwit-db/migrations")]
async fn delete_nonexistent_provider_profile_returns_not_found(pool: PgPool) {
    seed_super_admin(&pool).await;
    let app = build_app(pool.clone());
    let token = super_admin_token();

    let req = Request::builder()
        .method("DELETE")
        .uri(format!("/api/v1/provider-profiles/{}", Uuid::new_v4()))
        .header("authorization", format!("Bearer {token}"))
        .body(Body::empty())
        .unwrap();
    let res = app.oneshot(req).await.unwrap();
    assert_eq!(res.status(), 404);
    let body = body_json(res).await;
    assert_eq!(body["title"], "Not Found");
}
