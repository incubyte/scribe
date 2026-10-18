use std::path::Path;

use scribe_tray::claude_desktop::{MergeResult, UnreadableConfig, merge_scribe_server};
use serde_json::Value;

const SCRIBE_PATH: &str = "/Applications/Scribe.app/Contents/MacOS/scribe-mcp";

fn merged(existing: Option<&str>, path: &str) -> String {
    match merge_scribe_server(existing, Path::new(path)).unwrap() {
        MergeResult::Write(content) => content,
        MergeResult::Unchanged => panic!("expected a write"),
    }
}

fn parse(content: &str) -> Value {
    serde_json::from_str(content).unwrap()
}

fn keys_of(value: &Value) -> Vec<&str> {
    value
        .as_object()
        .unwrap()
        .keys()
        .map(String::as_str)
        .collect()
}

#[test]
fn a_fresh_config_contains_only_scribes_server() {
    let content = parse(&merged(None, SCRIBE_PATH));

    assert_eq!(
        content,
        serde_json::json!({"mcpServers": {"scribe": {"command": SCRIBE_PATH}}})
    );
}

#[test]
fn scribe_is_added_next_to_the_users_other_servers_without_changing_them() {
    let existing =
        r#"{"mcpServers":{"zeta":{"command":"z","args":["1"]},"alpha":{"command":"a"}}}"#;

    let content = parse(&merged(Some(existing), SCRIBE_PATH));

    assert_eq!(content["mcpServers"]["zeta"]["args"][0], "1");
    assert_eq!(content["mcpServers"]["alpha"]["command"], "a");
    assert_eq!(content["mcpServers"]["scribe"]["command"], SCRIBE_PATH);
    assert_eq!(keys_of(&content["mcpServers"]), ["zeta", "alpha", "scribe"]);
}

#[test]
fn other_top_level_settings_keep_their_values_and_order() {
    let existing = r#"{"theme":"dark","mcpServers":{},"analytics":false}"#;

    let content = parse(&merged(Some(existing), SCRIBE_PATH));

    assert_eq!(keys_of(&content), ["theme", "mcpServers", "analytics"]);
    assert_eq!(content["theme"], "dark");
    assert_eq!(content["analytics"], false);
}

#[test]
fn an_old_scribe_path_is_replaced_with_the_current_one() {
    let existing = r#"{"mcpServers":{"scribe":{"command":"/old/scribe-mcp"}}}"#;

    let content = parse(&merged(Some(existing), SCRIBE_PATH));

    assert_eq!(content["mcpServers"]["scribe"]["command"], SCRIBE_PATH);
}

#[test]
fn a_config_already_pointing_at_scribe_needs_no_change() {
    let existing = format!(r#"{{"mcpServers":{{"scribe":{{"command":"{SCRIBE_PATH}"}}}}}}"#);

    let result = merge_scribe_server(Some(&existing), Path::new(SCRIBE_PATH)).unwrap();

    assert_eq!(result, MergeResult::Unchanged);
}

#[test]
fn an_empty_servers_object_gets_scribe_added() {
    let content = parse(&merged(Some(r#"{"mcpServers":{}}"#), SCRIBE_PATH));

    assert_eq!(content["mcpServers"]["scribe"]["command"], SCRIBE_PATH);
}

#[test]
fn a_config_that_is_not_valid_json_is_refused() {
    let result = merge_scribe_server(Some("{ not json"), Path::new(SCRIBE_PATH));

    assert!(matches!(result, Err(UnreadableConfig::InvalidJson)));
}

#[test]
fn a_config_whose_top_level_is_a_list_is_refused() {
    let result = merge_scribe_server(Some("[1, 2]"), Path::new(SCRIBE_PATH));

    assert!(matches!(result, Err(UnreadableConfig::RootNotObject)));
}

#[test]
fn servers_that_are_not_an_object_are_refused() {
    let result = merge_scribe_server(Some(r#"{"mcpServers":[]}"#), Path::new(SCRIBE_PATH));

    assert!(matches!(result, Err(UnreadableConfig::ServersNotObject)));
}

#[test]
fn a_path_with_spaces_and_backslashes_is_stored_exactly() {
    let path = r"C:\Program Files\Scribe App\scribe-mcp.exe";

    let content = parse(&merged(None, path));

    assert_eq!(content["mcpServers"]["scribe"]["command"], path);
}

#[test]
fn the_merged_config_ends_with_a_newline() {
    assert!(merged(None, SCRIBE_PATH).ends_with('\n'));
}

#[test]
fn a_whitespace_only_config_is_treated_like_a_missing_one() {
    let content = parse(&merged(Some(" \n\t"), SCRIBE_PATH));

    assert_eq!(
        content,
        serde_json::json!({"mcpServers": {"scribe": {"command": SCRIBE_PATH}}})
    );
}
