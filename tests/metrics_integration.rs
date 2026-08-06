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
