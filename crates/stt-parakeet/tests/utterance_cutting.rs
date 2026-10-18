use std::time::Duration;

use scribe_core::recording::AudioChunk;
use stt_parakeet::{CutterConfig, PendingUtterance, UtteranceCutter, VAD_WINDOW_SAMPLES};

const WINDOW: Duration = Duration::from_millis(32);
const SPEECH_LEVEL: f32 = 0.25;
const LANE_FIRST_HEARD_AT: Duration = Duration::from_secs(7);
const FIFTEEN_SECONDS_OF_SAMPLES: usize = 240_000;

struct Lane {
    cutter: UtteranceCutter,
    next_window_at: Duration,
}

impl Lane {
    fn new() -> Lane {
        Lane {
            cutter: UtteranceCutter::new(CutterConfig::default()),
            next_window_at: LANE_FIRST_HEARD_AT,
        }
    }

    fn speech(&mut self, duration: Duration) -> Vec<PendingUtterance> {
        self.hear(duration, true)
    }

    fn silence(&mut self, duration: Duration) -> Vec<PendingUtterance> {
        self.hear(duration, false)
    }

    fn hear(&mut self, duration: Duration, is_speech: bool) -> Vec<PendingUtterance> {
        let windows = usize::try_from(duration.as_millis() / WINDOW.as_millis()).unwrap();
        let level = if is_speech { SPEECH_LEVEL } else { 0.0 };
        let chunk = AudioChunk {
            samples: vec![level; windows * VAD_WINDOW_SAMPLES],
            captured_at: self.next_window_at,
        };
        self.next_window_at += WINDOW * u32::try_from(windows).unwrap();
        self.cutter.feed(&chunk, &vec![is_speech; windows])
    }
}

fn millis(value: u64) -> Duration {
    Duration::from_millis(value)
}

#[test]
fn silence_alone_produces_no_utterance() {
    let mut lane = Lane::new();

    let heard = lane.silence(millis(5_024));

    assert!(heard.is_empty());
    assert_eq!(lane.cutter.flush(), None);
}

#[test]
fn a_sentence_followed_by_a_pause_becomes_one_utterance_timed_from_the_first_audio_heard() {
    let mut lane = Lane::new();
    lane.silence(millis(1_024));
    lane.speech(millis(2_048));

    let heard = lane.silence(millis(640));

    assert_eq!(heard.len(), 1);
    assert_eq!(heard[0].start, millis(824));
    assert_eq!(heard[0].end(), millis(3_272));
}

#[test]
fn a_sixteen_second_monologue_is_split_at_the_fifteen_second_limit() {
    let mut lane = Lane::new();

    let heard = lane.speech(millis(16_000));
    let remainder = lane.cutter.flush().unwrap();

    assert_eq!(heard.len(), 1);
    assert!(heard[0].samples.len() <= FIFTEEN_SECONDS_OF_SAMPLES);
    assert!(heard[0].samples.len() > FIFTEEN_SECONDS_OF_SAMPLES - VAD_WINDOW_SAMPLES);
    assert_eq!(remainder.start, heard[0].end());
}

#[test]
fn a_short_pause_inside_a_sentence_does_not_split_it() {
    let mut lane = Lane::new();
    lane.speech(millis(1_024));
    lane.silence(millis(320));
    lane.speech(millis(1_024));

    let heard = lane.silence(millis(640));

    assert_eq!(heard.len(), 1);
    assert_eq!(heard[0].start, Duration::ZERO);
}

#[test]
fn stopping_mid_sentence_hands_over_the_speech_in_progress() {
    let mut lane = Lane::new();
    lane.silence(millis(512));
    let heard_while_speaking = lane.speech(millis(1_024));

    let flushed = lane.cutter.flush().unwrap();

    assert!(heard_while_speaking.is_empty());
    assert_eq!(flushed.start, millis(312));
    assert_eq!(flushed.end(), millis(1_536));
    assert_eq!(lane.cutter.flush(), None);
}
