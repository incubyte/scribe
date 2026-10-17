#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Readiness {
    pub privacy_acknowledged: bool,
    pub model_ready: bool,
}

impl Readiness {
    fn is_complete(self) -> bool {
        self.privacy_acknowledged && self.model_ready
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RecorderState {
    Locked,
    Idle,
    Recording,
}

#[must_use]
pub fn recorder_state(readiness: &Readiness, has_active_session: bool) -> RecorderState {
    match (readiness.is_complete(), has_active_session) {
        (false, _) => RecorderState::Locked,
        (true, false) => RecorderState::Idle,
        (true, true) => RecorderState::Recording,
    }
}
