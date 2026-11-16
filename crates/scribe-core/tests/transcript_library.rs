#![allow(clippy::assert_is_empty)]

use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use chrono::{DateTime, FixedOffset, NaiveDate};
use scribe_core::capture::SessionMoment;
use scribe_core::library::{
    LibraryError, Limit, ListQuery, SearchHit, SearchQuery, TranscriptLibrary,
};
use scribe_core::ports::{ScreenshotStore, StoreError, TranscriptStore};
use scribe_core::recording::{
    Segment, SessionDocument, SessionHeader, SessionId, SessionStatus, SessionSummary, Speaker,
};
use scribe_core::testing::{InMemoryScreenshotStore, InMemoryTranscriptStore, frame_filled};

fn moment(text: &str) -> DateTime<FixedOffset> {
    DateTime::parse_from_rfc3339(text).unwrap()
}

fn day(year: i32, month: u32, day: u32) -> NaiveDate {
    NaiveDate::from_ymd_opt(year, month, day).unwrap()
}

fn library_over(store: &Arc<InMemoryTranscriptStore>) -> TranscriptLibrary {
    library_with_screenshots(store, &Arc::new(InMemoryScreenshotStore::default()))
}

fn library_with_screenshots(
    store: &Arc<InMemoryTranscriptStore>,
    screenshots: &Arc<InMemoryScreenshotStore>,
) -> TranscriptLibrary {
    TranscriptLibrary::new(store.clone(), screenshots.clone())
}

fn screenshot_at(screenshots: &InMemoryScreenshotStore, session: &SessionId, seconds: u64) {
    let moment = SessionMoment {
        session: session.clone(),
        offset: Duration::from_secs(seconds),
    };
    screenshots
        .save(&moment, &frame_filled(2, 2, [0, 0, 0, 255]))
        .unwrap();
}

fn finished_call(store: &InMemoryTranscriptStore, started_at: &str) -> SessionId {
    let id = ongoing_call(store, started_at);
    store.finalise(&id, moment(started_at)).unwrap();
    id
}

fn ongoing_call(store: &InMemoryTranscriptStore, started_at: &str) -> SessionId {
    let header = SessionHeader::recording(moment(started_at), "speechanalyzer", "en-US");
    store.create(&header).unwrap()
}

fn query(limit: usize, since: Option<NaiveDate>) -> ListQuery {
    ListQuery {
        limit: Limit::clamped(limit),
        since,
    }
}

#[test]
fn an_unspecified_limit_lists_twenty_calls() {
    assert_eq!(Limit::default().get(), 20);
}

#[test]
fn a_limit_of_zero_still_lists_one_call() {
    assert_eq!(Limit::clamped(0).get(), 1);
}

#[test]
fn a_limit_above_one_hundred_is_capped_at_one_hundred() {
    assert_eq!(Limit::clamped(5_000).get(), 100);
}

#[test]
fn a_limit_within_range_is_kept() {
    assert_eq!(Limit::clamped(37).get(), 37);
}

#[test]
fn saved_calls_are_listed_newest_first() {
    let store = Arc::new(InMemoryTranscriptStore::default());
    finished_call(&store, "2026-10-15T09:00:00+00:00");
    finished_call(&store, "2026-10-17T09:00:00+00:00");
    finished_call(&store, "2026-10-16T09:00:00+00:00");

    let ids: Vec<String> = library_over(&store)
        .list(query(10, None))
        .unwrap()
        .iter()
        .map(|summary| summary.id.to_string())
        .collect();

    assert_eq!(
        ids,
        [
            "2026-10-17-090000",
            "2026-10-16-090000",
            "2026-10-15-090000"
        ]
    );
}

#[test]
fn a_call_still_being_recorded_is_not_listed() {
    let store = Arc::new(InMemoryTranscriptStore::default());
    finished_call(&store, "2026-10-15T09:00:00+00:00");
    ongoing_call(&store, "2026-10-16T09:00:00+00:00");

    let statuses: Vec<SessionStatus> = library_over(&store)
        .list(query(10, None))
        .unwrap()
        .iter()
        .map(|summary| summary.status)
        .collect();

    assert_eq!(statuses, [SessionStatus::Complete]);
}

