use tauri::{AppHandle, Runtime};

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
