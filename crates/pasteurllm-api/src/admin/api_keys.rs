use axum::{
    extract::{Extension, State},
    routing::get,
    Json, Router,
};
use pasteurllm_auth::{api_keys::generate_api_key, jwt::Claims, rbac::Role};
use serde::Deserialize;
use std::sync::Arc;

use crate::{error::ApiError, state::AppState};

#[derive(Deserialize)]
pub struct CreateApiKeyRequest {
    name: String,
    scopes: Vec<String>,
}

pub fn router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/api-keys", get(list_api_keys).post(create_api_key))
}

async fn list_api_keys(
    State(state): State<Arc<AppState>>,
    Extension(claims): Extension<Claims>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let role = Role::from_str(&claims.role).ok_or(ApiError::Forbidden)?;
    if !role.can_manage_api_keys() {
        return Err(ApiError::Forbidden);
    }
    let keys = state
        .api_key_repo
        .list_for_organization(claims.organization_id)
        .await
        .map_err(ApiError::Core)?;
    Ok(Json(serde_json::json!({ "data": keys })))
}

async fn create_api_key(
    State(state): State<Arc<AppState>>,
    Extension(claims): Extension<Claims>,
    Json(req): Json<CreateApiKeyRequest>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let role = Role::from_str(&claims.role).ok_or(ApiError::Forbidden)?;
    if !role.can_manage_api_keys() {
        return Err(ApiError::Forbidden);
    }
    let (plaintext, hash, prefix) = generate_api_key();
    let key = state
        .api_key_repo
        .create(
            claims.user_id,
            claims.organization_id,
            &req.name,
            &prefix,
            &hash,
            &req.scopes,
            None,
            None,
        )
        .await
        .map_err(ApiError::Core)?;
    Ok(Json(serde_json::json!({
        "id": key.id,
        "key": plaintext,
        "name": key.name,
    })))
}
