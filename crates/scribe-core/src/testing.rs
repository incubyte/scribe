use std::collections::{BTreeMap, VecDeque};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::{Arc, Mutex, PoisonError};
use std::time::Duration;

use chrono::{DateTime, FixedOffset};

use crate::ports::{
    AudioSource, CaptureError, RecordingListener, StoreError, TranscribeError, Transcriber,
    TranscriptStore,
};
use crate::recording::{
    AudioChunk, RecordingEvent, SAMPLE_RATE_HZ, Segment, SessionClock, SessionDocument,
    SessionHeader, SessionId, SessionStatus, SessionSummary, TranscriptEvent, Transcription,
    Utterance, offset_as_clock,
};

const CHUNK_SAMPLES: u32 = SAMPLE_RATE_HZ / 10;

#[must_use]
pub fn chunk_at(captured_at: Duration) -> AudioChunk {
    AudioChunk {
        samples: vec![0.0; CHUNK_SAMPLES as usize],
        captured_at,
    }
}

#[must_use]
pub fn partial(text: &str) -> TranscriptEvent {
    TranscriptEvent::Partial(text.to_owned())
}

#[must_use]
pub fn finished(text: &str, start: Duration, end: Duration) -> TranscriptEvent {
    TranscriptEvent::Final(Utterance {
        start,
        end,
        text: text.to_owned(),
    })
}

pub struct FakeAudioSource {
    chunks: Vec<AudioChunk>,
    failure: Option<CaptureError>,
    sender: Option<Sender<AudioChunk>>,
    started: Arc<AtomicBool>,
}

impl FakeAudioSource {
    #[must_use]
    pub fn with_chunks(chunks: Vec<AudioChunk>) -> FakeAudioSource {
        FakeAudioSource {
            chunks,
            failure: None,
            sender: None,
            started: Arc::default(),
        }
    }

    #[must_use]
    pub fn failing(error: CaptureError) -> FakeAudioSource {
        FakeAudioSource {
            failure: Some(error),
            ..FakeAudioSource::with_chunks(Vec::new())
        }
    }

    #[must_use]
    pub fn reporting_start_to(self, started: &Arc<AtomicBool>) -> FakeAudioSource {
        FakeAudioSource {
            started: Arc::clone(started),
            ..self
        }
    }
}

impl AudioSource for FakeAudioSource {
    fn start(&mut self, _clock: SessionClock) -> Result<Receiver<AudioChunk>, CaptureError> {
        if let Some(error) = self.failure.clone() {
            return Err(error);
        }
        self.started.store(true, Ordering::SeqCst);
        let (sender, receiver) = mpsc::channel();
        for chunk in self.chunks.drain(..) {
            let _ = sender.send(chunk);
        }
        self.sender = Some(sender);
        Ok(receiver)
    }

