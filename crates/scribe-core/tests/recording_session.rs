use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc;
use std::thread;
use std::time::{Duration, Instant};

use chrono::DateTime;
use scribe_core::ports::{CaptureError, TranscribeError};
use scribe_core::recording::{
    AudioChunk, Lane, Readiness, Recorder, RecorderError, RecorderState, RecordingEvent, Segment,
    SessionHeader, SessionId, SessionStatus, Speaker, TranscriptEvent,
};
use scribe_core::testing::{
    FakeAudioSource, InMemoryTranscriptStore, RecordingListenerSpy, ScriptedReply,
    ScriptedTranscriber, chunk_at, finalising, finalising_through, finished, partial,
    still_speaking_since,
};

const READY: Readiness = Readiness {
    privacy_acknowledged: true,
    model_ready: true,
};

struct Call {
    store: Arc<InMemoryTranscriptStore>,
    listener: Arc<RecordingListenerSpy>,
    recorder: Recorder,
}

impl Call {
    fn with_readiness(readiness: Readiness) -> Call {
        let store = Arc::new(InMemoryTranscriptStore::default());
        let listener = Arc::new(RecordingListenerSpy::default());
        let mut recorder = Recorder::new(store.clone(), listener.clone());
        recorder.set_readiness(readiness);
        Call {
            store,
            listener,
            recorder,
        }
    }

    fn ready() -> Call {
        Call::with_readiness(READY)
    }

    fn start(&mut self, lanes: Vec<Lane>) -> Result<SessionId, RecorderError> {
        let started_at = DateTime::parse_from_rfc3339("2026-10-17T14:30:05+05:30").unwrap();
        let header = SessionHeader::recording(started_at, "speechanalyzer", "en-US");
        self.recorder.start(lanes, header)
    }

    fn record(&mut self, lanes: Vec<Lane>) -> SessionId {
        let id = self.start(lanes).unwrap();
        self.recorder.stop().unwrap();
        id
    }
}

fn lane(speaker: Speaker, transcriber: ScriptedTranscriber) -> Lane {
    let source = FakeAudioSource::with_chunks(vec![
        chunk_at(Duration::ZERO),
        chunk_at(Duration::from_millis(100)),
        chunk_at(Duration::from_millis(200)),
    ]);
    Lane {
        speaker,
        source: Box::new(source),
        transcriber: Box::new(transcriber),
    }
}

const CHUNK_MS: u64 = 100;
const PATIENCE: Duration = Duration::from_secs(2);

fn chunks_between(from_ms: u64, end_ms: u64, sample: f32) -> Vec<AudioChunk> {
    (from_ms / CHUNK_MS..end_ms / CHUNK_MS)
        .map(|index| AudioChunk {
            samples: vec![sample; 1_600],
            ..chunk_at(Duration::from_millis(index * CHUNK_MS))
        })
        .collect()
}

fn lane_lasting(speaker: Speaker, end_ms: u64, transcriber: ScriptedTranscriber) -> Lane {
    lane_hearing(speaker, chunks_between(0, end_ms, 0.0), transcriber)
}

fn lane_hearing(
    speaker: Speaker,
    chunks: Vec<AudioChunk>,
    transcriber: ScriptedTranscriber,
) -> Lane {
    Lane {
        speaker,
        source: Box::new(FakeAudioSource::with_chunks(chunks)),
        transcriber: Box::new(transcriber),
    }
}

fn stalled_lane(speaker: Speaker) -> Lane {
    lane_hearing(speaker, Vec::new(), ScriptedTranscriber::new(vec![]))
}

fn wait_until(condition: impl Fn() -> bool) {
    let deadline = Instant::now() + PATIENCE;
    while !condition() && Instant::now() < deadline {
        thread::sleep(Duration::from_millis(10));
    }
}

fn silence_warnings(events: Vec<RecordingEvent>) -> Vec<RecordingEvent> {
    events
        .into_iter()
        .filter(|event| matches!(event, RecordingEvent::CaptureSilenced { .. }))
        .collect()
}

