#![allow(unsafe_code)]

use std::collections::HashMap;

use scribe_core::ports::{CallProbe, CaptureError};
use wasapi::{
    AudioSessionEnumerator, DeviceCollection, DeviceEnumerator, Direction, SessionState,
    WasapiError, deinitialize, initialize_mta,
};
use windows::Win32::Foundation::{CloseHandle, HANDLE};
use windows::Win32::System::Diagnostics::ToolHelp::{
    CreateToolhelp32Snapshot, PROCESSENTRY32W, Process32FirstW, Process32NextW, TH32CS_SNAPPROCESS,
};

const TEAMS_EXE: &str = "ms-teams.exe";
const MAX_ANCESTRY: usize = 4;
const NO_PROCESS: u32 = 0;
#[allow(clippy::cast_possible_truncation)]
const PROCESSENTRY_SIZE: u32 = size_of::<PROCESSENTRY32W>() as u32;

#[derive(Default)]
pub struct WasapiCallProbe;

impl CallProbe for WasapiCallProbe {
    fn call_app_holds_microphone(&self) -> Result<bool, CaptureError> {
        let capturing_pids = {
            let _com = ComScope::enter()?;
            active_capture_pids()?
        };
        if capturing_pids.is_empty() {
            return Ok(false);
        }
        let processes = ProcessTable::snapshot()?;
        Ok(capturing_pids
            .into_iter()
            .any(|pid| processes.belongs_to_teams(pid)))
    }
}

struct ComScope;

impl ComScope {
    fn enter() -> Result<ComScope, CaptureError> {
        initialize_mta()
            .ok()
            .map_err(|error| CaptureError::StreamFailed(error.to_string()))?;
        Ok(ComScope)
    }
}

impl Drop for ComScope {
    fn drop(&mut self) {
        deinitialize();
    }
}

fn active_capture_pids() -> Result<Vec<u32>, CaptureError> {
    collect_capture_pids().map_err(|error| stream_failed(&error))
}

fn collect_capture_pids() -> Result<Vec<u32>, WasapiError> {
    let devices = DeviceEnumerator::new()?.get_device_collection(&Direction::Capture)?;
    let device_count = devices.get_nbr_devices()?;
    Ok((0..device_count)
        .flat_map(|index| device_session_pids(&devices, index))
        .collect())
}

fn device_session_pids(devices: &DeviceCollection, index: u32) -> Vec<u32> {
    skip_unreadable(
        active_session_pids(devices, index),
        "skipping an unreadable capture device",
    )
    .unwrap_or_default()
}

fn active_session_pids(devices: &DeviceCollection, index: u32) -> Result<Vec<u32>, WasapiError> {
    let sessions = devices
        .get_device_at_index(index)?
        .get_iaudiosessionmanager()?
        .get_audiosessionenumerator()?;
    Ok((0..sessions.get_count()?)
        .filter_map(|session_index| active_session_pid(&sessions, session_index))
        .filter(|pid| *pid != NO_PROCESS)
        .collect())
}

fn active_session_pid(sessions: &AudioSessionEnumerator, index: i32) -> Option<u32> {
    skip_unreadable(
        active_process_id(sessions, index),
        "skipping an unreadable audio session",
    )
    .flatten()
}

fn active_process_id(
    sessions: &AudioSessionEnumerator,
    index: i32,
) -> Result<Option<u32>, WasapiError> {
    let session = sessions.get_session(index)?;
    if session.get_state()? == SessionState::Active {
        return session.get_process_id().map(Some);
    }
    Ok(None)
}

fn skip_unreadable<T>(outcome: Result<T, WasapiError>, message: &str) -> Option<T> {
    outcome
        .inspect_err(|error| tracing::debug!(%error, "{message}"))
        .ok()
}

struct ProcessTable(HashMap<u32, (u32, String)>);

impl ProcessTable {
    fn snapshot() -> Result<ProcessTable, CaptureError> {
        let snapshot = SnapshotHandle::take()?;
        Ok(ProcessTable(snapshot.processes()))
    }

    fn belongs_to_teams(&self, pid: u32) -> bool {
        let mut current = pid;
        for _ in 0..=MAX_ANCESTRY {
            let Some((parent, exe_name)) = self.0.get(&current) else {
                return false;
            };
            if exe_name == TEAMS_EXE {
                return true;
            }
            current = *parent;
        }
        false
    }
}

struct SnapshotHandle(HANDLE);

impl SnapshotHandle {
    fn take() -> Result<SnapshotHandle, CaptureError> {
        unsafe { CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0) }
            .map(SnapshotHandle)
            .map_err(|error| CaptureError::StreamFailed(error.to_string()))
    }

    fn processes(&self) -> HashMap<u32, (u32, String)> {
        let mut entry = PROCESSENTRY32W {
            dwSize: PROCESSENTRY_SIZE,
            ..Default::default()
        };
        let mut processes = HashMap::new();
        let mut advanced = unsafe { Process32FirstW(self.0, &raw mut entry) };
        while advanced.is_ok() {
            processes.insert(
                entry.th32ProcessID,
                (entry.th32ParentProcessID, exe_name_of(&entry)),
            );
            advanced = unsafe { Process32NextW(self.0, &raw mut entry) };
        }
        processes
    }
}

impl Drop for SnapshotHandle {
    fn drop(&mut self) {
        let _ = unsafe { CloseHandle(self.0) };
    }
}

fn exe_name_of(entry: &PROCESSENTRY32W) -> String {
    let length = entry
        .szExeFile
        .iter()
        .position(|unit| *unit == 0)
        .unwrap_or(entry.szExeFile.len());
    String::from_utf16_lossy(&entry.szExeFile[..length]).to_lowercase()
}

fn stream_failed(error: &WasapiError) -> CaptureError {
    CaptureError::StreamFailed(error.to_string())
}
