use std::path::PathBuf;
use std::sync::mpsc::Receiver;

use chrono::{DateTime, FixedOffset};

use crate::recording::{
    AudioChunk, RecordingEvent, Segment, SessionClock, SessionDocument, SessionHeader, SessionId,
    SessionSummary, TranscriptEvent, Transcription,
};

pub trait AudioSource: Send {
    fn start(&mut self, clock: SessionClock) -> Result<Receiver<AudioChunk>, CaptureError>;
    fn stop(&mut self);
}

pub trait Transcriber: Send {
    fn push(&mut self, chunk: &AudioChunk) -> Result<Transcription, TranscribeError>;
    fn finish(&mut self) -> Result<Vec<TranscriptEvent>, TranscribeError>;
}

pub trait TranscriptStore: Send + Sync {
    fn create(&self, header: &SessionHeader) -> Result<SessionId, StoreError>;
    fn append(&self, id: &SessionId, segment: &Segment) -> Result<(), StoreError>;
    fn finalise(&self, id: &SessionId, ended_at: DateTime<FixedOffset>) -> Result<(), StoreError>;
    fn mark_abandoned_incomplete(&self) -> Result<usize, StoreError>;
    fn list(&self, limit: usize) -> Result<Vec<SessionSummary>, StoreError>;
    fn list_all(&self) -> Result<Vec<SessionSummary>, StoreError>;
    fn read(&self, id: &SessionId) -> Result<SessionDocument, StoreError>;
}

pub trait RecordingListener: Send + Sync {
    fn on_event(&self, event: RecordingEvent);
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum CaptureError {
    #[error("permission to capture audio was denied")]
    PermissionDenied,
    #[error("no audio capture device is available")]
    DeviceUnavailable,
    #[error("the audio stream could not be started: {0}")]
    StreamFailed(String),
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum TranscribeError {
    #[error("the speech engine is unavailable: {0}")]
    EngineUnavailable(String),
    #[error("the speech engine failed: {0}")]
    Engine(String),
}

#[derive(Debug, thiserror::Error)]
pub enum StoreError {
    #[error("the user's documents folder could not be located")]
    DocumentsFolderUnavailable,
    #[error("could not create the transcripts folder at {path}")]
    CreateFolder {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("could not access the transcript file {path}")]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("no transcript exists for session {id}")]
    NotFound { id: SessionId },
    #[error("the transcript file {file_name} is not in the expected format")]
    Malformed { file_name: String },
    #[error("every session id for this start time is already taken")]
    NoFreeSessionId,
}
