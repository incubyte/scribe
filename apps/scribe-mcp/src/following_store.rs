use std::path::PathBuf;

use chrono::{DateTime, FixedOffset};
use scribe_core::capture::{
    Frame, ScreenshotName, ScreenshotReference, ServedImage, SessionMoment,
};
use scribe_core::ports::{ScreenshotStore, StoreError, TranscriptStore};
use scribe_core::recording::{Segment, SessionDocument, SessionHeader, SessionId, SessionSummary};
use store_markdown::{MarkdownTranscriptStore, TranscriptsFolder};

pub struct SettingsFollowingStore {
    settings_path: PathBuf,
    inner: MarkdownTranscriptStore,
}

impl SettingsFollowingStore {
    pub fn new(settings_path: PathBuf) -> Result<SettingsFollowingStore, StoreError> {
        let folder = TranscriptsFolder::configured_in(&settings_path)?;
        Ok(SettingsFollowingStore {
            settings_path,
            inner: MarkdownTranscriptStore::new(folder),
        })
    }

    fn follow_settings(&self) -> Result<(), StoreError> {
        self.inner
            .relocate(TranscriptsFolder::configured_in(&self.settings_path)?);
        Ok(())
    }
}

impl TranscriptStore for SettingsFollowingStore {
    fn create(&self, header: &SessionHeader) -> Result<SessionId, StoreError> {
        self.inner.create(header)
    }

    fn append(&self, id: &SessionId, segment: &Segment) -> Result<(), StoreError> {
        self.inner.append(id, segment)
    }

    fn finalise(&self, id: &SessionId, ended_at: DateTime<FixedOffset>) -> Result<(), StoreError> {
        self.inner.finalise(id, ended_at)
    }

    fn list(&self, limit: usize) -> Result<Vec<SessionSummary>, StoreError> {
        self.follow_settings()?;
        self.inner.list(limit)
    }

    fn list_all(&self) -> Result<Vec<SessionSummary>, StoreError> {
        self.follow_settings()?;
        self.inner.list_all()
    }

    fn read(&self, id: &SessionId) -> Result<SessionDocument, StoreError> {
        self.follow_settings()?;
        self.inner.read(id)
    }

    fn mark_abandoned_incomplete(&self) -> Result<usize, StoreError> {
        self.inner.mark_abandoned_incomplete()
    }
}

impl ScreenshotStore for SettingsFollowingStore {
    fn save(
        &self,
        moment: &SessionMoment,
        screenshot: &Frame,
    ) -> Result<ScreenshotName, StoreError> {
        self.inner.save(moment, screenshot)
    }

    fn list_screenshots(&self, session: &SessionId) -> Result<Vec<ScreenshotName>, StoreError> {
        self.follow_settings()?;
        self.inner.list_screenshots(session)
    }

    fn serve(
        &self,
        reference: &ScreenshotReference,
        max_long_edge: u32,
    ) -> Result<ServedImage, StoreError> {
        self.follow_settings()?;
        self.inner.serve(reference, max_long_edge)
    }
}
