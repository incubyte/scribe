use std::collections::VecDeque;
use std::time::Duration;

use scribe_core::recording::{AudioChunk, SAMPLE_RATE_HZ};

pub const VAD_WINDOW_SAMPLES: usize = 512;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CutterConfig {
    pub speech_threshold: f32,
    pub end_silence: Duration,
    pub max_utterance: Duration,
    pub pre_roll: Duration,
}

impl Default for CutterConfig {
    fn default() -> CutterConfig {
        CutterConfig {
            speech_threshold: 0.5,
            end_silence: Duration::from_millis(600),
            max_utterance: Duration::from_secs(15),
            pre_roll: Duration::from_millis(200),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct PendingUtterance {
    pub start: Duration,
    pub samples: Vec<f32>,
}

impl PendingUtterance {
    #[must_use]
    pub fn end(&self) -> Duration {
        self.start + duration_of(self.samples.len())
    }
}

struct Limits {
    end_silence: usize,
    max_utterance: usize,
    pre_roll: usize,
}

struct HeardWindow<'a> {
    samples: &'a [f32],
    is_speech: bool,
    start: Duration,
}

struct OpenUtterance {
    start: Duration,
    samples: Vec<f32>,
    trailing_silence: usize,
}

pub struct UtteranceCutter {
    limits: Limits,
    origin: Option<Duration>,
    pre_roll: VecDeque<f32>,
    open: Option<OpenUtterance>,
}

impl UtteranceCutter {
    #[must_use]
    pub fn new(config: CutterConfig) -> UtteranceCutter {
        UtteranceCutter {
            limits: Limits {
                end_silence: samples_in(config.end_silence),
                max_utterance: samples_in(config.max_utterance),
                pre_roll: samples_in(config.pre_roll),
            },
            origin: None,
            pre_roll: VecDeque::new(),
            open: None,
        }
    }

    pub fn feed(&mut self, chunk: &AudioChunk, speech: &[bool]) -> Vec<PendingUtterance> {
        let origin = *self.origin.get_or_insert(chunk.captured_at);
        let chunk_start = chunk.captured_at.saturating_sub(origin);
        chunk
            .samples
            .chunks(VAD_WINDOW_SAMPLES)
            .zip(speech)
            .enumerate()
            .filter_map(|(index, (samples, is_speech))| {
                self.hear_window(&HeardWindow {
                    samples,
                    is_speech: *is_speech,
                    start: chunk_start + duration_of(index * VAD_WINDOW_SAMPLES),
                })
            })
            .collect()
    }

    pub fn flush(&mut self) -> Option<PendingUtterance> {
        self.pre_roll.clear();
        self.open.take().map(|open| self.closed(open))
    }

    fn hear_window(&mut self, window: &HeardWindow) -> Option<PendingUtterance> {
        let cut_at_limit = self.take_if_window_would_exceed_limit(window.samples.len());
        match self.open.as_mut() {
            Some(open) => open.extend(window),
            None if window.is_speech => self.open = Some(self.open_from_pre_roll(window)),
            None => self.remember_pre_roll(window.samples),
        }
        cut_at_limit.or_else(|| self.take_if_pause_ended_it())
    }

    fn take_if_window_would_exceed_limit(&mut self, window_len: usize) -> Option<PendingUtterance> {
        let would_exceed = self
            .open
            .as_ref()
            .is_some_and(|open| open.samples.len() + window_len > self.limits.max_utterance);
        would_exceed.then(|| self.flush()).flatten()
    }

    fn take_if_pause_ended_it(&mut self) -> Option<PendingUtterance> {
        let pause_ended_it = self
            .open
            .as_ref()
            .is_some_and(|open| open.trailing_silence >= self.limits.end_silence);
        pause_ended_it.then(|| self.flush()).flatten()
    }

    fn open_from_pre_roll(&mut self, window: &HeardWindow) -> OpenUtterance {
        let lead_in: Vec<f32> = self.pre_roll.drain(..).collect();
        let start = window.start.saturating_sub(duration_of(lead_in.len()));
        let mut samples = lead_in;
        samples.extend_from_slice(window.samples);
        OpenUtterance {
            start,
            samples,
            trailing_silence: 0,
        }
    }

    fn remember_pre_roll(&mut self, window: &[f32]) {
        self.pre_roll.extend(window);
        let excess = self.pre_roll.len().saturating_sub(self.limits.pre_roll);
        self.pre_roll.drain(..excess);
    }

    fn closed(&self, mut open: OpenUtterance) -> PendingUtterance {
        let trimmed_silence = open.trailing_silence.saturating_sub(self.limits.pre_roll);
        open.samples
            .truncate(open.samples.len().saturating_sub(trimmed_silence));
        PendingUtterance {
            start: open.start,
            samples: open.samples,
        }
    }
}

impl OpenUtterance {
    fn extend(&mut self, window: &HeardWindow) {
        self.samples.extend_from_slice(window.samples);
        self.trailing_silence = if window.is_speech {
            0
        } else {
            self.trailing_silence + window.samples.len()
        };
    }
}

fn samples_in(duration: Duration) -> usize {
    let samples = duration.as_micros() * u128::from(SAMPLE_RATE_HZ) / 1_000_000;
    usize::try_from(samples).unwrap_or(usize::MAX)
}

fn duration_of(samples: usize) -> Duration {
    let samples = u64::try_from(samples).unwrap_or(u64::MAX);
    Duration::from_micros(samples.saturating_mul(1_000_000) / u64::from(SAMPLE_RATE_HZ))
}
