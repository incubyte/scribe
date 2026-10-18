use std::path::Path;

use scribe_tray::mcp_config::{config_snippet, scribe_server_entry};
use serde_json::Value;

fn command_in(snippet: &str) -> String {
    let parsed: Value = serde_json::from_str(snippet).unwrap();
    parsed["mcpServers"]["scribe"]["command"]
        .as_str()
        .unwrap()
        .to_owned()
}

#[test]
fn the_snippet_registers_scribe_with_the_absolute_path_of_the_bundled_server() {
    let snippet = config_snippet(Path::new(
        "/Applications/Scribe.app/Contents/MacOS/scribe-mcp",
    ));

    assert_eq!(
        snippet,
        r#"{"mcpServers":{"scribe":{"command":"/Applications/Scribe.app/Contents/MacOS/scribe-mcp"}}}"#
    );
}

#[test]
fn a_path_with_spaces_is_kept_intact() {
    let path = "/Users/Ada Lovelace/Apps/Scribe.app/Contents/MacOS/scribe-mcp";

    assert_eq!(command_in(&config_snippet(Path::new(path))), path);
}

#[test]
fn a_windows_path_with_backslashes_survives_json_escaping() {
    let path = r"C:\Program Files\Scribe\scribe-mcp.exe";

    let snippet = config_snippet(Path::new(path));

    assert_eq!(command_in(&snippet), path);
    assert!(snippet.contains(r"C:\\Program Files\\Scribe\\scribe-mcp.exe"));
}

#[test]
fn a_path_with_quotes_cannot_break_out_of_the_command_value() {
    let path = r#"/tmp/odd "name"/scribe-mcp"#;

    assert_eq!(command_in(&config_snippet(Path::new(path))), path);
}

#[test]
fn the_snippet_registers_the_shared_scribe_server_entry() {
    let path = Path::new("/Applications/Scribe.app/Contents/MacOS/scribe-mcp");

    let snippet: Value = serde_json::from_str(&config_snippet(path)).unwrap();

    assert_eq!(snippet["mcpServers"]["scribe"], scribe_server_entry(path));
}
