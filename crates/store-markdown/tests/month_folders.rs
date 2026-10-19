use std::fs;
use std::path::Path;

use chrono::DateTime;
use scribe_core::ports::TranscriptStore;
use scribe_core::recording::{SessionHeader, SessionId, SessionStatus};
use store_markdown::{MarkdownTranscriptStore, TranscriptsFolder, render_header};

fn header_at(started_at: &str) -> SessionHeader {
    let started_at = DateTime::parse_from_rfc3339(started_at).unwrap();
    SessionHeader::recording(started_at, "speechanalyzer", "en-US")
}

fn store_in(folder: &Path) -> MarkdownTranscriptStore {
    MarkdownTranscriptStore::new(TranscriptsFolder::at(folder.to_path_buf()))
}

fn write_legacy_transcript(folder: &Path, started_at: &str) -> SessionId {
    let header = header_at(started_at);
    fs::create_dir_all(folder).unwrap();
    fs::write(
        folder.join(format!("{}.md", header.id)),
        render_header(&header),
    )
    .unwrap();
    header.id
}

fn listed_ids(store: &MarkdownTranscriptStore) -> Vec<String> {
    store
        .list(10)
        .unwrap()
        .into_iter()
        .map(|summary| summary.id.to_string())
        .collect()
}

#[test]
fn a_new_transcript_is_saved_inside_its_month_folder() {
    let folder = tempfile::tempdir().unwrap();
    let store = store_in(folder.path());

    let id = store
        .create(&header_at("2026-10-19T11:45:21+00:00"))
        .unwrap();

    assert!(
        folder
            .path()
            .join("2026-10")
            .join(format!("{id}.md"))
            .is_file()
    );
    assert!(!folder.path().join(format!("{id}.md")).exists());
}

#[test]
fn a_second_call_started_in_the_same_second_gets_its_own_file_in_the_same_month_folder() {
    let folder = tempfile::tempdir().unwrap();
    let store = store_in(folder.path());
    let header = header_at("2026-10-19T11:45:21+00:00");

    store.create(&header).unwrap();
    let second = store.create(&header).unwrap();

    assert_eq!(second.as_str(), "2026-10-19-114521-2");
    assert!(
        folder
            .path()
            .join("2026-10")
            .join("2026-10-19-114521-2.md")
            .is_file()
    );
}

#[cfg(unix)]
#[test]
fn a_month_folder_is_private_to_the_user() {
    use std::os::unix::fs::PermissionsExt;
    let folder = tempfile::tempdir().unwrap();
    let store = store_in(&folder.path().join("Scribe"));

    store
        .create(&header_at("2026-10-19T11:45:21+00:00"))
        .unwrap();

    let mode = fs::metadata(folder.path().join("Scribe").join("2026-10"))
        .unwrap()
        .permissions()
        .mode();
    assert_eq!(mode & 0o777, 0o700);
}

#[test]
fn a_transcript_saved_before_month_folders_is_still_listed_and_read() {
    let folder = tempfile::tempdir().unwrap();
    let id = write_legacy_transcript(folder.path(), "2026-09-30T08:00:00+00:00");
    let store = store_in(folder.path());

    let listed: Vec<SessionId> = store.list(10).unwrap().into_iter().map(|s| s.id).collect();

    assert_eq!(listed, vec![id.clone()]);
    assert_eq!(store.read(&id).unwrap().header.id, id);
}

#[test]
fn a_transcript_saved_before_month_folders_is_still_marked_incomplete_on_launch() {
    let folder = tempfile::tempdir().unwrap();
    let id = write_legacy_transcript(folder.path(), "2026-09-30T08:00:00+00:00");
    let store = store_in(folder.path());

    let marked = store.mark_abandoned_incomplete().unwrap();

    assert_eq!(marked, 1);
    assert_eq!(
        store.read(&id).unwrap().header.status,
        SessionStatus::Incomplete
    );
    assert!(folder.path().join(format!("{id}.md")).is_file());
}

#[test]
fn transcripts_across_two_months_are_listed_newest_first() {
    let folder = tempfile::tempdir().unwrap();
    let store = store_in(folder.path());
    store
        .create(&header_at("2026-09-30T08:00:00+00:00"))
        .unwrap();
    store
        .create(&header_at("2026-10-01T08:00:00+00:00"))
        .unwrap();
    write_legacy_transcript(folder.path(), "2026-08-15T08:00:00+00:00");

    assert_eq!(
        listed_ids(&store),
        [
            "2026-10-01-080000",
            "2026-09-30-080000",
            "2026-08-15-080000"
        ]
    );
}

#[test]
fn folders_that_are_not_months_are_not_searched_for_transcripts() {
    let folder = tempfile::tempdir().unwrap();
    for name in ["notes", "2026-1", "2026-10-01"] {
        let inside = folder.path().join(name);
        fs::create_dir(&inside).unwrap();
        fs::write(
            inside.join("2026-10-19-114521.md"),
            render_header(&header_at("2026-10-19T11:45:21+00:00")),
        )
        .unwrap();
    }
    let store = store_in(folder.path());

    assert!(store.list(10).unwrap().is_empty());
    assert_eq!(store.mark_abandoned_incomplete().unwrap(), 0);
}

#[test]
fn a_month_folder_nested_inside_a_month_folder_is_not_searched() {
    let folder = tempfile::tempdir().unwrap();
    let nested = folder.path().join("2026-10").join("2026-09");
    fs::create_dir_all(&nested).unwrap();
    fs::write(
        nested.join("2026-09-01-080000.md"),
        render_header(&header_at("2026-09-01T08:00:00+00:00")),
    )
    .unwrap();

    assert!(store_in(folder.path()).list(10).unwrap().is_empty());
}

#[test]
fn the_path_of_a_legacy_transcript_is_found_in_the_root_folder() {
    let folder = tempfile::tempdir().unwrap();
    let id = write_legacy_transcript(folder.path(), "2026-09-30T08:00:00+00:00");

    let path = store_in(folder.path()).path_for(&id);

    assert_eq!(path, folder.path().join(format!("{id}.md")));
}

#[test]
fn the_path_of_a_transcript_that_does_not_exist_points_into_its_month_folder() {
    let folder = tempfile::tempdir().unwrap();
    let id = SessionId::parse("2026-10-19-114521").unwrap();

    let path = store_in(folder.path()).path_for(&id);

    assert_eq!(
        path,
        folder.path().join("2026-10").join("2026-10-19-114521.md")
    );
}
