#![allow(unsafe_code)]

use scribe_core::ports::MeetingWindowTitles;
use windows::Win32::Foundation::{HWND, LPARAM};
use windows::Win32::UI::WindowsAndMessaging::{
    EnumWindows, GetWindowTextW, GetWindowThreadProcessId, IsWindowVisible,
};
use windows::core::BOOL;

use crate::call_probe::ProcessTable;

const MAX_TITLE_UNITS: usize = 512;

#[derive(Default)]
pub struct TeamsWindowTitles;

impl MeetingWindowTitles for TeamsWindowTitles {
    fn teams_window_titles(&self) -> Vec<String> {
        let Ok(processes) = ProcessTable::snapshot() else {
            return Vec::new();
        };
        visible_windows()
            .into_iter()
            .filter(|window| processes.belongs_to_teams(owner_pid(*window)))
            .map(title_of)
            .filter(|title| !title.is_empty())
            .collect()
    }
}

fn visible_windows() -> Vec<HWND> {
    let mut windows: Vec<HWND> = Vec::new();
    let collector = LPARAM(&raw mut windows as isize);
    let _ = unsafe { EnumWindows(Some(collect_if_visible), collector) };
    windows
}

unsafe extern "system" fn collect_if_visible(window: HWND, collector: LPARAM) -> BOOL {
    if unsafe { IsWindowVisible(window) }.as_bool() {
        let windows = unsafe { &mut *(collector.0 as *mut Vec<HWND>) };
        windows.push(window);
    }
    BOOL::from(true)
}

fn owner_pid(window: HWND) -> u32 {
    let mut pid = 0;
    unsafe { GetWindowThreadProcessId(window, Some(&raw mut pid)) };
    pid
}

fn title_of(window: HWND) -> String {
    let mut buffer = [0u16; MAX_TITLE_UNITS];
    let copied = unsafe { GetWindowTextW(window, &mut buffer) };
    let length = usize::try_from(copied).unwrap_or_default();
    String::from_utf16_lossy(&buffer[..length])
}
