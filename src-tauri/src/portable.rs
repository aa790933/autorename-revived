use std::path::PathBuf;
use tauri::Manager;

/// Returns true if the app is running in portable mode.
///
/// Portable mode is detected by the presence of a `.portable` marker file
/// in the same directory as the executable. This file can be empty — its
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
pub fn settings_dir(app: &tauri::AppHandle) -> PathBuf {
    if is_portable() {
        // Use the executable's parent directory
        let exe = std::env::current_exe()
            .expect("Failed to get current executable path");
        exe.parent()
            .expect("Failed to get executable parent directory")
            .to_path_buf()
    } else {
        // Use the standard OS app_data_dir
        app.path()
            .app_data_dir()
            .expect("Failed to get app data dir")
            .to_path_buf()
    }
}

/// Returns the full path to `settings.json` based on portability mode.
pub fn settings_path(app: &tauri::AppHandle) -> PathBuf {
    settings_dir(app).join("settings.json")
}