use std::sync::Arc;

use chrono::Local;
use scribe_core::calendar::{
    ArmedMeetings, Connection, MatchAnchor, MatchInputs, MatchOutcome, Meeting, MeetingLink,
    TodaysAgenda, match_meeting,
};
use scribe_core::recording::SessionId;
use store_markdown::MarkdownTranscriptStore;
use tauri::{AppHandle, Manager, Runtime};

use super::agenda_cache::AgendaCache;
use super::armed::ArmedStore;
use super::call_meeting_view::CallMeetingView;
use super::connection;
use super::selection::{CallMatch, FinishedChoice, MeetingSelection, call_match, chosen_meeting};
use super::titles;
use crate::call_watch::CallPresence;
use crate::webview;

const MATCH_EVENT: &str = "meeting://match";

pub fn note_recording_started<R: Runtime>(app: &AppHandle<R>, session: &SessionId) {
    let Some(selection) = app.try_state::<MeetingSelection>() else {
        return;
    };
    selection.begin_from_call(session.clone(), || {
        let clues = call_clues(app);
        chosen_meeting(
            connected_agenda(app)
                .as_ref()
                .map(|agenda| clues.inputs(agenda)),
        )
    });
    if let (Some(meeting), Some(armed)) =
        (selection.active_meeting(), app.try_state::<ArmedStore>())
    {
        armed.note_recording(&meeting.event_id);
    }
    publish_call_meeting(app);
}

pub fn link_finished_session<R: Runtime>(
    app: &AppHandle<R>,
    session: &SessionId,
) -> Option<FinishedChoice> {
    let finished = app.try_state::<MeetingSelection>()?.take_for(session)?;
    let store = app.state::<Arc<MarkdownTranscriptStore>>();
    let link = MeetingLink::from_meeting(&finished.meeting);
    if let Err(error) = store.link_meeting(session, link) {
        tracing::warn!(%error, "could not save the meeting with the transcript");
    }
    Some(finished)
}

pub(super) fn call_meeting_now<R: Runtime>(
    app: &AppHandle<R>,
    agenda: &TodaysAgenda,
) -> Option<Meeting> {
    match match_meeting(&call_clues(app).inputs(agenda)) {
        MatchOutcome::Matched(meeting) => Some(meeting),
        MatchOutcome::Ambiguous(_) | MatchOutcome::Unmatched => None,
    }
}

struct CallClues {
    anchor: MatchAnchor,
    titles: Vec<String>,
    armed: ArmedMeetings,
}

impl CallClues {
    fn inputs<'a>(&'a self, agenda: &'a TodaysAgenda) -> MatchInputs<'a> {
        MatchInputs {
            agenda,
            anchor: self.anchor,
            titles: &self.titles,
            armed: &self.armed,
        }
    }
}

fn call_clues<R: Runtime>(app: &AppHandle<R>) -> CallClues {
    CallClues {
        anchor: anchor_now(app),
        titles: call_titles(app),
        armed: app
            .try_state::<ArmedStore>()
            .map(|store| store.armed())
            .unwrap_or_default(),
    }
}

pub(super) fn connected_agenda<R: Runtime>(app: &AppHandle<R>) -> Option<TodaysAgenda> {
    match connection::current_connection(app) {
        Connection::Connected(_) => app.try_state::<AgendaCache>()?.agenda(),
        _ => None,
    }
}

fn anchor_now<R: Runtime>(app: &AppHandle<R>) -> MatchAnchor {
    let call_started_at = app
        .try_state::<CallPresence>()
        .and_then(|presence| presence.call_started_at());
    MatchAnchor::new(call_started_at.unwrap_or_else(|| Local::now().fixed_offset()))
}

fn call_titles<R: Runtime>(app: &AppHandle<R>) -> Vec<String> {
    app.try_state::<CallPresence>()
        .map(|presence| presence.call_titles())
        .unwrap_or_default()
}

pub fn note_call_started<R: Runtime>(app: &AppHandle<R>) {
    let Some(selection) = app.try_state::<MeetingSelection>() else {
        return;
    };
    selection.note_call_started(match_for_call(app));
    publish_call_meeting(app);
}

pub fn note_call_ended<R: Runtime>(app: &AppHandle<R>) {
    if let Some(selection) = app.try_state::<MeetingSelection>() {
        selection.note_call_ended();
        publish_call_meeting(app);
    }
}

pub fn current_call_meeting<R: Runtime>(app: &AppHandle<R>) -> CallMeetingView {
    let found = app
        .try_state::<MeetingSelection>()
        .and_then(|selection| selection.current());
    CallMeetingView::of(found.as_ref())
}

pub fn publish_call_meeting<R: Runtime>(app: &AppHandle<R>) {
    webview::emit(app, MATCH_EVENT, current_call_meeting(app));
}

fn match_for_call<R: Runtime>(app: &AppHandle<R>) -> CallMatch {
    if !matches!(
        connection::current_connection(app),
        Connection::Connected(_)
    ) {
        return CallMatch::settled(None);
    }
    app.state::<CallPresence>()
        .remember_call_titles(titles::teams_titles().teams_window_titles());
    let clues = call_clues(app);
    call_match(
        connected_agenda(app)
            .as_ref()
            .map(|agenda| clues.inputs(agenda)),
    )
}
