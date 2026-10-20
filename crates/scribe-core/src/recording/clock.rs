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

    #[cfg(feature = "testing")]
    pub(crate) fn started_ago(elapsed: Duration) -> SessionClock {
        SessionClock {
            started: Instant::now()
                .checked_sub(elapsed)
                .expect("a session clock cannot start before the machine's clock began"),
        }
    }

    #[must_use]
    pub fn elapsed(&self) -> Duration {
        self.started.elapsed()
    }
}
