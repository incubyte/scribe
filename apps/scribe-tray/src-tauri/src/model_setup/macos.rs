use scribe_core::ports::TranscribeError;

use super::SetupFailed;
use crate::lanes::{self, SpeechEngine};

pub fn prepare_speech_engine(progress: &dyn Fn(Option<u8>)) -> Result<SpeechEngine, SetupFailed> {
    lanes::ensure_speech_model(&|| progress(None)).map_err(|error| SetupFailed {
        reason_kind: failure_kind(&error),
    })
}

fn failure_kind(error: &TranscribeError) -> &'static str {
    match error {
        TranscribeError::EngineUnavailable(_) => "unsupported",
        TranscribeError::Engine(_) => "install",
    }
}
