use std::fs;
use std::path::{Path, PathBuf};

use scribe_tray::claude_desktop::{ClaudeDesktopConfig, ConnectionStatus, connection_status};
use tempfile::tempdir;

const SCRIBE_PATH: &str = "/Applications/Scribe.app/Contents/MacOS/scribe-mcp";
const CONFIG_FILE: &str = "claude_desktop_config.json";

fn status_in(folder: &Path) -> ConnectionStatus {
    connection_status(
        &ClaudeDesktopConfig::in_folder(folder.to_path_buf()),
        Path::new(SCRIBE_PATH),
    )
}

fn claude_folder_with(root: &Path, config: Option<&str>) -> PathBuf {
    let folder = root.join("Claude");
    fs::create_dir(&folder).unwrap();
    if let Some(contents) = config {
        fs::write(folder.join(CONFIG_FILE), contents).unwrap();
    }
    folder
}

#[test]
fn a_missing_claude_folder_means_claude_desktop_is_not_installed() {
    let root = tempdir().unwrap();

    assert_eq!(
        status_in(&root.path().join("Claude")),
        ConnectionStatus::NotInstalled
    );
}

#[test]
fn claude_desktop_without_a_config_is_not_connected() {
    let root = tempdir().unwrap();

    assert_eq!(
        status_in(&claude_folder_with(root.path(), None)),
        ConnectionStatus::NotConnected
    );
}

#[test]
fn a_config_with_only_other_servers_is_not_connected() {
    let root = tempdir().unwrap();
    let config = r#"{"mcpServers":{"postgres":{"command":"pg-mcp"}}}"#;

    assert_eq!(
        status_in(&claude_folder_with(root.path(), Some(config))),
        ConnectionStatus::NotConnected
    );
}

#[test]
fn a_config_pointing_at_this_scribe_is_connected() {
    let root = tempdir().unwrap();
    let config = format!(r#"{{"mcpServers":{{"scribe":{{"command":"{SCRIBE_PATH}"}}}}}}"#);

    assert_eq!(
        status_in(&claude_folder_with(root.path(), Some(&config))),
        ConnectionStatus::Connected
    );
}

#[test]
fn a_config_pointing_at_an_old_scribe_location_is_outdated() {
    let root = tempdir().unwrap();
    let config = r#"{"mcpServers":{"scribe":{"command":"/old/build/scribe-mcp"}}}"#;

    assert_eq!(
        status_in(&claude_folder_with(root.path(), Some(config))),
        ConnectionStatus::Outdated
    );
}

#[test]
fn a_config_scribe_cannot_read_is_treated_as_not_connected() {
    let root = tempdir().unwrap();

    assert_eq!(
        status_in(&claude_folder_with(root.path(), Some("{ not json"))),
        ConnectionStatus::NotConnected
    );
}
