use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Mutex, MutexGuard, PoisonError};

use scribe_core::capture::{PendingCapture, Selection};

#[derive(Default)]
pub struct CaptureGate {
    busy: AtomicBool,
    pending: Mutex<Option<PendingCapture>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum RegionRefusal {
    #[error("no region capture is waiting for a selection")]
    NothingPending,
    #[error("the selected area cannot be captured")]
    SelectionRefused,
}

impl CaptureGate {
    #[must_use]
    pub fn claim(&self) -> Option<CaptureClaim<'_>> {
        let claim = CaptureClaim::take(self)?;
        self.pending().is_none().then_some(claim)
    }

    pub fn take_accepting(&self, selection: Selection) -> Result<PendingCapture, RegionRefusal> {
        let mut pending = self.pending();
        if pending.is_none() {
            return Err(RegionRefusal::NothingPending);
        }
        pending
            .take_if(|capture| capture.accepts(selection))
            .ok_or(RegionRefusal::SelectionRefused)
    }

    pub fn read_pending<T>(&self, read: impl FnOnce(&PendingCapture) -> T) -> Option<T> {
        self.pending().as_ref().map(read)
    }

    pub fn discard_pending(&self) {
        self.pending().take();
    }

    fn pending(&self) -> MutexGuard<'_, Option<PendingCapture>> {
        self.pending.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

pub struct CaptureClaim<'a>(&'a CaptureGate);

impl<'a> CaptureClaim<'a> {
    fn take(gate: &'a CaptureGate) -> Option<CaptureClaim<'a>> {
        gate.busy
            .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
            .ok()
            .map(|_| CaptureClaim(gate))
    }

    pub fn park(self, pending: PendingCapture) {
        *self.0.pending() = Some(pending);
    }
}

impl Drop for CaptureClaim<'_> {
    fn drop(&mut self) {
        self.0.busy.store(false, Ordering::Release);
    }
}
