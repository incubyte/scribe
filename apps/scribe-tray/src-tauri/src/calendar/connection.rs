use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};
use std::thread;

use calendar_graph::OutlookAccount;
use scribe_core::calendar::{AccountIdentity, CalendarError, Connection, VaultError};
use tauri::{AppHandle, Manager, Runtime};
use tauri_plugin_opener::OpenerExt;
use url::Url;

use super::agenda_refresh;
use super::connection_payload::ConnectionPayload;
use crate::webview;

const CONNECTION_EVENT: &str = "outlook://connection";
const SIGN_IN_THREAD: &str = "outlook-sign-in";
const RESTORE_THREAD: &str = "outlook-restore";

struct ConnectionView {
    connection: Connection,
    failure: Option<&'static str>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct AttemptTicket(u64);

struct ConnectionState {
    view: ConnectionView,
    disconnects: u64,
}

impl ConnectionState {
    fn ticket(&self) -> AttemptTicket {
        AttemptTicket(self.disconnects)
    }
}

pub struct OutlookConnection {
    state: Mutex<ConnectionState>,
    attempt_running: AtomicBool,
}

impl OutlookConnection {
    pub fn new() -> OutlookConnection {
        OutlookConnection {
            state: Mutex::new(ConnectionState {
                view: ConnectionView {
                    connection: Connection::Connecting,
                    failure: None,
                },
                disconnects: 0,
            }),
            attempt_running: AtomicBool::new(false),
        }
    }

    fn begin_attempt(&self) -> bool {
        self.attempt_running
            .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
            .is_ok()
    }

    fn end_attempt(&self) {
        self.attempt_running.store(false, Ordering::Release);
    }

    fn payload(&self) -> ConnectionPayload {
        let state = self.state();
        ConnectionPayload::new(&state.view.connection, state.view.failure)
    }

    fn connection(&self) -> Connection {
        self.state().view.connection.clone()
    }

    fn ticket(&self) -> AttemptTicket {
        self.state().ticket()
    }

    fn state(&self) -> MutexGuard<'_, ConnectionState> {
        self.state.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

pub fn current<R: Runtime>(app: &AppHandle<R>) -> ConnectionPayload {
    app.state::<OutlookConnection>().payload()
}

pub fn current_connection<R: Runtime>(app: &AppHandle<R>) -> Connection {
    app.state::<OutlookConnection>().connection()
}

pub fn require_reconnect<R: Runtime>(app: &AppHandle<R>) {
    publish(app, settled(Connection::ReconnectNeeded));
}

pub fn connect_in_background<R: Runtime>(app: &AppHandle<R>) {
    if !app.state::<OutlookConnection>().begin_attempt() {
        return;
    }
    publish(app, settled(Connection::Connecting));
    run_attempt(app, SIGN_IN_THREAD, sign_in);
}

pub fn restore_in_background<R: Runtime>(app: &AppHandle<R>) {
    if app.state::<OutlookConnection>().begin_attempt() {
        run_attempt(app, RESTORE_THREAD, restore);
    }
}

pub fn give_up_restoring<R: Runtime>(app: &AppHandle<R>) {
    publish(app, settled(Connection::Disconnected));
}

pub fn disconnect<R: Runtime>(app: &AppHandle<R>) -> Result<ConnectionPayload, VaultError> {
    if let Some(account) = account(app) {
        account.disconnect()?;
    }
    let connection = app.state::<OutlookConnection>();
    let mut state = connection.state();
    state.disconnects += 1;
    Ok(announce(app, &mut state, settled(Connection::Disconnected)))
}

fn run_attempt<R: Runtime>(
    app: &AppHandle<R>,
    name: &str,
    attempt: fn(&AppHandle<R>) -> ConnectionView,
) {
    let handle = app.clone();
    let ticket = app.state::<OutlookConnection>().ticket();
    let spawned = thread::Builder::new().name(name.to_owned()).spawn(move || {
        let outcome = attempt(&handle);
        publish_unless_disconnected(&handle, ticket, outcome);
        handle.state::<OutlookConnection>().end_attempt();
    });
    if let Err(error) = spawned {
        tracing::error!(%error, "could not start the outlook connection");
        app.state::<OutlookConnection>().end_attempt();
        publish(app, settled(Connection::Disconnected));
    }
}

fn sign_in<R: Runtime>(app: &AppHandle<R>) -> ConnectionView {
    let Some(account) = account(app) else {
        return settled(Connection::Disconnected);
    };
    match account.connect(&|url| open_in_browser(app, url)) {
        Ok(identity) => settled(Connection::Connected(identity)),
        Err(error) => {
            tracing::warn!(kind = error.kind(), "outlook did not connect");
            ConnectionView {
                connection: Connection::Disconnected,
                failure: Some(error.plain_reason()),
            }
        }
    }
}

fn restore<R: Runtime>(app: &AppHandle<R>) -> ConnectionView {
    let Some(account) = account(app) else {
        return settled(Connection::Disconnected);
    };
    settled(restored_connection(account.restore()))
}

fn settled(connection: Connection) -> ConnectionView {
    ConnectionView {
        connection,
        failure: None,
    }
}

fn restored_connection(outcome: Result<Option<AccountIdentity>, CalendarError>) -> Connection {
    match outcome {
        Ok(Some(identity)) => Connection::Connected(identity),
        Ok(None) => Connection::Disconnected,
        Err(CalendarError::ReconnectNeeded) => Connection::ReconnectNeeded,
        Err(error) => {
            tracing::warn!(
                kind = error.kind(),
                "could not restore the outlook connection"
            );
            Connection::Connecting
        }
    }
}

fn account<R: Runtime>(app: &AppHandle<R>) -> Option<Arc<OutlookAccount>> {
    app.try_state::<Arc<OutlookAccount>>()
        .map(|account| Arc::clone(&account))
}

fn open_in_browser<R: Runtime>(app: &AppHandle<R>, url: &Url) {
    if app.opener().open_url(url.as_str(), None::<&str>).is_err() {
        tracing::warn!("could not open the browser for the outlook sign-in");
    }
}

fn publish<R: Runtime>(app: &AppHandle<R>, view: ConnectionView) -> ConnectionPayload {
    let connection = app.state::<OutlookConnection>();
    announce(app, &mut connection.state(), view)
}

fn publish_unless_disconnected<R: Runtime>(
    app: &AppHandle<R>,
    ticket: AttemptTicket,
    view: ConnectionView,
) {
    let connection = app.state::<OutlookConnection>();
    let mut state = connection.state();
    if state.ticket() == ticket {
        announce(app, &mut state, view);
    }
}

fn announce<R: Runtime>(
    app: &AppHandle<R>,
    state: &mut ConnectionState,
    view: ConnectionView,
) -> ConnectionPayload {
    agenda_refresh::follow_connection(app, &view.connection);
    let payload = ConnectionPayload::new(&view.connection, view.failure);
    state.view = view;
    webview::emit(app, CONNECTION_EVENT, payload.clone());
    payload
}
