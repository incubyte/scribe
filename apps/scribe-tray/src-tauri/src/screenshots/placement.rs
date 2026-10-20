use scribe_core::capture::{MonitorPlacement, PlacementUnits};
use tauri::{LogicalPosition, LogicalSize, PhysicalPosition, PhysicalSize, Position, Rect, Size};

#[must_use]
pub fn centred_in(placement: &MonitorPlacement, size: (f64, f64)) -> Position {
    let left = f64::from(placement.x) + (f64::from(placement.width) - size.0) / 2.0;
    let top = f64::from(placement.y) + (f64::from(placement.height) - size.1) / 2.0;
    match placement.units {
        PlacementUnits::Points => Position::Logical(LogicalPosition::new(left, top)),
        PlacementUnits::Pixels => {
            Position::Physical(PhysicalPosition::new(to_pixel(left), to_pixel(top)))
        }
    }
}

#[must_use]
pub fn covering(placement: &MonitorPlacement) -> Rect {
    match placement.units {
        PlacementUnits::Points => Rect {
            position: Position::Logical(LogicalPosition::new(
                f64::from(placement.x),
                f64::from(placement.y),
            )),
            size: Size::Logical(LogicalSize::new(
                f64::from(placement.width),
                f64::from(placement.height),
            )),
        },
        PlacementUnits::Pixels => Rect {
            position: Position::Physical(PhysicalPosition::new(placement.x, placement.y)),
            size: Size::Physical(PhysicalSize::new(placement.width, placement.height)),
        },
    }
}

#[allow(clippy::cast_possible_truncation)]
fn to_pixel(value: f64) -> i32 {
    value.round() as i32
}
