use std::sync::mpsc::{Receiver, Sender};
use std::time::Duration;

use crate::ports::{TranscribeError, Transcriber};
use crate::recording::{
    AudioChunk, Segment, SilenceWatchdog, Speaker, TranscriptEvent, Utterance, WatchdogVerdict,
};

const SILENCE_WARNING_AFTER: Duration = Duration::from_secs(5);

pub(crate) enum LaneMessage {
    Final(Segment),
    Watermark(Speaker, Duration),
    Partial(Speaker, String),
    Silenced(Speaker),
    Ended(Speaker),
}

pub(crate) struct LaneWorker {
    pub(crate) speaker: Speaker,
    pub(crate) transcriber: Box<dyn Transcriber>,
    pub(crate) out: Sender<LaneMessage>,
}

pub(crate) fn run_lane(worker: LaneWorker, chunks: &Receiver<AudioChunk>) {
    let mut lane = LaneRun {
        worker,
        audio_origin: None,
        watchdog: SilenceWatchdog::new(SILENCE_WARNING_AFTER),
        has_heard_sound: false,
    };
    if let Err(error) = lane.transcribe_until_closed(chunks) {
        tracing::warn!(speaker = lane.worker.speaker.label(), %error, "transcriber failed; ending this lane");
    }
    lane.send(LaneMessage::Ended(lane.worker.speaker));
}

struct LaneRun {
    worker: LaneWorker,
    audio_origin: Option<Duration>,
    watchdog: SilenceWatchdog,
    has_heard_sound: bool,
}

impl LaneRun {
    fn transcribe_until_closed(
        &mut self,
        chunks: &Receiver<AudioChunk>,
    ) -> Result<(), TranscribeError> {
        for chunk in chunks {
            self.audio_origin.get_or_insert(chunk.captured_at);
            self.watch_for_silence(&chunk);
            let events = self.worker.transcriber.push(&chunk)?;
            self.forward(events);
            self.send(LaneMessage::Watermark(self.worker.speaker, chunk.ends_at()));
        }
        let events = self.worker.transcriber.finish()?;
        self.forward(events);
        Ok(())
    }

    fn watch_for_silence(&mut self, chunk: &AudioChunk) {
        self.has_heard_sound |= !chunk.is_exact_silence();
        let verdict = self.watchdog.observe(chunk);
        if matches!(verdict, WatchdogVerdict::SilencedFor(_)) && !self.has_heard_sound {
            self.send(LaneMessage::Silenced(self.worker.speaker));
        }
    }

    fn forward(&self, events: Vec<TranscriptEvent>) {
        for event in events {
            self.send(self.message_for(event));
        }
    }

    fn message_for(&self, event: TranscriptEvent) -> LaneMessage {
        match event {
            TranscriptEvent::Partial(text) => LaneMessage::Partial(self.worker.speaker, text),
            TranscriptEvent::Final(utterance) => LaneMessage::Final(self.segment_from(utterance)),
        }
    }

    fn segment_from(&self, utterance: Utterance) -> Segment {
        let origin = self.audio_origin.unwrap_or_default();
        Segment {
            speaker: self.worker.speaker,
            start: origin + utterance.start,
            end: origin + utterance.end,
            text: utterance.text,
        }
    }

    fn send(&self, message: LaneMessage) {
        if self.worker.out.send(message).is_err() {
            tracing::warn!(
                speaker = self.worker.speaker.label(),
                "session writer is gone; dropping lane output"
            );
        }
    }
}
