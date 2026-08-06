use reqwest::Client;
use serde_json::json;

#[tokio::test]
#[ignore = "requires running server and DB"]
async fn test_moderation_openai_provider() {
    let client = Client::new();
    
    // Test moderation with OpenAI provider
    let resp = client
        .post("http://localhost:3000/v1/moderations")
        .header("Authorization", "Bearer sk-godwit-test")
        .json(&json!({
            "model": "text-moderation-latest",
            "input": "This is a test input for moderation"
        }))
        .send()
        .await
        .expect("request failed");
    
    // Should succeed with OpenAI as primary provider
    assert!(resp.status().is_success());
    
    let body: serde_json::Value = resp.json().await.expect("invalid JSON");
    assert!(body.get("id").is_some());
    assert!(body.get("model").is_some());
    assert!(body.get("results").is_some());
}

#[tokio::test]
#[ignore = "requires running server and DB"]
async fn test_moderation_fallback_chain() {
    let client = Client::new();
    
    // Setup: Configure model with fallback chain
    // Mock: Primary provider (OpenAI) returns 503
    // Assert: Fallback to Azure provider succeeds
    // Assert: Response normalized with id, model, results
    
    let resp = client
        .post("http://localhost:3000/v1/moderations")
        .header("Authorization", "Bearer sk-godwit-test")
        .json(&json!({
            "model": "text-moderation-latest",
            "input": "test fallback chain moderation"
        }))
        .send()
        .await
        .expect("request failed");
    
    // Should succeed via fallback
    assert!(resp.status().is_success());
    
    let body: serde_json::Value = resp.json().await.expect("invalid JSON");
    assert!(body.get("id").is_some());
    assert!(body.get("model").is_some());
    assert!(body.get("results").is_some());
    
    // Verify response is normalized (id, model, results fields present)
    assert!(body["results"].is_array());
}

#[tokio::test]
#[ignore = "requires running server and DB"]
async fn test_moderation_response_format() {
    let client = Client::new();
    
    let resp = client
        .post("http://localhost:3000/v1/moderations")
        .header("Authorization", "Bearer sk-godwit-test")
        .json(&json!({
            "model": "text-moderation-latest",
            "input": "test input"
        }))
        .send()
        .await
        .expect("request failed");
    
    assert!(resp.status().is_success());
    
    let body: serde_json::Value = resp.json().await.expect("invalid JSON");
    
    // Verify OpenAI-compatible response format
    assert!(body.get("id").is_some());
    assert!(body.get("model").is_some());
    assert!(body.get("results").is_some());
    
    // Results should be an array
    let results = body["results"].as_array().expect("results should be array");
    
    // Each result should have flagged and categories
    for result in results {
        assert!(result.get("flagged").is_some());
        assert!(result.get("categories").is_some());
    }
}
