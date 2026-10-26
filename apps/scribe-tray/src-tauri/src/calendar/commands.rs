use tauri::{AppHandle, Manager, Runtime};

use super::agenda_cache::AgendaCache;
use super::agenda_view::AgendaView;
use super::connection;
use super::connection_payload::ConnectionPayload;

#[tauri::command]
#[allow(clippy::needless_pass_by_value)]
pub fn outlook_connection<R: Runtime>(app: AppHandle<R>) -> ConnectionPayload {
    connection::current(&app)
}

#[tauri::command]
pub async fn connect_outlook<R: Runtime>(app: AppHandle<R>) -> ConnectionPayload {
    connection::connect_in_background(&app);
    connection::current(&app)
}

#[tauri::command]
pub async fn disconnect_outlook<R: Runtime>(
    app: AppHandle<R>,
) -> Result<ConnectionPayload, String> {
    connection::disconnect(&app).map_err(|error| error.to_string())
}

#[tauri::command]
#[allow(clippy::needless_pass_by_value)]
pub fn todays_agenda<R: Runtime>(app: AppHandle<R>) -> AgendaView {
    app.state::<AgendaCache>().view()
}

#[tauri::command]
#[allow(clippy::needless_pass_by_value)]
pub fn refresh_agenda<R: Runtime>(app: AppHandle<R>) {
    app.state::<AgendaCache>().nudge();
}
