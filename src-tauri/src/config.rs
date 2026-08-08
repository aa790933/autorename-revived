use crate::ai::AiConfig;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use tauri::Manager;
use tauri_plugin_store::StoreBuilder;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PdfConfig {
    pub vision: String,
    pub vision_provider: String,
    #[serde(default = "default_text_quality_threshold")]
    pub text_quality_threshold: f64,
}

fn default_text_quality_threshold() -> f64 {
    0.2
}

impl Default for PdfConfig {
    fn default() -> Self {
        Self {
            vision: "auto".to_string(),
            vision_provider: "gemini".to_string(),
            text_quality_threshold: 0.2,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
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
            date_format: String::from("%Y%m%d"),
            separator: String::from("_"),
            max_length: 128,
            sequence_zerofill: 2,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub ai: AiConfig,
    pub pdf: PdfConfig,
    pub naming: NamingConfig,
    pub undo: UndoConfig,
    pub harmonized_companies: Vec<HashMap<String, serde_json::Value>>,
    pub debug: bool,
    pub max_workers: u32,
}

impl Default for AppConfig {
    fn default() -> Self {
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

fn resolve_env_vars(value: &str) -> String {
    let mut result = value.to_string();
    // Resolve ${VAR} patterns
    let env_re = regex::Regex::new(r#"\$\{(\w+)\}"#).unwrap();
    let matches: Vec<String> = env_re
        .find_iter(&result)
        .map(|m| m.as_str().to_string())
        .collect();
    for mat in &matches {
        let var_name = &mat[2..mat.len() - 1]; // strip ${ and }
        if let Ok(val) = std::env::var(var_name) {
            result = result.replace(mat, &val);
        }
    }
    // Resolve $VAR patterns (word boundary)
    let simple_re = regex::Regex::new(r#"\$(\w+)"#).unwrap();
    let simple_matches: Vec<String> = simple_re
        .find_iter(&result)
        .map(|m| m.as_str().to_string())
        .collect();
    for mat in &simple_matches {
        let var_name = &mat[1..]; // strip $
        if let Ok(val) = std::env::var(var_name) {
            result = result.replace(mat, &val);
        }
    }
    result
}

fn resolve_config_env_vars(config: &mut AppConfig) {
    config.ai.api_key = resolve_env_vars(&config.ai.api_key);
    config.ai.gemini_api_key = resolve_env_vars(&config.ai.gemini_api_key);
}

pub async fn load_config(app: tauri::AppHandle) -> Result<AppConfig, String> {
    let store_path = get_store_path(&app);
    if store_path.exists() {
        let store = StoreBuilder::new(&app, store_path.clone())
        .build()
        .map_err(|e| e.to_string())?;
        if let Some(saved) = store.get("config") {
            if let Ok(mut cfg) = serde_json::from_value::<AppConfig>(saved) {
                resolve_config_env_vars(&mut cfg);
                return Ok(cfg);
            }
        }
    }
    let mut cfg = AppConfig::default();
    resolve_config_env_vars(&mut cfg);
    Ok(cfg)
}

pub async fn save_config(
    app: tauri::AppHandle,
    config: &AppConfig,
) -> Result<(), String> {
    let store_path = get_store_path(&app);
    let store = StoreBuilder::new(&app, store_path)
        .build()
        .map_err(|e| e.to_string())?;
    store
    .set("config", serde_json::to_value(config).map_err(|e| e.to_string())?);
    store.save().map_err(|e| e.to_string())?;
    Ok(())
}

pub async fn save_config_batch(
    app: tauri::AppHandle,
    updates: Vec<(String, String)>,
) -> Result<ConfigBatchResult, String> {
    let mut config = load_config(app.clone()).await?;
    let mut saved = 0u32;
    let mut failed = 0u32;
    let mut errors = Vec::new();

    for (key, value) in updates {
        match apply_config_update(&mut config, &key, &value) {
            Ok(()) => saved += 1,
            Err(e) => {
                failed += 1;
                errors.push(e);
            }
        }
    }

    if saved > 0 {
        save_config(app, &config).await?;
    }

    Ok(ConfigBatchResult {
        success: failed == 0,
        saved,
        failed,
        errors,
    })
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigBatchResult {
    pub success: bool,
    pub saved: u32,
    pub failed: u32,
    pub errors: Vec<String>,
}

pub fn apply_config_update(
    config: &mut AppConfig,
    key: &str,
    value: &str,
) -> Result<(), String> {
    let parts: Vec<&str> = key.splitn(2, '.').collect();
    if parts.len() != 2 {
        return Err(format!("Invalid config key format: {}", key));
    }
    let section = parts[0];
    let field = parts[1];

    match section {
        "ai" => {
            let ai = &mut config.ai;
            match field {
                "provider" => ai.provider = value.to_string(),
                "api_key" => ai.api_key = value.to_string(),
                "model" => ai.model = value.to_string(),
                "gemini_model" => ai.gemini_model = value.to_string(),
                "gemini_api_key" => ai.gemini_api_key = value.to_string(),
                "gemini_base_url" => ai.gemini_base_url = value.to_string(),
                "custom_model" => ai.custom_model = value.to_string(),
                "custom_base_url" => ai.custom_base_url = value.to_string(),
                "temperature" => {
                    ai.temperature = value.parse::<f64>().unwrap_or(ai.temperature);
                }
                "timeout" => {
                    ai.timeout = value.parse::<u64>().unwrap_or(ai.timeout);
                }
                _ => {
                    return Err(format!("Unknown AI config field: {}", field));
                }
            }
        }
        "pdf" => {
            let pdf = &mut config.pdf;
            match field {
                "vision" => pdf.vision = value.to_string(),
                "vision_provider" => pdf.vision_provider = value.to_string(),
                "text_quality_threshold" => {
                    pdf.text_quality_threshold =
                        value.parse::<f64>().unwrap_or(pdf.text_quality_threshold);
                }
                _ => {
                    return Err(format!("Unknown PDF config field: {}", field));
                }
            }
        }
        "naming" => {
            let naming = &mut config.naming;
            match field {
                "template" => naming.template = value.to_string(),
                "fallback" => naming.fallback = value.to_string(),
                "date_format" => naming.date_format = value.to_string(),
                "separator" => naming.separator = value.to_string(),
                "max_length" => {
                    naming.max_length = value.parse::<u32>().unwrap_or(naming.max_length);
                }
                "sequence_zerofill" => {
                    naming.sequence_zerofill =
                        value.parse::<u32>().unwrap_or(naming.sequence_zerofill);
                }
                _ => {
                    return Err(format!("Unknown naming config field: {}", field));
                }
            }
        }
        "undo" => {
            let undo = &mut config.undo;
            match field {
                "enabled" => {
                    undo.enabled = value == "true";
                }
                "log_path" => undo.log_path = value.to_string(),
                "max_entries" => {
                    undo.max_entries = value.parse::<u32>().unwrap_or(undo.max_entries);
                }
                _ => {
                    return Err(format!("Unknown undo config field: {}", field));
                }
            }
        }
        "_general" => match field {
            "debug" => {
                config.debug = value == "true";
            }
            "max_workers" => {
                config.max_workers = value.parse::<u32>().unwrap_or(config.max_workers);
            }
            _ => {
                return Err(format!("Unknown general config field: {}", field));
            }
        },
        _ => {
            return Err(format!("Unknown config section: {}", section));
        }
    }
    Ok(())
}
