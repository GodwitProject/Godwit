use crate::{ProviderError, SseEvent};

pub fn parse_sse_events(chunk: &str) -> Vec<SseEvent> {
    let mut events = Vec::new();
    for line in chunk.lines() {
        let line = line.trim();
        if line.is_empty() || line == ":" {
            continue;
        }
        if let Some(data) = line.strip_prefix("data: ") {
            if data == "[DONE]" {
                continue;
            }
            events.push(SseEvent {
                data: data.to_string(),
            });
        }
    }
    events
}

/// Builds a normalized `{ "type": "delta", "delta": "..." }` SSE payload.
pub fn build_sse_delta(delta: impl Into<String>) -> String {
    serde_json::json!({"type": "delta", "delta": delta.into()}).to_string()
}

/// Builds a normalized `{ "type": "finish", "usage": { ... }, "finish_reason": ... }`
/// SSE payload from prompt/completion token counts (in the proxy's canonical openai-shaped
/// usage fields) and the upstream finish reason (e.g. "stop", "length", "tool_calls").
pub fn build_sse_finish(
    prompt_tokens: i64,
    completion_tokens: i64,
    finish_reason: Option<&str>,
) -> String {
    let mut value = serde_json::json!({
        "type": "finish",
        "usage": {
            "prompt_tokens": prompt_tokens,
            "completion_tokens": completion_tokens,
            "total_tokens": prompt_tokens + completion_tokens,
        }
    });
    if let Some(reason) = finish_reason {
        value["finish_reason"] = serde_json::Value::String(reason.to_string());
    }
    value.to_string()
}

/// Builds a normalized `{ "type": "error", "message": ... }` SSE payload.
pub fn build_sse_error(message: &str) -> String {
    serde_json::json!({"type": "error", "message": message}).to_string()
}

/// Normalizes a single raw OpenAI-style chat-completions SSE event into zero or more
/// proxy-canonical `SseEvent`s.
///
/// The upstream streaming chunk looks like:
/// ```json
/// {"choices":[{"index":0,"delta":{"content":"Hello","tool_calls":[{}]},"finish_reason":null}]}
/// ```
/// A later chunk carries `"finish_reason":"stop"` and, on some backends, a top-level
/// `"usage":{...}` object. We translate that into a `{"type":"delta","delta":"Hello"}`
/// event for each text fragment, a `{"type":"finish","usage":{...}}` event on finish, and
/// forward `delta.tool_calls` fragments verbatim inside a delta event.
pub fn normalize_openai_sse_event(raw: SseEvent) -> Vec<Result<SseEvent, ProviderError>> {
    let parsed: serde_json::Value = match serde_json::from_str(&raw.data) {
        Ok(v) => v,
        Err(_) => return vec![],
    };

    let mut out: Vec<Result<SseEvent, ProviderError>> = Vec::new();
    let usage = parsed.get("usage").cloned();

    if let Some(choices) = parsed.get("choices").and_then(|c| c.as_array()) {
        for choice in choices {
            let delta = choice.get("delta");
            if let Some(content) = delta
                .and_then(|d| d.get("content"))
                .and_then(|c| c.as_str())
            {
                if !content.is_empty() {
                    out.push(Ok(SseEvent {
                        data: build_sse_delta(content),
                    }));
                }
            }
            if let Some(tool_calls) = delta
                .and_then(|d| d.get("tool_calls"))
                .and_then(|t| t.as_array())
            {
                for call in tool_calls {
                    out.push(Ok(SseEvent {
                        data: build_sse_delta(serde_json::to_string(call).unwrap_or_default()),
                    }));
                }
            }
            let finish_reason = choice
                .get("finish_reason")
                .and_then(|f| f.as_str())
                .map(|s| s.to_string());
            if finish_reason.is_some() {
                let (prompt, completion) = match &usage {
                    Some(u) => (
                        u.get("prompt_tokens")
                            .and_then(|v| v.as_i64())
                            .unwrap_or(0),
                        u.get("completion_tokens")
                            .and_then(|v| v.as_i64())
                            .unwrap_or(0),
                    ),
                    None => (0, 0),
                };
                out.push(Ok(SseEvent {
                    data: build_sse_finish(prompt, completion, finish_reason.as_deref()),
                }));
            }
        }
    }

    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_openai_sse_chunk() {
        let line = "data: {\"id\":\"1\",\"object\":\"chat.completion.chunk\"}\n\n";
        let events = parse_sse_events(line);
        assert_eq!(events.len(), 1);
        assert!(events[0].data.contains("chat.completion.chunk"));
    }

    #[test]
    fn ignores_sse_done() {
        let line = "data: [DONE]\n\n";
        let events = parse_sse_events(line);
        assert!(events.is_empty());
    }

    #[test]
    fn normalizes_openai_content_delta() {
        let raw = SseEvent {
            data: r#"{"choices":[{"index":0,"delta":{"content":"Hello"},"finish_reason":null}]}"#
                .to_string(),
        };
        let events: Vec<SseEvent> = normalize_openai_sse_event(raw)
            .into_iter()
            .filter_map(|r| r.ok())
            .collect();
        assert_eq!(events.len(), 1);
        let delta: serde_json::Value = serde_json::from_str(&events[0].data).unwrap();
        assert_eq!(delta["type"], "delta");
        assert_eq!(delta["delta"], "Hello");
    }

    #[test]
    fn normalizes_openai_finish_reason_into_finish_event() {
        let raw = SseEvent {
            data: r#"{"choices":[{"index":0,"delta":{"content":""},"finish_reason":"stop"}],"usage":{"prompt_tokens":5,"completion_tokens":2,"total_tokens":7}}"#
                .to_string(),
        };
        let events: Vec<SseEvent> = normalize_openai_sse_event(raw)
            .into_iter()
            .filter_map(|r| r.ok())
            .collect();
        assert_eq!(events.len(), 1);
        let finish: serde_json::Value = serde_json::from_str(&events[0].data).unwrap();
        assert_eq!(finish["type"], "finish");
        assert_eq!(finish["usage"]["prompt_tokens"], 5);
        assert_eq!(finish["usage"]["completion_tokens"], 2);
        assert_eq!(finish["usage"]["total_tokens"], 7);
    }

    #[test]
    fn normalizes_openai_tool_calls() {
        let raw = SseEvent {
            data: r#"{"choices":[{"index":0,"delta":{"tool_calls":[{"index":0,"function":{"name":"get_weather","arguments":"{\"city\":\"Paris\"}"}}]},"finish_reason":null}]}"#
                .to_string(),
        };
        let events: Vec<SseEvent> = normalize_openai_sse_event(raw)
            .into_iter()
            .filter_map(|r| r.ok())
            .collect();
        assert_eq!(events.len(), 1);
        let delta: serde_json::Value = serde_json::from_str(&events[0].data).unwrap();
        assert_eq!(delta["type"], "delta");
        assert!(delta["delta"].to_string().contains("get_weather"));
    }
}
