use std::time::Instant;

use chrono::{DateTime, FixedOffset, Local};
use scribe_core::calendar::{
    MatchAnchor, Meeting, SwitchInputs, TodaysAgenda, meeting_candidates, meeting_named_in_titles,
};
use tauri::{AppHandle, Manager, Runtime};

use super::armed::ArmedStore;
use super::selection::{AutoStart, MeetingSelection};
use super::session_link;
use super::switch_state::{SwitchPromptView, SwitchState};
use super::titles;
use crate::popover;
use crate::recording::{self, RecordingController};
use crate::webview;

const SWITCH_PROMPT_EVENT: &str = "meeting://switch-prompt";

pub fn on_poll<R: Runtime>(app: &AppHandle<R>, in_call: bool) {
    let Some(state) = app.try_state::<SwitchState>() else {
        return;
    };
    let Some(linked) = linked_meeting_in_call(app, in_call) else {
        if state.withdraw() {
            publish(app, None);
        }
        return;
    };
    if state.is_open() {
        return;
    }
    if let Some(view) = prompt_for_next(app, &state, &linked) {
        publish(app, Some(view));
        popover::open_for_call(app);
    }
}

fn linked_meeting_in_call<R: Runtime>(app: &AppHandle<R>, in_call: bool) -> Option<Meeting> {
    if !in_call || !recording::is_recording(app) {
        return None;
    }
    app.try_state::<MeetingSelection>()?.active_meeting()
}

fn prompt_for_next<R: Runtime>(
    app: &AppHandle<R>,
    state: &SwitchState,
    linked: &Meeting,
) -> Option<SwitchPromptView> {
    let agenda = session_link::connected_agenda(app)?;
    let now = Local::now().fixed_offset();
    let titled = named_in_titles(state, &agenda, now);
    let armed = app.try_state::<ArmedStore>()?.armed();
    state.offer(&SwitchInputs {
        linked,
        agenda: &agenda,
        armed: &armed,
        titled: titled.as_ref(),
        now,
    })
}

fn named_in_titles(
    state: &SwitchState,
    agenda: &TodaysAgenda,
    now: DateTime<FixedOffset>,
) -> Option<Meeting> {
    if !state.title_recheck_due(Instant::now()) {
        return None;
    }
    let titles = titles::teams_titles().teams_window_titles();
    let candidates = meeting_candidates(agenda, &MatchAnchor::new(now));
    meeting_named_in_titles(&candidates, &titles).cloned()
}

pub fn switch<R: Runtime>(app: &AppHandle<R>) {
    let Some(next) = app.state::<SwitchState>().take_for_switch() else {
        return;
    };
    publish(app, None);
    let start = app
        .try_state::<ArmedStore>()
        .map_or(AutoStart::No, |armed| armed.start_for(&next.event_id));
    app.state::<RecordingController>().stop();
    if recording::start_for_meeting(app, &next, start).is_err() {
        tracing::warn!("could not start recording the next meeting after switching");
    }
    app.state::<SwitchState>().finish_switch();
}

pub fn keep<R: Runtime>(app: &AppHandle<R>) {
    if app.state::<SwitchState>().withdraw() {
        publish(app, None);
    }
}

pub fn current<R: Runtime>(app: &AppHandle<R>) -> Option<SwitchPromptView> {
    app.state::<SwitchState>().open_view()
}

fn publish<R: Runtime>(app: &AppHandle<R>, view: Option<SwitchPromptView>) {
    webview::emit(app, SWITCH_PROMPT_EVENT, view);
}
