use std::sync::Arc;

use chrono::Local;
use scribe_core::calendar::{Connection, MatchAnchor, MeetingLink, TodaysAgenda};
use scribe_core::recording::SessionId;
use store_markdown::MarkdownTranscriptStore;
use tauri::{AppHandle, Manager, Runtime};

use super::agenda_cache::AgendaCache;
use super::connection;
use super::selection::{MeetingSelection, chosen_meeting};
use super::titles;
use crate::call_watch::CallPresence;

pub fn note_recording_started<R: Runtime>(app: &AppHandle<R>, session: &SessionId) {
    let Some(selection) = app.try_state::<MeetingSelection>() else {
        return;
    };
    let chosen = chosen_meeting(
        connected_agenda(app).as_ref(),
        &anchor_now(app),
        &call_titles(app),
    );
    selection.begin(session.clone(), chosen);
}

pub fn link_finished_session<R: Runtime>(app: &AppHandle<R>, session: &SessionId) {
    let Some(meeting) = app
        .try_state::<MeetingSelection>()
        .and_then(|selection| selection.take_for(session))
    else {
        return;
    };
    let store = app.state::<Arc<MarkdownTranscriptStore>>();
    if let Err(error) = store.link_meeting(session, MeetingLink::from_meeting(&meeting)) {
        tracing::warn!(%error, "could not save the meeting with the transcript");
    }
}

fn connected_agenda<R: Runtime>(app: &AppHandle<R>) -> Option<TodaysAgenda> {
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
    if !matches!(
        connection::current_connection(app),
        Connection::Connected(_)
    ) {
        return;
    }
    if let Some(presence) = app.try_state::<CallPresence>() {
        presence.remember_call_titles(titles::teams_titles().teams_window_titles());
    }
}
