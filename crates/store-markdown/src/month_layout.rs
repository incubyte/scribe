use std::fs;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};

use scribe_core::ports::StoreError;
use scribe_core::recording::SessionId;

use crate::store_error::{file_name, io_error};

const TRANSCRIPT_EXTENSION: &str = "md";
const MONTH_NAME_LENGTH: usize = 7;
const MONTH_SEPARATOR_INDEX: usize = 4;

pub(crate) fn month_path(folder: &Path, id: &SessionId) -> PathBuf {
    folder.join(id.month()).join(transcript_file_name(id))
}

pub(crate) fn legacy_path(folder: &Path, id: &SessionId) -> PathBuf {
    folder.join(transcript_file_name(id))
}

pub(crate) fn existing_path(folder: &Path, id: &SessionId) -> PathBuf {
    let in_month = month_path(folder, id);
    if in_month.is_file() {
        return in_month;
    }
    let legacy = legacy_path(folder, id);
    if legacy.is_file() { legacy } else { in_month }
}

pub(crate) fn transcript_paths_in(folder: &Path) -> Result<Vec<PathBuf>, StoreError> {
    let mut paths = Vec::new();
    for path in children_of(folder)? {
        if is_month_folder(&path) {
            paths.extend(transcripts_in_month_folder(&path));
        } else if is_transcript(&path) {
            paths.push(path);
        }
    }
    Ok(paths)
}

pub(crate) fn session_files_in(folder: &Path) -> Result<Vec<(SessionId, PathBuf)>, StoreError> {
    Ok(transcript_paths_in(folder)?
        .into_iter()
        .filter_map(|path| Some((session_id_of(&path)?, path)))
        .collect())
}

fn session_id_of(path: &Path) -> Option<SessionId> {
    SessionId::parse(path.file_stem()?.to_str()?).ok()
}

fn transcript_file_name(id: &SessionId) -> String {
    format!("{id}.{TRANSCRIPT_EXTENSION}")
}

fn children_of(folder: &Path) -> Result<Vec<PathBuf>, StoreError> {
    match fs::read_dir(folder) {
        Ok(entries) => Ok(entries
            .filter_map(Result::ok)
            .map(|entry| entry.path())
            .collect()),
        Err(error) if error.kind() == ErrorKind::NotFound => Ok(Vec::new()),
        Err(source) => Err(io_error(folder, source)),
    }
}

fn transcripts_in_month_folder(month_folder: &Path) -> Vec<PathBuf> {
    let Ok(children) = children_of(month_folder) else {
        tracing::warn!(folder = %file_name(month_folder), "skipped an unreadable month folder");
        return Vec::new();
    };
    children
        .into_iter()
        .filter(|path| is_transcript(path))
        .collect()
}

fn is_month_folder(path: &Path) -> bool {
    path.is_dir() && is_month_name(&file_name(path))
}

fn is_month_name(name: &str) -> bool {
    let bytes = name.as_bytes();
    bytes.len() == MONTH_NAME_LENGTH
        && bytes[MONTH_SEPARATOR_INDEX] == b'-'
        && bytes
            .iter()
            .enumerate()
            .all(|(index, byte)| index == MONTH_SEPARATOR_INDEX || byte.is_ascii_digit())
}

fn is_transcript(path: &Path) -> bool {
    path.is_file()
        && path
            .extension()
            .is_some_and(|extension| extension == TRANSCRIPT_EXTENSION)
}