    fn stop(&mut self) {
        self.sender = None;
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScriptedReply {
    events: Vec<TranscriptEvent>,
    finalised_through: Option<Duration>,
}

#[must_use]
pub fn finalising(events: Vec<TranscriptEvent>) -> ScriptedReply {
    ScriptedReply {
        events,
        finalised_through: None,
    }
}

#[must_use]
pub fn finalising_through(events: Vec<TranscriptEvent>, through: Duration) -> ScriptedReply {
    ScriptedReply {
        events,
        finalised_through: Some(through),
    }
}

#[must_use]
pub fn still_speaking_since(start: Duration) -> ScriptedReply {
    finalising_through(Vec::new(), start)
}

pub struct ScriptedTranscriber {
    replies: VecDeque<Result<ScriptedReply, TranscribeError>>,
    origin: Option<Duration>,
    final_results_wait: Option<Duration>,
    pushes: usize,
    gate: Option<(usize, Receiver<()>)>,
    announcement: Option<(usize, Sender<()>)>,
}

impl ScriptedTranscriber {
    #[must_use]
    pub fn new(replies: Vec<Vec<TranscriptEvent>>) -> ScriptedTranscriber {
        ScriptedTranscriber::scripted(replies.into_iter().map(finalising).collect())
    }

    #[must_use]
    pub fn scripted(replies: Vec<ScriptedReply>) -> ScriptedTranscriber {
        ScriptedTranscriber {
            replies: replies.into_iter().map(Ok).collect(),
            origin: None,
            final_results_wait: None,
            pushes: 0,
            gate: None,
            announcement: None,
        }
    }

    #[must_use]
    pub fn failing(error: TranscribeError) -> ScriptedTranscriber {
        ScriptedTranscriber {
            replies: VecDeque::from([Err(error)]),
            origin: None,
            final_results_wait: None,
            pushes: 0,
            gate: None,
            announcement: None,
        }
    }

    #[must_use]
    pub fn pausing_before_push(self, push: usize, gate: Receiver<()>) -> ScriptedTranscriber {
        ScriptedTranscriber {
            gate: Some((push, gate)),
            ..self
        }
    }

    #[must_use]
    pub fn announcing_push(self, push: usize, announce: Sender<()>) -> ScriptedTranscriber {
        ScriptedTranscriber {
            announcement: Some((push, announce)),
            ..self
        }
    }

    fn mark_push(&mut self) {
        let push = self.pushes;
        self.pushes += 1;
        if let Some((_, announce)) = self.announcement.as_ref().filter(|(at, _)| *at == push) {
            let _ = announce.send(());
        }
        if let Some((_, gate)) = self.gate.as_ref().filter(|(at, _)| *at == push) {
            let _ = gate.recv();
        }
    }

    #[must_use]
    pub fn waiting_for_finals_at_most(self, wait: Duration) -> ScriptedTranscriber {
        ScriptedTranscriber {
            final_results_wait: Some(wait),
            ..self
        }
    }

    fn transcription(&self, reply: ScriptedReply, chunk: &AudioChunk) -> Transcription {
        let heard_through = chunk
            .ends_at()
            .saturating_sub(self.origin.unwrap_or_default());
        Transcription {
            events: reply.events,
            finalised_through: reply.finalised_through.unwrap_or(heard_through),
        }
    }
}

impl Transcriber for ScriptedTranscriber {
    fn push(&mut self, chunk: &AudioChunk) -> Result<Transcription, TranscribeError> {
        self.origin.get_or_insert(chunk.captured_at);
        self.mark_push();
        let reply = self
            .replies
            .pop_front()
            .unwrap_or_else(|| Ok(finalising(Vec::new())))?;
        Ok(self.transcription(reply, chunk))
    }

    fn finish(&mut self) -> Result<Vec<TranscriptEvent>, TranscribeError> {
        Ok(Vec::new())
    }

    fn final_results_wait(&self) -> Option<Duration> {
        self.final_results_wait
    }
}

#[derive(Debug, Clone)]
struct StoredSession {
    header: SessionHeader,
    segments: Vec<Segment>,
}

impl StoredSession {
    fn document(&self) -> SessionDocument {
        let lines: Vec<String> = self
            .segments
            .iter()
            .map(|segment| {
                format!(
                    "[{}] **{}:** {}\n",
                    offset_as_clock(segment.start),
                    segment.speaker.label(),
                    segment.text
                )
            })
            .collect();
        SessionDocument {
            header: self.header.clone(),
            segments: self.segments.clone(),
            raw: lines.concat(),
        }
    }
}

#[derive(Default)]
pub struct InMemoryTranscriptStore {
    sessions: Mutex<BTreeMap<SessionId, StoredSession>>,
}

impl InMemoryTranscriptStore {
    #[must_use]
    pub fn header(&self, id: &SessionId) -> Option<SessionHeader> {
        self.with_sessions(|sessions| sessions.get(id).map(|session| session.header.clone()))
    }

    #[must_use]
    pub fn segments(&self, id: &SessionId) -> Vec<Segment> {
        self.with_sessions(|sessions| {
            sessions
                .get(id)
                .map(|session| session.segments.clone())
                .unwrap_or_default()
        })
    }

    #[must_use]
    pub fn session_count(&self) -> usize {
        self.with_sessions(|sessions| sessions.len())
    }

    fn with_sessions<T>(
        &self,
        read: impl FnOnce(&mut BTreeMap<SessionId, StoredSession>) -> T,
    ) -> T {
        let mut sessions = self.sessions.lock().unwrap_or_else(PoisonError::into_inner);
        read(&mut sessions)
    }

    fn update(
        &self,
        id: &SessionId,
        change: impl FnOnce(&mut StoredSession),
    ) -> Result<(), StoreError> {
        self.with_sessions(|sessions| match sessions.get_mut(id) {
            Some(session) => {
                change(session);
                Ok(())
            }
            None => Err(StoreError::NotFound { id: id.clone() }),
        })
    }
}

fn free_id(sessions: &BTreeMap<SessionId, StoredSession>, wanted: &SessionId) -> Option<SessionId> {
    wanted
        .collision_candidates()
        .find(|candidate| !sessions.contains_key(candidate))
}

impl TranscriptStore for InMemoryTranscriptStore {
    fn create(&self, header: &SessionHeader) -> Result<SessionId, StoreError> {
        self.with_sessions(|sessions| {
            let id = free_id(sessions, &header.id).ok_or(StoreError::NoFreeSessionId)?;
            let session = StoredSession {
                header: header.clone().with_id(id.clone()),
                segments: Vec::new(),
            };
            sessions.insert(id.clone(), session);
            Ok(id)
        })
    }

    fn append(&self, id: &SessionId, segment: &Segment) -> Result<(), StoreError> {
        self.update(id, |session| session.segments.push(segment.clone()))
    }

    fn finalise(&self, id: &SessionId, ended_at: DateTime<FixedOffset>) -> Result<(), StoreError> {
        self.update(id, |session| {
            session.header = session.header.clone().finalised(ended_at);
        })
    }

    fn mark_abandoned_incomplete(&self) -> Result<usize, StoreError> {
        self.with_sessions(|sessions| {
            let abandoned = sessions
                .values_mut()
                .filter(|session| session.header.status == SessionStatus::Recording);
            Ok(abandoned
                .map(|session| session.header = session.header.clone().abandoned())
                .count())
        })
    }

    fn list(&self, limit: usize) -> Result<Vec<SessionSummary>, StoreError> {
        let mut summaries: Vec<SessionSummary> = self.with_sessions(|sessions| {
            sessions
                .values()
                .map(|session| session.header.summary())
                .collect()
        });
        summaries.sort_by_key(|summary| std::cmp::Reverse(summary.started_at));
        summaries.truncate(limit);
        Ok(summaries)
    }

    fn list_all(&self) -> Result<Vec<SessionSummary>, StoreError> {
        self.list(usize::MAX)
    }

    fn read(&self, id: &SessionId) -> Result<SessionDocument, StoreError> {
        self.with_sessions(|sessions| {
            let session = sessions
                .get(id)
                .ok_or_else(|| StoreError::NotFound { id: id.clone() })?;
            Ok(session.document())
        })
    }
}

#[derive(Default)]
pub struct RecordingListenerSpy {
    events: Mutex<Vec<RecordingEvent>>,
}

impl RecordingListenerSpy {
    #[must_use]
    pub fn events(&self) -> Vec<RecordingEvent> {
        self.events
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clone()
    }
}

impl RecordingListener for RecordingListenerSpy {
    fn on_event(&self, event: RecordingEvent) {
        self.events
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .push(event);
    }
}
