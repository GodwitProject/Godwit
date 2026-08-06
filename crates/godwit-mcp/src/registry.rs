//! The `McpRegistry` — a shared, thread-safe registry of MCP servers, their spawned
//! client connections, and the bridge between MCP tool definitions and Godwit chat
//! tool calls.
//!
//! This is the "seam" the API dispatch layer can consult:
//!
//! * [`McpRegistry::all_tools`] returns every MCP server's tools converted to
//!   `godwit_core::Tool` (namespaced with `<server>__<tool>`).
//! * [`McpRegistry::call_tool`] routes a call for a tool name (as a model would emit it)
//!   to the owning MCP server and returns the rendered text result.
//!
//! Client connections are spawned lazily on first use and cached. Any server that fails to
//! connect is skipped (and its failure logged) rather than taking the whole chat dispatch
//! down with it.
use crate::client::McpClient;
use crate::config::{McpConfig, McpServerConfig};
use crate::tool::McpTool;
use godwit_core::Tool;
use std::collections::HashMap;
use tokio::sync::Mutex;
use tracing::{info, warn};

/// A configured MCP server and (once connected) its spawned client.
struct ServerEntry {
    config: McpServerConfig,
    client: Option<McpClient>,
}

/// Thread-safe registry of MCP servers shared via `Arc<McpRegistry>`.
pub struct McpRegistry {
    servers: Mutex<HashMap<String, ServerEntry>>,
}

impl McpRegistry {
    /// Build an empty registry with no MCP servers.
    pub fn new() -> Self {
        Self {
            servers: Mutex::new(HashMap::new()),
        }
    }

    /// Build a registry from an [`McpConfig`]. Connections are established lazily.
    pub fn from_config(config: &McpConfig) -> Self {
        let mut servers = HashMap::new();
        for cfg in &config.mcp_servers {
            servers.insert(cfg.name.clone(), ServerEntry {
                config: cfg.clone(),
                client: None,
            });
        }
        Self {
            servers: Mutex::new(servers),
        }
    }

    /// The number of configured MCP servers.
    pub async fn server_count(&self) -> usize {
        self.servers.lock().await.len()
    }

    /// The [`McpConfig`] reconstructed from the current registry contents.
    pub async fn config(&self) -> McpConfig {
        let servers = self.servers.lock().await;
        let mut configs: Vec<McpServerConfig> = servers
            .values()
            .map(|e| e.config.clone())
            .collect();
        configs.sort_by(|a, b| a.name.cmp(&b.name));
        McpConfig::new(configs)
    }

    /// Establish (and cache) a connection to the named server, if not already connected.
    async fn ensure_connected(
        &self,
        name: &str,
        servers: &mut HashMap<String, ServerEntry>,
    ) -> Result<(), McpError> {
        if servers
            .get(name)
            .map(|e| e.client.is_some())
            .unwrap_or(false)
        {
            return Ok(());
        }
        let config = servers
            .get(name)
            .map(|e| e.config.clone())
            .ok_or_else(|| McpError::UnknownServer(name.to_string()))?;
        let client = McpClient::connect(&config).await?;
        info!(server = %name, "MCP server connected");
        if let Some(entry) = servers.get_mut(name) {
            entry.client = Some(client);
        }
        Ok(())
    }

    /// All MCP tools across every connected server, converted to `godwit_core::Tool`.
    pub async fn all_tools(&self) -> Vec<Tool> {
        let mut servers = self.servers.lock().await;
        let names: Vec<String> = servers.keys().cloned().collect();
        let mut out = Vec::new();
        for name in names {
            if let Err(e) = self.ensure_connected(&name, &mut servers).await {
                warn!(server = %name, error = %e, "skipping MCP server; tools unavailable");
                continue;
            }
            let client = servers.get_mut(&name).and_then(|e| e.client.as_mut());
            match client {
                Some(client) => match client.list_tools().await {
                    Ok(tools) => out.extend(crate::tool::mcp_tools_to_core(&name, &tools)),
                    Err(e) => warn!(server = %name, error = %e, "failed to list tools"),
                },
                None => {}
            }
        }
        out
    }

    /// Raw MCP tools for a single named server (used by tests / introspection).
    pub async fn tools_for(&self, name: &str) -> Result<Vec<McpTool>, McpError> {
        let mut servers = self.servers.lock().await;
        self.ensure_connected(name, &mut servers).await?;
        let client = servers
            .get_mut(name)
            .and_then(|e| e.client.as_mut())
            .ok_or_else(|| McpError::UnknownServer(name.to_string()))?;
        Ok(client.list_tools().await?)
    }

    /// Route a tool call (as emitted by a model, e.g. `filesystem__read_file`) to the
    /// owning MCP server and return the text result.
    pub async fn call_tool(
        &self,
        tool_name: &str,
        arguments: serde_json::Value,
    ) -> Result<String, McpError> {
        let (server_name, bare_name) = split_tool_name(tool_name)
            .filter(|(s, bare)| !s.is_empty() && !bare.is_empty())
            .ok_or_else(|| McpError::InvalidToolName(tool_name.to_string()))?;
        let mut servers = self.servers.lock().await;
        self.ensure_connected(server_name, &mut servers).await?;
        let client = servers
            .get_mut(server_name)
            .and_then(|e| e.client.as_mut())
            .ok_or_else(|| McpError::UnknownServer(server_name.to_string()))?;
        Ok(client.call_tool(bare_name, arguments).await?)
    }
}

impl Default for McpRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// Errors surfaced by [`McpRegistry`].
#[derive(Debug, thiserror::Error)]
pub enum McpError {
    #[error("unknown MCP server '{0}'")]
    UnknownServer(String),
    #[error("invalid MCP tool name '{0}' (expected 'server__tool')")]
    InvalidToolName(String),
    #[error("MCP client error: {0}")]
    Client(#[from] crate::client::McpClientError),
}

/// Split a namespaced tool name (`server__tool`) into (server, bare tool).
pub(crate) fn split_tool_name(tool_name: &str) -> Option<(&str, &str)> {
    tool_name.split_once("__")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn split_tool_name_works() {
        assert_eq!(split_tool_name("fs__read").unwrap(), ("fs", "read"));
        assert_eq!(
            split_tool_name("fs__read_file").unwrap(),
            ("fs", "read_file")
        );
        assert!(split_tool_name("noprefix").is_none());
    }

    #[tokio::test]
    async fn empty_registry_returns_no_tools() {
        let registry = McpRegistry::new();
        assert!(registry.all_tools().await.is_empty());
        assert_eq!(registry.server_count().await, 0);
    }

    #[tokio::test]
    async fn config_round_trips() {
        let config = McpConfig::new(vec![McpServerConfig {
            name: "fs".to_string(),
            command: "npx".to_string(),
            args: vec!["-y".to_string()],
            env: Default::default(),
        }]);
        let registry = McpRegistry::from_config(&config);
        assert_eq!(registry.server_count().await, 1);
        let back = registry.config().await;
        assert_eq!(back.mcp_servers.len(), 1);
        assert_eq!(back.mcp_servers[0].name, "fs");
    }

    #[tokio::test]
    async fn unknown_tool_call_returns_error() {
        let registry = McpRegistry::new();
        let result = registry.call_tool("nope__thing", serde_json::json!({})).await;
        assert!(matches!(result, Err(McpError::UnknownServer(_))));
    }

    #[tokio::test]
    async fn invalid_tool_name_returns_error() {
        let registry = McpRegistry::new();
        let result = registry.call_tool("no-prefix", serde_json::json!({})).await;
        assert!(matches!(result, Err(McpError::InvalidToolName(_))));
    }
}
