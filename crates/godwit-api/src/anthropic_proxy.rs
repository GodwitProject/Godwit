use axum::{
    extract::{Extension, Json, State},
    http::StatusCode,
    middleware,
    response::{IntoResponse, Response, Sse},
    routing::post,
    Router,
};
use futures::StreamExt;
use godwit_core::{
    ChatCompletionRequest, ChatContent, ChatMessage, FunctionDefinition, FunctionName, Stop,
    Tool, ToolChoice, Usage,
};
use godwit_db::models::ApiKey;
use godwit_providers::{compute_cost, sse_egress::AnthropicStreamTranslator, ProviderResponse};
use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};
use uuid::Uuid;

use crate::{
    model_router::{DbModelRouter, ResolvedModel},
    proxy::RequestLogEntry,
    rate_limit,
    resilience::{with_retry, RetryPolicy},
    state::AppState,
};

#[derive(Debug, Deserialize)]
pub struct AnthropicMessagesRequest {
    pub model: String,
    pub messages: Vec<AnthropicMessage>,
    pub max_tokens: i32,
    #[serde(default)]
    pub stream: bool,
    #[serde(default)]
    pub system: Option<String>,
    pub temperature: Option<f32>,
    pub top_p: Option<f32>,
    pub top_k: Option<i32>,
    pub stop_sequences: Option<Vec<String>>,
    pub tools: Option<Vec<AnthropicTool>>,
    pub tool_choice: Option<AnthropicToolChoice>,
}

#[derive(Debug, Deserialize)]
pub struct AnthropicMessage {
    pub role: String,
    pub content: String,
}

#[derive(Debug, Deserialize)]
pub struct AnthropicTool {
    pub name: String,
    pub description: Option<String>,
    pub input_schema: Option<serde_json::Value>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AnthropicToolChoice {
    Auto,
    Any,
    Tool { name: String },
}

#[derive(Debug, Serialize)]
pub struct AnthropicMessagesResponse {
    pub id: String,
    pub r#type: String,
    pub role: String,
    pub model: String,
    pub content: Vec<AnthropicContentBlock>,
    pub stop_reason: Option<String>,
    pub usage: AnthropicUsage,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "snake_case", tag = "type")]
pub enum AnthropicContentBlock {
    Text { text: String },
    ToolUse { id: String, name: String, input: serde_json::Value },
}

#[derive(Debug, Serialize)]
pub struct AnthropicUsage {
    pub input_tokens: i32,
    pub output_tokens: i32,
}

pub fn router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/v1/messages", post(messages))
        .route_layer(middleware::from_fn(crate::middleware::model_scope))
}

fn anthropic_message_to_core(msg: AnthropicMessage) -> ChatMessage {
    ChatMessage {
        role: msg.role,
        content: Some(vec![ChatContent::text(msg.content)]),
        name: None,
        tool_calls: None,
        tool_call_id: None,
        cache_control: None,
    }
}

fn anthropic_tool_to_core(tool: AnthropicTool) -> Tool {
    Tool {
        r#type: "function".into(),
        function: FunctionDefinition {
            name: tool.name,
            description: tool.description,
            parameters: tool.input_schema,
        },
    }
}

fn anthropic_request_to_core(req: AnthropicMessagesRequest) -> ChatCompletionRequest {
    let mut messages = Vec::new();
    if let Some(system) = req.system {
        messages.push(ChatMessage {
            role: "system".into(),
            content: Some(vec![ChatContent::text(system)]),
            name: None,
            tool_calls: None,
            tool_call_id: None,
            cache_control: None,
        });
    }
    messages.extend(req.messages.into_iter().map(anthropic_message_to_core));

    ChatCompletionRequest {
        model: req.model,
        messages,
        max_tokens: Some(req.max_tokens),
        temperature: req.temperature,
        top_p: req.top_p,
        top_k: req.top_k,
        stop: req.stop_sequences.map(Stop::Array),
        stream: Some(req.stream),
        tools: req.tools.map(|ts| ts.into_iter().map(anthropic_tool_to_core).collect()),
        tool_choice: req.tool_choice.map(|tc| match tc {
            AnthropicToolChoice::Auto => ToolChoice::Auto,
            AnthropicToolChoice::Any => ToolChoice::Required,
            AnthropicToolChoice::Tool { name } => ToolChoice::Function {
                function: FunctionName { name },
            },
        }),
        ..Default::default()
    }
}

