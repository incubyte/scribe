use std::sync::Arc;
use std::sync::mpsc::{self, Receiver, Sender, SyncSender, TryRecvError};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

use scribe_core::ports::{CaptureError, RecordingListener};
use scribe_core::recording::{AudioChunk, RecordingEvent, SessionClock};

use crate::device_watch::{ChangeDebounce, OutputDeviceWatch};
use crate::recovery::{Recovery, reopen_after_device_change};
use crate::stream::LoopbackStream;

const DELIVERY_INTERVAL: Duration = Duration::from_millis(100);
const STARTUP_TIMEOUT: Duration = Duration::from_secs(5);

type StartupReport = Result<(), CaptureError>;

pub(crate) struct Delivery {
    pub(crate) clock: SessionClock,
    pub(crate) chunks: Sender<AudioChunk>,
    pub(crate) listener: Arc<dyn RecordingListener>,
}

pub(crate) struct Capture {
    stop: Sender<()>,
    thread: JoinHandle<()>,
}

impl Capture {
    pub(crate) fn spawn(delivery: Delivery) -> Result<Capture, CaptureError> {
        let (stop, stop_signal) = mpsc::channel();
        let (report, startup) = mpsc::sync_channel(1);
        let thread = thread::spawn(move || run_capture(&delivery, &stop_signal, &report));
        match startup.recv_timeout(STARTUP_TIMEOUT) {
            Ok(Ok(())) => Ok(Capture { stop, thread }),
            Ok(Err(error)) => Err(error),
            Err(_) => Err(CaptureError::DeviceUnavailable),
        }
    }

    pub(crate) fn stop(self) {
        let _ = self.stop.send(());
        if self.thread.join().is_err() {
            tracing::error!("the loopback capture thread panicked");
        }
    }
}

fn run_capture(
    delivery: &Delivery,
    stop_signal: &Receiver<()>,
    report: &SyncSender<StartupReport>,
) {
    if let Err(error) = wasapi::initialize_mta().ok() {
        let _ = report.send(Err(CaptureError::StreamFailed(error.to_string())));
        return;
    }
    match LoopbackStream::open() {
        Ok(stream) => {
            if report.send(Ok(())).is_ok() {
                CaptureSession::watching_output_device(stream).run(delivery, stop_signal);
            }
        }
        Err(error) => {
            let _ = report.send(Err(error));
        }
    }
    wasapi::deinitialize();
}

struct CaptureSession {
    stream: LoopbackStream,
    device_changes: Receiver<()>,
    debounce: ChangeDebounce,
    _watch: Option<OutputDeviceWatch>,
}

impl CaptureSession {
    fn watching_output_device(stream: LoopbackStream) -> CaptureSession {
        let (on_change, device_changes) = mpsc::channel();
        let watch = OutputDeviceWatch::new(on_change)
            .inspect_err(
                |error| tracing::warn!(%error, "output device changes will not be followed"),
            )
            .ok();
        CaptureSession {
            stream,
            device_changes,
            debounce: ChangeDebounce::default(),
            _watch: watch,
        }
    }

    fn run(mut self, delivery: &Delivery, stop_signal: &Receiver<()>) {
        let mut next_delivery = Instant::now();
        while keeps_running(stop_signal) {
            let needs_rebuild = self.capture_once();
            if (needs_rebuild || is_due(&mut next_delivery)) && !self.deliver(delivery) {
                return;
            }
            if needs_rebuild {
                match reopen_after_device_change(self.stream, stop_signal) {
                    Recovery::Resumed(stream) => self.stream = *stream,
                    Recovery::Stopped => return,
                    Recovery::GaveUp(error) => return give_up(&error),
                }
                delivery
                    .listener
                    .on_event(RecordingEvent::OutputDeviceChanged);
            }
        }
        self.deliver(delivery);
    }

    fn capture_once(&mut self) -> bool {
        let stream_failed = self
            .stream
            .wait_and_capture()
            .inspect_err(|error| tracing::warn!(%error, "the loopback stream stopped delivering"))
            .is_err();
        stream_failed || self.output_device_settled_on_a_change()
    }

    fn deliver(&mut self, delivery: &Delivery) -> bool {
        let samples = self.stream.take_samples();
        samples.is_empty()
            || delivery
                .chunks
                .send(AudioChunk::ending_at(samples, delivery.clock.elapsed()))
                .is_ok()
    }

    fn output_device_settled_on_a_change(&mut self) -> bool {
        let now = Instant::now();
        if self.device_changes.try_iter().count() > 0 {
            self.debounce.notice(now);
        }
        self.debounce.take_settled(now)
    }
}

fn keeps_running(stop_signal: &Receiver<()>) -> bool {
    matches!(stop_signal.try_recv(), Err(TryRecvError::Empty))
}

fn is_due(next_delivery: &mut Instant) -> bool {
    let now = Instant::now();
    let due = now >= *next_delivery;
    if due {
        *next_delivery = now + DELIVERY_INTERVAL;
    }
    due
}

fn give_up(error: &CaptureError) {
    tracing::error!(%error, "the other side of the call can no longer be captured");
}
