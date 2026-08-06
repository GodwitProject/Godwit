# Task 6: Batch Simulated Processor Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Create an async batch processor for simulated batches (Anthropic, Gemini, llama.cpp, Ollama, vLLM, SGLang) with semaphore-based concurrency limiting.

**Architecture:** The BatchProcessor will manage async execution of batch requests with a configurable semaphore (max 10 concurrent by default). It will iterate through parsed batch lines, execute requests through the appropriate provider adapter, and collect results.

**Tech Stack:** Rust, tokio (async runtime), tokio::sync::Semaphore for concurrency control, rust_decimal for cost calculations.

## Global Constraints

- Batch simulated : Godwit gère la boucle async, pas le provider
- Providers : Anthropic, Gemini, llama.cpp, Ollama, vLLM, SGLang
- Concurrent limit : max 10 requests parallèles (configurable)
- Use Decimal for all cost calculations (no floats)
- Follow existing code patterns in godwit-api/src/

---

### Task 1: Create batch_processor.rs module

**Files:**
- Create: `crates/godwit-api/src/batch_processor.rs`
- Modify: `crates/godwit-api/src/lib.rs` (add module declaration)
- Test: `crates/godwit-api/src/batch_processor.rs` (inline tests)

**Interfaces:**
- Consumes: `ParsedBatchLine` from `batch_parser.rs`, `AdapterRegistry` from `godwit-providers`
- Produces: `BatchProcessor` struct with `process_batch` async method

- [ ] **Step 1: Write the module structure with tests**

