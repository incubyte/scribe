mod error;
mod query;
mod search;
mod summary_view;

use std::sync::Arc;

pub use error::LibraryError;
pub use query::{Limit, ListQuery};
pub use search::{SearchHit, SearchQuery, SegmentMatch};
pub use summary_view::{MeetingSummaryView, PersonView, SessionSummaryView};

use search::find_matches;

use crate::capture::{ScreenshotReference, ServedImage};
use crate::ports::{ScreenshotStore, StoreError, TranscriptStore};
use crate::recording::{SessionDocument, SessionId, SessionStatus, SessionSummary};

#[derive(Clone)]
pub struct TranscriptLibrary {
    store: Arc<dyn TranscriptStore>,
    screenshots: Arc<dyn ScreenshotStore>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ServedScreenshot {
    pub reference: ScreenshotReference,
    pub image: ServedImage,
}

impl TranscriptLibrary {
    #[must_use]
    pub fn new(
        store: Arc<dyn TranscriptStore>,
        screenshots: Arc<dyn ScreenshotStore>,
    ) -> TranscriptLibrary {
        TranscriptLibrary { store, screenshots }
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

    pub fn screenshots(&self, id: &SessionId) -> Result<Vec<ScreenshotReference>, LibraryError> {
        let mut names = self.screenshots.list_screenshots(id)?;
        names.sort();
        Ok(names
            .into_iter()
            .map(|name| ScreenshotReference::new(id.clone(), name))
            .collect())
    }

    pub fn screenshot(
        &self,
        raw_reference: &str,
        max_long_edge: u32,
    ) -> Result<ServedScreenshot, LibraryError> {
        let reference = ScreenshotReference::parse(raw_reference)
            .map_err(|_| LibraryError::InvalidScreenshotName(raw_reference.to_owned()))?;
        match self.screenshots.serve(&reference, max_long_edge) {
            Ok(image) => Ok(ServedScreenshot { reference, image }),
            Err(StoreError::Malformed { .. }) => {
                Err(LibraryError::ScreenshotNotFound(reference.to_string()))
            }
            Err(other) => Err(other.into()),
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
        readable.sort_by_key(|summary| std::cmp::Reverse(summary.started_at));
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
