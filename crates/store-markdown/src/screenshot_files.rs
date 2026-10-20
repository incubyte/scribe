use std::fs;
use std::io::ErrorKind;
use std::path::Path;

use image::codecs::png::CompressionType;
use scribe_core::capture::{
    Frame, ScreenshotName, ScreenshotReference, ServedImage, SessionMoment,
};
use scribe_core::ports::{ScreenshotStore, StoreError};
use scribe_core::recording::SessionId;

use crate::image_codec::{encode_png, scale_png_to_fit};
use crate::month_layout::session_folder_path;
use crate::private_file;
use crate::store_error::{io_error, malformed};
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

    fn list_screenshots(&self, session: &SessionId) -> Result<Vec<ScreenshotName>, StoreError> {
        let session_folder = session_folder_path(self.folder().path(), session);
        match fs::read_dir(&session_folder) {
            Ok(entries) => Ok(entries.filter_map(screenshot_name_of).collect()),
            Err(error) if error.kind() == ErrorKind::NotFound => Ok(Vec::new()),
            Err(source) => Err(io_error(&session_folder, source)),
        }
    }

    fn serve(
        &self,
        reference: &ScreenshotReference,
        max_long_edge: u32,
    ) -> Result<ServedImage, StoreError> {
        let path = session_folder_path(self.folder().path(), reference.session())
            .join(reference.name().to_string());
        let png = fs::read(&path).map_err(|source| match source.kind() {
            ErrorKind::NotFound => StoreError::ScreenshotNotFound {
                reference: reference.clone(),
            },
            _ => io_error(&path, source),
        })?;
        scale_png_to_fit(&png, max_long_edge).ok_or_else(|| malformed(&path))
    }
}

fn screenshot_name_of(entry: std::io::Result<fs::DirEntry>) -> Option<ScreenshotName> {
    let file_name = entry.ok()?.file_name();
    ScreenshotName::parse(file_name.to_str()?).ok()
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
