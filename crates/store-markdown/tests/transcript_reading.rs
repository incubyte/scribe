use std::fs;
use std::path::Path;
use std::time::Duration;

use chrono::{DateTime, FixedOffset};
use scribe_core::ports::{StoreError, TranscriptStore};
use scribe_core::recording::{Segment, SessionHeader, SessionId, SessionStatus, Speaker};
use store_markdown::{MarkdownTranscriptStore, TranscriptsFolder};

fn moment(text: &str) -> DateTime<FixedOffset> {
    DateTime::parse_from_rfc3339(text).unwrap()
}

fn store_in(folder: &Path) -> MarkdownTranscriptStore {
    MarkdownTranscriptStore::new(TranscriptsFolder::at(folder.to_path_buf()))
}

fn say(speaker: Speaker, seconds: u64, text: &str) -> Segment {
    Segment {
        speaker,
        start: Duration::from_secs(seconds),
        end: Duration::from_secs(seconds + 2),
        text: text.to_owned(),
    }
}

fn start_call(store: &MarkdownTranscriptStore) -> SessionId {
    let header = SessionHeader::recording(
        moment("2026-10-15T09:00:00+00:00"),
        "speechanalyzer",
        "en-US",
    );
    store.create(&header).unwrap()
}

fn finished_conversation(store: &MarkdownTranscriptStore) -> SessionId {
    let id = start_call(store);
    store
        .append(&id, &say(Speaker::Me, 3, "Hello there"))
        .unwrap();
    store
        .append(&id, &say(Speaker::Them, 3725, "Hi, how are you"))
        .unwrap();
    store
        .finalise(&id, moment("2026-10-15T10:05:00+00:00"))
        .unwrap();
    id
}

#[test]
fn a_saved_call_is_read_back_with_its_header_and_original_text() {
    let folder = tempfile::tempdir().unwrap();
    let store = store_in(folder.path());
    let id = finished_conversation(&store);

    let document = store.read(&id).unwrap();

    assert_eq!(document.header.status, SessionStatus::Complete);
    assert_eq!(document.header.title, "Call 2026-10-15 09:00");
    assert_eq!(
        document.raw,
        fs::read_to_string(store.path_for(&id)).unwrap()
    );
}

#[test]
fn a_saved_call_is_read_back_with_each_line_its_speaker_and_time() {
    let folder = tempfile::tempdir().unwrap();
    let store = store_in(folder.path());
    let id = finished_conversation(&store);

    let segments = store.read(&id).unwrap().segments;

    let spoken: Vec<(Speaker, Duration, &str)> = segments
        .iter()
        .map(|segment| (segment.speaker, segment.start, segment.text.as_str()))
        .collect();
    assert_eq!(
        spoken,
        [
            (Speaker::Me, Duration::from_secs(3), "Hello there"),
            (Speaker::Them, Duration::from_secs(3725), "Hi, how are you"),
        ]
    );
}

#[test]
fn reading_a_call_that_was_never_saved_reports_not_found() {
    let folder = tempfile::tempdir().unwrap();
    let store = store_in(folder.path());
    let unknown = SessionId::parse("2026-10-15-090000").unwrap();

    let error = store.read(&unknown).unwrap_err();

    assert!(matches!(error, StoreError::NotFound { id } if id == unknown));
}

#[test]
fn reading_a_file_without_a_transcript_header_reports_it_by_file_name() {
    let folder = tempfile::tempdir().unwrap();
    let store = store_in(folder.path());
    let id = SessionId::parse("2026-10-15-090000").unwrap();
    let path = store.path_for(&id);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, "just some notes\n").unwrap();

    let error = store.read(&id).unwrap_err();

    assert!(
        matches!(error, StoreError::Malformed { file_name } if file_name == "2026-10-15-090000.md")
    );
}

#[test]
fn listing_every_call_skips_a_corrupt_file_and_keeps_the_rest() {
    let folder = tempfile::tempdir().unwrap();
    let store = store_in(folder.path());
    let good = start_call(&store);
    fs::write(folder.path().join("2026-10-16-090000.md"), "garbage").unwrap();

    let listed = store.list_all().unwrap();

    assert_eq!(listed.len(), 1);
    assert_eq!(listed[0].id, good);
}

#[test]
fn listing_every_call_in_a_folder_that_does_not_exist_yet_is_empty() {
    let folder = tempfile::tempdir().unwrap();
    let store = store_in(&folder.path().join("Scribe"));

    assert!(store.list_all().unwrap().is_empty());
}

#[test]
fn listing_every_call_returns_more_than_a_page_of_calls() {
    let folder = tempfile::tempdir().unwrap();
    let store = store_in(folder.path());
    for minute in 0..30 {
        let started = format!("2026-10-15T09:{minute:02}:00+00:00");
        let header = SessionHeader::recording(moment(&started), "speechanalyzer", "en-US");
        store.create(&header).unwrap();
    }

    assert_eq!(store.list_all().unwrap().len(), 30);
}
