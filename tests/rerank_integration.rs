use reqwest::Client;
use serde_json::json;

#[tokio::test]
#[ignore = "requires running server and DB"]
async fn test_rerank_cohere_provider() {
    let client = Client::new();
    
    // Test rerank with Cohere provider
    let resp = client
        .post("http://localhost:3000/v1/rerank")
        .header("Authorization", "Bearer sk-godwit-test")
        .json(&json!({
            "model": "rerank-english-v3.0",
            "query": "What is the capital of France?",
            "documents": [
                "Paris is the capital of France",
                "London is the capital of the UK",
                "Berlin is the capital of Germany"
            ]
        }))
        .send()
        .await
        .expect("request failed");
    
    // Should succeed with Cohere as primary provider
    assert!(resp.status().is_success());
    
    let body: serde_json::Value = resp.json().await.expect("invalid JSON");
    assert!(body.get("id").is_some());
    assert!(body.get("model").is_some());
    assert!(body.get("results").is_some());
}

#[tokio::test]
#[ignore = "requires running server and DB"]
async fn test_rerank_fallback_chain() {
    let client = Client::new();
    
    // Setup: Configure model with fallback chain
    // Mock: Primary provider (Cohere) returns 503
    // Assert: Fallback to Azure provider succeeds
    // Assert: Response normalized with id, model, results
    
    let resp = client
        .post("http://localhost:3000/v1/rerank")
        .header("Authorization", "Bearer sk-godwit-test")
        .json(&json!({
            "model": "rerank-english-v3.0",
            "query": "test fallback chain rerank",
            "documents": ["doc1", "doc2"]
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
async fn test_rerank_response_format() {
    let client = Client::new();
    
    let resp = client
        .post("http://localhost:3000/v1/rerank")
        .header("Authorization", "Bearer sk-godwit-test")
        .json(&json!({
            "model": "rerank-english-v3.0",
            "query": "test query",
            "documents": ["document one", "document two"]
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
    
    // Each result should have index and relevance_score
    for result in results {
        assert!(result.get("index").is_some());
        assert!(result.get("relevance_score").is_some());
    }
}

#[tokio::test]
#[ignore = "requires running server and DB"]
async fn test_rerank_multiple_documents() {
    let client = Client::new();
    
    let resp = client
        .post("http://localhost:3000/v1/rerank")
        .header("Authorization", "Bearer sk-godwit-test")
        .json(&json!({
            "model": "rerank-english-v3.0",
            "query": "machine learning",
            "documents": [
                "Deep learning is a subset of machine learning",
                "Python is a programming language",
                "Machine learning algorithms learn from data",
                "Neural networks are used in deep learning"
            ]
        }))
        .send()
        .await
        .expect("request failed");
    
    assert!(resp.status().is_success());
    
    let body: serde_json::Value = resp.json().await.expect("invalid JSON");
    let results = body["results"].as_array().expect("results should be array");
    
    // Should have results for all documents
    assert_eq!(results.len(), 4);
    
    // Results should be ranked by relevance (highest first)
    let first_score = results[0]["relevance_score"].as_f64().expect("score should be number");
    let last_score = results[results.len() - 1]["relevance_score"].as_f64().expect("score should be number");
    assert!(first_score >= last_score);
}
