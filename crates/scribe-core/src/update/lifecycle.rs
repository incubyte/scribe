use std::fmt;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReleaseVersion(String);

impl ReleaseVersion {
    #[must_use]
    pub fn new(text: impl Into<String>) -> Self {
        Self(text.into())
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for ReleaseVersion {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

enum Phase<P> {
    UpToDate,
    Downloading(ReleaseVersion),
    Ready { version: ReleaseVersion, package: P },
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
            Phase::Downloading(_) => false,
            Phase::Ready { version: ready, .. } => ready != version,
        };
        if starts {
            self.phase = Phase::Downloading(version.clone());
        }
        starts
    }

    pub fn finish_download(&mut self, package: P) {
        if let Phase::Downloading(version) = &self.phase {
            self.phase = Phase::Ready {
                version: version.clone(),
                package,
            };
        }
    }

    pub fn abandon_download(&mut self) {
        if matches!(self.phase, Phase::Downloading(_)) {
            self.phase = Phase::UpToDate;
        }
    }

    #[must_use]
    pub fn is_ready(&self) -> bool {
        matches!(self.phase, Phase::Ready { .. })
    }

    #[must_use]
    pub fn ready_version(&self) -> Option<&ReleaseVersion> {
        match &self.phase {
            Phase::Ready { version, .. } => Some(version),
            _ => None,
        }
    }

    #[must_use]
    pub fn package_to_install(&self) -> Option<P>
    where
        P: Clone,
    {
        match &self.phase {
            Phase::Ready { package, .. } => Some(package.clone()),
            _ => None,
        }
    }
}
