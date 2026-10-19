use std::ffi::OsString;
use std::fs;
use std::io::{self, ErrorKind};
use std::path::{Path, PathBuf};

use scribe_core::ports::StoreError;
use scribe_core::recording::SessionId;

use crate::month_layout::{month_path, session_files_in};
use crate::store_error::file_name;
use crate::transcripts_folder::TranscriptsFolder;

const PARTIAL_COPY_SUFFIX: &str = ".part";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct MoveReport {
    pub moved: usize,
    pub kept: usize,
}

enum MoveOutcome {
    Moved,
    Kept,
}

pub fn count_transcripts(folder: &TranscriptsFolder) -> Result<usize, StoreError> {
    Ok(session_files_in(folder.path())?.len())
}

pub fn move_transcripts(
    from: &TranscriptsFolder,
    to: &TranscriptsFolder,
) -> Result<MoveReport, StoreError> {
    let mut report = MoveReport::default();
    for (id, source) in session_files_in(from.path())? {
        match move_into_month(to, &id, &source) {
            Some(MoveOutcome::Moved) => report.moved += 1,
            Some(MoveOutcome::Kept) | None => {
                tracing::warn!(file = %file_name(&source), "kept a transcript in the old folder");
                report.kept += 1;
            }
        }
    }
    Ok(report)
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
    let installed = fs::copy(source, &partial).and_then(|_| fs::hard_link(&partial, destination));
    let _ = fs::remove_file(&partial);
    match installed {
        Ok(()) => fs::remove_file(source).map(|()| MoveOutcome::Moved),
        Err(error) if error.kind() == ErrorKind::AlreadyExists => Ok(MoveOutcome::Kept),
        Err(error) => Err(error),
    }
}

fn partial_copy_path(destination: &Path) -> PathBuf {
    let mut name = OsString::from(destination.as_os_str());
    name.push(PARTIAL_COPY_SUFFIX);
    PathBuf::from(name)
}
