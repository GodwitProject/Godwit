//! Egress translation: converts the proxy-canonical stream envelope
//! ({"type":"delta"|"finish"|"error"}) back into wire-compatible
//! Server-Sent-Event framing for OpenAI (chat.completion.chunk) and
//! Anthropic (/v1/messages). The canonical envelope is produced by each
//! adapter's chat_stream via streaming::normalize_*_sse_event; these
//! translators reconstruct protocol-native framing on egress.

use crate::SseEvent;
use serde_json::Value;

/// A single Server-Sent-Event frame ready for rendering onto the wire.
#[derive(Debug, Clone, PartialEq)]
pub enum ServerSentFrame {
    /// Renders as `data: <payload>` (the default SSE event with no name).
    Data(String),
    /// Renders as `event: <name>\ndata: <payload>`.
    Event { name: String, data: String },
}

impl ServerSentFrame {
    /// Render this frame as wire text. A `data:` payload containing raw
    /// newlines would need splitting in a fully general SSE encoder; here the
    /// payloads we emit are single-line JSON, so a one-line render suffices.
    pub fn render(&self) -> String {
        match self {
            ServerSentFrame::Data(data) => format!("data: {data}"),
            ServerSentFrame::Event { name, data } => format!("event: {name}\ndata: {data}"),
        }
    }
}

/// Typed interpretation of a canonical `SseEvent` payload.
#[derive(Debug, Clone, PartialEq)]
pub enum CanonicalEvent {
    /// `{"type":"delta","delta":"<text>"}`
    Delta(String),
    /// `{"type":"delta","delta":"<serialized tool-call object>"}` (delta starts with `{`)
    ToolCall(String),
    /// `{"type":"finish","usage":{...},"finish_reason":...}`
    Finish {
        prompt_tokens: i32,
        completion_tokens: i32,
        finish_reason: Option<String>,
    },
    /// `{"type":"error","message":...}`
    Error(String),
    /// Anything we cannot interpret.
    Unknown,
}

/// Parse the `data` payload of a canonical `SseEvent` into a typed event.
///
/// The canonical envelope is `{"type":"delta"|"finish"|"error",...}`. A
/// `delta` whose `delta` field starts with `{` is a packed serialized OpenAI
/// tool-call object (fields `index`/`id`/`function`), not plain text.
pub fn parse_canonical_event(data: &str) -> CanonicalEvent {
    let parsed: Value = match serde_json::from_str(data) {
        Ok(v) => v,
        Err(_) => return CanonicalEvent::Unknown,
    };

    match parsed.get("type").and_then(|t| t.as_str()) {
        Some("delta") => {
            let delta = parsed
                .get("delta")
                .and_then(|d| d.as_str())
                .unwrap_or_default()
                .to_string();
            if delta.starts_with('{') {
                CanonicalEvent::ToolCall(delta)
            } else {
                CanonicalEvent::Delta(delta)
            }
        }
        Some("finish") => {
            let usage = parsed.get("usage");
            let prompt_tokens = usage
                .and_then(|u| u.get("prompt_tokens"))
                .and_then(|v| v.as_i64())
                .unwrap_or(0) as i32;
            let completion_tokens = usage
                .and_then(|u| u.get("completion_tokens"))
                .and_then(|v| v.as_i64())
                .unwrap_or(0) as i32;
            let finish_reason = parsed
                .get("finish_reason")
                .and_then(|f| f.as_str())
                .map(|s| s.to_string());
            CanonicalEvent::Finish {
                prompt_tokens,
                completion_tokens,
                finish_reason,
            }
        }
        Some("error") => {
            let message = parsed
                .get("message")
                .and_then(|m| m.as_str())
                .unwrap_or_default()
                .to_string();
            CanonicalEvent::Error(message)
        }
        _ => CanonicalEvent::Unknown,
    }
}

