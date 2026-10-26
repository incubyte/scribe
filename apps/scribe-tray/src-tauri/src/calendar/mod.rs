pub mod agenda_cache;
mod agenda_refresh;
pub mod agenda_snapshot;
pub mod agenda_view;
pub mod call_meeting_view;
pub(crate) mod commands;
mod connection;
pub mod connection_payload;
pub mod day_bounds;
pub mod selection;
mod session_link;
mod titles;
pub mod vault;

use std::error::Error;
use std::sync::Arc;

use calendar_graph::{OutlookAccount, ReqwestTransport};
use tauri::{AppHandle, Manager, Runtime};

use agenda_cache::AgendaCache;
use connection::OutlookConnection;
use selection::MeetingSelection;
use vault::KeyringVault;

pub use session_link::{
    link_finished_session, note_call_ended, note_call_started, note_recording_started,
};

pub fn install<R: Runtime>(app: &AppHandle<R>) {
    app.manage(OutlookConnection::new());
    app.manage(AgendaCache::new());
    app.manage(MeetingSelection::new());
    match outlook_account() {
        Ok(account) => {
            app.manage(account);
            agenda_refresh::install(app);
            connection::restore_in_background(app);
        }
        Err(error) => {
            tracing::error!(%error, "could not prepare the outlook connection");
            connection::give_up_restoring(app);
        }
    }
}

fn outlook_account() -> Result<Arc<OutlookAccount>, Box<dyn Error>> {
    let transport = ReqwestTransport::new()?;
    let vault = KeyringVault::open()?;
    Ok(Arc::new(OutlookAccount::new(
        Arc::new(transport),
        Arc::new(vault),
    )))
}
