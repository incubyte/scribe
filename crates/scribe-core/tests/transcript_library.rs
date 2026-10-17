use std::sync::Arc;
use std::time::Duration;

use chrono::{DateTime, FixedOffset, NaiveDate};
use scribe_core::library::{
    LibraryError, Limit, ListQuery, SearchHit, SearchQuery, TranscriptLibrary,
};
use scribe_core::ports::TranscriptStore;
use scribe_core::recording::{Segment, SessionHeader, SessionId, SessionStatus, Speaker};
use scribe_core::testing::InMemoryTranscriptStore;

fn moment(text: &str) -> DateTime<FixedOffset> {
    DateTime::parse_from_rfc3339(text).unwrap()
}

fn day(year: i32, month: u32, day: u32) -> NaiveDate {
    NaiveDate::from_ymd_opt(year, month, day).unwrap()
}

fn library_over(store: &Arc<InMemoryTranscriptStore>) -> TranscriptLibrary {
    TranscriptLibrary::new(store.clone())
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
