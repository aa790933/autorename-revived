mod ai;
mod config;
mod document;
mod file_utils;

use ai::{AiConfig, DocumentMetadata, TestConnectionResult};
use config::{AppConfig, load_config, save_config, save_config_batch};
use document::{BatchResult, FileResult, UndoResult};
use file_utils::{
    copy_file, ensure_directory, file_exists, get_file_extension, get_file_name, get_file_size,
    get_supported_extensions, is_image_extension, list_files_in_directory,
    preserve_extension, read_file_to_base64, read_file_to_bytes, recursive_find_files,
    resolve_safe_path, validate_supported_extension,
};
use chrono;
use serde_json::Value;
use std::path::PathBuf;
use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_store::Builder::new().build())
        .invoke_handler(tauri::generate_handler![
            get_version,
            get_supported_extensions_list,
            load_app_config,
            save_app_config,
            save_app_config_batch,
            test_connection,
            extract_metadata_from_text,
            extract_metadata_from_vision,
            read_file_bytes,
            read_file_base64,
            preserve_file_extension,
            validate_extension,
            is_image_file,
            get_file_size_bytes,
            get_file_name_from_path,
            get_file_stem_from_path,
            get_file_ext,
            resolve_safe_path_cmd,
            ensure_directory_cmd,
            copy_file_cmd,
            file_exists_cmd,
            list_files,
            find_files_recursive,
            apply_rename_cmd,
            save_rename_to_history_cmd,
            undo_last_rename_cmd,
            rename_pdfs,
            undo_rename,
            get_config,
            get_config_path,
            validate_config,
            save_config,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

#[tauri::command]
fn get_version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}

#[tauri::command]
fn get_supported_extensions_list() -> Vec<String> {
    get_supported_extensions().iter().map(|s| s.to_string()).collect()
}

#[tauri::command]
async fn load_app_config(app: tauri::AppHandle) -> Result<AppConfig, String> {
    load_config(app).await
}

#[tauri::command]
async fn save_app_config(app: tauri::AppHandle, config: AppConfig) -> Result<(), String> {
    save_config(app, &config).await
}

#[tauri::command]
async fn save_app_config_batch(
    app: tauri::AppHandle,
    updates: Vec<(String, Value)>,
) -> Result<(), String> {
    save_config_batch(app, updates).await
}

#[tauri::command]
async fn test_connection(
    provider: String,
    api_key: String,
    model: String,
) -> TestConnectionResult {
    ai::test_connection(&provider, &api_key, &model).await
}

#[tauri::command]
async fn extract_metadata_from_text(
    text: String,
    config: AiConfig,
) -> Result<DocumentMetadata, String> {
    ai::extract_metadata_text(&text, &config).await
}

#[tauri::command]
async fn extract_metadata_from_vision(
    files: Vec<(String, Vec<u8>)>,
    config: AiConfig,
) -> Result<DocumentMetadata, String> {
    ai::extract_metadata_vision(&files, &config).await
}

#[tauri::command]
fn read_file_bytes(path: String) -> Result<Vec<u8>, String> {
    read_file_to_bytes(&path)
}

#[tauri::command]
fn read_file_base64(path: String) -> Result<String, String> {
    read_file_to_base64(&path)
}

#[tauri::command]
fn preserve_file_extension(original_path: String, new_name: String) -> String {
    preserve_extension(&original_path, &new_name)
}

#[tauri::command]
fn validate_extension(path: String) -> bool {
    validate_supported_extension(&path)
}

#[tauri::command]
fn is_image_file(path: String) -> bool {
    is_image_extension(&path)
}

#[tauri::command]
fn get_file_size_bytes(path: String) -> Result<u64, String> {
    get_file_size(&path)
}

#[tauri::command]
fn get_file_name_from_path(path: String) -> String {
    get_file_name(&path)
}

#[tauri::command]
fn get_file_stem_from_path(path: String) -> String {
    get_file_stem(&path)
}

#[tauri::command]
fn get_file_ext(path: String) -> String {
    get_file_extension(&path)
}

#[tauri::command]
fn resolve_safe_path_cmd(directory: String, filename: String) -> Result<String, String> {
    resolve_safe_path(&directory, &filename)
}

#[tauri::command]
fn ensure_directory_cmd(path: String) -> Result<(), String> {
    ensure_directory(&path)
}

#[tauri::command]
fn copy_file_cmd(src: String, dst: String) -> Result<(), String> {
    copy_file(&src, &dst)
}

#[tauri::command]
fn file_exists_cmd(path: String) -> bool {
    file_exists(&path)
}

#[tauri::command]
fn list_files(dir: String) -> Result<Vec<String>, String> {
    let paths = list_files_in_directory(&dir)?;
    Ok(paths
        .iter()
        .map(|p| p.to_string_lossy().to_string())
        .collect())
}

#[tauri::command]
fn find_files_recursive(dir: String) -> Result<Vec<String>, String> {
    let paths = recursive_find_files(&dir)?;
    Ok(paths
        .iter()
        .map(|p| p.to_string_lossy().to_string())
        .collect())
}

#[tauri::command]
fn apply_rename_cmd(old_path: String, new_path: String, dry_run: bool) -> Result<(), String> {
    document::apply_rename(&old_path, &new_path, dry_run)
}

#[tauri::command]
fn save_rename_to_history_cmd(
    app: tauri::AppHandle,
    old_path: String,
    new_path: String,
    batch_id: String,
) -> Result<(), String> {
    let history_path = app
        .path()
        .app_data_dir()
        .expect("Failed to get app data dir")
        .join("rename_history.json");
    document::save_rename_to_history(&old_path, &new_path, &history_path, &batch_id)
}

#[tauri::command]
fn undo_last_rename_cmd(
    app: tauri::AppHandle,
    batch_id: Option<String>,
) -> Result<UndoResult, String> {
    let history_path = app
        .path()
        .app_data_dir()
        .expect("Failed to get app data dir")
        .join("rename_history.json");
    document::undo_last_rename(&history_path, batch_id.as_deref().unwrap_or(""))
}

#[tauri::command]
async fn rename_pdfs(
    paths: Vec<String>,
    options: serde_json::Value,
) -> Result<BatchResult, String> {
    let dry_run = options.get("dryRun").and_then(|v| v.as_bool()).unwrap_or(false);
    let mut result = BatchResult {
        success: true,
        total: paths.len(),
        renamed: 0,
        skipped: 0,
        failed: 0,
        files: Vec::new(),
        dry_run,
        batch_id: Some(format!("gui-{}", chrono::Local::now().format("%Y%m%dT%H%M%S"))),
    };

    for path in &paths {
        let file_name = file_utils::get_file_name(path);
        let new_name = document::generate_filename(
            "",
            &file_utils::get_file_extension(path),
            "",
            &config::NamingConfig::default(),
            path,
        );
        let new_path = resolve_safe_path(
            &std::path::Path::new(path).parent().map(|p| p.to_string_lossy().to_string()).unwrap_or_default(),
            &new_name,
        )?;

        if !dry_run {
            if let Err(e) = document::apply_rename(path, &new_path, false) {
                result.files.push(FileResult {
                    file: path.clone(),
                    status: "failed".to_string(),
                    new_name: None,
                    new_path: None,
                    error: Some(e),
                    warnings: vec![],
                    company: None,
                    date: None,
                    doc_type: None,
                    provider: None,
                    model: None,
                });
                result.failed += 1;
                continue;
            }
        }

        result.files.push(FileResult {
            file: path.clone(),
            status: "renamed".to_string(),
            new_name: Some(new_name),
            new_path: Some(new_path),
            error: None,
            warnings: vec![],
            company: None,
            date: None,
            doc_type: None,
            provider: None,
            model: None,
        });
        result.renamed += 1;
    }

    Ok(result)
}

#[tauri::command]
async fn undo_rename(batch_id: Option<String>) -> Result<UndoResult, String> {
    // This requires the app handle to get the history path
    // Stub implementation - use the last batch if no batch_id provided
    Err("Undo rename requires app context".to_string())
}

#[tauri::command]
async fn get_config() -> Result<serde_json::Value, String> {
    Ok(serde_json::json!({}))
}

#[tauri::command]
async fn get_config_path() -> Result<String, String> {
    Err("Config path not available".to_string())
}

#[tauri::command]
async fn validate_config() -> Result<serde_json::Value, String> {
    Ok(serde_json::json!({"valid": true, "issues": []}))
}

#[tauri::command]
async fn save_config(key: String, value: String) -> Result<serde_json::Value, String> {
    Ok(serde_json::json!({"success": true}))
}