mod ai;
mod config;
mod document;
mod file_utils;

use ai::{AiConfig, DocumentMetadata, TestConnectionResult};
use config::{AppConfig, ConfigBatchResult, load_config, save_config, save_config_batch};
use document::{BatchResult, FileResult, UndoResult};
use file_utils::{
    copy_file, ensure_directory, file_exists, get_file_extension, get_file_name, get_file_size,
    get_supported_extensions, is_image_extension, list_files_in_directory,
    preserve_extension, read_file_to_base64, read_file_to_bytes, recursive_find_files,
    resolve_safe_path, validate_supported_extension,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
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
            save_config_cmd,
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

#[derive(Debug, Deserialize)]
struct ConfigUpdate {
    key: String,
    value: String,
}

#[tauri::command]
async fn save_app_config_batch(
    app: tauri::AppHandle,
    updates: Vec<ConfigUpdate>,
) -> Result<ConfigBatchResult, String> {
    let pairs: Vec<(String, String)> = updates.into_iter().map(|u| (u.key, u.value)).collect();
    save_config_batch(app, pairs).await
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

fn is_text_extension(path: &str) -> bool {
    let ext = std::path::Path::new(path)
        .extension()
        .map(|e| e.to_string_lossy().to_lowercase())
        .unwrap_or_default();
    matches!(
        ext.as_str(),
        "txt" | "csv" | "md" | "rtf" | "json" | "xml" | "html" | "htm"
    )
}

#[tauri::command]
async fn rename_pdfs(
    app: tauri::AppHandle,
    paths: Vec<String>,
    options: serde_json::Value,
) -> Result<BatchResult, String> {
    let dry_run = options
        .get("dryRun")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    let provider_override = options
        .get("provider")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    let config = load_config(app.clone()).await?;

    let mut ai_config = config.ai.clone();
    if let Some(ref prov) = provider_override {
        ai_config.provider = prov.clone();
    }

    let batch_id = format!(
        "gui-{}",
        chrono::Local::now().format("%Y%m%dT%H%M%S")
    );

    let mut result = BatchResult {
        success: true,
        total: paths.len(),
        renamed: 0,
        skipped: 0,
        failed: 0,
        files: Vec::new(),
        dry_run,
        batch_id: Some(batch_id.clone()),
    };

    for path in &paths {
        let file_name = file_utils::get_file_name(path);
        let parent_dir = std::path::Path::new(path)
            .parent()
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_default();

        let file_bytes = match std::fs::read(path) {
            Ok(b) => b,
            Err(e) => {
                result.files.push(FileResult {
                    file: path.clone(),
                    status: "failed".to_string(),
                    new_name: None,
                    new_path: None,
                    error: Some(format!("Failed to read file: {}", e)),
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
        };

        let metadata = if is_text_extension(path) {
            let text = String::from_utf8_lossy(&file_bytes).to_string();
            match ai::extract_metadata_text(&text, &ai_config).await {
                Ok(m) => Some(m),
                Err(e) => {
                    tracing::warn!("AI text extraction failed for {}: {}", path, e);
                    None
                }
            }
        } else {
            match ai::extract_metadata_vision(&[(path.clone(), file_bytes)], &ai_config).await {
                Ok(m) => Some(m),
                Err(e) => {
                    tracing::warn!("AI vision extraction failed for {}: {}", path, e);
                    None
                }
            }
        };

        let meta = metadata.unwrap_or_default();
        let company = document::harmonize_company_name(
            &meta.company_name,
            &config.harmonized_companies,
        );
        let date = document::parse_document_date(&meta.document_date).unwrap_or_default();

        let new_name = document::generate_filename(
            &company,
            &meta.document_type,
            &date,
            &config.naming,
            path,
        );

        let ext = file_utils::get_file_extension(path);
        let final_name = if !ext.is_empty() && !new_name.ends_with(&format!(".{}", ext)) {
            format!("{}.{}", new_name, ext)
        } else {
            new_name.clone()
        };

        let new_path = match resolve_safe_path(&parent_dir, &final_name) {
            Ok(p) => p,
            Err(e) => {
                result.files.push(FileResult {
                    file: path.clone(),
                    status: "failed".to_string(),
                    new_name: Some(final_name),
                    new_path: None,
                    error: Some(e),
                    warnings: vec![],
                    company: Some(company.clone()),
                    date: Some(date.clone()),
                    doc_type: Some(meta.document_type.clone()),
                    provider: Some(ai_config.provider.clone()),
                    model: None,
                });
                result.failed += 1;
                continue;
            }
        };

        if std::path::Path::new(path)
            .canonicalize()
            .ok()
            .and_then(|p| p.to_string_lossy().to_string().into())
            .as_ref()
            == new_path.canonicalize().ok().as_ref().map(|p| p.to_string_lossy().to_string()).as_ref()
        {
            result.files.push(FileResult {
                file: path.clone(),
                status: "skipped".to_string(),
                new_name: Some(final_name),
                new_path: Some(new_path.clone()),
                error: None,
                warnings: vec!["Already matches target name".to_string()],
                company: Some(company.clone()),
                date: Some(date.clone()),
                doc_type: Some(meta.document_type.clone()),
                provider: Some(ai_config.provider.clone()),
                model: None,
            });
            result.skipped += 1;
            continue;
        }

        if !dry_run {
            if let Err(e) = document::apply_rename(path, &new_path, false) {
                result.files.push(FileResult {
                    file: path.clone(),
                    status: "failed".to_string(),
                    new_name: Some(final_name),
                    new_path: Some(new_path),
                    error: Some(e),
                    warnings: vec![],
                    company: Some(company.clone()),
                    date: Some(date.clone()),
                    doc_type: Some(meta.document_type.clone()),
                    provider: Some(ai_config.provider.clone()),
                    model: None,
                });
                result.failed += 1;
                continue;
            }

            if config.undo.enabled {
                let history_path = app
                    .path()
                    .app_data_dir()
                    .expect("Failed to get app data dir")
                    .join("rename_history.json");
                if let Err(e) = document::save_rename_to_history(path, &new_path, &history_path, &batch_id) {
                    tracing::warn!("Failed to save undo history: {}", e);
                }
            }
        }

        result.files.push(FileResult {
            file: path.clone(),
            status: "renamed".to_string(),
            new_name: Some(final_name),
            new_path: Some(new_path),
            error: None,
            warnings: vec![],
            company: Some(company),
            date: Some(date),
            doc_type: Some(meta.document_type),
            provider: Some(ai_config.provider.clone()),
            model: None,
        });
        result.renamed += 1;
    }

    result.success = result.failed == 0;
    Ok(result)
}

#[tauri::command]
async fn undo_rename(
    app: tauri::AppHandle,
    batch_id: Option<String>,
) -> Result<UndoResult, String> {
    let history_path = app
        .path()
        .app_data_dir()
        .map_err(|e| e.to_string())?
        .join("rename_history.json");
    document::undo_last_rename(&history_path, batch_id.as_deref().unwrap_or(""))
}

#[tauri::command]
async fn get_config(app: tauri::AppHandle) -> Result<serde_json::Value, String> {
    let config = load_config(app).await?;
    serde_json::to_value(config).map_err(|e| e.to_string())
}

#[tauri::command]
async fn get_config_path(app: tauri::AppHandle) -> Result<String, String> {
    let path = config::get_store_path(&app);
    Ok(path.to_string_lossy().to_string())
}

#[tauri::command]
async fn validate_config(app: tauri::AppHandle) -> Result<serde_json::Value, String> {
    let config = load_config(app).await?;
    let mut issues: Vec<serde_json::Value> = Vec::new();

    if config.ai.provider.is_empty() {
        issues.push(serde_json::json!({
            "field": "ai.provider",
            "level": "error",
            "message": "AI provider is required"
        }));
    }

    let has_key = match config.ai.provider.as_str() {
        "gemini" => !config.ai.gemini_api_key.is_empty(),
        "openai" => !config.ai.api_key.is_empty(),
        "anthropic" => !config.ai.api_key.is_empty(),
        "ollama" => true,
        "xai" => !config.ai.api_key.is_empty(),
        "custom" => true,
        _ => false,
    };
    if !has_key {
        issues.push(serde_json::json!({
            "field": "ai.api_key",
            "level": "warning",
            "message": "No API key configured for selected provider"
        }));
    }

    let valid = issues.iter().all(|i| i["level"] != "error");

    Ok(serde_json::json!({
        "valid": valid,
        "issues": issues
    }))
}

#[tauri::command]
async fn save_config_cmd(
    app: tauri::AppHandle,
    key: String,
    value: String,
) -> Result<serde_json::Value, String> {
    let mut cfg = load_config(app.clone()).await?;
    config::apply_config_update(&mut cfg, &key, &value).map_err(|e| e.to_string())?;
    save_config(app, &cfg).await?;
    Ok(serde_json::json!({"success": true}))
}
