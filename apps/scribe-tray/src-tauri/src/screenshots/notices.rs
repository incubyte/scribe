use scribe_core::capture::CaptureOutcome;
use scribe_core::recording::offset_as_clock;

const SCREENSHOT_TAKEN_TITLE: &str = "Screenshot taken";
const SCREENSHOT_NOT_SAVED_TITLE: &str = "Screenshot not saved";
const SCREENSHOT_FAILED_TITLE: &str = "Screenshot failed";
const SHORTCUT_UNAVAILABLE_TITLE: &str = "Screenshot shortcut unavailable";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Notice {
    pub title: &'static str,
    pub body: String,
}

#[must_use]
pub fn notice_for(outcome: &CaptureOutcome) -> Notice {
    match outcome {
        CaptureOutcome::SavedIntoSession { name, .. } => Notice {
            title: SCREENSHOT_TAKEN_TITLE,
            body: format!(
                "Screenshot saved at {} and copied to the clipboard",
                offset_as_clock(name.offset())
            ),
        },
        CaptureOutcome::CopiedOnly { .. } => Notice {
            title: SCREENSHOT_TAKEN_TITLE,
            body: "Screenshot copied to the clipboard".to_owned(),
        },
        CaptureOutcome::CopiedButNotSaved { .. } => Notice {
            title: SCREENSHOT_NOT_SAVED_TITLE,
            body: "Screenshot copied, but it couldn't be saved to this meeting".to_owned(),
        },
    }
}

#[must_use]
pub fn capture_failed_notice() -> Notice {
    Notice {
        title: SCREENSHOT_FAILED_TITLE,
        body: "Couldn't take the screenshot. Try again.".to_owned(),
    }
}

#[must_use]
pub fn shortcut_unavailable_notice(label: &str) -> Notice {
    Notice {
        title: SHORTCUT_UNAVAILABLE_TITLE,
        body: format!(
            "Scribe couldn't use {label} for screenshots because another app or the system already uses it. Change it in Scribe Settings."
        ),
    }
}
