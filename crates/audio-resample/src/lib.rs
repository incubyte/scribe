use rubato::audioadapter_buffers::SizeError;
use rubato::audioadapter_buffers::direct::InterleavedSlice;
use rubato::{Fft, FixedSync, Resampler};

const TARGET_RATE_HZ: usize = 16_000;
const INPUT_CHUNK_FRAMES: usize = 1_024;
const MONO: usize = 1;

#[derive(Debug, thiserror::Error)]
pub enum ResampleError {
    #[error("cannot resample audio from {input_rate_hz} Hz")]
    UnsupportedRate {
        input_rate_hz: u32,
        #[source]
        source: rubato::ResamplerConstructionError,
    },
    #[error("resampling failed")]
    Processing(#[from] rubato::ResampleError),
    #[error("the audio block does not match the resampler's chunk size")]
    BlockSize(#[from] SizeError),
}

pub struct MonoResampler {
    resampler: Fft<f32>,
    pending: Vec<f32>,
}

impl MonoResampler {
    pub fn new(input_rate_hz: u32) -> Result<MonoResampler, ResampleError> {
        let resampler = Fft::new(
            input_rate_hz as usize,
            TARGET_RATE_HZ,
            INPUT_CHUNK_FRAMES,
            MONO,
            FixedSync::Input,
        )
        .map_err(|source| ResampleError::UnsupportedRate {
            input_rate_hz,
            source,
        })?;
        Ok(MonoResampler {
            resampler,
            pending: Vec::with_capacity(INPUT_CHUNK_FRAMES * 2),
        })
    }

    pub fn process(
        &mut self,
        interleaved: &[f32],
        channels: u16,
    ) -> Result<Vec<f32>, ResampleError> {
        self.pending.extend(downmix(interleaved, channels));
        let mut output = Vec::new();
        while self.pending.len() >= self.resampler.input_frames_next() {
            output.extend(self.resample_next_chunk()?);
        }
        Ok(output)
    }

    fn resample_next_chunk(&mut self) -> Result<Vec<f32>, ResampleError> {
        let frames = self.resampler.input_frames_next();
        let input = InterleavedSlice::new(&self.pending[..frames], MONO, frames)?;
        let resampled = self.resampler.process(&input, None)?;
        self.pending.drain(..frames);
        Ok(resampled.take_data())
    }
}

fn downmix(interleaved: &[f32], channels: u16) -> impl Iterator<Item = f32> + '_ {
    let channels = channels.max(1);
    let scale = 1.0 / f32::from(channels);
    interleaved
        .chunks_exact(usize::from(channels))
        .map(move |frame| frame.iter().sum::<f32>() * scale)
}
