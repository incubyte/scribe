use scribe_tray::tray_anchor::{ScreenArea, anchored_to_tray};
use tauri::{PhysicalPosition, PhysicalSize};

const POPOVER: PhysicalSize<f64> = PhysicalSize::new(720.0, 880.0);

fn area(origin: (f64, f64), size: (f64, f64)) -> ScreenArea {
    ScreenArea {
        position: PhysicalPosition::new(origin.0, origin.1),
        size: PhysicalSize::new(size.0, size.1),
    }
}

#[test]
fn the_popover_hangs_centred_below_a_menu_bar_icon() {
    let menu_bar_icon = area((2000.0, 0.0), (60.0, 48.0));
    let screen = area((0.0, 0.0), (3024.0, 1964.0));

    let position = anchored_to_tray(&menu_bar_icon, POPOVER, &screen);

    assert_eq!(position, PhysicalPosition::new(1670.0, 48.0));
}

#[test]
fn the_popover_sits_above_a_taskbar_icon_at_the_bottom_of_the_screen() {
    let taskbar_icon = area((1500.0, 1040.0), (40.0, 40.0));
    let screen = area((0.0, 0.0), (1920.0, 1080.0));

    let position = anchored_to_tray(&taskbar_icon, POPOVER, &screen);

    assert_eq!(position, PhysicalPosition::new(1160.0, 160.0));
}

#[test]
fn the_popover_stays_on_screen_for_an_icon_near_the_right_edge() {
    let icon_at_right_edge = area((2980.0, 0.0), (40.0, 48.0));
    let screen = area((0.0, 0.0), (3024.0, 1964.0));

    let position = anchored_to_tray(&icon_at_right_edge, POPOVER, &screen);

    assert_eq!(position, PhysicalPosition::new(2304.0, 48.0));
}

#[test]
fn the_popover_stays_on_screen_for_an_icon_near_the_left_edge() {
    let icon_at_left_edge = area((10.0, 0.0), (40.0, 48.0));
    let screen = area((0.0, 0.0), (3024.0, 1964.0));

    let position = anchored_to_tray(&icon_at_left_edge, POPOVER, &screen);

    assert_eq!(position, PhysicalPosition::new(0.0, 48.0));
}

#[test]
fn the_popover_stays_on_the_screen_that_holds_the_icon() {
    let icon_on_second_screen = area((4900.0, 0.0), (40.0, 48.0));
    let second_screen = area((3024.0, 0.0), (1920.0, 1080.0));

    let position = anchored_to_tray(&icon_on_second_screen, POPOVER, &second_screen);

    assert_eq!(position, PhysicalPosition::new(4224.0, 48.0));
}
