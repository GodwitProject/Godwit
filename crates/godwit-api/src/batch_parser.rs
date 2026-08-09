use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use godwit_core::Capability;
use godwit_providers::adapter::UsageReport;

#[derive(Debug, Error)]
pub enum BatchParseError {
    #[error("invalid JSONL format: {0}")]
    InvalidFormat(String),
    #[error("missing required field: {0}")]
    MissingField(String),
    #[error("invalid model reference: {0}")]
    InvalidModel(String),
    #[error("cost estimation failed: {0}")]
    CostEstimationFailed(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchRequest {
    pub custom_id: String,
    pub method: String,
    pub url: String,
    pub body: BatchRequestBody,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchRequestBody {
    pub model: String,
    #[serde(default)]
    pub messages: Vec<serde_json::Value>,
    #[serde(default)]
    pub max_tokens: Option<i32>,
}

#[derive(Debug, Clone)]
pub struct ParsedBatchLine {
    pub custom_id: String,
    pub method: String,
    pub url: String,
    pub body: BatchRequestBody,
    pub estimated_cost: Decimal,
}

#[derive(Debug, Clone)]
pub struct BatchParseResult {
    pub lines: Vec<ParsedBatchLine>,
    pub total_estimated_cost: Decimal,
}

pub fn parse_jsonl(jsonl_content: &str) -> Result<Vec<BatchRequest>, BatchParseError> {
    let mut requests = Vec::new();
    
    for (line_num, line) in jsonl_content.lines().enumerate() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        
        let request: BatchRequest = serde_json::from_str(trimmed).map_err(|e| {
            BatchParseError::InvalidFormat(format!("line {}: invalid JSON: {}", line_num + 1, e))
        })?;
        
        validate_request(&request)?;
        requests.push(request);
    }
    
    Ok(requests)
}

pub fn validate_request(request: &BatchRequest) -> Result<(), BatchParseError> {
    if request.custom_id.is_empty() {
        return Err(BatchParseError::MissingField("custom_id".to_string()));
    }
    
    if request.method != "POST" {
        return Err(BatchParseError::InvalidFormat(format!(
            "method must be POST, got: {}",
            request.method
        )));
    }
    
    if !request.url.starts_with('/') {
        return Err(BatchParseError::InvalidFormat(format!(
            "url must start with '/', got: {}",
            request.url
        )));
    }
    
    if request.body.model.is_empty() {
        return Err(BatchParseError::MissingField("model".to_string()));
    }
    
    Ok(())
}

pub fn estimate_cost(
    requests: &[BatchRequest],
    pricing_catalog: &std::collections::HashMap<String, serde_json::Value>,
) -> Result<BatchParseResult, BatchParseError> {
    let mut lines = Vec::new();
    let mut total_cost = Decimal::ZERO;
    
    for request in requests {
        let pricing = pricing_catalog.get(&request.body.model).ok_or_else(|| {
            BatchParseError::InvalidModel(format!(
                "model '{}' not found in pricing catalog",
                request.body.model
            ))
        })?;
        
        let usage = estimate_usage_from_request(request);
        let cost = godwit_providers::usage::compute_cost(pricing, Capability::Chat, &usage)
            .ok_or_else(|| {
                BatchParseError::CostEstimationFailed(format!(
                    "could not compute cost for model '{}'",
                    request.body.model
                ))
            })?;
        
        total_cost += cost;
        
        lines.push(ParsedBatchLine {
            custom_id: request.custom_id.clone(),
            method: request.method.clone(),
            url: request.url.clone(),
            body: request.body.clone(),
            estimated_cost: cost,
        });
    }
    
    Ok(BatchParseResult {
        lines,
        total_estimated_cost: total_cost,
    })
}

fn estimate_usage_from_request(request: &BatchRequest) -> UsageReport {
    let mut prompt_tokens = 0i32;
    #[allow(unused_assignments)]
    let mut max_completion_tokens = 0i32;
    
    for message in &request.body.messages {
        if let Some(content) = message.get("content") {
            if let Some(text) = content.as_str() {
                prompt_tokens += estimate_tokens_from_text(text);
            } else if let Some(parts) = content.as_array() {
                for part in parts {
                    if let Some(text) = part.get("text").and_then(|t| t.as_str()) {
                        prompt_tokens += estimate_tokens_from_text(text);
                    }
                }
            }
        }
    }
    
    if let Some(max_tokens) = request.body.max_tokens {
        max_completion_tokens = max_tokens;
    } else {
        max_completion_tokens = (prompt_tokens as f64 * 0.5) as i32;
    }
    
    UsageReport {
        prompt_tokens: Some(prompt_tokens),
        completion_tokens: Some(max_completion_tokens),
        ..Default::default()
    }
}

fn estimate_tokens_from_text(text: &str) -> i32 {
    let char_count = text.len() as i32;
    (char_count / 4) as i32
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn sample_pricing_catalog() -> HashMap<String, serde_json::Value> {
        let mut catalog = HashMap::new();
        catalog.insert(
            "gpt-4o".to_string(),
            serde_json::json!({
                "input_price_per_million": 2.5,
                "output_price_per_million": 10.0,
            }),
        );
        catalog.insert(
            "gpt-4o-mini".to_string(),
            serde_json::json!({
                "input_price_per_million": 0.15,
                "output_price_per_million": 0.6,
            }),
        );
        catalog
    }

    #[test]
    fn test_parse_valid_jsonl() {
        let jsonl = r#"{"custom_id":"req-1","method":"POST","url":"/v1/chat/completions","body":{"model":"gpt-4o","messages":[{"role":"user","content":"Hello"}],"max_tokens":100}}"#;
        
        let result = parse_jsonl(jsonl);
        assert!(result.is_ok());
        let requests = result.unwrap();
        assert_eq!(requests.len(), 1);
        assert_eq!(requests[0].custom_id, "req-1");
        assert_eq!(requests[0].method, "POST");
        assert_eq!(requests[0].url, "/v1/chat/completions");
        assert_eq!(requests[0].body.model, "gpt-4o");
    }

    #[test]
    fn test_parse_multiple_lines() {
        let jsonl = r#"{"custom_id":"req-1","method":"POST","url":"/v1/chat/completions","body":{"model":"gpt-4o","messages":[{"role":"user","content":"Hello"}],"max_tokens":100}}
{"custom_id":"req-2","method":"POST","url":"/v1/chat/completions","body":{"model":"gpt-4o-mini","messages":[{"role":"user","content":"World"}],"max_tokens":50}}"#;
        
        let result = parse_jsonl(jsonl);
        assert!(result.is_ok());
        let requests = result.unwrap();
        assert_eq!(requests.len(), 2);
        assert_eq!(requests[0].custom_id, "req-1");
        assert_eq!(requests[1].custom_id, "req-2");
    }

    #[test]
    fn test_parse_invalid_jsonl_rejected() {
        let jsonl = r#"{"custom_id":"req-1","method":"POST","url":"/v1/chat/completions","body":{"model":"gpt-4o","messages":[{"role":"user","content":"Hello"}],"max_tokens":100}
{"custom_id":"req-2","method":"POST","url":"/v1/chat/completions","body":{"model":"gpt-4o","messages":[{"role":"user","content":"World"}],"max_tokens":50}}"#;
        
        let result = parse_jsonl(jsonl);
        assert!(result.is_err());
        match result.unwrap_err() {
            BatchParseError::InvalidFormat(msg) => assert!(msg.contains("invalid JSON")),
            _ => panic!("expected InvalidFormat error"),
        }
    }

    #[test]
    fn test_validate_request_missing_custom_id() {
        let request = BatchRequest {
            custom_id: "".to_string(),
            method: "POST".to_string(),
            url: "/v1/chat/completions".to_string(),
            body: BatchRequestBody {
                model: "gpt-4o".to_string(),
                messages: vec![],
                max_tokens: None,
            },
        };
        
        let result = validate_request(&request);
        assert!(result.is_err());
        match result.unwrap_err() {
            BatchParseError::MissingField(field) => assert_eq!(field, "custom_id"),
            _ => panic!("expected MissingField error"),
        }
    }

    #[test]
    fn test_validate_request_wrong_method() {
        let request = BatchRequest {
            custom_id: "req-1".to_string(),
            method: "GET".to_string(),
            url: "/v1/chat/completions".to_string(),
            body: BatchRequestBody {
                model: "gpt-4o".to_string(),
                messages: vec![],
                max_tokens: None,
            },
        };
        
        let result = validate_request(&request);
        assert!(result.is_err());
        match result.unwrap_err() {
            BatchParseError::InvalidFormat(msg) => assert!(msg.contains("method must be POST")),
            _ => panic!("expected InvalidFormat error"),
        }
    }

    #[test]
    fn test_validate_request_invalid_url() {
        let request = BatchRequest {
            custom_id: "req-1".to_string(),
            method: "POST".to_string(),
            url: "v1/chat/completions".to_string(),
            body: BatchRequestBody {
                model: "gpt-4o".to_string(),
                messages: vec![],
                max_tokens: None,
            },
        };
        
        let result = validate_request(&request);
        assert!(result.is_err());
        match result.unwrap_err() {
            BatchParseError::InvalidFormat(msg) => assert!(msg.contains("url must start with")),
            _ => panic!("expected InvalidFormat error"),
        }
    }

    #[test]
    fn test_validate_request_missing_model() {
        let request = BatchRequest {
            custom_id: "req-1".to_string(),
            method: "POST".to_string(),
            url: "/v1/chat/completions".to_string(),
            body: BatchRequestBody {
                model: "".to_string(),
                messages: vec![],
                max_tokens: None,
            },
        };
        
        let result = validate_request(&request);
        assert!(result.is_err());
        match result.unwrap_err() {
            BatchParseError::MissingField(field) => assert_eq!(field, "model"),
            _ => panic!("expected MissingField error"),
        }
    }

    #[test]
    fn test_cost_estimation_correct() {
        let jsonl = r#"{"custom_id":"req-1","method":"POST","url":"/v1/chat/completions","body":{"model":"gpt-4o","messages":[{"role":"user","content":"Hello, how are you?"}],"max_tokens":100}}"#;
        
        let requests = parse_jsonl(jsonl).unwrap();
        let pricing_catalog = sample_pricing_catalog();
        let result = estimate_cost(&requests, &pricing_catalog).unwrap();
        
        assert_eq!(result.lines.len(), 1);
        assert!(result.total_estimated_cost > Decimal::ZERO);
        
        let line = &result.lines[0];
        assert_eq!(line.custom_id, "req-1");
        assert!(line.estimated_cost > Decimal::ZERO);
    }

    #[test]
    fn test_cost_estimation_multiple_lines() {
        let jsonl = r#"{"custom_id":"req-1","method":"POST","url":"/v1/chat/completions","body":{"model":"gpt-4o","messages":[{"role":"user","content":"Hello"}],"max_tokens":100}}
{"custom_id":"req-2","method":"POST","url":"/v1/chat/completions","body":{"model":"gpt-4o-mini","messages":[{"role":"user","content":"World"}],"max_tokens":50}}"#;
        
        let requests = parse_jsonl(jsonl).unwrap();
        let pricing_catalog = sample_pricing_catalog();
        let result = estimate_cost(&requests, &pricing_catalog).unwrap();
        
        assert_eq!(result.lines.len(), 2);
        
        let total = result.total_estimated_cost;
        let line1_cost = result.lines[0].estimated_cost;
        let line2_cost = result.lines[1].estimated_cost;
        
        assert_eq!(total, line1_cost + line2_cost);
    }

    #[test]
    fn test_cost_estimation_unknown_model() {
        let jsonl = r#"{"custom_id":"req-1","method":"POST","url":"/v1/chat/completions","body":{"model":"unknown-model","messages":[{"role":"user","content":"Hello"}],"max_tokens":100}}"#;
        
        let requests = parse_jsonl(jsonl).unwrap();
        let pricing_catalog = sample_pricing_catalog();
        let result = estimate_cost(&requests, &pricing_catalog);
        
        assert!(result.is_err());
        match result.unwrap_err() {
            BatchParseError::InvalidModel(msg) => assert!(msg.contains("not found in pricing catalog")),
            _ => panic!("expected InvalidModel error"),
        }
    }

    #[test]
    fn test_estimate_tokens_from_text() {
        assert_eq!(estimate_tokens_from_text("Hello"), 1);
        assert_eq!(estimate_tokens_from_text("Hello, how are you?"), 4);
        assert_eq!(estimate_tokens_from_text(""), 0);
    }

    #[test]
    fn test_empty_lines_skipped() {
        let jsonl = r#"{"custom_id":"req-1","method":"POST","url":"/v1/chat/completions","body":{"model":"gpt-4o","messages":[{"role":"user","content":"Hello"}],"max_tokens":100}}

{"custom_id":"req-2","method":"POST","url":"/v1/chat/completions","body":{"model":"gpt-4o","messages":[{"role":"user","content":"World"}],"max_tokens":50}}
"#;
        
        let result = parse_jsonl(jsonl);
        assert!(result.is_ok());
        let requests = result.unwrap();
        assert_eq!(requests.len(), 2);
    }
}
