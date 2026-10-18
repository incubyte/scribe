use std::env;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use scribe_core::ports::Transcriber;
use scribe_core::recording::{AudioChunk, TranscriptEvent};
use stt_parakeet::{ParakeetTranscriber, default_models_dir};

const MODELS_DIR_OVERRIDE: &str = "SCRIBE_PARAKEET_MODELS_DIR";
const CHUNK_SAMPLES: usize = 1_600;
const CHUNK_DURATION: Duration = Duration::from_millis(100);
const TRAILING_SILENCE_SAMPLES: usize = 16_000;
const DECODE_BUDGET: Duration = Duration::from_millis(1_400);
const I16_FULL_SCALE: f32 = 32_768.0;

struct Heard {
    text: String,
    slowest_finalising_push: Duration,
}

fn models_dir() -> PathBuf {
    env::var_os(MODELS_DIR_OVERRIDE).map_or_else(|| default_models_dir().unwrap(), PathBuf::from)
}

fn fixture(name: &str) -> Vec<f32> {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name);
    let mut samples: Vec<f32> = hound::WavReader::open(path)
        .unwrap()
        .samples::<i16>()
        .map(|sample| f32::from(sample.unwrap()) / I16_FULL_SCALE)
        .collect();
    samples.resize(samples.len() + TRAILING_SILENCE_SAMPLES, 0.0);
    samples
}

fn transcribe(samples: &[f32]) -> Heard {
    let mut transcriber = loaded_transcriber();
    let mut heard = Heard {
        text: String::new(),
        slowest_finalising_push: Duration::ZERO,
    };
    let mut captured_at = Duration::ZERO;
    for samples in samples.chunks(CHUNK_SAMPLES) {
        let chunk = AudioChunk {
            samples: samples.to_vec(),
            captured_at,
        };
        let pushing = Instant::now();
        let events = transcriber.push(&chunk).unwrap();
        record(&mut heard, &events, pushing.elapsed());
        captured_at += CHUNK_DURATION;
    }
    let events = transcriber.finish().unwrap();
    record(&mut heard, &events, Duration::ZERO);
    heard
}

fn loaded_transcriber() -> ParakeetTranscriber {
    let loading = Instant::now();
    let transcriber = ParakeetTranscriber::load(&models_dir()).unwrap();
    eprintln!("model loaded in {:?}", loading.elapsed());
    transcriber
}

fn record(heard: &mut Heard, events: &[TranscriptEvent], push_time: Duration) {
    for event in events {
        let TranscriptEvent::Final(utterance) = event else {
            panic!("Parakeet must only finalise whole utterances, got {event:?}");
        };
        eprintln!(
            "[{:?} - {:?}] decoded in {push_time:?}: {}",
            utterance.start, utterance.end, utterance.text
        );
        heard.text.push_str(&utterance.text);
        heard.slowest_finalising_push = heard.slowest_finalising_push.max(push_time);
    }
}

#[test]
#[ignore = "needs the downloaded Parakeet model; run manually in release mode"]
fn english_speech_is_transcribed_in_english_within_the_latency_budget() {
    let heard = transcribe(&fixture("english.wav"));

    assert!(
        heard.text.to_lowercase().contains("weather"),
        "{}",
        heard.text
    );
    assert!(heard.slowest_finalising_push < DECODE_BUDGET);
}

#[test]
#[ignore = "needs the downloaded Parakeet model; run manually in release mode"]
fn german_speech_is_transcribed_in_german_without_choosing_a_language() {
    let heard = transcribe(&fixture("german.wav"));

    assert!(
        heard.text.to_lowercase().contains("wetter"),
        "{}",
        heard.text
    );
    assert!(heard.slowest_finalising_push < DECODE_BUDGET);
}
