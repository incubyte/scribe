use std::path::PathBuf;

#[derive(Debug, thiserror::Error)]
pub enum StoreError {
    #[error("the user's documents folder could not be located")]
    DocumentsFolderUnavailable,
    #[error("could not create the transcripts folder at {path}")]
    CreateFolder {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
}
