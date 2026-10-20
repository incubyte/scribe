use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

use store_markdown::{MoveReport, TranscriptsFolder, count_transcripts, move_transcripts};

const OCTOBER_ID: &str = "2026-10-19-114521";
const SEPTEMBER_ID: &str = "2026-09-30-235959";

fn folder_in(parent: &Path, name: &str) -> TranscriptsFolder {
    let path = parent.join(name);
    fs::create_dir_all(&path).unwrap();
    TranscriptsFolder::at(path)
}

fn write_flat(folder: &TranscriptsFolder, id: &str, contents: &str) {
    fs::write(folder.path().join(format!("{id}.md")), contents).unwrap();
}

fn write_in_month(folder: &TranscriptsFolder, id: &str, contents: &str) {
    let month = folder.path().join(&id[..7]);
    fs::create_dir_all(&month).unwrap();
    fs::write(month.join(format!("{id}.md")), contents).unwrap();
}

fn month_file(folder: &TranscriptsFolder, id: &str) -> std::path::PathBuf {
    folder.path().join(&id[..7]).join(format!("{id}.md"))
}

#[test]
fn moving_to_a_new_folder_places_each_transcript_in_its_month_folder() {
    let parent = tempfile::tempdir().unwrap();
    let old = folder_in(parent.path(), "old");
    let new = folder_in(parent.path(), "new");
    write_in_month(&old, OCTOBER_ID, "october");
    write_in_month(&old, SEPTEMBER_ID, "september");

    let report = move_transcripts(&old, &new).unwrap();

    assert_eq!(report, MoveReport { moved: 2, kept: 0 });
    assert_eq!(
        fs::read_to_string(month_file(&new, OCTOBER_ID)).unwrap(),
        "october"
    );
    assert_eq!(
        fs::read_to_string(month_file(&new, SEPTEMBER_ID)).unwrap(),
        "september"
    );
}

#[test]
fn moved_transcripts_are_removed_from_the_old_folder() {
    let parent = tempfile::tempdir().unwrap();
    let old = folder_in(parent.path(), "old");
    let new = folder_in(parent.path(), "new");
    write_in_month(&old, OCTOBER_ID, "october");

    move_transcripts(&old, &new).unwrap();

    assert!(!month_file(&old, OCTOBER_ID).exists());
}

#[test]
fn legacy_flat_transcripts_are_moved_into_their_month_folder() {
    let parent = tempfile::tempdir().unwrap();
    let old = folder_in(parent.path(), "old");
    let new = folder_in(parent.path(), "new");
    write_flat(&old, OCTOBER_ID, "legacy");

    let report = move_transcripts(&old, &new).unwrap();

    assert_eq!(report, MoveReport { moved: 1, kept: 0 });
    assert_eq!(
        fs::read_to_string(month_file(&new, OCTOBER_ID)).unwrap(),
        "legacy"
    );
    assert!(!old.path().join(format!("{OCTOBER_ID}.md")).exists());
}

#[test]
fn a_transcript_already_present_in_the_new_folder_is_kept_in_the_old_folder() {
    let parent = tempfile::tempdir().unwrap();
    let old = folder_in(parent.path(), "old");
    let new = folder_in(parent.path(), "new");
    write_in_month(&old, OCTOBER_ID, "from the old folder");
    write_in_month(&new, OCTOBER_ID, "already in the new folder");

    let report = move_transcripts(&old, &new).unwrap();

    assert_eq!(report, MoveReport { moved: 0, kept: 1 });
    assert_eq!(
        fs::read_to_string(month_file(&new, OCTOBER_ID)).unwrap(),
        "already in the new folder"
    );
    assert_eq!(
        fs::read_to_string(month_file(&old, OCTOBER_ID)).unwrap(),
        "from the old folder"
    );
}

#[test]
fn files_that_are_not_transcripts_are_left_where_they_are() {
    let parent = tempfile::tempdir().unwrap();
    let old = folder_in(parent.path(), "old");
    let new = folder_in(parent.path(), "new");
    fs::write(old.path().join("notes.md"), "notes").unwrap();
    fs::write(old.path().join("todo.txt"), "todo").unwrap();

    let report = move_transcripts(&old, &new).unwrap();

    assert_eq!(report, MoveReport::default());
    assert!(old.path().join("notes.md").is_file());
    assert!(old.path().join("todo.txt").is_file());
    assert!(!new.path().join("notes.md").exists());
}

