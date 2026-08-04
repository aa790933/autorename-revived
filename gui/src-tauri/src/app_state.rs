use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum AiProvider {
    Gemini,
    OpenAI,
    Custom,
}

impl Default for AiProvider {
    fn default() -> Self {
        AiProvider::Gemini
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderSettings {
    pub provider: AiProvider,
    pub api_key: String,
    pub model: String,
    pub custom_base_url: String,
    pub naming_pattern: String,
}

impl Default for ProviderSettings {
    fn default() -> Self {
        Self {
            provider: AiProvider::Gemini,
            api_key: String::new(),
            model: "gemini-3.1-flash-lite".to_string(),
            custom_base_url: String::new(),
            naming_pattern: "{date}_{company}_{doctype}".to_string(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AppState {
    pub settings: ProviderSettings,
    pub processing: bool,
}

impl AppState {
    pub fn new() -> Self {
        Self {
            settings: ProviderSettings::default(),
            processing: false,
        }
    }
}