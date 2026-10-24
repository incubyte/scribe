use std::time::{Duration, Instant};

pub const CALL_END_GRACE: Duration = Duration::from_secs(30);

pub struct CallWatch {
    grace: Duration,
    in_call: bool,
    released_at: Option<Instant>,
    reported: bool,
}

impl CallWatch {
    #[must_use]
    pub fn new(grace: Duration) -> CallWatch {
        CallWatch {
            grace,
            in_call: false,
            released_at: None,
            reported: false,
        }
    }

    pub fn observe(&mut self, call_app_holds_microphone: bool, now: Instant) {
        if call_app_holds_microphone {
            self.hold();
        } else {
            self.release(now);
        }
    }

    pub fn mark_reported(&mut self) {
        self.reported = true;
    }

    #[must_use]
    pub fn is_start_due(&self) -> bool {
        self.in_call && !self.reported
    }

    #[must_use]
    pub fn is_in_call(&self) -> bool {
        self.in_call
    }

    fn hold(&mut self) {
        self.released_at = None;
        if !self.in_call {
            self.in_call = true;
            self.reported = false;
        }
    }

    fn release(&mut self, now: Instant) {
        if !self.in_call {
            return;
        }
        let released_at = *self.released_at.get_or_insert(now);
        if now.saturating_duration_since(released_at) >= self.grace {
            self.in_call = false;
            self.released_at = None;
        }
    }
}
