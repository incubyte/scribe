use std::fs;
use std::path::{Path, PathBuf};

use scribe_tray::claude_desktop::{ClaudeDesktopConfig, ConnectOutcome, connect};
use tempfile::tempdir;

const SCRIBE_PATH: &str = "/Applications/Scribe.app/Contents/MacOS/scribe-mcp";
const CONFIG_FILE: &str = "claude_desktop_config.json";
const BACKUP_FILE: &str = "claude_desktop_config.json.scribe-backup";
const STAGING_FILE: &str = "claude_desktop_config.json.scribe-tmp";

fn connect_in(folder: &Path) -> ConnectOutcome {
    connect(
        &ClaudeDesktopConfig::in_folder(folder.to_path_buf()),
        Path::new(SCRIBE_PATH),
    )
}

fn claude_folder(root: &Path) -> PathBuf {
    let folder = root.join("Claude");
    fs::create_dir(&folder).unwrap();
    folder
}

#[test]
fn a_missing_claude_folder_reports_claude_desktop_is_not_installed_and_creates_nothing() {
    let root = tempdir().unwrap();
    let folder = root.path().join("Claude");

    let outcome = connect_in(&folder);

    assert_eq!(outcome, ConnectOutcome::ClaudeNotInstalled);
    assert!(!folder.exists());
}

#[test]
fn a_claude_folder_without_a_config_gets_a_new_config_containing_scribe() {
    let root = tempdir().unwrap();
    let folder = claude_folder(root.path());

    let outcome = connect_in(&folder);

    let written = fs::read_to_string(folder.join(CONFIG_FILE)).unwrap();
    assert_eq!(outcome, ConnectOutcome::Connected);
    assert!(written.contains(SCRIBE_PATH));
    assert!(!folder.join(BACKUP_FILE).exists());
}

#[test]
fn connecting_backs_up_the_previous_config_first() {
    let root = tempdir().unwrap();
    let folder = claude_folder(root.path());
    let previous = r#"{"theme":"dark"}"#;
    fs::write(folder.join(CONFIG_FILE), previous).unwrap();

    let outcome = connect_in(&folder);

    assert_eq!(outcome, ConnectOutcome::Connected);
    assert_eq!(
        fs::read_to_string(folder.join(BACKUP_FILE)).unwrap(),
        previous
    );
    assert!(
        fs::read_to_string(folder.join(CONFIG_FILE))
            .unwrap()
            .contains("\"theme\": \"dark\"")
    );
}

#[test]
fn no_staging_file_is_left_behind_after_connecting() {
    let root = tempdir().unwrap();
    let folder = claude_folder(root.path());

    connect_in(&folder);

    assert!(!folder.join(STAGING_FILE).exists());
}

#[test]
fn connecting_twice_leaves_the_file_untouched_and_makes_no_backup_the_second_time() {
    let root = tempdir().unwrap();
    let folder = claude_folder(root.path());
    connect_in(&folder);
    let after_first = fs::read_to_string(folder.join(CONFIG_FILE)).unwrap();

    let outcome = connect_in(&folder);

    assert_eq!(outcome, ConnectOutcome::AlreadyConnected);
    assert_eq!(
        fs::read_to_string(folder.join(CONFIG_FILE)).unwrap(),
        after_first
    );
    assert!(!folder.join(BACKUP_FILE).exists());
}

#[test]
fn an_unreadable_config_is_left_exactly_as_it_was() {
    let root = tempdir().unwrap();
    let folder = claude_folder(root.path());
    let broken = "{ \"mcpServers\": oops";
    fs::write(folder.join(CONFIG_FILE), broken).unwrap();

    let outcome = connect_in(&folder);

    assert_eq!(outcome, ConnectOutcome::ConfigUnreadable);
    assert_eq!(
        fs::read_to_string(folder.join(CONFIG_FILE)).unwrap(),
        broken
    );
    assert!(!folder.join(BACKUP_FILE).exists());
    assert!(!folder.join(STAGING_FILE).exists());
}

#[cfg(unix)]
#[test]
fn a_replaced_config_keeps_the_permissions_of_the_original() {
    use std::os::unix::fs::PermissionsExt;
    let root = tempdir().unwrap();
    let folder = claude_folder(root.path());
    fs::write(folder.join(CONFIG_FILE), "{}").unwrap();
    fs::set_permissions(folder.join(CONFIG_FILE), fs::Permissions::from_mode(0o640)).unwrap();

    connect_in(&folder);

    let mode = fs::metadata(folder.join(CONFIG_FILE))
        .unwrap()
        .permissions()
        .mode();
    assert_eq!(mode & 0o777, 0o640);
}

#[cfg(unix)]
#[test]
fn a_new_config_is_private_to_the_owner() {
    use std::os::unix::fs::PermissionsExt;
    let root = tempdir().unwrap();
    let folder = claude_folder(root.path());

    connect_in(&folder);

    let mode = fs::metadata(folder.join(CONFIG_FILE))
        .unwrap()
        .permissions()
        .mode();
    assert_eq!(mode & 0o777, 0o600);
}
