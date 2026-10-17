use std::sync::mpsc::{self, Receiver};

use scribe_core::ports::{AudioSource, CaptureError};
use scribe_core::recording::{AudioChunk, SessionClock};

use crate::capture::Capture;

#[derive(Default)]
pub struct MicSource {
    capture: Option<Capture>,
}

impl MicSource {
    #[must_use]
    pub fn new() -> MicSource {
        MicSource::default()
    }
}

impl AudioSource for MicSource {
    fn start(&mut self, clock: SessionClock) -> Result<Receiver<AudioChunk>, CaptureError> {
        #[cfg(target_os = "macos")]
        crate::permission::ensure_microphone_access()?;
        let (chunks, received) = mpsc::channel();
        self.capture = Some(Capture::spawn(clock, chunks)?);
        Ok(received)
    }

    fn stop(&mut self) {
        if let Some(capture) = self.capture.take() {
            capture.stop();
        }
    }
}

impl Drop for MicSource {
    fn drop(&mut self) {
        self.stop();
    }
}
