use scribe_core::ports::{CaptureError, TranscribeError};
use scribe_core::recording::RecorderError;

#[derive(Debug, thiserror::Error)]
pub enum StartError {
    #[error(transparent)]
    Recorder(#[from] RecorderError),
    #[error(transparent)]
    Engine(#[from] TranscribeError),
}

impl From<CaptureError> for StartError {
    fn from(error: CaptureError) -> StartError {
        StartError::Recorder(RecorderError::Capture(error))
    }
}

impl StartError {
    #[must_use]
    pub fn kind(&self) -> StartFailureKind {
        match self {
            StartError::Recorder(error) => recorder_failure(error),
            StartError::Engine(_) => StartFailureKind::SpeechEngine,
        }
    }
}

fn recorder_failure(error: &RecorderError) -> StartFailureKind {
    match error {
        RecorderError::NotReady => StartFailureKind::NotReady,
        RecorderError::AlreadyRecording => StartFailureKind::AlreadyRecording,
        RecorderError::Capture(capture) => capture_failure(capture),
        RecorderError::Store(_) => StartFailureKind::TranscriptsFolder,
        RecorderError::WorkerPanicked => StartFailureKind::Internal,
    }
}

fn capture_failure(error: &CaptureError) -> StartFailureKind {
    match error {
        CaptureError::PermissionDenied => StartFailureKind::MicrophoneDenied,
        CaptureError::DeviceUnavailable => StartFailureKind::NoMicrophone,
        CaptureError::StreamFailed(_) => StartFailureKind::AudioStream,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StartFailureKind {
    NotReady,
    AlreadyRecording,
    MicrophoneDenied,
    NoMicrophone,
    AudioStream,
    TranscriptsFolder,
    SpeechEngine,
    Internal,
}

impl StartFailureKind {
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            StartFailureKind::NotReady => "not_ready",
            StartFailureKind::AlreadyRecording => "already_recording",
            StartFailureKind::MicrophoneDenied => "microphone_denied",
            StartFailureKind::NoMicrophone => "no_microphone",
            StartFailureKind::AudioStream => "audio_stream",
            StartFailureKind::TranscriptsFolder => "transcripts_folder",
            StartFailureKind::SpeechEngine => "speech_engine",
            StartFailureKind::Internal => "internal",
        }
    }

    #[must_use]
    pub fn advice(self) -> &'static str {
        match self {
            StartFailureKind::NotReady => "Scribe is still getting ready. Try again in a moment.",
            StartFailureKind::AlreadyRecording => "A recording is already in progress.",
            StartFailureKind::MicrophoneDenied => {
                "Allow Scribe to use the microphone, then start recording again."
            }
            StartFailureKind::NoMicrophone => "No microphone was found. Connect one and try again.",
            StartFailureKind::AudioStream => {
                "The audio device could not be opened. Try again, or pick another device."
            }
            StartFailureKind::TranscriptsFolder => {
                "The transcript could not be created in Documents/Scribe. Check the folder and try again."
            }
            StartFailureKind::SpeechEngine => {
                "The on-device speech engine could not start. Try again in a moment."
            }
            StartFailureKind::Internal => "Something went wrong while starting. Try again.",
        }
    }
}