```rust
use rust_decimal::Decimal;
use std::sync::Arc;
use tokio::sync::Semaphore;

use godwit_core::Capability;
use godwit_db::models::Model;
use godwit_providers::{adapter::{Adapter, ProviderError, ProviderResponse, UsageReport}, AdapterRegistry};

use crate::batch_parser::ParsedBatchLine;
use crate::error::ApiError;
use crate::state::AppState;

/// Maximum concurrent requests allowed in a simulated batch
const DEFAULT_MAX_CONCURRENT: usize = 10;

/// Result of processing a single batch request
#[derive(Debug, Clone)]
pub struct BatchItemResult {
    pub custom_id: String,
    pub success: bool,
    pub response: Option<ProviderResponse>,
    pub usage: Option<UsageReport>,
    pub estimated_cost: Decimal,
    pub error: Option<String>,
}

/// Result of processing an entire batch
#[derive(Debug, Clone)]
pub struct BatchProcessResult {
    pub items: Vec<BatchItemResult>,
    pub total_estimated_cost: Decimal,
    pub success_count: usize,
    pub failure_count: usize,
}

/// Async batch processor for simulated batches (Anthropic, Gemini, llama.cpp, Ollama, vLLM, SGLang)
/// 
/// Manages concurrent execution of batch requests with a configurable semaphore limit.
pub struct BatchProcessor {
    max_concurrent: usize,
}

impl BatchProcessor {
    /// Create a new batch processor with default concurrency limit (10)
    pub fn new() -> Self {
        Self {
            max_concurrent: DEFAULT_MAX_CONCURRENT,
        }
    }

    /// Create a new batch processor with custom concurrency limit
    pub fn with_max_concurrent(max_concurrent: usize) -> Self {
        Self { max_concurrent }
    }

    /// Process all requests in the batch with concurrency limiting
    /// 
    /// Uses a semaphore to limit concurrent requests to max_concurrent.
    /// Each request is processed through the appropriate provider adapter.
    pub async fn process_batch(
        &self,
        state: &Arc<AppState>,
        requests: Vec<ParsedBatchLine>,
    ) -> Result<BatchProcessResult, ApiError> {
        let semaphore = Arc::new(Semaphore::new(self.max_concurrent));
        let mut handles = Vec::new();

        // Spawn tasks for each request with semaphore permits
        for request in requests {
            let semaphore = Arc::clone(&semaphore);
            let state = Arc::clone(state);
            
            let handle = tokio::spawn(async move {
                // Acquire permit (limits concurrency)
                let _permit = semaphore.acquire().await.unwrap();
                
                // Process individual request
                Self::process_single_request(&state, request).await
            });
            
            handles.push(handle);
        }

        // Collect all results
        let mut results = Vec::new();
        for handle in handles {
            let result = handle.await.map_err(|e| {
                ApiError::Internal(format!("Task join error: {}", e))
            })?;
            results.push(result);
        }

        Ok(Self::aggregate_results(results))
    }

    /// Process a single batch request
    async fn process_single_request(
        state: &Arc<AppState>,
        request: ParsedBatchLine,
    ) -> BatchItemResult {
        // Resolve model and adapter
        let model = match state.model_router.resolve_model(&request.body.model).await {
            Ok(m) => m,
            Err(e) => {
                return BatchItemResult {
                    custom_id: request.custom_id,
                    success: false,
                    response: None,
                    usage: None,
                    estimated_cost: request.estimated_cost,
                    error: Some(format!("Model resolution failed: {}", e)),
                };
            }
        };

        // Get adapter for the model's provider
        let adapter = match state.adapter_registry.get_adapter(&model.provider) {
            Some(a) => a,
            None => {
                return BatchItemResult {
                    custom_id: request.custom_id,
                    success: false,
                    response: None,
                    usage: None,
                    estimated_cost: request.estimated_cost,
                    error: Some(format!("No adapter found for provider: {}", model.provider)),
                };
            }
        };

        // Resolve profile (API key, base URL)
        let profile = match state.model_router.resolve_profile(&model).await {
            Ok(p) => p,
            Err(e) => {
                return BatchItemResult {
                    custom_id: request.custom_id,
                    success: false,
                    response: None,
                    usage: None,
                    estimated_cost: request.estimated_cost,
                    error: Some(format!("Profile resolution failed: {}", e)),
                };
            }
        };

        // Execute the request through the adapter
        // For simulated batches, we use the chat capability
        match adapter.chat(&profile, &model, convert_to_chat_request(request.body)).await {
            Ok((response, usage)) => BatchItemResult {
                custom_id: request.custom_id,
                success: true,
                response: Some(response),
                usage: Some(usage),
                estimated_cost: request.estimated_cost,
                error: None,
            },
            Err(e) => BatchItemResult {
                custom_id: request.custom_id,
                success: false,
                response: None,
                usage: None,
                estimated_cost: request.estimated_cost,
                error: Some(format!("Request failed: {}", e)),
            },
        }
    }

    /// Aggregate individual results into a batch result
    fn aggregate_results(item_results: Vec<BatchItemResult>) -> BatchProcessResult {
        let total_estimated_cost = item_results.iter()
            .map(|r| r.estimated_cost)
            .sum();
        
        let success_count = item_results.iter().filter(|r| r.success).count();
        let failure_count = item_results.len() - success_count;

        BatchProcessResult {
            items: item_results,
            total_estimated_cost,
            success_count,
            failure_count,
        }
    }
}

/// Convert BatchRequestBody to ChatCompletionRequest
fn convert_to_chat_request(body: crate::batch_parser::BatchRequestBody) -> godwit_core::ChatCompletionRequest {
    godwit_core::ChatCompletionRequest {
        model: body.model,
        messages: body.messages,
        max_tokens: body.max_tokens.map(|t| t as i64),
        ..Default::default()
    }
}

impl Default for BatchProcessor {
    fn default() -> Self {
        Self::new()
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
                response: None,
                usage: None,
                estimated_cost: Decimal::from(5),
                error: None,
            },
            BatchItemResult {
                custom_id: "req-2".to_string(),
                success: false,
                response: None,
                usage: None,
                estimated_cost: Decimal::from(3),
                error: Some("error".to_string()),
            },
            BatchItemResult {
                custom_id: "req-3".to_string(),
                success: true,
                response: None,
                usage: None,
                estimated_cost: Decimal::from(2),
                error: None,
            },
        ];

        let aggregated = BatchProcessor::aggregate_results(results);
        assert_eq!(aggregated.success_count, 2);
        assert_eq!(aggregated.failure_count, 1);
        assert_eq!(aggregated.total_estimated_cost, Decimal::from(10));
    }
}
```

- [ ] **Step 2: Add module to lib.rs**

Add `pub mod batch_processor;` to `crates/godwit-api/src/lib.rs`

- [ ] **Step 3: Run tests to verify module compiles and tests pass**

Run: `cargo test -p godwit-api batch_processor --no-fail-fast`
Expected: All tests pass

- [ ] **Step 4: Commit**

```bash
git add crates/godwit-api/src/batch_processor.rs crates/godwit-api/src/lib.rs
git commit -m "feat: add simulated batch processor for Anthropic/Gemini/etc."
```

---

## Verification Commands

After implementation:
```bash
export PATH="/usr/local/opt/rustup/bin:$PATH"
cargo check -p godwit-api
cargo test -p godwit-api batch_processor --no-fail-fast
```
