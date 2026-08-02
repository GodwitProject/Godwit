use axum::{
    extract::{Extension, Json, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::{get, post},
    Router,
};
use futures::StreamExt;
use pasteurllm_core::{ChatCompletionRequest, ChatCompletionResponse, ChatMessage};
use pasteurllm_db::models::ApiKey;
use pasteurllm_db::repositories::models::ModelRepository;
use std::sync::Arc;

use crate::state::AppState;

pub fn router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/v1/models", get(list_models))
        .route("/v1/chat/completions", post(chat_completions))
}

pub fn models_response(models: &[pasteurllm_db::models::Model]) -> serde_json::Value {
    let data: Vec<serde_json::Value> = models
        .iter()
        .map(|m| {
            serde_json::json!({
                "id": m.public_id,
                "object": "model",
                "created": m.created_at.timestamp(),
                "owned_by": "organization"
            })
        })
        .collect();
    serde_json::json!({ "object": "list", "data": data })
}

async fn list_models(
    State(state): State<Arc<AppState>>,
    Extension(api_key): Extension<ApiKey>,
) -> Result<impl IntoResponse, crate::error::ApiError> {
    let models = if let Some(cached) = state.model_cache.get(&(api_key.organization_id, "".to_string())).await {
        vec![cached]
    } else {
        let repo = ModelRepository::new(state.pool.clone());
        let models = repo
            .list_for_organization(api_key.organization_id)
            .await
            .map_err(crate::error::ApiError::Core)?;
        for m in &models {
            state.model_cache.insert((api_key.organization_id, m.public_id.clone()), m.clone()).await;
        }
        models
    };
    Ok((StatusCode::OK, Json(models_response(&models))))
}

async fn chat_completions(
    State(state): State<Arc<AppState>>,
    Extension(api_key): Extension<ApiKey>,
    Json(req): Json<ChatCompletionRequest>,
) -> Result<Response, crate::error::ApiError> {
    let start = std::time::Instant::now();
    let model = state
        .model_cache
        .get(&(api_key.organization_id, req.model.clone()))
        .await
        .ok_or(crate::error::ApiError::NotFound)?;

    let provider = state
        .provider_router
        .route(api_key.organization_id, &model.provider_model_id)
        .await
        .ok_or(crate::error::ApiError::NotFound)?;

    let streamed = req.stream == Some(true);
    let result = if streamed {
        let stream = provider
            .stream_chat_completion(req)
            .await
            .map_err(|_| crate::error::ApiError::Core(pasteurllm_core::PasteurError::Provider("provider request failed".to_string())))?;
        let sse_stream = stream.map(move |event| {
            let event = event
                .map(|e| axum::response::sse::Event::default().data(e.data))
                .unwrap_or_else(|_| axum::response::sse::Event::default().data("[ERROR]"));
            Ok::<_, std::convert::Infallible>(event)
        });
        Ok(axum::response::Sse::new(sse_stream).into_response())
    } else {
        let resp = provider
            .chat_completion(req)
            .await
            .map_err(|_| crate::error::ApiError::Core(pasteurllm_core::PasteurError::Provider("provider request failed".to_string())))?;
        match resp {
            pasteurllm_providers::ProviderResponse::Json(json) => Ok(Json(json).into_response()),
        }
    };

    // Asynchronous logging to avoid blocking the response.
    let log = RequestLogEntry {
        api_key_id: api_key.id,
        user_id: api_key.user_id,
        organization_id: api_key.organization_id,
        team_id: api_key.team_id,
        model: model.public_id.clone(),
        provider: model.provider.clone(),
        provider_model_id: model.provider_model_id.clone(),
        duration_ms: start.elapsed().as_millis() as i32,
        streamed,
        status: "success".to_string(),
    };
    let pool = state.pool.clone();
    tokio::spawn(async move {
        let _ = sqlx::query(
            "INSERT INTO request_logs (api_key_id, user_id, organization_id, team_id, model, provider, provider_model_id, duration_ms, streamed, status)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)"
        )
        .bind(log.api_key_id)
        .bind(log.user_id)
        .bind(log.organization_id)
        .bind(log.team_id)
        .bind(log.model)
        .bind(log.provider)
        .bind(log.provider_model_id)
        .bind(log.duration_ms)
        .bind(log.streamed)
        .bind(log.status)
        .execute(&pool)
        .await;
    });

    result
}

#[derive(Clone)]
struct RequestLogEntry {
    api_key_id: uuid::Uuid,
    user_id: uuid::Uuid,
    organization_id: uuid::Uuid,
    team_id: Option<uuid::Uuid>,
    model: String,
    provider: String,
    provider_model_id: String,
    duration_ms: i32,
    streamed: bool,
    status: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_api_key() -> ApiKey {
        ApiKey {
            id: uuid::Uuid::new_v4(),
            user_id: uuid::Uuid::new_v4(),
            team_id: None,
            organization_id: uuid::Uuid::new_v4(),
            name: "test".to_string(),
            key_prefix: "test".to_string(),
            key_hash: "hash".to_string(),
            scopes: vec!["proxy:write".to_string()],
            budget_limit_usd: None,
            budget_spent_usd: rust_decimal::Decimal::ZERO,
            rate_limit_requests_per_minute: None,
            expires_at: None,
            disabled: false,
            created_at: chrono::Utc::now(),
        }
    }

    #[test]
    fn models_response_has_openai_shape() {
        let body = models_response(&[]);
        assert_eq!(body["object"], "list");
        assert!(body["data"].as_array().unwrap().is_empty());
    }
}
