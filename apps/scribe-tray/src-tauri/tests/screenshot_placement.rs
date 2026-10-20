use scribe_core::capture::{MonitorPlacement, PlacementUnits};
use scribe_tray::screenshots::placement::centred_in;
use tauri::{LogicalPosition, PhysicalPosition, Position};

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
