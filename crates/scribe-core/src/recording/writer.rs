use std::sync::Arc;
use std::sync::mpsc::Receiver;

use chrono::{DateTime, FixedOffset, TimeDelta};

use crate::ports::{RecordingListener, StoreError, TranscriptStore};
use crate::recording::lane_worker::{LaneMessage, LaneProgress};
use crate::recording::{
    RecordingEvent, Segment, SegmentSequencer, SessionClock, SessionId, Speaker,
};

pub(crate) struct SessionWriter {
    pub(crate) store: Arc<dyn TranscriptStore>,
    pub(crate) listener: Arc<dyn RecordingListener>,
    pub(crate) id: SessionId,
    pub(crate) started_at: DateTime<FixedOffset>,
    pub(crate) clock: SessionClock,
    pub(crate) sequencer: SegmentSequencer,
    pub(crate) open_lanes: usize,
}

pub(crate) fn run_writer(
    mut writer: SessionWriter,
    messages: &Receiver<LaneMessage>,
) -> Result<(), StoreError> {
    while writer.open_lanes > 0 {
        let Ok(message) = messages.recv() else { break };
        writer.handle(message);
    }
    writer.finish()
}

impl SessionWriter {
    fn handle(&mut self, message: LaneMessage) {
        match message {
            LaneMessage::Final(segment) => self.sequencer.accept(segment),
            LaneMessage::Transcribing(speaker) => self.sequencer.start_transcribing(speaker),
            LaneMessage::Progress(progress) => self.advance(&progress),
            LaneMessage::Partial(speaker, text) => self
                .listener
                .on_event(RecordingEvent::PartialSegmentUpdated { speaker, text }),
            LaneMessage::Silenced(speaker) => self
                .listener
                .on_event(RecordingEvent::CaptureSilenced { speaker }),
            LaneMessage::Ended(speaker) => self.end_lane(speaker),
        }
        let ready = self.sequencer.release_ready();
        self.write_all(ready);
    }

    fn advance(&mut self, progress: &LaneProgress) {
        self.sequencer
            .advance(progress.speaker, progress.finalised_through);
        self.sequencer
            .hear(progress.speaker, progress.heard_through);
        self.sequencer.finish_transcribing(progress.speaker);
    }

    fn end_lane(&mut self, speaker: Speaker) {
        self.sequencer.finish_transcribing(speaker);
        self.sequencer.close(speaker);
        self.open_lanes = self.open_lanes.saturating_sub(1);
    }

    fn finish(mut self) -> Result<(), StoreError> {
        let remaining = self.sequencer.drain();
        self.write_all(remaining);
        self.store.finalise(&self.id, self.ended_at())?;
        self.listener.on_event(RecordingEvent::RecordingStopped {
            id: self.id.clone(),
        });
        Ok(())
    }

    fn write_all(&self, segments: Vec<Segment>) {
        for segment in segments {
            self.write(segment);
        }
    }

    fn write(&self, segment: Segment) {
        match self.store.append(&self.id, &segment) {
            Ok(()) => self
                .listener
                .on_event(RecordingEvent::SegmentFinalised(segment)),
            Err(error) => {
                tracing::error!(session = %self.id, %error, "could not save a finished segment");
            }
        }
    }

    fn ended_at(&self) -> DateTime<FixedOffset> {
        let elapsed = TimeDelta::from_std(self.clock.elapsed()).unwrap_or(TimeDelta::zero());
        self.started_at + elapsed
    }
}
