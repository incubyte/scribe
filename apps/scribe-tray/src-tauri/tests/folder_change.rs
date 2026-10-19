use std::sync::Arc;

use scribe_core::recording::{Recorder, SessionId};
use scribe_core::testing::{InMemoryTranscriptStore, RecordingListenerSpy};
use scribe_tray::recording::RecordingController;
use scribe_tray::transcripts_location::{FolderChange, folder_changed_message};
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
fn a_cancelled_pick_tells_the_popover_nothing_changed() {
    let wire = serde_json::to_value(FolderChange::Cancelled).unwrap();

    assert_eq!(wire, json!({"outcome": "cancelled"}));
}

#[test]
fn choosing_the_current_folder_tells_the_popover_nothing_changed() {
    let wire = serde_json::to_value(FolderChange::Unchanged).unwrap();

    assert_eq!(wire, json!({"outcome": "unchanged"}));
}

#[test]
fn a_refused_change_during_a_recording_tells_the_popover_why() {
    let wire = serde_json::to_value(FolderChange::Recording).unwrap();

    assert_eq!(wire, json!({"outcome": "recording"}));
}

#[test]
fn a_completed_change_tells_the_popover_the_new_folder_and_how_many_transcripts_moved() {
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

#[test]
fn a_change_that_moved_nothing_only_names_the_new_folder() {
    assert_eq!(
        folder_changed_message("Notes", 0, 0),
        "New transcripts will be saved in Notes."
    );
}

#[test]
fn a_change_that_moved_one_transcript_says_so_in_the_singular() {
    assert_eq!(
        folder_changed_message("Notes", 1, 0),
        "New transcripts will be saved in Notes. Moved 1 transcript."
    );
}

#[test]
fn a_change_that_moved_many_transcripts_says_so_in_the_plural() {
    assert_eq!(
        folder_changed_message("Notes", 3, 0),
        "New transcripts will be saved in Notes. Moved 3 transcripts."
    );
}

#[test]
fn transcripts_left_behind_are_mentioned_after_the_moved_count() {
    assert_eq!(
        folder_changed_message("Notes", 2, 1),
        "New transcripts will be saved in Notes. Moved 2 transcripts. 1 transcript was left in the old folder."
    );
    assert_eq!(
        folder_changed_message("Notes", 0, 4),
        "New transcripts will be saved in Notes. 4 transcripts were left in the old folder."
    );
}
