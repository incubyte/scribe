use chrono::{DateTime, FixedOffset};

use crate::library::LibraryError;
use crate::recording::{Segment, SessionDocument, SessionId, Speaker, offset_as_clock};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SearchQuery(String);

impl SearchQuery {
    pub fn parse(raw: &str) -> Result<SearchQuery, LibraryError> {
        let folded = raw.trim().to_lowercase();
        if folded.is_empty() {
            Err(LibraryError::EmptyQuery)
        } else {
            Ok(SearchQuery(folded))
        }
    }

    fn matches(&self, text: &str) -> bool {
        text.to_lowercase().contains(self.0.as_str())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SearchHit {
    pub id: SessionId,
    pub title: String,
    pub started_at: DateTime<FixedOffset>,
    pub matches: Vec<SegmentMatch>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SegmentMatch {
    pub at: String,
    pub speaker: Speaker,
    pub text: String,
}

impl SegmentMatch {
    fn from_segment(segment: &Segment) -> SegmentMatch {
        SegmentMatch {
            at: offset_as_clock(segment.start),
            speaker: segment.speaker,
            text: segment.text.clone(),
        }
    }
}

pub(super) fn find_matches(document: &SessionDocument, query: &SearchQuery) -> Vec<SegmentMatch> {
    document
        .segments
        .iter()
        .filter(|segment| query.matches(&segment.text))
        .map(SegmentMatch::from_segment)
        .collect()
}