fn core_finish_reason_to_anthropic(reason: Option<&str>) -> Option<String> {
    match reason {
        Some("stop") => Some("end_turn".into()),
        Some("length") => Some("max_tokens".into()),
        Some("tool_calls") => Some("tool_use".into()),
        Some("content_filter") => Some("end_turn".into()),
        Some(other) => Some(other.into()),
        None => None,
    }
}

fn core_usage_to_anthropic(usage: Option<&Usage>) -> AnthropicUsage {
    match usage {
        Some(u) => AnthropicUsage {
            input_tokens: u.prompt_tokens,
            output_tokens: u.completion_tokens,
        },
        None => AnthropicUsage {
            input_tokens: 0,
            output_tokens: 0,
        },
    }
}

fn core_response_to_anthropic(response: godwit_core::ChatCompletionResponse) -> AnthropicMessagesResponse {
    let choice = response.choices.into_iter().next();
    let mut content_blocks = Vec::new();
    let stop_reason;

    if let Some(choice) = choice {
        stop_reason = core_finish_reason_to_anthropic(choice.finish_reason.as_deref());

        if let Some(tool_calls) = &choice.message.tool_calls {
            for call in tool_calls {
                let input = serde_json::from_str(&call.function.arguments).unwrap_or_else(|_| {
                    serde_json::json!({"arguments": call.function.arguments})
                });
                content_blocks.push(AnthropicContentBlock::ToolUse {
                    id: call.id.clone(),
                    name: call.function.name.clone(),
                    input,
                });
            }
        }

        if let Some(text) = choice.message.content_as_text() {
            if !text.is_empty() {
                content_blocks.push(AnthropicContentBlock::Text { text });
            }
        }
    } else {
        stop_reason = None;
    }

    AnthropicMessagesResponse {
        id: response.id,
        r#type: "message".into(),
        role: "assistant".into(),
        model: response.model,
        content: content_blocks,
        stop_reason,
        usage: core_usage_to_anthropic(response.usage.as_ref()),
    }
}

