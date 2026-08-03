use axum::{
    extract::{Extension, Json, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::{get, post},
    Router,
};
use futures::StreamExt;
use godwit_core::{Capability, ChatCompletionRequest, ImageGenerationRequest};
use godwit_db::models::ApiKey;
use godwit_db::repositories::{
    models::ModelRepository, provider_profiles::ProviderProfileRepository,
};
use godwit_providers::ProviderResponse;
use rust_decimal::Decimal;
use std::sync::Arc;

use crate::{admin::spend::compute_cost, state::AppState};

pub fn router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/v1/models", get(list_models))
        .route("/v1/chat/completions", post(chat_completions))
        .route("/v1/embeddings", post(embeddings))
        .route("/v1/images/generations", post(image_generations))
        .route("/v1/audio/speech", post(audio_speech))
        .route("/v1/audio/transcriptions", post(audio_transcriptions))
        .route("/v1/images/edits", post(image_edits))
}

/// Maps an adapter error onto an HTTP-appropriate `ApiError`.
///
/// `CapabilityNotSupported` is a *client* problem — the caller asked a model/backend to do
/// something it does not implement (e.g. `/v1/images/edits` against a vllm-backed model) —
/// so it must surface as a 400 with the adapter's explanation, not as the opaque 500 that
/// `ApiError::Core` renders. Every other variant stays a 500.
fn map_provider_error(err: godwit_providers::adapter::ProviderError) -> crate::error::ApiError {
    match err {
        godwit_providers::adapter::ProviderError::CapabilityNotSupported(msg) => {
            crate::error::ApiError::BadRequest(msg)
        }
        other => crate::error::ApiError::Core(godwit_core::PasteurError::Provider(
            other.to_string(),
        )),
    }
}

pub fn models_response(models: &[godwit_db::models::Model]) -> serde_json::Value {
    let data: Vec<serde_json::Value> = models
        .iter()
        .map(|m| {
            serde_json::json!({
                "id": m.public_id,
                "object": "model",
                "created": m.created_at.timestamp(),
                "owned_by": m.provider
            })
        })
        .collect();
    serde_json::json!({ "object": "list", "data": data })
}

/// Drops catalog models whose backing provider profile is disabled — they are not
/// resolvable by `DbModelRouter::resolve`, so advertising them here would be misleading.
pub fn filter_models_with_enabled_profiles(
    models: Vec<godwit_db::models::Model>,
    profiles: &[godwit_db::models::ProviderProfile],
) -> Vec<godwit_db::models::Model> {
    let enabled: std::collections::HashMap<uuid::Uuid, bool> =
        profiles.iter().map(|p| (p.id, p.enabled)).collect();
    models
        .into_iter()
        .filter(|m| enabled.get(&m.provider_profile_id).copied().unwrap_or(false))
        .collect()
}

async fn list_models(
    State(state): State<Arc<AppState>>,
    Extension(_api_key): Extension<ApiKey>,
) -> Result<impl IntoResponse, crate::error::ApiError> {
    let repo = ModelRepository::new(state.pool.clone());
    let models = repo.list().await.map_err(crate::error::ApiError::Core)?;
    let profiles = ProviderProfileRepository::new(state.pool.clone())
        .list()
        .await
        .map_err(crate::error::ApiError::Core)?;
    let models = filter_models_with_enabled_profiles(models, &profiles);
    Ok((StatusCode::OK, Json(models_response(&models))))
}

