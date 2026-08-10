use axum::{
    extract::{Extension, Path, State},
    routing::{get, patch},
    Json, Router,
};
use godwit_auth::{credentials::encrypt_api_key, jwt::Claims};
use godwit_db::repositories::provider_profiles::ProviderProfileRepository;
use serde::Deserialize;
use std::sync::Arc;
use uuid::Uuid;

use crate::{admin::require_super_admin, error::ApiError, state::AppState};

pub fn router() -> Router<Arc<AppState>> {
    Router::new()
        .route(
            "/provider-profiles",
            get(list_profiles).post(create_profile),
        )
        .route("/provider-profiles/:id", patch(update_profile).delete(delete_profile))
}

fn profile_json(profile: &godwit_db::models::ProviderProfile) -> serde_json::Value {
    serde_json::json!({
        "id": profile.id,
        "name": profile.name,
        "protocol": profile.protocol,
        "base_url": profile.base_url,
        "allow_wildcard": profile.allow_wildcard,
        "enabled": profile.enabled,
        "has_credentials": !profile.auth.is_null() && profile.auth != serde_json::json!({}),
        "created_at": profile.created_at,
    })
}

async fn list_profiles(
    State(state): State<Arc<AppState>>,
    Extension(claims): Extension<Claims>,
) -> Result<Json<serde_json::Value>, ApiError> {
    require_super_admin(&claims)?;
    let repo = ProviderProfileRepository::new(state.pool.clone());
    let profiles = repo.list().await.map_err(ApiError::Core)?;
    Ok(Json(
        serde_json::json!({ "data": profiles.iter().map(profile_json).collect::<Vec<_>>() }),
    ))
}

#[derive(Debug, Deserialize)]
pub struct CreateProfileRequest {
    pub name: String,
    pub protocol: String,
    pub base_url: Option<String>,
    pub api_key: Option<String>,
    #[serde(default)]
    pub allow_wildcard: bool,
}

async fn create_profile(
    State(state): State<Arc<AppState>>,
    Extension(claims): Extension<Claims>,
    Json(req): Json<CreateProfileRequest>,
) -> Result<Json<serde_json::Value>, ApiError> {
    require_super_admin(&claims)?;
    let repo = ProviderProfileRepository::new(state.pool.clone());
    let profile = repo
        .create(
            &req.name,
            &req.protocol,
            req.base_url.as_deref(),
            req.allow_wildcard,
        )
        .await
        .map_err(ApiError::Core)?;
    let profile = if let Some(api_key) = req.api_key {
        let secret = encrypt_api_key(&state.credential_master_key, &api_key);
        repo.set_auth(profile.id, &secret)
            .await
            .map_err(ApiError::Core)?
    } else {
        profile
    };
    Ok(Json(profile_json(&profile)))
}

#[derive(Debug, Deserialize)]
pub struct UpdateProfileRequest {
    pub base_url: Option<String>,
    pub api_key: Option<String>,
    pub allow_wildcard: Option<bool>,
    pub enabled: Option<bool>,
}

async fn update_profile(
    State(state): State<Arc<AppState>>,
    Extension(claims): Extension<Claims>,
    Path(id): Path<Uuid>,
    Json(req): Json<UpdateProfileRequest>,
) -> Result<Json<serde_json::Value>, ApiError> {
    require_super_admin(&claims)?;
    let repo = ProviderProfileRepository::new(state.pool.clone());
    let profile = repo
        .update(id, req.base_url.as_deref(), req.allow_wildcard, req.enabled)
        .await
        .map_err(ApiError::Core)?;
    let profile = if let Some(api_key) = req.api_key {
        let secret = encrypt_api_key(&state.credential_master_key, &api_key);
        repo.set_auth(profile.id, &secret)
            .await
            .map_err(ApiError::Core)?
    } else {
        profile
    };
    Ok(Json(profile_json(&profile)))
}

async fn delete_profile(
    State(state): State<Arc<AppState>>,
    Extension(claims): Extension<Claims>,
    Path(id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, ApiError> {
    require_super_admin(&claims)?;
    let repo = ProviderProfileRepository::new(state.pool.clone());
    repo.delete(id).await?;
    Ok(Json(serde_json::json!({ "deleted": true })))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn profile_json_never_includes_raw_auth() {
        let profile = godwit_db::models::ProviderProfile {
            id: Uuid::nil(),
            name: "openai".to_string(),
            protocol: "openai".to_string(),
            base_url: Some("https://api.openai.com/v1".to_string()),
            allow_wildcard: false,
            auth: serde_json::json!({"ciphertext": "abc", "nonce": "def"}),
            config: serde_json::json!({}),
            enabled: true,
            created_at: chrono::Utc::now(),
        };
        let json = profile_json(&profile);
        assert_eq!(json["has_credentials"], true);
        assert!(json.get("auth").is_none());
        assert!(json.get("ciphertext").is_none());
    }

    #[test]
    fn profile_json_has_credentials_false_when_auth_empty() {
        let profile = godwit_db::models::ProviderProfile {
            id: Uuid::nil(),
            name: "openai".to_string(),
            protocol: "openai".to_string(),
            base_url: None,
            allow_wildcard: false,
            auth: serde_json::json!({}),
            config: serde_json::json!({}),
            enabled: true,
            created_at: chrono::Utc::now(),
        };
        let json = profile_json(&profile);
        assert_eq!(json["has_credentials"], false);
    }
}
