use std::fs;
use std::path::{Path, PathBuf};

use store_markdown::{TRANSCRIPTS_FOLDER_KEY, TranscriptsFolder};

fn chosen_folder() -> PathBuf {
    std::env::temp_dir().join("chosen-transcripts")
}

fn settings_with(contents: &str) -> (tempfile::TempDir, PathBuf) {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("settings.json");
    fs::write(&path, contents).unwrap();
    (directory, path)
}

fn settings_with_folder_value(value: &serde_json::Value) -> (tempfile::TempDir, PathBuf) {
    settings_with(&serde_json::json!({ TRANSCRIPTS_FOLDER_KEY: value }).to_string())
}

fn resolved(settings_path: &Path) -> TranscriptsFolder {
    TranscriptsFolder::configured_in(settings_path).unwrap()
}

fn default_folder() -> TranscriptsFolder {
    TranscriptsFolder::default_location().unwrap()
}

#[test]
fn no_settings_file_uses_the_default_folder() {
    let directory = tempfile::tempdir().unwrap();

    let folder = resolved(&directory.path().join("settings.json"));

    assert_eq!(folder, default_folder());
}

#[test]
fn a_chosen_folder_is_used() {
    let (_directory, settings) =
        settings_with_folder_value(&chosen_folder().to_str().unwrap().into());

    assert_eq!(resolved(&settings), TranscriptsFolder::at(chosen_folder()));
}

#[test]
fn an_unreadable_settings_file_uses_the_default_folder() {
    let (_directory, settings) = settings_with("{ not json");

    assert_eq!(resolved(&settings), default_folder());
}

#[test]
fn settings_without_a_chosen_folder_use_the_default_folder() {
    let (_directory, settings) = settings_with(r#"{"other": "value"}"#);

    assert_eq!(resolved(&settings), default_folder());
}

#[test]
fn a_chosen_folder_that_is_not_text_uses_the_default_folder() {
    let (_directory, settings) = settings_with_folder_value(&serde_json::json!(42));

    assert_eq!(resolved(&settings), default_folder());
}

#[test]
fn an_empty_chosen_folder_uses_the_default_folder() {
    let (_directory, settings) = settings_with_folder_value(&"".into());

    assert_eq!(resolved(&settings), default_folder());
}

#[test]
fn a_relative_chosen_folder_uses_the_default_folder() {
    let (_directory, settings) = settings_with_folder_value(&"relative/transcripts".into());

    assert_eq!(resolved(&settings), default_folder());
}

#[test]
fn other_settings_are_ignored_when_reading_the_chosen_folder() {
    let chosen = chosen_folder();
    let (_directory, settings) = settings_with(
        &serde_json::json!({
            "privacy_acknowledged": true,
            TRANSCRIPTS_FOLDER_KEY: chosen.to_str().unwrap(),
        })
        .to_string(),
    );

    assert_eq!(resolved(&settings), TranscriptsFolder::at(chosen));
}
