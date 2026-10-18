#![cfg(target_os = "macos")]

mod bridge;
mod reply;

use std::time::Duration;

use scribe_core::ports::{TranscribeError, Transcriber};
use scribe_core::recording::{AudioChunk, TranscriptEvent, Transcription};

use crate::reply::Reply;

pub const ENGINE_NAME: &str = "speechanalyzer";
pub const ENGLISH: &str = "en-US";
pub const FINAL_RESULTS_WAIT: Duration = Duration::from_secs(6);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AssetStatus {
    Installed,
    Missing,
    Unsupported,
}

pub fn english_asset_status() -> Result<AssetStatus, TranscribeError> {
    let status = Reply::parse(&bridge::asset_status(ENGLISH))?.status()?;
    Ok(match status.as_str() {
        "installed" => AssetStatus::Installed,
        "supported" | "downloading" => AssetStatus::Missing,
        _ => AssetStatus::Unsupported,
    })
}

pub fn install_english_asset() -> Result<(), TranscribeError> {
    Reply::parse(&bridge::install_asset(ENGLISH)).map(|_| ())
}

pub struct SpeechAnalyzerTranscriber {
    session: bridge::SessionToken,
    finished: bool,
}

impl SpeechAnalyzerTranscriber {
    pub fn start(locale: &str) -> Result<SpeechAnalyzerTranscriber, TranscribeError> {
        let session = Reply::parse(&bridge::start(locale))?.session()?;
        Ok(SpeechAnalyzerTranscriber {
            session,
            finished: false,
        })
    }
}

impl Transcriber for SpeechAnalyzerTranscriber {
    fn push(&mut self, chunk: &AudioChunk) -> Result<Transcription, TranscribeError> {
        Reply::parse(&bridge::push(self.session, &chunk.samples))?.transcription()
    }

    fn finish(&mut self) -> Result<Vec<TranscriptEvent>, TranscribeError> {
        self.finished = true;
        Ok(Reply::parse(&bridge::finish(self.session))?.events())
    }

    fn final_results_wait(&self) -> Option<Duration> {
        Some(FINAL_RESULTS_WAIT)
    }
}

impl Drop for SpeechAnalyzerTranscriber {
    fn drop(&mut self) {
        if !self.finished {
            let _ = bridge::finish(self.session);
        }
    }
}
