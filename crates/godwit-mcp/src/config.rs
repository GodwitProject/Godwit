//! Configuration types for registering MCP servers.
//!
//! A `McpConfig` is intended to be nested in the top-level `AppConfig` under an
//! `mcp_servers` key, but it deserializes independently so it can be unit tested in
//! isolation. MCP servers are spawned as child processes speaking JSON-RPC over stdio.
use serde::{Deserialize, Serialize};

/// The top-level MCP configuration block (the `mcp_servers` list).
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub struct McpConfig {
    pub mcp_servers: Vec<McpServerConfig>,
}

impl McpConfig {
    /// Convenience constructor for programmatic tests and the registry.
    pub fn new(servers: Vec<McpServerConfig>) -> Self {
        Self {
            mcp_servers: servers,
        }
    }
}

/// A single MCP server to launch as a subprocess.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub struct McpServerConfig {
    pub name: String,
    pub command: String,
    #[serde(default = "default_args")]
    pub args: Vec<String>,
    /// Any extra environment variables to set for the child process (optional).
    #[serde(default)]
    pub env: std::collections::HashMap<String, String>,
}

impl Default for McpServerConfig {
    fn default() -> Self {
        Self {
            name: String::new(),
            command: String::new(),
            args: Vec::new(),
            env: std::collections::HashMap::new(),
        }
    }
}

fn default_args() -> Vec<String> {
    Vec::new()
}

impl McpServerConfig {
    /// A normalized unique key used to key tools exposed by this server.
    pub fn tool_prefix(&self) -> String {
        format!("{}__", self.name)
    }

    /// Returns a base command for spawning the child, embedding any configured env vars.
    pub fn command_with_env(&self) -> tokio::process::Command {
        let mut cmd = tokio::process::Command::new(&self.command);
        cmd.args(&self.args).stdin(std::process::Stdio::piped());
        for (k, v) in &self.env {
            cmd.env(k, v);
        }
        cmd
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_mcp_servers_from_yaml() {
        let yaml = r#"
mcp_servers:
  - name: filesystem
    command: npx
    args: ["-y", "@modelcontextprotocol/server-filesystem", "/tmp"]
  - name: git
    command: npx
    args: ["-y", "@modelcontextprotocol/server-git"]
"#;
        let config: McpConfig = serde_yaml::from_str(yaml).expect("parse yaml");
        assert_eq!(config.mcp_servers.len(), 2);
        assert_eq!(config.mcp_servers[0].name, "filesystem");
        assert_eq!(config.mcp_servers[0].command, "npx");
        assert_eq!(
            config.mcp_servers[0].args,
            vec![
                "-y",
                "@modelcontextprotocol/server-filesystem",
                "/tmp"
            ]
        );
        assert_eq!(config.mcp_servers[1].name, "git");
    }

    #[test]
    fn serializes_and_round_trips() {
        let config = McpConfig::new(vec![McpServerConfig {
            name: "fs".into(),
            command: "npx".into(),
            args: vec!["-y".into(), "server-fs".into()],
            env: Default::default(),
        }]);
        let json = serde_json::to_string(&config).unwrap();
        let parsed: McpConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.mcp_servers[0].name, "fs");
        assert_eq!(parsed.mcp_servers[0].args.len(), 2);
    }

    #[test]
    fn defaults_apply_when_fields_missing() {
        let yaml = "mcp_servers:\n  - name: bare\n    command: echo\n";
        let config: McpConfig = serde_yaml::from_str(yaml).unwrap();
        assert!(config.mcp_servers[0].args.is_empty());
        assert!(config.mcp_servers[0].env.is_empty());
        assert_eq!(config.mcp_servers[0].tool_prefix(), "bare__");
    }

    #[test]
    fn tool_prefix_embeds_name() {
        let c = McpServerConfig {
            name: "my-server".into(),
            command: "echo".into(),
            args: vec![],
            env: Default::default(),
        };
        assert_eq!(c.tool_prefix(), "my-server__");
    }
}
