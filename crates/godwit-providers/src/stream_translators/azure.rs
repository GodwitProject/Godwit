use crate::adapter::{ProviderError, SseEvent};
use crate::streaming::{build_sse_delta, build_sse_finish, build_sse_error};
use bytes::Bytes;
use futures::stream::{self, BoxStream, StreamExt};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
struct AzureDelta {
    #[serde(default)]
    content: Option<String>,
    #[serde(default)]
    tool_calls: Option<Vec<serde_json::Value>>,
}

#[derive(Debug, Deserialize)]
struct AzureChoice {
    #[serde(default)]
    delta: Option<AzureDelta>,
    #[serde(default)]
    finish_reason: Option<String>,
}

#[derive(Debug, Deserialize)]
struct AzureUsage {
    #[serde(default)]
    prompt_tokens: i64,
    #[serde(default)]
    completion_tokens: i64,
    #[serde(default)]
    total_tokens: i64,
}

#[derive(Debug, Deserialize)]
struct AzureStreamChunk {
    #[serde(default)]
    choices: Vec<AzureChoice>,
    #[serde(default)]
    usage: Option<AzureUsage>,
}

pub struct AzureOpenAiStreamTranslator {
    finish_emitted: bool,
}

impl AzureOpenAiStreamTranslator {
    pub fn new() -> Self {
        Self {
            finish_emitted: false,
        }
    }

    pub fn translate_chunk(
        &mut self,
        chunk: &str,
    ) -> Vec<Result<SseEvent, ProviderError>> {
        if chunk.trim().is_empty() || chunk.trim() == "[DONE]" {
            return vec![];
        }

        let parsed: AzureStreamChunk = match serde_json::from_str(chunk) {
            Ok(v) => v,
            Err(e) => {
                return vec![Ok(SseEvent {
                    data: build_sse_error(&format!("failed to parse azure chunk: {e}")),
                })];
            }
        };

        let mut events: Vec<Result<SseEvent, ProviderError>> = Vec::new();

        for choice in &parsed.choices {
            if let Some(ref delta) = choice.delta {
                if let Some(ref content) = delta.content {
                    if !content.is_empty() {
                        events.push(Ok(SseEvent {
                            data: build_sse_delta(content),
                        }));
                    }
                }
                if let Some(ref tool_calls) = delta.tool_calls {
                    for call in tool_calls {
                        events.push(Ok(SseEvent {
                            data: build_sse_delta(serde_json::to_string(call).unwrap_or_default()),
                        }));
                    }
                }
            }

            if let Some(ref finish_reason) = choice.finish_reason {
                if !finish_reason.is_empty() && !self.finish_emitted {
                    self.finish_emitted = true;
                    let (prompt_tokens, completion_tokens) = parsed
                        .usage
                        .as_ref()
                        .map(|u| (u.prompt_tokens, u.completion_tokens))
                        .unwrap_or((0, 0));

                    events.push(Ok(SseEvent {
                        data: build_sse_finish(
                            prompt_tokens,
                            completion_tokens,
                            Some(finish_reason),
                        ),
                    }));
                }
            }
        }

        events
    }

    pub fn into_stream(
        self,
        byte_stream: impl futures::Stream<Item = Result<Bytes, reqwest::Error>>
            + Send
            + 'static,
    ) -> BoxStream<'static, Result<SseEvent, ProviderError>> {
        let mut translator = self;
        byte_stream
            .flat_map(move |bytes_result| {
                let text = match bytes_result {
                    Ok(bytes) => String::from_utf8_lossy(&bytes).to_string(),
                    Err(e) => {
                        return stream::iter(vec![Ok(SseEvent {
                            data: build_sse_error(&format!("utf8 decode error: {e}")),
                        })]);
                    }
                };

                stream::iter(translator.translate_chunk(&text))
            })
            .boxed()
    }
}

impl Default for AzureOpenAiStreamTranslator {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_translate_delta_chunk() {
        let mut translator = AzureOpenAiStreamTranslator::new();
        let chunk = r#"{"choices":[{"index":0,"delta":{"content":"Hello"},"finish_reason":null}]}"#;

