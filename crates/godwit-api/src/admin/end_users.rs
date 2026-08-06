use axum::{
    extract::{Extension, Path, Query, State},
    routing::{delete, get, patch, post},
    Json, Router,
};
use godwit_auth::{jwt::Claims, rbac::Role};
use godwit_db::repositories::end_users::EndUsersRepository;
use serde::Deserialize;
use std::sync::Arc;
use uuid::Uuid;

use crate::{error::ApiError, state::AppState};

pub fn router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/end-users", get(list_end_users).post(create_end_user))
        .route(
            "/end-users/:user_id",
            get(get_end_user).patch(update_end_user).delete(delete_end_user),
        )
}

fn require_manage_users(claims: &Claims) -> Result<Role, ApiError> {
    let role = Role::from_str(&claims.role).ok_or(ApiError::Forbidden)?;
    if !role.can_manage_users() {
        return Err(ApiError::Forbidden);
    }
    Ok(role)
}

#[derive(Deserialize)]
pub struct ListEndUsersQuery {
    organization_id: Option<Uuid>,
}

async fn list_end_users(
    State(state): State<Arc<AppState>>,
    Extension(claims): Extension<Claims>,
    Query(query): Query<ListEndUsersQuery>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let role = require_manage_users(&claims)?;
    let repo = EndUsersRepository::new(state.pool.clone());
    
    let end_users = if role == Role::SuperAdmin {
        match query.organization_id {
            Some(org_id) => repo.list_by_organization(org_id).await,
            None => repo.list_by_organization(claims.organization_id).await,
        }
    } else {
        repo.list_by_organization(claims.organization_id).await
    }
    .map_err(ApiError::Core)?;
    
    Ok(Json(serde_json::json!({ "data": end_users })))
}

async fn get_end_user(
    State(state): State<Arc<AppState>>,
    Extension(claims): Extension<Claims>,
    Path(user_id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let role = require_manage_users(&claims)?;
    let repo = EndUsersRepository::new(state.pool.clone());
    
    let organization_id = if role == Role::SuperAdmin {
        claims.organization_id
    } else {
        claims.organization_id
    };
    
    let end_user = repo
        .get_by_user(organization_id, user_id)
        .await
        .map_err(ApiError::Core)?;
    
    Ok(Json(serde_json::json!({ "data": end_user })))
}

#[derive(Deserialize)]
pub struct CreateEndUserRequest {
    user_id: Uuid,
    organization_id: Option<Uuid>,
    budget_usd: Option<rust_decimal::Decimal>,
    max_budget_usd: Option<rust_decimal::Decimal>,
}

async fn create_end_user(
    State(state): State<Arc<AppState>>,
    Extension(claims): Extension<Claims>,
    Json(req): Json<CreateEndUserRequest>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let role = require_manage_users(&claims)?;
    let repo = EndUsersRepository::new(state.pool.clone());
    
    let organization_id = if role == Role::SuperAdmin {
        req.organization_id
            .ok_or_else(|| ApiError::BadRequest("organization_id is required".to_string()))?
    } else {
        claims.organization_id
    };
    
    let end_user = repo
        .create(organization_id, req.user_id, req.budget_usd, req.max_budget_usd)
        .await
        .map_err(ApiError::Core)?;
    
    Ok(Json(serde_json::json!({ "data": end_user })))
}

#[derive(Deserialize)]
pub struct UpdateEndUserRequest {
    budget_usd: Option<rust_decimal::Decimal>,
    max_budget_usd: Option<rust_decimal::Decimal>,
}

async fn update_end_user(
    State(state): State<Arc<AppState>>,
    Extension(claims): Extension<Claims>,
    Path(user_id): Path<Uuid>,
    Json(req): Json<UpdateEndUserRequest>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let role = require_manage_users(&claims)?;
    let repo = EndUsersRepository::new(state.pool.clone());
    
    let organization_id = claims.organization_id;
    
    let end_user = repo
        .get_by_user(organization_id, user_id)
        .await
        .map_err(ApiError::Core)?;
    
    if role != Role::SuperAdmin && end_user.organization_id != claims.organization_id {
        return Err(ApiError::Forbidden);
    }
    
    let updated = repo
        .update_budgets(organization_id, user_id, req.budget_usd, req.max_budget_usd)
        .await
        .map_err(ApiError::Core)?;
    
    Ok(Json(serde_json::json!({ "data": updated })))
}

async fn delete_end_user(
    State(state): State<Arc<AppState>>,
    Extension(claims): Extension<Claims>,
    Path(user_id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let role = require_manage_users(&claims)?;
    let repo = EndUsersRepository::new(state.pool.clone());
    
    let organization_id = claims.organization_id;
    
    let end_user = repo
        .get_by_user(organization_id, user_id)
        .await
        .map_err(ApiError::Core)?;
    
    if role != Role::SuperAdmin && end_user.organization_id != claims.organization_id {
        return Err(ApiError::Forbidden);
    }
    
    repo.delete(organization_id, user_id)
        .await
        .map_err(ApiError::Core)?;
    
    Ok(Json(serde_json::json!({ "deleted": true })))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn create_end_user_request_deserializes_without_organization_id() {
        let json = r#"{"user_id":"550e8400-e29b-41d4-a716-446655440000"}"#;
        let req: CreateEndUserRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.user_id.to_string(), "550e8400-e29b-41d4-a716-446655440000");
        assert_eq!(req.organization_id, None);
    }
}
