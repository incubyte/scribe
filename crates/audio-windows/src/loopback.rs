use std::sync::Arc;
use std::sync::mpsc::{self, Receiver};

use scribe_core::ports::{AudioSource, CaptureError, RecordingListener};
use scribe_core::recording::{AudioChunk, SessionClock};

use crate::capture::{Capture, Delivery};

pub struct LoopbackSource {
    listener: Arc<dyn RecordingListener>,
    capture: Option<Capture>,
}

impl LoopbackSource {
    #[must_use]
    pub fn new(listener: Arc<dyn RecordingListener>) -> LoopbackSource {
        LoopbackSource {
            listener,
            capture: None,
        }
    }
}

impl AudioSource for LoopbackSource {
    fn start(&mut self, clock: SessionClock) -> Result<Receiver<AudioChunk>, CaptureError> {
        let (chunks, received) = mpsc::channel();
        let delivery = Delivery {
            clock,
            chunks,
            listener: Arc::clone(&self.listener),
        };
        self.capture = Some(Capture::spawn(delivery)?);
        Ok(received)
    }

    fn stop(&mut self) {
        if let Some(capture) = self.capture.take() {
            capture.stop();
        }
    }
}

impl Drop for LoopbackSource {
    fn drop(&mut self) {
        self.stop();
    }
}
