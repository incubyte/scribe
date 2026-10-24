use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use std::time::{Duration, Instant};

use scribe_core::call::{CALL_END_GRACE, CallWatch};
use scribe_core::ports::CallProbe;
use tauri::{AppHandle, Manager, Runtime};

use crate::call_probes;
use crate::popover;
use crate::recording;
use crate::updates;

const POLL_INTERVAL: Duration = Duration::from_secs(2);
const THREAD_NAME: &str = "call-watch";

#[derive(Default)]
pub struct CallPresence(AtomicBool);

impl CallPresence {
    pub fn is_in_call(&self) -> bool {
        self.0.load(Ordering::Relaxed)
    }

    fn publish(&self, in_call: bool) -> bool {
        let was_in_call = self.0.swap(in_call, Ordering::Relaxed);
        was_in_call != in_call
    }
}

pub fn install<R: Runtime>(app: &AppHandle<R>) {
    app.manage(CallPresence::default());
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
                publish_presence(app, watch);
                open_if_call_started(app, watch);
            }
            Err(error) => tracing::debug!(%error, "could not read which apps hold the microphone"),
        }
    }
}

fn publish_presence<R: Runtime>(app: &AppHandle<R>, watch: &CallWatch) {
    let presence_changed = app.state::<CallPresence>().publish(watch.is_in_call());
    if presence_changed {
        updates::refresh_restart_item(app);
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
    if recording::is_recording(app) {
        tracing::info!("a call started while recording; the popover stays as it is");
        return;
    }
    tracing::info!("a call started; opening the popover");
    popover::open_for_call(app);
}
