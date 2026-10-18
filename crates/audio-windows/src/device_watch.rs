use std::sync::mpsc::Sender;
use std::time::{Duration, Instant};

use scribe_core::ports::CaptureError;
use wasapi::{DeviceEnumerator, DeviceEventCallbacks, DeviceEventRegistration, Direction, Role};

const SETTLE_TIME: Duration = Duration::from_millis(300);

pub(crate) struct OutputDeviceWatch {
    _registration: DeviceEventRegistration,
}

impl OutputDeviceWatch {
    pub(crate) fn new(on_change: Sender<()>) -> Result<OutputDeviceWatch, CaptureError> {
        let mut callbacks = DeviceEventCallbacks::new();
        callbacks.set_default_device_callback(move |direction, role, _device| {
            if is_default_output(direction, role) {
                let _ = on_change.send(());
            }
        });
        let registration = DeviceEnumerator::new()
            .and_then(|enumerator| enumerator.register_notification_callback(callbacks))
            .map_err(|error| {
                CaptureError::StreamFailed(format!(
                    "could not watch the default output device: {error}"
                ))
            })?;
        Ok(OutputDeviceWatch {
            _registration: registration,
        })
    }
}

fn is_default_output(direction: Direction, role: Role) -> bool {
    matches!((direction, role), (Direction::Render, Role::Console))
}

#[derive(Default)]
pub(crate) struct ChangeDebounce {
    settles_at: Option<Instant>,
}

impl ChangeDebounce {
    pub(crate) fn notice(&mut self, now: Instant) {
        self.settles_at = Some(now + SETTLE_TIME);
    }

    pub(crate) fn take_settled(&mut self, now: Instant) -> bool {
        let settled = self.settles_at.is_some_and(|settles_at| now >= settles_at);
        if settled {
            self.settles_at = None;
        }
        settled
    }
}
