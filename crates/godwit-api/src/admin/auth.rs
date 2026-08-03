use axum::{
    extract::{Path, Query, State},
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
async fn issue_token_pair(
    state: &AppState,
    user: &User,
) -> Result<serde_json::Value, crate::error::ApiError> {
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

    Ok(serde_json::json!({ "access_token": access_token, "refresh_token": refresh_plaintext }))
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
    Ok(Json(issue_token_pair(&state, &user).await?))
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
    Ok(Json(issue_token_pair(&state, &user).await?))
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
    Ok(Json(issue_token_pair(&state, &user).await?))
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