        let events = translator.translate_chunk(chunk);
        assert_eq!(events.len(), 1);
        let event = events[0].as_ref().unwrap();
        let data: serde_json::Value = serde_json::from_str(&event.data).unwrap();
        assert_eq!(data["type"], "delta");
        assert_eq!(data["delta"], "Hello");
    }

    #[test]
    fn test_translate_finish_chunk_with_usage() {
        let mut translator = AzureOpenAiStreamTranslator::new();
        let chunk = r#"{"choices":[{"index":0,"delta":{"content":""},"finish_reason":"stop"}],"usage":{"prompt_tokens":10,"completion_tokens":5,"total_tokens":15}}"#;

        let events = translator.translate_chunk(chunk);
        assert_eq!(events.len(), 1);
        let event = events[0].as_ref().unwrap();
        let data: serde_json::Value = serde_json::from_str(&event.data).unwrap();
        assert_eq!(data["type"], "finish");
        assert_eq!(data["usage"]["prompt_tokens"], 10);
        assert_eq!(data["usage"]["completion_tokens"], 5);
        assert_eq!(data["usage"]["total_tokens"], 15);
        assert_eq!(data["finish_reason"], "stop");
    }

    #[test]
    fn test_translate_tool_calls() {
        let mut translator = AzureOpenAiStreamTranslator::new();
        let chunk = r#"{"choices":[{"index":0,"delta":{"tool_calls":[{"index":0,"function":{"name":"get_weather","arguments":"{\"city\":\"Paris\"}"}}]},"finish_reason":null}]}"#;

        let events = translator.translate_chunk(chunk);
        assert_eq!(events.len(), 1);
        let event = events[0].as_ref().unwrap();
        let data: serde_json::Value = serde_json::from_str(&event.data).unwrap();
        assert_eq!(data["type"], "delta");
        assert!(data["delta"].to_string().contains("get_weather"));
    }

    #[test]
    fn test_multiple_deltas_then_finish() {
        let mut translator = AzureOpenAiStreamTranslator::new();

        let chunk1 = r#"{"choices":[{"index":0,"delta":{"content":"Hello"},"finish_reason":null}]}"#;
        let chunk2 = r#"{"choices":[{"index":0,"delta":{"content":" world"},"finish_reason":null}]}"#;
        let chunk3 = r#"{"choices":[{"index":0,"delta":{"content":""},"finish_reason":"stop"}],"usage":{"prompt_tokens":10,"completion_tokens":5,"total_tokens":15}}"#;

        let events1 = translator.translate_chunk(chunk1);
        let events2 = translator.translate_chunk(chunk2);
        let events3 = translator.translate_chunk(chunk3);

        assert_eq!(events1.len(), 1);
        assert_eq!(events2.len(), 1);
        assert_eq!(events3.len(), 1);

        let data1: serde_json::Value = serde_json::from_str(&events1[0].as_ref().unwrap().data).unwrap();
        assert_eq!(data1["delta"], "Hello");

        let data2: serde_json::Value = serde_json::from_str(&events2[0].as_ref().unwrap().data).unwrap();
        assert_eq!(data2["delta"], " world");

        let data3: serde_json::Value = serde_json::from_str(&events3[0].as_ref().unwrap().data).unwrap();
        assert_eq!(data3["type"], "finish");
    }

    #[test]
    fn test_empty_chunk_produces_no_events() {
        let mut translator = AzureOpenAiStreamTranslator::new();
        let events = translator.translate_chunk("");
        assert!(events.is_empty());
    }

    #[test]
    fn test_malformed_chunk_produces_error() {
        let mut translator = AzureOpenAiStreamTranslator::new();
        let chunk = "not valid json";
        let events = translator.translate_chunk(chunk);
        assert_eq!(events.len(), 1);
        let event = events[0].as_ref().unwrap();
        let data: serde_json::Value = serde_json::from_str(&event.data).unwrap();
        assert_eq!(data["type"], "error");
        assert!(data["message"].as_str().unwrap().contains("failed to parse azure chunk"));
    }

    #[test]
    fn test_done_terminator_ignored() {
        let mut translator = AzureOpenAiStreamTranslator::new();
        let chunk = "[DONE]";
        let events = translator.translate_chunk(chunk);
        assert!(events.is_empty());
    }
}