/// Map a canonical finish reason to the OpenAI wire value.
/// `end_turn -> stop`, `max_tokens -> length`, `tool_use -> tool_calls`,
/// anything else passes through as-is, `None -> stop`.
fn canonical_to_openai_finish(reason: Option<&str>) -> &'static str {
    match reason.map(str::to_ascii_lowercase).as_deref() {
        Some("end_turn") => "stop",
        Some("max_tokens") => "length",
        Some("tool_use") => "tool_calls",
        Some("stop") => "stop",
        Some("length") => "length",
        Some("tool_calls") => "tool_calls",
        Some("content_filter") => "content_filter",
        Some(_) => "stop",
        None => "stop",
    }
}

/// Map a canonical finish reason to the Anthropic wire `stop_reason`.
/// `stop -> end_turn`, `length -> max_tokens`, `tool_calls -> tool_use`,
/// anything else passes through as-is, `None -> end_turn`.
fn canonical_to_anthropic_stop(reason: Option<&str>) -> &'static str {
    match reason.map(str::to_ascii_lowercase).as_deref() {
        Some("stop") => "end_turn",
        Some("length") => "max_tokens",
        Some("tool_calls") => "tool_use",
        Some("end_turn") => "end_turn",
        Some("max_tokens") => "max_tokens",
        Some("tool_use") => "tool_use",
        Some("content_filter") => "end_turn",
        Some(_) => "end_turn",
        None => "end_turn",
    }
}

/// Build an OpenAI `chat.completion.chunk` `data:` frame from raw `choices`.
fn openai_chunk(id: &str, model: &str, created: i64, choices: Vec<Value>) -> ServerSentFrame {
    let payload = serde_json::json!({
        "id": id,
        "object": "chat.completion.chunk",
        "created": created,
        "model": model,
        "choices": choices,
    });
    ServerSentFrame::Data(payload.to_string())
}

fn openai_choice_common() -> Value {
    serde_json::json!({"index": 0, "finish_reason": null})
}

/// Stateful OpenAI egress translator.
///
/// The proxy-canonical envelope carries no id/model/created per event, so a
/// single translator is constructed once per request with its metadata and
/// drives the whole stream, emitting the leading role chunk on first push.
pub struct OpenAiStreamTranslator {
    id: String,
    model: String,
    created: i64,
    started: bool,
    finished: bool,
}

impl OpenAiStreamTranslator {
    pub fn new(id: impl Into<String>, model: impl Into<String>, created: i64) -> Self {
        Self {
            id: id.into(),
            model: model.into(),
            created,
            started: false,
            finished: false,
        }
    }

    /// Consume one canonical `SseEvent` and produce the OpenAI wire frames it
    /// maps to. After the finish/error event the translator is marked done and
    /// subsequent pushes return no further frames.
    pub fn push(&mut self, event: &SseEvent) -> Vec<ServerSentFrame> {
        if self.finished {
            return Vec::new();
        }

        let mut frames = Vec::new();

        // First push starts the message with a role chunk (OpenAI convention).
        if !self.started {
            self.started = true;
            let mut choice = openai_choice_common();
            choice["delta"] = serde_json::json!({"role": "assistant", "content": ""});
            frames.push(openai_chunk(&self.id, &self.model, self.created, vec![choice]));
        }

        match parse_canonical_event(&event.data) {
            CanonicalEvent::Delta(text) => {
                let mut choice = openai_choice_common();
                choice["delta"] = serde_json::json!({"content": text});
                frames.push(openai_chunk(&self.id, &self.model, self.created, vec![choice]));
            }
            CanonicalEvent::ToolCall(packed) => {
                frames.extend(self.tool_call_chunks(&packed));
            }
            CanonicalEvent::Finish { finish_reason, .. } => {
                self.finished = true;
                let mut choice = openai_choice_common();
                choice["finish_reason"] = serde_json::Value::String(
                    canonical_to_openai_finish(finish_reason.as_deref()).to_string(),
                );
                choice["delta"] = serde_json::json!({});
                frames.push(openai_chunk(&self.id, &self.model, self.created, vec![choice]));
            }
            CanonicalEvent::Error(message) => {
                self.finished = true;
                let payload = serde_json::json!({
                    "error": {
                        "message": message,
                        "type": "server_error",
                        "param": null,
                        "code": null,
                    }
                });
                frames.push(ServerSentFrame::Data(payload.to_string()));
            }
            CanonicalEvent::Unknown => {}
        }

        frames
    }

