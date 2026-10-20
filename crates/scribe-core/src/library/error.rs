use crate::ports::StoreError;
use crate::recording::SessionId;

#[derive(Debug, thiserror::Error)]
pub enum LibraryError {
    #[error("Invalid transcript id: {0}")]
    InvalidId(String),
    #[error("Transcript not found: {0}")]
    NotFound(SessionId),
    #[error("Search query is empty")]
    EmptyQuery,
    #[error("Invalid screenshot name: {0}")]
    InvalidScreenshotName(String),
    #[error("No screenshot named {0}")]
    ScreenshotNotFound(String),
    #[error(transparent)]
    Store(StoreError),
}

impl From<StoreError> for LibraryError {
    fn from(error: StoreError) -> LibraryError {
        match error {
            StoreError::NotFound { id } => LibraryError::NotFound(id),
            StoreError::ScreenshotNotFound { reference } => {
                LibraryError::ScreenshotNotFound(reference.to_string())
            }
            other => LibraryError::Store(other),
        }
    }
}