#[test]
fn an_abandoned_call_is_listed_as_incomplete() {
    let store = Arc::new(InMemoryTranscriptStore::default());
    ongoing_call(&store, "2026-10-15T09:00:00+00:00");
    store.mark_abandoned_incomplete().unwrap();

    let statuses: Vec<SessionStatus> = library_over(&store)
        .list(query(10, None))
        .unwrap()
        .iter()
        .map(|summary| summary.status)
        .collect();

    assert_eq!(statuses, [SessionStatus::Incomplete]);
}

#[test]
fn listing_stops_at_the_limit_after_dropping_calls_still_being_recorded() {
    let store = Arc::new(InMemoryTranscriptStore::default());
    finished_call(&store, "2026-10-15T09:00:00+00:00");
    finished_call(&store, "2026-10-16T09:00:00+00:00");
    ongoing_call(&store, "2026-10-17T09:00:00+00:00");

    let listed = library_over(&store).list(query(1, None)).unwrap();

    assert_eq!(listed.len(), 1);
    assert_eq!(listed[0].id.to_string(), "2026-10-16-090000");
}

#[test]
fn only_calls_on_or_after_the_since_date_are_listed() {
    let store = Arc::new(InMemoryTranscriptStore::default());
    finished_call(&store, "2026-10-14T23:59:00+00:00");
    finished_call(&store, "2026-10-15T00:01:00+00:00");
    finished_call(&store, "2026-10-16T09:00:00+00:00");

    let ids: Vec<String> = library_over(&store)
        .list(query(10, Some(day(2026, 10, 15))))
        .unwrap()
        .iter()
        .map(|summary| summary.id.to_string())
        .collect();

    assert_eq!(ids, ["2026-10-16-090000", "2026-10-15-000100"]);
}

#[test]
fn a_saved_call_is_read_with_its_header_and_segments() {
    let store = Arc::new(InMemoryTranscriptStore::default());
    let id = ongoing_call(&store, "2026-10-15T09:00:00+00:00");
    store
        .append(
            &id,
            &Segment {
                speaker: Speaker::Them,
                start: Duration::from_secs(5),
                end: Duration::from_secs(8),
                text: "Welcome everyone".to_owned(),
            },
        )
        .unwrap();
    store
        .finalise(&id, moment("2026-10-15T09:10:00+00:00"))
        .unwrap();

    let document = library_over(&store).get(id.as_str()).unwrap();

    assert_eq!(document.header.status, SessionStatus::Complete);
    assert_eq!(document.segments.len(), 1);
    assert_eq!(document.segments[0].text, "Welcome everyone");
}

#[test]
fn reading_an_unknown_call_reports_it_was_not_found() {
    let store = Arc::new(InMemoryTranscriptStore::default());

    let error = library_over(&store).get("2026-10-15-090000").unwrap_err();

    assert_eq!(error.to_string(), "Transcript not found: 2026-10-15-090000");
}

#[test]
fn reading_a_call_still_being_recorded_reports_it_was_not_found() {
    let store = Arc::new(InMemoryTranscriptStore::default());
    let id = ongoing_call(&store, "2026-10-15T09:00:00+00:00");

    let error = library_over(&store).get(id.as_str()).unwrap_err();

    assert!(matches!(error, LibraryError::NotFound(missing) if missing == id));
}

#[test]
fn reading_an_id_that_is_not_a_session_id_is_rejected() {
    let store = Arc::new(InMemoryTranscriptStore::default());

    let error = library_over(&store).get("../secrets").unwrap_err();

    assert_eq!(error.to_string(), "Invalid transcript id: ../secrets");
}

fn said(speaker: Speaker, start_secs: u64, text: &str) -> Segment {
    Segment {
        speaker,
        start: Duration::from_secs(start_secs),
        end: Duration::from_secs(start_secs),
        text: text.to_owned(),
    }
}

fn searching(store: &Arc<InMemoryTranscriptStore>, phrase: &str, limit: usize) -> Vec<SearchHit> {
    let query = SearchQuery::parse(phrase).unwrap();
    library_over(store)
        .search(&query, Limit::clamped(limit))
        .unwrap()
}

