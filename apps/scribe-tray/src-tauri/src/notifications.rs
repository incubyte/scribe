use std::path::Path;

use scribe_core::recording::AUTO_STOP_AFTER_QUIET;
use store_markdown::MoveReport;
use tauri::{AppHandle, Runtime};
use tauri_plugin_notification::NotificationExt;

use crate::claude_desktop::ConnectOutcome;
use crate::start_failure::StartFailureKind;
use crate::transcripts_location::folder_changed_message;

const MICROPHONE_DENIED_TITLE: &str = "Scribe can't use the microphone";
#[cfg(not(windows))]
const MICROPHONE_DENIED_BODY: &str = "Allow Scribe in System Settings > Privacy & Security > Microphone, then start recording again.";
#[cfg(not(windows))]
const OTHER_SIDE_SILENT_TITLE: &str = "Scribe can't hear the other side of the call";
#[cfg(not(windows))]
const OTHER_SIDE_SILENT_BODY: &str =
    "Allow Scribe in System Settings > Privacy & Security > Screen & System Audio Recording.";

#[cfg(target_os = "macos")]
const SCREEN_RECORDING_NEEDED_TITLE: &str =
    "Scribe needs Screen Recording access to take screenshots";
#[cfg(target_os = "macos")]
const SCREEN_RECORDING_NEEDED_BODY: &str = "Allow Scribe in Screen & System Audio Recording. If screenshots still fail after allowing it, quit and reopen Scribe.";

pub(crate) const RECORDING_NOT_STARTED_TITLE: &str = "Scribe couldn't start recording";
const RECORDING_AUTO_STOPPED_TITLE: &str = "Recording auto-stopped";
const SECONDS_PER_MINUTE: u64 = 60;

const TRANSCRIPTS_FOLDER_CHANGED_TITLE: &str = "Transcripts folder changed";
const TRANSCRIPTS_FOLDER_NOT_CHANGED_TITLE: &str = "Could not change the transcripts folder";
const TRANSCRIPTS_FOLDER_NOT_CHANGED_BODY: &str =
    "Scribe will keep saving transcripts in the current folder.";

const TRANSCRIPTS_NOT_MOVED_BODY: &str = "The folder was changed, but your existing transcripts could not be moved. They are still in the old folder.";

const MCP_CONFIG_COPIED_TITLE: &str = "MCP config copied";
const MCP_CONFIG_COPIED_BODY: &str =
    "Paste it into your AI app's MCP settings to let it read your transcripts.";
const MCP_CONFIG_NOT_COPIED_TITLE: &str = "Could not copy the MCP config";
const MCP_CONFIG_NOT_COPIED_BODY: &str =
    "Scribe's bundled MCP server was not found. Reinstall Scribe and try again.";

const CLAUDE_DESKTOP_CONNECTED: (&str, &str) = (
    "Scribe connected to Claude Desktop",
    "Restart Claude Desktop to use your Scribe transcripts.",
);
const CLAUDE_DESKTOP_ALREADY_CONNECTED: (&str, &str) = (
    "Scribe is already connected",
    "Claude Desktop is already set up to use Scribe.",
);
const CLAUDE_DESKTOP_NOT_INSTALLED: (&str, &str) = (
    "Claude Desktop not found",
    "Install Claude Desktop, open it once, then try again.",
);
const CLAUDE_DESKTOP_CONFIG_UNREADABLE: (&str, &str) = (
    "Couldn't update Claude Desktop",
    "Its config file has errors. Fix it, or use Copy MCP config.",
);
const CLAUDE_DESKTOP_FAILED: (&str, &str) = (
    "Couldn't connect to Claude Desktop",
    "Something went wrong. Use Copy MCP config instead.",
);

pub fn notify_claude_desktop<R: Runtime>(app: &AppHandle<R>, outcome: ConnectOutcome) {
    let (title, body) = match outcome {
        ConnectOutcome::Connected => CLAUDE_DESKTOP_CONNECTED,
        ConnectOutcome::AlreadyConnected => CLAUDE_DESKTOP_ALREADY_CONNECTED,
        ConnectOutcome::ClaudeNotInstalled => CLAUDE_DESKTOP_NOT_INSTALLED,
        ConnectOutcome::ConfigUnreadable => CLAUDE_DESKTOP_CONFIG_UNREADABLE,
        ConnectOutcome::Failed => CLAUDE_DESKTOP_FAILED,
    };
    show_plain(app, title, body);
}

pub fn notify_mcp_config_copied<R: Runtime>(app: &AppHandle<R>) {
    show_plain(app, MCP_CONFIG_COPIED_TITLE, MCP_CONFIG_COPIED_BODY);
}

pub fn notify_mcp_config_not_copied<R: Runtime>(app: &AppHandle<R>) {
    show_plain(app, MCP_CONFIG_NOT_COPIED_TITLE, MCP_CONFIG_NOT_COPIED_BODY);
}

pub fn notify_transcripts_folder_changed<R: Runtime>(
    app: &AppHandle<R>,
    folder: &Path,
    report: MoveReport,
) {
    let name = folder.file_name().map_or_else(
        || folder.to_string_lossy(),
        |last_component| last_component.to_string_lossy(),
    );
    let body = folder_changed_message(&name, report.moved, report.kept);
    show_plain(app, TRANSCRIPTS_FOLDER_CHANGED_TITLE, &body);
}

