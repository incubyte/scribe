use std::sync::{Mutex, MutexGuard, PoisonError};

use chrono::{DateTime, FixedOffset, SecondsFormat};
use scribe_core::recording::{RecorderState, Segment};
use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct LiveLine {
    pub speaker: &'static str,
    pub start_ms: u128,
    pub text: String,
}

impl From<&Segment> for LiveLine {
    fn from(segment: &Segment) -> LiveLine {
        LiveLine {
            speaker: segment.speaker.label(),
            start_ms: segment.start.as_millis(),
            text: segment.text.clone(),
        }
    }
}

#[derive(Debug, Serialize)]
pub struct RecorderSnapshot {
    state: &'static str,
    started_at: Option<String>,
    lines: Vec<LiveLine>,
}

#[derive(Debug, Default)]
struct Transcript {
    started_at: Option<DateTime<FixedOffset>>,
    lines: Vec<LiveLine>,
}

#[derive(Debug, Default)]
pub struct LiveFeed(Mutex<Transcript>);

impl LiveFeed {
    pub fn started(&self, at: DateTime<FixedOffset>) {
        *self.transcript() = Transcript {
            started_at: Some(at),
            lines: Vec::new(),
        };
    }

    pub fn finalised(&self, segment: &Segment) {
        self.transcript().lines.push(LiveLine::from(segment));
    }

    pub fn stopped(&self) {
        *self.transcript() = Transcript::default();
    }

    #[must_use]
    pub fn snapshot(&self, state: RecorderState) -> RecorderSnapshot {
        let transcript = self.transcript();
        RecorderSnapshot {
            state: state_name(state),
            started_at: transcript
                .started_at
                .map(|at| at.to_rfc3339_opts(SecondsFormat::Secs, false)),
            lines: transcript.lines.clone(),
        }
    }

    fn transcript(&self) -> MutexGuard<'_, Transcript> {
        self.0.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

#[must_use]
pub fn state_name(state: RecorderState) -> &'static str {
    match state {
        RecorderState::Locked => "locked",
        RecorderState::Idle => "idle",
        RecorderState::Recording => "recording",
    }
}
