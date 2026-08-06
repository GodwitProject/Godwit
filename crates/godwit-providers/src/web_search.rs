use godwit_core::{ChatCompletionRequest, FunctionDefinition, Tool};

/// Native web search tool names recognized across providers (OpenAI `web_search`,
/// OpenAI/Gemini `google_search`, Gemini `google_search_grounding`).
pub const NATIVE_WEB_SEARCH_TOOLS: &[&str] = &[
    "web_search",
    "web_search_20250305",
    "google_search",
    "google_search_grounding",
];

/// Creates a `web_search` tool definition for use in agentic tool injection.
/// This tool is injected when SearXNG is configured, allowing models to request
/// web searches via the SearXNG backend.
pub fn web_search_tool() -> Tool {
    Tool {
        r#type: "function".to_string(),
        function: FunctionDefinition {
            name: "web_search".to_string(),
            description: Some("Search the web for current information using SearXNG".to_string()),
            parameters: Some(serde_json::json!({
                "type": "object",
                "properties": {
                    "query": {
                        "type": "string",
                        "description": "The search query"
                    }
                },
                "required": ["query"]
            })),
        },
    }
}

/// Returns true if the given tool definition is a native web search tool.
pub fn is_native_web_search_tool(tool: &Tool) -> bool {
    NATIVE_WEB_SEARCH_TOOLS.contains(&tool.function.name.as_str())
}

/// Returns true if the request declares at least one native web search tool.
pub fn has_native_web_search_tool(tools: &[Tool]) -> bool {
    tools.iter().any(is_native_web_search_tool)
}

/// Removes any native web search tools, keeping only ordinary function tools.
///
/// Used by providers that do not support native web search, so those tools degrade
/// gracefully instead of being forwarded (and rejected) upstream.
pub fn strip_native_web_search_tools(tools: Vec<Tool>) -> Vec<Tool> {
    tools.into_iter().filter(|t| !is_native_web_search_tool(t)).collect()
}

/// Gracefully degrades a request by removing native web search tools before it is sent to
/// a provider that does not support them. No-op when the request has no web search tools.
pub fn strip_native_web_search_from_request(request: &mut ChatCompletionRequest) {
    if let Some(tools) = &mut request.tools {
        if has_native_web_search_tool(tools) {
            *tools = strip_native_web_search_tools(std::mem::take(tools));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tool_named(name: &str) -> Tool {
        Tool {
            r#type: "function".to_string(),
            function: godwit_core::FunctionDefinition {
                name: name.to_string(),
                description: None,
                parameters: None,
            },
        }
    }

    #[test]
    fn detects_native_web_search_tool_names() {
        for name in ["web_search", "web_search_20250305", "google_search", "google_search_grounding"] {
            assert!(is_native_web_search_tool(&tool_named(name)), "{name}");
        }
    }

    #[test]
    fn distinguishes_ordinary_function_tools() {
        assert!(!is_native_web_search_tool(&tool_named("get_weather")));
    }

    #[test]
    fn detects_web_search_in_request_tools() {
        let tools = vec![tool_named("get_weather"), tool_named("web_search")];
        assert!(has_native_web_search_tool(&tools));
        let tools = vec![tool_named("get_weather")];
        assert!(!has_native_web_search_tool(&tools));
    }

    #[test]
    fn strip_removes_only_web_search_tools() {
        let tools = vec![
            tool_named("get_weather"),
            tool_named("web_search"),
            tool_named("google_search"),
            tool_named("multiply"),
        ];
        let stripped = strip_native_web_search_tools(tools);
        let names: Vec<&str> = stripped.iter().map(|t| t.function.name.as_str()).collect();
        assert_eq!(names, vec!["get_weather", "multiply"]);
    }

    #[test]
    fn strip_returns_empty_when_only_web_search() {
        let stripped = strip_native_web_search_tools(vec![tool_named("web_search")]);
        assert!(stripped.is_empty());
    }

    #[test]
    fn strip_from_request_removes_web_search_tools_in_place() {
        let mut request = ChatCompletionRequest {
            model: "m".to_string(),
            messages: vec![],
            tools: Some(vec![
                tool_named("get_weather"),
                tool_named("web_search"),
                tool_named("google_search"),
                tool_named("multiply"),
            ]),
            ..Default::default()
        };
        strip_native_web_search_from_request(&mut request);
        let names: Vec<&str> = request
            .tools
            .as_ref()
            .unwrap()
            .iter()
            .map(|t| t.function.name.as_str())
            .collect();
        assert_eq!(names, vec!["get_weather", "multiply"]);
    }

    #[test]
    fn strip_from_request_leaves_tools_untouched_when_no_web_search() {
        let mut request = ChatCompletionRequest {
            model: "m".to_string(),
            messages: vec![],
            tools: Some(vec![tool_named("get_weather")]),
            ..Default::default()
        };
        strip_native_web_search_from_request(&mut request);
        assert_eq!(request.tools.as_ref().unwrap().len(), 1);
    }

    #[test]
    fn strip_from_request_handles_none_tools() {
        let mut request = ChatCompletionRequest {
            model: "m".to_string(),
            messages: vec![],
            tools: None,
            ..Default::default()
        };
        strip_native_web_search_from_request(&mut request);
        assert!(request.tools.is_none());
    }

    #[test]
    fn web_search_tool_has_correct_structure() {
        let tool = web_search_tool();
        assert_eq!(tool.r#type, "function");
        assert_eq!(tool.function.name, "web_search");
        assert!(tool.function.description.is_some());
        assert!(tool.function.description.as_ref().unwrap().contains("SearXNG"));
        
        let params = tool.function.parameters.unwrap();
        let obj = params.as_object().unwrap();
        assert_eq!(obj["type"], "object");
        
        let props = obj["properties"].as_object().unwrap();
        assert!(props.contains_key("query"));
        assert_eq!(props["query"]["type"], "string");
        
        let required = obj["required"].as_array().unwrap();
        assert!(required.iter().any(|v| v.as_str() == Some("query")));
    }
}
