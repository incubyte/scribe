use std::fs;
use std::path::{Path, PathBuf};

pub const SETTINGS_FILE_NAME: &str = "settings.json";
pub const TRANSCRIPTS_FOLDER_KEY: &str = "transcripts_folder";
const APP_IDENTIFIER: &str = "com.incubyte.scribe";

#[must_use]
pub fn default_settings_path() -> Option<PathBuf> {
    Some(
        dirs::data_dir()?
            .join(APP_IDENTIFIER)
            .join(SETTINGS_FILE_NAME),
    )
}

pub(crate) fn configured_folder_path(settings_path: &Path) -> Option<PathBuf> {
    let raw = fs::read_to_string(settings_path).ok()?;
    let settings: serde_json::Value = serde_json::from_str(&raw).ok()?;
    let folder = PathBuf::from(settings.get(TRANSCRIPTS_FOLDER_KEY)?.as_str()?);
    folder.is_absolute().then_some(folder)
}
