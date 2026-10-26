use scribe_core::ports::MeetingWindowTitles;

#[cfg(target_os = "macos")]
mod macos {
    use capture_macos::CaptureTeamsTitles;
    use scribe_core::ports::MeetingWindowTitles;

    pub fn teams_titles() -> Box<dyn MeetingWindowTitles> {
        Box::new(CaptureTeamsTitles)
    }
}

#[cfg(target_os = "macos")]
use macos::teams_titles as platform_teams_titles;

#[cfg(windows)]
mod windows {
    use audio_windows::TeamsWindowTitles;
    use scribe_core::ports::MeetingWindowTitles;

    pub fn teams_titles() -> Box<dyn MeetingWindowTitles> {
        Box::new(TeamsWindowTitles)
    }
}

#[cfg(windows)]
use windows::teams_titles as platform_teams_titles;

#[cfg(not(any(target_os = "macos", windows)))]
mod unsupported {
    use scribe_core::ports::MeetingWindowTitles;

    struct NoTeamsTitles;

    impl MeetingWindowTitles for NoTeamsTitles {
        fn teams_window_titles(&self) -> Vec<String> {
            Vec::new()
        }
    }

    pub fn teams_titles() -> Box<dyn MeetingWindowTitles> {
        Box::new(NoTeamsTitles)
    }
}

#[cfg(not(any(target_os = "macos", windows)))]
use unsupported::teams_titles as platform_teams_titles;

pub fn teams_titles() -> Box<dyn MeetingWindowTitles> {
    platform_teams_titles()
}