#[test]
fn a_phrase_no_call_mentions_finds_no_calls() {
    let store = Arc::new(InMemoryTranscriptStore::default());
    let id = finished_call(&store, "2026-10-15T09:00:00+00:00");
    store
        .append(&id, &said(Speaker::Them, 4, "Welcome everyone"))
        .unwrap();

    let hits = searching(&store, "quarterly", 10);

    assert!(hits.is_empty());
}

#[test]
fn each_match_carries_the_stamp_speaker_and_text_as_stored() {
    let store = Arc::new(InMemoryTranscriptStore::default());
    let id = finished_call(&store, "2026-10-15T09:00:00+00:00");
    store
        .append(&id, &said(Speaker::Me, 65, "Quarterly Review"))
        .unwrap();

    let hits = searching(&store, "quarterly", 10);

    let matched = &hits[0].matches[0];
    assert_eq!(matched.at, "00:01:05");
    assert_eq!(matched.speaker, Speaker::Me);
    assert_eq!(matched.text, "Quarterly Review");
}

#[test]
fn matching_calls_come_back_newest_first_up_to_the_limit() {
    let store = Arc::new(InMemoryTranscriptStore::default());
    for started_at in [
        "2026-10-15T09:00:00+00:00",
        "2026-10-17T09:00:00+00:00",
        "2026-10-16T09:00:00+00:00",
    ] {
        let id = finished_call(&store, started_at);
        store
            .append(&id, &said(Speaker::Them, 4, "budget approved"))
            .unwrap();
    }

    let ids: Vec<String> = searching(&store, "budget", 2)
        .iter()
        .map(|hit| hit.id.to_string())
        .collect();

    assert_eq!(ids, ["2026-10-17-090000", "2026-10-16-090000"]);
}

#[test]
fn a_blank_search_query_is_rejected() {
    let error = SearchQuery::parse(" \t ").unwrap_err();

    assert!(matches!(error, LibraryError::EmptyQuery));
}

#[test]
fn a_calls_screenshots_are_listed_in_offset_order() {
    let store = Arc::new(InMemoryTranscriptStore::default());
    let screenshots = Arc::new(InMemoryScreenshotStore::default());
    let id = finished_call(&store, "2026-10-20T14:30:05+05:30");
    screenshot_at(&screenshots, &id, 41 * 60 + 2);
    screenshot_at(&screenshots, &id, 12 * 60 + 34);
    screenshot_at(&screenshots, &id, 12 * 60 + 34);

    let references: Vec<String> = library_with_screenshots(&store, &screenshots)
        .screenshots(&id)
        .unwrap()
        .iter()
        .map(ToString::to_string)
        .collect();

    assert_eq!(
        references,
        [
            "2026-10-20-143005/00-12-34.png",
            "2026-10-20-143005/00-12-34-2.png",
            "2026-10-20-143005/00-41-02.png"
        ]
    );
}

#[test]
fn asking_for_a_screenshot_that_was_never_taken_reports_its_reference() {
    let store = Arc::new(InMemoryTranscriptStore::default());

    let failure = library_over(&store)
        .screenshot("2026-10-20-143005/00-12-34.png", 1568)
        .unwrap_err();

    assert_eq!(
        failure.to_string(),
        "No screenshot named 2026-10-20-143005/00-12-34.png"
    );
}

#[test]
fn a_malformed_screenshot_reference_is_rejected_by_name() {
    let store = Arc::new(InMemoryTranscriptStore::default());

    let failure = library_over(&store)
        .screenshot("../escape.png", 1568)
        .unwrap_err();

    assert_eq!(
        failure.to_string(),
        "Invalid screenshot name: ../escape.png"
    );
}

struct FailingReadTranscriptStore {
    inner: InMemoryTranscriptStore,
    failing_id: SessionId,
    make_error: fn(&SessionId) -> StoreError,
}

