//! End-to-end integration tests for the password-management auth endpoints, driven through
//! the real assembled `Router` with `tower::ServiceExt::oneshot` (no TCP listener). A fake
//! in-memory `SendEmail` impl stands in for SMTP, so the forgot-password reset link (and its
//! one-shot token) can be captured and consumed without any real mail transport.

use axum::{
    body::Body,
    http::{header::AUTHORIZATION, Request, StatusCode},
    Router,
};
use godwit_core::{AuthConfig, MailConfig, MailTls};
use godwit_db::repositories::{
    organizations::OrganizationRepository, users::UserRepository,
};
use sqlx::PgPool;
use std::sync::{Arc, Mutex};
use tower::ServiceExt;

const JWT_SECRET: &str = "test-jwt-secret";
const STRONG_PASSWORD: &str = "CorrectHorse1!";

struct FakeMailer(Mutex<Vec<(String, String, String, String)>>);

#[async_trait::async_trait]
impl godwit_api::mail::SendEmail for FakeMailer {
    async fn send(
        &self,
        to: &str,
        subject: &str,
        html: &str,
        text: &str,
    ) -> Result<(), godwit_api::mail::MailError> {
        self.0
            .lock()
            .unwrap()
            .push((to.to_string(), subject.to_string(), html.to_string(), text.to_string()));
        Ok(())
    }
}

impl FakeMailer {
    fn len(&self) -> usize {
        self.0.lock().unwrap().len()
    }
}

fn auth_with_mail() -> AuthConfig {
    AuthConfig {
        jwt_secret: JWT_SECRET.to_string(),
        access_token_ttl_minutes: 15,
        refresh_token_ttl_days: 7,
        cookie_secure: false,
        allowed_cookie_origin: "".to_string(),
        login_max_attempts_per_minute: 100,
        trust_proxy: false,
        oidc_providers: vec![],
        saml_providers: vec![],
        mail: Some(MailConfig {
            from: "no-reply@example.com".to_string(),
            host: "smtp.example.com".to_string(),
            port: 587,
            username: None,
            password: None,
            tls: MailTls::StartTls,
            app_url: "https://app.example.com".to_string(),
        }),
        password_policy: godwit_core::PasswordPolicy::default(),
    }
}

fn build_app(pool: PgPool, mailer: Option<Arc<dyn godwit_api::mail::SendEmail>>) -> Router {
    let state = godwit_api::app::build_test_state_with_auth(pool, auth_with_mail(), mailer);
    godwit_api::app::build_app(state)
}

/// Creates an organization and a user with a real (Argon2) password hash, mirroring
/// `router_integration.rs::seed_password_user`.
async fn seed_password_user(pool: &PgPool) -> (String, String) {
    let org = OrganizationRepository::new(pool.clone())
        .create("pw-test-org", None)
        .await
        .expect("create org");
    let email = "pw-user@example.com";
    let user = UserRepository::new(pool.clone())
        .create(email, None, godwit_db::models::UserRole::User, Some(org.id))
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

async fn post_json(app: &Router, uri: &str, body: serde_json::Value) -> axum::response::Response {
    app.clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(uri)
                .header("content-type", "application/json")
                .body(Body::from(body.to_string()))
                .unwrap(),
        )
        .await
        .expect("route response")
}

async fn post_json_auth(
    app: &Router,
    uri: &str,
    token: &str,
    body: serde_json::Value,
) -> axum::response::Response {
    app.clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(uri)
                .header("content-type", "application/json")
                .header(AUTHORIZATION, format!("Bearer {token}"))
                .body(Body::from(body.to_string()))
                .unwrap(),
        )
        .await
        .expect("route response")
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

async fn login(app: &Router, email: &str, password: &str) -> axum::response::Response {
    post_json(
        app,
        "/api/v1/auth/login",
        serde_json::json!({ "email": email, "password": password }),
    )
    .await
}

fn extract_token(mailer: &FakeMailer) -> String {
    let emails = mailer.0.lock().unwrap();
    let text = &emails[0].3;
    let link = text
        .lines()
        .find(|l| l.contains("reset-password"))
        .expect("reset link present in email body");
    link.split("token=")
        .nth(1)
        .expect("token in reset link")
        .trim()
        .to_string()
}

