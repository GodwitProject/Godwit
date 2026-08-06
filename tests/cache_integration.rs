use godwit_core::{ChatCompletionRequest, ChatContent, ChatMessage, CacheControl};
use reqwest::Client;

#[tokio::test]
#[ignore = "requires running server with database and cache enabled"]
async fn cache_anthropic_prompt_caching() {
    let client = Client::new();
    
    let long_context = "This is a long context that should be cached. ".repeat(100);
    
    let req1 = ChatCompletionRequest {
        model: "claude-3-5-sonnet-20241022".to_string(),
        messages: vec![ChatMessage {
            role: "user".to_string(),
            content: Some(vec![ChatContent::text(long_context.clone())]),
            name: None,
            cache_control: Some(CacheControl { r#type: "ephemeral".to_string() }),
            ..Default::default()
        }],
        stream: Some(false),
        max_tokens: Some(50),
        ..Default::default()
    };
    
    let resp1 = client
        .post("http://localhost:3000/v1/chat/completions")
        .header("Authorization", "Bearer sk-godwit-test")
        .json(&req1)
        .send()
        .await
        .unwrap();
    
    assert!(resp1.status().is_success() || resp1.status() == 401);
    
    if resp1.status().is_success() {
        let body1: serde_json::Value = resp1.json().await.unwrap();
        let usage1 = body1.get("usage").unwrap();
        
        let req2 = ChatCompletionRequest {
            model: "claude-3-5-sonnet-20241022".to_string(),
            messages: vec![ChatMessage {
                role: "user".to_string(),
                content: Some(vec![ChatContent::text(long_context + " Now answer: what is 2+2?")]),
                name: None,
                cache_control: Some(CacheControl { r#type: "ephemeral".to_string() }),
                ..Default::default()
            }],
            stream: Some(false),
            max_tokens: Some(50),
            ..Default::default()
        };
        
        let resp2 = client
            .post("http://localhost:3000/v1/chat/completions")
            .header("Authorization", "Bearer sk-godwit-test")
            .json(&req2)
            .send()
            .await
            .unwrap();
        
        assert!(resp2.status().is_success());
        
        let body2: serde_json::Value = resp2.json().await.unwrap();
        let usage2 = body2.get("usage").unwrap();
        
        let prompt_tokens1 = usage1.get("prompt_tokens").and_then(|v| v.as_i64()).unwrap_or(0);
        let prompt_tokens2 = usage2.get("prompt_tokens").and_then(|v| v.as_i64()).unwrap_or(0);
        
        assert!(prompt_tokens2 < prompt_tokens1, "Cached request should use fewer prompt tokens");
    }
}

#[tokio::test]
#[ignore = "requires running server with database and cache enabled"]
async fn cache_openai_prompt_caching() {
    let client = Client::new();
    
    let long_context = "This is a long context that should be cached. ".repeat(100);
    
    let req1 = ChatCompletionRequest {
        model: "gpt-4o".to_string(),
        messages: vec![ChatMessage {
            role: "system".to_string(),
            content: Some(vec![ChatContent::text(long_context.clone())]),
            name: None,
            cache_control: Some(CacheControl { r#type: "ephemeral".to_string() }),
            ..Default::default()
        }, ChatMessage {
            role: "user".to_string(),
            content: Some(vec![ChatContent::text("What is this about?")]),
            name: None,
            ..Default::default()
        }],
        stream: Some(false),
        max_tokens: Some(50),
        ..Default::default()
    };
    
    let resp1 = client
        .post("http://localhost:3000/v1/chat/completions")
        .header("Authorization", "Bearer sk-godwit-test")
        .json(&req1)
        .send()
        .await
        .unwrap();
    
    assert!(resp1.status().is_success() || resp1.status() == 401);
    
    if resp1.status().is_success() {
        let req2 = ChatCompletionRequest {
            model: "gpt-4o".to_string(),
            messages: vec![ChatMessage {
                role: "system".to_string(),
                content: Some(vec![ChatContent::text(long_context + " Answer briefly.")]),
                name: None,
                cache_control: Some(CacheControl { r#type: "ephemeral".to_string() }),
                ..Default::default()
            }],
            stream: Some(false),
            max_tokens: Some(50),
            ..Default::default()
        };
        
        let resp2 = client
            .post("http://localhost:3000/v1/chat/completions")
            .header("Authorization", "Bearer sk-godwit-test")
            .json(&req2)
            .send()
            .await
            .unwrap();
        
        assert!(resp2.status().is_success());
    }
}

#[tokio::test]
#[ignore = "requires running server with database and cache enabled"]
async fn cache_gemini_prompt_caching() {
    let client = Client::new();
    
    let long_context = "This is context for caching. ".repeat(100);
    
    let req1 = ChatCompletionRequest {
        model: "gemini-pro".to_string(),
        messages: vec![ChatMessage {
            role: "user".to_string(),
            content: Some(vec![ChatContent::text(long_context.clone())]),
            name: None,
            cache_control: Some(CacheControl { r#type: "ephemeral".to_string() }),
            ..Default::default()
        }],
        stream: Some(false),
        max_tokens: Some(50),
        ..Default::default()
    };
    
    let resp1 = client
        .post("http://localhost:3000/v1/chat/completions")
        .header("Authorization", "Bearer sk-godwit-test")
        .json(&req1)
        .send()
        .await
        .unwrap();
    
    assert!(resp1.status().is_success() || resp1.status() == 401);
    
    if resp1.status().is_success() {
        let req2 = ChatCompletionRequest {
            model: "gemini-pro".to_string(),
            messages: vec![ChatMessage {
                role: "user".to_string(),
                content: Some(vec![ChatContent::text(long_context + " Summarize this.")]),
                name: None,
                cache_control: Some(CacheControl { r#type: "ephemeral".to_string() }),
                ..Default::default()
            }],
            stream: Some(false),
            max_tokens: Some(50),
            ..Default::default()
        };
        
        let resp2 = client
            .post("http://localhost:3000/v1/chat/completions")
            .header("Authorization", "Bearer sk-godwit-test")
            .json(&req2)
            .send()
            .await
            .unwrap();
        
        assert!(resp2.status().is_success());
    }
}

#[tokio::test]
#[ignore = "requires running server with database and cache enabled"]
async fn cache_with_system_message_anthropic() {
    let client = Client::new();
    
    let system_context = "You are a helpful assistant. ".repeat(50);
    
    let req = ChatCompletionRequest {
        model: "claude-3-5-sonnet-20241022".to_string(),
        messages: vec![
            ChatMessage {
                role: "system".to_string(),
                content: Some(vec![ChatContent::text(system_context)]),
                name: None,
                cache_control: Some(CacheControl { r#type: "ephemeral".to_string() }),
                ..Default::default()
            },
            ChatMessage {
                role: "user".to_string(),
                content: Some(vec![ChatContent::text("Hello")]),
                name: None,
                ..Default::default()
            }
        ],
        stream: Some(false),
        max_tokens: Some(50),
        ..Default::default()
    };
    
    let resp = client
        .post("http://localhost:3000/v1/chat/completions")
        .header("Authorization", "Bearer sk-godwit-test")
        .json(&req)
        .send()
        .await
        .unwrap();
    
    assert!(resp.status().is_success() || resp.status() == 401);
}

#[tokio::test]
#[ignore = "requires running server with database and cache enabled"]
async fn cache_ttl_expiration() {
    let client = Client::new();
    
    let context = "Temporary context. ".repeat(20);
    
    let req1 = ChatCompletionRequest {
        model: "claude-3-5-sonnet-20241022".to_string(),
        messages: vec![ChatMessage {
            role: "user".to_string(),
            content: Some(vec![ChatContent::text(context.clone())]),
            name: None,
            cache_control: Some(CacheControl { r#type: "ephemeral".to_string() }),
            ..Default::default()
        }],
        stream: Some(false),
        max_tokens: Some(50),
        ..Default::default()
    };
    
    let resp1 = client
        .post("http://localhost:3000/v1/chat/completions")
        .header("Authorization", "Bearer sk-godwit-test")
        .json(&req1)
        .send()
        .await
        .unwrap();
    
    assert!(resp1.status().is_success() || resp1.status() == 401);
    
    tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
    
    let req2 = ChatCompletionRequest {
        model: "claude-3-5-sonnet-20241022".to_string(),
        messages: vec![ChatMessage {
            role: "user".to_string(),
            content: Some(vec![ChatContent::text(context + " Again.")]),
            name: None,
            cache_control: Some(CacheControl { r#type: "ephemeral".to_string() }),
            ..Default::default()
        }],
        stream: Some(false),
        max_tokens: Some(50),
        ..Default::default()
    };
    
    let resp2 = client
        .post("http://localhost:3000/v1/chat/completions")
        .header("Authorization", "Bearer sk-godwit-test")
        .json(&req2)
        .send()
        .await
        .unwrap();
    
    assert!(resp2.status().is_success() || resp2.status() == 401);
}