impl FailingReadTranscriptStore {
    fn malformed(inner: InMemoryTranscriptStore, failing_id: SessionId) -> Self {
        Self {
            inner,
            failing_id,
            make_error: |id| StoreError::Malformed {
                file_name: format!("{id}.md"),
            },
        }
    }

    fn io(inner: InMemoryTranscriptStore, failing_id: SessionId) -> Self {
        Self {
            inner,
            failing_id,
            make_error: |id| StoreError::Io {
                path: PathBuf::from(format!("/transcripts/{id}.md")),
                source: std::io::Error::new(
                    std::io::ErrorKind::PermissionDenied,
                    "permission denied",
                ),
            },
        }
    }

    fn not_found(inner: InMemoryTranscriptStore, failing_id: SessionId) -> Self {
        Self {
            inner,
            failing_id,
            make_error: |id| StoreError::NotFound { id: id.clone() },
        }
    }
}

impl TranscriptStore for FailingReadTranscriptStore {
    fn create(&self, header: &SessionHeader) -> Result<SessionId, StoreError> {
        self.inner.create(header)
    }

    fn append(&self, id: &SessionId, segment: &Segment) -> Result<(), StoreError> {
        self.inner.append(id, segment)
    }

    fn finalise(&self, id: &SessionId, ended_at: DateTime<FixedOffset>) -> Result<(), StoreError> {
        self.inner.finalise(id, ended_at)
    }

    fn mark_abandoned_incomplete(&self) -> Result<usize, StoreError> {
        self.inner.mark_abandoned_incomplete()
    }

    fn list(&self, limit: usize) -> Result<Vec<SessionSummary>, StoreError> {
        self.inner.list(limit)
    }

    fn list_all(&self) -> Result<Vec<SessionSummary>, StoreError> {
        self.inner.list_all()
    }

    fn read(&self, id: &SessionId) -> Result<SessionDocument, StoreError> {
        if id == &self.failing_id {
            Err((self.make_error)(id))
        } else {
            self.inner.read(id)
        }
    }
}

struct FailingListTranscriptStore;

impl TranscriptStore for FailingListTranscriptStore {
    fn create(&self, _header: &SessionHeader) -> Result<SessionId, StoreError> {
        Err(StoreError::DocumentsFolderUnavailable)
    }

    fn append(&self, _id: &SessionId, _segment: &Segment) -> Result<(), StoreError> {
        Err(StoreError::DocumentsFolderUnavailable)
    }

    fn finalise(
        &self,
        _id: &SessionId,
        _ended_at: DateTime<FixedOffset>,
    ) -> Result<(), StoreError> {
        Err(StoreError::DocumentsFolderUnavailable)
    }

    fn mark_abandoned_incomplete(&self) -> Result<usize, StoreError> {
        Err(StoreError::DocumentsFolderUnavailable)
    }

    fn list(&self, _limit: usize) -> Result<Vec<SessionSummary>, StoreError> {
        Err(StoreError::DocumentsFolderUnavailable)
    }

    fn list_all(&self) -> Result<Vec<SessionSummary>, StoreError> {
        Err(StoreError::DocumentsFolderUnavailable)
    }

    fn read(&self, _id: &SessionId) -> Result<SessionDocument, StoreError> {
        Err(StoreError::DocumentsFolderUnavailable)
    }
}

fn three_transcripts(inner: &InMemoryTranscriptStore) -> (SessionId, SessionId, SessionId) {
    let id_a = finished_call(inner, "2026-10-17T09:00:00+00:00");
    inner
        .append(&id_a, &said(Speaker::Them, 10, "Discuss the budget"))
        .unwrap();

    let id_b = finished_call(inner, "2026-10-16T09:00:00+00:00");
    inner
        .append(&id_b, &said(Speaker::Them, 10, "Any content"))
        .unwrap();

    let id_c = finished_call(inner, "2026-10-15T09:00:00+00:00");
    inner
        .append(&id_c, &said(Speaker::Them, 10, "Budget approved"))
        .unwrap();

    (id_a, id_b, id_c)
}

