use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};

use calendar_graph::OutlookAccount;
use chrono::Local;
use scribe_core::calendar::{
    AGENDA_REFRESH_INTERVAL, CalendarError, Connection, DayBounds, todays_agenda,
};
use scribe_core::ports::CalendarSource;
use scribe_core::update::CheckSchedule;
use tauri::{AppHandle, Manager, Runtime};

use super::agenda_cache::AgendaCache;
use super::agenda_snapshot::FetchedAgenda;
use super::agenda_view::AgendaView;
use super::armed::ArmedStore;
use super::connection;
use super::connection_state::AttemptTicket;
use super::day_bounds::today_bounds;
use crate::webview;

const AGENDA_EVENT: &str = "agenda://updated";
const THREAD_NAME: &str = "agenda";
const TICK_INTERVAL: Duration = Duration::from_secs(1);
const RETRY_AFTER_FAILURE: Duration = Duration::from_secs(30);

pub fn install<R: Runtime>(app: &AppHandle<R>) {
    refresh_after_wake(app);
    let handle = app.clone();
    let spawned = thread::Builder::new()
        .name(THREAD_NAME.to_owned())
        .spawn(move || refresh_forever(&handle));
    if let Err(error) = spawned {
        tracing::error!(%error, "could not start refreshing the agenda");
    }
}

pub fn follow_connection<R: Runtime>(app: &AppHandle<R>, connection: &Connection) {
    let Some(cache) = app.try_state::<AgendaCache>() else {
        return;
    };
    match connection {
        Connection::Connected(_) => cache.nudge(),
        Connection::Disconnected => cache.clear(),
        Connection::Connecting | Connection::ReconnectNeeded => {}
    }
}

#[cfg(any(target_os = "macos", windows))]
fn refresh_after_wake<R: Runtime>(app: &AppHandle<R>) {
    let app = app.clone();
    crate::power::on_did_wake(move || app.state::<AgendaCache>().nudge());
}

#[cfg(not(any(target_os = "macos", windows)))]
fn refresh_after_wake<R: Runtime>(_app: &AppHandle<R>) {}

fn refresh_forever<R: Runtime>(app: &AppHandle<R>) {
    let mut schedule = CheckSchedule::new(AGENDA_REFRESH_INTERVAL);
    loop {
        tick(app, &mut schedule, Instant::now());
        thread::sleep(TICK_INTERVAL);
    }
}

fn tick<R: Runtime>(app: &AppHandle<R>, schedule: &mut CheckSchedule, now: Instant) {
    if app.state::<AgendaCache>().take_nudge() {
        schedule.check_soon(now);
    }
    if schedule.is_check_due(now) {
        schedule.mark_checked(now);
        if !refresh(app) {
            schedule.check_soon(now + RETRY_AFTER_FAILURE);
        }
    }
}

fn refresh<R: Runtime>(app: &AppHandle<R>) -> bool {
    match connection::current_connection(app) {
        Connection::Connected(_) => fetch(app),
        Connection::Connecting => {
            connection::restore_in_background(app);
            false
        }
        Connection::Disconnected | Connection::ReconnectNeeded => true,
    }
}

fn fetch<R: Runtime>(app: &AppHandle<R>) -> bool {
    let Some(source) = calendar_source(app) else {
        return true;
    };
    let now = Local::now();
    let Some(day) = today_bounds(&now) else {
        tracing::warn!("could not work out today's bounds");
        return false;
    };
    let ticket = connection::ticket(app);
    match source.events_between(&day) {
        Ok(events) => {
            let agenda = fetched(todays_agenda(events, &day), day, now);
            connection::while_connected_since(app, ticket, || keep_fetched(app, agenda));
            true
        }
        Err(error) => settle_failure(app, ticket, &error),
    }
}

fn keep_fetched<R: Runtime>(app: &AppHandle<R>, agenda: FetchedAgenda) {
    publish(app, &app.state::<AgendaCache>().record_fetch(agenda));
}

fn fetched(
    agenda: scribe_core::calendar::TodaysAgenda,
    day: DayBounds,
    now: chrono::DateTime<Local>,
) -> FetchedAgenda {
    FetchedAgenda {
        agenda,
        day,
        at: now.fixed_offset(),
    }
}

fn settle_failure<R: Runtime>(
    app: &AppHandle<R>,
    ticket: AttemptTicket,
    error: &CalendarError,
) -> bool {
    if matches!(error, CalendarError::ReconnectNeeded) {
        connection::require_reconnect(app, ticket);
        return true;
    }
    tracing::warn!(kind = error.kind(), "could not refresh the agenda");
    connection::while_connected_since(app, ticket, || {
        publish(app, &app.state::<AgendaCache>().record_failure());
    });
    false
}

fn calendar_source<R: Runtime>(app: &AppHandle<R>) -> Option<Arc<dyn CalendarSource>> {
    let account = app.try_state::<Arc<OutlookAccount>>()?;
    Some(Arc::clone(&account) as Arc<dyn CalendarSource>)
}

pub(super) fn publish<R: Runtime>(app: &AppHandle<R>, view: &AgendaView) {
    let marked = match app.try_state::<ArmedStore>() {
        Some(armed) => armed.mark(view.clone()),
        None => view.clone(),
    };
    webview::emit(app, AGENDA_EVENT, marked);
}
