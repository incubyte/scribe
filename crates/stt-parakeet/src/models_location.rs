use std::io;
use std::path::PathBuf;

use crate::DownloadError;

const MODEL_DIRECTORY: &str = "parakeet-tdt-0.6b-v3-int8";

pub fn default_models_dir() -> Result<PathBuf, DownloadError> {
    let app_data = dirs::data_local_dir()
        .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "no per-user app-data folder"))?;
    Ok(app_data.join("Scribe").join("models").join(MODEL_DIRECTORY))
}
