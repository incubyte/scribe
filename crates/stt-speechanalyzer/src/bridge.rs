#![allow(unsafe_code)]

use swift_rs::{Int, SRData, SRString, swift};

pub(crate) type SessionToken = Int;

swift!(fn scribe_speech_start(locale: &SRString) -> SRString);
swift!(fn scribe_speech_push(session: Int, samples: &SRData) -> SRString);
swift!(fn scribe_speech_finish(session: Int) -> SRString);
swift!(fn scribe_speech_asset_status(locale: &SRString) -> SRString);
swift!(fn scribe_speech_install_asset(locale: &SRString) -> SRString);

pub(crate) fn start(locale: &str) -> String {
    let locale = SRString::from(locale);
    unsafe { scribe_speech_start(&locale) }.to_string()
}

pub(crate) fn push(session: SessionToken, samples: &[f32]) -> String {
    let bytes: Vec<u8> = samples
        .iter()
        .flat_map(|sample| sample.to_le_bytes())
        .collect();
    let samples = SRData::from(bytes.as_slice());
    unsafe { scribe_speech_push(session, &samples) }.to_string()
}

pub(crate) fn finish(session: SessionToken) -> String {
    unsafe { scribe_speech_finish(session) }.to_string()
}

pub(crate) fn asset_status(locale: &str) -> String {
    let locale = SRString::from(locale);
    unsafe { scribe_speech_asset_status(&locale) }.to_string()
}

pub(crate) fn install_asset(locale: &str) -> String {
    let locale = SRString::from(locale);
    unsafe { scribe_speech_install_asset(&locale) }.to_string()
}
