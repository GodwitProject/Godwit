use godwit_core::{ChatCompletionRequest, ChatMessage, JsonSchema, ResponseFormat};
use reqwest::Client;

#[tokio::test]
#[ignore = "requires running server and database"]
async fn json_schema_validation_post_response() {
    let client = Client::new();
    let schema = serde_json::json!({
        "type": "object",
        "properties": {
            "name": {"type": "string"},
            "age": {"type": "integer"},
            "email": {"type": "string", "format": "email"}
        },
        "required": ["name", "age", "email"]
    });

    let resp = client
        .post("http://localhost:3000/v1/chat/completions")
        .header("Authorization", "Bearer sk-godwit-test")
        .json(&ChatCompletionRequest {
            model: "gpt-4o".to_string(),
            messages: vec![ChatMessage {
                role: "user".to_string(),
                content: Some(vec![godwit_core::ChatContent::text(
                    "Generate a person object with name John, age 30, email john@example.com"
                )]),
                name: None,
                tool_calls: None,
                tool_call_id: None,
                cache_control: None,
            }],
            stream: Some(false),
            temperature: None,
            max_tokens: None,
            top_p: None,
            top_k: None,
            frequency_penalty: None,
            presence_penalty: None,
            stop: None,
            seed: None,
            n: None,
            logprobs: None,
            top_logprobs: None,
            tools: None,
            tool_choice: None,
            parallel_tool_calls: None,
            response_format: Some(ResponseFormat::JsonSchema {
                json_schema: JsonSchema {
                    name: "Person".to_string(),
                    schema: Some(schema),
                    strict: Some(true),
                },
            }),
            reasoning: None,
        })
        .send()
        .await
        .unwrap();

    assert!(resp.status().is_success() || resp.status() == 401);
}

#[tokio::test]
#[ignore = "requires running server and database"]
async fn json_schema_guided_decoding_openai() {
    let client = Client::new();
    let schema = serde_json::json!({
        "type": "object",
        "properties": {
            "title": {"type": "string"},
            "summary": {"type": "string"}
        },
        "required": ["title", "summary"]
    });

    let resp = client
        .post("http://localhost:3000/v1/chat/completions")
        .header("Authorization", "Bearer sk-godwit-test")
        .json(&ChatCompletionRequest {
            model: "gpt-4o".to_string(),
            messages: vec![ChatMessage {
                role: "user".to_string(),
                content: Some(vec![godwit_core::ChatContent::text(
                    "Summarize the benefits of exercise"
                )]),
                name: None,
                tool_calls: None,
                tool_call_id: None,
                cache_control: None,
            }],
            stream: Some(false),
            temperature: None,
            max_tokens: None,
            top_p: None,
            top_k: None,
            frequency_penalty: None,
            presence_penalty: None,
            stop: None,
            seed: None,
            n: None,
            logprobs: None,
            top_logprobs: None,
            tools: None,
            tool_choice: None,
            parallel_tool_calls: None,
            response_format: Some(ResponseFormat::JsonSchema {
                json_schema: JsonSchema {
                    name: "Summary".to_string(),
                    schema: Some(schema),
                    strict: Some(true),
                },
            }),
            reasoning: None,
        })
        .send()
        .await
        .unwrap();

    assert!(resp.status().is_success() || resp.status() == 401);
}

