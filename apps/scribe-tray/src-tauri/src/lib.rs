pub mod menu_action;
mod privacy;
mod tray;

use tauri::Builder;

pub fn run() -> tauri::Result<()> {
    Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_store::Builder::default().build())
        .invoke_handler(tauri::generate_handler![privacy::acknowledge_privacy])
        .setup(|app| {
            #[cfg(target_os = "macos")]
            app.set_activation_policy(tauri::ActivationPolicy::Accessory);
            tray::install(app.handle())?;
            privacy::show_statement_until_acknowledged(app.handle())?;
            Ok(())
        })
        .run(tauri::generate_context!())
}
