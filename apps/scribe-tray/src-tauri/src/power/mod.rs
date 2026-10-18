#[cfg(target_os = "macos")]
mod macos;
#[cfg(windows)]
mod windows;

#[cfg(target_os = "macos")]
pub use macos::on_will_sleep;
#[cfg(windows)]
pub use windows::on_will_sleep;
