use scribe_core::capture::{MonitorPlacement, PlacementUnits};
use tauri::{LogicalPosition, PhysicalPosition, Position};

#[must_use]
pub fn centred_in(placement: &MonitorPlacement, size: (f64, f64)) -> Position {
    let left = f64::from(placement.x) + (f64::from(placement.width) - size.0) / 2.0;
    let top = f64::from(placement.y) + (f64::from(placement.height) - size.1) / 2.0;
    match placement.units {
        PlacementUnits::Points => Position::Logical(LogicalPosition::new(left, top)),
        PlacementUnits::Pixels => Position::Physical(PhysicalPosition::new(
            left.round() as i32,
            top.round() as i32,
        )),
    }
}
