use std::ffi::OsString;
use std::fs;
use std::io::{self, ErrorKind};
use std::path::{Path, PathBuf};

use scribe_core::ports::StoreError;
use scribe_core::recording::SessionId;

use crate::month_layout::{month_path, session_folder_path, stored_sessions_in};
use crate::private_file;
use crate::store_error::file_name;
use crate::transcripts_folder::{TranscriptsFolder, private_dir_builder};

const PARTIAL_COPY_SUFFIX: &str = ".part";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct MoveReport {
    pub moved: usize,
    pub kept: usize,
}

#[derive(Debug, Clone, Copy)]
enum MoveOutcome {
    Moved,
    Kept,
}

pub fn count_transcripts(folder: &TranscriptsFolder) -> Result<usize, StoreError> {
    let stored = stored_sessions_in(folder.path())?;
    Ok(stored.loose_sessions().len() + stored.session_folders().len())
}

pub fn move_transcripts(
    from: &TranscriptsFolder,
    to: &TranscriptsFolder,
) -> Result<MoveReport, StoreError> {
    let stored = stored_sessions_in(from.path())?;
    let mut report = MoveReport::default();
    for (id, source) in stored.loose_sessions() {
        tally(&mut report, &source, move_into_month(to, &id, &source));
    }
    for (id, source) in stored.session_folders() {
        tally(&mut report, source, move_session_folder(to, id, source));
    }
    Ok(report)
}

fn tally(report: &mut MoveReport, source: &Path, outcome: Option<MoveOutcome>) {
    match outcome {
        Some(MoveOutcome::Moved) => report.moved += 1,
        Some(MoveOutcome::Kept) | None => {
            tracing::warn!(file = %file_name(source), "kept a transcript in the old folder");
            report.kept += 1;
        }
    }
}

fn move_session_folder(
    to: &TranscriptsFolder,
    id: &SessionId,
    source: &Path,
) -> Option<MoveOutcome> {
    to.ensure_month_exists(id).ok()?;
    move_folder(source, &session_folder_path(to.path(), id)).ok()
}

fn move_folder(source: &Path, destination: &Path) -> io::Result<MoveOutcome> {
    if destination.symlink_metadata().is_ok() {
        return Ok(MoveOutcome::Kept);
    }
    fs::rename(source, destination)
        .map(|()| MoveOutcome::Moved)
        .or_else(|error| copy_folder_if_crossing_volumes(error, source, destination))
}

fn copy_folder_if_crossing_volumes(
    refusal: io::Error,
    source: &Path,
    destination: &Path,
) -> io::Result<MoveOutcome> {
    if refusal.kind() == ErrorKind::CrossesDevices {
        copy_folder_across_volumes(source, destination)
    } else {
        Err(refusal)
    }
}

fn copy_folder_across_volumes(source: &Path, destination: &Path) -> io::Result<MoveOutcome> {
    let partial = partial_copy_path(destination);
    private_dir_builder().create(&partial)?;
    let installed = copy_tree(source, &partial)
        .and_then(|()| rename_into_free_destination(&partial, destination));
    if installed.is_err() {
        let _ = fs::remove_dir_all(&partial);
    }
    match installed {
        Ok(()) => fs::remove_dir_all(source).map(|()| MoveOutcome::Moved),
        Err(error) if error.kind() == ErrorKind::AlreadyExists => Ok(MoveOutcome::Kept),
        Err(error) => Err(error),
    }
}

fn copy_tree(source: &Path, destination: &Path) -> io::Result<()> {
    for entry in fs::read_dir(source)? {
        let entry = entry?;
        let target = destination.join(entry.file_name());
        let kind = entry.file_type()?;
        if kind.is_dir() {
            private_dir_builder().create(&target)?;
            copy_tree(&entry.path(), &target)?;
        } else if kind.is_file() {
            private_file::copy_new(&entry.path(), &target)?;
        } else {
            return Err(io::Error::from(ErrorKind::Unsupported));
        }
    }
    Ok(())
}

fn move_into_month(to: &TranscriptsFolder, id: &SessionId, source: &Path) -> Option<MoveOutcome> {
    to.ensure_month_exists(id).ok()?;
    move_one(source, &month_path(to.path(), id)).ok()
}

fn move_one(source: &Path, destination: &Path) -> io::Result<MoveOutcome> {
    match fs::hard_link(source, destination) {
        Ok(()) => fs::remove_file(source).map(|()| MoveOutcome::Moved),
        Err(error) => match error.kind() {
            ErrorKind::AlreadyExists => Ok(MoveOutcome::Kept),
            ErrorKind::CrossesDevices => copy_across_volumes(source, destination),
            ErrorKind::Unsupported => rename_unless_present(source, destination),
            _ => Err(error),
        },
    }
}

fn rename_unless_present(source: &Path, destination: &Path) -> io::Result<MoveOutcome> {
    if destination.symlink_metadata().is_ok() {
        return Ok(MoveOutcome::Kept);
    }
    match fs::rename(source, destination) {
        Ok(()) => Ok(MoveOutcome::Moved),
        Err(error) if error.kind() == ErrorKind::CrossesDevices => {
            copy_across_volumes(source, destination)
        }
        Err(error) => Err(error),
    }
}