async fn chat_completions(
    State(state): State<Arc<AppState>>,
    Extension(api_key): Extension<ApiKey>,
    Json(req): Json<ChatCompletionRequest>,
) -> Result<Response, crate::error::ApiError> {
    let start = std::time::Instant::now();

    let resolved = state
        .model_router
        .resolve(&req.model, Capability::Chat)
        .await?;

    let streamed = req.stream == Some(true);
    let (result, usage) = if streamed {
        let stream = resolved
            .adapter
            .chat_stream(&resolved.resolved_credentials, &resolved.model, req)
            .await
            .map_err(map_provider_error)?;
        let sse_stream = stream.map(move |event| {
            let event = event
                .map(|e| axum::response::sse::Event::default().data(e.data))
                .unwrap_or_else(|_| axum::response::sse::Event::default().data("[ERROR]"));
            Ok::<_, std::convert::Infallible>(event)
        });
        (
            Ok(axum::response::Sse::new(sse_stream).into_response()),
            None,
        )
    } else {
        let (resp, report) = resolved
            .adapter
            .chat(&resolved.resolved_credentials, &resolved.model, req)
            .await
            .map_err(map_provider_error)?;
        match resp {
            ProviderResponse::Chat(completion) => {
                (Ok(Json(completion).into_response()), Some(report))
            }
            _ => (
                Err(crate::error::ApiError::Core(godwit_core::PasteurError::Provider(
                    "unexpected provider response variant".to_string(),
                ))),
                None,
            ),
        }
    };

    // Asynchronous logging to avoid blocking the response.
    let cost_usd = usage.and_then(|u| compute_cost(&resolved.model, Capability::Chat, &u));
    let log = RequestLogEntry {
        api_key_id: api_key.id,
        user_id: api_key.user_id,
        organization_id: api_key.organization_id,
        team_id: api_key.team_id,
        model: resolved.model.public_id.clone(),
        provider: resolved.model.provider.clone(),
        provider_model_id: resolved.model.provider_model_id.clone(),
        capability: Capability::Chat.as_str().to_string(),
        duration_ms: start.elapsed().as_millis() as i32,
        streamed,
        status: "success".to_string(),
        cost_usd,
    };
    spawn_request_log(state.pool.clone(), log);

    result
}

async fn embeddings(
    State(state): State<Arc<AppState>>,
    Extension(api_key): Extension<ApiKey>,
    Json(req): Json<godwit_core::EmbeddingRequest>,
) -> Result<Response, crate::error::ApiError> {
    let start = std::time::Instant::now();
    let resolved = state
        .model_router
        .resolve(&req.model, Capability::Embedding)
        .await?;

    let (resp, _usage) = resolved
        .adapter
        .embedding(&resolved.resolved_credentials, &resolved.model, req)
        .await
        .map_err(map_provider_error)?;
    let ProviderResponse::Embedding(body) = resp else {
        return Err(crate::error::ApiError::Core(godwit_core::PasteurError::Provider(
            "unexpected provider response variant".to_string(),
        )));
    };

    let log = RequestLogEntry {
        api_key_id: api_key.id,
        user_id: api_key.user_id,
        organization_id: api_key.organization_id,
        team_id: api_key.team_id,
        model: resolved.model.public_id.clone(),
        provider: resolved.model.provider.clone(),
        provider_model_id: resolved.model.provider_model_id.clone(),
        capability: Capability::Embedding.as_str().to_string(),
        duration_ms: start.elapsed().as_millis() as i32,
        streamed: false,
        status: "success".to_string(),
        cost_usd: None,
    };
    spawn_request_log(state.pool.clone(), log);

    Ok(Json(body).into_response())
}

async fn image_generations(
    State(state): State<Arc<AppState>>,
    Extension(api_key): Extension<ApiKey>,
    Json(req): Json<ImageGenerationRequest>,
) -> Result<Response, crate::error::ApiError> {
    let start = std::time::Instant::now();
    let resolved = state
        .model_router
        .resolve(&req.model, Capability::ImageGeneration)
        .await?;

    let (resp, _usage) = resolved
        .adapter
        .image_generation(&resolved.resolved_credentials, &resolved.model, req)
        .await
        .map_err(map_provider_error)?;
    let ProviderResponse::Image(body) = resp else {
        return Err(crate::error::ApiError::Core(godwit_core::PasteurError::Provider(
            "unexpected provider response variant".to_string(),
        )));
    };

    spawn_request_log(state.pool.clone(), RequestLogEntry {
        api_key_id: api_key.id,
        user_id: api_key.user_id,
        organization_id: api_key.organization_id,
        team_id: api_key.team_id,
        model: resolved.model.public_id.clone(),
        provider: resolved.model.provider.clone(),
        provider_model_id: resolved.model.provider_model_id.clone(),
        capability: Capability::ImageGeneration.as_str().to_string(),
        duration_ms: start.elapsed().as_millis() as i32,
        streamed: false,
        status: "success".to_string(),
        cost_usd: None,
    });

    Ok(Json(body).into_response())
}

