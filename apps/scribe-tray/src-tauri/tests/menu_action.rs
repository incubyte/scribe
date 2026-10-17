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