fn silent_mic_lane() -> Lane {
    lane(Speaker::Me, ScriptedTranscriber::new(vec![]))
}

fn denied_lane(speaker: Speaker) -> Lane {
    Lane {
        speaker,
        source: Box::new(FakeAudioSource::failing(CaptureError::PermissionDenied)),
        transcriber: Box::new(ScriptedTranscriber::new(vec![])),
    }
}

fn line(speaker: Speaker, (from_ms, to_ms): (u64, u64), text: &str) -> Segment {
    Segment {
        speaker,
        start: Duration::from_millis(from_ms),
        end: Duration::from_millis(to_ms),
        text: text.to_owned(),
    }
}

fn utterance(from_ms: u64, to_ms: u64, text: &str) -> Vec<TranscriptEvent> {
    vec![finished(
        text,
        Duration::from_millis(from_ms),
        Duration::from_millis(to_ms),
    )]
}

#[test]
fn starting_creates_a_session_marked_as_recording_and_announces_it() {
    let mut call = Call::ready();

    let id = call.start(vec![silent_mic_lane()]).unwrap();

    assert_eq!(
        call.store.header(&id).unwrap().status,
        SessionStatus::Recording
    );
    assert_eq!(call.recorder.state(), RecorderState::Recording);
    assert_eq!(
        call.listener.events(),
        vec![RecordingEvent::RecordingStarted { id }]
    );
    call.recorder.stop().unwrap();
}

#[test]
fn finished_utterances_are_saved_as_my_lines_in_the_order_spoken() {
    let mut call = Call::ready();
    let transcriber = ScriptedTranscriber::new(vec![
        vec![partial("Hello")],
        utterance(0, 1_000, "Hello there"),
        utterance(1_200, 2_000, "How are you"),
    ]);

    let id = call.record(vec![lane(Speaker::Me, transcriber)]);

    assert_eq!(
        call.store.segments(&id),
        vec![
            line(Speaker::Me, (0, 1_000), "Hello there"),
            line(Speaker::Me, (1_200, 2_000), "How are you"),
        ]
    );
}

#[test]
fn in_progress_wording_is_shown_live_but_never_saved() {
    let mut call = Call::ready();
    let transcriber = ScriptedTranscriber::new(vec![vec![partial("Hel")]]);

    let id = call.record(vec![lane(Speaker::Me, transcriber)]);

    assert!(call.store.segments(&id).is_empty());
    assert!(
        call.listener
            .events()
            .contains(&RecordingEvent::PartialSegmentUpdated {
                speaker: Speaker::Me,
                text: "Hel".to_owned(),
            })
    );
}

#[test]
fn stopping_completes_the_session_and_returns_the_recorder_to_idle() {
    let mut call = Call::ready();

    let id = call.record(vec![silent_mic_lane()]);

    let header = call.store.header(&id).unwrap();
    assert_eq!(header.status, SessionStatus::Complete);
    assert!(header.ended_at.unwrap() >= header.started_at);
    assert_eq!(call.recorder.state(), RecorderState::Idle);
    assert_eq!(
        call.listener.events().last(),
        Some(&RecordingEvent::RecordingStopped { id })
    );
}

#[test]
fn starting_is_refused_until_privacy_is_acknowledged_and_the_model_is_ready() {
    let mut call = Call::with_readiness(Readiness::default());

    let outcome = call.start(vec![silent_mic_lane()]);

    assert!(matches!(outcome, Err(RecorderError::NotReady)));
    assert_eq!(call.store.session_count(), 0);
    assert_eq!(call.recorder.state(), RecorderState::Locked);
}

#[test]
fn a_second_recording_cannot_start_while_one_is_in_progress() {
    let mut call = Call::ready();
    call.start(vec![silent_mic_lane()]).unwrap();

    let outcome = call.start(vec![silent_mic_lane()]);

    assert!(matches!(outcome, Err(RecorderError::AlreadyRecording)));
    assert_eq!(call.store.session_count(), 1);
    call.recorder.stop().unwrap();
}

