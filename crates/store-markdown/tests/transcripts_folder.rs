use std::fs;

use store_markdown::TranscriptsFolder;

#[test]
fn creates_the_transcripts_folder_when_it_is_missing() {
    let parent = tempfile::tempdir().unwrap();
    let folder = TranscriptsFolder::at(parent.path().join("Scribe"));

    let created = folder.ensure_exists().unwrap().to_path_buf();

    assert!(created.is_dir());
    assert_eq!(created, parent.path().join("Scribe"));
}

#[cfg(unix)]
#[test]
fn keeps_the_transcripts_folder_private_to_the_user() {
    use std::os::unix::fs::PermissionsExt;
    let parent = tempfile::tempdir().unwrap();
    let folder = TranscriptsFolder::at(parent.path().join("Scribe"));

    let created = folder.ensure_exists().unwrap();

    let mode = fs::metadata(created).unwrap().permissions().mode();
    assert_eq!(mode & 0o777, 0o700);
}

#[test]
fn leaves_an_existing_transcripts_folder_untouched() {
    let parent = tempfile::tempdir().unwrap();
    let path = parent.path().join("Scribe");
    fs::create_dir(&path).unwrap();
    fs::write(path.join("kept.md"), "earlier transcript").unwrap();

    TranscriptsFolder::at(path.clone()).ensure_exists().unwrap();

    assert_eq!(
        fs::read_to_string(path.join("kept.md")).unwrap(),
        "earlier transcript"
    );
}

#[test]
fn reports_an_error_when_the_folder_cannot_be_created() {
    let parent = tempfile::tempdir().unwrap();
    let blocking_file = parent.path().join("Scribe");
    fs::write(&blocking_file, "not a folder").unwrap();

    let folder = TranscriptsFolder::at(blocking_file);

    assert!(folder.ensure_exists().is_err());
}

#[test]
fn default_location_is_a_scribe_folder_in_documents() {
    let folder = TranscriptsFolder::default_location().unwrap();

    assert!(folder.path().ends_with("Scribe"));
}
