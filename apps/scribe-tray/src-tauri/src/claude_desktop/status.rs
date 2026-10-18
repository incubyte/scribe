use std::path::Path;

use serde::Serialize;

use super::location::ClaudeDesktopConfig;
use super::merge::scribe_server_in;
use crate::mcp_config::scribe_server_entry;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ConnectionStatus {
    NotInstalled,
    NotConnected,
    Outdated,
    Connected,
}

#[must_use]
pub fn connection_status(config: &ClaudeDesktopConfig, server_command: &Path) -> ConnectionStatus {
    if !config.is_installed() {
        return ConnectionStatus::NotInstalled;
    }
    match config.read() {
        Ok(Some(existing)) => status_of(&existing, server_command),
        Ok(None) | Err(_) => ConnectionStatus::NotConnected,
    }
}

fn status_of(existing: &str, server_command: &Path) -> ConnectionStatus {
    match scribe_server_in(existing) {
        None => ConnectionStatus::NotConnected,
        Some(entry) if entry == scribe_server_entry(server_command) => ConnectionStatus::Connected,
        Some(_) => ConnectionStatus::Outdated,
    }
}
