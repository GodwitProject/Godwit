use reqwest::Client;
use serde_json::json;

#[tokio::test]
#[ignore = "requires running server"]
async fn test_token_counter_endpoint() {
    let client = Client::new();
    let payload = json!({
        "model": "gpt-4",
        "messages": [
            {
                "role": "user",
                "content": "Hello, world!"
            }
        ]
    });
    
    let response = client
        .post("http://localhost:3000/v1/utils/token_counter")
        .json(&payload)
        .send()
        .await
        .unwrap();
    
    assert_eq!(response.status(), 200);
    
    let body: serde_json::Value = response.json().await.unwrap();
    assert!(body.get("prompt_tokens").is_some());
    assert_eq!(body.get("model").unwrap().as_str(), Some("gpt-4"));
}

#[tokio::test]
#[ignore = "requires running server"]
async fn test_token_counter_multiple_messages() {
    let client = Client::new();
    let payload = json!({
        "model": "gpt-4",
        "messages": [
            {"role": "system", "content": "You are helpful"},
            {"role": "user", "content": "Hello"},
            {"role": "assistant", "content": "Hi there!"},
            {"role": "user", "content": "How are you?"}
        ]
    });
    
    let response = client
        .post("http://localhost:3000/v1/utils/token_counter")
        .json(&payload)
        .send()
        .await
        .unwrap();
    
    assert_eq!(response.status(), 200);
    
    let body: serde_json::Value = response.json().await.unwrap();
    let tokens = body.get("prompt_tokens").unwrap().as_u64().unwrap();
    assert!(tokens > 0);
}

#[tokio::test]
#[ignore = "requires running server"]
async fn test_token_counter_empty_messages() {
    let client = Client::new();
    let payload = json!({
        "model": "gpt-4",
        "messages": []
    });
    
    let response = client
        .post("http://localhost:3000/v1/utils/token_counter")
        .json(&payload)
        .send()
        .await
        .unwrap();
    
    assert_eq!(response.status(), 200);
    
    let body: serde_json::Value = response.json().await.unwrap();
    let tokens = body.get("prompt_tokens").unwrap().as_u64().unwrap();
    assert_eq!(tokens, 2);
}

#[tokio::test]
#[ignore = "requires running server"]
async fn test_token_counter_with_long_content() {
    let client = Client::new();
    let long_text = "Hello world. ".repeat(100);
    let payload = json!({
        "model": "gpt-4",
        "messages": [
            {"role": "user", "content": long_text}
        ]
    });
    
    let response = client
        .post("http://localhost:3000/v1/utils/token_counter")
        .json(&payload)
        .send()
        .await
        .unwrap();
    
    assert_eq!(response.status(), 200);
    
    let body: serde_json::Value = response.json().await.unwrap();
    let tokens = body.get("prompt_tokens").unwrap().as_u64().unwrap();
    assert!(tokens > 10);
}

#[tokio::test]
#[ignore = "requires running server"]
async fn test_model_info_endpoint() {
    let client = Client::new();
    
    let response = client
        .get("http://localhost:3000/v1/utils/model_info/gpt-4")
        .send()
        .await
        .unwrap();
    
    assert_eq!(response.status(), 200);
    
    let body: serde_json::Value = response.json().await.unwrap();
    assert!(body.get("id").is_some());
    assert!(body.get("provider").is_some());
    assert!(body.get("pricing").is_some());
    assert!(body.get("capabilities").is_some());
}

#[tokio::test]
#[ignore = "requires running server"]
async fn test_model_info_has_pricing_details() {
    let client = Client::new();
    
    let response = client
        .get("http://localhost:3000/v1/utils/model_info/gpt-4o")
        .send()
        .await
        .unwrap();
    
    assert_eq!(response.status(), 200);
    
    let body: serde_json::Value = response.json().await.unwrap();
    let pricing = body.get("pricing").unwrap();
    assert!(pricing.get("input_cost_per_1k").is_some());
    assert!(pricing.get("output_cost_per_1k").is_some());
}

#[tokio::test]
#[ignore = "requires running server"]
async fn test_model_info_has_capabilities() {
    let client = Client::new();
    
    let response = client
        .get("http://localhost:3000/v1/utils/model_info/gpt-4o")
        .send()
        .await
        .unwrap();
    
    assert_eq!(response.status(), 200);
    
    let body: serde_json::Value = response.json().await.unwrap();
    let capabilities = body.get("capabilities").unwrap();
    assert!(capabilities.get("supports_tool_calling").is_some());
    assert!(capabilities.get("supports_vision").is_some());
    assert!(capabilities.get("supports_streaming").is_some());
    assert!(capabilities.get("max_tokens").is_some());
}

#[tokio::test]
#[ignore = "requires running server"]
async fn test_model_info_nonexistent_model() {
    let client = Client::new();
    
    let response = client
        .get("http://localhost:3000/v1/utils/model_info/nonexistent-model-xyz")
        .send()
        .await
        .unwrap();
    
    assert_eq!(response.status(), 404);
}

#[tokio::test]
#[ignore = "requires running server"]
async fn test_health_endpoint() {
    let client = Client::new();
    
    let response = client
        .get("http://localhost:3000/v1/utils/health")
        .send()
        .await
        .unwrap();
    
    assert_eq!(response.status(), 200);
    
    let body: serde_json::Value = response.json().await.unwrap();
    assert_eq!(body.get("status").unwrap().as_str(), Some("healthy"));
    assert!(body.get("version").is_some());
    assert!(body.get("uptime_secs").is_some());
    assert_eq!(body.get("database").unwrap().as_str(), Some("connected"));
    assert!(body.get("providers").is_some());
}

#[tokio::test]
#[ignore = "requires running server"]
async fn test_health_endpoint_version_format() {
    let client = Client::new();
    
    let response = client
        .get("http://localhost:3000/v1/utils/health")
        .send()
        .await
        .unwrap();
    
    assert_eq!(response.status(), 200);
    
    let body: serde_json::Value = response.json().await.unwrap();
    let version = body.get("version").unwrap().as_str().unwrap();
    assert!(!version.is_empty());
}

#[tokio::test]
#[ignore = "requires running server"]
async fn test_health_endpoint_uptime_positive() {
    let client = Client::new();
    
    let response = client
        .get("http://localhost:3000/v1/utils/health")
        .send()
        .await
        .unwrap();
    
    assert_eq!(response.status(), 200);
    
    let body: serde_json::Value = response.json().await.unwrap();
    let uptime = body.get("uptime_secs").unwrap().as_u64().unwrap();
    assert!(uptime > 0);
}

#[tokio::test]
#[ignore = "requires running server"]
async fn test_health_database_connected() {
    let client = Client::new();
    
    let response = client
        .get("http://localhost:3000/v1/utils/health")
        .send()
        .await
        .unwrap();
    
    assert_eq!(response.status(), 200);
    
    let body: serde_json::Value = response.json().await.unwrap();
    assert_eq!(body.get("database").unwrap().as_str(), Some("connected"));
}
