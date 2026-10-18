use std::path::Path;

use ort::session::Session;
use ort::value::Tensor;
use scribe_core::ports::TranscribeError;
use scribe_core::recording::SAMPLE_RATE_HZ;

use crate::utterance_cutter::VAD_WINDOW_SAMPLES;

const CONTEXT_SAMPLES: usize = 64;
const STATE_SHAPE: [usize; 3] = [2, 1, 128];
const STATE_LEN: usize = STATE_SHAPE[0] * STATE_SHAPE[1] * STATE_SHAPE[2];
const INPUT_SAMPLES: usize = CONTEXT_SAMPLES + VAD_WINDOW_SAMPLES;

pub struct SileroVad {
    session: Session,
    state: Vec<f32>,
    context: Vec<f32>,
}

impl SileroVad {
    pub fn load(path: &Path) -> Result<SileroVad, TranscribeError> {
        let session = Session::builder()
            .and_then(|mut builder| builder.commit_from_file(path))
            .map_err(|error| TranscribeError::EngineUnavailable(error.to_string()))?;
        Ok(SileroVad {
            session,
            state: vec![0.0; STATE_LEN],
            context: vec![0.0; CONTEXT_SAMPLES],
        })
    }

    pub fn speech_probability(
        &mut self,
        window: &[f32; VAD_WINDOW_SAMPLES],
    ) -> Result<f32, TranscribeError> {
        let mut input = Vec::with_capacity(INPUT_SAMPLES);
        input.extend_from_slice(&self.context);
        input.extend_from_slice(window);
        self.context
            .copy_from_slice(&window[VAD_WINDOW_SAMPLES - CONTEXT_SAMPLES..]);
        self.run(input)
            .map_err(|error| TranscribeError::Engine(error.to_string()))
    }

    fn run(&mut self, input: Vec<f32>) -> ort::Result<f32> {
        let outputs = self.session.run(ort::inputs![
            "input" => Tensor::from_array(([1, INPUT_SAMPLES], input))?,
            "state" => Tensor::from_array((STATE_SHAPE, self.state.clone()))?,
            "sr" => Tensor::from_array(((), vec![i64::from(SAMPLE_RATE_HZ)]))?,
        ])?;
        let (_, next_state) = outputs["stateN"].try_extract_tensor::<f32>()?;
        self.state.copy_from_slice(next_state);
        let (_, probability) = outputs["output"].try_extract_tensor::<f32>()?;
        Ok(probability[0])
    }
}
