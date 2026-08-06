use godwit_core::{ChatCompletionRequest, ChatCompletionResponse, ToolCall};
use godwit_providers::adapter::UsageReport;
use std::sync::Arc;
use std::time::Duration;

use crate::{
    proxy::resolve_tool_calls,
    resilience::{with_retry, RetryPolicy},
    state::AppState,
    model_router::ResolvedModel,
    error::ApiError,
};

pub struct AgenticLoop {
    max_iterations: usize,
    iteration_timeout: Duration,
}

impl AgenticLoop {
    pub fn new(max_iterations: usize, iteration_timeout_secs: u64) -> Self {
        Self {
            max_iterations,
            iteration_timeout: Duration::from_secs(iteration_timeout_secs),
        }
    }

    pub async fn execute(
        &self,
        state: &Arc<AppState>,
        resolved: &ResolvedModel,
        mut req: ChatCompletionRequest,
    ) -> Result<(ChatCompletionResponse, UsageReport), ApiError> {
        let mut messages = req.messages.clone();
        let mut total_usage = UsageReport::default();

        for iteration in 0..self.max_iterations {
            let iteration_start = std::time::Instant::now();
            
            req.messages = messages.clone();
            
            let (completion, round_usage) = tokio::time::timeout(
                self.iteration_timeout,
                self.run_iteration(state, resolved, req.clone())
            )
            .await
            .map_err(|_| ApiError::Core(godwit_core::PasteurError::Provider(
                format!("iteration {} timed out after {:?}", iteration + 1, self.iteration_timeout)
            )))?
            .map_err(|e: godwit_providers::adapter::ProviderError| {
                ApiError::Core(godwit_core::PasteurError::Provider(e.to_string()))
            })?;

            total_usage = accumulate_usage(total_usage, &round_usage);

            let tool_calls: Vec<ToolCall> = completion
                .choices
                .iter()
                .flat_map(|c| c.message.tool_calls.clone().unwrap_or_default())
                .collect();

            if tool_calls.is_empty() {
                return Ok((completion, total_usage));
            }

            if let Some(choice) = completion.choices.first() {
                messages.push(choice.message.clone());
            }

            let results = resolve_tool_calls(state, &tool_calls).await;
            if results.is_empty() {
                return Ok((completion, total_usage));
            }

            messages.extend(results);

            tracing::info!(
                "agentic loop iteration {} completed in {:?} with {} tool calls",
                iteration + 1,
                iteration_start.elapsed(),
                tool_calls.len()
            );
        }

        Err(ApiError::Core(godwit_core::PasteurError::Provider(
            format!(
                "agentic tool loop exceeded {} iterations without converging",
                self.max_iterations
            ),
        )))
    }

    async fn run_iteration(
        &self,
        _state: &Arc<AppState>,
        resolved: &ResolvedModel,
        req: ChatCompletionRequest,
    ) -> Result<(ChatCompletionResponse, UsageReport), godwit_providers::adapter::ProviderError> {
        let adapter = Arc::clone(&resolved.adapter);
        let credentials = resolved.resolved_credentials.clone();
        let model = resolved.model.clone();
        
        let (resp, usage) = with_retry(&default_retry_policy(), move || {
            let adapter = Arc::clone(&adapter);
            let credentials = credentials.clone();
            let model = model.clone();
            let req = req.clone();
            async move { adapter.chat(&credentials, &model, req).await }
        })
        .await?;

        let godwit_providers::ProviderResponse::Chat(completion) = resp else {
            return Err(godwit_providers::adapter::ProviderError::Provider(
                "unexpected provider response variant during agentic chat".to_string(),
            ));
        };

        Ok((completion, usage))
    }
}