#[test]
fn an_empty_old_folder_moves_nothing() {
    let parent = tempfile::tempdir().unwrap();
    let old = folder_in(parent.path(), "old");
    let new = folder_in(parent.path(), "new");

    let report = move_transcripts(&old, &new).unwrap();

    assert_eq!(report, MoveReport::default());
    assert_eq!(fs::read_dir(new.path()).unwrap().count(), 0);
}

#[test]
fn the_count_of_transcripts_to_move_ignores_files_that_are_not_transcripts() {
    let parent = tempfile::tempdir().unwrap();
    let old = folder_in(parent.path(), "old");
    write_flat(&old, OCTOBER_ID, "legacy");
    write_in_month(&old, SEPTEMBER_ID, "september");
    fs::write(old.path().join("notes.md"), "notes").unwrap();

    assert_eq!(count_transcripts(&old).unwrap(), 2);
}

#[cfg(unix)]
#[test]
fn moved_transcripts_stay_private_to_the_user() {
    use std::os::unix::fs::PermissionsExt;

    let parent = tempfile::tempdir().unwrap();
    let old = folder_in(parent.path(), "old");
    let new = folder_in(parent.path(), "new");
    write_in_month(&old, OCTOBER_ID, "private");
    let source = month_file(&old, OCTOBER_ID);
    fs::set_permissions(&source, fs::Permissions::from_mode(0o600)).unwrap();

    move_transcripts(&old, &new).unwrap();

    let mode = fs::metadata(month_file(&new, OCTOBER_ID))
        .unwrap()
        .permissions()
        .mode();
    assert_eq!(mode & 0o777, 0o600);
}

#[test]
fn a_transcript_whose_month_folder_cannot_be_created_in_the_new_folder_is_kept_in_the_old_folder() {
    let parent = tempfile::tempdir().unwrap();
    let old = folder_in(parent.path(), "old");
    let new = folder_in(parent.path(), "new");
    write_in_month(&old, OCTOBER_ID, "october");
    fs::write(new.path().join(&OCTOBER_ID[..7]), "in the way").unwrap();

    let report = move_transcripts(&old, &new).unwrap();

    assert_eq!(report, MoveReport { moved: 0, kept: 1 });
    assert_eq!(
        fs::read_to_string(month_file(&old, OCTOBER_ID)).unwrap(),
        "october"
    );
}

#[cfg(unix)]
#[test]
fn a_dangling_link_already_at_the_destination_is_never_replaced() {
    let parent = tempfile::tempdir().unwrap();
    let old = folder_in(parent.path(), "old");
    let new = folder_in(parent.path(), "new");
    write_in_month(&old, OCTOBER_ID, "october");
    let destination = month_file(&new, OCTOBER_ID);
    fs::create_dir_all(destination.parent().unwrap()).unwrap();
    let missing_target = parent.path().join("nowhere");
    std::os::unix::fs::symlink(&missing_target, &destination).unwrap();

    let report = move_transcripts(&old, &new).unwrap();

    assert_eq!(report, MoveReport { moved: 0, kept: 1 });
    assert_eq!(
        fs::read_to_string(month_file(&old, OCTOBER_ID)).unwrap(),
        "october"
    );
    assert_eq!(fs::read_link(&destination).unwrap(), missing_target);
}

fn write_session_folder(
    folder: &TranscriptsFolder,
    id: &str,
    contents: &str,
) -> std::path::PathBuf {
    let session_folder = folder.path().join(&id[..7]).join(id);
    fs::create_dir_all(&session_folder).unwrap();
    fs::write(session_folder.join(format!("{id}.md")), contents).unwrap();
    session_folder
}

fn files_in(folder: &Path) -> BTreeMap<String, String> {
    let Ok(entries) = fs::read_dir(folder) else {
        return BTreeMap::new();
    };
    entries
        .map(|entry| entry.unwrap())
        .map(|entry| {
            let contents = fs::read_to_string(entry.path()).unwrap();
            (entry.file_name().to_string_lossy().into_owned(), contents)
        })
        .collect()
}