#[test]
fn a_denied_microphone_leaves_no_session_behind() {
    let mut call = Call::ready();

    let outcome = call.start(vec![denied_lane(Speaker::Me)]);

    assert!(matches!(
        outcome,
        Err(RecorderError::Capture(CaptureError::PermissionDenied))
    ));
    assert_eq!(call.store.session_count(), 0);
    assert_eq!(call.recorder.state(), RecorderState::Idle);
}

#[test]
fn a_denied_microphone_announces_nothing() {
    let mut call = Call::ready();

    let _ = call.start(vec![denied_lane(Speaker::Me)]);

    assert!(call.listener.events().is_empty());
}

#[test]
fn a_failing_transcriber_ends_only_its_own_lane() {
    let mut call = Call::ready();
    let broken = ScriptedTranscriber::failing(TranscribeError::Engine("crashed".to_owned()));
    let working = ScriptedTranscriber::new(vec![utterance(0, 1_000, "Can you hear me")]);

    let id = call.record(vec![
        lane(Speaker::Me, broken),
        lane(Speaker::Them, working),
    ]);

    assert_eq!(
        call.store.segments(&id),
        vec![line(Speaker::Them, (0, 1_000), "Can you hear me")]
    );
    assert_eq!(
        call.store.header(&id).unwrap().status,
        SessionStatus::Complete
    );
}

#[test]
fn stopping_when_nothing_is_recording_changes_nothing() {
    let mut call = Call::ready();

    call.recorder.stop().unwrap();

    assert_eq!(call.recorder.state(), RecorderState::Idle);
    assert!(call.listener.events().is_empty());
}

#[test]
fn my_lines_and_their_lines_are_saved_in_the_order_they_started() {
    let mut call = Call::ready();
    let mine = ScriptedTranscriber::new(vec![
        utterance(0, 1_000, "Hi there"),
        utterance(2_000, 2_500, "Good, thanks"),
    ]);
    let theirs = ScriptedTranscriber::new(vec![utterance(1_200, 1_800, "How are you")]);

    let id = call.record(vec![
        lane_lasting(Speaker::Me, 3_000, mine),
        lane_lasting(Speaker::Them, 3_000, theirs),
    ]);

    assert_eq!(
        call.store.segments(&id),
        vec![
            line(Speaker::Me, (0, 1_000), "Hi there"),
            line(Speaker::Them, (1_200, 1_800), "How are you"),
            line(Speaker::Me, (2_000, 2_500), "Good, thanks"),
        ]
    );
}

fn lane_finalising_after(
    speaker: Speaker,
    silent_pushes: usize,
    events: Vec<TranscriptEvent>,
) -> Lane {
    let mut replies = vec![Vec::new(); silent_pushes];
    replies.push(events);
    lane_lasting(speaker, 3_500, ScriptedTranscriber::new(replies))
}

#[test]
fn lines_finalised_only_after_each_pause_are_saved_in_the_order_spoken_when_speakers_take_turns() {
    let mut call = Call::ready();
    let id = call
        .start(vec![
            lane_finalising_after(Speaker::Me, 16, utterance(0, 1_000, "Hi there")),
            lane_finalising_after(Speaker::Them, 30, utterance(1_800, 2_400, "How are you")),
        ])
        .unwrap();

    wait_until(|| call.store.segments(&id).len() == 2);
    let saved_while_recording = call.store.segments(&id);
    call.recorder.stop().unwrap();

    assert_eq!(
        saved_while_recording,
        vec![
            line(Speaker::Me, (0, 1_000), "Hi there"),
            line(Speaker::Them, (1_800, 2_400), "How are you"),
        ]
    );
}

#[test]
fn a_stalled_side_does_not_hold_back_the_other_sides_lines_beyond_the_maximum_hold() {
    let mut call = Call::ready();
    let theirs = ScriptedTranscriber::new(vec![utterance(500, 1_000, "Can you hear me")]);
    let id = call
        .start(vec![
            stalled_lane(Speaker::Me),
            lane_lasting(Speaker::Them, 2_600, theirs),
        ])
        .unwrap();

    wait_until(|| !call.store.segments(&id).is_empty());
    let saved_while_recording = call.store.segments(&id);
    call.recorder.stop().unwrap();

    assert_eq!(
        saved_while_recording,
        vec![line(Speaker::Them, (500, 1_000), "Can you hear me")]
    );
}

