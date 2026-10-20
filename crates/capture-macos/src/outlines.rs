use std::collections::HashMap;

use cidre::{cg, sc};
use scribe_core::capture::WindowCandidate;

const APP_WINDOW_LAYER: isize = 0;

pub fn window_candidates(content: &sc::ShareableContent, own_pid: i32) -> Vec<WindowCandidate> {
    let windows = content.windows();
    let by_id: HashMap<cg::WindowId, &sc::Window> =
        windows.iter().map(|window| (window.id(), window)).collect();
    front_to_back_ids()
        .into_iter()
        .filter_map(|id| by_id.get(&id).map(|window| candidate_of(window, own_pid)))
        .collect()
}

fn front_to_back_ids() -> Vec<cg::WindowId> {
    let options = cg::WindowListOpt::ON_SCREEN_ONLY | cg::WindowListOpt::EXCLUDE_DESKTOP_ELEMENTS;
    cg::WindowList::new(options, cg::WINDOW_ID_NULL)
        .map(|list| (0..list.len()).map(|index| list.get(index)).collect())
        .unwrap_or_default()
}

fn candidate_of(window: &sc::Window, own_pid: i32) -> WindowCandidate {
    let frame = window.frame();
    WindowCandidate {
        left: frame.origin.x,
        top: frame.origin.y,
        width: frame.size.width,
        height: frame.size.height,
        is_app_window: window.window_layer() == APP_WINDOW_LAYER && window.is_on_screen(),
        is_own: owner_pid(window) == Some(own_pid),
    }
}

fn owner_pid(window: &sc::Window) -> Option<i32> {
    window
        .owning_app()
        .as_deref()
        .map(sc::RunningApp::process_id)
}
