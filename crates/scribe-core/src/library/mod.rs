mod error;
mod query;

use std::sync::Arc;

pub use error::LibraryError;
pub use query::{Limit, ListQuery};

use crate::ports::TranscriptStore;
use crate::recording::{SessionDocument, SessionId, SessionStatus, SessionSummary};

#[derive(Clone)]
pub struct TranscriptLibrary {
    store: Arc<dyn TranscriptStore>,
}

impl TranscriptLibrary {
    #[must_use]
    pub fn new(store: Arc<dyn TranscriptStore>) -> TranscriptLibrary {
        TranscriptLibrary { store }
    }

    pub fn list(&self, query: ListQuery) -> Result<Vec<SessionSummary>, LibraryError> {
        let mut readable: Vec<SessionSummary> = self
            .store
            .list_all()?
            .into_iter()
            .filter(|summary| is_readable(summary.status) && query.includes(summary))
            .collect();
        readable.sort_by(|a, b| b.started_at.cmp(&a.started_at));
        readable.truncate(query.limit.get());
        Ok(readable)
    }

    pub fn get(&self, raw_id: &str) -> Result<SessionDocument, LibraryError> {
        let id =
            SessionId::parse(raw_id).map_err(|_| LibraryError::InvalidId(raw_id.to_owned()))?;
        let document = self.store.read(&id)?;
        if is_readable(document.header.status) {
            Ok(document)
        } else {
            Err(LibraryError::NotFound(id))
        }
    }
}

fn is_readable(status: SessionStatus) -> bool {
    status != SessionStatus::Recording
}
