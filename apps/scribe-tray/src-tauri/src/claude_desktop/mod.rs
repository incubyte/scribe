mod location;
mod merge;

use std::io;
use std::path::Path;

pub use location::ClaudeDesktopConfig;
pub use merge::{MergeResult, UnreadableConfig, merge_scribe_server};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConnectOutcome {
    Connected,
    AlreadyConnected,
    ClaudeNotInstalled,
    ConfigUnreadable,
    Failed,
}

#[must_use]
pub fn connect(config: &ClaudeDesktopConfig, server_command: &Path) -> ConnectOutcome {
    if !config.is_installed() {
        return ConnectOutcome::ClaudeNotInstalled;
    }
    match read_existing(config) {
        Ok(existing) => merge_outcome(config, existing.as_deref(), server_command),
        Err(outcome) => outcome,
    }
}

fn read_existing(config: &ClaudeDesktopConfig) -> Result<Option<String>, ConnectOutcome> {
    config.read().map_err(|error| {
        tracing::error!(kind = ?error.kind(), "could not read the Claude Desktop config");
        match error.kind() {
            io::ErrorKind::InvalidData => ConnectOutcome::ConfigUnreadable,
            _ => ConnectOutcome::Failed,
        }
    })
}

fn merge_outcome(
    config: &ClaudeDesktopConfig,
    existing: Option<&str>,
    server_command: &Path,
) -> ConnectOutcome {
    match merge_scribe_server(existing, server_command) {
        Ok(MergeResult::Unchanged) => ConnectOutcome::AlreadyConnected,
        Ok(MergeResult::Write(contents)) => write_outcome(config, &contents),
        Err(error) => {
            tracing::error!(%error, "the Claude Desktop config is not in a shape Scribe can update");
            ConnectOutcome::ConfigUnreadable
        }
    }
}

fn write_outcome(config: &ClaudeDesktopConfig, contents: &str) -> ConnectOutcome {
    match config.replace(contents) {
        Ok(()) => ConnectOutcome::Connected,
        Err(error) => {
            tracing::error!(kind = ?error.kind(), "could not write the Claude Desktop config");
            ConnectOutcome::Failed
        }
    }
}
