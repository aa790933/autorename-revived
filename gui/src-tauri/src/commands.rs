use crate::app_state::{AiProvider, AppState, ProviderSettings};
use crate::provider;
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, State};
use tokio::fs;
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SettingsPayload {
    pub provider: String,
    pub api_key: String,
    pub model: String,
    pub custom_base_url: String,
    pub naming_pattern: String,
}

impl From<SettingsPayload> for ProviderSettings {
    fn from(payload: SettingsPayload) -> Self {
        Self {
            provider: match payload.provider.as_str() {
                "openai" => AiProvider::OpenAI,
                "custom" => AiProvider::Custom,
                _ => AiProvider::Gemini,
            },
            api_key: payload.api_key,
            model: payload.model,
            custom_base_url: payload.custom_base_url,
            naming_pattern: payload.naming_pattern,
        }
    }
}

impl From<ProviderSettings> for SettingsPayload {
    fn from(settings: ProviderSettings) -> Self {
        Self {
            provider: match settings.provider {
                AiProvider::Gemini => "gemini".to_string(),
                AiProvider::OpenAI => "openai".to_string(),
                AiProvider::Custom => "custom".to_string(),
            },
            api_key: settings.api_key,
            model: settings.model,
            custom_base_url: settings.custom_base_url,
            naming_pattern: settings.naming_pattern,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RenameResult {
    pub original_path: String,
    pub new_path: String,
    pub success: bool,
    pub error: Option<String>,
}

#[tauri::command]
pub async fn save_settings(
    app: AppHandle,
    state: State<'_, AppState>,
    payload: SettingsPayload,
) -> Result<(), String> {
    let settings = ProviderSettings::from(payload);
    state.settings = settings.clone();

    let store = tauri_plugin_store::StoreBuilder::new(
        app.handle(),
        app.path_resolver().app_data_dir().unwrap().join("settings.json"),
    )
    .build();

    store
        .set("settings", serde_json::to_value(&settings).map_err(|e| e.to_string())?);
    store.save().map_err(|e| e.to_string())?;

    Ok(())
}

#[tauri::command]
pub async fn get_settings(
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<SettingsPayload, String> {
    let app_data_dir = app.path_resolver().app_data_dir().ok_or("No app data dir")?;
    let store_path = app_data_dir.join("settings.json");

    if store_path.exists() {
        let store = tauri_plugin_store::StoreBuilder::new(app.handle(), store_path).build();
        if let Some(saved) = store.get("settings") {
            if let Ok(settings) = serde_json::from_value::<ProviderSettings>(saved) {
                state.settings = settings.clone();
                return Ok(SettingsPayload::from(settings));
            }
        }
    }

    Ok(SettingsPayload::from(state.settings.clone()))
}

#[tauri::command]
pub async fn test_connection(
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<bool, String> {
    provider::test_connection(&state.settings).await
}

#[tauri::command]
pub async fn rename_files(
    app: AppHandle,
    state: State<'_, AppState>,
    file_paths: Vec<String>,
) -> Result<Vec<RenameResult>, String> {
    state.processing = true;
    let settings = state.settings.clone();
    let mut results = Vec::new();

    for file_path in &file_paths {
        let path = Path::new(file_path);
        let mime_type = mime_guess::from_path(path)
            .first_or_octet_stream()
            .to_string();

        match provider::extract_metadata(&settings, file_path, &mime_type).await {
            Ok(metadata) => {
                let new_name = provider::apply_naming_pattern(&settings.naming_pattern, &metadata);
                let extension = path.extension().and_then(|e| e.to_str()).unwrap_or("");
                let new_filename = if extension.is_empty() {
                    new_name
                } else {
                    format!("{}.{}", new_name, extension)
                };
                let parent = path.parent().unwrap_or(Path::new(""));
                let new_path = parent.join(&new_filename);

                match fs::rename(file_path, &new_path).await {
                    Ok(_) => {
                        results.push(RenameResult {
                            original_path: file_path.clone(),
                            new_path: new_path.to_string_lossy().to_string(),
                            success: true,
                            error: None,
                        });
                    }
                    Err(e) => {
                        results.push(RenameResult {
                            original_path: file_path.clone(),
                            new_path: String::new(),
                            success: false,
                            error: Some(e.to_string()),
                        });
                    }
                }
            }
            Err(e) => {
                results.push(RenameResult {
                    original_path: file_path.clone(),
                    new_path: String::new(),
                    success: false,
                    error: Some(e.to_string()),
                });
            }
        }
    }

    state.processing = false;
    Ok(results)
}

#[tauri::command]
pub fn select_folder() -> Result<Vec<String>, String> {
    Ok(vec![])
}

#[tauri::command]
pub fn select_files() -> Result<Vec<String>, String> {
    Ok(vec![])
}