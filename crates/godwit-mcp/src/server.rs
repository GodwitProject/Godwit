//! An MCP **server** that exposes Godwit as a tool to external MCP clients.
//!
//! The server speaks JSON-RPC 2.0 over stdio (newline-delimited frames) and implements
//! the subset of MCP that lets an external MCP client discover and invoke the Godwit
//! gateway:
//!
//! * `initialize` — negotiate the protocol version and advertise the `tools` capability.
//! * `tools/list` — advertise the `godwit_chat` tool.
//! * `tools/call` — forward the call to the Godwit HTTP proxy `/v1/chat/completions`.
//! * `notifications/initialized` — acknowledged (no-op).
//!
//! It is designed to be run as a stdio subprocess (e.g. from an external client that uses
//! Godwit the same way it uses any MCP tool server).
use crate::jsonrpc::{
    FrameError, RpcMessage, RpcNotification, RpcRequest, RpcResponse, RpcError,
};
use serde_json::{json, Value};
use thiserror::Error;

/// Errors surfaced by the MCP server.
#[derive(Debug, Error)]
pub enum McpServerError {
    #[error("MCP frame error: {0}")]
    Frame(#[from] FrameError),
    #[error("MCP I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("could not reach Godwit gateway: {0}")]
    Gateway(String),
}

/// Configuration for the Godwit gateway the MCP server forwards to.
#[derive(Debug, Clone)]
pub struct GodwitGatewayConfig {
    /// Base URL of the Godwit HTTP proxy, e.g. `http://localhost:3000`.
    pub base_url: String,
    /// The API key to authenticate against Godwit.
    pub api_key: String,
    /// Default model to use when a `godwit_chat` call omits `model`.
    pub default_model: String,
}

impl GodwitGatewayConfig {
    pub fn new(base_url: impl Into<String>, api_key: impl Into<String>, default_model: impl Into<String>) -> Self {
        Self {
            base_url: base_url.into(),
            api_key: api_key.into(),
            default_model: default_model.into(),
        }
    }
}

/// The MCP server state.
pub struct McpServer {
    gateway: GodwitGatewayConfig,
    http: reqwest::Client,
}

impl McpServer {
    pub fn new(gateway: GodwitGatewayConfig) -> Self {
        let http = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(120))
            .build()
            .expect("build reqwest client");
        Self { gateway, http }
    }

    /// Run the server loop over a built-in line reader/writer (default: process stdio).
    /// This is a thin wrapper around [`McpServer::serve`] for the common case.
    pub async fn run_stdio(self) -> Result<(), McpServerError> {
        let reader = tokio::io::BufReader::new(tokio::io::stdin());
        let writer = tokio::io::stdout();
        self.serve(reader, writer).await
    }

    /// Serve MCP JSON-RPC requests read from `reader`, writing responses to `writer`.
    /// Abstracted over generic async read/write so unit tests can drive the server with
    /// in-memory streams.
    pub async fn serve<R, W>(
        &self,
        mut reader: R,
        mut writer: W,
    ) -> Result<(), McpServerError>
    where
        R: tokio::io::AsyncBufRead + Unpin,
        W: tokio::io::AsyncWrite + Unpin,
    {
        use tokio::io::{AsyncBufReadExt, AsyncWriteExt};
        let mut line = String::new();
        loop {
            line.clear();
            let n = reader.read_line(&mut line).await?;
            if n == 0 {
                break; // EOF — client disconnected.
            }
            if line.trim().is_empty() {
                continue;
            }
            let message = RpcMessage::from_frame(line.trim_end().as_bytes())?;
            match message {
                RpcMessage::Request(req) => {
                    let response = self.handle_request(&req).await;
                    let mut frame = serde_json::to_string(&response)
                        .map_err(|e| McpServerError::Gateway(e.to_string()))?;
                    frame.push('\n');
                    writer.write_all(frame.as_bytes()).await?;
                    writer.flush().await?;
                }
                RpcMessage::Notification(notif) => {
                    self.handle_notification(&notif).await;
                }
                // A peer should not send us a response; ignore it.
                RpcMessage::Response(_) => {}
            }
        }
        Ok(())
    }

    /// Handle a single request and produce the corresponding response object.
    async fn handle_request(&self, req: &RpcRequest) -> RpcResponse {
        let result = match req.method.as_str() {
            "initialize" => self.handle_initialize(req).await,
            "tools/list" => self.handle_tools_list(req).await,
            "tools/call" => self.handle_tools_call(req).await,
            other => {
                return RpcResponse::Error {
                    id: req.id,
                    error: RpcError {
                        code: -32601,
                        message: format!("method not found: {other}"),
                        data: None,
                    },
                };
            }
        };
        match result {
            Ok(value) => RpcResponse::Result { id: req.id, result: value },
            Err((code, message)) => RpcResponse::Error {
                id: req.id,
                error: RpcError {
                    code,
                    message,
                    data: None,
                },
            },
        }
    }

