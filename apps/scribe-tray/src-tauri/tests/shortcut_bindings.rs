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
