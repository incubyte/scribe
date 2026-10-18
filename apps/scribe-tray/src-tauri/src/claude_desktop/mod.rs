mod location;
mod merge;

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
    let existing = match config.read() {
        Ok(existing) => existing,
        Err(error) => {
            tracing::error!(kind = ?error.kind(), "could not read the Claude Desktop config");
            return ConnectOutcome::Failed;
        }
    };
    match merge_scribe_server(existing.as_deref(), server_command) {
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
