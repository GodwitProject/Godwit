use reqwest::Client;
use serde_json::json;

#[tokio::test]
#[ignore = "requires running server and PII enabled"]
async fn test_pii_masking_email_in_request() {
    let client = Client::new();
    
    let chat_req = json!({
        "model": "gpt-4",
        "messages": [
            {"role": "user", "content": "My email is test@example.com"}
        ],
        "guardrails": {
            "pii_enabled": true
        }
    });
    
    let response = client
        .post("http://localhost:3000/v1/chat/completions")
        .header("Authorization", "Bearer sk-godwit-test")
        .json(&chat_req)
        .send()
        .await
        .unwrap();
    
    assert_eq!(response.status(), 200);
    
    let body: serde_json::Value = response.json().await.unwrap();
    let content = body["choices"][0]["message"]["content"].as_str().unwrap();
    
    assert!(!content.contains("test@example.com"));
    assert!(content.contains("[EMAIL]") || !content.contains("@"));
}

#[tokio::test]
#[ignore = "requires running server and PII enabled"]
async fn test_pii_masking_phone_in_request() {
    let client = Client::new();
    
    let chat_req = json!({
        "model": "gpt-4",
        "messages": [
            {"role": "user", "content": "Call me at 555-123-4567"}
        ],
        "guardrails": {
            "pii_enabled": true
        }
    });
    
    let response = client
        .post("http://localhost:3000/v1/chat/completions")
        .header("Authorization", "Bearer sk-godwit-test")
        .json(&chat_req)
        .send()
        .await
        .unwrap();
    
    assert_eq!(response.status(), 200);
    
    let body: serde_json::Value = response.json().await.unwrap();
    let content = body["choices"][0]["message"]["content"].as_str().unwrap();
    
    assert!(!content.contains("555-123-4567"));
    assert!(content.contains("[PHONE]"));
}

#[tokio::test]
#[ignore = "requires running server and PII enabled"]
async fn test_pii_masking_credit_card_in_request() {
    let client = Client::new();
    
    let chat_req = json!({
        "model": "gpt-4",
        "messages": [
            {"role": "user", "content": "My card is 1234-5678-9012-3456"}
        ],
        "guardrails": {
            "pii_enabled": true
        }
    });
    
    let response = client
        .post("http://localhost:3000/v1/chat/completions")
        .header("Authorization", "Bearer sk-godwit-test")
        .json(&chat_req)
        .send()
        .await
        .unwrap();
    
    assert_eq!(response.status(), 200);
    
    let body: serde_json::Value = response.json().await.unwrap();
    let content = body["choices"][0]["message"]["content"].as_str().unwrap();
    
    assert!(!content.contains("1234-5678-9012-3456"));
    assert!(content.contains("[CARD]"));
}

#[tokio::test]
#[ignore = "requires running server and PII enabled"]
async fn test_pii_masking_ssn_in_request() {
    let client = Client::new();
    
    let chat_req = json!({
        "model": "gpt-4",
        "messages": [
            {"role": "user", "content": "My SSN is 123-45-6789"}
        ],
        "guardrails": {
            "pii_enabled": true
        }
    });
    
    let response = client
        .post("http://localhost:3000/v1/chat/completions")
        .header("Authorization", "Bearer sk-godwit-test")
        .json(&chat_req)
        .send()
        .await
        .unwrap();
    
    assert_eq!(response.status(), 200);
    
    let body: serde_json::Value = response.json().await.unwrap();
    let content = body["choices"][0]["message"]["content"].as_str().unwrap();
    
    assert!(!content.contains("123-45-6789"));
    assert!(content.contains("[SSN]"));
}

