use std::time::Duration;

pub const SAMPLE_RATE_HZ: u32 = 16_000;

#[derive(Debug, Clone, PartialEq)]
pub struct AudioChunk {
    pub samples: Vec<f32>,
    pub captured_at: Duration,
}

impl AudioChunk {
    #[must_use]
    pub fn ending_at(samples: Vec<f32>, end: Duration) -> AudioChunk {
        let mut chunk = AudioChunk {
            samples,
            captured_at: Duration::ZERO,
        };
        chunk.captured_at = end.saturating_sub(chunk.duration());
        chunk
    }

    #[must_use]
    pub fn duration(&self) -> Duration {
        let samples = u64::try_from(self.samples.len()).unwrap_or(u64::MAX);
        Duration::from_micros(samples.saturating_mul(1_000_000) / u64::from(SAMPLE_RATE_HZ))
    }

    #[must_use]
    pub fn ends_at(&self) -> Duration {
        self.captured_at + self.duration()
    }

    #[must_use]
    pub fn is_exact_silence(&self) -> bool {
        self.samples.iter().all(|sample| *sample == 0.0)
    }
}
