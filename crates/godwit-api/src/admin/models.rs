use axum::{
    extract::{Extension, Path, State},
    routing::{get, patch},
    Json, Router,
};
use godwit_auth::jwt::Claims;
use godwit_db::repositories::models::ModelRepository;
use serde::Deserialize;
use std::sync::Arc;
use uuid::Uuid;

use crate::{admin::require_super_admin, error::ApiError, state::AppState};

pub fn router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/models", get(list_models).post(create_model))
        .route("/models/:id", patch(update_model).delete(delete_model))
}

async fn list_models(
    State(state): State<Arc<AppState>>,
    Extension(claims): Extension<Claims>,
) -> Result<Json<serde_json::Value>, ApiError> {
    require_super_admin(&claims)?;
    let repo = ModelRepository::new(state.pool.clone());
    let models = repo.list().await.map_err(ApiError::Core)?;
    Ok(Json(serde_json::json!({ "data": models })))
}

#[derive(Debug, Deserialize)]
pub struct CreateModelRequest {
    pub public_id: String,
    pub provider: String,
    pub provider_profile_id: Uuid,
    pub provider_model_id: String,
    pub capabilities: String,
}

async fn create_model(
    State(state): State<Arc<AppState>>,
    Extension(claims): Extension<Claims>,
    Json(req): Json<CreateModelRequest>,
) -> Result<Json<serde_json::Value>, ApiError> {
    require_super_admin(&claims)?;
    let repo = ModelRepository::new(state.pool.clone());
    let model = repo
        .create(
            &req.public_id,
            &req.provider,
            req.provider_profile_id,
            &req.provider_model_id,
            &req.capabilities,
        )
        .await
        .map_err(ApiError::Core)?;
    Ok(Json(serde_json::json!({ "data": model })))
}

#[derive(Debug, Deserialize)]
pub struct UpdateModelRequest {
    pub public_id: Option<String>,
    pub capabilities: Option<String>,
}

async fn update_model(
    State(state): State<Arc<AppState>>,
    Extension(claims): Extension<Claims>,
    Path(id): Path<Uuid>,
    Json(req): Json<UpdateModelRequest>,
) -> Result<Json<serde_json::Value>, ApiError> {
    require_super_admin(&claims)?;
    let repo = ModelRepository::new(state.pool.clone());
    let model = repo
        .update(id, req.public_id.as_deref(), req.capabilities.as_deref())
        .await
        .map_err(ApiError::Core)?;
    Ok(Json(serde_json::json!({ "data": model })))
}

async fn delete_model(
    State(state): State<Arc<AppState>>,
    Extension(claims): Extension<Claims>,
    Path(id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, ApiError> {
    require_super_admin(&claims)?;
    let repo = ModelRepository::new(state.pool.clone());
    repo.delete(id).await.map_err(ApiError::Core)?;
    Ok(Json(serde_json::json!({ "deleted": true })))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn create_model_request_deserializes() {
        let json = serde_json::json!({
            "public_id": "gpt-4o",
            "provider": "openai",
            "provider_profile_id": Uuid::nil(),
            "provider_model_id": "gpt-4o",
            "capabilities": "chat,embedding"
        });
        let req: CreateModelRequest = serde_json::from_value(json).expect("deserialize");
        assert_eq!(req.public_id, "gpt-4o");
        assert_eq!(req.capabilities, "chat,embedding");
    }
}