#[test]
fn the_other_side_delivering_only_silence_raises_one_warning_while_my_side_keeps_recording() {
    let mut call = Call::ready();
    let mine = ScriptedTranscriber::new(vec![utterance(0, 900, "Hello? Anyone?")]);

    let id = call.record(vec![
        lane_lasting(Speaker::Me, 300, mine),
        lane_lasting(Speaker::Them, 8_000, ScriptedTranscriber::new(vec![])),
    ]);

    assert_eq!(
        silence_warnings(call.listener.events()),
        vec![RecordingEvent::CaptureSilenced {
            speaker: Speaker::Them
        }]
    );
    assert_eq!(
        call.store.segments(&id),
        vec![line(Speaker::Me, (0, 900), "Hello? Anyone?")]
    );
}

#[test]
fn the_other_side_falling_quiet_after_being_heard_raises_no_warning() {
    let mut call = Call::ready();
    let heard_then_quiet = [
        chunks_between(0, 1_000, 0.2),
        chunks_between(1_000, 8_000, 0.0),
    ]
    .concat();

    call.record(vec![lane_hearing(
        Speaker::Them,
        heard_then_quiet,
        ScriptedTranscriber::new(vec![]),
    )]);

    assert!(silence_warnings(call.listener.events()).is_empty());
}

#[test]
fn when_the_other_side_cannot_be_captured_my_side_still_records_and_says_so() {
    let mut call = Call::ready();
    let mine = ScriptedTranscriber::new(vec![utterance(0, 900, "Hello? Anyone?")]);

    let id = call.record(vec![lane(Speaker::Me, mine), denied_lane(Speaker::Them)]);

    assert_eq!(
        call.store.segments(&id),
        vec![line(Speaker::Me, (0, 900), "Hello? Anyone?")]
    );
    assert!(
        call.listener
            .events()
            .contains(&RecordingEvent::CaptureUnavailable {
                speaker: Speaker::Them
            })
    );
    assert_eq!(
        call.store.header(&id).unwrap().status,
        SessionStatus::Complete
    );
}

#[test]
fn the_other_side_is_never_captured_when_my_microphone_is_denied() {
    let mut call = Call::ready();
    let other_side_started = Arc::new(AtomicBool::new(false));
    let other_side = Lane {
        speaker: Speaker::Them,
        source: Box::new(
            FakeAudioSource::with_chunks(Vec::new()).reporting_start_to(&other_side_started),
        ),
        transcriber: Box::new(ScriptedTranscriber::new(vec![])),
    };

    let outcome = call.start(vec![other_side, denied_lane(Speaker::Me)]);

    assert!(matches!(
        outcome,
        Err(RecorderError::Capture(CaptureError::PermissionDenied))
    ));
    assert!(!other_side_started.load(Ordering::SeqCst));
}

fn speaking_until(silent_pushes: usize, events: Vec<TranscriptEvent>) -> ScriptedTranscriber {
    let mut replies = vec![Vec::new(); silent_pushes];
    replies.push(events);
    ScriptedTranscriber::new(replies)
}

fn still_speaking_until(
    pushes: usize,
    since_ms: u64,
    events: Vec<TranscriptEvent>,
) -> ScriptedTranscriber {
    let mut replies = vec![still_speaking_since(Duration::from_millis(since_ms)); pushes];
    replies.push(finalising(events));
    ScriptedTranscriber::scripted(replies)
}

#[test]
fn their_earlier_line_is_saved_before_my_later_line_even_when_its_final_arrives_late() {
    let mut call = Call::ready();
    let mine = speaking_until(25, utterance(2_000, 2_500, "Yeah"));
    let theirs = still_speaking_until(56, 0, utterance(0, 5_000, "So the plan is simple"));

    let id = call.record(vec![
        lane_lasting(Speaker::Me, 3_900, mine),
        lane_lasting(Speaker::Them, 6_000, theirs),
    ]);

    assert_eq!(
        call.store.segments(&id),
        vec![
            line(Speaker::Them, (0, 5_000), "So the plan is simple"),
            line(Speaker::Me, (2_000, 2_500), "Yeah"),
        ]
    );
}

