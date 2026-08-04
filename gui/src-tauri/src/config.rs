use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use tauri::Manager;
use tauri_plugin_store::Store;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AiConfig {
    pub provider: String,
    pub api_key: String,
    pub model: String,
    pub gemini_model: String,
    pub gemini_api_key: String,
    pub gemini_base_url: String,
    pub custom_model: String,
    pub custom_base_url: String,
    pub temperature: f64,
    pub timeout: u64,
}

impl Default for AiConfig {
    fn default() -> Self {
        Self {
            provider: "gemini".to_string(),
            api_key: String::new(),
            model: "gpt-4o-mini".to_string(),
            gemini_model: "gemini-3.1-flash-lite".to_string(),
            gemini_api_key: String::new(),
            gemini_base_url: String::new(),
            custom_model: String::new(),
            custom_base_url: String::new(),
            temperature: 0.0,
            timeout: 30,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PdfConfig {
    pub vision: String,
    pub vision_provider: String,
}

impl Default for PdfConfig {
    fn default() -> Self {
        Self {
            vision: "auto".to_string(),
            vision_provider: "gemini".to_string(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct NamingConfig {
    pub template: String,
    pub fallback: String,
    pub date_format: String,
    pub separator: String,
    pub max_length: u32,
    pub sequence_zerofill: u32,
}

impl Default for NamingConfig {
    fn default() -> Self {
        Self {
            template: "{date}_{company}_{doctype}".to_string(),
            fallback: "{date}_Unknown_{doctype}".to_string(),
            date_format: "%Y%m%d".to_string(),
            separator: "_".to_string(),
            max_length: 128,
            sequence_zerofill: 2,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct UndoConfig {
    pub enabled: bool,
    pub log_path: String,
    pub max_entries: u32,
}

impl Default for UndoConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            log_path: "~/.autorename-revived/rename_history.json".to_string(),
            max_entries: 100,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AppConfig {
    pub ai: AiConfig,
    pub pdf: PdfConfig,
    pub naming: NamingConfig,
    pub undo: UndoConfig,
    pub harmonized_companies: Vec<HashMap<String, serde_json::Value>>,
    pub debug: bool,
    pub max_workers: u32,
}

impl AppConfig {
    pub fn default() -> Self {
        Self {
            ai: AiConfig::default(),
            pdf: PdfConfig::default(),
            naming: NamingConfig::default(),
            undo: UndoConfig::default(),
            harmonized_companies: Vec::new(),
            debug: false,
            max_workers: 4,
        }
    }
}

pub fn get_store_path(app: &tauri::AppHandle) -> PathBuf {
    app.path()
        .app_data_dir()
        .expect("Failed to get app data dir")
        .join("settings.json")
}

pub async fn load_config(app: tauri::AppHandle) -> Result<AppConfig, String> {
    let store_path = get_store_path(&app);
    if store_path.exists() {
        let store = Store::new(app.clone(), store_path.clone())
            .await
            .map_err(|e| e.to_string())?;
        if let Some(saved) = store.get("config") {
            if let Ok(cfg) = serde_json::from_value::<AppConfig>(saved) {
                return Ok(cfg);
            }
        }
    }
    Ok(AppConfig::default())
}

pub async fn save_config(
    app: tauri::AppHandle,
    config: &AppConfig,
) -> Result<(), String> {
    let store_path = get_store_path(&app);
    let store = Store::new(app.clone(), store_path)
        .await
        .map_err(|e| e.to_string())?;
    store
        .set("config", serde_json::to_value(config).map_err(|e| e.to_string())?)
        .await
        .map_err(|e| e.to_string())?;
    store.save().await.map_err(|e| e.to_string())?;
    Ok(())
}

pub async fn save_config_batch(
    app: tauri::AppHandle,
    updates: Vec<(String, serde_json::Value)>,
) -> Result<(), String> {
    let mut config = load_config(app.clone()).await?;
    for (key, value) in updates {
        apply_config_update(&mut config, &key, &value);
    }
    save_config(app, &config).await?;
    Ok(())
}

fn apply_config_update(config: &mut AppConfig, key: &str, value: &serde_json::Value) {
    let parts: Vec<&str> = key.splitn(2, '.').collect();
    if parts.len() != 2 {
        return;
    }
    let section = parts[0];
    let field = parts[1];

    match section {
        "ai" => {
            let ai = &mut config.ai;
            match field {
                "provider" => ai.provider = value.as_str().unwrap_or(&ai.provider).to_string(),
                "api_key" => ai.api_key = value.as_str().unwrap_or(&ai.api_key).to_string(),
                "model" => ai.model = value.as_str().unwrap_or(&ai.model).to_string(),
                "gemini_model" => ai.gemini_model = value.as_str().unwrap_or(&ai.gemini_model).to_string(),
                "gemini_api_key" => ai.gemini_api_key = value.as_str().unwrap_or(&ai.gemini_api_key).to_string(),
                "gemini_base_url" => ai.gemini_base_url = value.as_str().unwrap_or(&ai.gemini_base_url).to_string(),
                "custom_model" => ai.custom_model = value.as_str().unwrap_or(&ai.custom_model).to_string(),
                "custom_base_url" => ai.custom_base_url = value.as_str().unwrap_or(&ai.custom_base_url).to_string(),
                "temperature" => ai.temperature = value.as_f64().unwrap_or(ai.temperature),
                "timeout" => ai.timeout = value.as_u64().unwrap_or(ai.timeout),
                _ => {}
            }
        }
        "pdf" => {
            let pdf = &mut config.pdf;
            match field {
                "vision" => pdf.vision = value.as_str().unwrap_or(&pdf.vision).to_string(),
                "vision_provider" => pdf.vision_provider = value.as_str().unwrap_or(&pdf.vision_provider).to_string(),
                _ => {}
            }
        }
        "naming" => {
            let naming = &mut config.naming;
            match field {
                "template" => naming.template = value.as_str().unwrap_or(&naming.template).to_string(),
                "fallback" => naming.fallback = value.as_str().unwrap_or(&naming.fallback).to_string(),
                "date_format" => naming.date_format = value.as_str().unwrap_or(&naming.date_format).to_string(),
                "separator" => naming.separator = value.as_str().unwrap_or(&naming.separator).to_string(),
                "max_length" => naming.max_length = value.as_u64().unwrap_or(naming.max_length as u64) as u32,
                "sequence_zerofill" => naming.sequence_zerofill = value.as_u64().unwrap_or(naming.sequence_zerofill as u64) as u32,
                _ => {}
            }
        }
        "undo" => {
            let undo = &mut config.undo;
            match field {
                "enabled" => undo.enabled = value.as_bool().unwrap_or(undo.enabled),
                "log_path" => undo.log_path = value.as_str().unwrap_or(&undo.log_path).to_string(),
                "max_entries" => undo.max_entries = value.as_u64().unwrap_or(undo.max_entries as u64) as u32,
                _ => {}
            }
        }
        "_general" => {
            match field {
                "debug" => config.debug = value.as_bool().unwrap_or(config.debug),
                "max_workers" => config.max_workers = value.as_u64().unwrap_or(config.max_workers as u64) as u32,
                _ => {}
            }
        }
        _ => {}
    }
}