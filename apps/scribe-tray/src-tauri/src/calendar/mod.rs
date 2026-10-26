pub(crate) mod commands;
mod connection;
pub mod connection_payload;
pub mod vault;

use std::error::Error;
use std::sync::Arc;

use calendar_graph::{OutlookAccount, ReqwestTransport};
use tauri::{AppHandle, Manager, Runtime};

use connection::OutlookConnection;
use vault::KeyringVault;

pub fn install<R: Runtime>(app: &AppHandle<R>) {
    app.manage(OutlookConnection::new());
    match outlook_account() {
        Ok(account) => {
            app.manage(account);
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
