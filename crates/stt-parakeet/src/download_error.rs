use std::io;

#[derive(Debug, thiserror::Error)]
pub enum DownloadError {
    #[error("the model download failed on the network: {0}")]
    Network(String),
    #[error("{file} does not match its pinned SHA-256")]
    HashMismatch { file: String },
    #[error("the model files could not be written: {0}")]
    Io(#[from] io::Error),
}

impl DownloadError {
    #[must_use]
    pub fn kind(&self) -> &'static str {
        match self {
            DownloadError::Network(_) => "network",
            DownloadError::HashMismatch { .. } => "hash_mismatch",
            DownloadError::Io(_) => "io",
        }
    }
}
