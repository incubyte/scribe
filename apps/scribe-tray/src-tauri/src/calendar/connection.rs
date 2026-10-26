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

pub struct OutlookConnection {
    view: Mutex<ConnectionView>,
    attempt_running: AtomicBool,
}

impl OutlookConnection {
    pub fn new() -> OutlookConnection {
        OutlookConnection {
            view: Mutex::new(ConnectionView {
                connection: Connection::Connecting,
                failure: None,
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

    fn change(&self, connection: Connection, failure: Option<&'static str>) -> ConnectionPayload {
        let mut view = self.view();
        *view = ConnectionView {
            connection,
            failure,
        };
        ConnectionPayload::new(&view.connection, view.failure)
    }

    fn payload(&self) -> ConnectionPayload {
        let view = self.view();
        ConnectionPayload::new(&view.connection, view.failure)
    }

    fn connection(&self) -> Connection {
        self.view().connection.clone()
    }

    fn view(&self) -> MutexGuard<'_, ConnectionView> {
        self.view.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

pub fn current<R: Runtime>(app: &AppHandle<R>) -> ConnectionPayload {
    app.state::<OutlookConnection>().payload()
}

pub fn current_connection<R: Runtime>(app: &AppHandle<R>) -> Connection {
    app.state::<OutlookConnection>().connection()
}

pub fn require_reconnect<R: Runtime>(app: &AppHandle<R>) {
    publish(app, Connection::ReconnectNeeded, None);
}

pub fn connect_in_background<R: Runtime>(app: &AppHandle<R>) {
    if !app.state::<OutlookConnection>().begin_attempt() {
        return;
    }
    publish(app, Connection::Connecting, None);
    run_attempt(app, SIGN_IN_THREAD, sign_in);
}

pub fn restore_in_background<R: Runtime>(app: &AppHandle<R>) {
    if app.state::<OutlookConnection>().begin_attempt() {
        run_attempt(app, RESTORE_THREAD, restore);
    }
}

pub fn give_up_restoring<R: Runtime>(app: &AppHandle<R>) {
    publish(app, Connection::Disconnected, None);
}

pub fn disconnect<R: Runtime>(app: &AppHandle<R>) -> Result<ConnectionPayload, VaultError> {
    if let Some(account) = account(app) {
        account.disconnect()?;
    }
    Ok(publish(app, Connection::Disconnected, None))
}

fn run_attempt<R: Runtime>(app: &AppHandle<R>, name: &str, attempt: fn(&AppHandle<R>)) {
    let handle = app.clone();
    let spawned = thread::Builder::new().name(name.to_owned()).spawn(move || {
        attempt(&handle);
        handle.state::<OutlookConnection>().end_attempt();
    });
    if let Err(error) = spawned {
        tracing::error!(%error, "could not start the outlook connection");
        app.state::<OutlookConnection>().end_attempt();
        publish(app, Connection::Disconnected, None);
    }
}

fn sign_in<R: Runtime>(app: &AppHandle<R>) {
    let Some(account) = account(app) else {
        publish(app, Connection::Disconnected, None);
        return;
    };
    match account.connect(&|url| open_in_browser(app, url)) {
        Ok(identity) => publish(app, Connection::Connected(identity), None),
        Err(error) => {
            tracing::warn!(kind = error.kind(), "outlook did not connect");
            publish(app, Connection::Disconnected, Some(error.plain_reason()))
        }
    };
}

fn restore<R: Runtime>(app: &AppHandle<R>) {
    let Some(account) = account(app) else {
        publish(app, Connection::Disconnected, None);
        return;
    };
    publish(app, restored_connection(account.restore()), None);
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

fn publish<R: Runtime>(
    app: &AppHandle<R>,
    connection: Connection,
    failure: Option<&'static str>,
) -> ConnectionPayload {
    agenda_refresh::follow_connection(app, &connection);
    let payload = app.state::<OutlookConnection>().change(connection, failure);
    webview::emit(app, CONNECTION_EVENT, payload.clone());
    payload
}
