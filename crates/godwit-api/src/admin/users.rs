use axum::{
    extract::{Extension, Path, Query, State},
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
        .route(
            "/users/:id",
            get(get_user).patch(update_user).delete(delete_user),
        )
}

fn require_role(claims: &Claims, allowed: &[Role]) -> Result<Role, ApiError> {
    let role = Role::from_str(&claims.role).ok_or(ApiError::Forbidden)?;
    if !allowed.contains(&role) {
        return Err(ApiError::Forbidden);
    }
    Ok(role)
}

/// `org_admin` may only act on a user already in its own org; `super_admin` may act on anyone.
fn check_same_org(role: Role, claims: &Claims, target_org: Option<Uuid>) -> Result<(), ApiError> {
    if role != Role::SuperAdmin && target_org != Some(claims.organization_id) {
        return Err(ApiError::Forbidden);
    }
    Ok(())
}

/// A caller who is not itself `super_admin` may never act on a target that already holds
/// `super_admin` — regardless of same-org membership. Without this, an `org_admin` sharing
/// an org with the instance's `super_admin` (the default case, since `create_user` always
/// places new users in the creator's own org) could demote or delete it via `check_same_org`
/// alone, since that check only compares organizations, never privilege level.
fn check_not_acting_on_super_admin(role: Role, target_role: &str) -> Result<(), ApiError> {
    if role != Role::SuperAdmin && target_role == "super_admin" {
        return Err(ApiError::Forbidden);
    }
    Ok(())
}

#[derive(Deserialize)]
pub struct ListUsersQuery {
    organization_id: Option<Uuid>,
}

async fn list_users(
    State(state): State<Arc<AppState>>,
    Extension(claims): Extension<Claims>,
    Query(query): Query<ListUsersQuery>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let role = require_role(&claims, &[Role::SuperAdmin, Role::OrgAdmin])?;
    let users = if role == Role::SuperAdmin {
        match query.organization_id {
            Some(org_id) => state.user_repo.list_for_organization(org_id).await,
            None => state.user_repo.list_all().await,
        }
    } else {
        state.user_repo.list_for_organization(claims.organization_id).await
    }
    .map_err(ApiError::Core)?;
    Ok(Json(serde_json::json!({ "data": users })))
}

async fn create_user(
    State(state): State<Arc<AppState>>,
    Extension(claims): Extension<Claims>,
    Json(req): Json<CreateUserRequest>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let caller_role = require_role(&claims, &[Role::SuperAdmin, Role::OrgAdmin])?;
    let role = godwit_db::models::UserRole::from_str(&req.role)
        .ok_or(ApiError::BadRequest("invalid role".to_string()))?;
    if req.role == "super_admin" && caller_role != Role::SuperAdmin {
        return Err(ApiError::Forbidden);
    }
    let org_id = claims.organization_id;
    let user = state
        .user_repo
        .create(&req.email, req.name.as_deref(), role, Some(org_id))
        .await
        .map_err(ApiError::Core)?;
    Ok(Json(serde_json::json!({ "data": user })))
}

async fn get_user(
    State(state): State<Arc<AppState>>,
    Extension(claims): Extension<Claims>,
    Path(id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let role = require_role(&claims, &[Role::SuperAdmin, Role::OrgAdmin])?;
    let user = state.user_repo.get_by_id(id).await.map_err(ApiError::Core)?;
    check_same_org(role, &claims, user.organization_id)?;
    Ok(Json(serde_json::json!({ "data": user })))
}

#[derive(Deserialize)]
pub struct UpdateUserRequest {
    name: Option<String>,
    role: Option<String>,
    organization_id: Option<Uuid>,
}

async fn update_user(
    State(state): State<Arc<AppState>>,
    Extension(claims): Extension<Claims>,
    Path(id): Path<Uuid>,
    Json(req): Json<UpdateUserRequest>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let role = require_role(&claims, &[Role::SuperAdmin, Role::OrgAdmin])?;
    if req.role.is_some() && claims.user_id == id {
        return Err(ApiError::BadRequest(
            "cannot change your own role".to_string(),
        ));
    }
    let target = state.user_repo.get_by_id(id).await.map_err(ApiError::Core)?;
    check_same_org(role, &claims, target.organization_id)?;
    check_not_acting_on_super_admin(role, &target.role)?;
    if req.organization_id.is_some() && role != Role::SuperAdmin {
        return Err(ApiError::Forbidden);
    }
    if let Some(ref role_str) = req.role {
        godwit_db::models::UserRole::from_str(role_str)
            .ok_or(ApiError::BadRequest("invalid role".to_string()))?;
        if role_str == "super_admin" && role != Role::SuperAdmin {
            return Err(ApiError::Forbidden);
        }
    }
    let reassigning_org =
        req.organization_id.is_some() && req.organization_id != target.organization_id;
    let updated = state
        .user_repo
        .update(id, req.name.as_deref(), req.role.as_deref(), req.organization_id)
        .await
        .map_err(ApiError::Core)?;
    if reassigning_org {
        // A team membership only makes sense within the org the team belongs to, and this
        // branch doesn't support cross-org team membership: dropping all of this user's
        // memberships here prevents the authorization-continuation bug where a moved user
        // keeps team-management rights over its old org's teams (see
        // `TeamMembershipRepository::remove_all_for_user`).
        state
            .team_membership_repo
            .remove_all_for_user(id)
            .await
            .map_err(ApiError::Core)?;
    }
    Ok(Json(serde_json::json!({ "data": updated })))
}

async fn delete_user(
    State(state): State<Arc<AppState>>,
    Extension(claims): Extension<Claims>,
    Path(id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let role = require_role(&claims, &[Role::SuperAdmin, Role::OrgAdmin])?;
    if claims.user_id == id {
        return Err(ApiError::BadRequest("cannot delete your own account".to_string()));
    }
    let target = state.user_repo.get_by_id(id).await.map_err(ApiError::Core)?;
    check_same_org(role, &claims, target.organization_id)?;
    check_not_acting_on_super_admin(role, &target.role)?;
    state.user_repo.delete(id).await.map_err(ApiError::Core)?;
    Ok(Json(serde_json::json!({ "deleted": true })))
}