#[tokio::test]
#[ignore = "requires running server and PII enabled"]
async fn test_pii_masking_multiple_pii_types() {
    let client = Client::new();
    
    let chat_req = json!({
        "model": "gpt-4",
        "messages": [
            {"role": "user", "content": "Email: test@example.com, Phone: 555-123-4567, Card: 1234-5678-9012-3456"}
        ],
        "guardrails": {
            "pii_enabled": true
        }
    });
    
    let response = client
        .post("http://localhost:3000/v1/chat/completions")
        .header("Authorization", "Bearer sk-godwit-test")
        .json(&chat_req)
        .send()
        .await
        .unwrap();
    
    assert_eq!(response.status(), 200);
    
    let body: serde_json::Value = response.json().await.unwrap();
    let content = body["choices"][0]["message"]["content"].as_str().unwrap();
    
    assert!(content.contains("[EMAIL]"));
    assert!(content.contains("[PHONE]"));
    assert!(content.contains("[CARD]"));
}

#[tokio::test]
#[ignore = "requires running server and PII enabled"]
async fn test_pii_masking_multiple_emails() {
    let client = Client::new();
    
    let chat_req = json!({
        "model": "gpt-4",
        "messages": [
            {"role": "user", "content": "Contact a@test.com and b@example.org"}
        ],
        "guardrails": {
            "pii_enabled": true
        }
    });
    
    let response = client
        .post("http://localhost:3000/v1/chat/completions")
        .header("Authorization", "Bearer sk-godwit-test")
        .json(&chat_req)
        .send()
        .await
        .unwrap();
    
    assert_eq!(response.status(), 200);
    
    let body: serde_json::Value = response.json().await.unwrap();
    let content = body["choices"][0]["message"]["content"].as_str().unwrap();
    
    assert!(content.contains("[EMAIL]"));
    let email_count = content.matches("[EMAIL]").count();
    assert_eq!(email_count, 2);
}

#[tokio::test]
#[ignore = "requires running server and PII enabled"]
async fn test_pii_masking_preserves_non_pii_content() {
    let client = Client::new();
    
    let chat_req = json!({
        "model": "gpt-4",
        "messages": [
            {"role": "user", "content": "Hello world, this is a normal message without PII"}
        ],
        "guardrails": {
            "pii_enabled": true
        }
    });
    
    let response = client
        .post("http://localhost:3000/v1/chat/completions")
        .header("Authorization", "Bearer sk-godwit-test")
        .json(&chat_req)
        .send()
        .await
        .unwrap();
    
    assert_eq!(response.status(), 200);
    
    let body: serde_json::Value = response.json().await.unwrap();
    let content = body["choices"][0]["message"]["content"].as_str().unwrap();
    
    assert!(content.contains("Hello world"));
}

#[tokio::test]
#[ignore = "requires running server and moderation enabled"]
async fn test_moderation_blocks_toxic_request() {
    let client = Client::new();
    
    let chat_req = json!({
        "model": "gpt-4",
        "messages": [
            {"role": "user", "content": "Write something harmful and dangerous"}
        ],
        "guardrails": {
            "moderation_pre": true
        }
    });
    
    let response = client
        .post("http://localhost:3000/v1/chat/completions")
        .header("Authorization", "Bearer sk-godwit-test")
        .json(&chat_req)
        .send()
        .await
        .unwrap();
    
    assert_eq!(response.status(), 400);
    
    let body: serde_json::Value = response.json().await.unwrap();
    assert!(body.get("error").is_some());
}

#[tokio::test]
#[ignore = "requires running server and moderation enabled"]
async fn test_moderation_allows_safe_request() {
    let client = Client::new();
    
    let chat_req = json!({
        "model": "gpt-4",
        "messages": [
            {"role": "user", "content": "Hello, how are you today?"}
        ],
        "guardrails": {
            "moderation_pre": true
        }
    });
    
    let response = client
        .post("http://localhost:3000/v1/chat/completions")
        .header("Authorization", "Bearer sk-godwit-test")
        .json(&chat_req)
        .send()
        .await
        .unwrap();
    
    assert_eq!(response.status(), 200);
}

