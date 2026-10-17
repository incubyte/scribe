use std::io;
use std::path::Path;

use scribe_core::ports::StoreError;

pub(crate) fn file_name(path: &Path) -> String {
    path.file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_default()
}

pub(crate) fn malformed(path: &Path) -> StoreError {
    StoreError::Malformed {
        file_name: file_name(path),
    }
}

pub(crate) fn io_error(path: &Path, source: io::Error) -> StoreError {
    StoreError::Io {
        path: path.to_path_buf(),
        source,
    }
}
