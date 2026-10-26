use std::thread;

use chrono::Local;
use scribe_core::calendar::{
    ArmedAction, ArmedEntry, ArmedMeetings, EventId, Meeting, TodaysAgenda, WatchInputs,
};
use store_markdown::SETTINGS_FILE_NAME;
use tauri::{AppHandle, Manager, Runtime};
use tauri_plugin_store::StoreExt;

use super::agenda_cache::AgendaCache;
use super::agenda_refresh;
use super::agenda_view::AgendaView;
use super::armed::{
    ARMED_MEETINGS_KEY, ArmedStore, CallStartPopover, entries_from, popover_on_call_start,
    settings_value,
};
use super::notify;
use super::selection::{AutoStart, FinishedChoice, MeetingSelection};
use super::session_link;
use super::switch_state::SwitchState;
use crate::recording::{self, RecordingController};

const RECORD_THREAD: &str = "record-meeting";
const AUTO_START_THREAD: &str = "auto-start-meeting";
const AUTO_STOP_THREAD: &str = "auto-stop-meeting";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArmError {
    NotOnTodaysAgenda,
}

impl ArmError {
    #[must_use]
    pub fn reason(self) -> &'static str {
        match self {
            ArmError::NotOnTodaysAgenda => "that meeting is not on today's agenda",
        }
    }
}

pub fn install<R: Runtime>(app: &AppHandle<R>) {
    let saved = saved_entries(app);
    let saved_count = saved.len();
    let armed = ArmedMeetings::for_day(saved, Local::now().date_naive());
    let pruned = armed.entries().len() != saved_count;
    app.manage(ArmedStore::new(armed));
    if pruned {
        save(app);
    }
    resume_after_wake(app);
}

fn saved_entries<R: Runtime>(app: &AppHandle<R>) -> Vec<ArmedEntry> {
    let saved = app
        .store(SETTINGS_FILE_NAME)
        .ok()
        .and_then(|store| store.get(ARMED_MEETINGS_KEY));
    entries_from(saved.as_ref())
}

fn save<R: Runtime>(app: &AppHandle<R>) {
    let value = settings_value(&app.state::<ArmedStore>().armed());
    let saved = app.store(SETTINGS_FILE_NAME).and_then(|store| {
        store.set(ARMED_MEETINGS_KEY, value);
        store.save()
    });
    if let Err(error) = saved {
        tracing::warn!(%error, "could not save the meetings set to record");
    }
}

#[cfg(any(target_os = "macos", windows))]
fn resume_after_wake<R: Runtime>(app: &AppHandle<R>) {
    let app = app.clone();
    crate::power::on_did_wake(move || app.state::<ArmedStore>().resume_interrupted());
}

#[cfg(not(any(target_os = "macos", windows)))]
fn resume_after_wake<R: Runtime>(_app: &AppHandle<R>) {}

pub fn agenda_view<R: Runtime>(app: &AppHandle<R>) -> AgendaView {
    app.state::<ArmedStore>()
        .mark(app.state::<AgendaCache>().view())
}

pub fn arm<R: Runtime>(app: &AppHandle<R>, id: EventId) -> Result<(), ArmError> {
    todays_meeting(app, &id).ok_or(ArmError::NotOnTodaysAgenda)?;
    app.state::<ArmedStore>().arm(id, Local::now().date_naive());
    save(app);
    publish_agenda(app);
    Ok(())
}

pub fn disarm<R: Runtime>(app: &AppHandle<R>, id: &EventId) {
    app.state::<ArmedStore>().disarm(id);
    save(app);
    publish_agenda(app);
}

pub fn record<R: Runtime>(app: &AppHandle<R>, id: &EventId) {
    let Some(meeting) = todays_meeting(app, id) else {
        tracing::warn!("the meeting to record is no longer on today's agenda");
        return;
    };
    let handle = app.clone();
    run_on_thread(RECORD_THREAD, move || {
        let _ = recording::start_for_meeting(&handle, &meeting, AutoStart::No);
    });
}

fn run_on_thread(name: &str, job: impl FnOnce() + Send + 'static) {
    let spawned = thread::Builder::new().name(name.to_owned()).spawn(job);
    if let Err(error) = spawned {
        tracing::error!(%error, thread = name, "could not start a background thread");
    }
}

