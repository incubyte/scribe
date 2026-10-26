use std::sync::mpsc;
use std::time::Duration;

use cidre::{cg, sc};
use scribe_core::call::TEAMS_BUNDLE_ID_PREFIX;
use scribe_core::ports::MeetingWindowTitles;

const READ_TIMEOUT: Duration = Duration::from_secs(2);

#[derive(Default)]
pub struct CaptureTeamsTitles;

impl MeetingWindowTitles for CaptureTeamsTitles {
    fn teams_window_titles(&self) -> Vec<String> {
        if !cg::screen_capture_access::preflight() {
            return Vec::new();
        }
        let (delivery, delivered) = mpsc::sync_channel(1);
        sc::ShareableContent::current_with_ch(move |content, _error| {
            let titles = content.map(teams_titles_in).unwrap_or_default();
            let _ = delivery.send(titles);
        });
        delivered.recv_timeout(READ_TIMEOUT).unwrap_or_default()
    }
}

fn teams_titles_in(content: &sc::ShareableContent) -> Vec<String> {
    content
        .windows()
        .iter()
        .filter(|window| window.is_on_screen() && is_owned_by_teams(window))
        .filter_map(sc::Window::title)
        .map(|title| title.to_string())
        .filter(|title| !title.is_empty())
        .collect()
}

fn is_owned_by_teams(window: &sc::Window) -> bool {
    window.owning_app().is_some_and(|app| {
        app.bundle_id()
            .to_string()
            .starts_with(TEAMS_BUNDLE_ID_PREFIX)
    })
}