#[test]
fn search_skips_unreadable_transcript_and_returns_other_matches() {
    let inner = InMemoryTranscriptStore::default();
    let (id_a, id_b, id_c) = three_transcripts(&inner);

    let store = Arc::new(FailingReadTranscriptStore::malformed(inner, id_b.clone()));
    let library = TranscriptLibrary::new(store, Arc::new(InMemoryScreenshotStore::default()));

    let query = SearchQuery::parse("budget").unwrap();
    let hits = library
        .search(&query, Limit::clamped(10))
        .expect("search should succeed and skip unreadable transcripts");

    let matched_ids: Vec<String> = hits.iter().map(|hit| hit.id.to_string()).collect();
    assert_eq!(hits.len(), 2);
    assert_eq!(matched_ids, vec![id_a.to_string(), id_c.to_string()]);
    assert!(!matched_ids.contains(&id_b.to_string()));
}

#[test]
fn search_skipped_unreadable_transcript_does_not_consume_limit() {
    let inner = InMemoryTranscriptStore::default();

    // Meeting A: newer, but unreadable
    let id_a = finished_call(&inner, "2026-10-17T09:00:00+00:00");
    inner
        .append(&id_a, &said(Speaker::Them, 10, "Discuss the budget"))
        .unwrap();

    // Meeting B: older, readable, matches "budget"
    let id_b = finished_call(&inner, "2026-10-16T09:00:00+00:00");
    inner
        .append(&id_b, &said(Speaker::Them, 10, "Discuss the budget"))
        .unwrap();

    let store = Arc::new(FailingReadTranscriptStore::malformed(inner, id_a));
    let library = TranscriptLibrary::new(store, Arc::new(InMemoryScreenshotStore::default()));

    let query = SearchQuery::parse("budget").unwrap();
    let hits = library
        .search(&query, Limit::clamped(1))
        .expect("search should succeed and skip unreadable transcript");

    assert_eq!(hits.len(), 1);
    assert_eq!(hits[0].id, id_b);
}

#[test]
fn search_fails_when_store_enumeration_fails() {
    let store = Arc::new(FailingListTranscriptStore);
    let library = TranscriptLibrary::new(store, Arc::new(InMemoryScreenshotStore::default()));

    let query = SearchQuery::parse("budget").unwrap();
    let result = library.search(&query, Limit::clamped(10));

    assert!(matches!(
        result,
        Err(LibraryError::Store(StoreError::DocumentsFolderUnavailable))
    ));
}

#[test]
fn search_skips_transcript_with_io_read_error_and_returns_other_matches() {
    let inner = InMemoryTranscriptStore::default();
    let (id_a, id_b, id_c) = three_transcripts(&inner);

    let store = Arc::new(FailingReadTranscriptStore::io(inner, id_b.clone()));
    let library = TranscriptLibrary::new(store, Arc::new(InMemoryScreenshotStore::default()));

    let query = SearchQuery::parse("budget").unwrap();
    let hits = library
        .search(&query, Limit::clamped(10))
        .expect("search should succeed and skip I/O failed transcript");

    let matched_ids: Vec<String> = hits.iter().map(|hit| hit.id.to_string()).collect();
    assert_eq!(hits.len(), 2);
    assert_eq!(matched_ids, vec![id_a.to_string(), id_c.to_string()]);
    assert!(!matched_ids.contains(&id_b.to_string()));
}

#[test]
fn search_skips_transcript_not_found_on_read_and_returns_other_matches() {
    let inner = InMemoryTranscriptStore::default();
    let (id_a, id_b, id_c) = three_transcripts(&inner);

    let store = Arc::new(FailingReadTranscriptStore::not_found(inner, id_b.clone()));
    let library = TranscriptLibrary::new(store, Arc::new(InMemoryScreenshotStore::default()));

    let query = SearchQuery::parse("budget").unwrap();
    let hits = library
        .search(&query, Limit::clamped(10))
        .expect("search should succeed and skip missing transcript");

    let matched_ids: Vec<String> = hits.iter().map(|hit| hit.id.to_string()).collect();
    assert_eq!(hits.len(), 2);
    assert_eq!(matched_ids, vec![id_a.to_string(), id_c.to_string()]);
    assert!(!matched_ids.contains(&id_b.to_string()));
}
