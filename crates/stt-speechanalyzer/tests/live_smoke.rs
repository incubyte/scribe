#![cfg(target_os = "macos")]

use std::time::Duration;

use scribe_core::ports::Transcriber;
use scribe_core::recording::{AudioChunk, TranscriptEvent};
use stt_speechanalyzer::{
    AssetStatus, ENGLISH, SpeechAnalyzerTranscriber, english_asset_status, install_english_asset,
};

const FIXTURE: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/hello.wav");
const CHUNK_SAMPLES: usize = 1_600;
const CHUNK_DURATION: Duration = Duration::from_millis(100);
const ROUNDING: Duration = Duration::from_millis(1);

fn fixture_chunks() -> Vec<AudioChunk> {
    let mut reader = hound::WavReader::open(FIXTURE).unwrap();
    let samples: Vec<f32> = reader
        .samples::<i16>()
        .map(|sample| f32::from(sample.unwrap()) / f32::from(i16::MAX))
        .collect();
    samples
        .chunks(CHUNK_SAMPLES)
        .zip(0u32..)
        .map(|(chunk, index)| AudioChunk {
            samples: chunk.to_vec(),
            captured_at: CHUNK_DURATION * index,
        })
        .collect()
}

fn final_text(events: Vec<TranscriptEvent>) -> String {
    events
        .into_iter()
        .filter_map(|event| match event {
            TranscriptEvent::Final(utterance) => Some(utterance.text),
            TranscriptEvent::Partial(_) => None,
        })
        .collect::<Vec<_>>()
        .join(" ")
}

#[test]
#[ignore = "needs macOS 26 with the English speech asset installed"]
fn english_speech_from_the_fixture_is_transcribed_on_device() {
    assert_eq!(english_asset_status().unwrap(), AssetStatus::Installed);
    let mut transcriber = SpeechAnalyzerTranscriber::start(ENGLISH).unwrap();

    let mut events: Vec<_> = fixture_chunks()
        .iter()
        .flat_map(|chunk| transcriber.push(chunk).unwrap().events)
        .collect();
    events.extend(transcriber.finish().unwrap());

    let text = final_text(events).to_lowercase();
    assert!(text.contains("test"), "unexpected transcript: {text}");
}

#[test]
#[ignore = "needs macOS 26 and downloads Apple's English speech asset"]
fn the_english_speech_asset_can_be_installed_on_device() {
    install_english_asset().unwrap();

    assert_eq!(english_asset_status().unwrap(), AssetStatus::Installed);
}

#[test]
#[ignore = "needs macOS 26 with the English speech asset installed"]
fn the_finalised_point_never_runs_ahead_of_the_audio_pushed() {
    let mut transcriber = SpeechAnalyzerTranscriber::start(ENGLISH).unwrap();

    let ahead: Vec<Duration> = fixture_chunks()
        .iter()
        .map(|chunk| (chunk.ends_at(), transcriber.push(chunk).unwrap()))
        .filter(|(heard, transcription)| transcription.finalised_through > *heard + ROUNDING)
        .map(|(heard, _)| heard)
        .collect();
    transcriber.finish().unwrap();

    assert!(ahead.is_empty(), "ran ahead after {ahead:?}");
}