#[sqlx::test(migrations = "../godwit-db/migrations")]
async fn forgot_reset_flow(pool: PgPool) {
    let (email, old_password) = seed_password_user(&pool).await;
    let mailer = Arc::new(FakeMailer(Mutex::new(Vec::new())));
    let app = build_app(pool.clone(), Some(mailer.clone()));

    let response = post_json(
        &app,
        "/api/v1/auth/forgot-password",
        serde_json::json!({ "email": email }),
    )
    .await;
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(body_json(response).await["ok"], true);

    assert_eq!(mailer.len(), 1, "exactly one reset email must be captured");
    let token = extract_token(&mailer);

    let new_password = STRONG_PASSWORD;
    let response = post_json(
        &app,
        "/api/v1/auth/reset-password",
        serde_json::json!({ "token": token, "new_password": new_password }),
    )
    .await;
    assert_eq!(response.status(), StatusCode::OK, "reset must succeed");
    assert_eq!(body_json(response).await["ok"], true);

    let response = login(&app, &email, old_password.as_str()).await;
    assert_eq!(
        response.status(),
        StatusCode::UNAUTHORIZED,
        "old password must no longer work"
    );

    let response = login(&app, &email, new_password).await;
    assert_eq!(
        response.status(),
        StatusCode::OK,
        "new password must work after reset"
    );

    let response = post_json(
        &app,
        "/api/v1/auth/reset-password",
        serde_json::json!({ "token": token, "new_password": "Another-Pass-456" }),
    )
    .await;
    assert_eq!(
        response.status(),
        StatusCode::BAD_REQUEST,
        "a reset token is one-shot and must not be reusable"
    );
}

#[sqlx::test(migrations = "../godwit-db/migrations")]
async fn forgot_unknown_email_returns_200(pool: PgPool) {
    let app = build_app(pool, None);
    let response = post_json(
        &app,
        "/api/v1/auth/forgot-password",
        serde_json::json!({ "email": "nobody@example.com" }),
    )
    .await;
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(body_json(response).await["ok"], true);
}

#[sqlx::test(migrations = "../godwit-db/migrations")]
async fn login_with_forced_change_returns_flag(pool: PgPool) {
    let (email, password) = seed_password_user(&pool).await;
    let app = build_app(pool.clone(), None);

    let response = login(&app, &email, &password).await;
    assert_eq!(response.status(), StatusCode::OK);
    let body = body_json(response).await;
    assert_eq!(
        body["must_change_password"], false,
        "a normal user should not be flagged for forced change"
    );

    let org = OrganizationRepository::new(pool.clone())
        .create("pw-test-org-2", None)
        .await
        .expect("create org");
    let forced_email = "forced@example.com";
    let user = UserRepository::new(pool.clone())
        .create(forced_email, None, godwit_db::models::UserRole::User, Some(org.id))
        .await
        .expect("create user");
    let forced_password = "correct-horse-battery-staple";
    let hash = godwit_auth::api_keys::hash_password(forced_password);
    sqlx::query("UPDATE users SET password_hash = $1 WHERE id = $2")
        .bind(&hash)
        .bind(user.id)
        .execute(&pool)
        .await
        .expect("set password hash");
    UserRepository::new(pool.clone())
        .set_must_change(user.id, true)
        .await
        .expect("set must change");

    let response = login(&app, forced_email, forced_password).await;
    assert_eq!(response.status(), StatusCode::OK);
    let body = body_json(response).await;
    assert_eq!(
        body["must_change_password"], true,
        "a user flagged for forced change must carry the flag in the login body"
    );
}

#[sqlx::test(migrations = "../godwit-db/migrations")]
async fn change_password_happy_path(pool: PgPool) {
    let (email, password) = seed_password_user(&pool).await;
    let app = build_app(pool, None);

    let response = login(&app, &email, &password).await;
    assert_eq!(response.status(), StatusCode::OK);
    let body = body_json(response).await;
    let access_token = body["access_token"]
        .as_str()
        .expect("access_token present")
        .to_string();

    let new_password = STRONG_PASSWORD;
    let response = post_json_auth(
        &app,
        "/api/v1/auth/change-password",
        &access_token,
        serde_json::json!({ "current_password": password, "new_password": new_password }),
    )
    .await;
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(body_json(response).await["changed"], true);

    let response = login(&app, &email, new_password).await;
    assert_eq!(
        response.status(),
        StatusCode::OK,
        "new password must work after change-password"
    );
}
