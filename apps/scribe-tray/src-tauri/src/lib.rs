pub mod menu_action;
mod tray;

use tauri::Builder;

pub fn run() -> tauri::Result<()> {
    Builder::default()
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            #[cfg(target_os = "macos")]
            app.set_activation_policy(tauri::ActivationPolicy::Accessory);
            tray::install(app.handle())?;
            Ok(())
        })
        .run(tauri::generate_context!())
}
