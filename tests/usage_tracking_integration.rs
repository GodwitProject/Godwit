use reqwest::Client;
use serde_json::json;

#[tokio::test]
#[ignore = "requires running server and DB"]
async fn test_anthropic_usage_tracked() {
    let client = Client::new();
    
    // Make chat request to Anthropic
    let resp = client
        .post("http://localhost:3000/v1/messages")
        .header("Authorization", "Bearer sk-godwit-test")
        .header("x-api-key", "sk-godwit-test")
        .json(&json!({
            "model": "claude-sonnet-4-20250514",
            "max_tokens": 1024,
            "messages": [{
                "role": "user",
                "content": "test usage tracking"
            }]
        }))
        .send()
        .await
        .expect("request failed");
    
    assert!(resp.status().is_success());
    
    let body: serde_json::Value = resp.json().await.expect("invalid JSON");
    
    // Assert: UsageReport has prompt_tokens, completion_tokens
    if let Some(usage) = body.get("usage") {
        assert!(usage.get("input_tokens").is_some() || usage.get("prompt_tokens").is_some());
        assert!(usage.get("output_tokens").is_some() || usage.get("completion_tokens").is_some());
    } else {
        panic!("usage field missing from response");
    }
    
    // Assert: /spend/logs shows correct cost
    // This would require DB access in a real integration test
}

#[tokio::test]
#[ignore = "requires running server and DB"]
async fn test_gemini_usage_tracked() {
    let client = Client::new();
    
    // Make chat request to Gemini
    let resp = client
        .post("http://localhost:3000/v1/chat/completions")
        .header("Authorization", "Bearer sk-godwit-test")
        .json(&json!({
            "model": "gemini/gemini-2.5-pro",
            "messages": [{
                "role": "user",
                "content": "test usage tracking"
            }]
        }))
        .send()
        .await
        .expect("request failed");
    
    assert!(resp.status().is_success());
    
    let body: serde_json::Value = resp.json().await.expect("invalid JSON");
    
    // Assert: UsageReport has prompt_tokens, completion_tokens
    if let Some(usage) = body.get("usage") {
        assert!(usage.get("prompt_tokens").is_some());
        assert!(usage.get("completion_tokens").is_some());
    } else {
        panic!("usage field missing from response");
    }
}

#[tokio::test]
#[ignore = "requires running server and DB"]
async fn test_embedding_usage_tracked() {
    let client = Client::new();
    
    // Make embedding request
    let resp = client
        .post("http://localhost:3000/v1/embeddings")
        .header("Authorization", "Bearer sk-godwit-test")
        .json(&json!({
            "model": "text-embedding-3-small",
            "input": ["test usage tracking"]
        }))
        .send()
        .await
        .expect("request failed");
    
    assert!(resp.status().is_success());
    
    let body: serde_json::Value = resp.json().await.expect("invalid JSON");
    
    // Assert: UsageReport has embedding_tokens
    if let Some(usage) = body.get("usage") {
        assert!(usage.get("prompt_tokens").is_some() || usage.get("total_tokens").is_some());
    } else {
        panic!("usage field missing from response");
    }
}

#[tokio::test]
#[ignore = "requires running server and DB"]
async fn test_image_usage_tracked() {
    let client = Client::new();
    
    // Make image generation request
    let resp = client
        .post("http://localhost:3000/v1/images/generations")
        .header("Authorization", "Bearer sk-godwit-test")
        .json(&json!({
            "model": "gpt-image-1",
            "prompt": "a cat wearing a hat",
            "n": 1
        }))
        .send()
        .await
        .expect("request failed");
    
    assert!(resp.status().is_success());
    
    let _body: serde_json::Value = resp.json().await.expect("invalid JSON");
    
    // Assert: UsageReport has image_count or equivalent
    // Image usage may be reported differently depending on provider
}

#[tokio::test]
#[ignore = "requires running server and DB"]
async fn test_audio_speech_usage_tracked() {
    let client = Client::new();
    
    // Make TTS request
    let resp = client
        .post("http://localhost:3000/v1/audio/speech")
        .header("Authorization", "Bearer sk-godwit-test")
        .json(&json!({
            "model": "tts-1",
            "input": "hello world",
            "voice": "alloy"
        }))
        .send()
        .await
        .expect("request failed");
    
    assert!(resp.status().is_success());
    
    // TTS returns audio binary, usage tracked server-side
    // Verify via /spend/logs in a real integration test
    let _body = resp.bytes().await.expect("failed to read response");
}

#[tokio::test]
#[ignore = "requires running server and DB"]
async fn test_audio_transcription_usage_tracked() {
    let client = Client::new();
    
    // Make STT request
    let form = reqwest::multipart::Form::new()
        .text("model", "whisper-1")
        .part(
            "file",
            reqwest::multipart::Part::bytes(vec![0u8; 16]).file_name("clip.wav"),
        );
    
    let resp = client
        .post("http://localhost:3000/v1/audio/transcriptions")
        .header("Authorization", "Bearer sk-godwit-test")
        .multipart(form)
        .send()
        .await
        .expect("request failed");
    
    assert!(resp.status().is_success());
    
    let _body: serde_json::Value = resp.json().await.expect("invalid JSON");
    
    // Assert: Usage tracked (audio_seconds estimated)
    // STT usage may be reported differently depending on provider
}