#[test]
fn an_echo_is_dropped_even_when_their_line_is_finalised_after_mine() {
    let mut call = Call::ready();
    let mine = speaking_until(21, utterance(1_100, 2_100, "let's review the budget"));
    let theirs = still_speaking_until(
        35,
        1_000,
        utterance(1_000, 2_000, "Let's review the budget"),
    );

    let id = call.record(vec![
        lane_lasting(Speaker::Me, 3_500, mine),
        lane_lasting(Speaker::Them, 4_000, theirs),
    ]);

    assert_eq!(
        call.store.segments(&id),
        vec![line(
            Speaker::Them,
            (1_000, 2_000),
            "Let's review the budget"
        )]
    );
}

#[test]
fn the_end_of_an_earlier_session_leaves_the_current_recording_running() {
    let mut call = Call::ready();
    let earlier = call.record(vec![silent_mic_lane()]);
    let current = call.start(vec![silent_mic_lane()]).unwrap();

    call.recorder.stop_session(&earlier).unwrap();

    assert_eq!(call.recorder.state(), RecorderState::Recording);
    call.recorder.stop().unwrap();
    assert_eq!(
        call.store.header(&current).unwrap().status,
        SessionStatus::Complete
    );
}

#[test]
fn the_current_session_can_be_stopped_by_its_id() {
    let mut call = Call::ready();
    let id = call.start(vec![silent_mic_lane()]).unwrap();

    call.recorder.stop_session(&id).unwrap();

    assert_eq!(call.recorder.state(), RecorderState::Idle);
    assert_eq!(
        call.store.header(&id).unwrap().status,
        SessionStatus::Complete
    );
}

const FINAL_RESULTS_WAIT: Duration = Duration::from_secs(6);

fn replies(parts: &[(usize, ScriptedReply)]) -> Vec<ScriptedReply> {
    parts
        .iter()
        .flat_map(|(count, reply)| vec![reply.clone(); *count])
        .collect()
}

fn silent_after_a_late_final() -> ScriptedTranscriber {
    ScriptedTranscriber::scripted(replies(&[
        (15, still_speaking_since(Duration::ZERO)),
        (
            1,
            finalising_through(utterance(0, 1_000, "Hi there"), Duration::from_secs(1)),
        ),
        (80, still_speaking_since(Duration::from_secs(1))),
    ]))
    .waiting_for_finals_at_most(FINAL_RESULTS_WAIT)
}

#[test]
fn their_line_is_saved_while_recording_once_my_silent_side_has_been_waited_for_long_enough() {
    let mut call = Call::ready();
    let theirs = speaking_until(30, utterance(2_000, 2_500, "How are you"));
    let id = call
        .start(vec![
            lane_lasting(Speaker::Me, 9_000, silent_after_a_late_final()),
            lane_lasting(Speaker::Them, 3_900, theirs),
        ])
        .unwrap();

    wait_until(|| call.store.segments(&id).len() == 2);
    let saved_while_recording = call.store.segments(&id);
    call.recorder.stop().unwrap();

    assert_eq!(
        saved_while_recording,
        vec![
            line(Speaker::Me, (0, 1_000), "Hi there"),
            line(Speaker::Them, (2_000, 2_500), "How are you"),
        ]
    );
}

fn their_late_echoed_final_then_silence() -> ScriptedTranscriber {
    ScriptedTranscriber::scripted(replies(&[
        (35, still_speaking_since(Duration::from_secs(1))),
        (
            1,
            finalising_through(
                utterance(1_000, 2_000, "Let's review the budget"),
                Duration::from_secs(2),
            ),
        ),
        (70, still_speaking_since(Duration::from_secs(2))),
    ]))
    .waiting_for_finals_at_most(FINAL_RESULTS_WAIT)
}