    async fn handle_initialize(&self, req: &RpcRequest) -> Result<Value, (i64, String)> {
        grant_initialize(&req.params)?;
        Ok(json!({
            "protocolVersion": crate::jsonrpc::MCP_PROTOCOL_VERSION,
            "capabilities": { "tools": {} },
            "serverInfo": { "name": "godwit-mcp", "version": env!("CARGO_PKG_VERSION") },
            "instructions": "Godwit MCP server. Use the godwit_chat tool to call a Godwit model.",
        }))
    }

    async fn handle_tools_list(&self, req: &RpcRequest) -> Result<Value, (i64, String)> {
        let _ = req;
        Ok(json!({
            "tools": [
                {
                    "name": "godwit_chat",
                    "description": "Send a chat completion request to a Godwit model. Takes a `model` (optional) and a `messages` array of OpenAI-style chat messages.",
                    "inputSchema": {
                        "type": "object",
                        "properties": {
                            "model": { "type": "string", "description": "Godwit model identifier (defaults to configured default)" },
                            "messages": {
                                "type": "array",
                                "items": {
                                    "type": "object",
                                    "properties": {
                                        "role": { "type": "string" },
                                        "content": { "type": "string" },
                                        "name": { "type": "string" }
                                    },
                                    "required": ["role", "content"],
                                    "additionalProperties": true
                                },
                                "description": "OpenAI-style chat messages"
                            }
                        },
                        "required": ["messages"]
                    }
                }
            ]
        }))
    }

    async fn handle_tools_call(&self, req: &RpcRequest) -> Result<Value, (i64, String)> {
        let name = req
            .params
            .as_ref()
            .and_then(|p| p.get("name"))
            .and_then(|n| n.as_str())
            .unwrap_or_default();
        if name != "godwit_chat" {
            return Err((
                -32602,
                format!("unknown tool '{name}' (only 'godwit_chat' is available)"),
            ));
        }
        let args = req
            .params
            .as_ref()
            .and_then(|p| p.get("arguments"))
            .ok_or_else(|| (-32602, "missing 'arguments'".to_string()))?;
        let model = args
            .get("model")
            .and_then(|m| m.as_str())
            .unwrap_or(&self.gateway.default_model)
            .to_string();
        let messages = args
            .get("messages")
            .cloned()
            .ok_or_else(|| (-32602, "missing 'messages' argument".to_string()))?;

        let body = json!({ "model": model, "messages": messages });
        let url = format!("{}/v1/chat/completions", self.gateway.base_url.trim_end_matches('/'));
        let response = self
            .http
            .post(&url)
            .header("Authorization", format!("Bearer {}", self.gateway.api_key))
            .json(&body)
            .send()
            .await
            .map_err(|e| {
                (
                    -32603,
                    format!("gateway request failed: {e}"),
                )
            })?;
        if !response.status().is_success() {
            let status = response.status().as_u16();
            let text = response.text().await.unwrap_or_default();
            return Err((
                -32603,
                format!("gateway returned {status}: {text}"),
            ));
        }
        let completion: Value = response
            .json()
            .await
            .map_err(|e| (-32603, format!("failed to parse gateway response: {e}")))?;
        // Render the first choice's message content as the tool result.
        let text = completion
            .pointer("/choices/0/message/content")
            .and_then(|c| c.as_str())
            .unwrap_or("(empty response)");
        Ok(json!({
            "content": [{ "type": "text", "text": text }],
            "isError": false,
        }))
    }

    async fn handle_notification(&self, notif: &RpcNotification) {
        match notif.method.as_str() {
            "notifications/initialized" => {
                tracing::debug!("MCP client initialized");
            }
            other => {
                tracing::debug!(method = %other, "ignored MCP notification");
            }
        }
    }
}

