#[cfg(target_os = "macos")]
mod capturer;
#[cfg(target_os = "macos")]
mod rgba;

#[cfg(target_os = "macos")]
pub use capturer::ScreenCaptureKitCapturer;