async fn audio_speech(
    State(state): State<Arc<AppState>>,
    Extension(api_key): Extension<ApiKey>,
    Json(req): Json<godwit_core::AudioTtsRequest>,
) -> Result<Response, crate::error::ApiError> {
    let start = std::time::Instant::now();
    let resolved = state
        .model_router
        .resolve(&req.model, Capability::AudioTts)
        .await?;

    let (resp, _usage) = resolved
        .adapter
        .audio_tts(&resolved.resolved_credentials, &resolved.model, req)
        .await
        .map_err(map_provider_error)?;
    let ProviderResponse::Bytes(bytes, content_type) = resp else {
        return Err(crate::error::ApiError::Core(godwit_core::PasteurError::Provider(
            "unexpected provider response variant".to_string(),
        )));
    };

    spawn_request_log(state.pool.clone(), RequestLogEntry {
        api_key_id: api_key.id,
        user_id: api_key.user_id,
        organization_id: api_key.organization_id,
        team_id: api_key.team_id,
        model: resolved.model.public_id.clone(),
        provider: resolved.model.provider.clone(),
        provider_model_id: resolved.model.provider_model_id.clone(),
        capability: Capability::AudioTts.as_str().to_string(),
        duration_ms: start.elapsed().as_millis() as i32,
        streamed: false,
        status: "success".to_string(),
        cost_usd: None,
    });

    Ok((
        [(axum::http::header::CONTENT_TYPE, content_type)],
        bytes,
    ).into_response())
}

async fn audio_transcriptions(
    State(state): State<Arc<AppState>>,
    Extension(api_key): Extension<ApiKey>,
    mut multipart: axum::extract::Multipart,
) -> Result<Response, crate::error::ApiError> {
    let mut model_name: Option<String> = None;
    let mut language: Option<String> = None;
    let mut response_format: Option<String> = None;
    let mut file_bytes: Option<Vec<u8>> = None;
    let mut filename = "audio".to_string();
    let mut content_type = "application/octet-stream".to_string();

    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|e| crate::error::ApiError::BadRequest(e.to_string()))?
    {
        match field.name().unwrap_or_default() {
            "model" => model_name = Some(field.text().await.unwrap_or_default()),
            "language" => language = Some(field.text().await.unwrap_or_default()),
            "response_format" => response_format = Some(field.text().await.unwrap_or_default()),
            "file" => {
                filename = field.file_name().unwrap_or("audio").to_string();
                content_type = field
                    .content_type()
                    .unwrap_or("application/octet-stream")
                    .to_string();
                file_bytes = Some(
                    field
                        .bytes()
                        .await
                        .map_err(|e| crate::error::ApiError::BadRequest(e.to_string()))?
                        .to_vec(),
                );
            }
            _ => {}
        }
    }

    let model_name = model_name
        .ok_or_else(|| crate::error::ApiError::BadRequest("missing 'model' field".to_string()))?;
    let file_bytes = file_bytes
        .ok_or_else(|| crate::error::ApiError::BadRequest("missing 'file' field".to_string()))?;

    let start = std::time::Instant::now();
    let resolved = state
        .model_router
        .resolve(&model_name, Capability::AudioStt)
        .await?;

    let req = godwit_core::AudioSttRequest {
        model: model_name,
        language,
        response_format,
    };
    let (resp, _usage) = resolved
        .adapter
        .audio_stt(
            &resolved.resolved_credentials,
            &resolved.model,
            req,
            file_bytes,
            filename,
            content_type,
        )
        .await
        .map_err(map_provider_error)?;
    let ProviderResponse::AudioStt(body) = resp else {
        return Err(crate::error::ApiError::Core(godwit_core::PasteurError::Provider(
            "unexpected provider response variant".to_string(),
        )));
    };

    spawn_request_log(state.pool.clone(), RequestLogEntry {
        api_key_id: api_key.id,
        user_id: api_key.user_id,
        organization_id: api_key.organization_id,
        team_id: api_key.team_id,
        model: resolved.model.public_id.clone(),
        provider: resolved.model.provider.clone(),
        provider_model_id: resolved.model.provider_model_id.clone(),
        capability: Capability::AudioStt.as_str().to_string(),
        duration_ms: start.elapsed().as_millis() as i32,
        streamed: false,
        status: "success".to_string(),
        cost_usd: None,
    });

    Ok(Json(body).into_response())
}