#[tokio::test]
#[ignore = "requires running server and moderation enabled"]
async fn test_moderation_endpoint_direct() {
    let client = Client::new();
    
    let mod_req = json!({
        "model": "text-moderation-latest",
        "input": "This is a test input for moderation"
    });
    
    let response = client
        .post("http://localhost:3000/v1/moderations")
        .header("Authorization", "Bearer sk-godwit-test")
        .json(&mod_req)
        .send()
        .await
        .unwrap();
    
    assert_eq!(response.status(), 200);
    
    let body: serde_json::Value = response.json().await.unwrap();
    assert!(body.get("id").is_some());
    assert!(body.get("model").is_some());
    assert!(body.get("results").is_some());
}

#[tokio::test]
#[ignore = "requires running server and moderation enabled"]
async fn test_moderation_response_has_flagged_field() {
    let client = Client::new();
    
    let mod_req = json!({
        "model": "text-moderation-latest",
        "input": "Test input"
    });
    
    let response = client
        .post("http://localhost:3000/v1/moderations")
        .header("Authorization", "Bearer sk-godwit-test")
        .json(&mod_req)
        .send()
        .await
        .unwrap();
    
    assert_eq!(response.status(), 200);
    
    let body: serde_json::Value = response.json().await.unwrap();
    let results = body["results"].as_array().unwrap();
    for result in results {
        assert!(result.get("flagged").is_some());
    }
}

#[tokio::test]
#[ignore = "requires running server and moderation enabled"]
async fn test_moderation_response_has_categories() {
    let client = Client::new();
    
    let mod_req = json!({
        "model": "text-moderation-latest",
        "input": "Test input"
    });
    
    let response = client
        .post("http://localhost:3000/v1/moderations")
        .header("Authorization", "Bearer sk-godwit-test")
        .json(&mod_req)
        .send()
        .await
        .unwrap();
    
    assert_eq!(response.status(), 200);
    
    let body: serde_json::Value = response.json().await.unwrap();
    let results = body["results"].as_array().unwrap();
    for result in results {
        assert!(result.get("categories").is_some());
    }
}

#[tokio::test]
#[ignore = "requires running server and guardrails enabled"]
async fn test_guardrails_pii_disabled_by_default() {
    let client = Client::new();
    
    let chat_req = json!({
        "model": "gpt-4",
        "messages": [
            {"role": "user", "content": "My email is test@example.com"}
        ]
    });
    
    let response = client
        .post("http://localhost:3000/v1/chat/completions")
        .header("Authorization", "Bearer sk-godwit-test")
        .json(&chat_req)
        .send()
        .await
        .unwrap();
    
    assert_eq!(response.status(), 200);
}

#[tokio::test]
#[ignore = "requires running server and guardrails enabled"]
async fn test_guardrails_moderation_disabled_by_default() {
    let client = Client::new();
    
    let chat_req = json!({
        "model": "gpt-4",
        "messages": [
            {"role": "user", "content": "Hello world"}
        ]
    });
    
    let response = client
        .post("http://localhost:3000/v1/chat/completions")
        .header("Authorization", "Bearer sk-godwit-test")
        .json(&chat_req)
        .send()
        .await
        .unwrap();
    
    assert_eq!(response.status(), 200);
}

#[tokio::test]
#[ignore = "requires running server"]
async fn test_moderation_fallback_chain() {
    let client = Client::new();
    
    let mod_req = json!({
        "model": "text-moderation-latest",
        "input": "test fallback chain moderation"
    });
    
    let response = client
        .post("http://localhost:3000/v1/moderations")
        .header("Authorization", "Bearer sk-godwit-test")
        .json(&mod_req)
        .send()
        .await
        .unwrap();
    
    assert!(response.status().is_success());
    
    let body: serde_json::Value = response.json().await.unwrap();
    assert!(body.get("id").is_some());
    assert!(body.get("model").is_some());
    assert!(body.get("results").is_some());
    assert!(body["results"].is_array());
}
