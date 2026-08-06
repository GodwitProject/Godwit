//! Minimal MCP client transport over a subprocess's stdio.
//!
//! A subprocess is spawned (per the [`McpServerConfig`]), and the client speaks JSON-RPC
//! 2.0 over its stdin/stdout using newline-delimited frames (see [`crate::jsonrpc`]).
//! This module performs the MCP `initialize` handshake and exposes the two operations the
//! rest of Godwit needs:
//!
//! * [`McpClient::list_tools`] — `tools/list`, returning raw MCP tool definitions.
//! * [`McpClient::call_tool`] — `tools/call`, returning the server's text result.
//!
//! The transport is abstracted behind the [`Transport`] trait so unit tests can drive the
//! client against an in-memory duplex pipe instead of a real subprocess.
use crate::config::McpServerConfig;
use crate::jsonrpc::{FrameError, RpcMessage, RpcNotification, RpcRequest, RpcResponse};
use crate::tool::McpTool;
use serde_json::Value;
use thiserror::Error;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};

/// Errors surfaced by the MCP client.
#[derive(Debug, Error)]
pub enum McpClientError {
    #[error("failed to spawn MCP server '{name}': {source}")]
    Spawn {
        name: String,
        #[source]
        source: std::io::Error,
    },
    #[error("MCP I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("MCP frame error: {0}")]
    Frame(#[from] FrameError),
    #[error("MCP serialization error: {0}")]
    Serialize(#[from] serde_json::Error),
    #[error("MCP server responded with an error for '{method}': {message}")]
    Remote {
        method: String,
        code: i64,
        message: String,
    },
    #[error("MCP server closed the connection while awaiting a response to '{method}'")]
    Closed { method: String },
    #[error("MCP server sent a response with an unexpected id (expected {expected}, got {actual})")]
    UnexpectedResponseId { expected: i64, actual: i64 },
    #[error("MCP call '{name}' returned no text content")]
    EmptyToolResult { name: String },
}

/// A byte-stream transport: write a raw frame, read a raw line. Implementations must be
/// `Send` so the client can live on the async runtime and be shared across requests.
pub trait Transport: Send {
    fn write_frame(&mut self, frame: &str) -> impl std::future::Future<Output = std::io::Result<()>> + Send;
    fn read_frame(&mut self) -> impl std::future::Future<Output = std::io::Result<String>> + Send;
}

/// A transport over a running subprocess's stdin/stdout.
pub struct StdioTransport {
    stdin: tokio::process::ChildStdin,
    stdout: BufReader<tokio::process::ChildStdout>,
}

impl Transport for StdioTransport {
    async fn write_frame(&mut self, frame: &str) -> std::io::Result<()> {
        self.stdin.write_all(frame.as_bytes()).await?;
        self.stdin.flush().await
    }
    async fn read_frame(&mut self) -> std::io::Result<String> {
        let mut line = String::new();
        loop {
            let n = self.stdout.read_line(&mut line).await?;
            if n == 0 {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::UnexpectedEof,
                    "MCP subprocess closed stdout",
                ));
            }
            let trimmed = line.trim();
            if trimmed.is_empty() {
                line.clear();
                continue;
            }
            return Ok(trimmed.to_string());
        }
    }
}

/// An MCP client connection. Spawns and owns a subprocess.
pub struct McpClient<T: Transport = StdioTransport> {
    transport: T,
    child: Option<tokio::process::Child>,
    next_id: i64,
}

impl McpClient<StdioTransport> {
    /// Spawn the configured MCP server subprocess and run the `initialize` handshake.
    pub async fn connect(config: &McpServerConfig) -> Result<Self, McpClientError> {
        let mut cmd = config.command_with_env();
        cmd.stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::inherit())
            .kill_on_drop(true);
        let mut child = cmd.spawn().map_err(|e| McpClientError::Spawn {
            name: config.name.clone(),
            source: e,
        })?;
        let stdin = child.stdin.take().expect("stdin piped");
        let stdout = BufReader::new(child.stdout.take().expect("stdout piped"));
        let transport = StdioTransport { stdin, stdout };
        let mut client = McpClient {
            transport,
            child: Some(child),
            next_id: 1,
        };
        client.initialize_handshake(config).await?;
        Ok(client)
    }

    /// Terminate the child process (also killed on drop).
    pub fn shutdown(&mut self) {
        if let Some(child) = &mut self.child {
            let _ = child.start_kill();
        }
    }
}

