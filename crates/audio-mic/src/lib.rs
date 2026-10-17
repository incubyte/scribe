mod capture;
mod mic_source;
#[cfg(target_os = "macos")]
mod permission;

pub use mic_source::MicSource;
