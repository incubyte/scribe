use std::path::Path;

use parakeet_rs::{ParakeetTDT, TimestampMode, Transcriber as _};
use scribe_core::ports::{TranscribeError, Transcriber};
use scribe_core::recording::{AudioChunk, SAMPLE_RATE_HZ, TranscriptEvent, Utterance};

use crate::utterance_cutter::{
    CutterConfig, PendingUtterance, UtteranceCutter, VAD_WINDOW_SAMPLES,
};
use crate::vad::SileroVad;
use crate::vad_windows::VadWindows;

pub const ENGINE_NAME: &str = "parakeet";
pub const AUTO_LANGUAGE: &str = "auto";

const SILERO_FILE: &str = "silero_vad.onnx";
const MONO: u16 = 1;

pub struct ParakeetTranscriber {
    model: ParakeetTDT,
    vad: SileroVad,
    speech_threshold: f32,
    windows: VadWindows,
    cutter: UtteranceCutter,
}

impl ParakeetTranscriber {
    pub fn load(models_dir: &Path) -> Result<ParakeetTranscriber, TranscribeError> {
        let config = CutterConfig::default();
        let model = ParakeetTDT::from_pretrained(models_dir, None)
            .map_err(|error| TranscribeError::EngineUnavailable(error.to_string()))?;
        Ok(ParakeetTranscriber {
            model,
            vad: SileroVad::load(&models_dir.join(SILERO_FILE))?,
            speech_threshold: config.speech_threshold,
            windows: VadWindows::default(),
            cutter: UtteranceCutter::new(config),
        })
    }

    fn mark_speech(&mut self, windows: &AudioChunk) -> Result<Vec<bool>, TranscribeError> {
        windows
            .samples
            .as_chunks::<VAD_WINDOW_SAMPLES>()
            .0
            .iter()
            .map(|window| {
                let probability = self.vad.speech_probability(window)?;
                Ok(probability >= self.speech_threshold)
            })
            .collect()
    }

    fn decode_all(
        &mut self,
        utterances: Vec<PendingUtterance>,
    ) -> Result<Vec<TranscriptEvent>, TranscribeError> {
        let mut events = Vec::new();
        for utterance in utterances {
            events.extend(self.decode(utterance)?);
        }
        Ok(events)
    }

    fn decode(
        &mut self,
        utterance: PendingUtterance,
    ) -> Result<Option<TranscriptEvent>, TranscribeError> {
        let start = utterance.start;
        let end = utterance.end();
        let result = self
            .model
            .transcribe_samples(
                utterance.samples,
                SAMPLE_RATE_HZ,
                MONO,
                Some(TimestampMode::Sentences),
            )
            .map_err(|error| TranscribeError::Engine(error.to_string()))?;
        let text = result.text.trim();
        Ok((!text.is_empty()).then(|| {
            TranscriptEvent::Final(Utterance {
                start,
                end,
                text: text.to_owned(),
            })
        }))
    }
}

impl Transcriber for ParakeetTranscriber {
    fn push(&mut self, chunk: &AudioChunk) -> Result<Vec<TranscriptEvent>, TranscribeError> {
        let windows = self.windows.take_whole_windows(chunk);
        let speech = self.mark_speech(&windows)?;
        let utterances = self.cutter.feed(&windows, &speech);
        self.decode_all(utterances)
    }

    fn finish(&mut self) -> Result<Vec<TranscriptEvent>, TranscribeError> {
        let utterances = self.cutter.flush().into_iter().collect();
        self.decode_all(utterances)
    }
}
