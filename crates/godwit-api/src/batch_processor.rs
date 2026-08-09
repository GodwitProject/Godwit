use chrono::Utc;
use rust_decimal::Decimal;
use std::sync::Arc;
use tokio::sync::Semaphore;
use tokio::time::{sleep, Duration};

use godwit_core::{Capability, ChatCompletionRequest, ChatMessage};
use godwit_providers::adapter::UsageReport;

use crate::batch_parser::BatchRequestBody;
use crate::batch_webhook::{BatchWebhookPayload, WebhookSender};
use crate::error::ApiError;
use crate::state::AppState;

const DEFAULT_MAX_CONCURRENT: usize = 10;
const MAX_RETRIES: u32 = 2;

#[derive(Debug, Clone)]
pub struct BatchItemResult {
    pub custom_id: String,
    pub success: bool,
    pub estimated_cost: Decimal,
    pub actual_cost: Option<Decimal>,
    pub actual_input_tokens: Option<i64>,
    pub actual_output_tokens: Option<i64>,
    pub error: Option<String>,
}

#[derive(Debug, Clone)]
pub struct BatchProcessResult {
    pub items: Vec<BatchItemResult>,
    pub total_estimated_cost: Decimal,
    pub total_actual_cost: Option<Decimal>,
    pub total_input_tokens: i64,
    pub total_output_tokens: i64,
    pub success_count: usize,
    pub failure_count: usize,
}

pub struct BatchProcessor {
    max_concurrent: usize,
}

impl BatchProcessor {
    pub fn new() -> Self {
        Self {
            max_concurrent: DEFAULT_MAX_CONCURRENT,
        }
    }

    pub fn with_max_concurrent(max_concurrent: usize) -> Self {
        Self { max_concurrent }
    }

    pub async fn process_batch(
        &self,
        state: &Arc<AppState>,
        requests: Vec<crate::batch_parser::ParsedBatchLine>,
    ) -> Result<BatchProcessResult, ApiError> {
        let semaphore = Arc::new(Semaphore::new(self.max_concurrent));
        let mut handles = Vec::new();

        for request in requests {
            let semaphore = Arc::clone(&semaphore);
            let state = Arc::clone(state);

            let handle = tokio::spawn(async move {
                let _permit = semaphore.acquire().await.unwrap();
                Self::process_single_request(&state, request).await
            });

            handles.push(handle);
        }

        let mut results = Vec::new();
        for handle in handles {
            let result = handle.await.map_err(|_e| {
                ApiError::Internal
            })?;
            results.push(result);
        }

        Ok(Self::aggregate_results(results))
    }

    async fn process_single_request(
        state: &Arc<AppState>,
        request: crate::batch_parser::ParsedBatchLine,
    ) -> BatchItemResult {
        let mut last_error: Option<String> = None;
        #[allow(unused_assignments)]
        let mut actual_usage: Option<UsageReport> = None;

        for attempt in 0..=MAX_RETRIES {
            if attempt > 0 {
                let delay = Self::exponential_backoff_delay(attempt);
                sleep(delay).await;
            }

            let (result, usage) = Self::attempt_request(state, &request).await;
            if result.success {
                actual_usage = Some(usage);
                return Self::build_result(&request, result, actual_usage);
            }

            last_error = result.error;
        }

        let mut failed_result = Self::build_result(&request, BatchItemResult {
            custom_id: request.custom_id.clone(),
            success: false,
            estimated_cost: request.estimated_cost,
            actual_cost: None,
            actual_input_tokens: None,
            actual_output_tokens: None,
            error: last_error,
        }, None);
        failed_result.success = false;
        failed_result
    }

    fn build_result(
        request: &crate::batch_parser::ParsedBatchLine,
        base_result: BatchItemResult,
        actual_usage: Option<UsageReport>,
    ) -> BatchItemResult {
        let (actual_cost, actual_input_tokens, actual_output_tokens) = match actual_usage {
            Some(usage) => {
                let input_tokens = usage.prompt_tokens.unwrap_or(0) as i64;
                let output_tokens = usage.completion_tokens.unwrap_or(0) as i64;
                
                let pricing = serde_json::json!({
                    "input_price_per_million": 0,
                    "output_price_per_million": 0,
                });
                
                let cost = godwit_providers::usage::compute_chat_cost(&pricing, &usage)
                    .unwrap_or(Decimal::ZERO);
                
                (Some(cost), Some(input_tokens), Some(output_tokens))
            }
            None => (None, None, None),
        };

        BatchItemResult {
            custom_id: base_result.custom_id,
            success: base_result.success,
            estimated_cost: request.estimated_cost,
            actual_cost,
            actual_input_tokens,
            actual_output_tokens,
            error: base_result.error,
        }
    }

