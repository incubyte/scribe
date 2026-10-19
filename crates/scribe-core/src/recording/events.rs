use crate::recording::{Segment, SessionId, Speaker};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RecordingEvent {
    RecordingStarted { id: SessionId },
    PartialSegmentUpdated { speaker: Speaker, text: String },
    SegmentFinalised(Segment),
    RecordingStopped { id: SessionId },
    CaptureSilenced { speaker: Speaker },
    CaptureUnavailable { speaker: Speaker },
    OutputDeviceChanged,
    ConversationWentQuiet { id: SessionId },
}
