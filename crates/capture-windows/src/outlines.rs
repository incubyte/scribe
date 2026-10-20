use scribe_core::capture::WindowCandidate;
use xcap::Window;

pub fn window_candidates(own_pid: u32) -> Vec<WindowCandidate> {
    let mut windows: Vec<(i32, WindowCandidate)> = Window::all()
        .unwrap_or_default()
        .iter()
        .filter_map(|window| candidate_of(window, own_pid))
        .collect();
    windows.sort_by_key(|(depth, _)| std::cmp::Reverse(*depth));
    windows
        .into_iter()
        .map(|(_, candidate)| candidate)
        .collect()
}

fn candidate_of(window: &Window, own_pid: u32) -> Option<(i32, WindowCandidate)> {
    if window.is_minimized().ok()? {
        return None;
    }
    let candidate = WindowCandidate {
        left: f64::from(window.x().ok()?),
        top: f64::from(window.y().ok()?),
        width: f64::from(window.width().ok()?),
        height: f64::from(window.height().ok()?),
        is_app_window: true,
        is_own: window.pid().ok()? == own_pid,
    };
    Some((window.z().ok()?, candidate))
}
