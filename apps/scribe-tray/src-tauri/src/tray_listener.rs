use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender};
use std::thread;
use std::time::{Duration, Instant};

use chrono::Local;
use scribe_core::ports::RecordingListener;
use scribe_core::recording::{RecordingEvent, Segment, SessionId, Speaker};
use serde::Serialize;
use tauri::{AppHandle, Manager, Runtime};

use crate::live_feed::{LiveFeed, LiveLine};
use crate::notifications;
use crate::recording::{self, RecordingController};
use crate::tray;
use crate::webview;

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
            RecordingEvent::SegmentFinalised(segment) => self.segment(&segment),
            RecordingEvent::RecordingStopped { id } => self.stopped(&id),
            RecordingEvent::CaptureSilenced { speaker } => self.capture_silenced(speaker),
            RecordingEvent::CaptureUnavailable { speaker } => self.capture_unavailable(speaker),
            RecordingEvent::OutputDeviceChanged => {
                tracing::info!("the output device changed; the other side is being captured again");
            }
            RecordingEvent::ConversationWentQuiet { id } => self.went_quiet(&id),
        }
    }

    fn went_quiet(&self, id: &SessionId) {
        if self
            .app
            .state::<RecordingController>()
            .stop_if_recording(id)
        {
            notifications::notify_recording_auto_stopped(&self.app);
        }
    }

    fn capture_silenced(&self, speaker: Speaker) {
        match speaker {
            Speaker::Them => notifications::notify_cannot_hear_other_side(&self.app),
            Speaker::Me => tracing::warn!("the microphone delivers only silence"),
        }
    }

    fn capture_unavailable(&self, speaker: Speaker) {
        match speaker {
            Speaker::Them => notifications::notify_other_side_unavailable(&self.app),
            Speaker::Me => tracing::warn!("the microphone could not be started"),
        }
    }

    fn started(&mut self, id: &SessionId) {
        self.recording_since = Some(Instant::now());
        self.app
            .state::<LiveFeed>()
            .started(Local::now().fixed_offset());
        recording::publish_state(&self.app);
        self.emit(STARTED_EVENT, SessionPayload { id: id.to_string() });
    }

    fn partial(&self, speaker: Speaker, text: String) {
        let payload = PartialPayload {
            speaker: speaker.label(),
            text,
        };
        self.emit(PARTIAL_EVENT, payload);
    }

    fn segment(&self, segment: &Segment) {
        self.app.state::<LiveFeed>().finalised(segment);
        self.emit(SEGMENT_EVENT, LiveLine::from(segment));
    }

    fn stopped(&mut self, id: &SessionId) {
        self.recording_since = None;
        self.app.state::<LiveFeed>().stopped();
        self.app.state::<RecordingController>().stop_session(id);
        recording::publish_state(&self.app);
        self.emit(STOPPED_EVENT, SessionPayload { id: id.to_string() });
    }

    fn tick(&self) {
        if let Some(since) = self.recording_since {
            tray::show_elapsed(&self.app, since.elapsed());
        }
    }

    fn emit<P: Serialize + Clone>(&self, event: &str, payload: P) {
        webview::emit(&self.app, event, payload);
    }
}
