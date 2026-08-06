use godwit_core::{ChatCompletionRequest, ChatContent, ChatContentPart, ChatMessage, ImageUrl};
use reqwest::Client;

#[tokio::test]
#[ignore = "requires running server and database"]
async fn multimodal_image_url_in_request() {
    let client = Client::new();
    let messages = vec![ChatMessage {
        role: "user".to_string(),
        content: Some(vec![ChatContent::Parts(vec![
            ChatContentPart::Text { text: "What is in this image?".to_string() },
            ChatContentPart::ImageUrl {
                image_url: ImageUrl {
                    url: "https://example.com/image.png".to_string(),
                    detail: Some("high".to_string()),
                },
            },
        ])]),
        name: None,
        tool_calls: None,
        tool_call_id: None,
        cache_control: None,
    }];

    let resp = client
        .post("http://localhost:3000/v1/chat/completions")
        .header("Authorization", "Bearer sk-godwit-test")
        .json(&ChatCompletionRequest {
            model: "gpt-4o".to_string(),
            messages,
            stream: Some(false),
            temperature: None,
            max_tokens: None,
            top_p: None,
            top_k: None,
            frequency_penalty: None,
            presence_penalty: None,
            repetition_penalty: None,
            stop: None,
            seed: None,
            n: None,
            logprobs: None,
            top_logprobs: None,
            logit_bias: None,
            user: None,
            tools: None,
            tool_choice: None,
            parallel_tool_calls: None,
            response_format: None,
            reasoning: None,
        })
        .send()
        .await
        .unwrap();

    assert!(resp.status().is_success() || resp.status() == 401);
}

#[tokio::test]
#[ignore = "requires running server and database"]
async fn multimodal_base64_image_in_request() {
    let client = Client::new();
    let base64_image = "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mNk+M9QDwADhgGAWjR9awAAAABJRU5ErkJggg==";
    let data_url = format!("data:image/png;base64,{}", base64_image);

    let messages = vec![ChatMessage {
        role: "user".to_string(),
        content: Some(vec![ChatContent::Parts(vec![
            ChatContentPart::Text { text: "Describe this image".to_string() },
            ChatContentPart::ImageUrl {
                image_url: ImageUrl {
                    url: data_url,
                    detail: None,
                },
            },
        ])]),
        name: None,
        tool_calls: None,
        tool_call_id: None,
        cache_control: None,
    }];

    let resp = client
        .post("http://localhost:3000/v1/chat/completions")
        .header("Authorization", "Bearer sk-godwit-test")
        .json(&ChatCompletionRequest {
            model: "claude-sonnet-4-20250514".to_string(),
            messages,
            stream: Some(false),
            temperature: None,
            max_tokens: None,
            top_p: None,
            top_k: None,
            frequency_penalty: None,
            presence_penalty: None,
            repetition_penalty: None,
            stop: None,
            seed: None,
            n: None,
            logprobs: None,
            top_logprobs: None,
            logit_bias: None,
            user: None,
            tools: None,
            tool_choice: None,
            parallel_tool_calls: None,
            response_format: None,
            reasoning: None,
        })
        .send()
        .await
        .unwrap();

    assert!(resp.status().is_success() || resp.status() == 401);
}

#[tokio::test]
#[ignore = "requires running server and database"]
async fn multimodal_backward_compatibility_string_content() {
    let client = Client::new();
    let messages = vec![ChatMessage {
        role: "user".to_string(),
        content: Some(vec![ChatContent::Text("Hello, world!".to_string())]),
        name: None,
        tool_calls: None,
        tool_call_id: None,
        cache_control: None,
    }];

    let resp = client
        .post("http://localhost:3000/v1/chat/completions")
        .header("Authorization", "Bearer sk-godwit-test")
        .json(&ChatCompletionRequest {
            model: "gpt-4o".to_string(),
            messages,
            stream: Some(false),
            temperature: None,
            max_tokens: None,
            top_p: None,
            top_k: None,
            frequency_penalty: None,
            presence_penalty: None,
            repetition_penalty: None,
            stop: None,
            seed: None,
            n: None,
            logprobs: None,
            top_logprobs: None,
            logit_bias: None,
            user: None,
            tools: None,
            tool_choice: None,
            parallel_tool_calls: None,
            response_format: None,
            reasoning: None,
        })
        .send()
        .await
        .unwrap();

    assert!(resp.status().is_success() || resp.status() == 401);
}

#[tokio::test]
#[ignore = "requires running server and database"]
async fn multimodal_provider_translation_anthropic() {
    let client = Client::new();
    let messages = vec![ChatMessage {
        role: "user".to_string(),
        content: Some(vec![ChatContent::Parts(vec![
            ChatContentPart::Text { text: "Analyze this diagram".to_string() },
            ChatContentPart::ImageUrl {
                image_url: ImageUrl {
                    url: "https://example.com/diagram.png".to_string(),
                    detail: Some("high".to_string()),
                },
            },
        ])]),
        name: None,
        tool_calls: None,
        tool_call_id: None,
        cache_control: None,
    }];

    let resp = client
        .post("http://localhost:3000/v1/chat/completions")
        .header("Authorization", "Bearer sk-godwit-test")
        .json(&ChatCompletionRequest {
            model: "claude-sonnet-4-20250514".to_string(),
            messages,
            stream: Some(false),
            temperature: None,
            max_tokens: None,
            top_p: None,
            top_k: None,
            frequency_penalty: None,
            presence_penalty: None,
            repetition_penalty: None,
            stop: None,
            seed: None,
            n: None,
            logprobs: None,
            top_logprobs: None,
            logit_bias: None,
            user: None,
            tools: None,
            tool_choice: None,
            parallel_tool_calls: None,
            response_format: None,
            reasoning: None,
        })
        .send()
        .await
        .unwrap();

    assert!(resp.status().is_success() || resp.status() == 401);
}

#[tokio::test]
#[ignore = "requires running server and database"]
async fn multimodal_provider_translation_openai() {
    let client = Client::new();
    let messages = vec![ChatMessage {
        role: "user".to_string(),
        content: Some(vec![ChatContent::Parts(vec![
            ChatContentPart::Text { text: "What do you see?".to_string() },
            ChatContentPart::ImageUrl {
                image_url: ImageUrl {
                    url: "https://example.com/photo.jpg".to_string(),
                    detail: Some("low".to_string()),
                },
            },
        ])]),
        name: None,
        tool_calls: None,
        tool_call_id: None,
        cache_control: None,
    }];

    let resp = client
        .post("http://localhost:3000/v1/chat/completions")
        .header("Authorization", "Bearer sk-godwit-test")
        .json(&ChatCompletionRequest {
            model: "gpt-4o".to_string(),
            messages,
            stream: Some(false),
            temperature: None,
            max_tokens: None,
            top_p: None,
            top_k: None,
            frequency_penalty: None,
            presence_penalty: None,
            repetition_penalty: None,
            stop: None,
            seed: None,
            n: None,
            logprobs: None,
            top_logprobs: None,
            logit_bias: None,
            user: None,
            tools: None,
            tool_choice: None,
            parallel_tool_calls: None,
            response_format: None,
            reasoning: None,
        })
        .send()
        .await
        .unwrap();

    assert!(resp.status().is_success() || resp.status() == 401);
}
