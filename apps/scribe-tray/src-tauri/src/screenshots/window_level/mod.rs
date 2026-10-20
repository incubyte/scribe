#[cfg(target_os = "macos")]
mod macos;

#[cfg(target_os = "macos")]
pub use macos::raise_above_menu_bar;

#[cfg(not(target_os = "macos"))]
pub fn raise_above_menu_bar<R: tauri::Runtime>(_window: &tauri::WebviewWindow<R>) {}
