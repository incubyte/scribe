use std::sync::mpsc::{Receiver, RecvTimeoutError};
use std::time::Duration;

use scribe_core::ports::CaptureError;

use crate::stream::LoopbackStream;

const RETRY_DELAYS: [Duration; 4] = [
    Duration::ZERO,
    Duration::from_secs(1),
    Duration::from_secs(2),
    Duration::from_secs(4),
];

pub(crate) enum Recovery {
    Resumed(Box<LoopbackStream>),
    Stopped,
    GaveUp(CaptureError),
}

pub(crate) fn reopen_after_device_change(
    stale: LoopbackStream,
    stop_signal: &Receiver<()>,
) -> Recovery {
    drop(stale);
    let mut last_error = CaptureError::DeviceUnavailable;
    for delay in RETRY_DELAYS {
        if !matches!(
            stop_signal.recv_timeout(delay),
            Err(RecvTimeoutError::Timeout)
        ) {
            return Recovery::Stopped;
        }
        match LoopbackStream::open() {
            Ok(stream) => return Recovery::Resumed(Box::new(stream)),
            Err(error) => last_error = error,
        }
    }
    Recovery::GaveUp(last_error)
}
