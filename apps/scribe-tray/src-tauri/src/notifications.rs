use tauri::{AppHandle, Runtime};
use tauri_plugin_notification::NotificationExt;

const MICROPHONE_DENIED_TITLE: &str = "Scribe can't use the microphone";
#[cfg(not(windows))]
const MICROPHONE_DENIED_BODY: &str = "Allow Scribe in System Settings > Privacy & Security > Microphone, then start recording again.";
#[cfg(not(windows))]
const OTHER_SIDE_SILENT_TITLE: &str = "Scribe can't hear the other side of the call";
#[cfg(not(windows))]
const OTHER_SIDE_SILENT_BODY: &str =
    "Allow Scribe in System Settings > Privacy & Security > Screen & System Audio Recording.";

const MCP_CONFIG_COPIED_TITLE: &str = "MCP config copied";
const MCP_CONFIG_COPIED_BODY: &str =
    "Paste it into your AI app's MCP settings to let it read your transcripts.";
const MCP_CONFIG_NOT_COPIED_TITLE: &str = "Could not copy the MCP config";
const MCP_CONFIG_NOT_COPIED_BODY: &str =
    "Scribe's bundled MCP server was not found. Reinstall Scribe and try again.";

pub fn notify_mcp_config_copied<R: Runtime>(app: &AppHandle<R>) {
    show_plain(app, MCP_CONFIG_COPIED_TITLE, MCP_CONFIG_COPIED_BODY);
}

pub fn notify_mcp_config_not_copied<R: Runtime>(app: &AppHandle<R>) {
    show_plain(app, MCP_CONFIG_NOT_COPIED_TITLE, MCP_CONFIG_NOT_COPIED_BODY);
}

#[cfg(not(windows))]
pub fn notify_microphone_denied<R: Runtime>(app: &AppHandle<R>) {
    show_plain(app, MICROPHONE_DENIED_TITLE, MICROPHONE_DENIED_BODY);
}

#[cfg(windows)]
pub use privacy_settings::notify_microphone_denied;

#[cfg(target_os = "macos")]
pub use system_settings::notify_cannot_hear_other_side;

#[cfg(not(any(target_os = "macos", windows)))]
pub fn notify_cannot_hear_other_side<R: Runtime>(app: &AppHandle<R>) {
    show_plain(app, OTHER_SIDE_SILENT_TITLE, OTHER_SIDE_SILENT_BODY);
}

#[cfg(windows)]
pub fn notify_cannot_hear_other_side<R: Runtime>(_app: &AppHandle<R>) {
    tracing::info!("the other side has been silent since the recording started");
}

fn show_plain<R: Runtime>(app: &AppHandle<R>, title: &str, body: &str) {
    let shown = app.notification().builder().title(title).body(body).show();
    if let Err(error) = shown {
        tracing::warn!(%error, title, "could not show a notification");
    }
}

#[cfg(target_os = "macos")]
mod system_settings {
    use std::thread;

    use mac_notification_sys::{MainButton, Notification, NotificationResponse};
    use tauri::{AppHandle, Runtime};
    use tauri_plugin_opener::OpenerExt;

    use super::{OTHER_SIDE_SILENT_BODY, OTHER_SIDE_SILENT_TITLE};

    const OPEN_SYSTEM_SETTINGS: &str = "Open System Settings";
    const SYSTEM_AUDIO_SETTINGS_URL: &str =
        "x-apple.systempreferences:com.apple.preference.security?Privacy_AudioCapture";
    const DEV_NOTIFICATION_SENDER: &str = "com.apple.Terminal";

    pub fn notify_cannot_hear_other_side<R: Runtime>(app: &AppHandle<R>) {
        let app = app.clone();
        thread::spawn(move || {
            if user_chose_to_open_settings(&app) {
                open_system_audio_settings(&app);
            }
        });
    }

    fn user_chose_to_open_settings<R: Runtime>(app: &AppHandle<R>) -> bool {
        let _ = mac_notification_sys::set_application(&notification_sender(app));
        let response = Notification::new()
            .title(OTHER_SIDE_SILENT_TITLE)
            .message(OTHER_SIDE_SILENT_BODY)
            .main_button(MainButton::SingleAction(OPEN_SYSTEM_SETTINGS))
            .send();
        match response {
            Ok(NotificationResponse::ActionButton(_) | NotificationResponse::Click) => true,
            Ok(_) => false,
            Err(error) => {
                tracing::warn!(%error, "could not show the system audio permission notification");
                false
            }
        }
    }

    fn notification_sender<R: Runtime>(app: &AppHandle<R>) -> String {
        if tauri::is_dev() {
            DEV_NOTIFICATION_SENDER.to_owned()
        } else {
            app.config().identifier.clone()
        }
    }

    fn open_system_audio_settings<R: Runtime>(app: &AppHandle<R>) {
        if let Err(error) = app
            .opener()
            .open_url(SYSTEM_AUDIO_SETTINGS_URL, None::<&str>)
        {
            tracing::warn!(%error, "could not open the system audio privacy settings");
        }
    }
}

#[cfg(windows)]
mod privacy_settings {
    use tauri::{AppHandle, Runtime};
    use tauri_plugin_opener::OpenerExt;
    use tauri_winrt_notification::Toast;

    use super::MICROPHONE_DENIED_TITLE;

    const MICROPHONE_BLOCKED_BODY: &str = "Turn on microphone access for desktop apps in Settings > Privacy & security > Microphone, then start recording again.";
    const OPEN_SETTINGS: &str = "Open Settings";
    const OPEN_SETTINGS_ACTION: &str = "open-microphone-privacy";
    const MICROPHONE_PRIVACY_URL: &str = "ms-settings:privacy-microphone";

    pub fn notify_microphone_denied<R: Runtime>(app: &AppHandle<R>) {
        let opener = app.clone();
        let shown = Toast::new(&notification_sender(app))
            .title(MICROPHONE_DENIED_TITLE)
            .text1(MICROPHONE_BLOCKED_BODY)
            .add_button(OPEN_SETTINGS, OPEN_SETTINGS_ACTION)
            .on_activated(move |_action| {
                open_microphone_privacy_settings(&opener);
                Ok(())
            })
            .show();
        if let Err(error) = shown {
            tracing::warn!(%error, "could not show the microphone permission notification");
        }
    }

    fn notification_sender<R: Runtime>(app: &AppHandle<R>) -> String {
        if tauri::is_dev() {
            Toast::POWERSHELL_APP_ID.to_owned()
        } else {
            app.config().identifier.clone()
        }
    }

    fn open_microphone_privacy_settings<R: Runtime>(app: &AppHandle<R>) {
        if let Err(error) = app.opener().open_url(MICROPHONE_PRIVACY_URL, None::<&str>) {
            tracing::warn!(%error, "could not open the microphone privacy settings");
        }
    }
}
