pub const MENU_TOGGLE_RECORDING: &str = "toggle-recording";
pub const MENU_OPEN_FOLDER: &str = "open-transcripts-folder";
pub const MENU_PRIVACY: &str = "privacy";
pub const MENU_QUIT: &str = "quit";

const ACTIONS_BY_ID: [(&str, MenuAction); 4] = [
    (MENU_TOGGLE_RECORDING, MenuAction::ToggleRecording),
    (MENU_OPEN_FOLDER, MenuAction::OpenTranscriptsFolder),
    (MENU_PRIVACY, MenuAction::ShowPrivacy),
    (MENU_QUIT, MenuAction::Quit),
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MenuAction {
    ToggleRecording,
    OpenTranscriptsFolder,
    ShowPrivacy,
    Quit,
}

impl MenuAction {
    #[must_use]
    pub fn from_id(id: &str) -> Option<MenuAction> {
        ACTIONS_BY_ID
            .iter()
            .find(|(known_id, _)| *known_id == id)
            .map(|(_, action)| *action)
    }
}
