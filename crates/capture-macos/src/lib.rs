#[cfg(target_os = "macos")]
mod capturer;
#[cfg(target_os = "macos")]
mod outlines;
#[cfg(target_os = "macos")]
mod rgba;
#[cfg(target_os = "macos")]
mod teams_titles;

#[cfg(target_os = "macos")]
pub use capturer::ScreenCaptureKitCapturer;
#[cfg(target_os = "macos")]
pub use teams_titles::CaptureTeamsTitles;
