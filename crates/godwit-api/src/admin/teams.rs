use axum::{
    extract::{Extension, State},
    routing::get,
    Json, Router,
};
use godwit_auth::{jwt::Claims, rbac::Role};
use std::sync::Arc;

use crate::{error::ApiError, state::AppState};

pub fn router() -> Router<Arc<AppState>> {
    Router::new().route("/teams", get(list_teams))
}

async fn list_teams(
    _state: State<Arc<AppState>>,
    Extension(claims): Extension<Claims>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let role = Role::from_str(&claims.role).ok_or(ApiError::Forbidden)?;
    if !role.can_manage_users() {
        return Err(ApiError::Forbidden);
    }
    Ok(Json(serde_json::json!({ "data": [] })))
}
