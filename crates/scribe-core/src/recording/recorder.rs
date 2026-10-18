use std::sync::Arc;
use std::sync::mpsc::{self, Receiver, Sender};
use std::thread::{self, JoinHandle};

use crate::ports::{
    AudioSource, CaptureError, RecordingListener, StoreError, Transcriber, TranscriptStore,
};
use crate::recording::lane_worker::{LaneMessage, LaneWorker, run_lane};
use crate::recording::writer::{SessionWriter, run_writer};
use crate::recording::{
    AudioChunk, Readiness, RecorderState, RecordingEvent, SegmentSequencer, SessionClock,
    SessionHeader, SessionId, Speaker, recorder_state,
};

pub struct Lane {
    pub speaker: Speaker,
    pub source: Box<dyn AudioSource>,
    pub transcriber: Box<dyn Transcriber>,
}

#[derive(Debug, thiserror::Error)]
pub enum RecorderError {
    #[error("recording is locked until privacy is acknowledged and the speech model is ready")]
    NotReady,
    #[error("a recording is already in progress")]
    AlreadyRecording,
    #[error(transparent)]
    Capture(#[from] CaptureError),
    #[error(transparent)]
    Store(#[from] StoreError),
    #[error("a recording worker stopped unexpectedly")]
    WorkerPanicked,
}

pub struct Recorder {
    store: Arc<dyn TranscriptStore>,
    listener: Arc<dyn RecordingListener>,
    readiness: Readiness,
    active: Option<ActiveSession>,
}

impl Recorder {
    #[must_use]
    pub fn new(store: Arc<dyn TranscriptStore>, listener: Arc<dyn RecordingListener>) -> Recorder {
        Recorder {
            store,
            listener,
            readiness: Readiness::default(),
            active: None,
        }
    }

    pub fn set_readiness(&mut self, readiness: Readiness) {
        self.readiness = readiness;
    }

    #[must_use]
    pub fn readiness(&self) -> Readiness {
        self.readiness
    }

    #[must_use]
    pub fn state(&self) -> RecorderState {
        recorder_state(&self.readiness, self.is_recording())
    }

    #[must_use]
    pub fn is_recording(&self) -> bool {
        self.active.is_some()
    }

    pub fn start(
        &mut self,
        lanes: Vec<Lane>,
        header: SessionHeader,
    ) -> Result<SessionId, RecorderError> {
        self.ensure_idle()?;
        self.record(CapturingLanes::start(lanes)?, header)
    }

    pub fn record(
        &mut self,
        mut capturing: CapturingLanes,
        header: SessionHeader,
    ) -> Result<SessionId, RecorderError> {
        self.ensure_idle()?;
        let id = self.store.create(&header)?;
        let context = SessionContext {
            id: id.clone(),
            header,
            clock: capturing.clock,
        };
        self.announce_start(&id, &capturing.unavailable);
        self.active = Some(self.spawn_session(capturing.take_lanes(), context));
        Ok(id)
    }

    pub fn stop(&mut self) -> Result<(), RecorderError> {
        match self.active.take() {
            Some(session) => session.stop(),
            None => Ok(()),
        }
    }

    pub fn stop_session(&mut self, id: &SessionId) -> Result<(), RecorderError> {
        if self
            .active
            .as_ref()
            .is_some_and(|session| session.id == *id)
        {
            return self.stop();
        }
        Ok(())
    }

    pub fn ensure_idle(&self) -> Result<(), RecorderError> {
        match self.state() {
            RecorderState::Idle => Ok(()),
            RecorderState::Locked => Err(RecorderError::NotReady),
            RecorderState::Recording => Err(RecorderError::AlreadyRecording),
        }
    }

    fn announce_start(&self, id: &SessionId, unavailable: &[Speaker]) {
        self.listener
            .on_event(RecordingEvent::RecordingStarted { id: id.clone() });
        for speaker in unavailable {
            self.listener
                .on_event(RecordingEvent::CaptureUnavailable { speaker: *speaker });
        }
    }

    fn spawn_session(&self, lanes: Vec<StartedLane>, context: SessionContext) -> ActiveSession {
        let (out, messages) = mpsc::channel();
        let id = context.id.clone();
        let writer = self.session_writer(&lanes, context);
        let writer = thread::spawn(move || run_writer(writer, &messages));
        let (sources, workers) = spawn_lane_workers(lanes, &out);
        ActiveSession {
            id,
            sources,
            workers,
            writer,
        }
    }

    fn session_writer(&self, lanes: &[StartedLane], context: SessionContext) -> SessionWriter {
        let speakers: Vec<Speaker> = lanes.iter().map(|lane| lane.speaker).collect();
        let mut sequencer = SegmentSequencer::new(&speakers);
        for lane in lanes {
            if let Some(wait) = lane.transcriber.final_results_wait() {
                sequencer.limit_wait_for_finals(lane.speaker, wait);
            }
        }
        SessionWriter {
            store: Arc::clone(&self.store),
            listener: Arc::clone(&self.listener),
            id: context.id,
            started_at: context.header.started_at,
            clock: context.clock,
            sequencer,
            open_lanes: speakers.len(),
        }
    }
}

struct SessionContext {
    id: SessionId,
    header: SessionHeader,
    clock: SessionClock,
}

struct StartedLane {
    speaker: Speaker,
    source: Box<dyn AudioSource>,
    transcriber: Box<dyn Transcriber>,
    chunks: Receiver<AudioChunk>,
}

impl Lane {
    fn is_required(&self) -> bool {
        self.speaker == Speaker::Me
    }

    fn start(mut self, clock: SessionClock) -> Result<StartedLane, CaptureError> {
        let chunks = self.source.start(clock)?;
        Ok(StartedLane {
            speaker: self.speaker,
            source: self.source,
            transcriber: self.transcriber,
            chunks,
        })
    }
}

pub struct CapturingLanes {
    clock: SessionClock,
    lanes: Vec<StartedLane>,
    unavailable: Vec<Speaker>,
}

impl CapturingLanes {
    pub fn start(mut lanes: Vec<Lane>) -> Result<CapturingLanes, CaptureError> {
        lanes.sort_by_key(|lane| !lane.is_required());
        let mut capturing = CapturingLanes {
            clock: SessionClock::start(),
            lanes: Vec::with_capacity(lanes.len()),
            unavailable: Vec::new(),
        };
        for lane in lanes {
            capturing.start_lane(lane)?;
        }
        Ok(capturing)
    }

    fn start_lane(&mut self, lane: Lane) -> Result<(), CaptureError> {
        let (speaker, is_required) = (lane.speaker, lane.is_required());
        match lane.start(self.clock) {
            Ok(started) => self.lanes.push(started),
            Err(error) if is_required => return Err(error),
            Err(error) => {
                tracing::warn!(speaker = speaker.label(), %error, "recording without this side");
                self.unavailable.push(speaker);
            }
        }
        Ok(())
    }

    fn take_lanes(&mut self) -> Vec<StartedLane> {
        std::mem::take(&mut self.lanes)
    }
}

impl Drop for CapturingLanes {
    fn drop(&mut self) {
        for lane in &mut self.lanes {
            lane.source.stop();
        }
    }
}

fn spawn_lane_workers(
    lanes: Vec<StartedLane>,
    out: &Sender<LaneMessage>,
) -> (Vec<Box<dyn AudioSource>>, Vec<JoinHandle<()>>) {
    lanes
        .into_iter()
        .map(|lane| {
            let worker = LaneWorker {
                speaker: lane.speaker,
                transcriber: lane.transcriber,
                out: out.clone(),
            };
            let chunks = lane.chunks;
            (
                lane.source,
                thread::spawn(move || run_lane(worker, &chunks)),
            )
        })
        .unzip()
}

struct ActiveSession {
    id: SessionId,
    sources: Vec<Box<dyn AudioSource>>,
    workers: Vec<JoinHandle<()>>,
    writer: JoinHandle<Result<(), StoreError>>,
}

impl ActiveSession {
    fn stop(mut self) -> Result<(), RecorderError> {
        for source in &mut self.sources {
            source.stop();
        }
        let panicked_workers = self
            .workers
            .into_iter()
            .map(JoinHandle::join)
            .filter(Result::is_err)
            .count();
        self.writer
            .join()
            .map_err(|_| RecorderError::WorkerPanicked)??;
        match panicked_workers {
            0 => Ok(()),
            _ => Err(RecorderError::WorkerPanicked),
        }
    }
}
