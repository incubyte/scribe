#[cfg(target_os = "macos")]
mod macos {
    use std::sync::Arc;

    use audio_macos::SystemOutputSource;
    use audio_mic::MicSource;
    use scribe_core::ports::{RecordingListener, TranscribeError};
    use scribe_core::recording::{Lane, Speaker};
    use stt_speechanalyzer::{AssetStatus, SpeechAnalyzerTranscriber};

    pub const ENGINE: &str = stt_speechanalyzer::ENGINE_NAME;
    pub const LANGUAGE: &str = stt_speechanalyzer::ENGLISH;

    pub fn build_lanes(
        listener: &Arc<dyn RecordingListener>,
    ) -> Result<Vec<Lane>, TranscribeError> {
        let mic = Lane {
            speaker: Speaker::Me,
            source: Box::new(MicSource::new()),
            transcriber: Box::new(SpeechAnalyzerTranscriber::start(LANGUAGE)?),
        };
        let system_output = Lane {
            speaker: Speaker::Them,
            source: Box::new(SystemOutputSource::new(Arc::clone(listener))),
            transcriber: Box::new(SpeechAnalyzerTranscriber::start(LANGUAGE)?),
        };
        Ok(vec![mic, system_output])
    }

    pub fn ensure_speech_model(on_installing: &dyn Fn()) -> Result<(), TranscribeError> {
        match stt_speechanalyzer::english_asset_status()? {
            AssetStatus::Installed => Ok(()),
            AssetStatus::Missing => {
                on_installing();
                stt_speechanalyzer::install_english_asset()
            }
            AssetStatus::Unsupported => Err(TranscribeError::EngineUnavailable(
                "this Mac cannot transcribe English on-device".to_owned(),
            )),
        }
    }
}

#[cfg(target_os = "macos")]
pub use macos::{ENGINE, LANGUAGE, build_lanes, ensure_speech_model};

#[cfg(not(target_os = "macos"))]
mod unsupported {
    use std::sync::Arc;

    use scribe_core::ports::{RecordingListener, TranscribeError};
    use scribe_core::recording::Lane;

    pub const ENGINE: &str = "none";
    pub const LANGUAGE: &str = "none";

    pub fn build_lanes(
        _listener: &Arc<dyn RecordingListener>,
    ) -> Result<Vec<Lane>, TranscribeError> {
        Err(no_engine())
    }

    fn no_engine() -> TranscribeError {
        TranscribeError::EngineUnavailable(
            "no speech engine is available on this system yet".to_owned(),
        )
    }
}

#[cfg(not(target_os = "macos"))]
pub use unsupported::{ENGINE, LANGUAGE, build_lanes};