#[test]
fn an_echo_is_dropped_when_their_side_finalises_late_and_then_falls_silent() {
    let mut call = Call::ready();
    let mine = speaking_until(21, utterance(1_100, 2_100, "let's review the budget"));

    let id = call.record(vec![
        lane_lasting(Speaker::Me, 3_500, mine),
        lane_lasting(
            Speaker::Them,
            10_000,
            their_late_echoed_final_then_silence(),
        ),
    ]);

    assert_eq!(
        call.store.segments(&id),
        vec![line(
            Speaker::Them,
            (1_000, 2_000),
            "Let's review the budget"
        )]
    );
}

const BRIEFLY: Duration = Duration::from_millis(300);

fn saved_after_a_moment(call: &Call, id: &SessionId) -> Vec<Segment> {
    let deadline = Instant::now() + BRIEFLY;
    while call.store.segments(id).is_empty() && Instant::now() < deadline {
        thread::sleep(Duration::from_millis(10));
    }
    call.store.segments(id)
}

struct Gate {
    open: mpsc::Sender<()>,
    closed: mpsc::Receiver<()>,
}

fn gate() -> Gate {
    let (open, closed) = mpsc::channel();
    Gate { open, closed }
}

fn my_side_pausing_mid_transcription(is_waiting: &Gate, gate: Gate) -> Lane {
    let mine = speaking_until(15, utterance(1_000, 1_500, "Hello"))
        .announcing_push(15, is_waiting.open.clone())
        .pausing_before_push(15, gate.closed);
    lane_lasting(Speaker::Me, 4_000, mine)
}

fn their_side_running_ahead(gate: Gate, is_done: &Gate) -> Lane {
    let theirs = speaking_until(25, utterance(2_000, 2_500, "How are you"))
        .pausing_before_push(0, gate.closed)
        .announcing_push(59, is_done.open.clone());
    lane_lasting(Speaker::Them, 6_000, theirs)
}

struct SlowTranscription {
    my_side_is_waiting: Gate,
    open_my_gate: mpsc::Sender<()>,
    open_their_gate: mpsc::Sender<()>,
    their_side_is_done: Gate,
}

impl SlowTranscription {
    fn let_their_side_run_ahead(&self) {
        self.my_side_is_waiting
            .closed
            .recv_timeout(PATIENCE)
            .unwrap();
        self.open_their_gate.send(()).unwrap();
        self.their_side_is_done
            .closed
            .recv_timeout(PATIENCE)
            .unwrap();
    }

    fn let_my_side_finish(&self) {
        self.open_my_gate.send(()).unwrap();
    }
}

fn start_with_my_side_paused_mid_transcription(call: &mut Call) -> (SessionId, SlowTranscription) {
    let (my_gate, their_gate) = (gate(), gate());
    let slow = SlowTranscription {
        my_side_is_waiting: gate(),
        open_my_gate: my_gate.open.clone(),
        open_their_gate: their_gate.open.clone(),
        their_side_is_done: gate(),
    };
    let lanes = vec![
        my_side_pausing_mid_transcription(&slow.my_side_is_waiting, my_gate),
        their_side_running_ahead(their_gate, &slow.their_side_is_done),
    ];
    (call.start(lanes).unwrap(), slow)
}

#[test]
fn a_side_waiting_on_a_slow_transcription_is_not_mistaken_for_a_stalled_one() {
    let mut call = Call::ready();
    let (id, slow) = start_with_my_side_paused_mid_transcription(&mut call);

    slow.let_their_side_run_ahead();
    let saved_while_my_side_transcribes = saved_after_a_moment(&call, &id);
    slow.let_my_side_finish();
    call.recorder.stop().unwrap();

    assert!(saved_while_my_side_transcribes.is_empty());
    assert_eq!(
        call.store.segments(&id),
        vec![
            line(Speaker::Me, (1_000, 1_500), "Hello"),
            line(Speaker::Them, (2_000, 2_500), "How are you"),
        ]
    );
}
