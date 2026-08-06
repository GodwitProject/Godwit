//! Minimal JSON-RPC 2.0 message types and the newline-delimited framing used by the
//! Model Context Protocol (MCP) stdio transport.
//!
//! MCP speaks JSON-RPC 2.0 over stdio. Each message is a single JSON object encoded on
//! its own line (newline-delimited JSON / NDJSON). This module models the subset of the
//! protocol we need — requests, responses and notifications — and provides serialization
//! helpers plus a line-based frame reader/writer suitable for streaming over a pipe.
use serde::{Deserialize, Serialize};
use serde_json::Value;
use thiserror::Error;

/// The JSON-RPC version tag mandated by the spec.
pub const JSONRPC_VERSION: &str = "2.0";
/// The MCP protocol version we advertise during `initialize`.
pub const MCP_PROTOCOL_VERSION: &str = "2024-11-05";

/// Errors produced while reading or writing JSON-RPC frames.
#[derive(Debug, Error)]
pub enum FrameError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("failed to parse JSON-RPC frame: {0}")]
    Parse(#[from] serde_json::Error),
    #[error("frame is not a JSON object: {0}")]
    NotAnObject(String),
}

/// A JSON-RPC request object (`id` + `method` + optional `params`).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RpcRequest {
    pub id: i64,
    pub method: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub params: Option<Value>,
}

/// A JSON-RPC response object (`id` + `result` or `id` + `error`).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(untagged)]
pub enum RpcResponse {
    Result { id: i64, result: Value },
    Error { id: i64, error: RpcError },
}

impl RpcResponse {
    pub fn id(&self) -> i64 {
        match self {
            RpcResponse::Result { id, .. } | RpcResponse::Error { id, .. } => *id,
        }
    }
}

/// A JSON-RPC error object.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RpcError {
    pub code: i64,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<Value>,
}

/// A JSON-RPC notification object (no `id`, fire-and-forget).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RpcNotification {
    pub method: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub params: Option<Value>,
}

/// The parsed-invariant envelope that any single line of JSON-RPC can deserialize into.
///
/// This is the sum type used when reading frames off a stdio stream: a peer may send a
/// request, a response, or a notification.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(untagged)]
pub enum RpcMessage {
    Request(RpcRequest),
    Response(RpcResponse),
    Notification(RpcNotification),
}

impl RpcMessage {
    /// Serialize this message to a single newline-terminated JSON frame.
    pub fn to_frame(&self) -> Result<String, serde_json::Error> {
        let mut line = serde_json::to_string(self)?;
        line.push('\n');
        Ok(line)
    }

    /// Parse a single line into a message.
    pub fn from_frame(line: &[u8]) -> Result<Self, FrameError> {
        let value: Value = serde_json::from_slice(line)?;
        match value {
            Value::Object(_) => Ok(serde_json::from_value(value)?),
            other => Err(FrameError::NotAnObject(other.to_string())),
        }
    }
}

/// A line frame reader that yields one JSON-RPC message per non-empty, non-whitespace
/// line, tolerant of `\r\n` and blank lines that real MCP servers sometimes emit.
#[derive(Debug)]
pub struct FrameReader {
    buffer: Vec<u8>,
}

impl Default for FrameReader {
    fn default() -> Self {
        Self::new()
    }
}

impl FrameReader {
    pub fn new() -> Self {
        Self {
            buffer: Vec::with_capacity(1024),
        }
    }

    /// Feed raw bytes read from the pipe into the reader and return any complete frames.
    pub fn push(&mut self, bytes: &[u8]) -> Vec<Result<RpcMessage, FrameError>> {
        self.buffer.extend_from_slice(bytes);
        let mut frames = Vec::new();
        loop {
            match self.next_line_offset() {
                Some(newline) => {
                    let line: Vec<u8> = self.buffer.drain(..=newline).collect();
                    let trimmed = trim_buf(&line);
                    if trimmed.is_empty() {
                        continue;
                    }
                    frames.push(RpcMessage::from_frame(trimmed));
                }
                None => break,
            }
        }
        frames
    }

    fn next_line_offset(&self) -> Option<usize> {
        self.buffer.iter().position(|&b| b == b'\n')
    }
}