async fn image_edits(
    State(state): State<Arc<AppState>>,
    Extension(api_key): Extension<ApiKey>,
    mut multipart: axum::extract::Multipart,
) -> Result<Response, crate::error::ApiError> {
    let mut model_name: Option<String> = None;
    let mut prompt: Option<String> = None;
    let mut n: Option<i32> = None;
    let mut size: Option<String> = None;
    let mut response_format: Option<String> = None;
    let mut image_bytes: Option<Vec<u8>> = None;
    let mut image_filename = "image.png".to_string();
    let mut mask_bytes: Option<Vec<u8>> = None;

    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|e| crate::error::ApiError::BadRequest(e.to_string()))?
    {
        match field.name().unwrap_or_default() {
            "model" => model_name = Some(field.text().await.unwrap_or_default()),
            "prompt" => prompt = Some(field.text().await.unwrap_or_default()),
            "n" => n = field.text().await.ok().and_then(|s| s.parse().ok()),
            "size" => size = Some(field.text().await.unwrap_or_default()),
            "response_format" => response_format = Some(field.text().await.unwrap_or_default()),
            "image" => {
                image_filename = field.file_name().unwrap_or("image.png").to_string();
                image_bytes = Some(
                    field
                        .bytes()
                        .await
                        .map_err(|e| crate::error::ApiError::BadRequest(e.to_string()))?
                        .to_vec(),
                );
            }
            "mask" => {
                mask_bytes = Some(
                    field
                        .bytes()
                        .await
                        .map_err(|e| crate::error::ApiError::BadRequest(e.to_string()))?
                        .to_vec(),
                );
            }
            _ => {}
        }
    }

    let model_name = model_name
        .ok_or_else(|| crate::error::ApiError::BadRequest("missing 'model' field".to_string()))?;
    let prompt = prompt
        .ok_or_else(|| crate::error::ApiError::BadRequest("missing 'prompt' field".to_string()))?;
    let image_bytes = image_bytes
        .ok_or_else(|| crate::error::ApiError::BadRequest("missing 'image' field".to_string()))?;

    let start = std::time::Instant::now();
    let resolved = state
        .model_router
        .resolve(&model_name, Capability::ImageEdit)
        .await?;

    let req = godwit_core::ImageEditRequest {
        model: model_name,
        prompt,
        n,
        size,
        response_format,
    };
    let (resp, _usage) = resolved
        .adapter
        .image_edit(
            &resolved.resolved_credentials,
            &resolved.model,
            req,
            image_bytes,
            image_filename,
            mask_bytes,
        )
        .await
        .map_err(map_provider_error)?;
    let ProviderResponse::Image(body) = resp else {
        return Err(crate::error::ApiError::Core(godwit_core::PasteurError::Provider(
            "unexpected provider response variant".to_string(),
        )));
    };

    spawn_request_log(state.pool.clone(), RequestLogEntry {
        api_key_id: api_key.id,
        user_id: api_key.user_id,
        organization_id: api_key.organization_id,
        team_id: api_key.team_id,
        model: resolved.model.public_id.clone(),
        provider: resolved.model.provider.clone(),
        provider_model_id: resolved.model.provider_model_id.clone(),
        capability: Capability::ImageEdit.as_str().to_string(),
        duration_ms: start.elapsed().as_millis() as i32,
        streamed: false,
        status: "success".to_string(),
        cost_usd: None,
    });

    Ok(Json(body).into_response())
}