fn copy_across_volumes(source: &Path, destination: &Path) -> io::Result<MoveOutcome> {
    let partial = partial_copy_path(destination);
    let installed = fs::copy(source, &partial).and_then(|_| install(&partial, destination));
    let _ = fs::remove_file(&partial);
    match installed {
        Ok(()) => fs::remove_file(source).map(|()| MoveOutcome::Moved),
        Err(error) if error.kind() == ErrorKind::AlreadyExists => Ok(MoveOutcome::Kept),
        Err(error) => Err(error),
    }
}

fn install(partial: &Path, destination: &Path) -> io::Result<()> {
    match fs::hard_link(partial, destination) {
        Err(error) if error.kind() == ErrorKind::Unsupported => {
            rename_into_free_destination(partial, destination)
        }
        linked => linked,
    }
}

fn rename_into_free_destination(partial: &Path, destination: &Path) -> io::Result<()> {
    if destination.symlink_metadata().is_ok() {
        return Err(io::Error::from(ErrorKind::AlreadyExists));
    }
    fs::rename(partial, destination)
}

fn partial_copy_path(destination: &Path) -> PathBuf {
    let mut name = OsString::from(destination.as_os_str());
    name.push(PARTIAL_COPY_SUFFIX);
    PathBuf::from(name)
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use super::*;

    const TRANSCRIPT_NAME: &str = "2026-10-19-114521.md";

    fn session_folder_with_screenshot(parent: &Path) -> PathBuf {
        let folder = parent.join("2026-10-19-114521");
        fs::create_dir(&folder).unwrap();
        fs::write(folder.join(TRANSCRIPT_NAME), "transcript").unwrap();
        fs::write(folder.join("00-00-04.png"), "screenshot").unwrap();
        folder
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

    fn complete_session() -> BTreeMap<String, String> {
        BTreeMap::from([
            (TRANSCRIPT_NAME.to_owned(), "transcript".to_owned()),
            ("00-00-04.png".to_owned(), "screenshot".to_owned()),
        ])
    }

    #[test]
    fn a_rename_refused_for_crossing_volumes_is_retried_as_a_copy() {
        let parent = tempfile::tempdir().unwrap();
        let source = session_folder_with_screenshot(parent.path());
        let destination = parent.path().join("copy");

        let outcome = copy_folder_if_crossing_volumes(
            io::Error::from(ErrorKind::CrossesDevices),
            &source,
            &destination,
        )
        .unwrap();

        assert!(matches!(outcome, MoveOutcome::Moved));
        assert_eq!(files_in(&destination), complete_session());
    }

    #[test]
    fn a_rename_refused_for_any_other_reason_is_not_retried_as_a_copy() {
        let parent = tempfile::tempdir().unwrap();
        let source = session_folder_with_screenshot(parent.path());
        let destination = parent.path().join("copy");

        let outcome = copy_folder_if_crossing_volumes(
            io::Error::from(ErrorKind::PermissionDenied),
            &source,
            &destination,
        );

        assert_eq!(outcome.unwrap_err().kind(), ErrorKind::PermissionDenied);
        assert!(!destination.exists());
        assert_eq!(files_in(&source), complete_session());
    }

    #[test]
    fn a_folder_copied_across_volumes_arrives_complete_before_the_source_is_removed() {
        let parent = tempfile::tempdir().unwrap();
        let source = session_folder_with_screenshot(parent.path());
        let destination = parent.path().join("copy");

        let outcome = copy_folder_across_volumes(&source, &destination).unwrap();

        assert!(matches!(outcome, MoveOutcome::Moved));
        assert_eq!(files_in(&destination), complete_session());
        assert!(!source.exists());
    }

    fn occupied_destination(parent: &Path) -> PathBuf {
        let destination = parent.join("copy");
        fs::create_dir(&destination).unwrap();
        fs::write(destination.join("mine.md"), "already here").unwrap();
        destination
    }

    #[test]
    fn a_folder_copied_across_volumes_onto_an_existing_destination_leaves_both_untouched() {
        let parent = tempfile::tempdir().unwrap();
        let source = session_folder_with_screenshot(parent.path());
        let destination = occupied_destination(parent.path());

        let outcome = copy_folder_across_volumes(&source, &destination).unwrap();

        assert!(matches!(outcome, MoveOutcome::Kept));
        assert_eq!(files_in(&source), complete_session());
        assert_eq!(
            files_in(&destination),
            BTreeMap::from([("mine.md".to_owned(), "already here".to_owned())])
        );
    }

    #[test]
    fn a_folder_copied_across_volumes_onto_an_existing_destination_leaves_no_partial_folder() {
        let parent = tempfile::tempdir().unwrap();
        let source = session_folder_with_screenshot(parent.path());
        let destination = occupied_destination(parent.path());

        copy_folder_across_volumes(&source, &destination).unwrap();

        assert!(!partial_copy_path(&destination).exists());
    }

    #[cfg(unix)]
    #[test]
    fn a_copy_that_fails_midway_keeps_the_source_whole_and_leaves_no_partial_folder() {
        use std::os::unix::fs::PermissionsExt;
        let parent = tempfile::tempdir().unwrap();
        let source = session_folder_with_screenshot(parent.path());
        let destination = parent.path().join("copy");
        let unreadable = source.join("00-00-04.png");
        fs::set_permissions(&unreadable, fs::Permissions::from_mode(0o000)).unwrap();

        let result = copy_folder_across_volumes(&source, &destination);

        fs::set_permissions(&unreadable, fs::Permissions::from_mode(0o600)).unwrap();
        assert!(result.is_err());
        assert_eq!(files_in(&source), complete_session());
        assert!(!destination.exists());
        assert!(!partial_copy_path(&destination).exists());
    }
}
