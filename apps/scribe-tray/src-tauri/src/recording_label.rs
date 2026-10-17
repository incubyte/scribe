use std::time::Duration;

use scribe_core::recording::offset_as_clock;

pub const START_RECORDING: &str = "Start recording";

#[must_use]
pub fn stop_recording_label(elapsed: Duration) -> String {
    format!("Stop recording ({})", offset_as_clock(elapsed))
}
