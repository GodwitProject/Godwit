use godwit_core::{ChatCompletionRequest, ChatContent, ChatMessage, ReasoningConfig, ThinkingConfig, Stop};
use reqwest::Client;
use futures::StreamExt;

#[tokio::test]
#[ignore = "requires running server"]
async fn proxy_chat_completion_smoke() {
    let client = Client::new();
    let resp = client
        .post("http://localhost:3000/v1/chat/completions")
        .header("Authorization", "Bearer sk-godwit-test")
        .json(&ChatCompletionRequest {
            model: "gpt-4o".to_string(),
            messages: vec![ChatMessage {
                role: "user".to_string(),
                content: Some(vec![ChatContent::text("Hi")]),
                name: None,
                ..Default::default()
            }],
            stream: Some(false),
            temperature: None,
            max_tokens: None,
            ..Default::default()
        })
        .send()
        .await
        .unwrap();
    assert!(resp.status().is_success() || resp.status() == 401);
}

#[tokio::test]
#[ignore = "requires running server"]
async fn proxy_embeddings_smoke() {
    let client = reqwest::Client::new();
    let res = client
        .post("http://localhost:3000/v1/embeddings")
        .header("Authorization", "Bearer sk-godwit-test")
        .json(&serde_json::json!({"model": "text-embedding-3-small", "input": ["hello"]}))
        .send()
        .await
        .expect("request");
    assert!(res.status().is_success());
}

#[tokio::test]
#[ignore = "requires running server"]
async fn proxy_image_generations_smoke() {
    let client = reqwest::Client::new();
    let res = client
        .post("http://localhost:3000/v1/images/generations")
        .header("Authorization", "Bearer sk-godwit-test")
        .json(&serde_json::json!({"model": "gpt-image-1", "prompt": "a cat wearing a hat"}))
        .send()
        .await
        .expect("request");
    assert!(res.status().is_success());
}

#[tokio::test]
#[ignore = "requires running server"]
async fn proxy_audio_speech_smoke() {
    let client = reqwest::Client::new();
    let res = client
        .post("http://localhost:3000/v1/audio/speech")
        .header("Authorization", "Bearer sk-godwit-test")
        .json(&serde_json::json!({"model": "tts-1", "input": "hello world", "voice": "alloy"}))
        .send()
        .await
        .expect("request");
    assert!(res.status().is_success());
}

#[tokio::test]
#[ignore = "requires running server"]
async fn proxy_audio_transcriptions_smoke() {
    let form = reqwest::multipart::Form::new()
        .text("model", "whisper-1")
        .part(
            "file",
            reqwest::multipart::Part::bytes(vec![0u8; 16]).file_name("clip.wav"),
        );
    let client = reqwest::Client::new();
    let res = client
        .post("http://localhost:3000/v1/audio/transcriptions")
        .header("Authorization", "Bearer sk-godwit-test")
        .multipart(form)
        .send()
        .await
        .expect("request");
    assert!(res.status().is_success());
}

#[tokio::test]
#[ignore = "requires running server"]
async fn proxy_image_edits_smoke() {
    let form = reqwest::multipart::Form::new()
        .text("model", "gpt-image-1")
        .text("prompt", "add a hat")
        .part(
            "image",
            reqwest::multipart::Part::bytes(vec![0u8; 16]).file_name("image.png"),
        );
    let client = reqwest::Client::new();
    let res = client
        .post("http://localhost:3000/v1/images/edits")
        .header("Authorization", "Bearer sk-godwit-test")
        .multipart(form)
        .send()
        .await
        .expect("request");
    assert!(res.status().is_success());
}

#[tokio::test]
#[ignore = "requires running server"]
async fn proxy_streaming_with_advanced_params() {
    let client = Client::new();
    let req = ChatCompletionRequest {
        model: "gpt-4o".to_string(),
        messages: vec![ChatMessage {
            role: "user".to_string(),
            content: Some(vec![ChatContent::text("Say hello")]),
            name: None,
            ..Default::default()
        }],
        stream: Some(true),
        temperature: Some(0.7),
        max_tokens: Some(100),
        top_p: Some(0.9),
        top_k: Some(40),
        frequency_penalty: Some(0.5),
        presence_penalty: Some(0.3),
        repetition_penalty: Some(1.2),
        stop: Some(Stop::Array(vec!["STOP".to_string()])),
        seed: Some(42),
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
        assert!(received_events > 0, "Should receive streaming events");
    }
}

#[tokio::test]
#[ignore = "requires running server"]
async fn proxy_chat_with_all_advanced_params() {
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
        max_tokens: Some(100),
        top_p: Some(0.9),
        top_k: Some(40),
        frequency_penalty: Some(0.5),
        presence_penalty: Some(0.3),
        repetition_penalty: Some(1.2),
        stop: Some(Stop::Array(vec!["STOP".to_string()])),
        seed: Some(42),
        n: Some(1),
        logprobs: Some(false),
        user: Some("test-user".to_string()),
        parallel_tool_calls: Some(false),
        reasoning: Some(ReasoningConfig {
            effort: Some("medium".to_string()),
            thinking: Some(ThinkingConfig { r#type: "enabled".to_string(), budget_tokens: 500 }),
        }),
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
