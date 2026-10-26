use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Mutex, PoisonError};
use std::thread;
use std::time::{Duration, Instant};

use chrono::{DateTime, FixedOffset, Local};
use scribe_core::call::{CALL_END_GRACE, CallWatch};
use scribe_core::ports::CallProbe;
use tauri::{AppHandle, Manager, Runtime};

use crate::calendar;
use crate::call_probes;
use crate::popover;
use crate::recording;
use crate::updates;

const POLL_INTERVAL: Duration = Duration::from_secs(2);
const THREAD_NAME: &str = "call-watch";

#[derive(Default)]
pub struct CallPresence {
    in_call: AtomicBool,
    call_started_at: Mutex<Option<DateTime<FixedOffset>>>,
    call_titles: Mutex<Vec<String>>,
}

impl CallPresence {
    pub fn is_in_call(&self) -> bool {
        self.in_call.load(Ordering::Relaxed)
    }

    pub fn call_started_at(&self) -> Option<DateTime<FixedOffset>> {
        *self
            .call_started_at
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
    }

    pub fn call_titles(&self) -> Vec<String> {
        self.call_titles
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clone()
    }

    pub fn remember_call_titles(&self, titles: Vec<String>) {
        *self
            .call_titles
            .lock()
            .unwrap_or_else(PoisonError::into_inner) = titles;
    }

    fn publish(&self, in_call: bool) -> bool {
        let was_in_call = self.in_call.swap(in_call, Ordering::Relaxed);
        let changed = was_in_call != in_call;
        if changed {
            *self
                .call_started_at
                .lock()
                .unwrap_or_else(PoisonError::into_inner) =
                in_call.then(|| Local::now().fixed_offset());
            self.remember_call_titles(Vec::new());
        }
        changed
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
    let in_call = watch.is_in_call();
    let presence_changed = app.state::<CallPresence>().publish(in_call);
    if presence_changed {
        updates::refresh_restart_item(app);
    }
    if presence_changed && in_call {
        calendar::note_call_started(app);
    }
    if presence_changed && !in_call {
        calendar::note_call_ended(app);
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
