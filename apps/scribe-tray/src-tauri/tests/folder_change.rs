use std::sync::Arc;

use scribe_core::recording::{Recorder, SessionId};
use scribe_core::testing::{InMemoryTranscriptStore, RecordingListenerSpy};
use scribe_tray::recording::RecordingController;
use scribe_tray::transcripts_location::FolderChange;
use serde_json::json;
use store_markdown::{MarkdownTranscriptStore, TranscriptsFolder};

fn idle_controller() -> RecordingController {
    let listener = Arc::new(RecordingListenerSpy::default());
    let recorder = Recorder::new(
        Arc::new(InMemoryTranscriptStore::default()),
        listener.clone(),
    );
    RecordingController::new(recorder, listener)
}

#[test]
fn cancelled_change_serialises_to_its_outcome_alone() {
    let wire = serde_json::to_value(FolderChange::Cancelled).unwrap();

    assert_eq!(wire, json!({"outcome": "cancelled"}));
}

#[test]
fn unchanged_change_serialises_to_its_outcome_alone() {
    let wire = serde_json::to_value(FolderChange::Unchanged).unwrap();

    assert_eq!(wire, json!({"outcome": "unchanged"}));
}

#[test]
fn recording_change_serialises_to_its_outcome_alone() {
    let wire = serde_json::to_value(FolderChange::Recording).unwrap();

    assert_eq!(wire, json!({"outcome": "recording"}));
}

#[test]
fn changed_change_serialises_with_folder_and_move_counts() {
    let changed = FolderChange::Changed {
        folder: "/x".to_owned(),
        moved: 2,
        kept: 1,
    };

    let wire = serde_json::to_value(changed).unwrap();

    assert_eq!(
        wire,
        json!({"outcome": "changed", "folder": "/x", "moved": 2, "kept": 1})
    );
}

#[test]
fn relocating_an_idle_store_changes_where_new_transcripts_are_saved() {
    let old_root = tempfile::tempdir().unwrap();
    let new_root = tempfile::tempdir().unwrap();
    let store = MarkdownTranscriptStore::new(TranscriptsFolder::at(old_root.path().to_path_buf()));

    idle_controller()
        .relocate_store(&store, TranscriptsFolder::at(new_root.path().to_path_buf()))
        .unwrap();

    let id = SessionId::parse("2026-10-19-100000").unwrap();
    assert!(store.path_for(&id).starts_with(new_root.path()));
}
