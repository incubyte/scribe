use cidre::core_audio as ca;
use scribe_core::call::TEAMS_BUNDLE_ID_PREFIX;
use scribe_core::ports::{CallProbe, CaptureError};

#[derive(Default)]
pub struct CoreAudioCallProbe;

impl CoreAudioCallProbe {
    pub fn bundle_ids_using_microphone(&self) -> Result<Vec<String>, CaptureError> {
        let processes =
            ca::System::processes().map_err(|status| failure("list audio processes", status))?;
        Ok(processes
            .iter()
            .filter(|process| uses_microphone(process))
            .filter_map(bundle_id_of)
            .collect())
    }
}

impl CallProbe for CoreAudioCallProbe {
    fn call_app_holds_microphone(&self) -> Result<bool, CaptureError> {
        Ok(self
            .bundle_ids_using_microphone()?
            .iter()
            .any(|bundle_id| bundle_id.starts_with(TEAMS_BUNDLE_ID_PREFIX)))
    }
}

fn uses_microphone(process: &ca::Process) -> bool {
    process.is_running_input().unwrap_or(false)
}

fn bundle_id_of(process: &ca::Process) -> Option<String> {
    let bundle_id = process.bundle_id().ok()?.to_string();
    (!bundle_id.is_empty()).then_some(bundle_id)
}

fn failure(action: &str, status: impl std::fmt::Debug) -> CaptureError {
    CaptureError::StreamFailed(format!("could not {action}: {status:?}"))
}
