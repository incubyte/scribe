use std::thread;
use std::time::{Duration, Instant};

use scribe_core::call::{CALL_END_GRACE, CallWatch};
use scribe_core::ports::CallProbe;
use scribe_core::recording::RecorderState;
use tauri::{AppHandle, Manager, Runtime};

use crate::call_probes;
use crate::popover;
use crate::recording::RecordingController;

const POLL_INTERVAL: Duration = Duration::from_secs(2);
const THREAD_NAME: &str = "call-watch";

pub fn install<R: Runtime>(app: &AppHandle<R>) {
    let handle = app.clone();
    let spawned = thread::Builder::new()
        .name(THREAD_NAME.to_owned())
        .spawn(move || {
            let probe = call_probes::call_probe();
            let mut watch = CallWatch::new(CALL_END_GRACE);
            watch_forever(&handle, probe.as_ref(), &mut watch);
        });
    if let Err(error) = spawned {
        tracing::error!(%error, "could not start watching for calls");
    }
}

fn watch_forever<R: Runtime>(app: &AppHandle<R>, probe: &dyn CallProbe, watch: &mut CallWatch) {
    loop {
        thread::sleep(POLL_INTERVAL);
        match probe.call_app_holds_microphone() {
            Ok(holds_microphone) => {
                watch.observe(holds_microphone, Instant::now());
                open_if_call_started(app, watch);
            }
            Err(error) => tracing::debug!(%error, "could not read which apps hold the microphone"),
        }
    }
}

fn open_if_call_started<R: Runtime>(app: &AppHandle<R>, watch: &mut CallWatch) {
    if !watch.is_start_due() {
        return;
    }
    watch.mark_reported();
    open_unless_recording(app);
}

fn open_unless_recording<R: Runtime>(app: &AppHandle<R>) {
    if app.state::<RecordingController>().state() == RecorderState::Recording {
        tracing::info!("a call started while recording; the popover stays as it is");
        return;
    }
    tracing::info!("a call started; opening the popover");
    popover::open_for_call(app);
}