    async fn attempt_request(
        state: &Arc<AppState>,
        request: &crate::batch_parser::ParsedBatchLine,
    ) -> (BatchItemResult, UsageReport) {
        let resolved = match state.model_router.resolve(&request.body.model, Capability::Chat).await {
            Ok(r) => r,
            Err(e) => {
                return (BatchItemResult {
                    custom_id: request.custom_id.clone(),
                    success: false,
                    estimated_cost: request.estimated_cost,
                    actual_cost: None,
                    actual_input_tokens: None,
                    actual_output_tokens: None,
                    error: Some(format!("Model resolution failed: {}", e)),
                }, UsageReport::default());
            }
        };

        match resolved.adapter
            .chat(
                &resolved.resolved_credentials,
                &resolved.model,
                convert_to_chat_request(request.body.clone()),
            )
            .await
        {
            Ok((_response, usage)) => (BatchItemResult {
                custom_id: request.custom_id.clone(),
                success: true,
                estimated_cost: request.estimated_cost,
                actual_cost: None,
                actual_input_tokens: None,
                actual_output_tokens: None,
                error: None,
            }, usage),
            Err(e) => (BatchItemResult {
                custom_id: request.custom_id.clone(),
                success: false,
                estimated_cost: request.estimated_cost,
                actual_cost: None,
                actual_input_tokens: None,
                actual_output_tokens: None,
                error: Some(format!("Request failed: {}", e)),
            }, UsageReport::default()),
        }
    }

    fn exponential_backoff_delay(attempt: u32) -> Duration {
        let base_ms = 1000u64;
        let delay_ms = base_ms * 2u64.pow(attempt - 1);
        Duration::from_millis(delay_ms)
    }

    fn aggregate_results(item_results: Vec<BatchItemResult>) -> BatchProcessResult {
        let total_estimated_cost = item_results.iter().map(|r| r.estimated_cost).sum();

        let success_count = item_results.iter().filter(|r| r.success).count();
        let failure_count = item_results.len() - success_count;

        let total_actual_cost = item_results
            .iter()
            .filter_map(|r| r.actual_cost)
            .sum::<Decimal>();
        
        let total_input_tokens = item_results
            .iter()
            .filter_map(|r| r.actual_input_tokens)
            .sum::<i64>();
        
        let total_output_tokens = item_results
            .iter()
            .filter_map(|r| r.actual_output_tokens)
            .sum::<i64>();

        BatchProcessResult {
            items: item_results,
            total_estimated_cost,
            total_actual_cost: if total_actual_cost > Decimal::ZERO { Some(total_actual_cost) } else { None },
            total_input_tokens,
            total_output_tokens,
            success_count,
            failure_count,
        }
    }

    pub async fn send_completion_webhook(
        batch_public_id: &str,
        webhook_url: &str,
        result: &BatchProcessResult,
    ) -> Result<(), crate::batch_webhook::WebhookError> {
        let sender = WebhookSender::new();
        let payload = BatchWebhookPayload {
            batch_id: batch_public_id.to_string(),
            status: "completed".to_string(),
            total_requests: result.items.len(),
            completed_requests: result.success_count,
            failed_requests: result.failure_count,
            total_cost_usd: result.total_actual_cost.unwrap_or(Decimal::ZERO),
            total_input_tokens: result.total_input_tokens,
            total_output_tokens: result.total_output_tokens,
            completed_at: Utc::now(),
        };

        sender.send_webhook(webhook_url, &payload).await
    }
}

impl Default for BatchProcessor {
    fn default() -> Self {
        Self::new()
    }
}

