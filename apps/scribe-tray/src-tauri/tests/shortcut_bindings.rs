use scribe_tray::screenshots::bindings::{
    BindingProblem, Bindings, Platform, PressedKeys, RebindOutcome, ShortcutBinding, ShortcutKind,
};
use serde_json::{Value, json};
use std::sync::{Arc, Mutex};
use tauri_plugin_global_shortcut::{Code, Modifiers, Shortcut};
use tracing_subscriber::fmt::MakeWriter;

fn pressed(code: &str) -> PressedKeys {
    PressedKeys {
        code: code.to_owned(),
        control: false,
        alt: false,
        shift: false,
        meta: false,
    }
}

fn with_control(code: &str) -> PressedKeys {
    PressedKeys {
        control: true,
        ..pressed(code)
    }
}

fn default_for(kind: ShortcutKind, platform: Platform) -> ShortcutBinding {
    ShortcutBinding::default_for(kind, platform)
}

fn default_bindings(platform: Platform) -> Bindings {
    Bindings::new(
        default_for(ShortcutKind::FullScreen, platform),
        default_for(ShortcutKind::Region, platform),
    )
}

#[test]
fn the_macos_defaults_are_control_option_command_1_and_2() {
    let full_screen = default_for(ShortcutKind::FullScreen, Platform::MacOs);
    let region = default_for(ShortcutKind::Region, Platform::MacOs);

    assert_eq!(full_screen.label(Platform::MacOs), "⌃⌥⌘1");
    assert_eq!(region.label(Platform::MacOs), "⌃⌥⌘2");
    assert_eq!(
        region.keys().parse::<Shortcut>().unwrap(),
        Shortcut::new(
            Some(Modifiers::CONTROL | Modifiers::ALT | Modifiers::SUPER),
            Code::Digit2
        )
    );
}

#[test]
fn the_windows_defaults_are_ctrl_shift_alt_1_and_2() {
    let full_screen = default_for(ShortcutKind::FullScreen, Platform::Windows);
    let region = default_for(ShortcutKind::Region, Platform::Windows);

    assert_eq!(full_screen.label(Platform::Windows), "Ctrl+Shift+Alt+1");
    assert_eq!(region.label(Platform::Windows), "Ctrl+Shift+Alt+2");
    assert_eq!(
        region.keys().parse::<Shortcut>().unwrap(),
        Shortcut::new(
            Some(Modifiers::CONTROL | Modifiers::SHIFT | Modifiers::ALT),
            Code::Digit2
        )
    );
}

#[test]
fn the_two_defaults_never_share_a_combination_on_any_platform() {
    for platform in [Platform::MacOs, Platform::Windows] {
        let bindings = default_bindings(platform);

        assert_ne!(
            bindings.get(ShortcutKind::FullScreen),
            bindings.get(ShortcutKind::Region)
        );
    }
}

#[test]
fn a_pressed_combination_is_stored_in_canonical_order_and_labelled_per_platform() {
    let pressed_keys = PressedKeys {
        control: true,
        alt: true,
        shift: true,
        meta: true,
        ..pressed("KeyR")
    };

    let binding = ShortcutBinding::from_pressed(&pressed_keys).unwrap();

    assert_eq!(binding.keys(), "shift+control+alt+super+KeyR");
    assert_eq!(binding.label(Platform::MacOs), "⌃⌥⇧⌘R");
    assert_eq!(binding.label(Platform::Windows), "Ctrl+Shift+Alt+Win+R");
}

#[test]
fn a_function_key_is_labelled_as_itself() {
    let binding = ShortcutBinding::from_pressed(&with_control("F5")).unwrap();

    assert_eq!(binding.label(Platform::Windows), "Ctrl+F5");
}

#[test]
fn a_combination_needs_control_alt_or_command() {
    let shift_only = PressedKeys {
        shift: true,
        ..pressed("KeyR")
    };

    assert_eq!(
        ShortcutBinding::from_pressed(&shift_only),
        Err(BindingProblem::NeedsModifier)
    );
    assert_eq!(
        ShortcutBinding::from_pressed(&pressed("KeyR")),
        Err(BindingProblem::NeedsModifier)
    );
}

