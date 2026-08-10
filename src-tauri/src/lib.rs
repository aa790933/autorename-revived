mod ai;
mod config;
mod document;
mod extractors;
mod file_utils;
mod portable;

use ai::{AiConfig, DocumentMetadata, TestConnectionResult};
use config::{AppConfig, ConfigBatchResult, load_config, save_config, save_config_batch};
use document::{BatchResult, FileResult, UndoResult, resolve_safe_path};
use file_utils::{
    copy_file, ensure_directory, file_exists, get_file_extension, get_file_name, get_file_stem,
    get_file_size, get_supported_extensions, is_image_extension, list_files_in_directory,
    preserve_extension, read_file_to_base64, read_file_to_bytes, recursive_find_files,
    validate_supported_extension,
};
use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicBool, Ordering};
use tauri::Manager;

static CANCEL_RENAME: AtomicBool = AtomicBool::new(false);

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
            is_portable_app,
            get_settings_path,
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
            cancel_rename,
            get_config,
            get_config_path,
            validate_config,
            save_config_cmd,
        ])
        .setup(|app| {
            let handle = app.handle();
            if let Err(e) = config::ensure_settings_directory(handle) {
                tracing::warn!("Failed to create settings directory: {}", e);
            }
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

#[tauri::command]
fn get_version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}

#[tauri::command]
fn is_portable_app() -> bool {
    crate::portable::is_portable()
}

#[tauri::command]
fn get_settings_path(app: tauri::AppHandle) -> String {
    crate::config::get_store_path(&app).to_string_lossy().to_string()
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

#[derive(Debug, Deserialize, Serialize)]
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
    language: Option<String>,
) -> Result<DocumentMetadata, String> {
    let lang = language.unwrap_or_else(|| "English".to_string());
    ai::extract_metadata_text(&text, &config, &lang).await
}

#[tauri::command]
async fn extract_metadata_from_vision(
    files: Vec<(String, Vec<u8>)>,
    config: AiConfig,
    language: Option<String>,
) -> Result<DocumentMetadata, String> {
    let lang = language.unwrap_or_else(|| "English".to_string());
    ai::extract_metadata_vision(&files, &config, &lang).await
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
    let history_path = config::get_settings_directory(&app).join("rename_history.json");
    document::save_rename_to_history(&old_path, &new_path, &history_path, &batch_id)
}

#[tauri::command]
fn undo_last_rename_cmd(
    app: tauri::AppHandle,
    batch_id: Option<String>,
) -> Result<UndoResult, String> {
    let history_path = config::get_settings_directory(&app).join("rename_history.json");
    document::undo_last_rename(&history_path, batch_id.as_deref().unwrap_or(""))
}

#[tauri::command]
async fn rename_pdfs(
    app: tauri::AppHandle,
    paths: Vec<String>,
    options: serde_json::Value,
) -> Result<BatchResult, String> {
    reset_cancel_flag();
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

    let vision_ai_config = {
        let mut vc = config.ai.clone();
        vc.provider = config.pdf.vision_provider.clone();
        if let Some(ref prov) = provider_override {
            vc.provider = prov.clone();
        }
        vc
    };

    let batch_id = format!(
        "gui-{}",
        chrono::Local::now().format(r#"%Y%m%dT%H%M%S"#)
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
        if is_cancelled() {
            result.files.push(FileResult {
                file: path.clone(),
                status: "failed".to_string(),
                new_name: None,
                new_path: None,
                error: Some("Rename cancelled by user".to_string()),
                warnings: vec![],
                company: None,
                date: None,
                doc_type: None,
                provider: None,
                model: None,
                suggestion_names: vec![],
                suggestion_languages: vec![],
            });
            result.failed += 1;
            continue;
        }

        let parent_dir = std::path::Path::new(path)
            .parent()
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_default();

        let file_bytes = match tokio::task::spawn_blocking({
            let path = path.clone();
            move || std::fs::read(path)
        })
        .await
        .map_err(|e| e.to_string())?
        {
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
                    suggestion_names: vec![],
                    suggestion_languages: vec![],
                });
                result.failed += 1;
                continue;
            }
        };

        let use_vision = match config.pdf.vision.as_str() {
            "true" => true,
            "false" => false,
            _ => false,
        };

        let current_model = ai::get_model_name(&ai_config);
        let languages = ai::get_all_languages(
            &config.naming.primary_language,
            &config.naming.suggestion_languages,
        );

        let all_metadata: Vec<ai::DocumentMetadata> = if extractors::is_image_extension(path) {
            ai::extract_metadata_vision_multi(
                &[(path.clone(), file_bytes.clone())],
                &vision_ai_config,
                &languages,
            )
            .await
        } else if extractors::is_text_extension(path) {
            let text = String::from_utf8_lossy(&file_bytes).to_string();
            ai::extract_metadata_text_multi(&text, &ai_config, &languages).await
        } else if extractors::is_office_extension(path) || extractors::is_pdf_extension(path) {
            let local_result = tokio::task::spawn_blocking({
                let file_path = path.clone();
                move || extractors::extract_text_from_file(&file_path)
            })
            .await
            .map_err(|e| e.to_string())?;
            match local_result {
                Ok((text, quality, method)) => {
                    tracing::info!(
                        "Local extraction for {}: method={}, quality={:.2}",
                        path,
                        method,
                        quality
                    );
                    if quality >= config.pdf.text_quality_threshold && !use_vision {
                        ai::extract_metadata_text_multi(&text, &ai_config, &languages).await
                    } else {
                        ai::extract_metadata_vision_multi(
                            &[(path.clone(), file_bytes.clone())],
                            &vision_ai_config,
                            &languages,
                        )
                        .await
                    }
                }
                Err(e) => {
                    tracing::warn!("Local extraction failed for {}: {}", path, e);
                    ai::extract_metadata_vision_multi(
                        &[(path.clone(), file_bytes.clone())],
                        &vision_ai_config,
                        &languages,
                    )
                    .await
                }
            }
        } else {
            ai::extract_metadata_vision_multi(
                &[(path.clone(), file_bytes.clone())],
                &vision_ai_config,
                &languages,
            )
            .await
        };

        let primary_meta = all_metadata.first().cloned().unwrap_or_default();
        let suggestion_metas: Vec<ai::DocumentMetadata> = all_metadata.into_iter().skip(1).collect();

        let ai_failed = primary_meta.company_name.is_empty()
            && primary_meta.document_type.is_empty()
            && primary_meta.document_date.is_empty();
        let ai_warning = if ai_failed {
            Some(format!(
                "AI metadata extraction failed for {} — file named with defaults. Check your API key, model name, and provider settings.",
                path
            ))
        } else {
            None
        };
        let company = document::harmonize_company_name(
            &primary_meta.company_name,
            &config.harmonized_companies,
        );
        let date = document::parse_document_date(&primary_meta.document_date).unwrap_or_default();

        let new_name = document::generate_filename(
            &company,
            &primary_meta.document_type,
            &date,
            &primary_meta.category,
            &primary_meta.subject,
            &config.naming,
            path,
        );

        let suggestion_names: Vec<String> = suggestion_metas
            .iter()
            .filter_map(|sm| {
                let s_company = document::harmonize_company_name(&sm.company_name, &config.harmonized_companies);
                let s_date = document::parse_document_date(&sm.document_date).unwrap_or_default();
                let s_name = document::generate_filename(
                    &s_company,
                    &sm.document_type,
                    &s_date,
                    &sm.category,
                    &sm.subject,
                    &config.naming,
                    path,
                );
                let s_ext = file_utils::get_file_extension(path);
                let s_final = if !s_ext.is_empty() && !s_name.ends_with(&format!(".{}", s_ext)) {
                    format!("{}.{}", s_name, s_ext)
                } else {
                    s_name.clone()
                };
                if s_name == new_name {
                    None
                } else {
                    Some(s_final)
                }
            })
            .collect();

        let suggestion_lang_labels: Vec<String> = config.naming.suggestion_languages.clone();

        let ext = file_utils::get_file_extension(path);
        let final_name = if !ext.is_empty() && !new_name.ends_with(&format!(".{}", ext)) {
            format!("{}.{}", new_name, ext)
        } else {
            new_name.clone()
        };

        let final_name = document::ensure_unique_filename(
            &parent_dir,
            &final_name,
            config.naming.sequence_zerofill,
        );

        let new_path = match resolve_safe_path(&parent_dir, &final_name) {
            Ok(p) => p,
             Err(e) => {
                result.files.push(FileResult {
                    file: path.clone(),
                    status: "failed".to_string(),
                    new_name: Some(final_name),
                    new_path: None,
                    error: Some(e),
                    warnings: ai_warning.iter().cloned().collect(),
                    company: Some(company.clone()),
                    date: Some(date.clone()),
                    doc_type: Some(primary_meta.document_type.clone()),
                    provider: Some(ai_config.provider.clone()),
                    model: Some(current_model.clone()),
                    suggestion_names: suggestion_names.clone(),
                    suggestion_languages: suggestion_lang_labels.clone(),
                });
                result.failed += 1;
                continue;
            }
        };

        let src_normalized = std::path::Path::new(path)
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_default();
        let dst_normalized = std::path::Path::new(&new_path)
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_default();
        if src_normalized == dst_normalized
            && std::path::Path::new(path)
                .parent()
                .and_then(|p| p.canonicalize().ok())
                == std::path::Path::new(&new_path)
                    .parent()
                    .and_then(|p| p.canonicalize().ok())
        {
            result.files.push(FileResult {
                file: path.clone(),
                status: "skipped".to_string(),
                new_name: Some(final_name),
                new_path: Some(new_path.clone()),
                error: None,
                warnings: ai_warning.iter().cloned().chain(std::iter::once("Already matches target name".to_string())).collect(),
                company: Some(company.clone()),
                date: Some(date.clone()),
                doc_type: Some(primary_meta.document_type.clone()),
                provider: Some(ai_config.provider.clone()),
                model: Some(current_model.clone()),
                suggestion_names: suggestion_names.clone(),
                suggestion_languages: suggestion_lang_labels.clone(),
            });
            result.skipped += 1;
            continue;
        }

        if !dry_run {
            let old_path = path.clone();
            let new_path_clone = new_path.clone();
            if let Err(e) = tokio::task::spawn_blocking(move || {
                document::apply_rename(&old_path, &new_path_clone, false)
            })
            .await
            .map_err(|e| e.to_string())?
            {
                result.files.push(FileResult {
                    file: path.clone(),
                    status: "failed".to_string(),
                    new_name: Some(final_name),
                    new_path: Some(new_path_clone.clone()),
                    error: Some(e),
                    warnings: ai_warning.iter().cloned().collect(),
                    company: Some(company.clone()),
                    date: Some(date.clone()),
                    doc_type: Some(primary_meta.document_type.clone()),
                    provider: Some(ai_config.provider.clone()),
                    model: Some(current_model.clone()),
                    suggestion_names: suggestion_names.clone(),
                    suggestion_languages: suggestion_lang_labels.clone(),
                });
                result.failed += 1;
                continue;
            }

            if config.undo.enabled {
                let history_path = config::get_settings_directory(&app).join("rename_history.json");
                let old_path = path.clone();
                let new_path_clone2 = new_path.clone();
                let batch_id_clone = batch_id.clone();
                if let Err(e) = tokio::task::spawn_blocking(move || {
                    document::save_rename_to_history(&old_path, &new_path_clone2, &history_path, &batch_id_clone)
                })
                .await
                .map_err(|e| e.to_string())?
                {
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
            warnings: ai_warning.iter().cloned().collect(),
            company: Some(company),
            date: Some(date),
            doc_type: Some(primary_meta.document_type),
            provider: Some(ai_config.provider.clone()),
            model: Some(current_model.clone()),
            suggestion_names: suggestion_names.clone(),
            suggestion_languages: suggestion_lang_labels.clone(),
        });
        result.renamed += 1;
    }

    result.success = result.failed == 0;
    reset_cancel_flag();
    Ok(result)
}

#[tauri::command]
async fn undo_rename(
    app: tauri::AppHandle,
    batch_id: Option<String>,
) -> Result<UndoResult, String> {
    let history_path = config::get_settings_directory(&app).join("rename_history.json");
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
        "gemini" => !config.ai.api_key.is_empty(),
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

#[tauri::command]
fn cancel_rename() -> bool {
    CANCEL_RENAME.store(true, Ordering::SeqCst);
    true
}

fn reset_cancel_flag() {
    CANCEL_RENAME.store(false, Ordering::SeqCst);
}

fn is_cancelled() -> bool {
    CANCEL_RENAME.load(Ordering::SeqCst)
}


