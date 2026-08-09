use crate::adapter::{ProviderError, SseEvent};
use crate::streaming::{build_sse_delta, build_sse_finish, build_sse_error};
use bytes::Bytes;
use futures::stream::{self, BoxStream, StreamExt};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct OllamaMessage {
    #[serde(default)]
    role: Option<String>,
    #[serde(default)]
    content: Option<String>,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct OllamaStreamChunk {
    #[serde(default)]
    model: Option<String>,
    #[serde(default)]
    created_at: Option<String>,
    #[serde(default)]
    message: Option<OllamaMessage>,
    #[serde(default)]
    done: Option<bool>,
    #[serde(default)]
    total_duration: Option<i64>,
    #[serde(default)]
    load_duration: Option<i64>,
    #[serde(default)]
    prompt_eval_count: Option<i64>,
    #[serde(default)]
    eval_count: Option<i64>,
}

pub struct OllamaStreamTranslator {
    finish_emitted: bool,
}

impl OllamaStreamTranslator {
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

        let parsed: OllamaStreamChunk = match serde_json::from_str(chunk) {
            Ok(v) => v,
            Err(e) => {
                return vec![Ok(SseEvent {
                    data: build_sse_error(&format!("failed to parse ollama chunk: {e}")),
                })];
            }
        };

        let mut events: Vec<Result<SseEvent, ProviderError>> = Vec::new();

        if let Some(ref message) = parsed.message {
            if let Some(ref content) = message.content {
                if !content.is_empty() {
                    events.push(Ok(SseEvent {
                        data: build_sse_delta(content),
                    }));
                }
            }
        }

        if parsed.done == Some(true) && !self.finish_emitted {
            self.finish_emitted = true;
            let prompt_tokens = parsed.prompt_eval_count.unwrap_or(0) as i64;
            let completion_tokens = parsed.eval_count.unwrap_or(0) as i64;

            events.push(Ok(SseEvent {
                data: build_sse_finish(
                    prompt_tokens,
                    completion_tokens,
                    Some("stop"),
                ),
            }));
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

impl Default for OllamaStreamTranslator {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_translate_content_chunk() {
        let mut translator = OllamaStreamTranslator::new();
        let chunk = r#"{"model":"llama3","created_at":"2024-01-01T00:00:00Z","message":{"role":"assistant","content":"Hello"},"done":false}"#;

        let events = translator.translate_chunk(chunk);
        assert_eq!(events.len(), 1);
        let event = events[0].as_ref().unwrap();
        let data: serde_json::Value = serde_json::from_str(&event.data).unwrap();
        assert_eq!(data["type"], "delta");
        assert_eq!(data["delta"], "Hello");
    }

    #[test]
    fn test_translate_finish_chunk_with_usage() {
        let mut translator = OllamaStreamTranslator::new();
        let chunk = r#"{"model":"llama3","created_at":"2024-01-01T00:00:02Z","message":{"role":"assistant","content":""},"done":true,"total_duration":1234567890,"load_duration":123456789,"prompt_eval_count":10,"eval_count":5}"#;

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
    fn test_multiple_content_then_finish() {
        let mut translator = OllamaStreamTranslator::new();

        let chunk1 = r#"{"model":"llama3","message":{"role":"assistant","content":"Hello"},"done":false}"#;
        let chunk2 = r#"{"model":"llama3","message":{"role":"assistant","content":" world"},"done":false}"#;
        let chunk3 = r#"{"model":"llama3","message":{"role":"assistant","content":""},"done":true,"prompt_eval_count":10,"eval_count":5}"#;

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
        let mut translator = OllamaStreamTranslator::new();
        let events = translator.translate_chunk("");
        assert!(events.is_empty());
    }

    #[test]
    fn test_malformed_chunk_produces_error() {
        let mut translator = OllamaStreamTranslator::new();
        let chunk = "not valid json";
        let events = translator.translate_chunk(chunk);
        assert_eq!(events.len(), 1);
        let event = events[0].as_ref().unwrap();
        let data: serde_json::Value = serde_json::from_str(&event.data).unwrap();
        assert_eq!(data["type"], "error");
        assert!(data["message"].as_str().unwrap().contains("failed to parse ollama chunk"));
    }

    #[test]
    fn test_done_terminator_ignored() {
        let mut translator = OllamaStreamTranslator::new();
        let chunk = "[DONE]";
        let events = translator.translate_chunk(chunk);
        assert!(events.is_empty());
    }

    #[test]
    fn test_finish_only_emitted_once() {
        let mut translator = OllamaStreamTranslator::new();
        
        let chunk1 = r#"{"model":"llama3","done":true,"prompt_eval_count":10,"eval_count":5}"#;
        let chunk2 = r#"{"model":"llama3","done":true,"prompt_eval_count":20,"eval_count":10}"#;

        let events1 = translator.translate_chunk(chunk1);
        let events2 = translator.translate_chunk(chunk2);

        assert_eq!(events1.len(), 1);
        assert_eq!(events2.len(), 0);
    }

    #[test]
    fn test_empty_content_ignored() {
        let mut translator = OllamaStreamTranslator::new();
        let chunk = r#"{"model":"llama3","message":{"role":"assistant","content":""},"done":false}"#;

        let events = translator.translate_chunk(chunk);
        assert!(events.is_empty());
    }

    #[test]
    fn test_chunk_without_done_field() {
        let mut translator = OllamaStreamTranslator::new();
        let chunk = r#"{"model":"llama3","message":{"role":"assistant","content":"Hello"}}"#;

        let events = translator.translate_chunk(chunk);
        assert_eq!(events.len(), 1);
        let event = events[0].as_ref().unwrap();
        let data: serde_json::Value = serde_json::from_str(&event.data).unwrap();
        assert_eq!(data["type"], "delta");
        assert_eq!(data["delta"], "Hello");
    }

    #[test]
    fn test_chunk_with_missing_message() {
        let mut translator = OllamaStreamTranslator::new();
        let chunk = r#"{"model":"llama3","done":false}"#;

        let events = translator.translate_chunk(chunk);
        assert!(events.is_empty());
    }
}