#[test]
fn a_combination_needs_a_letter_digit_or_function_key() {
    for code in ["Enter", "Space", "ArrowUp", "F13", "Numpad1", "Comma"] {
        assert_eq!(
            ShortcutBinding::from_pressed(&with_control(code)),
            Err(BindingProblem::NeedsModifier),
            "{code} should not be accepted"
        );
    }
}

#[test]
fn a_combination_bound_to_the_other_row_is_a_duplicate_of_it() {
    let bindings = default_bindings(Platform::MacOs);
    let region_keys = bindings.get(ShortcutKind::Region).clone();

    let result = bindings.replacing(ShortcutKind::FullScreen, region_keys);

    assert_eq!(
        result,
        Err(BindingProblem::DuplicateOf(ShortcutKind::Region))
    );
}

#[test]
fn replacing_a_binding_leaves_the_other_row_untouched() {
    let bindings = default_bindings(Platform::MacOs);
    let new_binding = ShortcutBinding::from_pressed(&with_control("KeyR")).unwrap();

    let updated = bindings
        .replacing(ShortcutKind::Region, new_binding.clone())
        .unwrap();

    assert_eq!(updated.get(ShortcutKind::Region), &new_binding);
    assert_eq!(
        updated.get(ShortcutKind::FullScreen),
        bindings.get(ShortcutKind::FullScreen)
    );
}

#[test]
fn a_saved_shortcut_is_restored_after_a_restart() {
    let original = ShortcutBinding::from_pressed(&PressedKeys {
        alt: true,
        shift: true,
        ..pressed("Digit7")
    })
    .unwrap();

    let restored = ShortcutBinding::from_stored(
        ShortcutKind::Region,
        Some(&json!(original.keys())),
        Platform::MacOs,
    );

    assert_eq!(restored, original);
}

fn stored_region(raw: Option<&Value>) -> ShortcutBinding {
    ShortcutBinding::from_stored(ShortcutKind::Region, raw, Platform::Windows)
}

fn region_default() -> ShortcutBinding {
    default_for(ShortcutKind::Region, Platform::Windows)
}

fn warnings_logged_by(action: impl FnOnce()) -> String {
    let written = CapturedLog::default();
    let subscriber = tracing_subscriber::fmt()
        .with_writer(written.clone())
        .with_ansi(false)
        .finish();
    tracing::subscriber::with_default(subscriber, action);
    written.text()
}

#[derive(Clone, Default)]
struct CapturedLog(Arc<Mutex<Vec<u8>>>);

impl CapturedLog {
    fn text(&self) -> String {
        String::from_utf8(self.0.lock().unwrap().clone()).unwrap()
    }
}

