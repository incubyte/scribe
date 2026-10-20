use scribe_tray::screenshots::bindings::{
    BindingProblem, Bindings, Platform, PressedKeys, RebindOutcome, ShortcutBinding, ShortcutKind,
};
use tauri_plugin_global_shortcut::{Code, Modifiers, Shortcut};

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
fn a_stored_combination_round_trips() {
    let original = ShortcutBinding::from_pressed(&PressedKeys {
        alt: true,
        shift: true,
        ..pressed("Digit7")
    })
    .unwrap();

    let restored =
        ShortcutBinding::from_stored(ShortcutKind::Region, Some(original.keys()), Platform::MacOs);

    assert_eq!(restored, original);
}

#[test]
fn a_missing_empty_or_unparseable_stored_value_falls_back_to_the_default() {
    for raw in [None, Some(""), Some("not a shortcut"), Some("control+")] {
        let binding = ShortcutBinding::from_stored(ShortcutKind::Region, raw, Platform::Windows);

        assert_eq!(
            binding,
            default_for(ShortcutKind::Region, Platform::Windows),
            "{raw:?} should fall back"
        );
    }
}

#[test]
fn a_rebind_outcome_is_tagged_for_the_webview() {
    let accepted = serde_json::to_value(RebindOutcome::Accepted {
        label: "⌃R".to_owned(),
    })
    .unwrap();
    let duplicate = serde_json::to_value(RebindOutcome::from(BindingProblem::DuplicateOf(
        ShortcutKind::FullScreen,
    )))
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
fn pressed_keys_arrive_from_the_webview_as_code_and_modifier_flags() {
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
