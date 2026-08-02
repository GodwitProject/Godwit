use axum::{
    extract::{Extension, Path, State},
    routing::get,
    Json, Router,
};
use godwit_auth::{jwt::Claims, rbac::Role};
use serde::Deserialize;
use std::sync::Arc;
use uuid::Uuid;

use crate::{error::ApiError, state::AppState};

#[derive(Deserialize)]
pub struct CreateUserRequest {
    email: String,
    name: Option<String>,
    role: String,
}

pub fn router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/users", get(list_users).post(create_user))
        .route("/users/:id", get(get_user).patch(update_user).delete(delete_user))
}

fn require_role(claims: &Claims, allowed: &[Role]) -> Result<Role, ApiError> {
    let role = Role::from_str(&claims.role).ok_or(ApiError::Forbidden)?;
    if !allowed.contains(&role) {
        return Err(ApiError::Forbidden);
    }
    Ok(role)
}

async fn list_users(
    State(state): State<Arc<AppState>>,
    Extension(claims): Extension<Claims>,
) -> Result<Json<serde_json::Value>, ApiError> {
    require_role(&claims, &[Role::SuperAdmin, Role::OrgAdmin])?;
    let users = state
        .user_repo
        .list_for_organization(claims.organization_id)
        .await
        .map_err(ApiError::Core)?;
    Ok(Json(serde_json::json!({ "data": users })))
}

async fn create_user(
    State(state): State<Arc<AppState>>,
    Extension(claims): Extension<Claims>,
    Json(req): Json<CreateUserRequest>,
) -> Result<Json<serde_json::Value>, ApiError> {
    require_role(&claims, &[Role::SuperAdmin, Role::OrgAdmin])?;
    let role = godwit_db::models::UserRole::from_str(&req.role).ok_or(ApiError::BadRequest("invalid role".to_string()))?;
    let org_id = claims.organization_id;
    let user = state
        .user_repo
        .create(&req.email, req.name.as_deref(), role, Some(org_id))
        .await
        .map_err(ApiError::Core)?;
    Ok(Json(serde_json::json!({ "data": user })))
}

async fn get_user(
    State(_state): State<Arc<AppState>>,
    Extension(_claims): Extension<Claims>,
    Path(_id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, ApiError> {
    Err(ApiError::NotFound)
}

async fn update_user(
    State(_state): State<Arc<AppState>>,
    Extension(_claims): Extension<Claims>,
    Path(_id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, ApiError> {
    Err(ApiError::NotFound)
}

async fn delete_user(
    State(_state): State<Arc<AppState>>,
    Extension(_claims): Extension<Claims>,
    Path(_id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, ApiError> {
    Err(ApiError::NotFound)
}
