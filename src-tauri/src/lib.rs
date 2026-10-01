mod ai;
mod config;
mod document;
mod extractors;
mod portable;

use ai::{AiConfig, DocumentMetadata, TestConnectionResult};
use config::{
    AppConfig, ConfigBatchResult, NamingConfig, load_config, save_config, save_config_batch,
};
use document::{BatchResult, FileResult, UndoHistory, UndoResult, resolve_safe_path};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::panic::AssertUnwindSafe;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use tokio::task::JoinSet;

static CANCEL_RENAME: AtomicBool = AtomicBool::new(false);

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_store::Builder::new().build())
        .invoke_handler(tauri::generate_handler![
            get_version,
            is_portable_app,
            get_settings_path,
            get_supported_extensions_list,
            get_undo_log_path,
            load_app_config,
            save_app_config,
            save_app_config_batch,
            test_connection,
            rename_files,
            undo_rename,
            cancel_rename,
            get_config,
            get_config_path,
            validate_config,
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
fn get_settings_path(app: tauri::AppHandle) -> Result<String, String> {
    Ok(crate::config::get_store_path(&app)?
        .to_string_lossy()
        .to_string())
}

/// Supported file extensions, mirroring the frontend's `SUPPORTED_EXTENSIONS`.
#[tauri::command]
fn get_supported_extensions_list() -> Vec<String> {
    extractors::supported_extensions()
        .iter()
        .map(|s| s.to_string())
        .collect()
}

/// Directory holding the undo log, as resolved by the backend (portable-aware
/// and honoring `undo.log_path`). The frontend must use this instead of
/// guessing `appDataDir`, which breaks portable mode and custom log paths.
#[tauri::command]
async fn get_undo_log_path(app: tauri::AppHandle) -> Result<String, String> {
    let config = load_config(app.clone()).await?;
    let path = config::resolve_undo_log_path(&app, &config)?;
    let dir = path
        .parent()
        .map(|p| p.to_path_buf())
        .unwrap_or_else(|| path.clone());
    Ok(dir.to_string_lossy().to_string())
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
    app: tauri::AppHandle,
    provider: Option<String>,
    api_key: Option<String>,
    model: Option<String>,
) -> TestConnectionResult {
    // Start from the *saved* configuration so the probe exercises the real
    // base URLs / model wiring, then apply the live UI overrides.
    let mut config = match load_config(app).await {
        Ok(cfg) => cfg.ai,
        Err(_) => AiConfig::default(),
    };
    if let Some(p) = provider.filter(|p| !p.trim().is_empty()) {
        config.provider = p;
    }
    if let Some(k) = api_key.filter(|k| !k.trim().is_empty()) {
        config.api_key = k;
    }
    if let Some(m) = model.filter(|m| !m.trim().is_empty()) {
        match config.provider.as_str() {
            "gemini" => config.gemini_model = m,
            "openai" | "anthropic" => config.model = m,
            _ => config.custom_model = m,
        }
    }
    ai::test_connection(&config).await
}

#[tauri::command]
fn cancel_rename() -> bool {
    CANCEL_RENAME.store(true, Ordering::SeqCst);
    true
}

fn reset_cancel_flag() {
    CANCEL_RENAME.store(false, Ordering::SeqCst);
}

pub(crate) fn is_cancelled() -> bool {
    CANCEL_RENAME.load(Ordering::SeqCst)
}

// ---------------------------------------------------------------------------
// rename_files pipeline
// ---------------------------------------------------------------------------

/// Builds the cancelled/skipped FileResult pushed when a batch is interrupted.
fn cancelled_file_result(path: &str) -> FileResult {
    failed_file_result(path, "Rename cancelled by user".to_string())
}

/// A failed FileResult with all optional fields empty.
fn failed_file_result(path: &str, error: String) -> FileResult {
    FileResult {
        file: path.to_string(),
        status: "failed".to_string(),
        new_name: None,
        new_path: None,
        error: Some(error),
        warnings: vec![],
        company: None,
        date: None,
        doc_type: None,
        provider: None,
        model: None,
        suggestion_names: vec![],
        suggestion_languages: vec![],
    }
}

/// True when local text is good enough to skip vision under `auto` mode.
fn text_quality_sufficient(quality: f64, threshold: f64) -> bool {
    quality >= threshold && quality > 0.0
}

/// Append the original extension to a generated stem unless it already has it.
fn with_extension(mut name: String, ext: &str) -> String {
    if ext.is_empty() || name.to_lowercase().ends_with(&format!(".{}", ext.to_lowercase())) {
        return name;
    }
    name.push('.');
    name.push_str(ext);
    name
}

/// Refuse to read files that would exhaust memory (or hang extraction) —
/// documents that big are not the target of this tool anyway.
const MAX_FILE_BYTES: u64 = 100 * 1024 * 1024;

/// Inputs shared by every file in one batch. Built once per run and
/// cheap-cloned into the concurrent analysis tasks via `Arc`.
struct Pipeline {
    ai_config: AiConfig,
    vision_ai_config: AiConfig,
    languages: Vec<String>,
    company_lookup: HashMap<String, String>,
    naming: NamingConfig,
    vision_mode: String,
    text_quality_threshold: f64,
    provider: String,
    current_model: String,
    undo_enabled: bool,
    dry_run: bool,
    batch_id: String,
}

/// Per-file analysis output: everything needed to build the result and, in the
/// sequential commit phase, to perform the rename and record the undo entry.
struct Prepared {
    path: String,
    parent_dir: String,
    target_name: String,
    warnings: Vec<String>,
    company: String,
    date: String,
    doc_type: String,
    suggestion_names: Vec<String>,
    suggestion_lang_labels: Vec<String>,
}

impl Pipeline {
    /// The slow half of the work for one file (read + extract + AI), safe to
    /// run concurrently for multiple files because it never mutates anything.
    async fn analyze(&self, path: String) -> Result<Prepared, FileResult> {
        if is_cancelled() {
            return Err(cancelled_file_result(&path));
        }

        let parent_dir = std::path::Path::new(&path)
            .parent()
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_default();

        let meta = match std::fs::metadata(&path) {
            Ok(m) => m,
            Err(e) => return Err(failed_file_result(&path, format!("Failed to read file: {}", e))),
        };
        if !meta.is_file() {
            return Err(failed_file_result(&path, "Path is not a regular file".to_string()));
        }
        // Reject non-document extensions before spending any AI call on them.
        if !extractors::is_supported_extension(&path) {
            return Err(failed_file_result(
                &path,
                "Unsupported file type — only images, text, Office documents and PDFs are renamed"
                    .to_string(),
            ));
        }
        if meta.len() > MAX_FILE_BYTES {
            return Err(failed_file_result(
                &path,
                format!(
                    "File is too large to process ({} MB exceeds the 100 MB limit)",
                    meta.len() / (1024 * 1024)
                ),
            ));
        }

        let file_bytes = match tokio::task::spawn_blocking({
            let path = path.clone();
            move || std::fs::read(path)
        })
        .await
        {
            Ok(Ok(b)) => b,
            Ok(Err(e)) => {
                return Err(failed_file_result(&path, format!("Failed to read file: {}", e)));
            }
            Err(join_err) => {
                return Err(failed_file_result(
                    &path,
                    format!("File read task failed: {}", join_err),
                ));
            }
        };

        let all_metadata = self.extract(&path, &file_bytes).await;

        if is_cancelled() {
            return Err(cancelled_file_result(&path));
        }

        // An empty Vec means the primary extraction failed outright (API
        // error, vision disabled…) — fail the file instead of renaming it to
        // a bogus name built from empty metadata. The multi-language helpers
        // guarantee index alignment, so index 0 is always the primary result.
        if all_metadata.is_empty() {
            return Err(failed_file_result(
                &path,
                format!(
                    "AI metadata extraction failed for {} — check your API key, model name, vision setting, and provider configuration.",
                    path
                ),
            ));
        }

        let primary_meta = all_metadata.first().cloned().unwrap_or_default();

        let company = document::harmonize_with_lookup(&primary_meta.company_name, &self.company_lookup);
        let date = document::parse_document_date(&primary_meta.document_date).unwrap_or_default();
        let (meta_is_error, meta_warnings) = document::validate_metadata(
            &company,
            &primary_meta.document_type,
            &date,
            &primary_meta.subject,
            primary_meta.is_unreadable_or_error,
        );

        let mut warnings = meta_warnings;
        if meta_is_error {
            warnings.push(format!(
                "AI metadata extraction failed for {} — company='{}', doctype='{}', date='{}', subject='{}', unreadable={}. Check your API key, model name, and provider settings.",
                path,
                &primary_meta.company_name,
                &primary_meta.document_type,
                &primary_meta.document_date,
                &primary_meta.subject,
                primary_meta.is_unreadable_or_error
            ));
        }

        let ext = std::path::Path::new(&path)
            .extension()
            .map(|e| e.to_string_lossy().to_string())
            .unwrap_or_default();

        let new_name = document::generate_filename(
            &company,
            &primary_meta.document_type,
            &date,
            &primary_meta.subject,
            &self.naming,
            &path,
            primary_meta.is_unreadable_or_error,
        );
        let final_name_stem = with_extension(new_name.clone(), &ext);

        // Suggestions: index i of `all_metadata` (i >= 1) corresponds to
        // `naming.suggestion_languages[i - 1]`. Carry both the name and its
        // language label together — the previous version collected names
        // while returning the *full* configured language list, so labels
        // misaligned whenever a suggestion was dropped.
        let mut suggestion_names = Vec::new();
        let mut suggestion_lang_labels = Vec::new();
        for (i, sm) in all_metadata.iter().enumerate().skip(1) {
            let Some(lang) = self.naming.suggestion_languages.get(i - 1).cloned() else {
                continue;
            };
            if sm.is_empty() || sm.is_unreadable_or_error {
                continue;
            }
            let s_company = document::harmonize_with_lookup(&sm.company_name, &self.company_lookup);
            let s_date = document::parse_document_date(&sm.document_date).unwrap_or_default();
            let s_name = document::generate_filename(
                &s_company,
                &sm.document_type,
                &s_date,
                &sm.subject,
                &self.naming,
                &path,
                sm.is_unreadable_or_error,
            );
            let s_final = with_extension(s_name, &ext);
            if s_final == final_name_stem {
                continue;
            }
            suggestion_names.push(s_final);
            suggestion_lang_labels.push(lang);
        }

        Ok(Prepared {
            path,
            parent_dir,
            target_name: final_name_stem,
            warnings,
            company,
            date,
            doc_type: primary_meta.document_type,
            suggestion_names,
            suggestion_lang_labels,
        })
    }

    /// Resolve vision mode: `true` forces vision, `auto` falls back to vision when
    /// local text extraction is poor, `false` (and anything else) never uses it.
    fn vision_allowed(&self) -> bool {
        self.vision_mode != "false"
    }

    /// Pick the extraction strategy for one file. Returned Vec follows the
    /// `extract_metadata_*_multi` alignment contract (index = language).
    async fn extract(&self, path: &str, file_bytes: &[u8]) -> Vec<DocumentMetadata> {
        if extractors::is_image_extension(path) {
            // Images have no text layer: vision is the only way in. With
            // vision disabled the file cannot be processed — say so instead
            // of calling an AI that will be misconfigured for images.
            if !self.vision_allowed() {
                tracing::warn!(
                    "Image {} cannot be processed while vision AI is disabled",
                    path
                );
                return Vec::new();
            }
            return self.vision_multi(path, file_bytes).await;
        }

        if extractors::is_text_extension(path) {
            let text = extractors::decode_bytes_to_string(file_bytes);
            return ai::extract_metadata_text_multi(&text, &self.ai_config, &self.languages).await;
        }

        if extractors::is_office_extension(path) || extractors::is_pdf_extension(path) {
            return self.process_office_or_pdf(path, file_bytes).await;
        }

        // Anything else (`.zip`, `.exe`, no extension...) is not a document
        // this app can read. The old code fired a vision call at it anyway;
        // rejecting here keeps the failure message actionable and saves API
        // spend. The frontend filters the same list before dropping files.
        tracing::warn!("Unsupported file type for renaming: {}", path);
        Vec::new()
    }

    async fn vision_multi(&self, path: &str, file_bytes: &[u8]) -> Vec<DocumentMetadata> {
        ai::extract_metadata_vision_multi(
            &[(path.to_string(), file_bytes.to_vec())],
            &self.vision_ai_config,
            &self.languages,
        )
        .await
    }

    /// Extraction strategy for Office documents and PDFs (the `auto` pipeline).
    ///
    /// - Extract text locally and measure its quality.
    /// - Good text → text AI.
    /// - Poor/no text, or local extraction failed → vision AI when enabled or
    ///   in `auto` mode; otherwise fall back to text AI on whatever was
    ///   extracted (empty text fails the file through validation).
    async fn process_office_or_pdf(
        &self,
        path: &str,
        file_bytes: &[u8],
    ) -> Vec<DocumentMetadata> {
        // The bytes are already in memory: handing them to the extractor
        // avoids reading the file from disk a second time per document.
        let local_result = tokio::task::spawn_blocking({
            let file_path = path.to_string();
            let bytes = file_bytes.to_vec();
            move || extractors::extract_text_from_bytes(&file_path, &bytes)
        })
        .await;

        let local_result = match local_result {
            Ok(inner) => inner,
            Err(join_err) => {
                tracing::warn!(
                    "spawn_blocking failed for {}: {} — falling back to vision AI",
                    path,
                    join_err
                );
                Err(join_err.to_string())
            }
        };

        // `force_vision` implies the mode is not "false", so one predicate
        // covers every "may we call vision?" question.
        let vision_mode_ok = self.vision_allowed();
        let force_vision = self.vision_mode == "true";

        match local_result {
            Ok((text, quality, method)) => {
                tracing::info!(
                    "Local extraction for {}: method={}, quality={:.2}",
                    path,
                    method,
                    quality
                );

                if quality == 0.0 {
                    // No text layer at all. Without vision there is nothing
                    // to send the text AI: fail the file (the frontend shows
                    // an actionable error) instead of burning an API call on
                    // empty text and accepting hallucinated metadata.
                    if !vision_mode_ok {
                        tracing::warn!(
                            "PDF/Office file has no text layer (quality=0.0) and vision AI is disabled: {}",
                            path
                        );
                        return Vec::new();
                    }
                    return self.vision_multi(path, file_bytes).await;
                }

                if force_vision {
                    return self.vision_multi(path, file_bytes).await;
                }

                if text_quality_sufficient(quality, self.text_quality_threshold) {
                    ai::extract_metadata_text_multi(&text, &self.ai_config, &self.languages).await
                } else if vision_mode_ok {
                    tracing::info!(
                        "Text quality {:.2} below threshold {:.2} — using vision AI for {}",
                        quality,
                        self.text_quality_threshold,
                        path
                    );
                    self.vision_multi(path, file_bytes).await
                } else {
                    ai::extract_metadata_text_multi(&text, &self.ai_config, &self.languages).await
                }
            }
            Err(e) => {
                tracing::warn!("Local extraction failed for {}: {}", path, e);
                if vision_mode_ok {
                    self.vision_multi(path, file_bytes).await
                } else {
                    Vec::new()
                }
            }
        }
    }

    /// The fast half: pick a collision-free name, rename, record the undo
    /// entry. Runs strictly sequentially (single consumer task), so name
    /// uniqueness and the on-disk rename are atomic with respect to this
    /// batch. Returns the FileResult for `prepared`.
    async fn commit(&self, prepared: Prepared, history: &mut UndoHistory) -> FileResult {
        let Prepared {
            path,
            parent_dir,
            target_name,
            warnings,
            company,
            date,
            doc_type,
            suggestion_names,
            suggestion_lang_labels,
        } = prepared;

        let base = FileResult {
            file: path.clone(),
            status: String::new(),
            new_name: None,
            new_path: None,
            error: None,
            warnings,
            company: Some(company),
            date: Some(date),
            doc_type: Some(doc_type),
            provider: Some(self.provider.clone()),
            model: Some(self.current_model.clone()),
            suggestion_names,
            suggestion_languages: suggestion_lang_labels,
        };

        let final_name = match document::ensure_unique_filename(
            &parent_dir,
            &target_name,
            self.naming.sequence_zerofill,
        ) {
            Ok(name) => name,
            Err(e) => {
                return FileResult {
                    status: "failed".to_string(),
                    new_name: Some(target_name),
                    error: Some(e),
                    ..base
                };
            }
        };

        let new_path = match resolve_safe_path(
            &parent_dir,
            &final_name,
            self.naming.max_length as usize,
        ) {
            Ok(p) => p,
            Err(e) => FileResult {
                status: "failed".to_string(),
                new_name: Some(final_name),
                error: Some(e),
                ..base
            },
        };

        let src_name = std::path::Path::new(&path)
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_default();
        let dst_name = std::path::Path::new(&new_path)
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_default();
        // `resolve_safe_path` canonicalized the directory and
        // `ensure_unique_filename` guarantees the name is free inside it, so
        // comparing file names is sufficient. The previous parent-vs-parent
        // `canonicalize()` comparison compared two paths that are equal by
        // construction, and could disagree on a `\\?\` prefix.
        if src_name == dst_name {
            let mut warnings = base.warnings.clone();
            warnings.push("Already matches target name".to_string());
            return FileResult {
                status: "skipped".to_string(),
                new_name: Some(final_name),
                new_path: Some(new_path),
                warnings,
                ..base
            };
        }

        if is_cancelled() {
            return FileResult {
                status: "failed".to_string(),
                new_name: Some(final_name),
                error: Some("Rename cancelled by user".to_string()),
                ..base
            };
        }

        // Dry run: report the target name without touching disk.
        if self.dry_run {
            return FileResult {
                status: "completed".to_string(),
                new_name: Some(final_name),
                new_path: Some(new_path),
                ..base
            };
        }

        let old_path = path.clone();
        let new_path_for_rename = new_path.clone();
        let rename_result = tokio::task::spawn_blocking(move || {
            document::apply_rename(&old_path, &new_path_for_rename, false)
        })
        .await;
        let rename_err = match rename_result {
            Ok(Ok(())) => None,
            Ok(Err(e)) => Some(e),
            Err(join_err) => Some(format!("Rename task failed: {}", join_err)),
        };
        if let Some(e) = rename_err {
            return FileResult {
                status: "failed".to_string(),
                new_name: Some(final_name),
                new_path: Some(new_path),
                error: Some(e),
                ..base
            };
        }

        if self.undo_enabled {
            history.add_entry(&self.batch_id, &path, &new_path);
        }

        FileResult {
            status: "completed".to_string(),
            new_name: Some(final_name),
            new_path: Some(new_path),
            ..base
        }
    }
}

#[tauri::command]
async fn rename_files(
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
        vc.provider = config.document.vision_provider.clone();
        if let Some(ref prov) = provider_override {
            vc.provider = prov.clone();
        }
        vc
    };

    let batch_id = format!(
        "gui-{}",
        chrono::Local::now().format(r#"%Y%m%dT%H%M%S"#)
    );

    // Resolve the undo log location once (honors undo.log_path + portable).
    let history_path = if config.undo.enabled && !dry_run {
        Some(config::resolve_undo_log_path(&app, &config)?)
    } else {
        None
    };
    let mut history = match &history_path {
        Some(p) => {
            let path = p.clone();
            tokio::task::spawn_blocking(move || UndoHistory::load(&path))
                .await
                .map_err(|e| format!("History load task failed: {}", e))??
        }
        None => UndoHistory::new(),
    };

    let total = paths.len();
    let mut result = BatchResult {
        success: true,
        total,
        completed: 0,
        skipped: 0,
        failed: 0,
        files: Vec::new(),
        dry_run,
        batch_id: Some(batch_id.clone()),
    };

    // Load + naming inputs are identical for every file: hoisted here (the
    // old loop rebuilt the company-variation map per file and re-asked the AI
    // layer for the model name on every iteration).
    let current_model = ai::get_model_name(&ai_config);
    let provider = ai_config.provider.clone();
    let pipeline = Arc::new(Pipeline {
        ai_config,
        vision_ai_config,
        languages: ai::get_all_languages(
            &config.naming.primary_language,
            &config.naming.suggestion_languages,
        ),
        company_lookup: document::build_company_lookup(&config.harmonized_companies),
        naming: config.naming.clone(),
        vision_mode: config.document.vision.clone(),
        text_quality_threshold: config.document.text_quality_threshold,
        provider,
        current_model,
        undo_enabled: config.undo.enabled,
        dry_run,
        batch_id: batch_id.clone(),
    });

    // Bounded-concurrency analysis (network + decode bound), sequential
    // commit (renames are cheap, and keeping them serialized makes undo
    // history and filename uniqueness race-free). `max_workers` is honored.
    let workers = (config.max_workers as usize).clamp(1, 32).min(total.max(1));
    let mut inflight: JoinSet<(usize, Result<Prepared, FileResult>)> = JoinSet::new();
    let mut outcomes: Vec<Option<FileResult>> = (0..total).map(|_| None).collect();
    let mut next = 0usize;

    while next < total || !inflight.is_empty() {
        if is_cancelled() {
            break;
        }
        while next < total && inflight.len() < workers {
            let path = paths[next].clone();
            let index = next;
            next += 1;
            let pipeline = Arc::clone(&pipeline);
            inflight.spawn(async move {
                // Catch a panic inside the analysis of *one* file so the batch
                // keeps going and the offending path is still reported — the
                // previous version lost the index with the panicked task and
                // only produced a generic "failed unexpectedly".
                let worker_path = path.clone();
                let analyzed = AssertUnwindSafe(pipeline.analyze(path))
                    .catch_unwind()
                    .await;
                let outcome = match analyzed {
                    Ok(result) => result,
                    Err(_) => Err(failed_file_result(
                        &worker_path,
                        "Internal error while analysing this file (worker panicked)".to_string(),
                    )),
                };
                (index, outcome)
            });
        }
        if inflight.is_empty() {
            break;
        }
        match inflight.join_next().await {
            Some(Ok((index, Ok(prepared)))) => {
                let file_result = pipeline.commit(prepared, &mut history).await;
                outcomes[index] = Some(file_result);
            }
            Some(Ok((index, Err(failed)))) => {
                outcomes[index] = Some(failed);
            }
            Some(Err(join_err)) => {
                // The task itself could not be joined (cancelled or aborted
                // between spawn and join); the affected slot is filled by the
                // sweep below.
                tracing::error!("Rename worker failed to join: {}", join_err);
            }
            None => break,
        }
    }
    inflight.abort_all();

    let cancelled = is_cancelled();
    for (index, slot) in outcomes.iter_mut().enumerate() {
        if slot.is_none() {
            *slot = Some(if cancelled {
                cancelled_file_result(&paths[index])
            } else {
                failed_file_result(&paths[index], "Processing failed unexpectedly".to_string())
            });
        }
    }

    // Keep the results in the original input order the UI expects.
    let files: Vec<FileResult> = outcomes.into_iter().flatten().collect();
    result.completed = files.iter().filter(|f| f.status == "completed").count();
    result.skipped = files.iter().filter(|f| f.status == "skipped").count();
    result.failed = files.iter().filter(|f| f.status == "failed").count();
    result.files = files;
    result.success = result.failed == 0;

    // One history rewrite per batch (the old code reloaded and rewrote the
    // whole JSON log after every renamed file — O(files * log size)).
    if let Some(path) = &history_path {
        if history
            .batches
            .iter()
            .any(|b| b.batch_id == batch_id && !b.files.is_empty())
        {
            let trimmed = config.undo.max_entries.max(1) as usize;
            let path = path.clone();
            let save = tokio::task::spawn_blocking(move || {
                let mut history = history;
                history.trim(trimmed);
                history.save(&path)
            })
            .await;
            match save {
                Ok(Ok(())) => {}
                Ok(Err(e)) => tracing::error!("Failed to save undo history: {}", e),
                Err(e) => tracing::error!("Undo history save task failed: {}", e),
            }
        }
    }

    reset_cancel_flag();
    Ok(result)
}

#[tauri::command]
async fn undo_rename(
    app: tauri::AppHandle,
    batch_id: Option<String>,
) -> Result<UndoResult, String> {
    let config = load_config(app.clone()).await?;
    let history_path = config::resolve_undo_log_path(&app, &config)?;
    // Undo walks the whole batch on the blocking pool: it can touch hundreds
    // of files and must not stall the async runtime.
    tokio::task::spawn_blocking(move || {
        document::undo_last_rename(&history_path, batch_id.as_deref().unwrap_or(""))
    })
    .await
    .map_err(|e| format!("Undo task failed: {}", e))?
}

#[tauri::command]
async fn get_config(app: tauri::AppHandle) -> Result<serde_json::Value, String> {
    let config = load_config(app).await?;
    serde_json::to_value(config).map_err(|e| e.to_string())
}

#[tauri::command]
async fn get_config_path(app: tauri::AppHandle) -> Result<String, String> {
    let path = config::get_store_path(&app)?;
    Ok(path.to_string_lossy().to_string())
}

#[tauri::command]
async fn validate_config(app: tauri::AppHandle) -> Result<serde_json::Value, String> {
    let config = load_config(app).await?;
    let mut issues: Vec<serde_json::Value> = Vec::new();

    if config.ai.provider.trim().is_empty() {
        issues.push(serde_json::json!({
            "field": "ai.provider",
            "level": "error",
            "message": "AI provider is required"
        }));
    } else if !config::KNOWN_PROVIDERS.contains(&config.ai.provider.trim()) {
        issues.push(serde_json::json!({
            "field": "ai.provider",
            "level": "error",
            "message": format!(
                "Unknown AI provider '{}' (expected one of {})",
                config.ai.provider,
                config::KNOWN_PROVIDERS.join(", ")
            )
        }));
    }

    let has_key = match config.ai.provider.as_str() {
        "gemini" | "openai" | "anthropic" | "xai" => !config.ai.api_key.trim().is_empty(),
        // Ollama needs no key; custom endpoints may be key-less (e.g. local
        // vLLM) — the base URL is the required piece there.
        "ollama" | "custom" => true,
        _ => false,
    };
    if !has_key {
        issues.push(serde_json::json!({
            "field": "ai.api_key",
            "level": "error",
            "message": "No API key configured for the selected provider"
        }));
    }

    if config.ai.provider == "custom" && config.ai.custom_base_url.trim().is_empty() {
        issues.push(serde_json::json!({
            "field": "ai.custom_base_url",
            "level": "error",
            "message": "Custom provider requires a base URL"
        }));
    }

    if ai::get_model_name(&config.ai).trim().is_empty() {
        issues.push(serde_json::json!({
            "field": "ai.model",
            "level": "error",
            "message": "No model name configured for the selected provider"
        }));
    }

    if config.document.vision != "false" {
        let vision_provider = if config.document.vision_provider.trim().is_empty() {
            config.ai.provider.as_str()
        } else {
            config.document.vision_provider.as_str()
        };
        if !config::KNOWN_PROVIDERS.contains(&vision_provider) {
            issues.push(serde_json::json!({
                "field": "document.vision_provider",
                "level": "error",
                "message": format!("Unknown vision provider '{}'", vision_provider)
            }));
        }
    }

    let valid = issues.iter().all(|i| i["level"] != "error");

    Ok(serde_json::json!({
        "valid": valid,
        "issues": issues
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn with_extension_appends_once_and_is_case_insensitive() {
        assert_eq!(with_extension("name".to_string(), "pdf"), "name.pdf");
        assert_eq!(with_extension("name.pdf".to_string(), "pdf"), "name.pdf");
        assert_eq!(with_extension("name.PDF".to_string(), "pdf"), "name.PDF");
        // No extension on the source file: nothing to append.
        assert_eq!(with_extension("name".to_string(), ""), "name");
    }

    #[test]
    fn text_quality_threshold_requires_actual_text() {
        // A zero-quality extraction never counts as "good enough", even when
        // the user sets the threshold to 0.
        assert!(!text_quality_sufficient(0.0, 0.0));
        assert!(!text_quality_sufficient(0.0, 0.2));
        assert!(text_quality_sufficient(0.2, 0.2));
        assert!(text_quality_sufficient(0.9, 0.2));
        assert!(!text_quality_sufficient(0.1, 0.2));
    }

    #[test]
    fn failed_and_cancelled_results_are_shaped_for_the_ui() {
        let failed = failed_file_result("C:\\a.pdf", "boom".to_string());
        assert_eq!(failed.status, "failed");
        assert_eq!(failed.file, "C:\\a.pdf");
        assert_eq!(failed.error.as_deref(), Some("boom"));
        assert!(failed.new_name.is_none());
        assert!(failed.warnings.is_empty());
        assert!(failed.suggestion_names.is_empty());

        let cancelled = cancelled_file_result("C:\\b.pdf");
        assert_eq!(cancelled.status, "failed");
        assert_eq!(
            cancelled.error.as_deref(),
            Some("Rename cancelled by user")
        );
    }
}
