mod audio;
mod clock;
mod events;
mod lane_worker;
mod offset;
mod readiness;
mod recorder;
mod segment;
mod sequencer;
mod session;
mod silence_watchdog;
mod writer;

pub use audio::{AudioChunk, SAMPLE_RATE_HZ};
pub use clock::SessionClock;
pub use events::RecordingEvent;
pub use offset::offset_as_clock;
pub use readiness::{Readiness, RecorderState, recorder_state};
pub use recorder::{Lane, Recorder, RecorderError};
pub use segment::{Segment, Speaker, TranscriptEvent, Utterance};
pub use sequencer::{MAX_HOLD, SegmentSequencer};
pub use session::{
    InvalidSessionId, SessionHeader, SessionId, SessionStatus, SessionSummary,
    UnknownSessionStatus, default_title,
};
pub use silence_watchdog::{SilenceWatchdog, WatchdogVerdict};
