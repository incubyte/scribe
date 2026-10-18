use std::time::Duration;

use scribe_core::recording::AudioChunk;

use crate::utterance_cutter::VAD_WINDOW_SAMPLES;

#[derive(Default)]
pub(crate) struct VadWindows {
    leftover: Vec<f32>,
    leftover_starts_at: Duration,
}

impl VadWindows {
    pub(crate) fn take_whole_windows(&mut self, chunk: &AudioChunk) -> AudioChunk {
        if self.leftover.is_empty() {
            self.leftover_starts_at = chunk.captured_at;
        }
        self.leftover.extend_from_slice(&chunk.samples);
        let whole_len = self.leftover.len() - self.leftover.len() % VAD_WINDOW_SAMPLES;
        let remainder = self.leftover.split_off(whole_len);
        let windows = AudioChunk {
            samples: std::mem::replace(&mut self.leftover, remainder),
            captured_at: self.leftover_starts_at,
        };
        self.leftover_starts_at = windows.ends_at();
        windows
    }
}