impl<T: Transport> McpClient<T> {
    /// Run the MCP `initialize` handshake: `initialize` request, then the
    /// `notifications/initialized` notification. Used by [`McpClient::connect`] and by
    /// tests that construct a client over a synthetic transport.
    pub async fn initialize_handshake(
        &mut self,
        config: &McpServerConfig,
    ) -> Result<(), McpClientError> {
        let id = self.next_id;
        self.next_id += 1;
        let params = serde_json::json!({
            "protocolVersion": crate::jsonrpc::MCP_PROTOCOL_VERSION,
            "capabilities": {},
            "clientInfo": { "name": "godwit", "version": env!("CARGO_PKG_VERSION") },
        });
        self.write_request(id, "initialize", Some(params)).await?;
        loop {
            let line = self
                .transport
                .read_frame()
                .await
                .map_err(|_| McpClientError::Closed {
                    method: "initialize".to_string(),
                })?;
            let message = RpcMessage::from_frame(line.as_bytes()).map_err(McpClientError::Frame)?;
            match message {
                RpcMessage::Response(r) => match r {
                    RpcResponse::Result { result, .. } => {
                        let server = result.get("serverInfo").cloned().unwrap_or_default();
                        tracing::info!(
                            server = %server,
                            "MCP initialize handshake completed for '{}'",
                            config.name
                        );
                        break;
                    }
                    RpcResponse::Error { error, .. } => {
                        return Err(McpClientError::Remote {
                            method: "initialize".to_string(),
                            code: error.code,
                            message: error.message,
                        });
                    }
                },
                _ => continue,
            }
        }
        self.write_notification("notifications/initialized").await?;
        Ok(())
    }

    /// The list of tools exposed by the connected MCP server.
    pub async fn list_tools(&mut self) -> Result<Vec<McpTool>, McpClientError> {
        let result = self.request("tools/list", Some(serde_json::json!({}))).await?;
        let tools: Vec<McpTool> = serde_json::from_value(result["tools"].clone())
            .map_err(|e| McpClientError::Remote {
                method: "tools/list".to_string(),
                code: -32603,
                message: format!("could not parse tools from server: {e}"),
            })?;
        Ok(tools)
    }

    /// Invoke a tool call on the connected MCP server and return its rendered text.
    pub async fn call_tool(
        &mut self,
        name: &str,
        arguments: Value,
    ) -> Result<String, McpClientError> {
        let params = serde_json::json!({
            "name": name,
            "arguments": arguments,
        });
        let result = self.request("tools/call", Some(params)).await?;
        let content = result
            .get("content")
            .and_then(|c| c.as_array())
            .cloned()
            .unwrap_or_default();
        let mut text = String::new();
        for item in content {
            if let Some(t) = item.get("text").and_then(|t| t.as_str()) {
                if !text.is_empty() {
                    text.push('\n');
                }
                text.push_str(t);
            }
        }
        if text.is_empty() {
            return Err(McpClientError::EmptyToolResult {
                name: name.to_string(),
            });
        }
        Ok(text)
    }

    /// Send a request and wait for the matching response, ignoring unsolicited
    /// notifications and requests that arrive in the meantime.
    async fn request(
        &mut self,
        method: &str,
        params: Option<Value>,
    ) -> Result<Value, McpClientError> {
        let id = self.next_id;
        self.next_id += 1;
        self.write_request(id, method, params).await?;
        loop {
            let line = self.transport.read_frame().await.map_err(|_| {
                McpClientError::Closed {
                    method: method.to_string(),
                }
            })?;
            let message = RpcMessage::from_frame(line.as_bytes()).map_err(McpClientError::Frame)?;
            match message {
                RpcMessage::Response(r) => match r {
                    RpcResponse::Result { id: rid, result } => {
                        if rid != id {
                            return Err(McpClientError::UnexpectedResponseId {
                                expected: id,
                                actual: rid,
                            });
                        }
                        return Ok(result);
                    }
                    RpcResponse::Error { error, .. } => {
                        return Err(McpClientError::Remote {
                            method: method.to_string(),
                            code: error.code,
                            message: error.message,
                        });
                    }
                },
                _ => continue,
            }
        }
    }

    async fn write_request(
        &mut self,
        id: i64,
        method: &str,
        params: Option<Value>,
    ) -> Result<(), McpClientError> {
        let msg = RpcMessage::Request(RpcRequest {
            id,
            method: method.to_string(),
            params,
        });
        let frame = msg.to_frame()?;
        self.transport.write_frame(&frame).await?;
        Ok(())
    }

    /// Write a fire-and-forget notification (no `id`).
    async fn write_notification(&mut self, method: &str) -> Result<(), McpClientError> {
        let msg = RpcMessage::Notification(RpcNotification {
            method: method.to_string(),
            params: None,
        });
        let frame = msg.to_frame()?;
        self.transport.write_frame(&frame).await?;
        Ok(())
    }

