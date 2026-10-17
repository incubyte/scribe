use tauri::{AppHandle, Runtime};
use tauri_plugin_clipboard_manager::ClipboardExt;

use crate::mcp_config::{McpConfigError, bundled_mcp_path, config_snippet};
use crate::notifications;

#[derive(Debug, thiserror::Error)]
enum CopyError {
    #[error(transparent)]
    Config(#[from] McpConfigError),
    #[error(transparent)]
    Clipboard(#[from] tauri_plugin_clipboard_manager::Error),
}

pub fn copy_config<R: Runtime>(app: &AppHandle<R>) {
    match try_copy_config(app) {
        Ok(()) => notifications::notify_mcp_config_copied(app),
        Err(error) => {
            tracing::error!(%error, "could not copy the MCP config");
            notifications::notify_mcp_config_not_copied(app);
        }
    }
}

fn try_copy_config<R: Runtime>(app: &AppHandle<R>) -> Result<(), CopyError> {
    let path = bundled_mcp_path()?;
    app.clipboard().write_text(config_snippet(&path))?;
    Ok(())
}
