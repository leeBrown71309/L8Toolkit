//! "Launch at startup" through the OS login items (registry Run key on Windows).

use tauri::AppHandle;
use tauri_plugin_autostart::ManagerExt;

pub fn sync(app: &AppHandle, enabled: bool) -> Result<(), String> {
    let launcher = app.autolaunch();
    let currently_enabled = launcher.is_enabled().unwrap_or(false);
    if currently_enabled == enabled {
        return Ok(());
    }
    let outcome = if enabled { launcher.enable() } else { launcher.disable() };
    outcome.map_err(|error| format!("Cannot update launch at startup: {error}"))
}
