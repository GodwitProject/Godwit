use axum::{
    extract::{Extension, State},
    routing::get,
    Json, Router,
};
use pasteurllm_auth::{jwt::Claims, rbac::Role};
use pasteurllm_db::repositories::models::ModelRepository;
use std::sync::Arc;

use crate::{error::ApiError, state::AppState};

pub fn router() -> Router<Arc<AppState>> {
    Router::new().route("/models", get(list_models))
}

async fn list_models(
    State(state): State<Arc<AppState>>,
    Extension(claims): Extension<Claims>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let role = Role::from_str(&claims.role).ok_or(ApiError::Forbidden)?;
    if !role.can_manage_api_keys() {
        return Err(ApiError::Forbidden);
    }
    let repo = ModelRepository::new(state.pool.clone());
    let models = repo
        .list_for_organization(claims.organization_id)
        .await
        .map_err(ApiError::Core)?;
    Ok(Json(serde_json::json!({ "data": models })))
}
