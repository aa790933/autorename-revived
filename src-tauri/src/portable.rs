use std::path::PathBuf;
use tauri::Manager;

/// Returns true if the app is running in portable mode.
///
/// Portable mode is detected by the presence of a `.portable` marker file
/// in the same directory as the executable. This file can be empty , its
/// mere existence signals that settings should be stored alongside the
/// executable rather than in the OS application-data directory.
pub fn is_portable() -> bool {
    portable_marker_path().map(|p| p.exists()).unwrap_or(false)
}

/// Returns the path to the `.portable` marker file next to the executable.
fn portable_marker_path() -> Option<PathBuf> {
    let exe = std::env::current_exe().ok()?;
    let exe_dir = exe.parent()?;
    Some(exe_dir.join(".portable"))
}

/// Returns the directory where settings should be stored.
///
/// - Portable: the directory containing the executable
/// - Installer: the OS standard app_data_dir
///
/// Returns an `Err` rather than panicking: this runs inside Tauri commands,
/// where a panic surfaces as a crashed window instead of a usable message.
/// If portable mode is active but the executable path is unavailable, the
/// call falls back to the standard app-data directory.
pub fn settings_dir(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    if is_portable() {
        if let Some(dir) = std::env::current_exe()
            .ok()
            .and_then(|exe| exe.parent().map(|p| p.to_path_buf()))
        {
            return Ok(dir);
        }
        // Portable marker present but the exe path is unavailable; fall
        // through to app data so the app still has a writable settings home.
    }
    app.path()
        .app_data_dir()
        .map_err(|e| format!("Failed to resolve the settings directory: {}", e))
}

/// Returns the full path to `settings.json` based on portability mode.
pub fn settings_path(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    Ok(settings_dir(app)?.join("settings.json"))
}

/// Absolute path of the undo/rename history log inside the settings dir.
pub fn undo_log_path(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    Ok(settings_dir(app)?.join("rename_history.json"))
}
