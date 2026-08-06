use crate::adapter::{ProviderError, SseEvent};
use crate::streaming::{build_sse_delta, build_sse_finish, build_sse_error};
use bytes::Bytes;
use futures::stream::{self, BoxStream, StreamExt};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct GeminiPartResponse {
    text: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct GeminiContentResponse {
    #[serde(default)]
    parts: Vec<GeminiPartResponse>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct GeminiCandidate {
    content: GeminiContentResponse,
    #[serde(default)]
    finish_reason: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct GeminiUsageMetadata {
    #[serde(default)]
    prompt_token_count: i64,
    #[serde(default)]
    candidates_token_count: i64,
    #[serde(default)]
    total_token_count: i64,
    #[serde(default)]
    cached_content_token_count: Option<i64>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct GeminiStreamChunk {
    candidates: Vec<GeminiCandidate>,
    #[serde(default)]
    usage_metadata: Option<GeminiUsageMetadata>,
}

pub struct GeminiStreamTranslator {
    accumulated_content: String,
    finish_emitted: bool,
}

impl GeminiStreamTranslator {
    pub fn new() -> Self {
        Self {
            accumulated_content: String::new(),
            finish_emitted: false,
        }
    }

    pub fn translate_chunk(
        &mut self,
        chunk: &str,
    ) -> Vec<Result<SseEvent, ProviderError>> {
        if chunk.trim().is_empty() {
            return vec![];
        }

        let parsed: GeminiStreamChunk = match serde_json::from_str(chunk) {
            Ok(v) => v,
            Err(e) => {
                return vec![Ok(SseEvent {
                    data: build_sse_error(&format!("failed to parse gemini chunk: {e}")),
                })];
            }
        };

        let mut events: Vec<Result<SseEvent, ProviderError>> = Vec::new();

        for candidate in &parsed.candidates {
            for part in &candidate.content.parts {
                if let Some(text) = &part.text {
                    if !text.is_empty() {
                        self.accumulated_content.push_str(text);
                        events.push(Ok(SseEvent {
                            data: build_sse_delta(text),
                        }));
                    }
                }
            }

            if let Some(ref finish_reason) = candidate.finish_reason {
                if !finish_reason.is_empty() && !self.finish_emitted {
                    self.finish_emitted = true;
                    let (prompt_tokens, completion_tokens) = parsed
                        .usage_metadata
                        .as_ref()
                        .map(|m| (m.prompt_token_count, m.candidates_token_count))
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

        if events.is_empty() && !chunk.trim().is_empty() {
            tracing::debug!("ignoring gemini streaming chunk: no emit-able parts");
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

impl Default for GeminiStreamTranslator {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_translate_delta_chunk() {
        let mut translator = GeminiStreamTranslator::new();
        let chunk = r#"{
            "candidates": [{
                "content": {
                    "role": "model",
                    "parts": [{"text": "Hello"}]
                },
                "finishReason": null
            }]
        }"#;

        let events = translator.translate_chunk(chunk);
        assert_eq!(events.len(), 1);
        let event = events[0].as_ref().unwrap();
        let data: serde_json::Value = serde_json::from_str(&event.data).unwrap();
        assert_eq!(data["type"], "delta");
        assert_eq!(data["delta"], "Hello");
    }

    #[test]
    fn test_translate_finish_chunk_with_usage() {
        let mut translator = GeminiStreamTranslator::new();
        let chunk = r#"{
            "candidates": [{
                "content": {
                    "role": "model",
                    "parts": [{"text": ""}]
                },
                "finishReason": "STOP"
            }],
            "usageMetadata": {
                "promptTokenCount": 10,
                "candidatesTokenCount": 5,
                "totalTokenCount": 15
            }
        }"#;

        let events = translator.translate_chunk(chunk);
        assert_eq!(events.len(), 1);
        let event = events[0].as_ref().unwrap();
        let data: serde_json::Value = serde_json::from_str(&event.data).unwrap();
        assert_eq!(data["type"], "finish");
        assert_eq!(data["usage"]["prompt_tokens"], 10);
        assert_eq!(data["usage"]["completion_tokens"], 5);
        assert_eq!(data["usage"]["total_tokens"], 15);
        assert_eq!(data["finish_reason"], "STOP");
    }

    #[test]
    fn test_translate_finish_chunk_maps_finish_reason() {
        let mut translator = GeminiStreamTranslator::new();
        let chunk = r#"{
            "candidates": [{
                "content": {
                    "role": "model",
                    "parts": [{"text": ""}]
                },
                "finishReason": "LENGTH"
            }]
        }"#;

        let events = translator.translate_chunk(chunk);
        assert_eq!(events.len(), 1);
        let event = events[0].as_ref().unwrap();
        let data: serde_json::Value = serde_json::from_str(&event.data).unwrap();
        assert_eq!(data["type"], "finish");
        assert_eq!(data["finish_reason"], "LENGTH");
    }

    #[test]
    fn test_usage_extracted_from_metadata() {
        let mut translator = GeminiStreamTranslator::new();
        let chunk = r#"{
            "candidates": [{
                "content": {
                    "role": "model",
                    "parts": [{"text": ""}]
                },
                "finishReason": "STOP"
            }],
            "usageMetadata": {
                "promptTokenCount": 100,
                "candidatesTokenCount": 50,
                "totalTokenCount": 150,
                "cachedContentTokenCount": 20
            }
        }"#;

        let events = translator.translate_chunk(chunk);
        let event = events[0].as_ref().unwrap();
        let data: serde_json::Value = serde_json::from_str(&event.data).unwrap();
        assert_eq!(data["usage"]["prompt_tokens"], 100);
        assert_eq!(data["usage"]["completion_tokens"], 50);
        assert_eq!(data["usage"]["total_tokens"], 150);
    }

    #[test]
    fn test_multiple_deltas_then_finish() {
        let mut translator = GeminiStreamTranslator::new();
        
        let chunk1 = r#"{
            "candidates": [{
                "content": {
                    "role": "model",
                    "parts": [{"text": "Hello"}]
                }
            }]
        }"#;
        let chunk2 = r#"{
            "candidates": [{
                "content": {
                    "role": "model",
                    "parts": [{"text": " world"}]
                }
            }]
        }"#;
        let chunk3 = r#"{
            "candidates": [{
                "content": {
                    "role": "model",
                    "parts": [{"text": ""}]
                },
                "finishReason": "STOP"
            }],
            "usageMetadata": {
                "promptTokenCount": 10,
                "candidatesTokenCount": 5,
                "totalTokenCount": 15
            }
        }"#;

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
        let mut translator = GeminiStreamTranslator::new();
        let events = translator.translate_chunk("");
        assert!(events.is_empty());
    }

    #[test]
    fn test_malformed_chunk_produces_error() {
        let mut translator = GeminiStreamTranslator::new();
        let chunk = "not valid json";
        let events = translator.translate_chunk(chunk);
        assert_eq!(events.len(), 1);
        let event = events[0].as_ref().unwrap();
        let data: serde_json::Value = serde_json::from_str(&event.data).unwrap();
        assert_eq!(data["type"], "error");
        assert!(data["message"].as_str().unwrap().contains("failed to parse gemini chunk"));
    }
}
