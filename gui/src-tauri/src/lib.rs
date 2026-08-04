pub fn run() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() > 1 && args.iter().any(|a| a == "--version" || a == "-V") {
        println!("autorename-revived-gui {}", env!("CARGO_PKG_VERSION"));
        return;
    }

    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![get_version])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

#[tauri::command]
fn get_version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}
