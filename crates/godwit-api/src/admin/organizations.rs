use axum::{
    extract::{Extension, State},
    routing::get,
    Json, Router,
};
use godwit_auth::{jwt::Claims, rbac::Role};
use std::sync::Arc;

use crate::{error::ApiError, state::AppState};

pub fn router() -> Router<Arc<AppState>> {
    Router::new().route("/organizations", get(list_organizations))
}

async fn list_organizations(
    State(state): State<Arc<AppState>>,
    Extension(claims): Extension<Claims>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let role = Role::from_str(&claims.role).ok_or(ApiError::Forbidden)?;
    if role != Role::SuperAdmin {
        return Err(ApiError::Forbidden);
    }
    let orgs = state
        .org_repo
        .list()
        .await
        .map_err(ApiError::Core)?;
    Ok(Json(serde_json::json!({ "data": orgs })))
}