impl std::io::Write for CapturedLog {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        self.0.lock().unwrap().extend_from_slice(bytes);
        Ok(bytes.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

impl<'a> MakeWriter<'a> for CapturedLog {
    type Writer = CapturedLog;

    fn make_writer(&'a self) -> CapturedLog {
        self.clone()
    }
}

#[test]
fn a_missing_stored_value_falls_back_to_the_default_without_a_warning() {
    let mut binding = None;

    let log = warnings_logged_by(|| binding = Some(stored_region(None)));

    assert_eq!(binding, Some(region_default()));
    assert_eq!(log, "");
}

fn assert_falls_back_with_a_warning(raw: &Value) {
    let mut binding = None;

    let log = warnings_logged_by(|| binding = Some(stored_region(Some(raw))));

    assert_eq!(binding, Some(region_default()));
    assert!(log.contains("WARN"));
}

#[test]
fn a_stored_shortcut_that_is_not_a_key_combination_falls_back_to_the_default_with_a_warning() {
    for raw in [json!(""), json!("not a shortcut"), json!("control+")] {
        assert_falls_back_with_a_warning(&raw);
    }
}

#[test]
fn a_stored_shortcut_that_is_not_text_falls_back_to_the_default_with_a_warning() {
    for raw in [json!(42), json!(null)] {
        assert_falls_back_with_a_warning(&raw);
    }
}

#[test]
fn a_stored_shortcut_without_control_alt_or_command_falls_back_to_the_default_with_a_warning() {
    assert_falls_back_with_a_warning(&json!("shift+KeyR"));
}

#[test]
fn a_stored_shortcut_on_an_unsupported_key_falls_back_to_the_default_with_a_warning() {
    for raw in [json!("control+Enter"), json!("control+F13")] {
        assert_falls_back_with_a_warning(&raw);
    }
}

#[test]
fn a_stored_alias_is_read_and_kept_in_canonical_form() {
    let binding = stored_region(Some(&json!("ctrl+option+r")));

    assert_eq!(binding.keys(), "control+alt+KeyR");
}

#[test]
fn a_stored_region_shortcut_that_duplicates_the_full_screen_one_falls_back_to_its_default() {
    let shared = json!("control+alt+KeyR");

    let bindings = Bindings::from_stored(Some(&shared), Some(&shared), Platform::Windows);

    assert_eq!(
        bindings.get(ShortcutKind::FullScreen).keys(),
        "control+alt+KeyR"
    );
    assert_eq!(bindings.get(ShortcutKind::Region), &region_default());
}

#[test]
fn when_the_stored_full_screen_shortcut_is_the_region_default_both_fall_back_to_their_defaults() {
    let region_default_keys = json!(region_default().keys());
    let mut bindings = None;

    let log = warnings_logged_by(|| {
        bindings = Some(Bindings::from_stored(
            Some(&region_default_keys),
            Some(&json!("not a shortcut")),
            Platform::Windows,
        ));
    });

    assert_eq!(bindings, Some(default_bindings(Platform::Windows)));
    assert!(log.contains("full screen shortcut"));
}

#[test]
fn settings_is_told_whether_a_combination_was_accepted_or_why_it_was_refused() {
    let accepted = serde_json::to_value(RebindOutcome::Accepted {
        label: "⌃R".to_owned(),
    })
    .unwrap();
    let duplicate = serde_json::to_value(RebindOutcome::refused(
        BindingProblem::DuplicateOf(ShortcutKind::FullScreen),
        Platform::MacOs,
    ))
    .unwrap();

    assert_eq!(
        accepted,
        serde_json::json!({ "outcome": "accepted", "label": "⌃R" })
    );
    assert_eq!(
        duplicate,
        serde_json::json!({ "outcome": "duplicate_of", "kind": "full_screen" })
    );
}

#[test]
fn a_combination_without_a_modifier_is_refused_in_the_platforms_own_words() {
    let on_mac = RebindOutcome::refused(BindingProblem::NeedsModifier, Platform::MacOs);
    let on_windows = RebindOutcome::refused(BindingProblem::NeedsModifier, Platform::Windows);

    assert_eq!(
        serde_json::to_value(on_mac).unwrap(),
        json!({
            "outcome": "needs_modifier",
            "message": "Use ⌘, ⌃ or ⌥ with a letter, number or function key"
        })
    );
    assert_eq!(
        serde_json::to_value(on_windows).unwrap(),
        json!({
            "outcome": "needs_modifier",
            "message": "Use Ctrl or Alt with a letter, number or function key"
        })
    );
}

#[test]
fn settings_understands_a_key_press_as_the_physical_key_and_the_held_modifiers() {
    let pressed_keys: PressedKeys = serde_json::from_value(serde_json::json!({
        "code": "Digit2",
        "control": true,
        "alt": true,
        "shift": false,
        "meta": false
    }))
    .unwrap();

    assert_eq!(
        pressed_keys,
        PressedKeys {
            control: true,
            alt: true,
            ..pressed("Digit2")
        }
    );
}
