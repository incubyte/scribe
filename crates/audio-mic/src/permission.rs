#![allow(unsafe_code)]

use std::sync::mpsc;
use std::time::Duration;

use block2::RcBlock;
use objc2::runtime::Bool;
use objc2_av_foundation::{AVAuthorizationStatus, AVCaptureDevice, AVMediaType, AVMediaTypeAudio};
use scribe_core::ports::CaptureError;

const PROMPT_TIMEOUT: Duration = Duration::from_secs(120);

pub(crate) fn ensure_microphone_access() -> Result<(), CaptureError> {
    let media_type = audio_media_type()?;
    match unsafe { AVCaptureDevice::authorizationStatusForMediaType(media_type) } {
        AVAuthorizationStatus::Authorized => Ok(()),
        AVAuthorizationStatus::NotDetermined => request_access(media_type),
        _ => Err(CaptureError::PermissionDenied),
    }
}

fn audio_media_type() -> Result<&'static AVMediaType, CaptureError> {
    unsafe { AVMediaTypeAudio }.ok_or(CaptureError::DeviceUnavailable)
}

fn request_access(media_type: &AVMediaType) -> Result<(), CaptureError> {
    let (answer, answered) = mpsc::sync_channel(1);
    let handler = RcBlock::new(move |granted: Bool| {
        let _ = answer.send(granted.as_bool());
    });
    unsafe { AVCaptureDevice::requestAccessForMediaType_completionHandler(media_type, &handler) };
    match answered.recv_timeout(PROMPT_TIMEOUT) {
        Ok(true) => Ok(()),
        _ => Err(CaptureError::PermissionDenied),
    }
}