    /// Translate a packed serialized OpenAI tool-call object into the two
    /// wire chunks OpenAI expects: one carrying the function name + id, and
    /// one (to be re-emitted on subsequent partial deltas) carrying arguments.
    fn tool_call_chunks(&self, packed: &str) -> Vec<ServerSentFrame> {
        let parsed: Value = match serde_json::from_str(packed) {
            Ok(v) => v,
            Err(_) => return Vec::new(),
        };

        let index = parsed.get("index").and_then(|v| v.as_i64()).unwrap_or(0);
        let id = parsed
            .get("id")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        let name = parsed
            .get("function")
            .and_then(|f| f.get("name"))
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        let args = parsed
            .get("function")
            .and_then(|f| f.get("arguments"))
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        let mut frames = Vec::new();

        // Chunk with the id + function name (open a tool call).
        if !id.is_empty() || !name.is_empty() {
            let mut choice = openai_choice_common();
            let mut call = serde_json::json!({
                "index": index,
                "type": "function",
                "function": {"name": name, "arguments": ""},
            });
            if !id.is_empty() {
                call["id"] = serde_json::Value::String(id.clone());
            }
            choice["delta"] = serde_json::json!({ "tool_calls": vec![call] });
            frames.push(openai_chunk(&self.id, &self.model, self.created, vec![choice]));
        }

        // Chunk with the arguments fragment.
        if !args.is_empty() {
            let mut choice = openai_choice_common();
            let mut call = serde_json::json!({
                "index": index,
                "type": "function",
                "function": {"arguments": args},
            });
            if !id.is_empty() {
                call["id"] = serde_json::Value::String(id.clone());
            }
            choice["delta"] = serde_json::json!({ "tool_calls": vec![call] });
            frames.push(openai_chunk(&self.id, &self.model, self.created, vec![choice]));
        }

        frames
    }
}

/// Stateful Anthropic `/v1/messages` egress translator.
///
/// Emits the proper named SSE events (`message_start`,
/// `content_block_start`, `content_block_delta`, `message_delta`, plus the
/// final `message_stop`) so the stream matches Anthropic's streaming API.
pub struct AnthropicStreamTranslator {
    id: String,
    model: String,
    started: bool,
    finished: bool,
}

