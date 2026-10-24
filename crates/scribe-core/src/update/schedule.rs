use std::time::{Duration, Instant};

pub struct CheckSchedule {
    interval: Duration,
    next_check: Option<Instant>,
}

impl CheckSchedule {
    #[must_use]
    pub fn new(interval: Duration) -> Self {
        Self {
            interval,
            next_check: None,
        }
    }

    #[must_use]
    pub fn is_check_due(&self, now: Instant) -> bool {
        self.next_check.is_none_or(|next| now >= next)
    }

    pub fn mark_checked(&mut self, now: Instant) {
        self.next_check = Some(now + self.interval);
    }

    pub fn check_soon(&mut self, at: Instant) {
        if let Some(next) = self.next_check {
            self.next_check = Some(next.min(at));
        }
    }
}
