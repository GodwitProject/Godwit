//! Conversion between MCP tool definitions and Godwit's `godwit_core::Tool` type.
//!
//! MCP servers advertise tools via `tools/list` with a camelCase schema:
//!
//! ```json
//! { "name": "read_file", "description": "...", "inputSchema": { "type": "object", "properties": {...} } }
//! ```
//!
//! Godwit exposes OpenAI-style chat tool definitions to its own chat backends:
//!
//! ```json
//! { "type": "function", "function": { "name": "...", "description": "...", "parameters": {...} } }
//! ```
use godwit_core::Tool;
use serde::{Deserialize, Serialize};

/// The MCP `tool` object as returned by `tools/list`.
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct McpTool {
    pub name: String,
    pub description: Option<String>,
    pub input_schema: serde_json::Value,
}

impl McpTool {
    /// Convert this MCP tool into a `godwit_core::Tool`, namespacing it with the owning
    /// MCP server's prefix so that tool names stay unique across multiple servers and
    /// tool calls can be routed back to the correct server.
    pub fn to_core_tool(&self, server_name: &str) -> Tool {
        Tool {
            r#type: "function".to_string(),
            function: godwit_core::FunctionDefinition {
                name: format!("{server_name}__{}", self.name),
                description: self.description.clone(),
                parameters: Some(self.input_schema.clone()),
            },
        }
    }

    /// Convert a namespaced tool name back into the owning server name and bare tool
    /// name. Returns `None` if the name has no server prefix.
    pub fn split_namespaced_name<'a>(server_name: &'a str, tool_name: &'a str) -> Option<(&'a str, &'a str)> {
        let prefix = format!("{server_name}__");
        tool_name
            .strip_prefix(&prefix)
            .map(|bare| (server_name, bare))
    }
}

/// Convert a vector of MCP tools from one server into `godwit_core::Tool`s.
pub fn mcp_tools_to_core(server_name: &str, tools: &[McpTool]) -> Vec<Tool> {
    tools
        .iter()
        .map(|t| t.to_core_tool(server_name))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_mcp_tool() -> McpTool {
        McpTool {
            name: "read_file".to_string(),
            description: Some("Read a file from disk".to_string()),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": { "path": { "type": "string" } },
                "required": ["path"]
            }),
        }
    }

    #[test]
    fn mcp_tool_converts_to_core_tool_with_prefix() {
        let t = sample_mcp_tool();
        let core = t.to_core_tool("filesystem");
        assert_eq!(core.r#type, "function");
        assert_eq!(core.function.name, "filesystem__read_file");
        assert_eq!(
            core.function.description.as_deref(),
            Some("Read a file from disk")
        );
        assert_eq!(
            core.function.parameters.as_ref().unwrap()["required"][0],
            "path"
        );
    }

    #[test]
    fn converts_a_slice_of_mcp_tools() {
        let tools = vec![sample_mcp_tool(), sample_mcp_tool()];
        let core = mcp_tools_to_core("srv", &tools);
        assert_eq!(core.len(), 2);
        assert_eq!(core[0].function.name, "srv__read_file");
    }

    #[test]
    fn split_namespaced_name() {
        let (server, bare) = McpTool::split_namespaced_name("srv", "srv__read_file").unwrap();
        assert_eq!(server, "srv");
        assert_eq!(bare, "read_file");

        let (server2, bare2) = McpTool::split_namespaced_name("srv", "srv__a__b").unwrap();
        assert_eq!(server2, "srv");
        assert_eq!(bare2, "a__b");

        assert!(McpTool::split_namespaced_name("srv", "other__x").is_none());
    }

    #[test]
    fn core_tool_serializes_in_openai_shape() {
        let core = sample_mcp_tool().to_core_tool("srv");
        let json = serde_json::to_string(&core).unwrap();
        let v: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(v["type"], "function");
        assert_eq!(v["function"]["name"], "srv__read_file");
        assert!(v["function"]["parameters"].is_object());
    }
}