impl AnthropicStreamTranslator {
    pub fn new(id: impl Into<String>, model: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            model: model.into(),
            started: false,
            finished: false,
        }
    }

    pub fn push(&mut self, event: &SseEvent) -> Vec<ServerSentFrame> {
        if self.finished {
            return Vec::new();
        }

        let mut frames = Vec::new();

        if !self.started {
            self.started = true;
            let message_start = serde_json::json!({
                "type": "message_start",
                "message": {
                    "id": self.id,
                    "type": "message",
                    "role": "assistant",
                    "model": self.model,
                    "content": [],
                    "stop_reason": null,
                    "stop_sequence": null,
                    "usage": {"input_tokens": 0, "output_tokens": 0},
                },
            });
            frames.push(ServerSentFrame::Event {
                name: "message_start".into(),
                data: message_start.to_string(),
            });

            let content_block_start = serde_json::json!({
                "type": "content_block_start",
                "index": 0,
                "content_block": {"type": "text", "text": ""},
            });
            frames.push(ServerSentFrame::Event {
                name: "content_block_start".into(),
                data: content_block_start.to_string(),
            });
        }

        match parse_canonical_event(&event.data) {
            CanonicalEvent::Delta(text) => {
                let content_block_delta = serde_json::json!({
                    "type": "content_block_delta",
                    "index": 0,
                    "delta": {"type": "text_delta", "text": text},
                });
                frames.push(ServerSentFrame::Event {
                    name: "content_block_delta".into(),
                    data: content_block_delta.to_string(),
                });
            }
            CanonicalEvent::ToolCall(packed) => {
                if let Some(partial_json) = extract_partial_json(&packed) {
                    let content_block_delta = serde_json::json!({
                        "type": "content_block_delta",
                        "index": 0,
                        "delta": {"type": "input_json_delta", "partial_json": partial_json},
                    });
                    frames.push(ServerSentFrame::Event {
                        name: "content_block_delta".into(),
                        data: content_block_delta.to_string(),
                    });
                }
            }
            CanonicalEvent::Finish {
                finish_reason,
                completion_tokens,
                ..
            } => {
                self.finished = true;
                let message_delta = serde_json::json!({
                    "type": "message_delta",
                    "delta": {
                        "stop_reason": canonical_to_anthropic_stop(finish_reason.as_deref()),
                        "stop_sequence": null,
                    },
                    "usage": {"output_tokens": completion_tokens},
                });
                frames.push(ServerSentFrame::Event {
                    name: "message_delta".into(),
                    data: message_delta.to_string(),
                });
            }
            CanonicalEvent::Error(message) => {
                self.finished = true;
                let error = serde_json::json!({
                    "type": "error",
                    "error": {"type": "api_error", "message": message},
                });
                frames.push(ServerSentFrame::Data(error.to_string()));
            }
            CanonicalEvent::Unknown => {}
        }

        if self.finished {
            frames.push(ServerSentFrame::Event {
                name: "message_stop".into(),
                data: serde_json::json!({"type": "message_stop"}).to_string(),
            });
        }

        frames
    }
}