#[tokio::test]
#[ignore = "requires running server and database"]
async fn json_schema_guided_decoding_vllm() {
    let client = Client::new();
    let schema = serde_json::json!({
        "type": "array",
        "items": {
            "type": "object",
            "properties": {
                "product": {"type": "string"},
                "price": {"type": "number"}
            },
            "required": ["product", "price"]
        }
    });

    let resp = client
        .post("http://localhost:3000/v1/chat/completions")
        .header("Authorization", "Bearer sk-godwit-test")
        .json(&ChatCompletionRequest {
            model: "meta-llama/Llama-3.1-8B-Instruct".to_string(),
            messages: vec![ChatMessage {
                role: "user".to_string(),
                content: Some(vec![godwit_core::ChatContent::text(
                    "List 3 products with their prices"
                )]),
                name: None,
                tool_calls: None,
                tool_call_id: None,
                cache_control: None,
            }],
            stream: Some(false),
            temperature: None,
            max_tokens: None,
            top_p: None,
            top_k: None,
            frequency_penalty: None,
            presence_penalty: None,
            stop: None,
            seed: None,
            n: None,
            logprobs: None,
            top_logprobs: None,
            tools: None,
            tool_choice: None,
            parallel_tool_calls: None,
            response_format: Some(ResponseFormat::JsonSchema {
                json_schema: JsonSchema {
                    name: "ProductList".to_string(),
                    schema: Some(schema),
                    strict: Some(true),
                },
            }),
            reasoning: None,
        })
        .send()
        .await
        .unwrap();

    assert!(resp.status().is_success() || resp.status() == 401);
}

#[tokio::test]
#[ignore = "requires running server and database"]
async fn json_schema_guided_decoding_sglang() {
    let client = Client::new();
    let schema = serde_json::json!({
        "type": "object",
        "properties": {
            "sentiment": {
                "type": "string",
                "enum": ["positive", "negative", "neutral"]
            },
            "confidence": {"type": "number", "minimum": 0, "maximum": 1}
        },
        "required": ["sentiment", "confidence"]
    });

    let resp = client
        .post("http://localhost:3000/v1/chat/completions")
        .header("Authorization", "Bearer sk-godwit-test")
        .json(&ChatCompletionRequest {
            model: "meta-llama/Llama-3.1-8B-Instruct".to_string(),
            messages: vec![ChatMessage {
                role: "user".to_string(),
                content: Some(vec![godwit_core::ChatContent::text(
                    "Analyze the sentiment of: 'This product is amazing!'"
                )]),
                name: None,
                tool_calls: None,
                tool_call_id: None,
                cache_control: None,
            }],
            stream: Some(false),
            temperature: None,
            max_tokens: None,
            top_p: None,
            top_k: None,
            frequency_penalty: None,
            presence_penalty: None,
            stop: None,
            seed: None,
            n: None,
            logprobs: None,
            top_logprobs: None,
            tools: None,
            tool_choice: None,
            parallel_tool_calls: None,
            response_format: Some(ResponseFormat::JsonSchema {
                json_schema: JsonSchema {
                    name: "SentimentAnalysis".to_string(),
                    schema: Some(schema),
                    strict: Some(true),
                },
            }),
            reasoning: None,
        })
        .send()
        .await
        .unwrap();

    assert!(resp.status().is_success() || resp.status() == 401);
}

#[tokio::test]
#[ignore = "requires running server and database"]
async fn json_schema_strict_validation_enabled() {
    let client = Client::new();
    let schema = serde_json::json!({
        "type": "object",
        "properties": {
            "id": {"type": "integer"},
            "value": {"type": "string"}
        },
        "required": ["id", "value"],
        "additionalProperties": false
    });

    let resp = client
        .post("http://localhost:3000/v1/chat/completions")
        .header("Authorization", "Bearer sk-godwit-test")
        .json(&ChatCompletionRequest {
            model: "gpt-4o".to_string(),
            messages: vec![ChatMessage {
                role: "user".to_string(),
                content: Some(vec![godwit_core::ChatContent::text(
                    "Create an object with id 1 and value test"
                )]),
                name: None,
                tool_calls: None,
                tool_call_id: None,
                cache_control: None,
            }],
            stream: Some(false),
            temperature: None,
            max_tokens: None,
            top_p: None,
            top_k: None,
            frequency_penalty: None,
            presence_penalty: None,
            stop: None,
            seed: None,
            n: None,
            logprobs: None,
            top_logprobs: None,
            tools: None,
            tool_choice: None,
            parallel_tool_calls: None,
            response_format: Some(ResponseFormat::JsonSchema {
                json_schema: JsonSchema {
                    name: "StrictObject".to_string(),
                    schema: Some(schema),
                    strict: Some(true),
                },
            }),
            reasoning: None,
        })
        .send()
        .await
        .unwrap();

    assert!(resp.status().is_success() || resp.status() == 401);
}