/// Validate an `initialize` request's protocol version. Returns `Ok(())` (accepting any
/// current version) or an error tuple for the official `-32602` invalid-params response.
fn grant_initialize(params: &Option<Value>) -> Result<(), (i64, String)> {
    let version = params
        .as_ref()
        .and_then(|p| p.get("protocolVersion"))
        .and_then(|v| v.as_str())
        .unwrap_or_default();
    if version.is_empty() {
        return Err((-32602, "missing protocolVersion".to_string()));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_gateway() -> GodwitGatewayConfig {
        GodwitGatewayConfig::new("http://localhost:39999", "test-key", "test-model")
    }

    async fn run_server_with_input(server: &McpServer, input: &str) -> String {
        let mut reader = tokio::io::BufReader::new(input.as_bytes());
        let mut writer = Vec::new();
        // Reaching into serve is fine, but we need to stop after input is consumed; the
        // loop exits on EOF.
        let _ = server.serve(&mut reader, &mut writer).await;
        String::from_utf8_lossy(&writer).to_string()
    }

    #[tokio::test]
    async fn initialize_returns_server_info() {
        let server = McpServer::new(test_gateway());
        let input = serde_json::json!({
            "jsonrpc": "2.0", "id": 1, "method": "initialize",
            "params": { "protocolVersion": "2024-11-05", "capabilities": {}, "clientInfo": {"name":"c","version":"1"} }
        }).to_string();
        let out = run_server_with_input(&server, &format!("{input}\n")).await;
        let lines: Vec<&str> = out.lines().collect();
        assert_eq!(lines.len(), 1);
        let v: Value = serde_json::from_str(lines[0]).unwrap();
        assert_eq!(v["id"], 1);
        assert_eq!(v["result"]["serverInfo"]["name"], "godwit-mcp");
        assert!(v["result"]["capabilities"]["tools"].is_object());
    }

    #[tokio::test]
    async fn initialize_missing_version_is_error() {
        let server = McpServer::new(test_gateway());
        let input = serde_json::json!({
            "jsonrpc": "2.0", "id": 5, "method": "initialize",
            "params": { "capabilities": {} }
        }).to_string();
        let out = run_server_with_input(&server, &format!("{input}\n")).await;
        let v: Value = serde_json::from_str(out.trim()).unwrap();
        assert!(v["error"].is_object());
        assert_eq!(v["error"]["code"], -32602);
    }

    #[tokio::test]
    async fn tools_list_advertises_godwit_chat() {
        let server = McpServer::new(test_gateway());
        let input = serde_json::json!({"jsonrpc":"2.0","id":2,"method":"tools/list"}).to_string();
        let out = run_server_with_input(&server, &format!("{input}\n")).await;
        let v: Value = serde_json::from_str(out.trim()).unwrap();
        let tools = v["result"]["tools"].as_array().unwrap();
        assert_eq!(tools.len(), 1);
        assert_eq!(tools[0]["name"], "godwit_chat");
        assert_eq!(tools[0]["inputSchema"]["required"][0], "messages");
    }

    #[tokio::test]
    async fn unknown_method_returns_error() {
        let server = McpServer::new(test_gateway());
        let input = serde_json::json!({"jsonrpc":"2.0","id":9,"method":"bogus"}).to_string();
        let out = run_server_with_input(&server, &format!("{input}\n")).await;
        let v: Value = serde_json::from_str(out.trim()).unwrap();
        assert_eq!(v["error"]["code"], -32601);
    }

    #[tokio::test]
    async fn tools_call_unknown_tool_is_error() {
        let server = McpServer::new(test_gateway());
        let input = serde_json::json!({
            "jsonrpc":"2.0","id":3,"method":"tools/call",
            "params": {"name":"nope","arguments":{}}
        }).to_string();
        let out = run_server_with_input(&server, &format!("{input}\n")).await;
        let v: Value = serde_json::from_str(out.trim()).unwrap();
        assert_eq!(v["error"]["code"], -32602);
        assert!(v["error"]["message"].to_string().contains("unknown tool"));
    }

    #[tokio::test]
    async fn tools_call_missing_messages_is_error() {
        let server = McpServer::new(test_gateway());
        let input = serde_json::json!({
            "jsonrpc":"2.0","id":4,"method":"tools/call",
            "params": {"name":"godwit_chat","arguments":{"model":"x"}}
        }).to_string();
        let out = run_server_with_input(&server, &format!("{input}\n")).await;
        let v: Value = serde_json::from_str(out.trim()).unwrap();
        assert_eq!(v["error"]["code"], -32602);
    }

    #[tokio::test]
    async fn multiple_requests_in_one_stream() {
        let server = McpServer::new(test_gateway());
        let init = serde_json::json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2024-11-05","capabilities":{},"clientInfo":{"name":"c","version":"1"}}}).to_string();
        let list = serde_json::json!({"jsonrpc":"2.0","id":2,"method":"tools/list"}).to_string();
        let input = format!("{init}\n{list}\n");
        let out = run_server_with_input(&server, &input).await;
        let lines: Vec<&str> = out.lines().collect();
        assert_eq!(lines.len(), 2);
        let v1: Value = serde_json::from_str(lines[0]).unwrap();
        let v2: Value = serde_json::from_str(lines[1]).unwrap();
        assert_eq!(v1["id"], 1);
        assert_eq!(v2["id"], 2);
        assert!(v2["result"]["tools"].is_array());
    }
}
