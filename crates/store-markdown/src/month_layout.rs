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

pub(crate) fn session_folder_path(folder: &Path, id: &SessionId) -> PathBuf {
    folder.join(id.month()).join(id.as_str())
}

pub(crate) fn session_path(folder: &Path, id: &SessionId) -> PathBuf {
    session_folder_path(folder, id).join(transcript_file_name(id))
}

pub(crate) fn has_older_layout_file(folder: &Path, id: &SessionId) -> bool {
    month_path(folder, id).exists() || legacy_path(folder, id).exists()
}

pub(crate) fn existing_path(folder: &Path, id: &SessionId) -> PathBuf {
    [
        session_path(folder, id),
        month_path(folder, id),
        legacy_path(folder, id),
    ]
    .into_iter()
    .find(|path| path.is_file())
    .unwrap_or_else(|| session_path(folder, id))
}

pub(crate) struct StoredSessions {
    loose_files: Vec<PathBuf>,
    session_folders: Vec<(SessionId, PathBuf)>,
}

impl StoredSessions {
    pub(crate) fn loose_sessions(&self) -> Vec<(SessionId, PathBuf)> {
        self.loose_files
            .iter()
            .filter_map(|path| Some((session_id_of(path)?, path.clone())))
            .collect()
    }

    pub(crate) fn session_folders(&self) -> &[(SessionId, PathBuf)] {
        &self.session_folders
    }

    pub(crate) fn transcript_paths(&self) -> Vec<PathBuf> {
        let in_folders = self
            .session_folders
            .iter()
            .map(|(id, session_folder)| session_folder.join(transcript_file_name(id)));
        self.loose_files.iter().cloned().chain(in_folders).collect()
    }

    fn absorb_month_folder(&mut self, month_folder: &Path) {
        let Ok(children) = children_of(month_folder) else {
            tracing::warn!(folder = %file_name(month_folder), "skipped an unreadable month folder");
            return;
        };
        for path in children {
            if is_transcript(&path) {
                self.loose_files.push(path);
            } else if let Some(id) = session_folder_id(&path) {
                self.session_folders.push((id, path));
            }
        }
    }
}

pub(crate) fn stored_sessions_in(folder: &Path) -> Result<StoredSessions, StoreError> {
    let mut stored = StoredSessions {
        loose_files: Vec::new(),
        session_folders: Vec::new(),
    };
    for path in children_of(folder)? {
        if is_month_folder(&path) {
            stored.absorb_month_folder(&path);
        } else if is_transcript(&path) {
            stored.loose_files.push(path);
        }
    }
    Ok(stored)
}

fn session_folder_id(path: &Path) -> Option<SessionId> {
    let id = SessionId::parse(&file_name(path)).ok()?;
    path.join(transcript_file_name(&id)).is_file().then_some(id)
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
