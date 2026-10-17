use std::time::{Duration, Instant};

#[derive(Debug, Clone, Copy)]
pub struct SessionClock {
    started: Instant,
}

impl SessionClock {
    #[must_use]
    pub fn start() -> SessionClock {
        SessionClock {
            started: Instant::now(),
        }
    }

    #[must_use]
    pub fn elapsed(&self) -> Duration {
        self.started.elapsed()
    }
}
