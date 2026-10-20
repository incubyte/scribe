use std::path::PathBuf;
use std::sync::mpsc::Receiver;
use std::time::Duration;

use chrono::{DateTime, FixedOffset};

use crate::capture::{
    Frame, MonitorShot, ScreenshotName, ScreenshotReference, ServedImage, SessionMoment,
};
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

    fn final_results_wait(&self) -> Option<Duration> {
        None
    }
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

pub trait ScreenCapturer: Send + Sync {
    fn access(&self) -> CaptureAccess;
    fn request_access(&self);
    fn capture_under_pointer(&self) -> Result<MonitorShot, ScreenCaptureError>;
}

pub trait ScreenshotStore: Send + Sync {
    fn save(
        &self,
        moment: &SessionMoment,
        screenshot: &Frame,
    ) -> Result<ScreenshotName, StoreError>;
    fn list_screenshots(&self, session: &SessionId) -> Result<Vec<ScreenshotName>, StoreError>;
    fn serve(
        &self,
        reference: &ScreenshotReference,
        max_long_edge: u32,
    ) -> Result<ServedImage, StoreError>;
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CaptureAccess {
    Granted,
    NotGranted,
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ScreenCaptureError {
    #[error("screen recording access has not been granted")]
    AccessNotGranted,
    #[error("no monitor is under the pointer")]
    NoMonitorUnderPointer,
    #[error("the screen could not be captured: {0}")]
    Failed(String),
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
    #[error("every screenshot name for this moment is already taken")]
    NoFreeScreenshotName,
    #[error("the screenshot could not be encoded as a PNG")]
    ScreenshotNotEncoded,
    #[error("no screenshot named {reference}")]
    ScreenshotNotFound { reference: ScreenshotReference },
}
