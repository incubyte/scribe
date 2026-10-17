#[cfg(target_os = "macos")]
mod macos;
#[cfg(not(any(target_os = "macos", windows)))]
mod unsupported;
#[cfg(windows)]
mod windows;

use serde::Serialize;

#[cfg(target_os = "macos")]
pub use macos::{install, retry, status};
#[cfg(not(any(target_os = "macos", windows)))]
pub use unsupported::{install, retry, status};
#[cfg(windows)]
pub use windows::{install, retry, status};

#[cfg_attr(not(windows), allow(dead_code))]
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "state", rename_all = "lowercase")]
pub enum ModelDownloadView {
    Ready,
    Downloading { percent: u8 },
    Failed { reason_kind: &'static str },
}
