use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender};
use std::thread;
use std::time::{Duration, Instant};

use scribe_core::ports::RecordingListener;
use scribe_core::recording::{RecorderState, RecordingEvent, Segment, SessionId, Speaker};
use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager, Runtime};

use crate::notifications;
use crate::recording::RecordingController;
use crate::tray;

const TICK: Duration = Duration::from_secs(1);
const STARTED_EVENT: &str = "recording://started";
const PARTIAL_EVENT: &str = "recording://partial";
const SEGMENT_EVENT: &str = "recording://segment";
const STOPPED_EVENT: &str = "recording://stopped";

pub struct TrayRecordingListener {
    events: Sender<RecordingEvent>,
}

impl TrayRecordingListener {
    pub fn spawn<R: Runtime>(app: AppHandle<R>) -> TrayRecordingListener {
        let (events, received) = mpsc::channel();
        let updater = TrayUpdater {
            app,
            recording_since: None,
        };
        thread::spawn(move || updater.run(&received));
        TrayRecordingListener { events }
    }
}

impl RecordingListener for TrayRecordingListener {
    fn on_event(&self, event: RecordingEvent) {
        if self.events.send(event).is_err() {
            tracing::warn!("the tray updater has stopped; recording updates are not shown");
        }
    }
}

#[derive(Clone, Serialize)]
struct SessionPayload {
    id: String,
}

#[derive(Clone, Serialize)]
struct PartialPayload {
    speaker: &'static str,
    text: String,
}

#[derive(Clone, Serialize)]
struct SegmentPayload {
    speaker: &'static str,
    start_ms: u128,
    text: String,
}

struct TrayUpdater<R: Runtime> {
    app: AppHandle<R>,
    recording_since: Option<Instant>,
}

impl<R: Runtime> TrayUpdater<R> {
    fn run(mut self, events: &Receiver<RecordingEvent>) {
        loop {
            match events.recv_timeout(TICK) {
                Ok(event) => self.apply(event),
                Err(RecvTimeoutError::Timeout) => self.tick(),
                Err(RecvTimeoutError::Disconnected) => return,
            }
        }
    }

    fn apply(&mut self, event: RecordingEvent) {
        match event {
            RecordingEvent::RecordingStarted { id } => self.started(&id),
            RecordingEvent::PartialSegmentUpdated { speaker, text } => self.partial(speaker, text),
            RecordingEvent::SegmentFinalised(segment) => self.segment(segment),
            RecordingEvent::RecordingStopped { id } => self.stopped(&id),
            RecordingEvent::CaptureSilenced { speaker } => self.capture_silenced(speaker),
            RecordingEvent::OutputDeviceChanged => {
                tracing::info!("the output device changed; the other side is being captured again");
            }
        }
    }

    fn capture_silenced(&self, speaker: Speaker) {
        match speaker {
            Speaker::Them => notifications::notify_cannot_hear_other_side(&self.app),
            Speaker::Me => tracing::warn!("the microphone delivers only silence"),
        }
    }

    fn started(&mut self, id: &SessionId) {
        self.recording_since = Some(Instant::now());
        tray::show_recorder_state(&self.app, RecorderState::Recording);
        self.emit(STARTED_EVENT, SessionPayload { id: id.to_string() });
    }

    fn partial(&self, speaker: Speaker, text: String) {
        let payload = PartialPayload {
            speaker: speaker.label(),
            text,
        };
        self.emit(PARTIAL_EVENT, payload);
    }

    fn segment(&self, segment: Segment) {
        let payload = SegmentPayload {
            speaker: segment.speaker.label(),
            start_ms: segment.start.as_millis(),
            text: segment.text,
        };
        self.emit(SEGMENT_EVENT, payload);
    }

    fn stopped(&mut self, id: &SessionId) {
        self.recording_since = None;
        let controller = self.app.state::<RecordingController>();
        controller.stop();
        tray::show_recorder_state(&self.app, controller.state());
        self.emit(STOPPED_EVENT, SessionPayload { id: id.to_string() });
    }

    fn tick(&self) {
        if let Some(since) = self.recording_since {
            tray::show_elapsed(&self.app, since.elapsed());
        }
    }

    fn emit<P: Serialize + Clone>(&self, event: &str, payload: P) {
        if let Err(error) = self.app.emit(event, payload) {
            tracing::warn!(event, %error, "could not notify the webview");
        }
    }
}