async fn messages(
    State(state): State<Arc<AppState>>,
    Extension(api_key): Extension<ApiKey>,
    Json(req): Json<AnthropicMessagesRequest>,
) -> Result<Response, crate::error::ApiError> {
    let start = std::time::Instant::now();
    let streamed = req.stream;

    let core_req = anthropic_request_to_core(req);
    let mut resolved = state
        .model_router
        .resolve(&core_req.model, godwit_core::Capability::Chat)
        .await?;

    let estimated_tokens = rate_limit::estimate_request_tokens(&core_req);
    let org = state
        .org_repo
        .get_by_id(api_key.organization_id)
        .await
        .map_err(crate::error::ApiError::Core)?;
    if let Err((_, retry_after)) = state.rate_limiter.check_and_consume(
        api_key.id,
        api_key.organization_id,
        &resolved.model.public_id,
        api_key.rate_limit_requests_per_minute,
        api_key.rate_limit_tokens_per_minute,
        org.rate_limit_requests_per_minute,
        org.rate_limit_tokens_per_minute,
        estimated_tokens,
    ) {
        return Err(crate::error::ApiError::RateLimited(retry_after));
    }

    let fallback_chain = DbModelRouter::fallback_chain(&resolved.model);
    let mut last_err: Option<crate::error::ApiError> = None;
    let mut rate_limited_err: Option<crate::error::ApiError> = None;

    if streamed {
        let stream = 'stream_attempt: {
            let result = attempt_anthropic_stream(&resolved, core_req.clone()).await;
            match result {
                Ok(s) => break 'stream_attempt s,
                Err(e) => {
                    last_err = Some(crate::proxy::map_provider_error(e));
                    // Release the primary model's in-flight slot before attempting fallbacks.
                    std::mem::drop(resolved.in_flight.take());
                    for fallback_id in fallback_chain {
                        tracing::info!(
                            "falling back from {} to {} for anthropic messages (stream)",
                            core_req.model,
                            fallback_id
                        );
                        match state
                            .model_router
                            .resolve(&fallback_id, godwit_core::Capability::Chat)
                            .await
                        {
                            Ok(fallback_resolved) => {
                                if let Err((_, retry_after)) = state.rate_limiter.check_and_consume(
                                    api_key.id,
                                    api_key.organization_id,
                                    &fallback_resolved.model.public_id,
                                    api_key.rate_limit_requests_per_minute,
                                    api_key.rate_limit_tokens_per_minute,
                                    org.rate_limit_requests_per_minute,
                                    org.rate_limit_tokens_per_minute,
                                    estimated_tokens,
                                ) {
                                    tracing::info!(
                                        "rate limited on fallback {} for anthropic messages (stream)",
                                        fallback_id
                                    );
                                    rate_limited_err =
                                        Some(crate::error::ApiError::RateLimited(retry_after));
                                    continue;
                                }
                                match attempt_anthropic_stream(
                                    &fallback_resolved,
                                    core_req.clone(),
                                )
                                .await
                                {
                                    Ok(s) => break 'stream_attempt s,
                                    Err(e) => {
                                        last_err =
                                            Some(crate::proxy::map_provider_error(e));
                                        continue;
                                    }
                                }
                            }
                            Err(_) => continue,
                        }
                    }
                    return Err(rate_limited_err.or(last_err).unwrap());
                }
            }
        };

        let cost_usd = compute_cost(
            &resolved.model.pricing,
            godwit_core::Capability::Chat,
            &godwit_providers::adapter::UsageReport::default(),
        );
        let log = RequestLogEntry {
            api_key_id: api_key.id,
            user_id: api_key.user_id,
            organization_id: api_key.organization_id,
            team_id: api_key.team_id,
            model: resolved.model.public_id.clone(),
            provider: resolved.model.provider.clone(),
            provider_model_id: resolved.model.provider_model_id.clone(),
            capability: godwit_core::Capability::Chat.as_str().to_string(),
            duration_ms: start.elapsed().as_millis() as i32,
            streamed: true,
            status: "success".to_string(),
            cost_usd,
            tags: vec![],
            attempt_number: 1,
            fallback_triggered: false,
        };
        crate::proxy::spawn_request_log(state.pool.clone(), log);

        if !resolved.model.id.is_nil() {
            state
                .model_router
                .record_latency(resolved.model.id, start.elapsed().as_millis() as i32);
        }

        return Ok((StatusCode::OK, stream).into_response());
    }

    let (anthropic_resp, used_model) = 'attempt: {
        let result = attempt_anthropic_chat(&resolved, core_req.clone()).await;
        match result {
            Ok(resp) => break 'attempt (resp, resolved.model.clone()),
            Err(e) => {
                last_err = Some(crate::proxy::map_provider_error(e));
                // Release the primary model's in-flight slot before attempting fallbacks.
                std::mem::drop(resolved.in_flight.take());
                for fallback_id in fallback_chain {
                    tracing::info!(
                        "falling back from {} to {} for anthropic messages",
                        core_req.model,
                        fallback_id
                    );
                    match state
                        .model_router
                        .resolve(&fallback_id, godwit_core::Capability::Chat)
                        .await
                    {
                        Ok(fallback_resolved) => {
                            if let Err((_, retry_after)) = state.rate_limiter.check_and_consume(
                                api_key.id,
                                api_key.organization_id,
                                &fallback_resolved.model.public_id,
                                api_key.rate_limit_requests_per_minute,
                                api_key.rate_limit_tokens_per_minute,
                                org.rate_limit_requests_per_minute,
                                org.rate_limit_tokens_per_minute,
                                estimated_tokens,
                            ) {
                                tracing::info!(
                                    "rate limited on fallback {} for anthropic messages",
                                    fallback_id
                                );
                                rate_limited_err =
                                    Some(crate::error::ApiError::RateLimited(retry_after));
                                continue;
                            }
                            match attempt_anthropic_chat(&fallback_resolved, core_req.clone())
                                .await
                            {
                                Ok(resp) => {
                                    break 'attempt (resp, fallback_resolved.model.clone())
                                }
                                Err(e) => {
                                    last_err = Some(crate::proxy::map_provider_error(e));
                                    continue;
                                }
                            }
                        }
                        Err(_) => continue,
                    }
                }
                return Err(rate_limited_err.or(last_err).unwrap());
            }
        }
    };

    let cost_usd = compute_cost(
        &used_model.pricing,
        godwit_core::Capability::Chat,
        &godwit_providers::adapter::UsageReport {
            prompt_tokens: Some(anthropic_resp.usage.input_tokens),
            completion_tokens: Some(anthropic_resp.usage.output_tokens),
            ..Default::default()
        },
    );
    let log = RequestLogEntry {
        api_key_id: api_key.id,
        user_id: api_key.user_id,
        organization_id: api_key.organization_id,
        team_id: api_key.team_id,
        model: used_model.public_id.clone(),
        provider: used_model.provider.clone(),
        provider_model_id: used_model.provider_model_id.clone(),
        capability: godwit_core::Capability::Chat.as_str().to_string(),
        duration_ms: start.elapsed().as_millis() as i32,
        streamed,
        status: "success".to_string(),
        cost_usd,
        tags: vec![],
        attempt_number: 1,
        fallback_triggered: false,
    };
    crate::proxy::spawn_request_log(state.pool.clone(), log);

    if !used_model.id.is_nil() {
        state
            .model_router
            .record_latency(used_model.id, start.elapsed().as_millis() as i32);
    }

    Ok((StatusCode::OK, Json(anthropic_resp)).into_response())
}