    /// Expose the next request id for tests.
    #[cfg(test)]
    pub fn next_request_id(&self) -> i64 {
        self.next_id
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::jsonrpc::FrameReader;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    /// A duplex transport that proxies to a peer `DuplexStream`. In the test, the fake
    /// MCP server reads from `peer_read` and writes to `peer_write`.
    struct DuplexTransport {
        read: tokio::io::DuplexStream,
        write: tokio::io::DuplexStream,
    }

    impl Transport for DuplexTransport {
        async fn write_frame(&mut self, frame: &str) -> std::io::Result<()> {
            self.write.write_all(frame.as_bytes()).await?;
            self.write.flush().await
        }
        async fn read_frame(&mut self) -> std::io::Result<String> {
            let mut buf = [0u8; 4096];
            let mut acc = Vec::new();
            loop {
                let n = self.read.read(&mut buf).await?;
                if n == 0 {
                    return Err(std::io::Error::new(
                        std::io::ErrorKind::UnexpectedEof,
                        "closed",
                    ));
                }
                acc.extend_from_slice(&buf[..n]);
                if acc.contains(&b'\n') {
                    break;
                }
            }
            Ok(String::from_utf8_lossy(&acc).trim().to_string())
        }
    }

    /// A fake MCP server that responds to initialize / tools/list / tools/call over a
    /// duplex pipe, ready to be spawned as the "server" side of the client test.
    async fn run_fake_server(mut rx: tokio::io::DuplexStream, mut tx: tokio::io::DuplexStream) {
        let mut frame_reader = FrameReader::new();
        let mut buf = [0u8; 4096];
        loop {
            let n = rx.read(&mut buf).await.unwrap();
            if n == 0 {
                break;
            }
            for msg in frame_reader.push(&buf[..n]) {
                let msg = msg.unwrap();
                if let RpcMessage::Request(req) = msg {
                    let response = match req.method.as_str() {
                        "initialize" => serde_json::json!({
                            "jsonrpc": "2.0", "id": req.id, "result": {
                                "protocolVersion": "2024-11-05",
                                "capabilities": { "tools": {} },
                                "serverInfo": { "name": "fake", "version": "1.0" },
                            }
                        }),
                        "tools/list" => serde_json::json!({
                            "jsonrpc": "2.0", "id": req.id, "result": {
                                "tools": [{
                                    "name": "echo",
                                    "description": "Echo input",
                                    "inputSchema": {
                                        "type": "object",
                                        "properties": { "text": { "type": "string" } },
                                        "required": ["text"],
                                    },
                                }]
                            }
                        }),
                        "tools/call" => {
                            let name = req
                                .params
                                .as_ref()
                                .and_then(|p| p["name"].as_str())
                                .unwrap_or("");
                            if name == "boom" {
                                serde_json::json!({
                                    "jsonrpc": "2.0", "id": req.id,
                                    "error": {"code": -32000, "message": "boom exploded"}
                                })
                            } else {
                                let text = req
                                    .params
                                    .as_ref()
                                    .and_then(|p| p["arguments"]["text"].as_str())
                                    .unwrap_or("?");
                                serde_json::json!({
                                    "jsonrpc": "2.0", "id": req.id, "result": {
                                        "content": [{"type": "text", "text": format!("echo:{text}")}],
                                        "isError": false,
                                    }
                                })
                            }
                        }
                        _ => serde_json::json!({
                            "jsonrpc": "2.0", "id": req.id,
                            "error": {"code": -32601, "message": "method not found"}
                        }),
                    };
                    let mut line = serde_json::to_string(&response).unwrap();
                    line.push('\n');
                    tx.write_all(line.as_bytes()).await.unwrap();
                    tx.flush().await.unwrap();
                }
            }
        }
    }

    fn pair() -> (McpClient<DuplexTransport>, tokio::io::DuplexStream, tokio::io::DuplexStream) {
        // client writes to tx_a, server reads from rx_a === tx_a
        let (client_write, server_read) = tokio::io::duplex(4096);
        let (server_write, client_read) = tokio::io::duplex(4096);
        let client = McpClient {
            transport: DuplexTransport {
                read: client_read,
                write: client_write,
            },
            child: None,
            next_id: 1,
        };
        (client, server_read, server_write)
    }

    #[tokio::test]
    async fn handshake_list_and_call_tools() {
        let (mut client, server_read, server_write) = pair();
        let server = tokio::spawn(run_fake_server(server_read, server_write));

        let config = McpServerConfig {
            name: "fake".to_string(),
            command: "true".to_string(),
            args: vec![],
            env: Default::default(),
        };
        client.initialize_handshake(&config).await.unwrap();

        let tools = client.list_tools().await.unwrap();
        assert_eq!(tools.len(), 1);
        assert_eq!(tools[0].name, "echo");
        assert_eq!(
            tools[0].input_schema["required"][0],
            "text"
        );

        let out = client.call_tool("echo", serde_json::json!({"text": "hi"})).await.unwrap();
        assert_eq!(out, "echo:hi");

        drop(client);
        server.await.unwrap();
    }

    #[tokio::test]
    async fn remote_error_is_surfaced() {
        let (mut client, server_read, server_write) = pair();
        let server = tokio::spawn(run_fake_server(server_read, server_write));
        let config = McpServerConfig {
            name: "fake".to_string(),
            command: "true".to_string(),
            args: vec![],
            env: Default::default(),
        };
        client.initialize_handshake(&config).await.unwrap();

        // A tool call whose server responds with a JSON-RPC error is surfaced as Remote.
        let res = client.call_tool("boom", serde_json::json!({})).await;
        match res {
            Err(McpClientError::Remote { code, message, .. }) => {
                assert_eq!(code, -32000);
                assert_eq!(message, "boom exploded");
            }
            Err(other) => panic!("expected Remote error, got {other:?}"),
            Ok(_) => panic!("expected error"),
        }

        drop(client);
        server.await.unwrap();
    }
}
