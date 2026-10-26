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

enum PresenceChange {
    Unchanged,
    CallStarted,
    CallEnded,
}

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

    fn update_in_call(&self, in_call: bool) -> PresenceChange {
        let was_in_call = self.in_call.swap(in_call, Ordering::Relaxed);
        match (was_in_call, in_call) {
            (false, true) => PresenceChange::CallStarted,
            (true, false) => PresenceChange::CallEnded,
            _ => PresenceChange::Unchanged,
        }
    }

    fn begin_call_details(&self) {
        self.replace_call_start(Some(Local::now().fixed_offset()));
        self.remember_call_titles(Vec::new());
    }

    fn clear_call_details(&self) {
        self.replace_call_start(None);
        self.remember_call_titles(Vec::new());
    }

    fn replace_call_start(&self, started_at: Option<DateTime<FixedOffset>>) {
        *self
            .call_started_at
            .lock()
            .unwrap_or_else(PoisonError::into_inner) = started_at;
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
                calendar::on_poll(app, watch.is_in_call());
                open_if_call_started(app, watch);
            }
            Err(error) => tracing::debug!(%error, "could not read which apps hold the microphone"),
        }
    }
}

fn publish_presence<R: Runtime>(app: &AppHandle<R>, watch: &CallWatch) {
    match app
        .state::<CallPresence>()
        .update_in_call(watch.is_in_call())
    {
        PresenceChange::Unchanged => {}
        PresenceChange::CallStarted => call_started(app),
        PresenceChange::CallEnded => call_ended(app),
    }
}

fn call_started<R: Runtime>(app: &AppHandle<R>) {
    app.state::<CallPresence>().begin_call_details();
    updates::refresh_restart_item(app);
    calendar::note_call_started(app);
}

fn call_ended<R: Runtime>(app: &AppHandle<R>) {
    app.state::<CallPresence>().clear_call_details();
    updates::refresh_restart_item(app);
    calendar::stop_if_auto_started(app);
    calendar::note_call_ended(app);
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
