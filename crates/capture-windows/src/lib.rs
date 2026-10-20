#[cfg(windows)]
mod capturer;

#[cfg(windows)]
pub use capturer::DesktopCapturer;
