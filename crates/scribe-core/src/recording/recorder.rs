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
        let clock = SessionClock::start();
        let mut started = start_sources(lanes, clock)?;
        let id = match self.store.create(&header) {
            Ok(id) => id,
            Err(error) => {
                stop_sources(&mut started);
                return Err(error.into());
            }
        };
        let context = SessionContext {
            id: id.clone(),
            header,
            clock,
        };
        self.listener
            .on_event(RecordingEvent::RecordingStarted { id: id.clone() });
        self.active = Some(self.spawn_session(started, context));
        Ok(id)
    }

    pub fn stop(&mut self) -> Result<(), RecorderError> {
        match self.active.take() {
            Some(session) => session.stop(),
            None => Ok(()),
        }
    }

    pub fn ensure_idle(&self) -> Result<(), RecorderError> {
        match self.state() {
            RecorderState::Idle => Ok(()),
            RecorderState::Locked => Err(RecorderError::NotReady),
            RecorderState::Recording => Err(RecorderError::AlreadyRecording),
        }
    }

    fn spawn_session(&self, lanes: Vec<StartedLane>, context: SessionContext) -> ActiveSession {
        let (out, messages) = mpsc::channel();
        let writer = self.session_writer(&lanes, context);
        let writer = thread::spawn(move || run_writer(writer, &messages));
        let (sources, workers) = spawn_lane_workers(lanes, &out);
        ActiveSession {
            sources,
            workers,
            writer,
        }
    }

    fn session_writer(&self, lanes: &[StartedLane], context: SessionContext) -> SessionWriter {
        let speakers: Vec<Speaker> = lanes.iter().map(|lane| lane.speaker).collect();
        SessionWriter {
            store: Arc::clone(&self.store),
            listener: Arc::clone(&self.listener),
            id: context.id,
            started_at: context.header.started_at,
            clock: context.clock,
            sequencer: SegmentSequencer::new(&speakers),
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

fn start_sources(lanes: Vec<Lane>, clock: SessionClock) -> Result<Vec<StartedLane>, CaptureError> {
    let mut started = Vec::with_capacity(lanes.len());
    for lane in lanes {
        match lane.start(clock) {
            Ok(lane) => started.push(lane),
            Err(error) => {
                stop_sources(&mut started);
                return Err(error);
            }
        }
    }
    Ok(started)
}

fn stop_sources(lanes: &mut [StartedLane]) {
    for lane in lanes {
        lane.source.stop();
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
