use std::fs::DirBuilder;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};

use scribe_core::ports::StoreError;
use scribe_core::recording::SessionId;

use crate::month_layout::session_folder_path;
use crate::settings_file;

const FOLDER_NAME: &str = "Scribe";

#[cfg(unix)]
const OWNER_ONLY_MODE: u32 = 0o700;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TranscriptsFolder(PathBuf);

impl TranscriptsFolder {
    pub fn default_location() -> Result<TranscriptsFolder, StoreError> {
        let documents = dirs::document_dir().ok_or(StoreError::DocumentsFolderUnavailable)?;
        Ok(TranscriptsFolder::at(documents.join(FOLDER_NAME)))
    }

    pub fn configured_in(settings_path: &Path) -> Result<TranscriptsFolder, StoreError> {
        match settings_file::configured_folder_path(settings_path) {
            Some(path) => Ok(TranscriptsFolder::at(path)),
            None => TranscriptsFolder::default_location(),
        }
    }

    pub fn configured_by(settings_path: Option<&Path>) -> Result<TranscriptsFolder, StoreError> {
        match settings_path {
            Some(path) => TranscriptsFolder::configured_in(path),
            None => TranscriptsFolder::default_location(),
        }
    }

    #[must_use]
    pub fn at(path: PathBuf) -> TranscriptsFolder {
        TranscriptsFolder(path)
    }

    pub fn ensure_exists(&self) -> Result<&Path, StoreError> {
        recursive_private_dir_builder()
            .create(&self.0)
            .map_err(|source| StoreError::CreateFolder {
                path: self.0.clone(),
                source,
            })?;
        Ok(&self.0)
    }

    pub(crate) fn ensure_month_exists(&self, id: &SessionId) -> Result<PathBuf, StoreError> {
        let month_folder = self.0.join(id.month());
        recursive_private_dir_builder()
            .create(&month_folder)
            .map_err(|source| StoreError::CreateFolder {
                path: month_folder.clone(),
                source,
            })?;
        Ok(month_folder)
    }

    pub(crate) fn ensure_session_folder_exists(
        &self,
        id: &SessionId,
    ) -> Result<Option<PathBuf>, StoreError> {
        let session_folder = session_folder_path(&self.0, id);
        match private_dir_builder().create(&session_folder) {
            Ok(()) => Ok(Some(session_folder)),
            Err(error) if error.kind() == ErrorKind::AlreadyExists => Ok(None),
            Err(source) => Err(StoreError::CreateFolder {
                path: session_folder,
                source,
            }),
        }
    }

    #[must_use]
    pub fn path(&self) -> &Path {
        &self.0
    }
}

fn recursive_private_dir_builder() -> DirBuilder {
    let mut builder = private_dir_builder();
    builder.recursive(true);
    builder
}

pub(crate) fn private_dir_builder() -> DirBuilder {
    let mut builder = DirBuilder::new();
    #[cfg(unix)]
    std::os::unix::fs::DirBuilderExt::mode(&mut builder, OWNER_ONLY_MODE);
    builder
}
