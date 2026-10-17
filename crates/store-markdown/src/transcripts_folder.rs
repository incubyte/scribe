use std::fs::DirBuilder;
use std::path::{Path, PathBuf};

use crate::StoreError;

const FOLDER_NAME: &str = "Scribe";

#[cfg(unix)]
const OWNER_ONLY_MODE: u32 = 0o700;

pub struct TranscriptsFolder(PathBuf);

impl TranscriptsFolder {
    pub fn default_location() -> Result<TranscriptsFolder, StoreError> {
        let documents = dirs::document_dir().ok_or(StoreError::DocumentsFolderUnavailable)?;
        Ok(TranscriptsFolder::at(documents.join(FOLDER_NAME)))
    }

    #[must_use]
    pub fn at(path: PathBuf) -> TranscriptsFolder {
        TranscriptsFolder(path)
    }

    pub fn ensure_exists(&self) -> Result<&Path, StoreError> {
        private_dir_builder()
            .create(&self.0)
            .map_err(|source| StoreError::CreateFolder {
                path: self.0.clone(),
                source,
            })?;
        Ok(&self.0)
    }

    #[must_use]
    pub fn path(&self) -> &Path {
        &self.0
    }
}

fn private_dir_builder() -> DirBuilder {
    let mut builder = DirBuilder::new();
    builder.recursive(true);
    #[cfg(unix)]
    std::os::unix::fs::DirBuilderExt::mode(&mut builder, OWNER_ONLY_MODE);
    builder
}
