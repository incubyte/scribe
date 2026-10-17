use crate::ports::StoreError;
use crate::recording::SessionId;

#[derive(Debug, thiserror::Error)]
pub enum LibraryError {
    #[error("Invalid transcript id: {0}")]
    InvalidId(String),
    #[error("Transcript not found: {0}")]
    NotFound(SessionId),
    #[error(transparent)]
    Store(StoreError),
}

impl From<StoreError> for LibraryError {
    fn from(error: StoreError) -> LibraryError {
        match error {
            StoreError::NotFound { id } => LibraryError::NotFound(id),
            other => LibraryError::Store(other),
        }
    }
}
