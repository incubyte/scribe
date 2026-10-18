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

    pub struct SpeechEngine;

    pub fn build_lanes(
        listener: &Arc<dyn RecordingListener>,
        _engine: &SpeechEngine,
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

    pub fn ensure_speech_model(on_installing: &dyn Fn()) -> Result<SpeechEngine, TranscribeError> {
        match stt_speechanalyzer::english_asset_status()? {
            AssetStatus::Installed => Ok(SpeechEngine),
            AssetStatus::Missing => {
                on_installing();
                stt_speechanalyzer::install_english_asset().map(|()| SpeechEngine)
            }
            AssetStatus::Unsupported => Err(TranscribeError::EngineUnavailable(
                "this Mac cannot transcribe English on-device".to_owned(),
            )),
        }
    }
}

#[cfg(target_os = "macos")]
pub use macos::{ENGINE, LANGUAGE, SpeechEngine, build_lanes, ensure_speech_model};

#[cfg(windows)]
mod windows {
    use std::path::PathBuf;
    use std::sync::Arc;

    use audio_mic::MicSource;
    use audio_windows::LoopbackSource;
    use scribe_core::ports::{RecordingListener, TranscribeError};
    use scribe_core::recording::{Lane, Speaker};
    use stt_parakeet::{ParakeetModel, ParakeetTranscriber, default_models_dir};

    pub const ENGINE: &str = stt_parakeet::ENGINE_NAME;
    pub const LANGUAGE: &str = stt_parakeet::AUTO_LANGUAGE;

    pub struct SpeechEngine {
        model: ParakeetModel,
        models_dir: PathBuf,
    }

    impl SpeechEngine {
        fn transcriber(&self) -> Result<ParakeetTranscriber, TranscribeError> {
            ParakeetTranscriber::sharing(self.model.clone(), &self.models_dir)
        }
    }

    pub fn load_speech_engine() -> Result<SpeechEngine, TranscribeError> {
        let models_dir = default_models_dir()
            .map_err(|error| TranscribeError::EngineUnavailable(error.to_string()))?;
        Ok(SpeechEngine {
            model: ParakeetModel::load(&models_dir)?,
            models_dir,
        })
    }

    pub fn build_lanes(
        listener: &Arc<dyn RecordingListener>,
        engine: &SpeechEngine,
    ) -> Result<Vec<Lane>, TranscribeError> {
        let mic = Lane {
            speaker: Speaker::Me,
            source: Box::new(MicSource::new()),
            transcriber: Box::new(engine.transcriber()?),
        };
        let loopback = Lane {
            speaker: Speaker::Them,
            source: Box::new(LoopbackSource::new(Arc::clone(listener))),
            transcriber: Box::new(engine.transcriber()?),
        };
        Ok(vec![mic, loopback])
    }
}

#[cfg(windows)]
pub use windows::{ENGINE, LANGUAGE, SpeechEngine, build_lanes, load_speech_engine};

#[cfg(not(any(target_os = "macos", windows)))]
mod unsupported {
    use std::sync::Arc;

    use scribe_core::ports::{RecordingListener, TranscribeError};
    use scribe_core::recording::Lane;

    pub const ENGINE: &str = "none";
    pub const LANGUAGE: &str = "none";

    pub struct SpeechEngine;

    pub fn build_lanes(
        _listener: &Arc<dyn RecordingListener>,
        _engine: &SpeechEngine,
    ) -> Result<Vec<Lane>, TranscribeError> {
        Err(TranscribeError::EngineUnavailable(
            "no speech engine is available on this system yet".to_owned(),
        ))
    }
}

#[cfg(not(any(target_os = "macos", windows)))]
pub use unsupported::{ENGINE, LANGUAGE, SpeechEngine, build_lanes};
