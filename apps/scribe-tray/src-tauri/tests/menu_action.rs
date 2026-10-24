use scribe_tray::menu_action::MenuAction;

#[test]
fn open_transcripts_folder_id_maps_to_opening_the_folder() {
    assert_eq!(
        MenuAction::from_id("open-transcripts-folder"),
        Some(MenuAction::OpenTranscriptsFolder)
    );
}

#[test]
fn quit_id_maps_to_quitting() {
    assert_eq!(MenuAction::from_id("quit"), Some(MenuAction::Quit));
}

#[test]
fn unknown_id_maps_to_no_action() {
    assert_eq!(MenuAction::from_id("something-else"), None);
}

#[test]
fn empty_id_maps_to_no_action() {
    assert_eq!(MenuAction::from_id(""), None);
}

#[test]
fn privacy_id_maps_to_showing_the_privacy_statement() {
    assert_eq!(
        MenuAction::from_id("privacy"),
        Some(MenuAction::ShowPrivacy)
    );
}

#[test]
fn toggle_recording_id_maps_to_starting_or_stopping_a_recording() {
    assert_eq!(
        MenuAction::from_id("toggle-recording"),
        Some(MenuAction::ToggleRecording)
    );
}

#[test]
fn copy_mcp_config_id_maps_to_copying_the_mcp_config() {
    assert_eq!(
        MenuAction::from_id("copy-mcp-config"),
        Some(MenuAction::CopyMcpConfig)
    );
}

#[test]
fn connect_claude_desktop_id_maps_to_connecting_claude_desktop() {
    assert_eq!(
        MenuAction::from_id("connect-claude-desktop"),
        Some(MenuAction::ConnectClaudeDesktop)
    );
}

#[test]
fn restart_to_update_id_maps_to_restarting_to_install_the_update() {
    assert_eq!(
        MenuAction::from_id("restart-to-update"),
        Some(MenuAction::RestartToUpdate)
    );
}
