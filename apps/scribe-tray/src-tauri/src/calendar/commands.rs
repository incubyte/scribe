use scribe_core::calendar::EventId;
use tauri::{AppHandle, Manager, Runtime};

use super::agenda_cache::AgendaCache;
use super::agenda_view::AgendaView;
use super::auto_record;
use super::call_meeting_view::CallMeetingView;
use super::connection;
use super::connection_payload::ConnectionPayload;
use super::selection::MeetingSelection;
use super::session_link;
use super::switch_state::SwitchPromptView;
use super::switching;

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
    auto_record::agenda_view(&app)
}

#[tauri::command]
#[allow(clippy::needless_pass_by_value)]
pub fn refresh_agenda<R: Runtime>(app: AppHandle<R>) {
    app.state::<AgendaCache>().nudge();
}

#[tauri::command]
#[allow(clippy::needless_pass_by_value)]
pub fn call_meeting<R: Runtime>(app: AppHandle<R>) -> CallMeetingView {
    session_link::current_call_meeting(&app)
}

#[tauri::command]
#[allow(clippy::needless_pass_by_value)]
pub fn choose_meeting<R: Runtime>(
    app: AppHandle<R>,
    event_id: Option<String>,
) -> Result<CallMeetingView, String> {
    let chosen = event_id.map(EventId::new);
    app.state::<MeetingSelection>()
        .choose(chosen.as_ref())
        .map_err(|error| error.reason().to_owned())?;
    session_link::publish_call_meeting(&app);
    Ok(session_link::current_call_meeting(&app))
}

#[tauri::command]
#[allow(clippy::needless_pass_by_value)]
pub fn arm_meeting<R: Runtime>(app: AppHandle<R>, event_id: String) -> Result<(), String> {
    auto_record::arm(&app, EventId::new(event_id)).map_err(|refusal| refusal.reason().to_owned())
}

#[tauri::command]
#[allow(clippy::needless_pass_by_value)]
pub fn disarm_meeting<R: Runtime>(app: AppHandle<R>, event_id: String) {
    auto_record::disarm(&app, &EventId::new(event_id));
}

#[tauri::command]
#[allow(clippy::needless_pass_by_value)]
pub fn record_meeting<R: Runtime>(app: AppHandle<R>, event_id: String) {
    auto_record::record(&app, &EventId::new(event_id));
}

#[tauri::command]
#[allow(clippy::needless_pass_by_value)]
pub fn switch_prompt<R: Runtime>(app: AppHandle<R>) -> Option<SwitchPromptView> {
    switching::current(&app)
}

#[tauri::command]
pub async fn switch_meeting<R: Runtime>(app: AppHandle<R>) {
    switching::switch(&app);
}

#[tauri::command]
#[allow(clippy::needless_pass_by_value)]
pub fn keep_recording<R: Runtime>(app: AppHandle<R>) {
    switching::keep(&app);
}
