use std::io;
use std::path::{Path, PathBuf};

use serde_json::json;

const MCP_BINARY_NAME: &str = if cfg!(windows) {
    "scribe-mcp.exe"
} else {
    "scribe-mcp"
};

#[derive(Debug, thiserror::Error)]
pub enum McpConfigError {
    #[error("could not locate the running app")]
    ExecutablePath(#[source] io::Error),
    #[error("the bundled MCP server is missing at {0}")]
    BundledServerMissing(PathBuf),
}

pub fn bundled_mcp_path() -> Result<PathBuf, McpConfigError> {
    let beside_app = std::env::current_exe()
        .map_err(McpConfigError::ExecutablePath)?
        .with_file_name(MCP_BINARY_NAME);
    if beside_app.is_file() {
        Ok(beside_app)
    } else {
        Err(McpConfigError::BundledServerMissing(beside_app))
    }
}

#[must_use]
pub fn config_snippet(path: &Path) -> String {
    json!({
        "mcpServers": {
            "scribe": { "command": path.to_string_lossy() }
        }
    })
    .to_string()
}
