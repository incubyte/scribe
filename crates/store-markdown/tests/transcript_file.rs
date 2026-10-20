use std::fs;
use std::path::Path;
use std::time::Duration;

use chrono::{DateTime, FixedOffset};
use scribe_core::ports::TranscriptStore;
use scribe_core::recording::{Segment, SessionHeader, SessionId, SessionStatus, Speaker};
use store_markdown::{MarkdownTranscriptStore, TranscriptsFolder, parse_header, render_header};

fn call_header() -> SessionHeader {
    let started_at = DateTime::parse_from_rfc3339("2026-10-17T14:30:05+05:30").unwrap();
    SessionHeader::recording(started_at, "speechanalyzer", "en-US")
}

fn store_in(parent: &Path) -> MarkdownTranscriptStore {
    MarkdownTranscriptStore::new(TranscriptsFolder::at(parent.join("Scribe")))
}

fn transcript_text(parent: &Path, id: &SessionId) -> String {
    fs::read_to_string(
        parent
            .join("Scribe")
            .join(id.month())
            .join(id.as_str())
            .join(format!("{id}.md")),
    )
    .unwrap()
}

fn my_line(start_secs: u64, text: &str) -> Segment {
    Segment {
        speaker: Speaker::Me,
        start: Duration::from_secs(start_secs),
        end: Duration::from_secs(start_secs + 2),
        text: text.to_owned(),
    }
}

fn status_of(parent: &Path, id: &SessionId) -> SessionStatus {
    parse_header(&transcript_text(parent, id)).unwrap().status
}

fn ended_at() -> DateTime<FixedOffset> {
    DateTime::parse_from_rfc3339("2026-10-17T15:02:41+05:30").unwrap()
}

#[test]
fn a_new_transcript_starts_with_every_header_key_and_a_quoted_title() {
    let parent = tempfile::tempdir().unwrap();
    let store = store_in(parent.path());

    let id = store.create(&call_header()).unwrap();

    assert_eq!(
        transcript_text(parent.path(), &id),
        "---\n\
         id: 2026-10-17-143005\n\
         title: \"Call 2026-10-17 14:30\"\n\
         started_at: 2026-10-17T14:30:05+05:30\n\
         ended_at:\n\
         status: recording\n\
         engine: speechanalyzer\n\
         language: en-US\n\
         ---\n"
    );
}

#[test]
fn each_finished_utterance_is_appended_as_a_timestamped_speaker_line() {
    let parent = tempfile::tempdir().unwrap();
    let store = store_in(parent.path());
    let id = store.create(&call_header()).unwrap();

    store
        .append(&id, &my_line(4, "Hi everyone, thanks for joining."))
        .unwrap();
    store
        .append(&id, &my_line(3_725, "Let's wrap up."))
        .unwrap();

    let text = transcript_text(parent.path(), &id);
    assert!(text.ends_with(
        "---\n\
         [00:00:04] **Me:** Hi everyone, thanks for joining.\n\
         [01:02:05] **Me:** Let's wrap up.\n"
    ));
}

#[test]
fn line_breaks_inside_an_utterance_become_spaces() {
    let parent = tempfile::tempdir().unwrap();
    let store = store_in(parent.path());
    let id = store.create(&call_header()).unwrap();

    store
        .append(&id, &my_line(9, "First part\nsecond part\r\nthird"))
        .unwrap();

    assert!(
        transcript_text(parent.path(), &id)
            .ends_with("[00:00:09] **Me:** First part second part third\n")
    );
}

#[test]
fn finishing_a_session_records_its_end_and_keeps_every_line() {
    let parent = tempfile::tempdir().unwrap();
    let store = store_in(parent.path());
    let id = store.create(&call_header()).unwrap();
    store.append(&id, &my_line(4, "Hello")).unwrap();
    store.append(&id, &my_line(9, "Goodbye")).unwrap();

    store.finalise(&id, ended_at()).unwrap();

    let text = transcript_text(parent.path(), &id);
    let header = parse_header(&text).unwrap();
    assert_eq!(header.status, SessionStatus::Complete);
    assert_eq!(header.ended_at, Some(ended_at()));
    assert!(text.ends_with("---\n[00:00:04] **Me:** Hello\n[00:00:09] **Me:** Goodbye\n"));
}

