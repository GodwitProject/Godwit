use godwit_core::{ChatCompletionRequest, ChatContent, ChatMessage, Stop, ReasoningConfig, ThinkingConfig, ResponseFormat, JsonSchema, Tool, FunctionDefinition, ToolChoice, FunctionName};
use reqwest::Client;
use std::collections::HashMap;

#[tokio::test]
#[ignore = "requires running server with configured providers"]
async fn params_temperature_top_p_top_k() {
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
        top_p: Some(0.9),
        top_k: Some(40),
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
#[ignore = "requires running server with configured providers"]
async fn params_frequency_presence_penalties() {
    let client = Client::new();
    let req = ChatCompletionRequest {
        model: "gpt-4o".to_string(),
        messages: vec![ChatMessage {
            role: "user".to_string(),
            content: Some(vec![ChatContent::text("Write a poem")]),
            name: None,
            ..Default::default()
        }],
        stream: Some(false),
        frequency_penalty: Some(0.5),
        presence_penalty: Some(0.3),
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
}

#[tokio::test]
#[ignore = "requires running server with configured providers"]
async fn params_repetition_penalty() {
    let client = Client::new();
    let req = ChatCompletionRequest {
        model: "claude-3-5-sonnet-20241022".to_string(),
        messages: vec![ChatMessage {
            role: "user".to_string(),
            content: Some(vec![ChatContent::text("Write a story")]),
            name: None,
            ..Default::default()
        }],
        stream: Some(false),
        repetition_penalty: Some(1.2),
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
}

#[tokio::test]
#[ignore = "requires running server with configured providers"]
async fn params_stop_sequences_string() {
    let client = Client::new();
    let req = ChatCompletionRequest {
        model: "gpt-4o".to_string(),
        messages: vec![ChatMessage {
            role: "user".to_string(),
            content: Some(vec![ChatContent::text("Count to 10")]),
            name: None,
            ..Default::default()
        }],
        stream: Some(false),
        stop: Some(Stop::String("5".to_string())),
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
#[ignore = "requires running server with configured providers"]
async fn params_stop_sequences_array() {
    let client = Client::new();
    let req = ChatCompletionRequest {
        model: "gpt-4o".to_string(),
        messages: vec![ChatMessage {
            role: "user".to_string(),
            content: Some(vec![ChatContent::text("Count to 10")]),
            name: None,
            ..Default::default()
        }],
        stream: Some(false),
        stop: Some(Stop::Array(vec!["5".to_string(), "STOP".to_string()])),
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
#[ignore = "requires running server with configured providers"]
async fn params_seed_reproducibility() {
    let client = Client::new();
    let req = ChatCompletionRequest {
        model: "gpt-4o".to_string(),
        messages: vec![ChatMessage {
            role: "user".to_string(),
            content: Some(vec![ChatContent::text("Generate a random number")]),
            name: None,
            ..Default::default()
        }],
        stream: Some(false),
        seed: Some(42),
        max_tokens: Some(20),
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
async fn params_n_choices() {
    let client = Client::new();
    let req = ChatCompletionRequest {
        model: "gpt-4o".to_string(),
        messages: vec![ChatMessage {
            role: "user".to_string(),
            content: Some(vec![ChatContent::text("Say hi")]),
            name: None,
            ..Default::default()
        }],
        stream: Some(false),
        n: Some(3),
        max_tokens: Some(20),
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
        let choices = body.get("choices").and_then(|v| v.as_array());
        assert!(choices.map(|c| c.len()) == Some(3));
    }
}

#[tokio::test]
#[ignore = "requires running server with configured providers"]
async fn params_logprobs() {
    let client = Client::new();
    let req = ChatCompletionRequest {
        model: "gpt-4o".to_string(),
        messages: vec![ChatMessage {
            role: "user".to_string(),
            content: Some(vec![ChatContent::text("Hello")]),
            name: None,
            ..Default::default()
        }],
        stream: Some(false),
        logprobs: Some(true),
        top_logprobs: Some(5),
        max_tokens: Some(20),
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
async fn params_logit_bias() {
    let client = Client::new();
    let mut logit_bias = HashMap::new();
    logit_bias.insert("1234".to_string(), 5);
    logit_bias.insert("5678".to_string(), -5);
    
    let req = ChatCompletionRequest {
        model: "gpt-4o".to_string(),
        messages: vec![ChatMessage {
            role: "user".to_string(),
            content: Some(vec![ChatContent::text("Hello")]),
            name: None,
            ..Default::default()
        }],
        stream: Some(false),
        logit_bias: Some(logit_bias),
        max_tokens: Some(20),
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
async fn params_user_field() {
    let client = Client::new();
    let req = ChatCompletionRequest {
        model: "gpt-4o".to_string(),
        messages: vec![ChatMessage {
            role: "user".to_string(),
            content: Some(vec![ChatContent::text("Hello")]),
            name: None,
            ..Default::default()
        }],
        stream: Some(false),
        user: Some("test-user-123".to_string()),
        max_tokens: Some(20),
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
async fn params_parallel_tool_calls() {
    let client = Client::new();
    let req = ChatCompletionRequest {
        model: "gpt-4o".to_string(),
        messages: vec![ChatMessage {
            role: "user".to_string(),
            content: Some(vec![ChatContent::text("What's the weather?")]),
            name: None,
            ..Default::default()
        }],
        stream: Some(false),
        parallel_tool_calls: Some(false),
        tools: Some(vec![Tool {
            r#type: "function".to_string(),
            function: FunctionDefinition {
                name: "get_weather".to_string(),
                description: Some("Get weather for a location".to_string()),
                parameters: Some(serde_json::json!({"type": "object", "properties": {"city": {"type": "string"}}})),
            },
        }]),
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
#[ignore = "requires running server with configured providers"]
async fn params_response_format_json_schema() {
    let client = Client::new();
    let req = ChatCompletionRequest {
        model: "gpt-4o".to_string(),
        messages: vec![ChatMessage {
            role: "user".to_string(),
            content: Some(vec![ChatContent::text("Generate a person object")]),
            name: None,
            ..Default::default()
        }],
        stream: Some(false),
        response_format: Some(ResponseFormat::JsonSchema {
            json_schema: JsonSchema {
                name: "Person".to_string(),
                schema: Some(serde_json::json!({
                    "type": "object",
                    "properties": {
                        "name": {"type": "string"},
                        "age": {"type": "integer"}
                    },
                    "required": ["name", "age"]
                })),
                strict: Some(true),
            },
        }),
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
}

#[tokio::test]
#[ignore = "requires running server with configured providers"]
async fn params_reasoning_effort() {
    let client = Client::new();
    let req = ChatCompletionRequest {
        model: "o1-preview".to_string(),
        messages: vec![ChatMessage {
            role: "user".to_string(),
            content: Some(vec![ChatContent::text("Solve this math problem")]),
            name: None,
            ..Default::default()
        }],
        stream: Some(false),
        reasoning: Some(ReasoningConfig {
            effort: Some("high".to_string()),
            thinking: None,
        }),
        max_tokens: Some(500),
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
async fn params_thinking_budget() {
    let client = Client::new();
    let req = ChatCompletionRequest {
        model: "claude-3-5-sonnet-20241022".to_string(),
        messages: vec![ChatMessage {
            role: "user".to_string(),
            content: Some(vec![ChatContent::text("Think carefully")]),
            name: None,
            ..Default::default()
        }],
        stream: Some(false),
        reasoning: Some(ReasoningConfig {
            effort: None,
            thinking: Some(ThinkingConfig {
                r#type: "enabled".to_string(),
                budget_tokens: 1000,
            }),
        }),
        max_tokens: Some(500),
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
async fn params_all_together_streaming() {
    let client = Client::new();
    let mut logit_bias = HashMap::new();
    logit_bias.insert("1234".to_string(), 5);
    
    let req = ChatCompletionRequest {
        model: "gpt-4o".to_string(),
        messages: vec![ChatMessage {
            role: "user".to_string(),
            content: Some(vec![ChatContent::text("Complex request with all params")]),
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
        n: Some(1),
        logprobs: Some(false),
        user: Some("test-user".to_string()),
        parallel_tool_calls: Some(false),
        logit_bias: Some(logit_bias),
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
async fn params_anthropic_translation() {
    let client = Client::new();
    let req = ChatCompletionRequest {
        model: "claude-3-5-sonnet-20241022".to_string(),
        messages: vec![ChatMessage {
            role: "user".to_string(),
            content: Some(vec![ChatContent::text("Hello")]),
            name: None,
            ..Default::default()
        }],
        stream: Some(false),
        temperature: Some(0.7),
        top_p: Some(0.9),
        top_k: Some(40),
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
#[ignore = "requires running server with configured providers"]
async fn params_gemini_translation() {
    let client = Client::new();
    let req = ChatCompletionRequest {
        model: "gemini-pro".to_string(),
        messages: vec![ChatMessage {
            role: "user".to_string(),
            content: Some(vec![ChatContent::text("Hello")]),
            name: None,
            ..Default::default()
        }],
        stream: Some(false),
        temperature: Some(0.7),
        top_p: Some(0.9),
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
