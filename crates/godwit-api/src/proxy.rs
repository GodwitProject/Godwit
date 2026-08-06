use axum::{
    extract::{Extension, Json, State},
    http::StatusCode,
    middleware,
    response::{IntoResponse, Response},
    routing::{get, post},
    Router,
};
use futures::StreamExt;
use godwit_core::{
    Capability, ChatCompletionRequest, ChatCompletionResponse, ChatContent, ChatMessage,
    ImageGenerationRequest, Tool, ToolCall,
};
#[cfg(test)]
use godwit_core::{AppConfig, AuthConfig, CompatConfig, DatabaseConfig, ServerConfig};
use godwit_db::models::ApiKey;
use godwit_db::repositories::{
    models::ModelRepository, provider_profiles::ProviderProfileRepository,
};
use godwit_providers::{ProviderResponse, UsageReport};
use rust_decimal::Decimal;
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};
use uuid::Uuid;

use crate::{
    admin::spend::compute_cost,
    model_info,
    model_router::{DbModelRouter, ResolvedModel},
    proxy_streaming::process_streaming_tool_calls,
    rate_limit,
    resilience::{with_retry, RetryPolicy},
    state::AppState,
};

pub fn router() -> Router<Arc<AppState>> {
    let chat_router = Router::new()
        .route("/v1/chat/completions", post(chat_completions))
        .route_layer(middleware::from_fn(crate::middleware::model_scope));

    Router::new()
        .merge(chat_router)
        .merge(model_info::router())
        .route("/v1/models", get(list_models))
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
pub(crate) fn map_provider_error(
    err: godwit_providers::adapter::ProviderError,
) -> crate::error::ApiError {
    match err {
        godwit_providers::adapter::ProviderError::CapabilityNotSupported(msg) => {
            crate::error::ApiError::BadRequest(msg)
        }
        other => {
            crate::error::ApiError::Core(godwit_core::PasteurError::Provider(other.to_string()))
        }
    }
}

/// Forwards an OpenAI-style request body verbatim to the resolved backend's `path`,
/// substituting the client-supplied model ref with the catalog row's upstream
/// `provider_model_id` (matching the chat/embedding adapters).
///
/// Used by OpenAI-compatible passthrough endpoints (`/v1/moderations`, `/v1/rerank`,
/// `/v1/batches`) that have no dedicated `Adapter` method of their own. Returns the
/// upstream JSON response body plus a default usage report.
pub(crate) async fn forward_openai_passthrough(
    state: &Arc<AppState>,
    method: reqwest::Method,
    path: &str,
    body: Option<serde_json::Value>,
    model_ref: &str,
    capability: Capability,
) -> Result<(serde_json::Value, godwit_providers::adapter::UsageReport), crate::error::ApiError>
{
    let resolved = state
        .model_router
        .resolve(model_ref, capability)
        .await?;

    let url = format!(
        "{}/{}",
        resolved.resolved_credentials.base_url,
        path.trim_start_matches('/')
    );
    let client = reqwest::Client::new();
    let mut req = client
        .request(method, &url)
        .header(reqwest::header::ACCEPT, "application/json");
    if let Some(key) = &resolved.resolved_credentials.api_key {
        req = req.header("Authorization", format!("Bearer {key}"));
    }
    if let Some(mut body) = body {
        body["model"] = serde_json::Value::String(resolved.model.provider_model_id.clone());
        req = req.json(&body);
    }
    let res = req.send().await.map_err(|e| {
        crate::error::ApiError::Core(godwit_core::PasteurError::Provider(e.to_string()))
    })?;
    if !res.status().is_success() {
        let status = res.status().as_u16();
        let text = res.text().await.unwrap_or_default();
        return Err(map_provider_error(godwit_providers::adapter::ProviderError::Http {
            status,
            message: text,
        }));
    }
    let value: serde_json::Value = res.json().await.map_err(|e| {
        crate::error::ApiError::Core(godwit_core::PasteurError::Provider(format!(
            "failed to deserialize upstream response: {e}"
        )))
    })?;
    Ok((value, godwit_providers::adapter::UsageReport::default()))
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
        .filter(|m| {
            enabled
                .get(&m.provider_profile_id)
                .copied()
                .unwrap_or(false)
        })
        .collect()
}

async fn check_rate_limit(
    state: &Arc<AppState>,
    api_key: &ApiKey,
    model: &godwit_db::models::Model,
    estimated_tokens: u32,
) -> Result<(), crate::error::ApiError> {
    let org = state
        .org_repo
        .get_by_id(api_key.organization_id)
        .await
        .map_err(crate::error::ApiError::Core)?;

    if let Err((_, retry_after)) = state.rate_limiter.check_and_consume(
        api_key.id,
        api_key.organization_id,
        &model.public_id,
        api_key.rate_limit_requests_per_minute,
        api_key.rate_limit_tokens_per_minute,
        org.rate_limit_requests_per_minute,
        org.rate_limit_tokens_per_minute,
        estimated_tokens,
    ) {
        return Err(crate::error::ApiError::RateLimited(retry_after));
    }
    Ok(())
}

async fn check_user_budget(
    state: &Arc<AppState>,
    api_key: &ApiKey,
) -> Result<(), crate::error::ApiError> {
    rate_limit::check_end_user_budget(
        &state.pool,
        api_key.user_id,
        api_key.organization_id,
    )
    .await
}

async fn check_team_budget(
    state: &Arc<AppState>,
    api_key: &ApiKey,
) -> Result<(), crate::error::ApiError> {
    if let Some(team_id) = api_key.team_id {
        rate_limit::check_team_budget(&state.pool, team_id).await
    } else {
        Ok(())
    }
}

fn default_retry_policy() -> RetryPolicy {
    RetryPolicy::default()
}

async fn call_chat(
    state: &Arc<AppState>,
    resolved: &ResolvedModel,
    req: ChatCompletionRequest,
) -> Result<(Response, Option<godwit_providers::adapter::UsageReport>), godwit_providers::adapter::ProviderError>
{
    let streamed = req.stream == Some(true);
    if streamed {
        let adapter = Arc::clone(&resolved.adapter);
        let credentials = resolved.resolved_credentials.clone();
        let model = resolved.model.clone();
        let has_tools = req
            .tools
            .as_ref()
            .map(|tools| {
                tools.iter().any(|t| {
                    t.function.name.contains("__")
                        || godwit_providers::NATIVE_WEB_SEARCH_TOOLS
                            .contains(&t.function.name.as_str())
                })
            })
            .unwrap_or(false);

        let mut stream = with_retry(&default_retry_policy(), move || {
            let adapter = Arc::clone(&adapter);
            let credentials = credentials.clone();
            let model = model.clone();
            let req = req.clone();
            async move { adapter.chat_stream(&credentials, &model, req).await }
        })
        .await?;

        if has_tools {
            stream = process_streaming_tool_calls(Arc::clone(state), stream);
        }
        let created = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs() as i64;
        let stream_id = format!("chatcmpl-{}", Uuid::new_v4());
        let stream_model = if resolved.model.provider_model_id.is_empty() {
            resolved.model.public_id.clone()
        } else {
            resolved.model.provider_model_id.clone()
        };
        let use_openai_wire = state
            .config
            .compat
            .as_ref()
            .map(|c| c.openai_wire_streaming)
            .unwrap_or(false);
        let translator = if use_openai_wire {
            Some(Mutex::new(
                godwit_providers::sse_egress::OpenAiStreamTranslator::new(
                    stream_id,
                    stream_model,
                    created,
                ),
            ))
        } else {
            None
        };
        let sse_stream = stream.flat_map(move |event| {
            let events = if use_openai_wire {
                event
                    .map(|e| {
                        let mut translator = translator.as_ref().unwrap().lock().unwrap();
                        let frames = translator.push(&e);
                        frames
                            .iter()
                            .map(|f| {
                                axum::response::sse::Event::default().data(f.render())
                            })
                            .collect::<Vec<_>>()
                    })
                    .unwrap_or_else(|_| {
                        let error_payload = serde_json::json!({
                            "error": {
                                "message": "upstream provider stream error",
                                "type": "server_error",
                                "param": null,
                                "code": null,
                            }
                        });
                        vec![axum::response::sse::Event::default()
                            .data(error_payload.to_string())]
                    })
            } else {
                event
                    .map(|e| {
                        vec![axum::response::sse::Event::default().data(&e.data)]
                    })
                    .unwrap_or_else(|_| {
                        let error_payload = serde_json::json!({
                            "error": {
                                "message": "upstream provider stream error",
                                "type": "server_error",
                            }
                        });
                        vec![axum::response::sse::Event::default()
                            .data(error_payload.to_string())]
                    })
            };
            futures::stream::iter(
                events
                    .into_iter()
                    .map(Ok::<_, std::convert::Infallible>),
            )
        });
        Ok((axum::response::Sse::new(sse_stream).into_response(), None))
    } else {
        let adapter = Arc::clone(&resolved.adapter);
        let credentials = resolved.resolved_credentials.clone();
        let model = resolved.model.clone();
        let (resp, report) = with_retry(&default_retry_policy(), move || {
            let adapter = Arc::clone(&adapter);
            let credentials = credentials.clone();
            let model = model.clone();
            let req = req.clone();
            async move { adapter.chat(&credentials, &model, req).await }
        })
        .await?;
        match resp {
            ProviderResponse::Chat(completion) => {
                Ok((Json(completion).into_response(), Some(report)))
            }
            _ => Err(godwit_providers::adapter::ProviderError::Provider(
                "unexpected provider response variant".to_string(),
            )),
        }
    }
}

/// Wire the agentic capabilities into a chat request before it is sent to the backend:
/// merge any configured MCP tools into the request's `tools`, skipping any MCP tool whose
/// name collides with an existing tool and ignoring native web-search tools. Native
/// web-search tools are left untouched so that providers which support them natively
/// (OpenAI/Gemini) keep using their own server-side search.
async fn merge_agentic_tools(
    state: &Arc<AppState>,
    req: &mut ChatCompletionRequest,
) -> Vec<Tool> {
    let mut existing: std::collections::HashSet<String> = req
        .tools
        .as_ref()
        .map(|ts| ts.iter().map(|t| t.function.name.clone()).collect())
        .unwrap_or_default();

    // Record the native web-search tool names already declared so we don't duplicate.
    let mut added = Vec::new();
    for tool in state.mcp.all_tools().await {
        let name = tool.function.name.clone();
        if godwit_providers::is_native_web_search_tool(&tool) || existing.contains(&name) {
            continue;
        }
        existing.insert(name);
        added.push(tool);
    }
    if !added.is_empty() {
        let tools = req.tools.get_or_insert_with(Vec::new);
        tools.extend(added.clone());
    }
    added
}

/// Resolve a set of model-emitted `tool_calls` into `tool`-role result messages.
///
/// * MCP tool calls (names `server__tool`) are routed through the MCP registry.
/// * `web_search` / `google_search` style calls are routed through SearXNG.
/// * Anything else cannot be resolved here and yields a short explanatory result message so
///   the conversation does not deadlock on an unhandled tool call.
pub(crate) async fn resolve_tool_calls(
    state: &Arc<AppState>,
    tool_calls: &[ToolCall],
) -> Vec<ChatMessage> {
    let mut out = Vec::new();
    for call in tool_calls {
        let id = call.id.clone();
        let name = call.function.name.clone();
        let args = serde_json::from_str::<serde_json::Value>(&call.function.arguments)
            .unwrap_or_else(|_| serde_json::json!({}));

        let result = if name.contains("__") {
            match state.mcp.call_tool(&name, args.clone()).await {
                Ok(text) => text,
                Err(e) => format!("MCP tool call to '{name}' failed: {e}"),
            }
        } else if godwit_providers::NATIVE_WEB_SEARCH_TOOLS.contains(&name.as_str()) {
            match web_search_result(state, &name, &args).await {
                Some(text) => text,
                None => format!(
                    "web search tool '{name}' was requested but no SearXNG backend is configured"
                ),
            }
        } else {
            format!("tool '{name}' is not provisioned by the gateway")
        };

        out.push(ChatMessage {
            role: "tool".to_string(),
            content: ChatContent::Text(result),
            name: None,
            tool_calls: None,
            tool_call_id: Some(id),
            cache_control: None,
        });
    }
    out
}

/// Run a search through the configured SearXNG backend for the given tool call.
/// Returns `None` when SearXNG is not configured.
async fn web_search_result(
    state: &Arc<AppState>,
    tool_name: &str,
    args: &serde_json::Value,
) -> Option<String> {
    let (provider, profile) = match (&state.searxng, &state.searxng_profile) {
        (Some(p), Some(pr)) => (p, pr),
        _ => return None,
    };
    let query = args
        .get("query")
        .or_else(|| args.get("search_query"))
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .trim();
    if query.is_empty() {
        return Some(
            "web search tool denied: no 'query' argument was supplied by the model".to_string(),
        );
    }
    match provider.search(profile, query).await {
        Ok(results) => {
            let mut lines = vec![format!("web search ({tool_name}) results for: {query}")];
            for (i, r) in results.iter().enumerate() {
                lines.push(format!(
                    "{}. {} — {}\n   {}",
                    i + 1,
                    r.title,
                    r.url,
                    r.content.as_deref().unwrap_or("")
                ));
            }
            Some(lines.join("\n"))
        }
        Err(e) => Some(format!("web search via SearXNG failed: {e}")),
    }
}

/// Drive a non-streaming chat request through a bounded agentic tool loop.
///
/// MCP and web-search tool calls emitted by the model are resolved and fed back for a
/// follow-up model round trip, up to [`MAX_AGENTIC_ITERATIONS`] times. When the model makes
/// no tool calls (or they cannot be resolved) the current completion is returned.
const MAX_AGENTIC_ITERATIONS: usize = 4;

async fn run_agentic_chat(
    state: &Arc<AppState>,
    resolved: &ResolvedModel,
    mut req: ChatCompletionRequest,
) -> Result<(ChatCompletionResponse, UsageReport), godwit_providers::adapter::ProviderError>
{
    let mut messages = req.messages.clone();
    let mut usage = UsageReport::default();
    for _ in 0..MAX_AGENTIC_ITERATIONS {
        req.messages = messages.clone();
        let (resp, round_usage) = with_retry(&default_retry_policy(), || {
            let adapter = Arc::clone(&resolved.adapter);
            let credentials = resolved.resolved_credentials.clone();
            let model = resolved.model.clone();
            let req = req.clone();
            async move { adapter.chat(&credentials, &model, req).await }
        })
        .await?;
        usage = accumulate_usage(usage, &round_usage);
        let ProviderResponse::Chat(completion) = resp else {
            return Err(godwit_providers::adapter::ProviderError::Provider(
                "unexpected provider response variant during agentic chat".to_string(),
            ));
        };

        let tool_calls: Vec<ToolCall> = completion
            .choices
            .iter()
            .flat_map(|c| c.message.tool_calls.clone().unwrap_or_default())
            .collect();
        if tool_calls.is_empty() {
            return Ok((completion, usage));
        }

        // Remember the assistant turn that produced the tool calls.
        if let Some(choice) = completion.choices.first() {
            messages.push(choice.message.clone());
        }
        let results = resolve_tool_calls(state, &tool_calls).await;
        if results.is_empty() {
            return Ok((completion, usage));
        }
        messages.extend(results);
    }
    Err(godwit_providers::adapter::ProviderError::Provider(
        format!(
            "agentic tool loop exceeded {MAX_AGENTIC_ITERATIONS} iterations without converging"
        ),
    ))
}

fn accumulate_usage(mut acc: UsageReport, report: &UsageReport) -> UsageReport {
    let add = |a: Option<i32>, b: Option<i32>| match (a, b) {
        (Some(x), Some(y)) => Some(x + y),
        (a, None) => a,
        (None, b) => b,
    };
    acc.prompt_tokens = add(acc.prompt_tokens, report.prompt_tokens);
    acc.completion_tokens = add(acc.completion_tokens, report.completion_tokens);
    acc
}

/// Dispatch a chat request, routing it through the agentic tool loop when appropriate.
///
/// Streaming requests are forwarded unchanged (tool resolution in a streaming response is
/// not yet supported). Non-streaming requests get MCP tools merged in and a bounded
/// tool-call resolution loop.
async fn call_chat_agentic(
    state: &Arc<AppState>,
    resolved: &ResolvedModel,
    mut req: ChatCompletionRequest,
) -> Result<(Response, Option<UsageReport>), godwit_providers::adapter::ProviderError> {
    if req.stream == Some(true) {
        return call_chat(state, resolved, req).await;
    }

    merge_agentic_tools(state, &mut req).await;
    let (completion, usage) = run_agentic_chat(state, resolved, req).await?;
    Ok((Json(completion).into_response(), Some(usage)))
}

async fn chat_completions(
    State(state): State<Arc<AppState>>,
    Extension(api_key): Extension<ApiKey>,
    req_headers: axum::http::Request<axum::body::Body>,
) -> Result<Response, crate::error::ApiError> {
    let start = std::time::Instant::now();

    let tags = extract_tags_from_header(
        req_headers.headers()
            .get("x-godwit-tags")
            .and_then(|v| v.to_str().ok())
    );

    let (_req_parts, req_body) = req_headers.into_parts();
    let body_bytes = axum::body::to_bytes(req_body, usize::MAX).await.unwrap();
    let req: ChatCompletionRequest = serde_json::from_slice(&body_bytes).unwrap();

    let mut primary_resolved = state
        .model_router
        .resolve(&req.model, Capability::Chat)
        .await?;
    let fallback_chain = DbModelRouter::fallback_chain(&primary_resolved.model);

    let estimated_tokens = rate_limit::estimate_request_tokens(&req);
    check_rate_limit(&state, &api_key, &primary_resolved.model, estimated_tokens).await?;
    check_user_budget(&state, &api_key).await?;
    check_team_budget(&state, &api_key).await?;

    let mut rate_limited_err: Option<crate::error::ApiError> = None;
    let (result, used_model) = match call_chat_agentic(&state, &primary_resolved, req.clone()).await
    {
        Ok((resp, usage)) => ((Ok(resp), usage), primary_resolved.model.clone()),
        Err(e) => {
            // Release the primary model's in-flight slot before attempting fallbacks.
            std::mem::drop(primary_resolved.in_flight.take());
            let mut fallback_result = None;
            for fallback_id in fallback_chain {
                tracing::info!(
                    "falling back from {} to {} for chat completion",
                    req.model,
                    fallback_id
                );
                match state
                    .model_router
                    .resolve(&fallback_id, Capability::Chat)
                    .await
                {
                    Ok(resolved) => {
                        if let Err(rl_err) =
                            check_rate_limit(&state, &api_key, &resolved.model, estimated_tokens)
                                .await
                        {
                            rate_limited_err = Some(rl_err);
                            continue;
                        }
                        match call_chat_agentic(&state, &resolved, req.clone()).await {
                            Ok((resp, usage)) => {
                                fallback_result = Some(((Ok(resp), usage), resolved.model.clone()));
                                break;
                            }
                            Err(_e) => {
                                continue;
                            }
                        }
                    }
                    Err(_) => continue,
                }
            }
            fallback_result.ok_or_else(|| rate_limited_err.unwrap())?
        }
    };

    let streamed = req.stream == Some(true);
    let (result, usage) = result;

    let cost_usd = usage.and_then(|u| compute_cost(&used_model, Capability::Chat, &u));
    let log = RequestLogEntry {
        api_key_id: api_key.id,
        user_id: api_key.user_id,
        organization_id: api_key.organization_id,
        team_id: api_key.team_id,
        model: used_model.public_id.clone(),
        provider: used_model.provider.clone(),
        provider_model_id: used_model.provider_model_id.clone(),
        capability: Capability::Chat.as_str().to_string(),
        duration_ms: start.elapsed().as_millis() as i32,
        streamed,
        status: "success".to_string(),
        cost_usd,
        tags,
    };
    spawn_request_log(state.pool.clone(), log);

    if !used_model.id.is_nil() {
        state
            .model_router
            .record_latency(used_model.id, start.elapsed().as_millis() as i32);
    }

    result
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

async fn embeddings(
    State(state): State<Arc<AppState>>,
    Extension(api_key): Extension<ApiKey>,
    Json(req): Json<godwit_core::EmbeddingRequest>,
) -> Result<Response, crate::error::ApiError> {
    let start = std::time::Instant::now();

    let mut primary_resolved = state
        .model_router
        .resolve(&req.model, Capability::Embedding)
        .await?;
    let fallback_chain = DbModelRouter::fallback_chain(&primary_resolved.model);

    let estimated_tokens = 1u32;
    check_rate_limit(&state, &api_key, &primary_resolved.model, estimated_tokens).await?;
    check_user_budget(&state, &api_key).await?;
    check_team_budget(&state, &api_key).await?;

    let mut rate_limited_err: Option<crate::error::ApiError> = None;
    let (body, usage, used_model) = match call_embedding(&primary_resolved, req.clone()).await {
        Ok((resp, usage)) => (resp, usage, primary_resolved.model.clone()),
        Err(e) => {
            std::mem::drop(primary_resolved.in_flight.take());
            let mut fallback_result = None;
            for fallback_id in fallback_chain {
                tracing::info!(
                    "falling back from {} to {} for embedding",
                    req.model,
                    fallback_id
                );
                match state
                    .model_router
                    .resolve(&fallback_id, Capability::Embedding)
                    .await
                {
                    Ok(resolved) => {
                        if let Err(rl_err) =
                            check_rate_limit(&state, &api_key, &resolved.model, estimated_tokens)
                                .await
                        {
                            rate_limited_err = Some(rl_err);
                            continue;
                        }
                        match call_embedding(&resolved, req.clone()).await {
                            Ok((resp, usage)) => {
                                fallback_result = Some((resp, usage, resolved.model.clone()));
                                break;
                            }
                            Err(_e) => {
                                continue;
                            }
                        }
                    }
                    Err(_) => continue,
                }
            }
            fallback_result.ok_or_else(|| rate_limited_err.unwrap())?
        }
    };
    let cost_usd = compute_cost(&used_model, Capability::Embedding, &usage);

    let log = RequestLogEntry {
        api_key_id: api_key.id,
        user_id: api_key.user_id,
        organization_id: api_key.organization_id,
        team_id: api_key.team_id,
        model: used_model.public_id.clone(),
        provider: used_model.provider.clone(),
        provider_model_id: used_model.provider_model_id.clone(),
        capability: Capability::Embedding.as_str().to_string(),
        duration_ms: start.elapsed().as_millis() as i32,
        streamed: false,
        status: "success".to_string(),
        cost_usd,
        tags: vec![],
    };
    spawn_request_log(state.pool.clone(), log);

    if !used_model.id.is_nil() {
        state
            .model_router
            .record_latency(used_model.id, start.elapsed().as_millis() as i32);
    }

    Ok(Json(body).into_response())
}

async fn call_embedding(
    resolved: &ResolvedModel,
    req: godwit_core::EmbeddingRequest,
) -> Result<(godwit_core::EmbeddingResponse, godwit_providers::adapter::UsageReport), godwit_providers::adapter::ProviderError>
{
    let adapter = Arc::clone(&resolved.adapter);
    let credentials = resolved.resolved_credentials.clone();
    let model = resolved.model.clone();
    let (resp, usage) = with_retry(&default_retry_policy(), move || {
        let adapter = Arc::clone(&adapter);
        let credentials = credentials.clone();
        let model = model.clone();
        let req = req.clone();
        async move { adapter.embedding(&credentials, &model, req).await }
    })
    .await?;
    let ProviderResponse::Embedding(body) = resp else {
        return Err(godwit_providers::adapter::ProviderError::Provider(
            "unexpected provider response variant".to_string(),
        ));
    };
    Ok((body, usage))
}

async fn image_generations(
    State(state): State<Arc<AppState>>,
    Extension(api_key): Extension<ApiKey>,
    Json(req): Json<ImageGenerationRequest>,
) -> Result<Response, crate::error::ApiError> {
    let start = std::time::Instant::now();

    let mut primary_resolved = state
        .model_router
        .resolve(&req.model, Capability::ImageGeneration)
        .await?;
    let fallback_chain = DbModelRouter::fallback_chain(&primary_resolved.model);

    let estimated_tokens = 1u32;
    check_rate_limit(&state, &api_key, &primary_resolved.model, estimated_tokens).await?;
    check_user_budget(&state, &api_key).await?;
    check_team_budget(&state, &api_key).await?;

    let mut rate_limited_err: Option<crate::error::ApiError> = None;
    let (body, usage, used_model) = match call_image_generation( &primary_resolved, req.clone()).await {
        Ok((resp, usage)) => (resp, usage, primary_resolved.model.clone()),
        Err(e) => {
            std::mem::drop(primary_resolved.in_flight.take());
            let mut fallback_result = None;
            for fallback_id in fallback_chain {
                tracing::info!(
                    "falling back from {} to {} for image generation",
                    req.model,
                    fallback_id
                );
                match state
                    .model_router
                    .resolve(&fallback_id, Capability::ImageGeneration)
                    .await
                {
                    Ok(resolved) => {
                        if let Err(rl_err) =
                            check_rate_limit(&state, &api_key, &resolved.model, estimated_tokens)
                                .await
                        {
                            rate_limited_err = Some(rl_err);
                            continue;
                        }
                        match call_image_generation( &resolved, req.clone()).await {
                            Ok((resp, usage)) => {
                                fallback_result = Some((resp, usage, resolved.model.clone()));
                                break;
                            }
                            Err(_e) => {
                                continue;
                            }
                        }
                    }
                    Err(_) => continue,
                }
            }
            fallback_result.ok_or_else(|| rate_limited_err.unwrap())?
        }
    };
    let cost_usd = compute_cost(&used_model, Capability::ImageGeneration, &usage);

    spawn_request_log(
        state.pool.clone(),
        RequestLogEntry {
            api_key_id: api_key.id,
            user_id: api_key.user_id,
            organization_id: api_key.organization_id,
            team_id: api_key.team_id,
            model: used_model.public_id.clone(),
            provider: used_model.provider.clone(),
            provider_model_id: used_model.provider_model_id.clone(),
            capability: Capability::ImageGeneration.as_str().to_string(),
            duration_ms: start.elapsed().as_millis() as i32,
            streamed: false,
            status: "success".to_string(),
            cost_usd,
            tags: vec![],
        },
    );

    if !used_model.id.is_nil() {
        state
            .model_router
            .record_latency(used_model.id, start.elapsed().as_millis() as i32);
    }

    Ok(Json(body).into_response())
}

async fn call_image_generation(
    resolved: &ResolvedModel,
    req: ImageGenerationRequest,
) -> Result<(godwit_core::ImageGenerationResponse, godwit_providers::adapter::UsageReport), godwit_providers::adapter::ProviderError>
{
    let adapter = Arc::clone(&resolved.adapter);
    let credentials = resolved.resolved_credentials.clone();
    let model = resolved.model.clone();
    let (resp, usage) = with_retry(&default_retry_policy(), move || {
        let adapter = Arc::clone(&adapter);
        let credentials = credentials.clone();
        let model = model.clone();
        let req = req.clone();
        async move { adapter.image_generation(&credentials, &model, req).await }
    })
    .await?;
    let ProviderResponse::Image(body) = resp else {
        return Err(godwit_providers::adapter::ProviderError::Provider(
            "unexpected provider response variant".to_string(),
        ));
    };
    Ok((body, usage))
}

async fn audio_speech(
    State(state): State<Arc<AppState>>,
    Extension(api_key): Extension<ApiKey>,
    Json(req): Json<godwit_core::AudioTtsRequest>,
) -> Result<Response, crate::error::ApiError> {
    let start = std::time::Instant::now();

    let mut primary_resolved = state
        .model_router
        .resolve(&req.model, Capability::AudioTts)
        .await?;
    let fallback_chain = DbModelRouter::fallback_chain(&primary_resolved.model);

    let estimated_tokens = 1u32;
    check_rate_limit(&state, &api_key, &primary_resolved.model, estimated_tokens).await?;
    check_user_budget(&state, &api_key).await?;
    check_team_budget(&state, &api_key).await?;

    let mut rate_limited_err: Option<crate::error::ApiError> = None;
    let ((bytes, content_type), usage, used_model) = match call_audio_speech( &primary_resolved, req.clone()).await {
        Ok(((bytes, content_type), usage)) => ((bytes, content_type), usage, primary_resolved.model.clone()),
        Err(e) => {
            std::mem::drop(primary_resolved.in_flight.take());
            let mut fallback_result = None;
            for fallback_id in fallback_chain {
                tracing::info!(
                    "falling back from {} to {} for audio speech",
                    req.model,
                    fallback_id
                );
                match state
                    .model_router
                    .resolve(&fallback_id, Capability::AudioTts)
                    .await
                {
                    Ok(resolved) => {
                        if let Err(rl_err) =
                            check_rate_limit(&state, &api_key, &resolved.model, estimated_tokens)
                                .await
                        {
                            rate_limited_err = Some(rl_err);
                            continue;
                        }
                        match call_audio_speech( &resolved, req.clone()).await {
                            Ok(((bytes, content_type), usage)) => {
                                fallback_result = Some(((bytes, content_type), usage, resolved.model.clone()));
                                break;
                            }
                            Err(_e) => {
                                continue;
                            }
                        }
                    }
                    Err(_) => continue,
                }
            }
            fallback_result.ok_or_else(|| rate_limited_err.unwrap())?
        }
    };
    let cost_usd = compute_cost(&used_model, Capability::AudioTts, &usage);

    spawn_request_log(
        state.pool.clone(),
        RequestLogEntry {
            api_key_id: api_key.id,
            user_id: api_key.user_id,
            organization_id: api_key.organization_id,
            team_id: api_key.team_id,
            model: used_model.public_id.clone(),
            provider: used_model.provider.clone(),
            provider_model_id: used_model.provider_model_id.clone(),
            capability: Capability::AudioTts.as_str().to_string(),
            duration_ms: start.elapsed().as_millis() as i32,
            streamed: false,
            status: "success".to_string(),
            cost_usd,
            tags: vec![],
        },
    );

    if !used_model.id.is_nil() {
        state
            .model_router
            .record_latency(used_model.id, start.elapsed().as_millis() as i32);
    }

    Ok(([(axum::http::header::CONTENT_TYPE, content_type)], bytes).into_response())
}

async fn call_audio_speech(
    resolved: &ResolvedModel,
    req: godwit_core::AudioTtsRequest,
) -> Result<((Vec<u8>, String), godwit_providers::adapter::UsageReport), godwit_providers::adapter::ProviderError>
{
    let adapter = Arc::clone(&resolved.adapter);
    let credentials = resolved.resolved_credentials.clone();
    let model = resolved.model.clone();
    let (resp, usage) = with_retry(&default_retry_policy(), move || {
        let adapter = Arc::clone(&adapter);
        let credentials = credentials.clone();
        let model = model.clone();
        let req = req.clone();
        async move { adapter.audio_tts(&credentials, &model, req).await }
    })
    .await?;
    let ProviderResponse::Bytes(bytes, content_type) = resp else {
        return Err(godwit_providers::adapter::ProviderError::Provider(
            "unexpected provider response variant".to_string(),
        ));
    };
    Ok(((bytes, content_type), usage))
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

    let mut primary_resolved = state
        .model_router
        .resolve(&model_name, Capability::AudioStt)
        .await?;
    let fallback_chain = DbModelRouter::fallback_chain(&primary_resolved.model);

    let estimated_tokens = 1u32;
    check_rate_limit(&state, &api_key, &primary_resolved.model, estimated_tokens).await?;
    check_user_budget(&state, &api_key).await?;
    check_team_budget(&state, &api_key).await?;

    let req = godwit_core::AudioSttRequest {
        model: model_name.clone(),
        language,
        response_format,
    };
    let file_bytes_clone = file_bytes.clone();
    let filename_clone = filename.clone();
    let content_type_clone = content_type.clone();

    let mut rate_limited_err: Option<crate::error::ApiError> = None;
    let (body, usage, used_model) = match call_audio_transcription( &primary_resolved, req.clone(), file_bytes_clone.clone(), filename_clone.clone(), content_type_clone.clone()).await {
        Ok((resp, usage)) => (resp, usage, primary_resolved.model.clone()),
        Err(e) => {
            std::mem::drop(primary_resolved.in_flight.take());
            let mut fallback_result = None;
            for fallback_id in fallback_chain {
                tracing::info!(
                    "falling back from {} to {} for audio transcription",
                    model_name,
                    fallback_id
                );
                match state
                    .model_router
                    .resolve(&fallback_id, Capability::AudioStt)
                    .await
                {
                    Ok(resolved) => {
                        if let Err(rl_err) =
                            check_rate_limit(&state, &api_key, &resolved.model, estimated_tokens)
                                .await
                        {
                            rate_limited_err = Some(rl_err);
                            continue;
                        }
                        match call_audio_transcription( &resolved, req.clone(), file_bytes_clone.clone(), filename_clone.clone(), content_type_clone.clone()).await {
                            Ok((resp, usage)) => {
                                fallback_result = Some((resp, usage, resolved.model.clone()));
                                break;
                            }
                            Err(_e) => {
                                continue;
                            }
                        }
                    }
                    Err(_) => continue,
                }
            }
            fallback_result.ok_or_else(|| rate_limited_err.unwrap())?
        }
    };
    let cost_usd = compute_cost(&used_model, Capability::AudioStt, &usage);

    spawn_request_log(
        state.pool.clone(),
        RequestLogEntry {
            api_key_id: api_key.id,
            user_id: api_key.user_id,
            organization_id: api_key.organization_id,
            team_id: api_key.team_id,
            model: used_model.public_id.clone(),
            provider: used_model.provider.clone(),
            provider_model_id: used_model.provider_model_id.clone(),
            capability: Capability::AudioStt.as_str().to_string(),
            duration_ms: start.elapsed().as_millis() as i32,
            streamed: false,
            status: "success".to_string(),
            cost_usd,
            tags: vec![],
        },
    );

    if !used_model.id.is_nil() {
        state
            .model_router
            .record_latency(used_model.id, start.elapsed().as_millis() as i32);
    }

    Ok(Json(body).into_response())
}

async fn call_audio_transcription(
    resolved: &ResolvedModel,
    req: godwit_core::AudioSttRequest,
    file_bytes: Vec<u8>,
    filename: String,
    content_type: String,
) -> Result<(godwit_core::AudioSttResponse, godwit_providers::adapter::UsageReport), godwit_providers::adapter::ProviderError>
{
    let adapter = Arc::clone(&resolved.adapter);
    let credentials = resolved.resolved_credentials.clone();
    let model = resolved.model.clone();
    let (resp, usage) = with_retry(&default_retry_policy(), move || {
        let adapter = Arc::clone(&adapter);
        let credentials = credentials.clone();
        let model = model.clone();
        let req = req.clone();
        let file_bytes = file_bytes.clone();
        let filename = filename.clone();
        let content_type = content_type.clone();
        async move {
            adapter
                .audio_stt(&credentials, &model, req, file_bytes, filename, content_type)
                .await
        }
    })
    .await?;
    let ProviderResponse::AudioStt(body) = resp else {
        return Err(godwit_providers::adapter::ProviderError::Provider(
            "unexpected provider response variant".to_string(),
        ));
    };
    Ok((body, usage))
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

    let mut primary_resolved = state
        .model_router
        .resolve(&model_name, Capability::ImageEdit)
        .await?;
    let fallback_chain = DbModelRouter::fallback_chain(&primary_resolved.model);

    let estimated_tokens = 1u32;
    check_rate_limit(&state, &api_key, &primary_resolved.model, estimated_tokens).await?;
    check_user_budget(&state, &api_key).await?;
    check_team_budget(&state, &api_key).await?;

    let req = godwit_core::ImageEditRequest {
        model: model_name.clone(),
        prompt,
        n,
        size,
        response_format,
    };
    let image_bytes_clone = image_bytes.clone();
    let image_filename_clone = image_filename.clone();
    let mask_bytes_clone = mask_bytes.clone();

    let mut rate_limited_err: Option<crate::error::ApiError> = None;
    let (body, usage, used_model) = match call_image_edit( &primary_resolved, req.clone(), image_bytes_clone.clone(), image_filename_clone.clone(), mask_bytes_clone.clone()).await {
        Ok((resp, usage)) => (resp, usage, primary_resolved.model.clone()),
        Err(e) => {
            std::mem::drop(primary_resolved.in_flight.take());
            let mut fallback_result = None;
            for fallback_id in fallback_chain {
                tracing::info!(
                    "falling back from {} to {} for image edit",
                    model_name,
                    fallback_id
                );
                match state
                    .model_router
                    .resolve(&fallback_id, Capability::ImageEdit)
                    .await
                {
                    Ok(resolved) => {
                        if let Err(rl_err) =
                            check_rate_limit(&state, &api_key, &resolved.model, estimated_tokens)
                                .await
                        {
                            rate_limited_err = Some(rl_err);
                            continue;
                        }
                        match call_image_edit( &resolved, req.clone(), image_bytes_clone.clone(), image_filename_clone.clone(), mask_bytes_clone.clone()).await {
                            Ok((resp, usage)) => {
                                fallback_result = Some((resp, usage, resolved.model.clone()));
                                break;
                            }
                            Err(_e) => {
                                continue;
                            }
                        }
                    }
                    Err(_) => continue,
                }
            }
            fallback_result.ok_or_else(|| rate_limited_err.unwrap())?
        }
    };
    let cost_usd = compute_cost(&used_model, Capability::ImageEdit, &usage);

    spawn_request_log(
        state.pool.clone(),
        RequestLogEntry {
            api_key_id: api_key.id,
            user_id: api_key.user_id,
            organization_id: api_key.organization_id,
            team_id: api_key.team_id,
            model: used_model.public_id.clone(),
            provider: used_model.provider.clone(),
            provider_model_id: used_model.provider_model_id.clone(),
            capability: Capability::ImageEdit.as_str().to_string(),
            duration_ms: start.elapsed().as_millis() as i32,
            streamed: false,
            status: "success".to_string(),
            cost_usd,
            tags: vec![],
        },
    );

    if !used_model.id.is_nil() {
        state
            .model_router
            .record_latency(used_model.id, start.elapsed().as_millis() as i32);
    }

    Ok(Json(body).into_response())
}

async fn call_image_edit(
    resolved: &ResolvedModel,
    req: godwit_core::ImageEditRequest,
    image_bytes: Vec<u8>,
    image_filename: String,
    mask_bytes: Option<Vec<u8>>,
) -> Result<(godwit_core::ImageGenerationResponse, godwit_providers::adapter::UsageReport), godwit_providers::adapter::ProviderError>
{
    let adapter = Arc::clone(&resolved.adapter);
    let credentials = resolved.resolved_credentials.clone();
    let model = resolved.model.clone();
    let (resp, usage) = with_retry(&default_retry_policy(), move || {
        let adapter = Arc::clone(&adapter);
        let credentials = credentials.clone();
        let model = model.clone();
        let req = req.clone();
        let image_bytes = image_bytes.clone();
        let image_filename = image_filename.clone();
        let mask_bytes = mask_bytes.clone();
        async move {
            adapter
                .image_edit(&credentials, &model, req, image_bytes, image_filename, mask_bytes)
                .await
        }
    })
    .await?;
    let ProviderResponse::Image(body) = resp else {
        return Err(godwit_providers::adapter::ProviderError::Provider(
            "unexpected provider response variant".to_string(),
        ));
    };
    Ok((body, usage))
}

fn extract_tags_from_header(header_value: Option<&str>) -> Vec<String> {
    header_value
        .map(|h| h.split(',').map(|s| s.trim().to_string()).filter(|s| !s.is_empty()).collect())
        .unwrap_or_default()
}

pub(crate) fn spawn_request_log(pool: sqlx::PgPool, log: RequestLogEntry) {
    tokio::spawn(async move {
        let _ = sqlx::query(
            "INSERT INTO request_logs (api_key_id, user_id, organization_id, team_id, model, provider, provider_model_id, capability, duration_ms, streamed, status, cost_usd, tags)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13)"
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
        .bind(&log.tags)
        .execute(&pool)
        .await;
    });
}

#[derive(Clone)]
pub(crate) struct RequestLogEntry {
    pub(crate) api_key_id: uuid::Uuid,
    pub(crate) user_id: uuid::Uuid,
    pub(crate) organization_id: uuid::Uuid,
    pub(crate) team_id: Option<uuid::Uuid>,
    pub(crate) model: String,
    pub(crate) provider: String,
    pub(crate) provider_model_id: String,
    pub(crate) capability: String,
    pub(crate) duration_ms: i32,
    pub(crate) streamed: bool,
    pub(crate) status: String,
    pub(crate) cost_usd: Option<Decimal>,
    pub(crate) tags: Vec<String>,
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
            model_for(uuid::Uuid::new_v4(), "orphaned"),
        ];
        let profiles = vec![
            profile_with(enabled_id, true),
            profile_with(disabled_id, false),
        ];

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

    #[test]
    fn rate_limited_error_maps_to_429_with_retry_after() {
        let resp = crate::error::ApiError::RateLimited(Some(42)).into_response();
        assert_eq!(resp.status(), StatusCode::TOO_MANY_REQUESTS);
        let retry_after = resp
            .headers()
            .get(axum::http::header::RETRY_AFTER)
            .and_then(|v| v.to_str().ok())
            .map(|s| s.to_string());
        assert_eq!(retry_after, Some("42".to_string()));
    }

    #[test]
    fn estimate_request_tokens_uses_chat_request() {
        let req = ChatCompletionRequest {
            model: "gpt-4o".to_string(),
            messages: vec![godwit_core::ChatMessage {
                role: "user".to_string(),
                content: godwit_core::ChatContent::Text("hello".to_string()),
                name: None,
                tool_calls: None,
                tool_call_id: None,
                cache_control: None,
            }],
            max_tokens: Some(10),
            ..Default::default()
        };
        let tokens = rate_limit::estimate_request_tokens(&req);
        assert!(tokens >= 10);
    }

    #[test]
    fn openai_wire_flag_changes_sse_format() {
        use godwit_core::CompatConfig;

        let config_with_wire = AppConfig {
            server: ServerConfig {
                host: "127.0.0.1".to_string(),
                port: 3000,
                request_timeout_seconds: 60,
            },
            database: DatabaseConfig {
                url: "postgres://test@test/test".to_string(),
            },
            auth: AuthConfig {
                jwt_secret: "test".to_string(),
                access_token_ttl_minutes: 15,
                refresh_token_ttl_days: 7,
                oidc_providers: vec![],
                saml_providers: vec![],
            },
            agentic: Default::default(),
            compat: Some(CompatConfig {
                openai_wire_streaming: true,
            }),
            circuit_breaker: None,
        };

        let config_without_wire = AppConfig {
            server: ServerConfig {
                host: "127.0.0.1".to_string(),
                port: 3000,
                request_timeout_seconds: 60,
            },
            database: DatabaseConfig {
                url: "postgres://test@test/test".to_string(),
            },
            auth: AuthConfig {
                jwt_secret: "test".to_string(),
                access_token_ttl_minutes: 15,
                refresh_token_ttl_days: 7,
                oidc_providers: vec![],
                saml_providers: vec![],
            },
            agentic: Default::default(),
            compat: Some(CompatConfig {
                openai_wire_streaming: false,
            }),
            circuit_breaker: None,
        };

        let config_default = AppConfig {
            server: ServerConfig {
                host: "127.0.0.1".to_string(),
                port: 3000,
                request_timeout_seconds: 60,
            },
            database: DatabaseConfig {
                url: "postgres://test@test/test".to_string(),
            },
            auth: AuthConfig {
                jwt_secret: "test".to_string(),
                access_token_ttl_minutes: 15,
                refresh_token_ttl_days: 7,
                oidc_providers: vec![],
                saml_providers: vec![],
            },
            agentic: Default::default(),
            compat: None,
            circuit_breaker: None,
        };

        assert!(config_with_wire
            .compat
            .as_ref()
            .map(|c| c.openai_wire_streaming)
            .unwrap_or(false));
        assert!(!config_without_wire
            .compat
            .as_ref()
            .map(|c| c.openai_wire_streaming)
            .unwrap_or(false));
        assert!(!config_default
            .compat
            .as_ref()
            .map(|c| c.openai_wire_streaming)
            .unwrap_or(false));
    }
}
