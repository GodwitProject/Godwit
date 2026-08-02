use godwit_core::{ChatCompletionRequest, ChatMessage};
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
                content: "Hi".to_string(),
            }],
            stream: Some(false),
            temperature: None,
            max_tokens: None,
        })
        .send()
        .await
        .unwrap();
    assert!(resp.status().is_success() || resp.status() == 401);
}
