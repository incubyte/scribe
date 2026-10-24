use crate::recording::RecorderState;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InstallConditions {
    pub install_due: bool,
    pub recorder: RecorderState,
    pub in_call: bool,
    pub popover_open: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WaitReason {
    NothingReady,
    Recording,
    InCall,
    PopoverOpen,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InstallMoment {
    Now,
    Wait(WaitReason),
}

#[must_use]
pub fn install_moment(conditions: &InstallConditions) -> InstallMoment {
    if !conditions.install_due {
        return InstallMoment::Wait(WaitReason::NothingReady);
    }
    if conditions.recorder == RecorderState::Recording {
        return InstallMoment::Wait(WaitReason::Recording);
    }
    if conditions.in_call {
        return InstallMoment::Wait(WaitReason::InCall);
    }
    if conditions.popover_open {
        return InstallMoment::Wait(WaitReason::PopoverOpen);
    }
    InstallMoment::Now
}
