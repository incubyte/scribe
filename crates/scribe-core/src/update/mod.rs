mod install_moment;
mod lifecycle;
mod schedule;

use std::time::Duration;

pub use install_moment::{InstallConditions, InstallMoment, WaitReason, install_moment};
pub use lifecycle::{ReleaseVersion, UpdateLifecycle};
pub use schedule::CheckSchedule;

pub const CHECK_INTERVAL: Duration = Duration::from_secs(60 * 60);
pub const CHECK_DELAY_AFTER_WAKE: Duration = Duration::from_secs(30);
