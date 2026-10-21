use tauri::{PhysicalPosition, PhysicalSize};

pub struct ScreenArea {
    pub position: PhysicalPosition<f64>,
    pub size: PhysicalSize<f64>,
}

#[must_use]
pub fn anchored_to_tray(
    tray: &ScreenArea,
    popover: PhysicalSize<f64>,
    monitor: &ScreenArea,
) -> PhysicalPosition<f64> {
    PhysicalPosition::new(
        centred_on_tray(tray, popover.width, monitor),
        beside_tray(tray, popover.height, monitor),
    )
}

fn centred_on_tray(tray: &ScreenArea, popover_width: f64, monitor: &ScreenArea) -> f64 {
    let centred = tray.position.x + tray.size.width / 2.0 - popover_width / 2.0;
    let rightmost = monitor.position.x + monitor.size.width - popover_width;
    centred.min(rightmost).max(monitor.position.x)
}

fn beside_tray(tray: &ScreenArea, popover_height: f64, monitor: &ScreenArea) -> f64 {
    let above = tray.position.y - popover_height;
    if above >= monitor.position.y {
        above
    } else {
        tray.position.y + tray.size.height
    }
}
