use std::fs;
use std::path::Path;

use chrono::{DateTime, FixedOffset};
use scribe_core::ports::TranscriptStore;
use scribe_core::recording::{SessionHeader, SessionStatus};
use store_markdown::{MarkdownTranscriptStore, TranscriptsFolder};

fn moment(text: &str) -> DateTime<FixedOffset> {
    DateTime::parse_from_rfc3339(text).unwrap()
}

fn store_in(parent: &Path) -> MarkdownTranscriptStore {
    MarkdownTranscriptStore::new(TranscriptsFolder::at(parent.join("Scribe")))
}

fn record_call(store: &MarkdownTranscriptStore, started_at: &str, ended_at: Option<&str>) {
    let header = SessionHeader::recording(moment(started_at), "speechanalyzer", "en-US");
    let id = store.create(&header).unwrap();
    if let Some(ended_at) = ended_at {
        store.finalise(&id, moment(ended_at)).unwrap();
    }
}

fn record_ten_minute_call_on(store: &MarkdownTranscriptStore, day: u8) {
    record_call(
        store,
        &format!("2026-10-{day}T09:00:00+00:00"),
        Some(&format!("2026-10-{day}T09:10:00+00:00")),
    );
}

#[test]
fn sessions_are_listed_newest_first() {
    let parent = tempfile::tempdir().unwrap();
    let store = store_in(parent.path());
    record_ten_minute_call_on(&store, 15);
    record_ten_minute_call_on(&store, 17);
    record_ten_minute_call_on(&store, 16);

    let titles: Vec<String> = store
        .list(10)
        .unwrap()
        .into_iter()
        .map(|summary| summary.title)
        .collect();

    assert_eq!(
        titles,
        [
            "Call 2026-10-17 09:00",
            "Call 2026-10-16 09:00",
            "Call 2026-10-15 09:00"
        ]
    );
}

#[test]
fn the_listing_stops_at_the_requested_limit() {
    let parent = tempfile::tempdir().unwrap();
    let store = store_in(parent.path());
    record_call(&store, "2026-10-15T09:00:00+00:00", None);
    record_call(&store, "2026-10-16T09:00:00+00:00", None);
    record_call(&store, "2026-10-17T09:00:00+00:00", None);

    let listed = store.list(2).unwrap();

    assert_eq!(listed.len(), 2);
    assert_eq!(listed[0].id.as_str(), "2026-10-17-090000");
}

#[test]
fn a_finished_session_reports_how_long_it_lasted() {
    let parent = tempfile::tempdir().unwrap();
    let store = store_in(parent.path());
    record_call(
        &store,
        "2026-10-17T09:00:00+00:00",
        Some("2026-10-17T09:32:41+00:00"),
    );

    let listed = store.list(5).unwrap();

    assert_eq!(listed[0].duration_seconds, Some(32 * 60 + 41));
    assert_eq!(listed[0].status, SessionStatus::Complete);
}

#[test]
fn a_session_still_recording_is_listed_with_its_status_and_no_duration() {
    let parent = tempfile::tempdir().unwrap();
    let store = store_in(parent.path());
    record_call(&store, "2026-10-17T09:00:00+00:00", None);

    let listed = store.list(5).unwrap();

    assert_eq!(listed[0].status, SessionStatus::Recording);
    assert_eq!(listed[0].duration_seconds, None);
}

#[test]
fn a_folder_that_does_not_exist_yet_lists_nothing() {
    let parent = tempfile::tempdir().unwrap();

    let listed = store_in(parent.path()).list(5).unwrap();

    assert!(listed.is_empty());
}

#[test]
fn a_file_that_is_not_a_transcript_is_left_out_of_the_listing() {
    let parent = tempfile::tempdir().unwrap();
    let store = store_in(parent.path());
    record_call(&store, "2026-10-17T09:00:00+00:00", None);
    fs::write(
        parent.path().join("Scribe").join("notes.md"),
        "just some notes\n",
    )
    .unwrap();

    let listed = store.list(5).unwrap();

    assert_eq!(listed.len(), 1);
}
