use std::time::{Duration, Instant};

pub struct CheckSchedule {
    interval: Duration,
    last_check: Option<Instant>,
}

impl CheckSchedule {
    #[must_use]
    pub fn new(interval: Duration) -> Self {
        Self {
            interval,
            last_check: None,
        }
    }

    #[must_use]
    pub fn is_check_due(&self, now: Instant) -> bool {
        self.last_check
            .is_none_or(|last| now.duration_since(last) >= self.interval)
    }

    pub fn mark_checked(&mut self, now: Instant) {
        self.last_check = Some(now);
    }
}