async fn attempt_anthropic_chat(
    resolved: &ResolvedModel,
    core_req: godwit_core::ChatCompletionRequest,
) -> Result<AnthropicMessagesResponse, godwit_providers::adapter::ProviderError> {
    let adapter = Arc::clone(&resolved.adapter);
    let credentials = resolved.resolved_credentials.clone();
    let model = resolved.model.clone();
    let (resp, _report) = with_retry(&RetryPolicy::default(), move || {
        let adapter = Arc::clone(&adapter);
        let credentials = credentials.clone();
        let model = model.clone();
        let core_req = core_req.clone();
        async move { adapter.chat(&credentials, &model, core_req).await }
    })
    .await?;
    match resp {
        ProviderResponse::Chat(completion) => Ok(core_response_to_anthropic(completion)),
        _ => Err(godwit_providers::adapter::ProviderError::Provider(
            "unexpected provider response variant".to_string(),
        )),
    }
}

async fn attempt_anthropic_stream(
    resolved: &ResolvedModel,
    core_req: godwit_core::ChatCompletionRequest,
) -> Result<Sse<impl futures::Stream<Item = Result<axum::response::sse::Event, std::convert::Infallible>>>, godwit_providers::adapter::ProviderError>
{
    let adapter = Arc::clone(&resolved.adapter);
    let credentials = resolved.resolved_credentials.clone();
    let model = resolved.model.clone();
    let stream = with_retry(&RetryPolicy::default(), move || {
        let adapter = Arc::clone(&adapter);
        let credentials = credentials.clone();
        let model = model.clone();
        let core_req = core_req.clone();
        async move { adapter.chat_stream(&credentials, &model, core_req).await }
    })
    .await?;

    let stream_id = format!("msg_{}", Uuid::new_v4());
    let stream_model = if resolved.model.provider_model_id.is_empty() {
        resolved.model.public_id.clone()
    } else {
        resolved.model.provider_model_id.clone()
    };
    let translator = Mutex::new(AnthropicStreamTranslator::new(stream_id, stream_model));
    let sse_stream = stream.flat_map(move |event| {
        let events = event
            .map(|e| {
                let mut translator = translator.lock().unwrap();
                translator
                    .push(&e)
                    .iter()
                    .map(|f| axum::response::sse::Event::default().data(f.render()))
                    .collect::<Vec<_>>()
            })
            .unwrap_or_else(|_| {
                let error_payload = serde_json::json!({
                    "error": {
                        "type": "error",
                        "message": "upstream provider stream error",
                    }
                });
                vec![axum::response::sse::Event::default().data(error_payload.to_string())]
            });
        futures::stream::iter(
            events
                .into_iter()
                .map(Ok::<_, std::convert::Infallible>),
        )
    });
    Ok(Sse::new(sse_stream))
}
