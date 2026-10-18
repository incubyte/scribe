use tauri::{AppHandle, Runtime};

use crate::claude_desktop::{ClaudeDesktopConfig, ConnectOutcome, connect};
use crate::mcp_config::bundled_mcp_path;
use crate::notifications;

pub fn connect_claude_desktop<R: Runtime>(app: &AppHandle<R>) {
    notifications::notify_claude_desktop(app, connect_outcome());
}

fn connect_outcome() -> ConnectOutcome {
    let server_command = match bundled_mcp_path() {
        Ok(path) => path,
        Err(error) => {
            tracing::error!(%error, "could not locate the bundled MCP server");
            return ConnectOutcome::Failed;
        }
    };
    match ClaudeDesktopConfig::default_location() {
        Some(config) => connect(&config, &server_command),
        None => ConnectOutcome::ClaudeNotInstalled,
    }
}
