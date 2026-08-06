use axum::{
    extract::{Extension, Path, State},
    routing::get,
    Json, Router,
};
use godwit_auth::jwt::Claims;
use serde::Deserialize;
use std::sync::Arc;
use uuid::Uuid;

use crate::{admin::require_super_admin, error::ApiError, state::AppState};

pub fn router() -> Router<Arc<AppState>> {
    Router::new()
        .route(
            "/organizations",
            get(list_organizations).post(create_organization),
        )
        .route(
            "/organizations/:id",
            get(get_organization)
                .patch(update_organization)
                .delete(delete_organization),
        )
}

async fn get_organization(
    State(state): State<Arc<AppState>>,
    Extension(claims): Extension<Claims>,
    Path(id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, ApiError> {
    require_super_admin(&claims)?;
    let org = state.org_repo.get_by_id(id).await.map_err(ApiError::Core)?;
    Ok(Json(serde_json::json!({ "data": org })))
}

async fn list_organizations(
    State(state): State<Arc<AppState>>,
    Extension(claims): Extension<Claims>,
) -> Result<Json<serde_json::Value>, ApiError> {
    require_super_admin(&claims)?;
    let orgs = state.org_repo.list().await.map_err(ApiError::Core)?;
    Ok(Json(serde_json::json!({ "data": orgs })))
}

#[derive(Deserialize)]
pub struct CreateOrganizationRequest {
    name: String,
    rate_limit_requests_per_minute: Option<i32>,
    rate_limit_tokens_per_minute: Option<i32>,
}

async fn create_organization(
    State(state): State<Arc<AppState>>,
    Extension(claims): Extension<Claims>,
    Json(req): Json<CreateOrganizationRequest>,
) -> Result<Json<serde_json::Value>, ApiError> {
    require_super_admin(&claims)?;
    let org = state
        .org_repo
        .create_with_limits(
            &req.name,
            req.rate_limit_requests_per_minute,
            req.rate_limit_tokens_per_minute,
        )
        .await
        .map_err(ApiError::Core)?;
    Ok(Json(serde_json::json!({ "data": org })))
}

#[derive(Deserialize)]
pub struct UpdateOrganizationRequest {
    name: Option<String>,
    rate_limit_requests_per_minute: Option<i32>,
    rate_limit_tokens_per_minute: Option<i32>,
}

async fn update_organization(
    State(state): State<Arc<AppState>>,
    Extension(claims): Extension<Claims>,
    Path(id): Path<Uuid>,
    Json(req): Json<UpdateOrganizationRequest>,
) -> Result<Json<serde_json::Value>, ApiError> {
    require_super_admin(&claims)?;
    let org = state
        .org_repo
        .update_with_limits(
            id,
            req.name.as_deref(),
            req.rate_limit_requests_per_minute,
            req.rate_limit_tokens_per_minute,
        )
        .await
        .map_err(ApiError::Core)?;
    Ok(Json(serde_json::json!({ "data": org })))
}

async fn delete_organization(
    State(state): State<Arc<AppState>>,
    Extension(claims): Extension<Claims>,
    Path(id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, ApiError> {
    require_super_admin(&claims)?;
    state.org_repo.delete(id).await.map_err(ApiError::Core)?;
    Ok(Json(serde_json::json!({ "deleted": true })))
}
