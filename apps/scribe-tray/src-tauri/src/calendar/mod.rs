pub mod agenda_cache;
mod agenda_refresh;
pub mod agenda_snapshot;
pub mod agenda_view;
pub mod armed;
mod auto_record;
pub mod call_meeting_view;
pub(crate) mod commands;
mod connection;
pub mod connection_payload;
pub mod connection_state;
pub mod day_bounds;
mod notify;
pub mod selection;
mod session_link;
pub mod switch_state;
mod switching;
mod titles;
pub mod vault;

use std::error::Error;
use std::sync::Arc;

use calendar_graph::{OutlookAccount, ReqwestTransport};
use tauri::{AppHandle, Manager, Runtime};

use agenda_cache::AgendaCache;
use armed::CallStartPopover;
use connection::OutlookConnection;
use selection::MeetingSelection;
use switch_state::SwitchState;
use vault::KeyringVault;

pub use auto_record::{note_going_to_sleep, note_meeting_finished, stop_if_auto_started};
pub use notify::notify_meeting_not_started;
pub use session_link::{
    call_match_for_meeting, link_finished_session, note_call_ended, note_call_started,
    note_recording_started,
};

pub fn install<R: Runtime>(app: &AppHandle<R>) {
    app.manage(OutlookConnection::new());
    app.manage(AgendaCache::new());
    app.manage(MeetingSelection::new());
    app.manage(SwitchState::new());
    auto_record::install(app);
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

pub fn on_poll<R: Runtime>(app: &AppHandle<R>, in_call: bool) -> CallStartPopover {
    let popover = auto_record::on_poll(app, in_call);
    switching::on_poll(app, in_call);
    popover
}

fn outlook_account() -> Result<Arc<OutlookAccount>, Box<dyn Error>> {
    let transport = ReqwestTransport::new()?;
    let vault = KeyringVault::open()?;
    Ok(Arc::new(OutlookAccount::new(
        Arc::new(transport),
        Arc::new(vault),
    )))
}