fn todays_meeting<R: Runtime>(app: &AppHandle<R>, id: &EventId) -> Option<Meeting> {
    let agenda = app.state::<AgendaCache>().agenda()?;
    agenda
        .meetings()
        .iter()
        .find(|meeting| &meeting.event_id == id)
        .cloned()
}

pub fn on_poll<R: Runtime>(app: &AppHandle<R>, in_call: bool) -> CallStartPopover {
    let Some(store) = app.try_state::<ArmedStore>() else {
        return CallStartPopover::Open;
    };
    if store.prune_for(Local::now().date_naive()) {
        save(app);
    }
    let due = take_due_actions(app, &store, in_call);
    let popover = popover_on_call_start(&due);
    for action in due {
        perform(app, action);
    }
    popover
}

fn take_due_actions<R: Runtime>(
    app: &AppHandle<R>,
    store: &ArmedStore,
    in_call: bool,
) -> Vec<ArmedAction> {
    let armed = store.armed();
    let agenda = session_link::connected_agenda(app).unwrap_or_default();
    let matched = (in_call && !armed.is_empty())
        .then(|| meeting_to_record_by_itself(app, &agenda))
        .flatten();
    store.take_due_actions(&WatchInputs {
        agenda: &agenda,
        armed: &armed,
        matched: matched.as_ref(),
        in_call,
        recording: recording::is_recording(app),
        now: Local::now().fixed_offset(),
    })
}

fn meeting_to_record_by_itself<R: Runtime>(
    app: &AppHandle<R>,
    agenda: &TodaysAgenda,
) -> Option<Meeting> {
    if is_switching(app) {
        return None;
    }
    let calendar_match = || session_link::call_meeting_now(app, agenda);
    match app.try_state::<MeetingSelection>() {
        Some(selection) => selection.meeting_for_the_call(calendar_match),
        None => calendar_match(),
    }
}

fn is_switching<R: Runtime>(app: &AppHandle<R>) -> bool {
    app.try_state::<SwitchState>()
        .is_some_and(|state| state.is_switching())
}

fn perform<R: Runtime>(app: &AppHandle<R>, action: ArmedAction) {
    match action {
        ArmedAction::StartRecording(meeting) => {
            let handle = app.clone();
            run_on_thread(AUTO_START_THREAD, move || {
                start_by_itself(&handle, &meeting);
            });
        }
        ArmedAction::ReportMissed(meeting) => {
            notify::notify_meeting_missed(app, &meeting.subject);
            publish_agenda(app);
        }
    }
}

fn start_by_itself<R: Runtime>(app: &AppHandle<R>, meeting: &Meeting) {
    tracing::info!("an armed meeting's call is in progress; recording it");
    if recording::start_for_meeting(app, meeting, AutoStart::Yes).is_ok() {
        notify::notify_meeting_recording(app, &meeting.subject);
    } else {
        app.state::<ArmedStore>()
            .note_start_failed(&meeting.event_id);
        publish_agenda(app);
    }
}

pub fn stop_if_auto_started<R: Runtime>(app: &AppHandle<R>) {
    let auto_started = app
        .try_state::<MeetingSelection>()
        .and_then(|selection| selection.auto_started_session());
    let Some(session) = auto_started else {
        return;
    };
    let handle = app.clone();
    run_on_thread(AUTO_STOP_THREAD, move || {
        tracing::info!("the call ended; stopping the recording that started with it");
        handle
            .state::<RecordingController>()
            .stop_if_recording(&session);
    });
}

pub fn note_meeting_finished<R: Runtime>(app: &AppHandle<R>, finished: &FinishedChoice) {
    if let Some(store) = app.try_state::<ArmedStore>() {
        store.note_recorded(&finished.meeting.event_id);
    }
    if finished.start == AutoStart::Yes {
        notify::notify_meeting_saved(app, &finished.meeting.subject);
    }
}

pub fn note_going_to_sleep<R: Runtime>(app: &AppHandle<R>) {
    let recording = app
        .try_state::<MeetingSelection>()
        .and_then(|selection| selection.active_meeting());
    if let (Some(meeting), Some(store)) = (recording, app.try_state::<ArmedStore>()) {
        store.remember_interrupted(meeting.event_id);
    }
}

fn publish_agenda<R: Runtime>(app: &AppHandle<R>) {
    agenda_refresh::publish(app, &app.state::<AgendaCache>().view());
}
