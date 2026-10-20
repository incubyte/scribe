use std::path::PathBuf;

use scribe_core::ports::StoreError;
use store_markdown::TranscriptsFolder;

#[derive(Debug, Clone)]
pub enum TranscriptsLocation {
    Fixed(TranscriptsFolder),
    FollowingSettings(PathBuf),
}

impl TranscriptsLocation {
    pub fn current(&self) -> Result<TranscriptsFolder, StoreError> {
        match self {
            TranscriptsLocation::Fixed(folder) => Ok(folder.clone()),
            TranscriptsLocation::FollowingSettings(settings_path) => {
                TranscriptsFolder::configured_in(settings_path)
            }
        }
    }
}