/// Extract the `arguments` fragment (serialized JSON string) from a packed
/// OpenAI tool-call object, so it can be re-emitted as an Anthropic
/// `input_json_delta`. Returns `None` when no arguments fragment exists.
fn extract_partial_json(packed: &str) -> Option<String> {
    let parsed: Value = serde_json::from_str(packed).ok()?;
    let args = parsed
        .get("function")
        .and_then(|f| f.get("arguments"))
        .and_then(|v| v.as_str())?;
    if args.is_empty() {
        return None;
    }
    Some(args.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::streaming::{build_sse_delta, build_sse_error, build_sse_finish};

    fn ev(data: String) -> SseEvent {
        SseEvent { data }
    }

    #[test]
    fn renders_data_and_event_frames() {
        assert_eq!(ServerSentFrame::Data("x".into()).render(), "data: x");
        assert_eq!(
            ServerSentFrame::Event {
                name: "n".into(),
                data: "x".into()
            }
            .render(),
            "event: n\ndata: x"
        );
    }

    #[test]
    fn parses_delta_toolcall_finish_error() {
        assert_eq!(
            parse_canonical_event(&build_sse_delta("hi")),
            CanonicalEvent::Delta("hi".into())
        );
        assert_eq!(
            parse_canonical_event(&build_sse_delta(r#"{"index":0,"id":"x"}"#)),
            CanonicalEvent::ToolCall(r#"{"index":0,"id":"x"}"#.into())
        );
        assert_eq!(
            parse_canonical_event(&build_sse_finish(5, 2, Some("stop"))),
            CanonicalEvent::Finish {
                prompt_tokens: 5,
                completion_tokens: 2,
                finish_reason: Some("stop".into())
            }
        );
        assert_eq!(
            parse_canonical_event(&build_sse_error("boom")),
            CanonicalEvent::Error("boom".into())
        );
        assert_eq!(parse_canonical_event("not json"), CanonicalEvent::Unknown);
    }

    #[test]
    fn openai_streams_role_then_content_then_finish() {
        let mut t = OpenAiStreamTranslator::new("cmpl-1", "gpt-4", 12345);
        let out: Vec<String> = t
            .push(&ev(build_sse_delta("Hello")))
            .iter()
            .map(ServerSentFrame::render)
            .collect();
        assert_eq!(out.len(), 2);
        let first: Value = serde_json::from_str(out[0].trim_start_matches("data: ")).unwrap();
        assert_eq!(first["object"], "chat.completion.chunk");
        assert_eq!(first["choices"][0]["delta"]["role"], "assistant");
        let second: Value = serde_json::from_str(out[1].trim_start_matches("data: ")).unwrap();
        assert_eq!(second["choices"][0]["delta"]["content"], "Hello");

        let fin = t.push(&ev(build_sse_finish(5, 2, Some("stop"))));
        let fin_payload: Value =
            serde_json::from_str(fin[0].render().trim_start_matches("data: ")).unwrap();
        assert_eq!(fin_payload["choices"][0]["finish_reason"], "stop");
    }

    #[test]
    fn openai_translates_tool_call_packed_object() {
        let mut t = OpenAiStreamTranslator::new("cmpl-2", "gpt-4", 12345);
        let packed = r#"{"index":0,"id":"call_1","type":"function","function":{"name":"get_weather","arguments":"{\"city\":\"Paris\"}"}}"#;
        let frames = t.push(&ev(build_sse_delta(packed)));
        // role chunk + name chunk + args chunk
        assert_eq!(frames.len(), 3);
        let name_chunk: Value =
            serde_json::from_str(frames[1].render().trim_start_matches("data: ")).unwrap();
        assert_eq!(
            name_chunk["choices"][0]["delta"]["tool_calls"][0]["function"]["name"],
            "get_weather"
        );
        let args_chunk: Value =
            serde_json::from_str(frames[2].render().trim_start_matches("data: ")).unwrap();
        assert!(args_chunk["choices"][0]["delta"]["tool_calls"][0]["function"]["arguments"]
            .as_str()
            .unwrap()
            .contains("city"));
    }

    #[test]
    fn anthropic_stream_protocol_frames() {
        let mut t = AnthropicStreamTranslator::new("msg_1", "claude-3-5");
        let out = t.push(&ev(build_sse_delta("Hi")));
        let names: Vec<&str> = out
            .iter()
            .map(|f| match f {
                ServerSentFrame::Event { name, .. } => name.as_str(),
                ServerSentFrame::Data(_) => "data",
            })
            .collect();
        assert_eq!(names, vec!["message_start", "content_block_start", "content_block_delta"]);

        let fin = t.push(&ev(build_sse_finish(10, 4, Some("tool_calls"))));
        let fin_names: Vec<&str> = fin
            .iter()
            .map(|f| match f {
                ServerSentFrame::Event { name, .. } => name.as_str(),
                ServerSentFrame::Data(_) => "data",
            })
            .collect();
        assert_eq!(fin_names, vec!["message_delta", "message_stop"]);
        assert!(fin[0].render().contains("tool_use"));
        assert!(fin[0].render().contains("\"output_tokens\":4"));
    }

    #[test]
    fn finish_reason_case_is_normalized_before_matching() {
        assert_eq!(canonical_to_openai_finish(Some("MAX_TOKENS")), "length");
        assert_eq!(canonical_to_anthropic_stop(Some("MAX_TOKENS")), "max_tokens");
        assert_eq!(canonical_to_openai_finish(Some("END_TURN")), "stop");
        assert_eq!(canonical_to_anthropic_stop(Some("END_TURN")), "end_turn");
    }

    #[test]
    fn anthropic_tool_delta_emits_input_json_delta() {
        let mut t = AnthropicStreamTranslator::new("msg_2", "claude-3-5");
        t.push(&ev(build_sse_delta(""))); // consume message_start + content_block_start
        let packed = r#"{"index":0,"id":"call_1","function":{"name":"get_weather","arguments":"{\"city\":\"Paris\"}"}}"#;
        let out = t.push(&ev(build_sse_delta(packed)));
        let frame = &out[0];
        assert!(matches!(frame, ServerSentFrame::Event { name, .. } if name == "content_block_delta"));
        assert!(frame.render().contains("input_json_delta"));
        assert!(frame.render().contains("Paris"));
    }
}
