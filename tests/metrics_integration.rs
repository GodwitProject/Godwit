use reqwest::Client;

#[tokio::test]
#[ignore = "requires running server"]
async fn test_metrics_endpoint_returns_prometheus_format() {
    let client = Client::new();
    let response = client
        .get("http://localhost:3000/metrics")
        .send()
        .await
        .unwrap();
    
    assert_eq!(response.status(), 200);
    
    let content_type = response.headers().get("content-type").unwrap().to_str().unwrap();
    assert!(content_type.contains("text/plain"));
    assert!(content_type.contains("version=0.0.4"));
    
    let body = response.text().await.unwrap();
    
    assert!(body.contains("godwit_requests_total"));
    assert!(body.contains("godwit_request_duration_seconds"));
    assert!(body.contains("godwit_tokens_total"));
}

#[tokio::test]
#[ignore = "requires running server"]
async fn test_metrics_endpoint_contains_all_registered_metrics() {
    let client = Client::new();
    let response = client
        .get("http://localhost:3000/metrics")
        .send()
        .await
        .unwrap();
    
    assert_eq!(response.status(), 200);
    let body = response.text().await.unwrap();
    
    assert!(body.contains("godwit_requests_total"));
    assert!(body.contains("godwit_request_duration_seconds_bucket"));
    assert!(body.contains("godwit_request_duration_seconds_sum"));
    assert!(body.contains("godwit_request_duration_seconds_count"));
    assert!(body.contains("godwit_tokens_total"));
    assert!(body.contains("godwit_cost_usd_total"));
    assert!(body.contains("godwit_active_requests"));
}

#[tokio::test]
#[ignore = "requires running server"]
async fn test_metrics_records_request_counter() {
    let client = Client::new();
    
    let chat_req = serde_json::json!({
        "model": "gpt-4",
        "messages": [{"role": "user", "content": "Hello"}]
    });
    
    let _ = client
        .post("http://localhost:3000/v1/chat/completions")
        .header("Authorization", "Bearer sk-godwit-test")
        .json(&chat_req)
        .send()
        .await;
    
    let response = client
        .get("http://localhost:3000/metrics")
        .send()
        .await
        .unwrap();
    
    let body = response.text().await.unwrap();
    assert!(body.contains("godwit_requests_total"));
}

#[tokio::test]
#[ignore = "requires running server"]
async fn test_metrics_records_request_duration() {
    let client = Client::new();
    
    let chat_req = serde_json::json!({
        "model": "gpt-4",
        "messages": [{"role": "user", "content": "Hello"}]
    });
    
    let _ = client
        .post("http://localhost:3000/v1/chat/completions")
        .header("Authorization", "Bearer sk-godwit-test")
        .json(&chat_req)
        .send()
        .await;
    
    let response = client
        .get("http://localhost:3000/metrics")
        .send()
        .await
        .unwrap();
    
    let body = response.text().await.unwrap();
    assert!(body.contains("godwit_request_duration_seconds"));
}

#[tokio::test]
#[ignore = "requires running server"]
async fn test_metrics_records_tokens() {
    let client = Client::new();
    
    let chat_req = serde_json::json!({
        "model": "gpt-4",
        "messages": [{"role": "user", "content": "Hello world"}]
    });
    
    let _ = client
        .post("http://localhost:3000/v1/chat/completions")
        .header("Authorization", "Bearer sk-godwit-test")
        .json(&chat_req)
        .send()
        .await;
    
    let response = client
        .get("http://localhost:3000/metrics")
        .send()
        .await
        .unwrap();
    
    let body = response.text().await.unwrap();
    assert!(body.contains("godwit_tokens_total"));
}

#[tokio::test]
#[ignore = "requires running server"]
async fn test_metrics_request_counter_has_labels() {
    let client = Client::new();
    
    let chat_req = serde_json::json!({
        "model": "gpt-4o",
        "messages": [{"role": "user", "content": "Test"}]
    });
    
    let _ = client
        .post("http://localhost:3000/v1/chat/completions")
        .header("Authorization", "Bearer sk-godwit-test")
        .json(&chat_req)
        .send()
        .await;
    
    let response = client
        .get("http://localhost:3000/metrics")
        .send()
        .await
        .unwrap();
    
    let body = response.text().await.unwrap();
    assert!(body.contains("godwit_requests_total{model="));
    assert!(body.contains("provider="));
    assert!(body.contains("status="));
}

#[tokio::test]
#[ignore = "requires running server"]
async fn test_metrics_active_requests_gauge() {
    let client = Client::new();
    
    let response = client
        .get("http://localhost:3000/metrics")
        .send()
        .await
        .unwrap();
    
    let body = response.text().await.unwrap();
    assert!(body.contains("godwit_active_requests"));
}

#[tokio::test]
#[ignore = "requires running server"]
async fn test_metrics_cost_tracking() {
    let client = Client::new();
    
    let response = client
        .get("http://localhost:3000/metrics")
        .send()
        .await
        .unwrap();
    
    let body = response.text().await.unwrap();
    assert!(body.contains("godwit_cost_usd_total"));
}

#[tokio::test]
#[ignore = "requires running server"]
async fn test_metrics_histogram_buckets() {
    let client = Client::new();
    
    let response = client
        .get("http://localhost:3000/metrics")
        .send()
        .await
        .unwrap();
    
    let body = response.text().await.unwrap();
    assert!(body.contains("godwit_request_duration_seconds_bucket"));
    assert!(body.contains("le="));
}
