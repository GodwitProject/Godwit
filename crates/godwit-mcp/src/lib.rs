//! `godwit-mcp` — Model Context Protocol (MCP) client and server support for Godwit.
//!
//! # Client
//!
//! [`McpRegistry`] loads MCP servers from configuration, spawns each as a child process
//! speaking JSON-RPC 2.0 over stdio ([`client::McpClient`]), converts each server's
//! `tools/list` output into `godwit_core::Tool` definitions ([`tool`]), and routes
//! `tools/call` invocations back to the owning server.
//!
//! # Server
//!
//! [`server::McpServer`] is a stdio JSON-RPC server that exposes Godwit's
//! `/v1/chat/completions` to external MCP clients via a `godwit_chat` tool.
//!
//! # Transport choice
//!
//! The stdio transport is hand-rolled (see [`jsonrpc`]): MCP frames are newline-delimited
//! JSON-RPC 2.0 objects. The maintained `rmcp` crate was considered and fetched
//! successfully, but a minimal self-contained implementation was preferred for the
//! narrow subset of the protocol Godwit needs (initialize / tools/list / tools/call), so
//! that both the client and server share one small, well-tested framing module and there
//! is no risk of a third-party API mismatch blocking a build.
pub mod client;
pub mod config;
pub mod jsonrpc;
pub mod registry;
pub mod server;
pub mod tool;

pub use crate::client::{McpClient, McpClientError};
pub use crate::config::{McpConfig, McpServerConfig};
pub use crate::registry::{McpError, McpRegistry};
pub use crate::server::{GodwitGatewayConfig, McpServer, McpServerError};
pub use crate::tool::McpTool;
