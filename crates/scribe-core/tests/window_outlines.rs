use scribe_core::capture::{
    FrameSize, MonitorPlacement, PixelRect, PlacementUnits, WindowCandidate, visible_outlines,
};

const MONITOR_POINTS: MonitorPlacement = MonitorPlacement {
    x: 0,
    y: 0,
    width: 1000,
    height: 600,
    units: PlacementUnits::Points,
};

fn window(left: f64, top: f64, (width, height): (f64, f64)) -> WindowCandidate {
    WindowCandidate {
        left,
        top,
        width,
        height,
        is_app_window: true,
        is_own: false,
    }
}

fn bounds_of(
    candidates: &[WindowCandidate],
    placement: &MonitorPlacement,
    frame: (u32, u32),
) -> Vec<PixelRect> {
    visible_outlines(
        candidates,
        placement,
        FrameSize {
            width: frame.0,
            height: frame.1,
        },
    )
    .iter()
    .map(|outline| outline.bounds)
    .collect()
}

fn rect(x: u32, y: u32, (width, height): (u32, u32)) -> PixelRect {
    PixelRect {
        x,
        y,
        width,
        height,
    }
}

#[test]
fn keeps_windows_in_front_to_back_order() {
    let candidates = [
        window(100.0, 100.0, (200.0, 100.0)),
        window(0.0, 0.0, (500.0, 300.0)),
    ];

    let bounds = bounds_of(&candidates, &MONITOR_POINTS, (1000, 600));

    assert_eq!(bounds, [rect(100, 100, (200, 100)), rect(0, 0, (500, 300))]);
}

#[test]
fn clips_a_window_hanging_off_the_monitor_to_its_visible_part() {
    let candidates = [window(900.0, 500.0, (300.0, 300.0))];

    let bounds = bounds_of(&candidates, &MONITOR_POINTS, (1000, 600));

    assert_eq!(bounds, [rect(900, 500, (100, 100))]);
}

#[test]
fn clips_a_window_starting_before_the_monitor_origin() {
    let candidates = [window(-50.0, -20.0, (150.0, 120.0))];

    let bounds = bounds_of(&candidates, &MONITOR_POINTS, (1000, 600));

    assert_eq!(bounds, [rect(0, 0, (100, 100))]);
}

#[test]
fn scales_points_to_output_pixels_on_a_retina_monitor() {
    let candidates = [window(100.0, 50.0, (200.0, 100.0))];

    let bounds = bounds_of(&candidates, &MONITOR_POINTS, (2000, 1200));

    assert_eq!(bounds, [rect(200, 100, (400, 200))]);
}

#[test]
fn translates_by_the_monitor_origin_on_a_secondary_monitor() {
    let secondary = MonitorPlacement {
        x: 1000,
        ..MONITOR_POINTS
    };
    let candidates = [window(1100.0, 50.0, (200.0, 100.0))];

    let bounds = bounds_of(&candidates, &secondary, (1000, 600));

    assert_eq!(bounds, [rect(100, 50, (200, 100))]);
}

#[test]
fn drops_a_window_wholly_on_another_monitor() {
    let candidates = [
        window(1200.0, 100.0, (300.0, 200.0)),
        window(-400.0, 0.0, (300.0, 200.0)),
    ];

    let bounds = bounds_of(&candidates, &MONITOR_POINTS, (1000, 600));

    assert!(bounds.is_empty());
}

#[test]
fn never_outlines_scribes_own_windows_or_non_app_windows() {
    let own = WindowCandidate {
        is_own: true,
        ..window(0.0, 0.0, (300.0, 300.0))
    };
    let dock = WindowCandidate {
        is_app_window: false,
        ..window(0.0, 500.0, (1000.0, 100.0))
    };
    let ordinary = window(400.0, 100.0, (200.0, 200.0));

    let bounds = bounds_of(&[own, dock, ordinary], &MONITOR_POINTS, (1000, 600));

    assert_eq!(bounds, [rect(400, 100, (200, 200))]);
}
