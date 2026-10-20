use std::ffi::OsString;
use std::fs;
use std::path::Path;
use std::sync::Arc;
use std::time::Duration;

use chrono::DateTime;
use scribe_core::recording::{Lane, Readiness, Recorder, SessionHeader, Speaker};
use scribe_core::testing::{
    FakeAudioSource, RecordingListenerSpy, ScriptedTranscriber, chunk_at, finished,
};
use store_markdown::{MarkdownTranscriptStore, TranscriptsFolder};

const AUDIO_EXTENSIONS: [&str; 6] = ["wav", "caf", "aiff", "m4a", "pcm", "raw"];

fn audio_files_in(dir: &Path) -> Vec<OsString> {
    fs::read_dir(dir)
        .unwrap()
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| {
            path.extension()
                .is_some_and(|ext| AUDIO_EXTENSIONS.iter().any(|audio| ext == *audio))
        })
        .map(std::path::PathBuf::into_os_string)
        .collect()
}

fn file_names_in(dir: &Path) -> Vec<OsString> {
    fs::read_dir(dir)
        .unwrap()
        .map(|entry| entry.unwrap().file_name())
        .collect()
}

fn recorder_saving_to(folder: &Path) -> Recorder {
    let store = MarkdownTranscriptStore::new(TranscriptsFolder::at(folder.to_path_buf()));
    let mut recorder = Recorder::new(Arc::new(store), Arc::new(RecordingListenerSpy::default()));
    recorder.set_readiness(Readiness {
        privacy_acknowledged: true,
        model_ready: true,
    });
    recorder
}

fn mic_saying_hello() -> Lane {
    let chunks = vec![
        chunk_at(Duration::ZERO),
        chunk_at(Duration::from_millis(100)),
    ];
    let reply = finished("Hello there", Duration::ZERO, Duration::from_secs(1));
    Lane {
        speaker: Speaker::Me,
        source: Box::new(FakeAudioSource::with_chunks(chunks)),
        transcriber: Box::new(ScriptedTranscriber::new(vec![vec![reply]])),
    }
}

#[test]
fn a_whole_recording_leaves_only_its_markdown_transcript_on_disk() {
    let parent = tempfile::tempdir().unwrap();
    let folder = parent.path().join("Scribe");
    let audio_in_temp_before = audio_files_in(&std::env::temp_dir());
    let mut recorder = recorder_saving_to(&folder);
    let started_at = DateTime::parse_from_rfc3339("2026-10-17T14:30:05+05:30").unwrap();
    let header = SessionHeader::recording(started_at, "speechanalyzer", "en-US");

    recorder.start(vec![mic_saying_hello()], header).unwrap();
    recorder.stop().unwrap();

    assert_eq!(
        file_names_in(&folder.join("2026-10").join("2026-10-17-143005")),
        vec!["2026-10-17-143005.md"]
    );
    assert_eq!(audio_files_in(&std::env::temp_dir()), audio_in_temp_before);
}
