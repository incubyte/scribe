use super::SetupFailed;
use crate::lanes::SpeechEngine;

pub fn prepare_speech_engine(_progress: &dyn Fn(Option<u8>)) -> Result<SpeechEngine, SetupFailed> {
    Ok(SpeechEngine)
}
