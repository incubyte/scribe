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
fn a_new_transcript_is_saved_inside_its_own_session_folder_in_its_month_folder() {
    let folder = tempfile::tempdir().unwrap();
    let store = store_in(folder.path());

    let id = store
        .create(&header_at("2026-10-19T11:45:21+00:00"))
        .unwrap();

    let month_folder = folder.path().join("2026-10");
    assert!(
        month_folder
            .join(id.as_str())
            .join(format!("{id}.md"))
            .is_file()
    );
    assert!(!month_folder.join(format!("{id}.md")).exists());
}

#[test]
fn a_second_call_started_in_the_same_second_gets_its_own_session_folder_in_the_same_month_folder() {
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
            .join("2026-10-19-114521-2")
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
fn the_path_of_a_transcript_that_does_not_exist_points_into_its_session_folder() {
    let folder = tempfile::tempdir().unwrap();
    let id = SessionId::parse("2026-10-19-114521").unwrap();

    let path = store_in(folder.path()).path_for(&id);

    assert_eq!(
        path,
        folder
            .path()
            .join("2026-10")
            .join("2026-10-19-114521")
            .join("2026-10-19-114521.md")
    );
}

#[test]
fn after_relocating_the_store_new_transcripts_are_saved_in_the_new_folder_and_old_ones_are_not_listed()
 {
    let old_folder = tempfile::tempdir().unwrap();
    let new_folder = tempfile::tempdir().unwrap();
    let store = store_in(old_folder.path());
    store
        .create(&header_at("2026-10-19T11:45:21+00:00"))
        .unwrap();

    store.relocate(TranscriptsFolder::at(new_folder.path().to_path_buf()));
    let id = store
        .create(&header_at("2026-10-20T09:00:00+00:00"))
        .unwrap();

    assert!(
        new_folder
            .path()
            .join("2026-10")
            .join(id.as_str())
            .join(format!("{id}.md"))
            .is_file()
    );
    assert_eq!(listed_ids(&store), [id.to_string()]);
    assert_eq!(store.folder().path(), new_folder.path());
}

fn write_month_file_transcript(folder: &Path, started_at: &str) -> SessionId {
    let header = header_at(started_at);
    let month_folder = folder.join(header.id.month());
    fs::create_dir_all(&month_folder).unwrap();
    fs::write(
        month_folder.join(format!("{}.md", header.id)),
        render_header(&header),
    )
    .unwrap();
    header.id
}

fn write_session_folder_transcript(folder: &Path, started_at: &str) -> SessionId {
    let header = header_at(started_at);
    let session_folder = folder.join(header.id.month()).join(header.id.as_str());
    fs::create_dir_all(&session_folder).unwrap();
    fs::write(
        session_folder.join(format!("{}.md", header.id)),
        render_header(&header),
    )
    .unwrap();
    header.id
}

#[test]
fn sessions_stored_flat_as_month_files_or_in_session_folders_are_all_listed_and_read() {
    let folder = tempfile::tempdir().unwrap();
    let flat = write_legacy_transcript(folder.path(), "2026-08-15T08:00:00+00:00");
    let month_file = write_month_file_transcript(folder.path(), "2026-09-30T08:00:00+00:00");
    let session_folder =
        write_session_folder_transcript(folder.path(), "2026-10-01T08:00:00+00:00");
    let store = store_in(folder.path());

    assert_eq!(
        listed_ids(&store),
        [
            session_folder.to_string(),
            month_file.to_string(),
            flat.to_string()
        ]
    );
    for id in [&flat, &month_file, &session_folder] {
        assert_eq!(&store.read(id).unwrap().header.id, id);
    }
}

#[test]
fn a_session_left_recording_by_a_crash_is_marked_incomplete_in_every_layout() {
    let folder = tempfile::tempdir().unwrap();
    let flat = write_legacy_transcript(folder.path(), "2026-08-15T08:00:00+00:00");
    let month_file = write_month_file_transcript(folder.path(), "2026-09-30T08:00:00+00:00");
    let session_folder =
        write_session_folder_transcript(folder.path(), "2026-10-01T08:00:00+00:00");
    let store = store_in(folder.path());

    let marked = store.mark_abandoned_incomplete().unwrap();

    assert_eq!(marked, 3);
    for id in [&flat, &month_file, &session_folder] {
        assert_eq!(
            store.read(id).unwrap().header.status,
            SessionStatus::Incomplete
        );
    }
}

#[test]
fn opening_older_layouts_leaves_their_files_where_they_were() {
    let folder = tempfile::tempdir().unwrap();
    let flat = write_legacy_transcript(folder.path(), "2026-08-15T08:00:00+00:00");
    let month_file = write_month_file_transcript(folder.path(), "2026-09-30T08:00:00+00:00");
    let store = store_in(folder.path());

    store.list(10).unwrap();

    assert!(folder.path().join(format!("{flat}.md")).is_file());
    assert!(
        folder
            .path()
            .join("2026-09")
            .join(format!("{month_file}.md"))
            .is_file()
    );
    assert!(
        !folder
            .path()
            .join("2026-09")
            .join(month_file.as_str())
            .exists()
    );
}

#[test]
fn a_new_session_never_takes_an_id_whose_month_file_transcript_already_exists() {
    let folder = tempfile::tempdir().unwrap();
    let existing = write_month_file_transcript(folder.path(), "2026-10-19T11:45:21+00:00");
    let store = store_in(folder.path());

    let created = store
        .create(&header_at("2026-10-19T11:45:21+00:00"))
        .unwrap();

    assert_eq!(created.as_str(), "2026-10-19-114521-2");
    assert_eq!(store.read(&existing).unwrap().header.id, existing);
}

#[test]
fn a_folder_inside_a_month_folder_that_is_not_a_session_id_is_ignored() {
    let folder = tempfile::tempdir().unwrap();
    let stray = folder.path().join("2026-10").join("holiday-photos");
    fs::create_dir_all(&stray).unwrap();
    fs::write(
        stray.join("2026-10-19-114521.md"),
        render_header(&header_at("2026-10-19T11:45:21+00:00")),
    )
    .unwrap();
    let store = store_in(folder.path());

    assert!(store.list(10).unwrap().is_empty());
    assert_eq!(store.mark_abandoned_incomplete().unwrap(), 0);
}

#[test]
fn inside_a_session_folder_only_the_transcript_named_after_the_session_is_read() {
    let folder = tempfile::tempdir().unwrap();
    let id = write_session_folder_transcript(folder.path(), "2026-10-19T11:45:21+00:00");
    let session_folder = folder.path().join("2026-10").join(id.as_str());
    fs::write(session_folder.join("notes.md"), "my notes").unwrap();
    fs::write(session_folder.join("00-00-04.png"), "not a transcript").unwrap();
    fs::write(
        session_folder.join("2026-10-20-090000.md"),
        render_header(&header_at("2026-10-20T09:00:00+00:00")),
    )
    .unwrap();
    let store = store_in(folder.path());

    assert_eq!(listed_ids(&store), [id.to_string()]);
}

#[cfg(unix)]
#[test]
fn a_session_folder_is_private_to_the_user_and_so_is_its_transcript() {
    use std::os::unix::fs::PermissionsExt;
    let folder = tempfile::tempdir().unwrap();
    let store = store_in(&folder.path().join("Scribe"));

    let id = store
        .create(&header_at("2026-10-19T11:45:21+00:00"))
        .unwrap();

    let session_folder = folder
        .path()
        .join("Scribe")
        .join("2026-10")
        .join(id.as_str());
    let folder_mode = fs::metadata(&session_folder).unwrap().permissions().mode();
    let file_mode = fs::metadata(session_folder.join(format!("{id}.md")))
        .unwrap()
        .permissions()
        .mode();
    assert_eq!(folder_mode & 0o777, 0o700);
    assert_eq!(file_mode & 0o777, 0o600);
}