fn spawn_request_log(pool: sqlx::PgPool, log: RequestLogEntry) {
    tokio::spawn(async move {
        let _ = sqlx::query(
            "INSERT INTO request_logs (api_key_id, user_id, organization_id, team_id, model, provider, provider_model_id, capability, duration_ms, streamed, status, cost_usd)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12)"
        )
        .bind(log.api_key_id)
        .bind(log.user_id)
        .bind(log.organization_id)
        .bind(log.team_id)
        .bind(log.model)
        .bind(log.provider)
        .bind(log.provider_model_id)
        .bind(log.capability)
        .bind(log.duration_ms)
        .bind(log.streamed)
        .bind(log.status)
        .bind(log.cost_usd)
        .execute(&pool)
        .await;
    });
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
    capability: String,
    duration_ms: i32,
    streamed: bool,
    status: String,
    cost_usd: Option<Decimal>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn models_response_has_openai_shape() {
        let body = models_response(&[]);
        assert_eq!(body["object"], "list");
        assert!(body["data"].as_array().unwrap().is_empty());
    }

    fn model_for(profile_id: uuid::Uuid, public_id: &str) -> godwit_db::models::Model {
        godwit_db::models::Model {
            id: uuid::Uuid::new_v4(),
            public_id: public_id.to_string(),
            provider: "openai".to_string(),
            provider_profile_id: profile_id,
            provider_model_id: public_id.to_string(),
            capabilities: vec!["chat".to_string()],
            pricing: serde_json::json!({}),
            config: serde_json::json!({}),
            created_at: chrono::Utc::now(),
        }
    }

    fn profile_with(id: uuid::Uuid, enabled: bool) -> godwit_db::models::ProviderProfile {
        godwit_db::models::ProviderProfile {
            id,
            name: format!("p-{id}"),
            protocol: "openai".to_string(),
            base_url: Some("https://api.openai.com/v1".to_string()),
            allow_wildcard: false,
            auth: serde_json::json!({}),
            config: serde_json::json!({}),
            enabled,
            created_at: chrono::Utc::now(),
        }
    }

    #[test]
    fn list_models_excludes_models_backed_by_disabled_profiles() {
        let enabled_id = uuid::Uuid::new_v4();
        let disabled_id = uuid::Uuid::new_v4();
        let models = vec![
            model_for(enabled_id, "visible"),
            model_for(disabled_id, "hidden"),
            // A model whose profile no longer exists is also not resolvable.
            model_for(uuid::Uuid::new_v4(), "orphaned"),
        ];
        let profiles = vec![profile_with(enabled_id, true), profile_with(disabled_id, false)];

        let filtered = filter_models_with_enabled_profiles(models, &profiles);
        let ids: Vec<&str> = filtered.iter().map(|m| m.public_id.as_str()).collect();
        assert_eq!(ids, vec!["visible"]);

        let body = models_response(&filtered);
        assert_eq!(body["data"].as_array().unwrap().len(), 1);
        assert_eq!(body["data"][0]["id"], "visible");
        assert_eq!(body["data"][0]["owned_by"], "openai");
    }

    #[test]
    fn capability_not_supported_maps_to_bad_request() {
        let err = map_provider_error(
            godwit_providers::adapter::ProviderError::CapabilityNotSupported(
                "image edit is not supported by vllm".to_string(),
            ),
        );
        match err {
            crate::error::ApiError::BadRequest(msg) => {
                assert_eq!(msg, "image edit is not supported by vllm")
            }
            _ => panic!("CapabilityNotSupported must map to a 400 BadRequest"),
        }

        let resp = map_provider_error(
            godwit_providers::adapter::ProviderError::CapabilityNotSupported("nope".to_string()),
        )
        .into_response();
        assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    }

    #[test]
    fn other_provider_errors_still_map_to_internal_error() {
        for err in [
            godwit_providers::adapter::ProviderError::Http {
                status: 502,
                message: "upstream exploded".to_string(),
            },
            godwit_providers::adapter::ProviderError::Serialization("bad json".to_string()),
            godwit_providers::adapter::ProviderError::Provider("boom".to_string()),
        ] {
            let mapped = map_provider_error(err);
            assert!(matches!(mapped, crate::error::ApiError::Core(_)));
            assert_eq!(
                mapped.into_response().status(),
                StatusCode::INTERNAL_SERVER_ERROR
            );
        }
    }
}
