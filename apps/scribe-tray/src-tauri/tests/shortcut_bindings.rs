use scribe_tray::screenshots::bindings::ShortcutKind;
use tauri_plugin_global_shortcut::{Code, Modifiers, Shortcut};

#[cfg(target_os = "macos")]
#[test]
fn the_full_screen_shortcut_is_control_option_command_1_on_macos() {
    let default = ShortcutKind::FullScreen.default_shortcut();

    let shortcut: Shortcut = default.keys.parse().unwrap();

    assert_eq!(default.label, "⌃⌥⌘1");
    assert_eq!(
        shortcut,
        Shortcut::new(
            Some(Modifiers::CONTROL | Modifiers::ALT | Modifiers::SUPER),
            Code::Digit1
        )
    );
}

#[cfg(windows)]
#[test]
fn the_full_screen_shortcut_is_ctrl_shift_alt_1_on_windows() {
    let default = ShortcutKind::FullScreen.default_shortcut();

    let shortcut: Shortcut = default.keys.parse().unwrap();

    assert_eq!(default.label, "Ctrl+Shift+Alt+1");
    assert_eq!(
        shortcut,
        Shortcut::new(
            Some(Modifiers::CONTROL | Modifiers::SHIFT | Modifiers::ALT),
            Code::Digit1
        )
    );
}

#[cfg(target_os = "macos")]
#[test]
fn the_region_shortcut_is_control_option_command_2_on_macos() {
    let default = ShortcutKind::Region.default_shortcut();

    let shortcut: Shortcut = default.keys.parse().unwrap();

    assert_eq!(default.label, "⌃⌥⌘2");
    assert_eq!(
        shortcut,
        Shortcut::new(
            Some(Modifiers::CONTROL | Modifiers::ALT | Modifiers::SUPER),
            Code::Digit2
        )
    );
}

#[cfg(windows)]
#[test]
fn the_region_shortcut_is_ctrl_shift_alt_2_on_windows() {
    let default = ShortcutKind::Region.default_shortcut();

    let shortcut: Shortcut = default.keys.parse().unwrap();

    assert_eq!(default.label, "Ctrl+Shift+Alt+2");
    assert_eq!(
        shortcut,
        Shortcut::new(
            Some(Modifiers::CONTROL | Modifiers::SHIFT | Modifiers::ALT),
            Code::Digit2
        )
    );
}

#[test]
fn the_two_screenshot_shortcuts_never_share_a_combination() {
    let full_screen = ShortcutKind::FullScreen.default_shortcut();

    let region = ShortcutKind::Region.default_shortcut();

    assert_ne!(full_screen.keys, region.keys);
}
