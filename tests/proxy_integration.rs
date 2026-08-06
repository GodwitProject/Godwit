use godwit_core::{ChatCompletionRequest, ChatContent, ChatMessage};
use reqwest::Client;

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
