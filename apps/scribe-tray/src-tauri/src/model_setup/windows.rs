use stt_parakeet::{DownloadError, HttpsFetcher, ModelProvisioner, default_models_dir};

use super::SetupFailed;
use crate::lanes::{self, SpeechEngine};

const ENGINE_FAILURE: &str = "speech_engine";

pub fn prepare_speech_engine(progress: &dyn Fn(Option<u8>)) -> Result<SpeechEngine, SetupFailed> {
    download_model(progress).map_err(|error| SetupFailed {
        reason_kind: error.kind(),
    })?;
    lanes::load_speech_engine().map_err(|_| SetupFailed {
        reason_kind: ENGINE_FAILURE,
    })
}

fn download_model(progress: &dyn Fn(Option<u8>)) -> Result<(), DownloadError> {
    let provisioner = ModelProvisioner::new(default_models_dir()?, Box::new(HttpsFetcher::new()?));
    if provisioner.is_ready() {
        return Ok(());
    }
    progress(Some(0));
    provisioner.provision(&|downloaded| progress(Some(downloaded.percent())))
}