fn texts<const N: usize>(files: [(&str, &str); N]) -> BTreeMap<String, String> {
    files
        .into_iter()
        .map(|(name, contents)| (name.to_owned(), contents.to_owned()))
        .collect()
}

fn session_folder_in(folder: &TranscriptsFolder, id: &str) -> std::path::PathBuf {
    folder.path().join(&id[..7]).join(id)
}

#[test]
fn a_session_folder_moves_whole_with_every_file_inside_it() {
    let parent = tempfile::tempdir().unwrap();
    let old = folder_in(parent.path(), "old");
    let new = folder_in(parent.path(), "new");
    let source = write_session_folder(&old, OCTOBER_ID, "october");
    fs::write(source.join("00-00-04.png"), "first screenshot").unwrap();
    fs::write(source.join("00-01-10.png"), "second screenshot").unwrap();

    let report = move_transcripts(&old, &new).unwrap();

    let moved = session_folder_in(&new, OCTOBER_ID);
    assert_eq!(report, MoveReport { moved: 1, kept: 0 });
    assert_eq!(
        files_in(&moved),
        texts([
            ("2026-10-19-114521.md", "october"),
            ("00-00-04.png", "first screenshot"),
            ("00-01-10.png", "second screenshot"),
        ])
    );
    assert!(!source.exists());
}

#[test]
fn a_session_folder_whose_destination_already_exists_stays_where_it_is() {
    let parent = tempfile::tempdir().unwrap();
    let old = folder_in(parent.path(), "old");
    let new = folder_in(parent.path(), "new");
    let source = write_session_folder(&old, OCTOBER_ID, "from the old folder");
    fs::write(source.join("00-00-04.png"), "screenshot").unwrap();
    let existing = write_session_folder(&new, OCTOBER_ID, "already in the new folder");

    let report = move_transcripts(&old, &new).unwrap();

    assert_eq!(report, MoveReport { moved: 0, kept: 1 });
    assert_eq!(
        files_in(&source),
        texts([
            ("2026-10-19-114521.md", "from the old folder"),
            ("00-00-04.png", "screenshot"),
        ])
    );
    assert_eq!(
        files_in(&existing),
        texts([("2026-10-19-114521.md", "already in the new folder")])
    );
}

#[test]
fn an_empty_session_folder_waiting_at_the_destination_is_never_filled_over() {
    let parent = tempfile::tempdir().unwrap();
    let old = folder_in(parent.path(), "old");
    let new = folder_in(parent.path(), "new");
    let source = write_session_folder(&old, OCTOBER_ID, "october");
    fs::create_dir_all(session_folder_in(&new, OCTOBER_ID)).unwrap();

    let report = move_transcripts(&old, &new).unwrap();

    assert_eq!(report, MoveReport { moved: 0, kept: 1 });
    assert!(source.join(format!("{OCTOBER_ID}.md")).is_file());
}

#[test]
fn session_folders_flat_files_and_month_files_are_each_counted_once() {
    let parent = tempfile::tempdir().unwrap();
    let old = folder_in(parent.path(), "old");
    write_flat(&old, OCTOBER_ID, "legacy");
    write_in_month(&old, SEPTEMBER_ID, "september");
    let session = write_session_folder(&old, "2026-08-01-090000", "august");
    fs::write(session.join("00-00-04.png"), "screenshot").unwrap();
    fs::write(session.join("00-00-09.png"), "screenshot").unwrap();

    assert_eq!(count_transcripts(&old).unwrap(), 3);
}

#[cfg(unix)]
#[test]
fn a_moved_session_folder_stays_private_to_the_user() {
    use std::os::unix::fs::PermissionsExt;
    let parent = tempfile::tempdir().unwrap();
    let old = folder_in(parent.path(), "old");
    let new = folder_in(parent.path(), "new");
    let source = write_session_folder(&old, OCTOBER_ID, "private");
    fs::set_permissions(&source, fs::Permissions::from_mode(0o700)).unwrap();

    move_transcripts(&old, &new).unwrap();

    let mode = fs::metadata(session_folder_in(&new, OCTOBER_ID))
        .unwrap()
        .permissions()
        .mode();
    assert_eq!(mode & 0o777, 0o700);
}