fn trim_buf(buf: &[u8]) -> &[u8] {
    let mut start = 0;
    let mut end = buf.len();
    while start < end && (buf[start] == b' ' || buf[start] == b'\t' || buf[start] == b'\r') {
        start += 1;
    }
    while end > start && (buf[end - 1] == b' ' || buf[end - 1] == b'\t' || buf[end - 1] == b'\r') {
        end -= 1;
    }
    &buf[start..end]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn request_round_trips() {
        let msg = RpcMessage::Request(RpcRequest {
            id: 1,
            method: "initialize".to_string(),
            params: Some(serde_json::json!({"protocolVersion": "2024-11-05"})),
        });
        let frame = msg.to_frame().unwrap();
        let parsed = RpcMessage::from_frame(frame.trim_end().as_bytes()).unwrap();
        assert_eq!(parsed, msg);
        assert!(frame.ends_with('\n'));
    }

    #[test]
    fn response_result_round_trips() {
        let msg = RpcMessage::Response(RpcResponse::Result {
            id: 2,
            result: serde_json::json!({"serverInfo": {"name": "x"}}),
        });
        let frame = msg.to_frame().unwrap();
        let parsed = RpcMessage::from_frame(frame.trim_end().as_bytes()).unwrap();
        assert_eq!(parsed, msg);
        assert_eq!(parsed.message_id(), Some(2));
    }

    #[test]
    fn response_error_round_trips() {
        let msg = RpcMessage::Response(RpcResponse::Error {
            id: 3,
            error: RpcError {
                code: -32601,
                message: "method not found".to_string(),
                data: None,
            },
        });
        let frame = msg.to_frame().unwrap();
        let parsed = RpcMessage::from_frame(frame.trim_end().as_bytes()).unwrap();
        assert_eq!(parsed, msg);
    }

    #[test]
    fn notification_round_trips() {
        let msg = RpcMessage::Notification(RpcNotification {
            method: "notifications/initialized".to_string(),
            params: None,
        });
        let frame = msg.to_frame().unwrap();
        let parsed = RpcMessage::from_frame(frame.trim_end().as_bytes()).unwrap();
        assert_eq!(parsed, msg);
        assert_eq!(parsed.message_id(), None);
    }

    #[test]
    fn frame_reader_handles_multiple_and_partial_lines() {
        let mut reader = FrameReader::new();
        let msg1 = RpcMessage::Request(RpcRequest {
            id: 1,
            method: "tools/list".to_string(),
            params: None,
        });
        let mut all = msg1.to_frame().unwrap();
        all.push_str(&msg1.to_frame().unwrap());

        // Feed a partial chunk then the remainder.
        let split = 7;
        let mut frames = reader.push(all[..split].as_bytes());
        assert!(frames.is_empty(), "partial line should not yield a frame yet");
        frames.extend(reader.push(all[split..].as_bytes()));
        assert_eq!(frames.len(), 2);
        assert_eq!(frames[0].as_ref().unwrap().message_id(), Some(1));
    }

    #[test]
    fn frame_reader_skips_blank_lines_and_crlf() {
        let mut reader = FrameReader::new();
        let msg = RpcMessage::Notification(RpcNotification {
            method: "notifications/initialized".to_string(),
            params: None,
        });
        reader.push(b"\n\r\n");
        let frames = reader.push(&msg.to_frame().unwrap().replace('\n', "\r\n").into_bytes());
        assert_eq!(frames.len(), 1);
        assert_eq!(frames[0].as_ref().unwrap(), &msg);
    }

    #[test]
    fn non_object_frame_is_rejected() {
        let mut reader = FrameReader::new();
        let frames = reader.push(b"[1, 2, 3]\n{\"foo\": 1}\n");
        assert_eq!(frames.len(), 2);
        // A JSON array is not a valid JSON-RPC object.
        assert!(matches!(frames[0], Err(FrameError::NotAnObject(_))));
        // A JSON object that is not a well-formed RPC message is a parse failure.
        assert!(matches!(frames[1], Err(FrameError::Parse(_))));
    }

    #[test]
    fn request_to_frame_preserves_id_and_method() {
        let msg = RpcMessage::Request(RpcRequest {
            id: 42,
            method: "tools/call".to_string(),
            params: Some(serde_json::json!({"name": "read_file", "arguments": {"path": "/tmp"}})),
        });
        let parsed: Value = serde_json::from_str(msg.to_frame().unwrap().trim_end()).unwrap();
        assert_eq!(parsed["id"], 42);
        assert_eq!(parsed["method"], "tools/call");
        assert_eq!(parsed["params"]["name"], "read_file");
    }
}

/// Internal helper to extract the `id` of a parsed message for request matching.
#[cfg(test)]
impl RpcMessage {
    fn message_id(&self) -> Option<i64> {
        match self {
            RpcMessage::Request(RpcRequest { id, .. }) => Some(*id),
            RpcMessage::Response(resp) => Some(resp.id()),
            RpcMessage::Notification(_) => None,
        }
    }
}
