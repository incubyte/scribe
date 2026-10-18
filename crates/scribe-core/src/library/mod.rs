mod error;
mod query;
mod search;
mod summary_view;

use std::sync::Arc;

pub use error::LibraryError;
pub use query::{Limit, ListQuery};
pub use search::{SearchHit, SearchQuery, SegmentMatch};
pub use summary_view::SessionSummaryView;

use search::find_matches;

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
        Ok(self
            .readable_newest_first()?
            .into_iter()
            .filter(|summary| query.includes(summary))
            .take(query.limit.get())
            .collect())
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

    pub fn search(
        &self,
        query: &SearchQuery,
        limit: Limit,
    ) -> Result<Vec<SearchHit>, LibraryError> {
        self.readable_newest_first()?
            .into_iter()
            .filter_map(|summary| self.hit_for(summary, query).transpose())
            .take(limit.get())
            .collect()
    }

    fn readable_newest_first(&self) -> Result<Vec<SessionSummary>, LibraryError> {
        let mut readable: Vec<SessionSummary> = self
            .store
            .list_all()?
            .into_iter()
            .filter(|summary| is_readable(summary.status))
            .collect();
        readable.sort_by(|a, b| b.started_at.cmp(&a.started_at));
        Ok(readable)
    }

    fn hit_for(
        &self,
        summary: SessionSummary,
        query: &SearchQuery,
    ) -> Result<Option<SearchHit>, LibraryError> {
        let document = self.store.read(&summary.id)?;
        let matches = find_matches(&document, query);
        Ok((!matches.is_empty()).then_some(SearchHit {
            id: summary.id,
            title: summary.title,
            started_at: summary.started_at,
            matches,
        }))
    }
}

fn is_readable(status: SessionStatus) -> bool {
    status != SessionStatus::Recording
}
