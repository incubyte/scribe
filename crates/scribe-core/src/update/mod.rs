mod lifecycle;
mod schedule;

use std::time::Duration;

pub use lifecycle::{ReleaseVersion, UpdateLifecycle};
pub use schedule::CheckSchedule;

pub const CHECK_INTERVAL: Duration = Duration::from_secs(60 * 60);