pub fn notify_transcripts_not_moved<R: Runtime>(app: &AppHandle<R>) {
    show_plain(
        app,
        TRANSCRIPTS_FOLDER_CHANGED_TITLE,
        TRANSCRIPTS_NOT_MOVED_BODY,
    );
}

pub fn notify_transcripts_folder_not_changed<R: Runtime>(app: &AppHandle<R>) {
    show_plain(
        app,
        TRANSCRIPTS_FOLDER_NOT_CHANGED_TITLE,
        TRANSCRIPTS_FOLDER_NOT_CHANGED_BODY,
    );
}

pub fn notify_recording_not_started<R: Runtime>(app: &AppHandle<R>, kind: StartFailureKind) {
    match kind {
        StartFailureKind::MicrophoneDenied => notify_microphone_denied(app),
        other => show_plain(app, RECORDING_NOT_STARTED_TITLE, other.advice()),
    }
}

pub fn notify_recording_auto_stopped<R: Runtime>(app: &AppHandle<R>) {
    let minutes = AUTO_STOP_AFTER_QUIET.as_secs() / SECONDS_PER_MINUTE;
    let body = format!(
        "No one spoke for {minutes} minutes, so Scribe stopped recording. The transcript is saved."
    );
    show_plain(app, RECORDING_AUTO_STOPPED_TITLE, &body);
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

#[cfg(target_os = "macos")]
pub use system_settings::notify_screen_recording_needed;

#[cfg(not(target_os = "macos"))]
pub fn notify_screen_recording_needed<R: Runtime>(_app: &AppHandle<R>) {
    tracing::info!("screen recording access is not required on this platform");
}

#[cfg(windows)]
pub fn notify_other_side_unavailable<R: Runtime>(app: &AppHandle<R>) {
    let alert = crate::other_side_alert::OTHER_SIDE_UNAVAILABLE_ON_WINDOWS;
    show_plain(app, alert.title, alert.body);
}

#[cfg(not(windows))]
pub fn notify_other_side_unavailable<R: Runtime>(app: &AppHandle<R>) {
    notify_cannot_hear_other_side(app);
}

#[cfg(windows)]
pub fn notify_cannot_hear_other_side<R: Runtime>(_app: &AppHandle<R>) {
    tracing::info!("the other side has been silent since the recording started");
}

pub(crate) fn show_plain<R: Runtime>(app: &AppHandle<R>, title: &str, body: &str) {
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

    use super::{
        OTHER_SIDE_SILENT_BODY, OTHER_SIDE_SILENT_TITLE, SCREEN_RECORDING_NEEDED_BODY,
        SCREEN_RECORDING_NEEDED_TITLE,
    };

    const OPEN_SYSTEM_SETTINGS: &str = "Open System Settings";
    const OPEN_SETTINGS: &str = "Open Settings";
    const SYSTEM_AUDIO_SETTINGS_URL: &str =
        "x-apple.systempreferences:com.apple.preference.security?Privacy_AudioCapture";
    const SCREEN_RECORDING_SETTINGS_URL: &str =
        "x-apple.systempreferences:com.apple.preference.security?Privacy_ScreenCapture";
    const DEV_NOTIFICATION_SENDER: &str = "com.apple.Terminal";

    struct SettingsPrompt {
        title: &'static str,
        body: &'static str,
        button: &'static str,
        settings_url: &'static str,
    }

    const OTHER_SIDE_PROMPT: SettingsPrompt = SettingsPrompt {
        title: OTHER_SIDE_SILENT_TITLE,
        body: OTHER_SIDE_SILENT_BODY,
        button: OPEN_SYSTEM_SETTINGS,
        settings_url: SYSTEM_AUDIO_SETTINGS_URL,
    };

    const SCREEN_RECORDING_PROMPT: SettingsPrompt = SettingsPrompt {
        title: SCREEN_RECORDING_NEEDED_TITLE,
        body: SCREEN_RECORDING_NEEDED_BODY,
        button: OPEN_SETTINGS,
        settings_url: SCREEN_RECORDING_SETTINGS_URL,
    };

    pub fn notify_cannot_hear_other_side<R: Runtime>(app: &AppHandle<R>) {
        prompt_to_open_settings(app, &OTHER_SIDE_PROMPT);
    }

    pub fn notify_screen_recording_needed<R: Runtime>(app: &AppHandle<R>) {
        prompt_to_open_settings(app, &SCREEN_RECORDING_PROMPT);
    }

    fn prompt_to_open_settings<R: Runtime>(app: &AppHandle<R>, prompt: &'static SettingsPrompt) {
        let app = app.clone();
        thread::spawn(move || {
            if user_chose_to_open_settings(&app, prompt) {
                open_settings(&app, prompt.settings_url);
            }
        });
    }

    fn user_chose_to_open_settings<R: Runtime>(
        app: &AppHandle<R>,
        prompt: &SettingsPrompt,
    ) -> bool {
        let _ = mac_notification_sys::set_application(&notification_sender(app));
        let response = Notification::new()
            .title(prompt.title)
            .message(prompt.body)
            .main_button(MainButton::SingleAction(prompt.button))
            .send();
        match response {
            Ok(NotificationResponse::ActionButton(_) | NotificationResponse::Click) => true,
            Ok(_) => false,
            Err(error) => {
                tracing::warn!(%error, title = prompt.title, "could not show the settings notification");
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

    fn open_settings<R: Runtime>(app: &AppHandle<R>, settings_url: &str) {
        if let Err(error) = app.opener().open_url(settings_url, None::<&str>) {
            tracing::warn!(%error, settings_url, "could not open the system settings");
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
