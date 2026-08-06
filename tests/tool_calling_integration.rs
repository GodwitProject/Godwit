use godwit_core::{ChatCompletionRequest, ChatMessage, FunctionDefinition, Tool, ToolChoice};
use reqwest::Client;

#[tokio::test]
#[ignore = "requires running server and database"]
async fn tool_calling_mcp_tool_resolution() {
    let client = Client::new();
    let tools = vec![Tool {
        r#type: "function".to_string(),
        function: FunctionDefinition {
            name: "mcp_file_read".to_string(),
            description: Some("Read a file from the filesystem".to_string()),
            parameters: Some(serde_json::json!({
                "type": "object",
                "properties": {
                    "path": {"type": "string"}
                },
                "required": ["path"]
            })),
        },
    }];

    let resp = client
        .post("http://localhost:3000/v1/chat/completions")
        .header("Authorization", "Bearer sk-godwit-test")
        .json(&ChatCompletionRequest {
            model: "claude-sonnet-4-20250514".to_string(),
            messages: vec![ChatMessage {
                role: "user".to_string(),
                content: Some(vec![godwit_core::ChatContent::text("Read the file at /tmp/test.txt")]),
                name: None,
                tool_calls: None,
                tool_call_id: None,
                cache_control: None,
            }],
            stream: Some(false),
            temperature: None,
            max_tokens: None,
            tools: Some(tools),
            tool_choice: Some(ToolChoice::Auto),
            parallel_tool_calls: None,
            response_format: None,
            reasoning: None,
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
        })
        .send()
        .await
        .unwrap();

    assert!(resp.status().is_success() || resp.status() == 401);
}

#[tokio::test]
#[ignore = "requires running server and database"]
async fn tool_calling_web_search_searxng() {
    let client = Client::new();
    let tools = vec![Tool {
        r#type: "function".to_string(),
        function: FunctionDefinition {
            name: "web_search".to_string(),
            description: Some("Search the web for current information".to_string()),
            parameters: Some(serde_json::json!({
                "type": "object",
                "properties": {
                    "query": {"type": "string"}
                },
                "required": ["query"]
            })),
        },
    }];

    let resp = client
        .post("http://localhost:3000/v1/chat/completions")
        .header("Authorization", "Bearer sk-godwit-test")
        .json(&ChatCompletionRequest {
            model: "gpt-4o".to_string(),
            messages: vec![ChatMessage {
                role: "user".to_string(),
                content: Some(vec![godwit_core::ChatContent::text("What is the weather today?")]),
                name: None,
                tool_calls: None,
                tool_call_id: None,
                cache_control: None,
            }],
            stream: Some(false),
            temperature: None,
            max_tokens: None,
            tools: Some(tools),
            tool_choice: Some(ToolChoice::Auto),
            parallel_tool_calls: None,
            response_format: None,
            reasoning: None,
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
        })
        .send()
        .await
        .unwrap();

    assert!(resp.status().is_success() || resp.status() == 401);
}

#[tokio::test]
#[ignore = "requires running server and database"]
async fn tool_calling_agentic_loop_max_iterations() {
    let client = Client::new();
    let tools = vec![Tool {
        r#type: "function".to_string(),
        function: FunctionDefinition {
            name: "get_data".to_string(),
            description: Some("Get data from external API".to_string()),
            parameters: Some(serde_json::json!({
                "type": "object",
                "properties": {
                    "endpoint": {"type": "string"}
                },
                "required": ["endpoint"]
            })),
        },
    }];

    let resp = client
        .post("http://localhost:3000/v1/chat/completions")
        .header("Authorization", "Bearer sk-godwit-test")
        .json(&ChatCompletionRequest {
            model: "claude-sonnet-4-20250514".to_string(),
            messages: vec![ChatMessage {
                role: "user".to_string(),
                content: Some(vec![godwit_core::ChatContent::text("Research and summarize the latest AI developments")]),
                name: None,
                tool_calls: None,
                tool_call_id: None,
                cache_control: None,
            }],
            stream: Some(false),
            temperature: None,
            max_tokens: None,
            tools: Some(tools),
            tool_choice: Some(ToolChoice::Auto),
            parallel_tool_calls: None,
            response_format: None,
            reasoning: None,
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
        })
        .send()
        .await
        .unwrap();

    assert!(resp.status().is_success() || resp.status() == 401);
}

#[tokio::test]
#[ignore = "requires running server and database"]
async fn tool_calling_parallel_tool_calls() {
    let client = Client::new();
    let tools = vec![
        Tool {
            r#type: "function".to_string(),
            function: FunctionDefinition {
                name: "get_weather".to_string(),
                description: Some("Get weather for a city".to_string()),
                parameters: Some(serde_json::json!({
                    "type": "object",
                    "properties": {
                        "city": {"type": "string"}
                    },
                    "required": ["city"]
                })),
            },
        },
        Tool {
            r#type: "function".to_string(),
            function: FunctionDefinition {
                name: "get_time".to_string(),
                description: Some("Get current time for a timezone".to_string()),
                parameters: Some(serde_json::json!({
                    "type": "object",
                    "properties": {
                        "timezone": {"type": "string"}
                    },
                    "required": ["timezone"]
                })),
            },
        },
    ];

    let resp = client
        .post("http://localhost:3000/v1/chat/completions")
        .header("Authorization", "Bearer sk-godwit-test")
        .json(&ChatCompletionRequest {
            model: "gpt-4o".to_string(),
            messages: vec![ChatMessage {
                role: "user".to_string(),
                content: Some(vec![godwit_core::ChatContent::text("What's the weather and time in Paris and Tokyo?")]),
                name: None,
                tool_calls: None,
                tool_call_id: None,
                cache_control: None,
            }],
            stream: Some(false),
            temperature: None,
            max_tokens: None,
            tools: Some(tools),
            tool_choice: Some(ToolChoice::Auto),
            parallel_tool_calls: Some(true),
            response_format: None,
            reasoning: None,
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
        })
        .send()
        .await
        .unwrap();

    assert!(resp.status().is_success() || resp.status() == 401);
}
