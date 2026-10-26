use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Mutex, MutexGuard, PoisonError};

use scribe_core::calendar::TodaysAgenda;

use super::agenda_snapshot::{AgendaSnapshot, FetchedAgenda};
use super::agenda_view::AgendaView;

#[derive(Default)]
pub struct AgendaCache {
    snapshot: Mutex<AgendaSnapshot>,
    nudged: AtomicBool,
}

impl AgendaCache {
    #[must_use]
    pub fn new() -> AgendaCache {
        AgendaCache::default()
    }

    #[must_use]
    pub fn view(&self) -> AgendaView {
        AgendaView::of(&self.snapshot())
    }

    #[must_use]
    pub fn agenda(&self) -> Option<TodaysAgenda> {
        let fetched = self.snapshot().fetched.clone()?;
        Some(fetched.agenda)
    }

    pub fn record_fetch(&self, fetched: FetchedAgenda) -> AgendaView {
        self.replace(|_| AgendaSnapshot::after_fetch(fetched))
    }

    pub fn record_failure(&self) -> AgendaView {
        self.replace(AgendaSnapshot::after_failure)
    }

    pub fn clear(&self) {
        self.replace(|_| AgendaSnapshot::default());
    }

    pub fn nudge(&self) {
        self.nudged.store(true, Ordering::Release);
    }

    pub fn take_nudge(&self) -> bool {
        self.nudged.swap(false, Ordering::AcqRel)
    }

    fn replace(&self, next: impl FnOnce(AgendaSnapshot) -> AgendaSnapshot) -> AgendaView {
        let mut snapshot = self.snapshot();
        *snapshot = next(std::mem::take(&mut *snapshot));
        AgendaView::of(&snapshot)
    }

    fn snapshot(&self) -> MutexGuard<'_, AgendaSnapshot> {
        self.snapshot.lock().unwrap_or_else(PoisonError::into_inner)
    }
}
