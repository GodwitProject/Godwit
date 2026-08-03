use axum::{
    extract::{Extension, Path, Query, State},
    routing::{get, patch},
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
        .route("/teams/:id", patch(update_team))
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

#[derive(Deserialize)]
pub struct CreateTeamRequest {
    name: String,
    organization_id: Option<Uuid>,
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
        .create(organization_id, &req.name)
        .await
        .map_err(ApiError::Core)?;
    Ok(Json(serde_json::json!({ "data": team })))
}

#[derive(Deserialize)]
pub struct UpdateTeamRequest {
    name: String,
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
    let updated = state
        .team_repo
        .update(id, &req.name)
        .await
        .map_err(ApiError::Core)?;
    Ok(Json(serde_json::json!({ "data": updated })))
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
