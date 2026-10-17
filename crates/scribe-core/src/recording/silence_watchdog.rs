use std::time::Duration;

use crate::recording::AudioChunk;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WatchdogVerdict {
    Healthy,
    SilencedFor(Duration),
    AlreadyReported,
}

pub struct SilenceWatchdog {
    threshold: Duration,
    silent_since: Option<Duration>,
    reported: bool,
}

impl SilenceWatchdog {
    #[must_use]
    pub fn new(threshold: Duration) -> SilenceWatchdog {
        SilenceWatchdog {
            threshold,
            silent_since: None,
            reported: false,
        }
    }

    pub fn observe(&mut self, chunk: &AudioChunk) -> WatchdogVerdict {
        if !chunk.is_exact_silence() {
            self.silent_since = None;
            self.reported = false;
            return WatchdogVerdict::Healthy;
        }
        let silent_since = *self.silent_since.get_or_insert(chunk.captured_at);
        self.judge(chunk.ends_at().saturating_sub(silent_since))
    }

    fn judge(&mut self, silent_for: Duration) -> WatchdogVerdict {
        if self.reported {
            WatchdogVerdict::AlreadyReported
        } else if silent_for >= self.threshold {
            self.reported = true;
            WatchdogVerdict::SilencedFor(silent_for)
        } else {
            WatchdogVerdict::Healthy
        }
    }
}
