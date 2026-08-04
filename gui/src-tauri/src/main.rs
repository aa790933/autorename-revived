mod app_state;
mod commands;
mod provider;

use app_state::AppState;
use tauri::Manager;

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_store::Builder::new().build())
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .manage(AppState::new())
        .invoke_handler(tauri::generate_handler![
            commands::save_settings,
            commands::get_settings,
            commands::test_connection,
            commands::rename_files,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}