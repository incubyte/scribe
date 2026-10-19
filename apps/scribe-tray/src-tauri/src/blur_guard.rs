use std::time::{Duration, Instant};

const SETTLE: Duration = Duration::from_millis(300);

#[derive(Debug, Default)]
pub struct BlurGuard {
    shown_at: Option<Instant>,
    hidden_by_blur_at: Option<Instant>,
    dialog_open: bool,
}

impl BlurGuard {
    pub fn shown(&mut self, now: Instant) {
        self.shown_at = Some(now);
    }

    pub fn dialog_opened(&mut self) {
        self.dialog_open = true;
    }

    pub fn dialog_closed(&mut self) {
        self.dialog_open = false;
    }

    pub fn blur_should_hide(&mut self, now: Instant) -> bool {
        let settled = !self.dialog_open && Self::has_settled(self.shown_at, now);
        if settled {
            self.hidden_by_blur_at = Some(now);
        }
        settled
    }

    #[must_use]
    pub fn click_may_show(&self, now: Instant) -> bool {
        Self::has_settled(self.hidden_by_blur_at, now)
    }

    fn has_settled(since: Option<Instant>, now: Instant) -> bool {
        since.is_none_or(|moment| now.saturating_duration_since(moment) >= SETTLE)
    }
}
