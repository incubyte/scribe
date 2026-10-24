use std::fmt;
use std::time::Instant;

use super::CHECK_INTERVAL;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReleaseVersion(String);

impl ReleaseVersion {
    #[must_use]
    pub fn new(text: impl Into<String>) -> Self {
        Self(text.into())
    }
}

impl fmt::Display for ReleaseVersion {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

struct StagedUpdate<P> {
    version: ReleaseVersion,
    package: P,
    failed_install_at: Option<Instant>,
}

impl<P> StagedUpdate<P> {
    fn is_install_due(&self, now: Instant) -> bool {
        self.failed_install_at
            .is_none_or(|failed| now.saturating_duration_since(failed) >= CHECK_INTERVAL)
    }
}

enum Phase<P> {
    UpToDate,
    Downloading {
        version: ReleaseVersion,
        previous: Option<StagedUpdate<P>>,
    },
    Ready(StagedUpdate<P>),
    Installing(ReleaseVersion),
    Installed,
}

impl<P> Phase<P> {
    fn installable(&self) -> Option<&StagedUpdate<P>> {
        match self {
            Phase::Ready(staged)
            | Phase::Downloading {
                previous: Some(staged),
                ..
            } => Some(staged),
            _ => None,
        }
    }

    fn into_installable(self) -> Result<StagedUpdate<P>, Self> {
        match self {
            Phase::Ready(staged)
            | Phase::Downloading {
                previous: Some(staged),
                ..
            } => Ok(staged),
            other => Err(other),
        }
    }
}

pub struct UpdateLifecycle<P> {
    phase: Phase<P>,
}

impl<P> Default for UpdateLifecycle<P> {
    fn default() -> Self {
        Self::new()
    }
}

impl<P> UpdateLifecycle<P> {
    #[must_use]
    pub fn new() -> Self {
        Self {
            phase: Phase::UpToDate,
        }
    }

    pub fn begin_download(&mut self, version: &ReleaseVersion) -> bool {
        let starts = match &self.phase {
            Phase::UpToDate => true,
            Phase::Ready(staged) => &staged.version != version,
            Phase::Downloading { .. } | Phase::Installing(_) | Phase::Installed => false,
        };
        if starts {
            let previous = self.take_phase().into_installable().ok();
            self.phase = Phase::Downloading {
                version: version.clone(),
                previous,
            };
        }
        starts
    }

    pub fn finish_download(&mut self, package: P) {
        if let Phase::Downloading { version, .. } = &self.phase {
            self.phase = Phase::Ready(StagedUpdate {
                version: version.clone(),
                package,
                failed_install_at: None,
            });
        }
    }

    pub fn abandon_download(&mut self) {
        if let Phase::Downloading { .. } = self.phase {
            self.phase = self
                .take_phase()
                .into_installable()
                .map_or(Phase::UpToDate, Phase::Ready);
        }
    }

    #[must_use]
    pub fn is_update_waiting(&self, now: Instant) -> bool {
        self.is_installed()
            || self
                .phase
                .installable()
                .is_some_and(|staged| staged.is_install_due(now))
    }

    pub fn claim_install(&mut self) -> Option<P> {
        match self.take_phase().into_installable() {
            Ok(staged) => {
                self.phase = Phase::Installing(staged.version);
                Some(staged.package)
            }
            Err(unchanged) => {
                self.phase = unchanged;
                None
            }
        }
    }

    pub fn release_failed_install(&mut self, package: P, now: Instant) {
        match self.take_phase() {
            Phase::Installing(version) => {
                self.phase = Phase::Ready(StagedUpdate {
                    version,
                    package,
                    failed_install_at: Some(now),
                });
            }
            unchanged => self.phase = unchanged,
        }
    }

    pub fn finish_install(&mut self) {
        if let Phase::Installing(_) = self.phase {
            self.phase = Phase::Installed;
        }
    }

    #[must_use]
    pub fn is_installed(&self) -> bool {
        matches!(self.phase, Phase::Installed)
    }

    fn take_phase(&mut self) -> Phase<P> {
        std::mem::replace(&mut self.phase, Phase::UpToDate)
    }
}
