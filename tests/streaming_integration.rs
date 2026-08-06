use godwit_core::{ChatCompletionRequest, ChatContent, ChatMessage};
use reqwest::Client;
use futures::StreamExt;

#[tokio::test]
#[ignore = "requires running server with configured providers"]
async fn streaming_gemini_completions() {
    let client = Client::new();
    let req = ChatCompletionRequest {
        model: "gemini-pro".to_string(),
        messages: vec![ChatMessage {
            role: "user".to_string(),
            content: Some(vec![ChatContent::text("Say hello in 3 sentences")]),
            name: None,
            ..Default::default()
        }],
        stream: Some(true),
        temperature: Some(0.7),
        max_tokens: Some(100),
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
    
    if resp.status().is_success() {
        let mut stream = resp.bytes_stream();
        let mut received_events = 0;
        while let Some(chunk) = stream.next().await {
            if let Ok(bytes) = chunk {
                let text = String::from_utf8_lossy(&bytes);
                if text.contains("data: ") {
                    received_events += 1;
                }
            }
        }
        assert!(received_events > 0, "Should receive streaming events from Gemini");
    }
}

#[tokio::test]
#[ignore = "requires running server with configured providers"]
async fn streaming_openai_completions() {
    let client = Client::new();
    let req = ChatCompletionRequest {
        model: "gpt-4o".to_string(),
        messages: vec![ChatMessage {
            role: "user".to_string(),
            content: Some(vec![ChatContent::text("Say hello in 3 sentences")]),
            name: None,
            ..Default::default()
        }],
        stream: Some(true),
        temperature: Some(0.7),
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
    
    if resp.status().is_success() {
        let mut stream = resp.bytes_stream();
        let mut received_events = 0;
        while let Some(chunk) = stream.next().await {
            if let Ok(bytes) = chunk {
                let text = String::from_utf8_lossy(&bytes);
                if text.contains("data: ") {
                    received_events += 1;
                }
            }
        }
        assert!(received_events > 0, "Should receive streaming events from OpenAI");
    }
}

#[tokio::test]
#[ignore = "requires running server with configured providers"]
async fn streaming_anthropic_completions() {
    let client = Client::new();
    let req = ChatCompletionRequest {
        model: "claude-3-5-sonnet-20241022".to_string(),
        messages: vec![ChatMessage {
            role: "user".to_string(),
            content: Some(vec![ChatContent::text("Say hello in 3 sentences")]),
            name: None,
            ..Default::default()
        }],
        stream: Some(true),
        temperature: Some(0.7),
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
    
    if resp.status().is_success() {
        let mut stream = resp.bytes_stream();
        let mut received_events = 0;
        while let Some(chunk) = stream.next().await {
            if let Ok(bytes) = chunk {
                let text = String::from_utf8_lossy(&bytes);
                if text.contains("data: ") {
                    received_events += 1;
                }
            }
        }
        assert!(received_events > 0, "Should receive streaming events from Anthropic");
    }
}

#[tokio::test]
#[ignore = "requires running server with configured providers"]
async fn streaming_azure_openai_completions() {
    let client = Client::new();
    let req = ChatCompletionRequest {
        model: "azure-gpt-4o".to_string(),
        messages: vec![ChatMessage {
            role: "user".to_string(),
            content: Some(vec![ChatContent::text("Say hello in 3 sentences")]),
            name: None,
            ..Default::default()
        }],
        stream: Some(true),
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
#[ignore = "requires running server with configured providers"]
async fn streaming_ollama_completions() {
    let client = Client::new();
    let req = ChatCompletionRequest {
        model: "ollama-llama3".to_string(),
        messages: vec![ChatMessage {
            role: "user".to_string(),
            content: Some(vec![ChatContent::text("Say hello")]),
            name: None,
            ..Default::default()
        }],
        stream: Some(true),
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
#[ignore = "requires running server with configured providers"]
async fn streaming_vllm_completions() {
    let client = Client::new();
    let req = ChatCompletionRequest {
        model: "vllm-mistral".to_string(),
        messages: vec![ChatMessage {
            role: "user".to_string(),
            content: Some(vec![ChatContent::text("Say hello")]),
            name: None,
            ..Default::default()
        }],
        stream: Some(true),
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
#[ignore = "requires running server with configured providers"]
async fn streaming_sglang_completions() {
    let client = Client::new();
    let req = ChatCompletionRequest {
        model: "sglang-llama".to_string(),
        messages: vec![ChatMessage {
            role: "user".to_string(),
            content: Some(vec![ChatContent::text("Say hello")]),
            name: None,
            ..Default::default()
        }],
        stream: Some(true),
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
#[ignore = "requires running server with configured providers"]
async fn streaming_llama_cpp_completions() {
    let client = Client::new();
    let req = ChatCompletionRequest {
        model: "llama-cpp".to_string(),
        messages: vec![ChatMessage {
            role: "user".to_string(),
            content: Some(vec![ChatContent::text("Say hello")]),
            name: None,
            ..Default::default()
        }],
        stream: Some(true),
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
#[ignore = "requires running server with configured providers"]
async fn streaming_non_streaming_response_format() {
    let client = Client::new();
    let req = ChatCompletionRequest {
        model: "gpt-4o".to_string(),
        messages: vec![ChatMessage {
            role: "user".to_string(),
            content: Some(vec![ChatContent::text("Say hello")]),
            name: None,
            ..Default::default()
        }],
        stream: Some(false),
        temperature: Some(0.7),
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
    
    if resp.status().is_success() {
        let body: serde_json::Value = resp.json().await.unwrap();
        assert!(body.get("choices").is_some());
        assert!(body.get("usage").is_some());
    }
}
