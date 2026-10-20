use std::sync::{Arc, PoisonError, RwLock, RwLockWriteGuard};
use std::time::Duration;

use crate::recording::{SessionClock, SessionId};

#[derive(Debug, Clone)]
pub struct LiveSession {
    id: SessionId,
    clock: SessionClock,
}

impl LiveSession {
    pub(crate) fn new(id: SessionId, clock: SessionClock) -> LiveSession {
        LiveSession { id, clock }
    }

    #[must_use]
    pub fn id(&self) -> &SessionId {
        &self.id
    }

    #[must_use]
    pub fn offset(&self) -> Duration {
        self.clock.elapsed()
    }
}

#[derive(Debug, Clone, Default)]
pub struct LiveSessionSlot(Arc<RwLock<Option<LiveSession>>>);

impl LiveSessionSlot {
    #[must_use]
    pub fn current(&self) -> Option<LiveSession> {
        self.0
            .read()
            .unwrap_or_else(PoisonError::into_inner)
            .clone()
    }

    pub(crate) fn publish(&self, session: LiveSession) {
        *self.slot() = Some(session);
    }

    pub(crate) fn clear(&self) {
        *self.slot() = None;
    }

    fn slot(&self) -> RwLockWriteGuard<'_, Option<LiveSession>> {
        self.0.write().unwrap_or_else(PoisonError::into_inner)
    }
}
