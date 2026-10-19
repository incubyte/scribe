use std::time::Duration;

pub const AUTO_STOP_AFTER_QUIET: Duration = Duration::from_secs(5 * 60);

pub struct QuietWatch {
    limit: Duration,
    heard_through: Duration,
    last_speech: Duration,
    reported: bool,
}

impl QuietWatch {
    #[must_use]
    pub fn new(limit: Duration) -> QuietWatch {
        QuietWatch {
            limit,
            heard_through: Duration::ZERO,
            last_speech: Duration::ZERO,
            reported: false,
        }
    }

    pub fn hear_through(&mut self, heard_through: Duration) {
        self.heard_through = self.heard_through.max(heard_through);
    }

    pub fn hear_speech(&mut self) {
        self.last_speech = self.heard_through;
        self.reported = false;
    }

    pub fn mark_reported(&mut self) {
        self.reported = true;
    }

    #[must_use]
    pub fn is_report_due(&self) -> bool {
        !self.reported && self.quiet_for() >= self.limit
    }

    fn quiet_for(&self) -> Duration {
        self.heard_through.saturating_sub(self.last_speech)
    }
}
