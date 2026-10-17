use std::sync::Arc;
use std::sync::mpsc::Sender;

use audio_resample::MonoResampler;
use scribe_core::ports::{CaptureError, RecordingListener};
use scribe_core::recording::{AudioChunk, SessionClock};

use crate::tap::{TapFormat, TapHandle, create_tap};

const DRAINS_BETWEEN_FORMAT_CHECKS: u32 = 128;

pub(crate) struct Delivery {
    pub(crate) clock: SessionClock,
    pub(crate) chunks: Sender<AudioChunk>,
    pub(crate) listener: Arc<dyn RecordingListener>,
}

pub(crate) struct Pump {
    tap: TapHandle,
    resampler: MonoResampler,
    format: TapFormat,
    drains: u32,
}

impl Pump {
    pub(crate) fn open() -> Result<Pump, CaptureError> {
        let tap = create_tap()?;
        let format = tap.format();
        Ok(Pump {
            resampler: resampler_for(format)?,
            tap,
            format,
            drains: 0,
        })
    }

    pub(crate) fn forward_captured_audio(&mut self, delivery: &Delivery) -> bool {
        self.follow_format_changes();
        let captured = self.tap.take_samples();
        let samples = match self.resampler.process(&captured, self.format.channels) {
            Ok(samples) => samples,
            Err(error) => {
                tracing::warn!(%error, "dropped system audio that could not be resampled");
                return true;
            }
        };
        samples.is_empty()
            || delivery
                .chunks
                .send(AudioChunk::ending_at(samples, delivery.clock.elapsed()))
                .is_ok()
    }

    fn follow_format_changes(&mut self) {
        self.drains = self.drains.wrapping_add(1);
        if !self.drains.is_multiple_of(DRAINS_BETWEEN_FORMAT_CHECKS) {
            return;
        }
        match self.tap.current_format() {
            Ok(format) if format != self.format => self.switch_format(format),
            Ok(_) => {}
            Err(error) => tracing::warn!(%error, "could not re-read the system audio format"),
        }
    }

    fn switch_format(&mut self, format: TapFormat) {
        match resampler_for(format) {
            Ok(resampler) => {
                self.resampler = resampler;
                self.format = format;
            }
            Err(error) => tracing::warn!(%error, "could not follow the new system audio format"),
        }
    }
}

fn resampler_for(format: TapFormat) -> Result<MonoResampler, CaptureError> {
    MonoResampler::new(format.sample_rate_hz)
        .map_err(|error| CaptureError::StreamFailed(error.to_string()))
}