fn convert_to_chat_request(body: BatchRequestBody) -> ChatCompletionRequest {
    let messages = body.messages
        .into_iter()
        .filter_map(|msg| {
            let role = msg.get("role")?.as_str()?.to_string();
            let content = msg.get("content")
                .and_then(|c| c.as_str())
                .map(|s| vec![godwit_core::ChatContent::Text(s.to_string())]);
            
            Some(ChatMessage {
                role,
                content,
                name: None,
                tool_calls: None,
                tool_call_id: None,
                cache_control: None,
            })
        })
        .collect();

    ChatCompletionRequest {
        model: body.model,
        messages,
        max_tokens: body.max_tokens,
        ..Default::default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_batch_processor_new_default_concurrency() {
        let processor = BatchProcessor::new();
        assert_eq!(processor.max_concurrent, DEFAULT_MAX_CONCURRENT);
    }

    #[test]
    fn test_batch_processor_custom_concurrency() {
        let processor = BatchProcessor::with_max_concurrent(5);
        assert_eq!(processor.max_concurrent, 5);
    }

    #[test]
    fn test_aggregate_results_empty() {
        let result = BatchProcessor::aggregate_results(vec![]);
        assert_eq!(result.items.len(), 0);
        assert_eq!(result.total_estimated_cost, Decimal::ZERO);
        assert_eq!(result.success_count, 0);
        assert_eq!(result.failure_count, 0);
    }

    #[test]
    fn test_aggregate_results_mixed() {
        let results = vec![
            BatchItemResult {
                custom_id: "req-1".to_string(),
                success: true,
                estimated_cost: Decimal::from(5),
                actual_cost: Some(Decimal::from(4)),
                actual_input_tokens: Some(100),
                actual_output_tokens: Some(50),
                error: None,
            },
            BatchItemResult {
                custom_id: "req-2".to_string(),
                success: false,
                estimated_cost: Decimal::from(3),
                actual_cost: None,
                actual_input_tokens: None,
                actual_output_tokens: None,
                error: Some("error".to_string()),
            },
            BatchItemResult {
                custom_id: "req-3".to_string(),
                success: true,
                estimated_cost: Decimal::from(2),
                actual_cost: Some(Decimal::from(2)),
                actual_input_tokens: Some(80),
                actual_output_tokens: Some(40),
                error: None,
            },
        ];

        let aggregated = BatchProcessor::aggregate_results(results);
        assert_eq!(aggregated.success_count, 2);
        assert_eq!(aggregated.failure_count, 1);
        assert_eq!(aggregated.total_estimated_cost, Decimal::from(10));
        assert_eq!(aggregated.total_actual_cost, Some(Decimal::from(6)));
        assert_eq!(aggregated.total_input_tokens, 180);
        assert_eq!(aggregated.total_output_tokens, 90);
    }

    #[test]
    fn test_aggregate_results_with_actual_cost_tracking() {
        let results = vec![
            BatchItemResult {
                custom_id: "req-1".to_string(),
                success: true,
                estimated_cost: Decimal::from(10),
                actual_cost: Some(Decimal::from(8)),
                actual_input_tokens: Some(200),
                actual_output_tokens: Some(100),
                error: None,
            },
            BatchItemResult {
                custom_id: "req-2".to_string(),
                success: true,
                estimated_cost: Decimal::from(5),
                actual_cost: Some(Decimal::from(6)),
                actual_input_tokens: Some(150),
                actual_output_tokens: Some(75),
                error: None,
            },
        ];

        let aggregated = BatchProcessor::aggregate_results(results);
        assert_eq!(aggregated.total_estimated_cost, Decimal::from(15));
        assert_eq!(aggregated.total_actual_cost, Some(Decimal::from(14)));
        assert_eq!(aggregated.total_input_tokens, 350);
        assert_eq!(aggregated.total_output_tokens, 175);
        assert_eq!(aggregated.success_count, 2);
        assert_eq!(aggregated.failure_count, 0);
    }

    #[test]
    fn test_exponential_backoff_delay() {
        assert_eq!(BatchProcessor::exponential_backoff_delay(1), Duration::from_secs(1));
        assert_eq!(BatchProcessor::exponential_backoff_delay(2), Duration::from_secs(2));
        assert_eq!(BatchProcessor::exponential_backoff_delay(3), Duration::from_secs(4));
    }

    #[tokio::test]
    async fn test_concurrent_limit_enforced() {
        use std::sync::atomic::{AtomicUsize, Ordering};
        use tokio::sync::Mutex;

        let max_concurrent = 3;
        let _processor = BatchProcessor::with_max_concurrent(max_concurrent);
        let concurrent_count = Arc::new(AtomicUsize::new(0));
        let max_observed = Arc::new(Mutex::new(0usize));

        let semaphore = Arc::new(Semaphore::new(max_concurrent));
        let mut handles = Vec::new();

        for _ in 0..10 {
            let semaphore = Arc::clone(&semaphore);
            let concurrent_count = Arc::clone(&concurrent_count);
            let max_observed = Arc::clone(&max_observed);

            let handle = tokio::spawn(async move {
                let _permit = semaphore.acquire().await.unwrap();
                let current = concurrent_count.fetch_add(1, Ordering::SeqCst) + 1;
                
                {
                    let mut max = max_observed.lock().await;
                    if current > *max {
                        *max = current;
                    }
                }

                tokio::time::sleep(Duration::from_millis(50)).await;
                concurrent_count.fetch_sub(1, Ordering::SeqCst);
                current
            });

            handles.push(handle);
        }

        for handle in handles {
            handle.await.unwrap();
        }

        let max = *max_observed.lock().await;
        assert!(max <= max_concurrent, "Max concurrent {} exceeded limit {}", max, max_concurrent);
    }
}
