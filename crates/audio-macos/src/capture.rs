use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender, SyncSender};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

use scribe_core::ports::CaptureError;
use scribe_core::recording::RecordingEvent;

use crate::device_watch::{ChangeDebounce, OutputDeviceWatch};
use crate::pump::{Delivery, Pump};
use crate::recovery::{Recovery, reopen_after_device_change};

const PUMP_INTERVAL: Duration = Duration::from_millis(100);
const PERMISSION_PROMPT_TIMEOUT: Duration = Duration::from_secs(120);

type StartupReport = Result<(), CaptureError>;

pub(crate) struct Capture {
    stop: Sender<()>,
    thread: JoinHandle<()>,
}

impl Capture {
    pub(crate) fn spawn(delivery: Delivery) -> Result<Capture, CaptureError> {
        let (stop, stop_signal) = mpsc::channel();
        let (report, startup) = mpsc::sync_channel(1);
        let thread = thread::spawn(move || run_capture(&delivery, &stop_signal, &report));
        match startup.recv_timeout(PERMISSION_PROMPT_TIMEOUT) {
            Ok(Ok(())) => Ok(Capture { stop, thread }),
            Ok(Err(error)) => Err(error),
            Err(_) => Err(CaptureError::DeviceUnavailable),
        }
    }

    pub(crate) fn stop(self) {
        let _ = self.stop.send(());
        if self.thread.join().is_err() {
            tracing::error!("the system audio capture thread panicked");
        }
    }
}

fn run_capture(
    delivery: &Delivery,
    stop_signal: &Receiver<()>,
    report: &SyncSender<StartupReport>,
) {
    match Pump::open() {
        Ok(pump) => {
            if report.send(Ok(())).is_ok() {
                CaptureSession::watching_output_device(pump).run(delivery, stop_signal);
            }
        }
        Err(error) => {
            let _ = report.send(Err(error));
        }
    }
}

struct CaptureSession {
    pump: Pump,
    device_changes: Receiver<()>,
    debounce: ChangeDebounce,
    _watch: Option<OutputDeviceWatch>,
}

impl CaptureSession {
    fn watching_output_device(pump: Pump) -> CaptureSession {
        let (on_change, device_changes) = mpsc::channel();
        let watch = OutputDeviceWatch::new(on_change)
            .inspect_err(
                |error| tracing::warn!(%error, "output device changes will not be followed"),
            )
            .ok();
        CaptureSession {
            pump,
            device_changes,
            debounce: ChangeDebounce::default(),
            _watch: watch,
        }
    }

    fn run(mut self, delivery: &Delivery, stop_signal: &Receiver<()>) {
        while let Err(RecvTimeoutError::Timeout) = stop_signal.recv_timeout(PUMP_INTERVAL) {
            if !self.pump.forward_captured_audio(delivery) {
                return;
            }
            if self.output_device_settled_on_a_change() {
                match reopen_after_device_change(self.pump, stop_signal) {
                    Recovery::Resumed(pump) => self.pump = *pump,
                    Recovery::Stopped => return,
                    Recovery::GaveUp(error) => return give_up(&error),
                }
                delivery
                    .listener
                    .on_event(RecordingEvent::OutputDeviceChanged);
            }
        }
        self.pump.forward_captured_audio(delivery);
    }

    fn output_device_settled_on_a_change(&mut self) -> bool {
        let now = Instant::now();
        if self.device_changes.try_iter().count() > 0 {
            self.debounce.notice(now);
        }
        self.debounce.take_settled(now)
    }
}

fn give_up(error: &CaptureError) {
    tracing::error!(%error, "the other side of the call can no longer be captured");
}
