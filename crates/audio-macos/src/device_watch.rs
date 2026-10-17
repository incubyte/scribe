#![allow(unsafe_code)]

use std::sync::mpsc::Sender;
use std::time::{Duration, Instant};

use cidre::{core_audio as ca, os};
use scribe_core::ports::CaptureError;

const SETTLE_TIME: Duration = Duration::from_millis(300);
const DEFAULT_OUTPUT_DEVICE: ca::PropAddr =
    ca::PropSelector::HW_DEFAULT_OUTPUT_DEVICE.global_addr();

pub(crate) struct OutputDeviceWatch {
    on_change: *mut Sender<()>,
}

impl OutputDeviceWatch {
    pub(crate) fn new(on_change: Sender<()>) -> Result<OutputDeviceWatch, CaptureError> {
        let on_change = Box::into_raw(Box::new(on_change));
        let registered = ca::System::OBJ.add_prop_listener(
            &DEFAULT_OUTPUT_DEVICE,
            output_device_changed,
            on_change,
        );
        match registered {
            Ok(()) => Ok(OutputDeviceWatch { on_change }),
            Err(status) => {
                drop(unsafe { Box::from_raw(on_change) });
                Err(CaptureError::StreamFailed(format!(
                    "could not watch the default output device: {status:?}"
                )))
            }
        }
    }
}

impl Drop for OutputDeviceWatch {
    fn drop(&mut self) {
        let removed = ca::System::OBJ.remove_prop_listener(
            &DEFAULT_OUTPUT_DEVICE,
            output_device_changed,
            self.on_change,
        );
        match removed {
            Ok(()) => drop(unsafe { Box::from_raw(self.on_change) }),
            Err(status) => tracing::warn!(?status, "could not stop watching the output device"),
        }
    }
}

extern "C-unwind" fn output_device_changed(
    _system: ca::Obj,
    _address_count: u32,
    _addresses: *const ca::PropAddr,
    on_change: *mut Sender<()>,
) -> os::Status {
    if let Some(on_change) = unsafe { on_change.as_ref() } {
        let _ = on_change.send(());
    }
    os::Status::NO_ERR
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
