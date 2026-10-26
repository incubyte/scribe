use std::sync::{Mutex, MutexGuard, PoisonError};
use std::time::Instant;

use scribe_core::calendar::{
    Meeting, SwitchInputs, SwitchPrompt, SwitchPrompts, TITLE_RECHECK_INTERVAL,
};
use serde::Serialize;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SwitchPromptView {
    from_subject: String,
    to_subject: String,
}

impl SwitchPromptView {
    #[must_use]
    pub fn new(from_subject: &str, to_subject: &str) -> SwitchPromptView {
        SwitchPromptView {
            from_subject: from_subject.to_owned(),
            to_subject: to_subject.to_owned(),
        }
    }
}

impl From<&SwitchPrompt> for SwitchPromptView {
    fn from(prompt: &SwitchPrompt) -> SwitchPromptView {
        SwitchPromptView::new(&prompt.from.subject, &prompt.to.subject)
    }
}

#[derive(Default)]
struct Switching {
    prompts: SwitchPrompts,
    open: Option<SwitchPrompt>,
    last_title_check: Option<Instant>,
}

#[derive(Default)]
pub struct SwitchState {
    switching: Mutex<Switching>,
}

impl SwitchState {
    #[must_use]
    pub fn new() -> SwitchState {
        SwitchState::default()
    }

    pub fn title_recheck_due(&self, now: Instant) -> bool {
        let mut switching = self.switching();
        let due = switching
            .last_title_check
            .is_none_or(|last| now.duration_since(last) >= TITLE_RECHECK_INTERVAL);
        if due {
            switching.last_title_check = Some(now);
        }
        due
    }

    pub fn offer(&self, inputs: &SwitchInputs) -> Option<SwitchPromptView> {
        let mut switching = self.switching();
        if switching.open.is_some() {
            return None;
        }
        let prompt = switching.prompts.due(inputs)?;
        let view = SwitchPromptView::from(&prompt);
        switching.open = Some(prompt);
        Some(view)
    }

    #[must_use]
    pub fn is_open(&self) -> bool {
        self.switching().open.is_some()
    }

    #[must_use]
    pub fn open_view(&self) -> Option<SwitchPromptView> {
        self.switching().open.as_ref().map(SwitchPromptView::from)
    }

    pub fn take_for_switch(&self) -> Option<Meeting> {
        self.switching().open.take().map(|prompt| prompt.to)
    }

    pub fn withdraw(&self) -> bool {
        self.switching().open.take().is_some()
    }

    fn switching(&self) -> MutexGuard<'_, Switching> {
        self.switching
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
    }
}
