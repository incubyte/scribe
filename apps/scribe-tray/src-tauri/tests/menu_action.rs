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
