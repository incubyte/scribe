use serde::{Deserialize, Serialize};
use tauri_plugin_global_shortcut::Shortcut;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ShortcutKind {
    FullScreen,
    Region,
}

pub const SHORTCUT_KINDS: [ShortcutKind; 2] = [ShortcutKind::FullScreen, ShortcutKind::Region];

impl ShortcutKind {
    #[must_use]
    pub fn storage_key(self) -> &'static str {
        match self {
            ShortcutKind::FullScreen => "screenshot_shortcut_full_screen",
            ShortcutKind::Region => "screenshot_shortcut_region",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Platform {
    MacOs,
    Windows,
}

type ModifierSymbols = [(&'static str, &'static str); 4];

const MAC_SYMBOLS: ModifierSymbols = [
    ("control", "⌃"),
    ("alt", "⌥"),
    ("shift", "⇧"),
    ("super", "⌘"),
];

const WINDOWS_SYMBOLS: ModifierSymbols = [
    ("control", "Ctrl"),
    ("shift", "Shift"),
    ("alt", "Alt"),
    ("super", "Win"),
];

impl Platform {
    #[must_use]
    pub fn current() -> Platform {
        if cfg!(target_os = "macos") {
            Platform::MacOs
        } else {
            Platform::Windows
        }
    }

    fn modifier_symbols(self) -> ModifierSymbols {
        match self {
            Platform::MacOs => MAC_SYMBOLS,
            Platform::Windows => WINDOWS_SYMBOLS,
        }
    }

    fn separator(self) -> &'static str {
        match self {
            Platform::MacOs => "",
            Platform::Windows => "+",
        }
    }

    fn default_keys(self, kind: ShortcutKind) -> &'static str {
        match (self, kind) {
            (Platform::MacOs, ShortcutKind::FullScreen) => "control+alt+super+Digit1",
            (Platform::MacOs, ShortcutKind::Region) => "control+alt+super+Digit2",
            (Platform::Windows, ShortcutKind::FullScreen) => "shift+control+alt+Digit1",
            (Platform::Windows, ShortcutKind::Region) => "shift+control+alt+Digit2",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct PressedKeys {
    pub code: String,
    pub control: bool,
    pub alt: bool,
    pub shift: bool,
    pub meta: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BindingProblem {
    NeedsModifier,
    DuplicateOf(ShortcutKind),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShortcutBinding {
    keys: String,
}

impl ShortcutBinding {
    #[must_use]
    pub fn default_for(kind: ShortcutKind, platform: Platform) -> ShortcutBinding {
        ShortcutBinding {
            keys: platform.default_keys(kind).to_owned(),
        }
    }

    pub fn from_pressed(pressed: &PressedKeys) -> Result<ShortcutBinding, BindingProblem> {
        let has_anchor_modifier = pressed.control || pressed.alt || pressed.meta;
        if !has_anchor_modifier || !is_supported_key(&pressed.code) {
            return Err(BindingProblem::NeedsModifier);
        }
        let modifiers = [
            (pressed.shift, "shift"),
            (pressed.control, "control"),
            (pressed.alt, "alt"),
            (pressed.meta, "super"),
        ];
        let keys = modifiers
            .into_iter()
            .filter(|(held, _)| *held)
            .map(|(_, name)| name)
            .chain([pressed.code.as_str()])
            .collect::<Vec<_>>()
            .join("+");
        Ok(ShortcutBinding { keys })
    }

    #[must_use]
    pub fn from_stored(
        kind: ShortcutKind,
        raw: Option<&str>,
        platform: Platform,
    ) -> ShortcutBinding {
        let Some(raw) = raw else {
            return ShortcutBinding::default_for(kind, platform);
        };
        raw.parse::<Shortcut>()
            .map(|shortcut| ShortcutBinding {
                keys: shortcut.into_string(),
            })
            .unwrap_or_else(|error| {
                tracing::warn!(%error, ?kind, "ignoring an unreadable stored screenshot shortcut");
                ShortcutBinding::default_for(kind, platform)
            })
    }

    #[must_use]
    pub fn keys(&self) -> &str {
        &self.keys
    }

    #[must_use]
    pub fn label(&self, platform: Platform) -> String {
        let (key, modifiers) = self.parts();
        platform
            .modifier_symbols()
            .into_iter()
            .filter(|(name, _)| modifiers.contains(name))
            .map(|(_, symbol)| symbol)
            .chain([key_text(key)])
            .collect::<Vec<_>>()
            .join(platform.separator())
    }

    fn parts(&self) -> (&str, Vec<&str>) {
        let mut tokens = self.keys.split('+').collect::<Vec<_>>();
        let key = tokens.pop().unwrap_or_default();
        (key, tokens)
    }
}

fn key_text(code: &str) -> &str {
    code.strip_prefix("Key")
        .or_else(|| code.strip_prefix("Digit"))
        .unwrap_or(code)
}

fn is_supported_key(code: &str) -> bool {
    is_letter(code) || is_digit(code) || is_function_key(code)
}

fn is_letter(code: &str) -> bool {
    code.strip_prefix("Key")
        .is_some_and(|letter| letter.len() == 1 && letter.chars().all(|c| c.is_ascii_uppercase()))
}

fn is_digit(code: &str) -> bool {
    code.strip_prefix("Digit")
        .is_some_and(|digit| digit.len() == 1 && digit.chars().all(|c| c.is_ascii_digit()))
}

fn is_function_key(code: &str) -> bool {
    (1..=12).any(|number| code == format!("F{number}"))
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Bindings {
    full_screen: ShortcutBinding,
    region: ShortcutBinding,
}

impl Bindings {
    #[must_use]
    pub fn new(full_screen: ShortcutBinding, region: ShortcutBinding) -> Bindings {
        Bindings {
            full_screen,
            region,
        }
    }

    #[must_use]
    pub fn get(&self, kind: ShortcutKind) -> &ShortcutBinding {
        match kind {
            ShortcutKind::FullScreen => &self.full_screen,
            ShortcutKind::Region => &self.region,
        }
    }

    pub fn replacing(
        &self,
        kind: ShortcutKind,
        binding: ShortcutBinding,
    ) -> Result<Bindings, BindingProblem> {
        let other = other_kind(kind);
        if self.get(other) == &binding {
            return Err(BindingProblem::DuplicateOf(other));
        }
        let mut updated = self.clone();
        match kind {
            ShortcutKind::FullScreen => updated.full_screen = binding,
            ShortcutKind::Region => updated.region = binding,
        }
        Ok(updated)
    }
}

fn other_kind(kind: ShortcutKind) -> ShortcutKind {
    match kind {
        ShortcutKind::FullScreen => ShortcutKind::Region,
        ShortcutKind::Region => ShortcutKind::FullScreen,
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "outcome", rename_all = "snake_case")]
pub enum RebindOutcome {
    Accepted { label: String },
    NeedsModifier,
    DuplicateOf { kind: ShortcutKind },
    InUseByAnotherApp,
}

impl From<BindingProblem> for RebindOutcome {
    fn from(problem: BindingProblem) -> RebindOutcome {
        match problem {
            BindingProblem::NeedsModifier => RebindOutcome::NeedsModifier,
            BindingProblem::DuplicateOf(kind) => RebindOutcome::DuplicateOf { kind },
        }
    }
}
