use std::fs;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};
use std::sync::{PoisonError, RwLock};

use chrono::{DateTime, FixedOffset};
use scribe_core::calendar::MeetingLink;
use scribe_core::ports::{StoreError, TranscriptStore};
use scribe_core::recording::{
    Segment, SessionDocument, SessionHeader, SessionId, SessionStatus, SessionSummary,
};

use crate::format::{parse_header, render_header, render_segment, split_front_matter};
use crate::month_layout;
use crate::private_file;
use crate::reader;
use crate::store_error::{file_name, io_error, malformed};
use crate::transcripts_folder::TranscriptsFolder;

pub struct MarkdownTranscriptStore {
    folder: RwLock<TranscriptsFolder>,
}

impl MarkdownTranscriptStore {
    #[must_use]
    pub fn new(folder: TranscriptsFolder) -> MarkdownTranscriptStore {
        MarkdownTranscriptStore {
            folder: RwLock::new(folder),
        }
    }

    pub fn relocate(&self, folder: TranscriptsFolder) {
        *self.folder.write().unwrap_or_else(PoisonError::into_inner) = folder;
    }

    #[must_use]
    pub fn folder(&self) -> TranscriptsFolder {
        self.folder
            .read()
            .unwrap_or_else(PoisonError::into_inner)
            .clone()
    }

    #[must_use]
    pub fn path_for(&self, id: &SessionId) -> PathBuf {
        month_layout::existing_path(self.folder().path(), id)
    }

    pub fn link_meeting(&self, id: &SessionId, link: MeetingLink) -> Result<(), StoreError> {
        rewrite_header(&self.path_for(id), |header| header.with_meeting(link))
    }

    fn try_create(
        folder: &TranscriptsFolder,
        header: &SessionHeader,
        id: SessionId,
    ) -> Result<Option<SessionId>, StoreError> {
        if month_layout::has_older_layout_file(folder.path(), &id) {
            return Ok(None);
        }
        if folder.ensure_session_folder_exists(&id)?.is_none() {
            return Ok(None);
        }
        let path = month_layout::session_path(folder.path(), &id);
        let header = header.clone().with_id(id.clone());
        match private_file::create_new(&path, &render_header(&header)) {
            Ok(()) => Ok(Some(id)),
            Err(error) if error.kind() == ErrorKind::AlreadyExists => Ok(None),
            Err(source) => Err(StoreError::Io { path, source }),
        }
    }

    fn transcript_paths(&self) -> Result<Vec<PathBuf>, StoreError> {
        Ok(month_layout::stored_sessions_in(self.folder().path())?.transcript_paths())
    }
}

impl TranscriptStore for MarkdownTranscriptStore {
    fn create(&self, header: &SessionHeader) -> Result<SessionId, StoreError> {
        let folder = self.folder();
        folder.ensure_exists()?;
        folder.ensure_month_exists(&header.id)?;
        for candidate in header.id.collision_candidates() {
            if let Some(id) = Self::try_create(&folder, header, candidate)? {
                return Ok(id);
            }
        }
        Err(StoreError::NoFreeSessionId)
    }

    fn append(&self, id: &SessionId, segment: &Segment) -> Result<(), StoreError> {
        let path = self.path_for(id);
        private_file::append(&path, &render_segment(segment)).map_err(|source| {
            match source.kind() {
                ErrorKind::NotFound => StoreError::NotFound { id: id.clone() },
                _ => io_error(&path, source),
            }
        })
    }

    fn finalise(&self, id: &SessionId, ended_at: DateTime<FixedOffset>) -> Result<(), StoreError> {
        rewrite_header(&self.path_for(id), |header| header.finalised(ended_at))
    }

    fn list(&self, limit: usize) -> Result<Vec<SessionSummary>, StoreError> {
        let paths = self.transcript_paths()?;
        Ok(reader::list_summaries(
            paths.iter().map(PathBuf::as_path),
            limit,
        ))
    }

    fn list_all(&self) -> Result<Vec<SessionSummary>, StoreError> {
        let paths = self.transcript_paths()?;
        Ok(reader::list_all_summaries(
            paths.iter().map(PathBuf::as_path),
        ))
    }

    fn read(&self, id: &SessionId) -> Result<SessionDocument, StoreError> {
        reader::read_document(&self.path_for(id), id)
    }

    fn mark_abandoned_incomplete(&self) -> Result<usize, StoreError> {
        let marked = self
            .transcript_paths()?
            .iter()
            .filter(|path| mark_if_abandoned(path))
            .count();
        Ok(marked)
    }
}

fn mark_if_abandoned(path: &Path) -> bool {
    abandon_if_recording(path).unwrap_or_else(|error| {
        tracing::warn!(file = %file_name(path), %error, "skipped a transcript file on launch");
        false
    })
}

fn abandon_if_recording(path: &Path) -> Result<bool, StoreError> {
    let (raw, header) = read_transcript(path)?;
    if header.status != SessionStatus::Recording {
        return Ok(false);
    }
    replace_header(path, &raw, &header.abandoned())?;
    Ok(true)
}

fn rewrite_header(
    path: &Path,
    change: impl FnOnce(SessionHeader) -> SessionHeader,
) -> Result<(), StoreError> {
    let (raw, header) = read_transcript(path)?;
    replace_header(path, &raw, &change(header))
}

fn read_transcript(path: &Path) -> Result<(String, SessionHeader), StoreError> {
    let raw = fs::read_to_string(path).map_err(|source| io_error(path, source))?;
    let header = parse_header(&raw).map_err(|_| malformed(path))?;
    Ok((raw, header))
}

fn replace_header(path: &Path, raw: &str, header: &SessionHeader) -> Result<(), StoreError> {
    let (_, body) = split_front_matter(raw).map_err(|_| malformed(path))?;
    let rewritten = render_header(header) + body;
    private_file::replace_atomically(path, &rewritten).map_err(|source| io_error(path, source))
}