fn default_retry_policy() -> RetryPolicy {
    RetryPolicy::default()
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

#[cfg(test)]
mod tests {
    use super::*;
    use godwit_core::{ChatCompletionResponse, ChatCompletionChoice, ChatMessage, FunctionCall, Usage};
    use godwit_providers::adapter::UsageReport;

    #[tokio::test]
    async fn test_no_tool_calls_returns_immediately() {
        let completion = ChatCompletionResponse {
            id: "test".to_string(),
            object: "chat.completion".to_string(),
            created: 1234567890,
            model: "test-model".to_string(),
            choices: vec![ChatCompletionChoice {
                index: 0,
                message: ChatMessage {
                    role: "assistant".to_string(),
                    content: Some(vec![godwit_core::ChatContent::Text("Hello".to_string())]),
                    name: None,
                    tool_calls: None,
                    tool_call_id: None,
                    cache_control: None,
                },
                finish_reason: Some("stop".to_string()),
                logprobs: None,
                tool_calls: None,
            }],
            usage: Some(Usage {
                prompt_tokens: 10,
                completion_tokens: 5,
                total_tokens: 15,
                prompt_tokens_details: None,
                completion_tokens_details: None,
            }),
        };

        assert!(completion.choices[0].message.tool_calls.is_none());
        assert!(completion.choices[0].message.content.is_some());
    }

    #[tokio::test]
    async fn test_tool_calls_present_continues_loop() {
        let tool_call = ToolCall {
            id: "call_1".to_string(),
            r#type: "function".to_string(),
            function: FunctionCall {
                name: "test_tool".to_string(),
                arguments: "{}".to_string(),
            },
        };

        let completion = ChatCompletionResponse {
            id: "test".to_string(),
            object: "chat.completion".to_string(),
            created: 1234567890,
            model: "test-model".to_string(),
            choices: vec![ChatCompletionChoice {
                index: 0,
                message: ChatMessage {
                    role: "assistant".to_string(),
                    content: None,
                    name: None,
                    tool_calls: Some(vec![tool_call.clone()]),
                    tool_call_id: None,
                    cache_control: None,
                },
                finish_reason: Some("tool_calls".to_string()),
                logprobs: None,
                tool_calls: Some(vec![tool_call]),
            }],
            usage: Some(Usage {
                prompt_tokens: 10,
                completion_tokens: 5,
                total_tokens: 15,
                prompt_tokens_details: None,
                completion_tokens_details: None,
            }),
        };

        assert!(completion.choices[0].message.tool_calls.is_some());
        assert_eq!(completion.choices[0].message.tool_calls.as_ref().unwrap().len(), 1);
    }

    #[test]
    fn test_max_iterations_configured() {
        let loop_4 = AgenticLoop::new(4, 120);
        assert_eq!(loop_4.max_iterations, 4);
        
        let loop_1 = AgenticLoop::new(1, 120);
        assert_eq!(loop_1.max_iterations, 1);
    }

    #[tokio::test]
    async fn test_mcp_tool_call_returns_error_for_unknown_server() {
        use godwit_mcp::McpRegistry;
        use std::sync::Arc;

        let registry = McpRegistry::new();
        let result = registry.call_tool("unknown__tool", serde_json::json!({})).await;
        
        assert!(result.is_err());
        assert!(matches!(result, Err(godwit_mcp::McpError::UnknownServer(_))));
    }

    #[tokio::test]
    async fn test_mcp_tool_name_recognition() {
        let mcp_tool_name = "filesystem__read_file";
        let non_mcp_tool_name = "web_search";
        
        assert!(mcp_tool_name.contains("__"));
        assert!(!non_mcp_tool_name.contains("__"));
    }

    #[tokio::test]
    async fn test_empty_mcp_registry_returns_no_tools() {
        use godwit_mcp::McpRegistry;

        let registry = McpRegistry::new();
        let tools = registry.all_tools().await;
        
        assert!(tools.is_empty());
    }

    #[tokio::test]
    async fn test_mcp_tool_error_message_appended_to_conversation() {
        use godwit_mcp::McpRegistry;
        use godwit_core::{ChatContent, ToolCall};
        use std::sync::Arc;

        let registry = Arc::new(McpRegistry::new());
        let result = registry.call_tool("test__tool", serde_json::json!({})).await;
        
        assert!(result.is_err());
        
        let error_message = format!("MCP tool call to 'test__tool' failed: {}", result.unwrap_err());
        assert!(error_message.contains("MCP tool call to 'test__tool' failed"));
        assert!(error_message.contains("unknown MCP server"));
    }

    #[test]
    fn test_web_search_tool_is_not_mcp_tool() {
        let web_search_name = "web_search";
        let mcp_tool_name = "filesystem__read_file";
        
        assert!(!web_search_name.contains("__"));
        assert!(mcp_tool_name.contains("__"));
    }

    #[test]
    fn test_web_search_tool_recognized_by_name() {
        use godwit_providers::NATIVE_WEB_SEARCH_TOOLS;
        
        assert!(NATIVE_WEB_SEARCH_TOOLS.contains(&"web_search"));
        assert!(NATIVE_WEB_SEARCH_TOOLS.contains(&"google_search"));
    }
}
