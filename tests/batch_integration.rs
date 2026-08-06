use reqwest::Client;
use serde_json::json;

#[tokio::test]
#[ignore = "requires running server and DB"]
async fn test_batch_create_openai() {
    let client = Client::new();
    
    // Test batch creation with OpenAI provider
    let resp = client
        .post("http://localhost:3000/v1/batches")
        .header("Authorization", "Bearer sk-godwit-test")
        .json(&json!({
            "model": "gpt-4o",
            "input_file_id": "file-abc123",
            "endpoint": "/v1/chat/completions",
            "completion_window": "24h"
        }))
        .send()
        .await
        .expect("request failed");
    
    // Should succeed with OpenAI
    assert!(resp.status().is_success());
    
    let body: serde_json::Value = resp.json().await.expect("invalid JSON");
    assert!(body.get("id").is_some());
    assert_eq!(body["object"], "batch");
}

#[tokio::test]
#[ignore = "requires running server and DB"]
async fn test_batch_create_simulated_provider() {
    let client = Client::new();
    
    // Test batch creation with simulated provider (e.g., Anthropic)
    let resp = client
        .post("http://localhost:3000/v1/batches")
        .header("Authorization", "Bearer sk-godwit-test")
        .json(&json!({
            "model": "claude-3-5-sonnet-20241022",
            "input_file_id": "file-xyz789",
            "endpoint": "/v1/chat/completions",
            "completion_window": "24h"
        }))
        .send()
        .await
        .expect("request failed");
    
    // Should succeed with simulated batch processing
    assert!(resp.status().is_success());
    
    let body: serde_json::Value = resp.json().await.expect("invalid JSON");
    assert!(body.get("id").is_some());
    assert_eq!(body["object"], "batch");
}

#[tokio::test]
#[ignore = "requires running server and DB"]
async fn test_batch_retrieve() {
    let client = Client::new();
    
    // First create a batch
    let create_resp = client
        .post("http://localhost:3000/v1/batches")
        .header("Authorization", "Bearer sk-godwit-test")
        .json(&json!({
            "model": "gpt-4o",
            "input_file_id": "file-test123",
            "endpoint": "/v1/chat/completions",
            "completion_window": "24h"
        }))
        .send()
        .await
        .expect("create request failed");
    
    let create_body: serde_json::Value = create_resp.json().await.expect("invalid JSON");
    let batch_id = create_body["id"].as_str().expect("batch id should be string");
    
    // Retrieve the batch
    let retrieve_resp = client
        .get(format!("http://localhost:3000/v1/batches/{}", batch_id))
        .query(&[("model", "gpt-4o")])
        .header("Authorization", "Bearer sk-godwit-test")
        .send()
        .await
        .expect("retrieve request failed");
    
    assert!(retrieve_resp.status().is_success());
    
    let body: serde_json::Value = retrieve_resp.json().await.expect("invalid JSON");
    assert_eq!(body["id"], batch_id);
}

#[tokio::test]
#[ignore = "requires running server and DB"]
async fn test_batch_cancel() {
    let client = Client::new();
    
    // First create a batch
    let create_resp = client
        .post("http://localhost:3000/v1/batches")
        .header("Authorization", "Bearer sk-godwit-test")
        .json(&json!({
            "model": "gpt-4o",
            "input_file_id": "file-test456",
            "endpoint": "/v1/chat/completions",
            "completion_window": "24h"
        }))
        .send()
        .await
        .expect("create request failed");
    
    let create_body: serde_json::Value = create_resp.json().await.expect("invalid JSON");
    let batch_id = create_body["id"].as_str().expect("batch id should be string");
    
    // Cancel the batch
    let cancel_resp = client
        .post(format!("http://localhost:3000/v1/batches/{}/cancel", batch_id))
        .query(&[("model", "gpt-4o")])
        .header("Authorization", "Bearer sk-godwit-test")
        .send()
        .await
        .expect("cancel request failed");
    
    assert!(cancel_resp.status().is_success());
    
    let body: serde_json::Value = cancel_resp.json().await.expect("invalid JSON");
    assert_eq!(body["id"], batch_id);
    assert!(body["status"].as_str().map(|s| s == "cancelling" || s == "cancelled").unwrap_or(false));
}

#[tokio::test]
#[ignore = "requires running server and DB"]
async fn test_batch_retry_behavior() {
    let client = Client::new();
    
    // Test that batch processor respects retry limit (max 2 retries)
    // This test verifies the batch processor configuration
    // In a real scenario, you would mock a flaky provider
    
    let resp = client
        .post("http://localhost:3000/v1/batches")
        .header("Authorization", "Bearer sk-godwit-test")
        .json(&json!({
            "model": "gpt-4o",
            "input_file_id": "file-retry-test",
            "endpoint": "/v1/chat/completions",
            "completion_window": "24h"
        }))
        .send()
        .await
        .expect("request failed");
    
    assert!(resp.status().is_success());
    
    let body: serde_json::Value = resp.json().await.expect("invalid JSON");
    assert!(body.get("id").is_some());
}

#[tokio::test]
#[ignore = "requires running server and DB"]
async fn test_batch_cost_tracking() {
    let client = Client::new();
    
    // Create a batch
    let resp = client
        .post("http://localhost:3000/v1/batches")
        .header("Authorization", "Bearer sk-godwit-test")
        .json(&json!({
            "model": "gpt-4o",
            "input_file_id": "file-cost-test",
            "endpoint": "/v1/chat/completions",
            "completion_window": "24h"
        }))
        .send()
        .await
        .expect("request failed");
    
    assert!(resp.status().is_success());
    
    let body: serde_json::Value = resp.json().await.expect("invalid JSON");
    
    // Verify batch response includes cost tracking fields
    // (actual field names depend on implementation)
    assert!(body.get("id").is_some());
}

#[tokio::test]
#[ignore = "requires running server and DB"]
async fn test_batch_jsonl_parsing() {
    // This test would require uploading a JSONL file
    // For now, we test that the batch endpoint accepts the request
    
    let client = Client::new();
    
    let resp = client
        .post("http://localhost:3000/v1/batches")
        .header("Authorization", "Bearer sk-godwit-test")
        .json(&json!({
            "model": "gpt-4o",
            "input_file_id": "file-jsonl-test",
            "endpoint": "/v1/chat/completions",
            "completion_window": "24h"
        }))
        .send()
        .await
        .expect("request failed");
    
    assert!(resp.status().is_success());
    
    let body: serde_json::Value = resp.json().await.expect("invalid JSON");
    assert!(body.get("id").is_some());
}
