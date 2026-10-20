use scribe_core::capture::{MonitorPlacement, PlacementUnits};
use scribe_tray::screenshots::placement::{centred_in, covering};
use tauri::{LogicalPosition, LogicalSize, PhysicalPosition, PhysicalSize, Position, Size};

fn monitor(units: PlacementUnits) -> MonitorPlacement {
    MonitorPlacement {
        x: 1440,
        y: 100,
        width: 1000,
        height: 800,
        units,
    }
}

#[test]
fn a_window_is_centred_in_a_monitor_measured_in_points() {
    let position = centred_in(&monitor(PlacementUnits::Points), (400.0, 200.0));

    assert_eq!(
        position,
        Position::Logical(LogicalPosition::new(1740.0, 400.0))
    );
}

#[test]
fn a_window_is_centred_in_a_monitor_measured_in_pixels() {
    let position = centred_in(&monitor(PlacementUnits::Pixels), (400.0, 200.0));

    assert_eq!(
        position,
        Position::Physical(PhysicalPosition::new(1740, 400))
    );
}

#[test]
fn a_window_covers_exactly_a_monitor_measured_in_points() {
    let bounds = covering(&monitor(PlacementUnits::Points));

    assert_eq!(
        bounds.position,
        Position::Logical(LogicalPosition::new(1440.0, 100.0))
    );
    assert_eq!(bounds.size, Size::Logical(LogicalSize::new(1000.0, 800.0)));
}

#[test]
fn a_window_covers_exactly_a_monitor_measured_in_pixels() {
    let bounds = covering(&monitor(PlacementUnits::Pixels));

    assert_eq!(
        bounds.position,
        Position::Physical(PhysicalPosition::new(1440, 100))
    );
    assert_eq!(bounds.size, Size::Physical(PhysicalSize::new(1000, 800)));
}
