#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShortcutKind {
    FullScreen,
}

pub const SHORTCUT_KINDS: [ShortcutKind; 1] = [ShortcutKind::FullScreen];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DefaultShortcut {
    pub keys: &'static str,
    pub label: &'static str,
}

#[cfg(target_os = "macos")]
const FULL_SCREEN_DEFAULT: DefaultShortcut = DefaultShortcut {
    keys: "control+alt+super+Digit1",
    label: "⌃⌥⌘1",
};

#[cfg(not(target_os = "macos"))]
const FULL_SCREEN_DEFAULT: DefaultShortcut = DefaultShortcut {
    keys: "shift+control+alt+Digit1",
    label: "Ctrl+Shift+Alt+1",
};

impl ShortcutKind {
    #[must_use]
    pub fn default_shortcut(self) -> DefaultShortcut {
        match self {
            ShortcutKind::FullScreen => FULL_SCREEN_DEFAULT,
        }
    }
}
