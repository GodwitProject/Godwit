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
