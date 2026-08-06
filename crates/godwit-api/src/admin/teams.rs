use axum::{
    extract::{Extension, Path, Query, State},
    routing::{delete, get, post},
    Json, Router,
};
use godwit_auth::{jwt::Claims, rbac::Role};
use serde::Deserialize;
use std::sync::Arc;
use uuid::Uuid;

use crate::{error::ApiError, state::AppState};

pub fn router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/teams", get(list_teams).post(create_team))
        .route(
            "/teams/:id",
            get(get_team).patch(update_team).delete(delete_team),
        )
        .route("/teams/:id/members", post(add_member))
        .route("/teams/:id/members/:user_id", delete(remove_member))
}

fn require_manage_users(claims: &Claims) -> Result<Role, ApiError> {
    let role = Role::from_str(&claims.role).ok_or(ApiError::Forbidden)?;
    if !role.can_manage_users() {
        return Err(ApiError::Forbidden);
    }
    Ok(role)
}

#[derive(Deserialize)]
pub struct ListTeamsQuery {
    organization_id: Option<Uuid>,
}

async fn list_teams(
    State(state): State<Arc<AppState>>,
    Extension(claims): Extension<Claims>,
    Query(query): Query<ListTeamsQuery>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let role = require_manage_users(&claims)?;
    let teams = if role == Role::SuperAdmin {
        match query.organization_id {
            Some(org_id) => state.team_repo.list_for_organization(org_id).await,
            None => state.team_repo.list_all().await,
        }
    } else {
        state
            .team_repo
            .list_for_organization(claims.organization_id)
            .await
    }
    .map_err(ApiError::Core)?;
    Ok(Json(serde_json::json!({ "data": teams })))
}

async fn get_team(
    State(state): State<Arc<AppState>>,
    Extension(claims): Extension<Claims>,
    Path(id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let role = require_manage_users(&claims)?;
    let team = state.team_repo.get_by_id(id).await.map_err(ApiError::Core)?;
    if role != Role::SuperAdmin && team.organization_id != claims.organization_id {
        return Err(ApiError::Forbidden);
    }
    Ok(Json(serde_json::json!({ "data": team })))
}

#[derive(Deserialize)]
pub struct CreateTeamRequest {
    name: String,
    organization_id: Option<Uuid>,
    budget_usd: Option<rust_decimal::Decimal>,
    max_budget_usd: Option<rust_decimal::Decimal>,
}

async fn create_team(
    State(state): State<Arc<AppState>>,
    Extension(claims): Extension<Claims>,
    Json(req): Json<CreateTeamRequest>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let role = require_manage_users(&claims)?;
    let organization_id = if role == Role::SuperAdmin {
        req.organization_id
            .ok_or_else(|| ApiError::BadRequest("organization_id is required".to_string()))?
    } else {
        claims.organization_id
    };
    let team = state
        .team_repo
        .create(organization_id, &req.name, req.budget_usd, req.max_budget_usd)
        .await
        .map_err(ApiError::Core)?;
    Ok(Json(serde_json::json!({ "data": team })))
}

#[derive(Deserialize)]
pub struct UpdateTeamRequest {
    name: Option<String>,
    budget_usd: Option<rust_decimal::Decimal>,
    max_budget_usd: Option<rust_decimal::Decimal>,
}

async fn update_team(
    State(state): State<Arc<AppState>>,
    Extension(claims): Extension<Claims>,
    Path(id): Path<Uuid>,
    Json(req): Json<UpdateTeamRequest>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let role = require_manage_users(&claims)?;
    let team = state
        .team_repo
        .get_by_id(id)
        .await
        .map_err(ApiError::Core)?;
    if role != Role::SuperAdmin && team.organization_id != claims.organization_id {
        return Err(ApiError::Forbidden);
    }
    let new_name = req.name.unwrap_or(team.name.clone());
    let new_budget = req.budget_usd.or(team.budget_usd);
    let new_max_budget = req.max_budget_usd.or(team.max_budget_usd);
    let updated = state
        .team_repo
        .update_with_budget(id, &new_name, new_budget, new_max_budget)
        .await
        .map_err(ApiError::Core)?;
    Ok(Json(serde_json::json!({ "data": updated })))
}

async fn delete_team(
    State(state): State<Arc<AppState>>,
    Extension(claims): Extension<Claims>,
    Path(id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let role = require_manage_users(&claims)?;
    let team = state.team_repo.get_by_id(id).await.map_err(ApiError::Core)?;
    if role != Role::SuperAdmin && team.organization_id != claims.organization_id {
        return Err(ApiError::Forbidden);
    }
    state.team_repo.delete(id).await.map_err(ApiError::Core)?;
    Ok(Json(serde_json::json!({ "deleted": true })))
}

async fn require_team_manage(
    state: &AppState,
    claims: &Claims,
    team_id: Uuid,
) -> Result<(), ApiError> {
    let role = Role::from_str(&claims.role).ok_or(ApiError::Forbidden)?;
    if role == Role::SuperAdmin {
        return Ok(());
    }
    let team = state
        .team_repo
        .get_by_id(team_id)
        .await
        .map_err(ApiError::Core)?;
    if role == Role::OrgAdmin && team.organization_id == claims.organization_id {
        return Ok(());
    }
    // A team_admin (or an org_admin of a *different* org, rejected above) must hold
    // team_admin membership for THIS specific team — not just the global role.
    match state
        .team_membership_repo
        .get_membership(team_id, claims.user_id)
        .await
    {
        Ok(membership) if membership.role == "team_admin" => Ok(()),
        _ => Err(ApiError::Forbidden),
    }
}

#[derive(Deserialize)]
pub struct AddMemberRequest {
    user_id: Uuid,
    role: String,
}

async fn add_member(
    State(state): State<Arc<AppState>>,
    Extension(claims): Extension<Claims>,
    Path(team_id): Path<Uuid>,
    Json(req): Json<AddMemberRequest>,
) -> Result<Json<serde_json::Value>, ApiError> {
    require_team_manage(&state, &claims, team_id).await?;
    if req.role != "team_admin" && req.role != "member" {
        return Err(ApiError::BadRequest(
            "role must be 'team_admin' or 'member'".to_string(),
        ));
    }
    let membership = state
        .team_membership_repo
        .add_member(team_id, req.user_id, &req.role)
        .await
        .map_err(ApiError::Core)?;
    Ok(Json(serde_json::json!({ "data": membership })))
}

async fn remove_member(
    State(state): State<Arc<AppState>>,
    Extension(claims): Extension<Claims>,
    Path((team_id, user_id)): Path<(Uuid, Uuid)>,
) -> Result<Json<serde_json::Value>, ApiError> {
    require_team_manage(&state, &claims, team_id).await?;
    state
        .team_membership_repo
        .remove_member(team_id, user_id)
        .await
        .map_err(ApiError::Core)?;
    Ok(Json(serde_json::json!({ "removed": true })))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn create_team_request_deserializes_without_organization_id() {
        let json = r#"{"name":"engineering"}"#;
        let req: CreateTeamRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.name, "engineering");
        assert_eq!(req.organization_id, None);
    }
}
