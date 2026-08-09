use axum::{
    extract::{Extension, Path, State},
    routing::{delete, get, post},
    Json, Router,
};
use godwit_auth::jwt::Claims;
use godwit_db::repositories::{model_aliases::ModelAliasRepository, models::ModelRepository};
use serde::Deserialize;
use std::sync::Arc;
use uuid::Uuid;

use crate::{admin::require_super_admin, error::ApiError, state::AppState};

pub fn router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/model-aliases", get(list_aliases).post(create_alias))
        .route("/model-aliases/:id", delete(delete_alias))
}

async fn list_aliases(
    State(state): State<Arc<AppState>>,
    Extension(claims): Extension<Claims>,
) -> Result<Json<serde_json::Value>, ApiError> {
    require_super_admin(&claims)?;
    let repo = ModelAliasRepository::new(state.pool.clone());
    let aliases = repo.list().await.map_err(ApiError::Core)?;
    Ok(Json(serde_json::json!({ "data": aliases })))
}

#[derive(Debug, Deserialize)]
pub struct CreateAliasRequest {
    pub alias: String,
    pub target_model_id: Uuid,
}

async fn create_alias(
    State(state): State<Arc<AppState>>,
    Extension(claims): Extension<Claims>,
    Json(req): Json<CreateAliasRequest>,
) -> Result<Json<serde_json::Value>, ApiError> {
    require_super_admin(&claims)?;
    let alias_repo = ModelAliasRepository::new(state.pool.clone());
    let model_repo = ModelRepository::new(state.pool.clone());

    model_repo.get(req.target_model_id).await.map_err(ApiError::Core)?;

    let alias = alias_repo
        .create(&req.alias, req.target_model_id)
        .await
        .map_err(ApiError::Core)?;
    Ok(Json(serde_json::json!({ "data": alias })))
}

async fn delete_alias(
    State(state): State<Arc<AppState>>,
    Extension(claims): Extension<Claims>,
    Path(id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, ApiError> {
    require_super_admin(&claims)?;
    let repo = ModelAliasRepository::new(state.pool.clone());
    repo.delete(id).await.map_err(ApiError::Core)?;
    Ok(Json(serde_json::json!({ "deleted": true })))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn create_alias_request_deserializes() {
        let json = serde_json::json!({
            "alias": "gpt-4-turbo",
            "target_model_id": Uuid::nil()
        });
        let req: CreateAliasRequest = serde_json::from_value(json).expect("deserialize");
        assert_eq!(req.alias, "gpt-4-turbo");
        assert_eq!(req.target_model_id, Uuid::nil());
    }
}
