use reqwest::Client;
use serde_json::json;

#[tokio::test]
#[ignore = "requires running server and DB"]
async fn test_fallback_chain() {
    let client = Client::new();
    
    // Setup: Configure model with fallback chain
    // Mock: Primary provider returns 503
    // Assert: Fallback to secondary provider succeeds
    // Assert: request_logs shows fallback_triggered = true
    
    let resp = client
        .post("http://localhost:3000/v1/chat/completions")
        .header("Authorization", "Bearer sk-godwit-test")
        .json(&json!({
            "model": "gpt-4o",
            "messages": [{
                "role": "user",
                "content": "test fallback chain"
            }]
        }))
        .send()
        .await
        .expect("request failed");
    
    // Should succeed via fallback
    assert!(resp.status().is_success());
    
    let body: serde_json::Value = resp.json().await.expect("invalid JSON");
    assert!(body.get("choices").is_some());
    
    // Verify request_logs shows fallback_triggered = true
    // This would require DB access in a real integration test
}

#[tokio::test]
#[ignore = "requires running server and DB"]
async fn test_fallback_exhausted() {
    let client = Client::new();
    
    // Setup: Configure model with 2 fallbacks
    // Mock: All providers return 503
    // Assert: Last error returned
    // Assert: request_logs shows 3 attempts
    
    let resp = client
        .post("http://localhost:3000/v1/chat/completions")
        .header("Authorization", "Bearer sk-godwit-test")
        .json(&json!({
            "model": "gpt-4o",
            "messages": [{
                "role": "user",
                "content": "test fallback exhaustion"
            }]
        }))
        .send()
        .await
        .expect("request failed");
    
    // Should fail after all fallbacks exhausted
    assert_eq!(resp.status().as_u16(), 503);
    
    // Verify request_logs shows 3 attempts
    // This would require DB access in a real integration test
}
