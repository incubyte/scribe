use std::io::ErrorKind;
use std::path::Path;

use image::codecs::png::CompressionType;
use scribe_core::capture::{
    Frame, ScreenshotName, ScreenshotReference, ServedImage, SessionMoment,
};
use scribe_core::ports::{ScreenshotStore, StoreError};
use scribe_core::recording::SessionId;

use crate::image_codec::encode_png;
use crate::month_layout::session_folder_path;
use crate::private_file;
use crate::store_error::io_error;
use crate::writer::MarkdownTranscriptStore;

impl ScreenshotStore for MarkdownTranscriptStore {
    fn save(
        &self,
        moment: &SessionMoment,
        screenshot: &Frame,
    ) -> Result<ScreenshotName, StoreError> {
        let session_folder = session_folder_path(self.folder().path(), &moment.session);
        let png = encode_png(screenshot, CompressionType::Default)?;
        for name in ScreenshotName::candidates(moment.offset) {
            if write_unless_taken(&session_folder, &name, &png)? {
                return Ok(name);
            }
        }
        Err(StoreError::NoFreeScreenshotName)
    }

    fn list(&self, _session: &SessionId) -> Result<Vec<ScreenshotName>, StoreError> {
        Ok(Vec::new())
    }

    fn serve(
        &self,
        reference: &ScreenshotReference,
        _max_long_edge: u32,
    ) -> Result<ServedImage, StoreError> {
        Err(StoreError::ScreenshotNotFound {
            reference: reference.clone(),
        })
    }
}

fn write_unless_taken(
    session_folder: &Path,
    name: &ScreenshotName,
    png: &[u8],
) -> Result<bool, StoreError> {
    let path = session_folder.join(name.to_string());
    match private_file::create_new_bytes(&path, png) {
        Ok(()) => Ok(true),
        Err(error) if error.kind() == ErrorKind::AlreadyExists => Ok(false),
        Err(source) => Err(io_error(&path, source)),
    }
}