#[test]
fn finishing_a_session_leaves_no_temporary_file_behind() {
    let parent = tempfile::tempdir().unwrap();
    let store = store_in(parent.path());
    let id = store.create(&call_header()).unwrap();

    store.finalise(&id, ended_at()).unwrap();

    let session_folder = parent
        .path()
        .join("Scribe")
        .join("2026-10")
        .join(id.as_str());
    let names: Vec<_> = fs::read_dir(session_folder)
        .unwrap()
        .map(|entry| entry.unwrap().file_name())
        .collect();
    assert_eq!(names, vec!["2026-10-17-143005.md"]);
}

#[test]
fn a_session_left_recording_by_a_crash_is_marked_incomplete_on_launch() {
    let parent = tempfile::tempdir().unwrap();
    let crashed = store_in(parent.path()).create(&call_header()).unwrap();
    store_in(parent.path())
        .append(&crashed, &my_line(4, "Said before the crash"))
        .unwrap();

    let marked = store_in(parent.path()).mark_abandoned_incomplete().unwrap();

    assert_eq!(marked, 1);
    assert_eq!(
        status_of(parent.path(), &crashed),
        SessionStatus::Incomplete
    );
    assert!(
        transcript_text(parent.path(), &crashed)
            .ends_with("---\n[00:00:04] **Me:** Said before the crash\n")
    );
}

#[test]
fn a_finished_session_stays_complete_on_launch() {
    let parent = tempfile::tempdir().unwrap();
    let finished = store_in(parent.path()).create(&call_header()).unwrap();
    store_in(parent.path())
        .finalise(&finished, ended_at())
        .unwrap();

    let marked = store_in(parent.path()).mark_abandoned_incomplete().unwrap();

    assert_eq!(marked, 0);
    assert_eq!(status_of(parent.path(), &finished), SessionStatus::Complete);
}

#[test]
fn a_file_that_is_not_a_transcript_is_left_alone_on_launch() {
    let parent = tempfile::tempdir().unwrap();
    let folder = parent.path().join("Scribe");
    fs::create_dir(&folder).unwrap();
    fs::write(folder.join("notes.md"), "just my own notes").unwrap();

    let marked = store_in(parent.path()).mark_abandoned_incomplete().unwrap();

    assert_eq!(marked, 0);
    assert_eq!(
        fs::read_to_string(folder.join("notes.md")).unwrap(),
        "just my own notes"
    );
}

#[test]
fn launching_before_any_call_was_recorded_marks_nothing() {
    let parent = tempfile::tempdir().unwrap();

    assert_eq!(
        store_in(parent.path()).mark_abandoned_incomplete().unwrap(),
        0
    );
}

#[cfg(unix)]
#[test]
fn transcript_files_are_private_to_the_user() {
    use std::os::unix::fs::PermissionsExt;
    let parent = tempfile::tempdir().unwrap();
    let store = store_in(parent.path());
    let id = store.create(&call_header()).unwrap();
    let path = parent
        .path()
        .join("Scribe")
        .join("2026-10")
        .join(id.as_str())
        .join(format!("{id}.md"));
    let created_mode = fs::metadata(&path).unwrap().permissions().mode();

    store.finalise(&id, ended_at()).unwrap();

    assert_eq!(created_mode & 0o777, 0o600);
    assert_eq!(
        fs::metadata(&path).unwrap().permissions().mode() & 0o777,
        0o600
    );
}

#[test]
fn a_second_call_started_in_the_same_second_gets_its_own_file() {
    let parent = tempfile::tempdir().unwrap();
    let store = store_in(parent.path());

    let first = store.create(&call_header()).unwrap();
    let second = store.create(&call_header()).unwrap();

    assert_eq!(first.as_str(), "2026-10-17-143005");
    assert_eq!(second.as_str(), "2026-10-17-143005-2");
    assert_eq!(
        parse_header(&transcript_text(parent.path(), &second))
            .unwrap()
            .id,
        second
    );
}

#[test]
fn a_header_reads_back_exactly_as_it_was_written() {
    let mut header = call_header();
    header.title = "Weekly sync: \"Q4\" plans \\ budget".to_owned();
    header.ended_at = Some(ended_at());
    header.status = SessionStatus::Complete;

    assert_eq!(parse_header(&render_header(&header)).unwrap(), header);
}
