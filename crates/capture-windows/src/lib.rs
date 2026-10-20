#[cfg(windows)]
mod capturer;
#[cfg(windows)]
mod outlines;

#[cfg(windows)]
pub use capturer::DesktopCapturer;
