mod audio;
mod clock;
mod echo_filter;
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
pub use echo_filter::{ECHO_OVERLAP_TOLERANCE, ECHO_SIMILARITY_THRESHOLD, EchoFilter};
pub use events::RecordingEvent;
pub use offset::{offset_as_clock, offset_from_clock};
pub use readiness::{Readiness, RecorderState, recorder_state};
pub use recorder::{CapturingLanes, Lane, Recorder, RecorderError};
pub use segment::{Segment, Speaker, TranscriptEvent, Transcription, Utterance};
pub use sequencer::{MAX_HOLD, SegmentSequencer};
pub use session::{
    InvalidSessionId, SessionDocument, SessionHeader, SessionId, SessionStatus, SessionSummary,
    UnknownSessionStatus, default_title,
};
pub use silence_watchdog::{SilenceWatchdog, WatchdogVerdict};
