use base64::{engine::general_purpose, Engine as _};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::Instant;
use tracing::info;

const GEMINI_DEFAULT_MODEL: &str = "gemini-3.5-flash-lite";
const GEMINI_API_BASE: &str = "https://generativelanguage.googleapis.com";
const OPENAI_API_BASE: &str = "https://api.openai.com";
const XAI_API_BASE: &str = "https://api.x.ai";

#[derive(Debug, Clone, Serialize, Deserialize)]
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
            gemini_model: GEMINI_DEFAULT_MODEL.to_string(),
            gemini_api_key: String::new(),
            gemini_base_url: String::new(),
            custom_model: String::new(),
            custom_base_url: String::new(),
            temperature: 0.0,
            timeout: 30,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocumentMetadata {
    pub company_name: String,
    pub document_date: String,
    pub document_type: String,
    pub category: String,
    pub subject: String,
    pub confidence: f64,
    pub invoice_number: String,
    pub total_amount: String,
}

impl Default for DocumentMetadata {
    fn default() -> Self {
        Self {
            company_name: String::new(),
            document_date: String::new(),
            document_type: String::new(),
            category: String::new(),
            subject: String::new(),
            confidence: 0.0,
            invoice_number: String::new(),
            total_amount: String::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestConnectionResult {
    pub success: bool,
    pub message: String,
    pub latency_ms: u128,
    pub provider: String,
}

const SYSTEM_PROMPT: &str = r#"You are an advanced AI document analyzer specialized in extracting precise metadata for automated file renaming.
Your primary task is to analyze the provided document or image and extract data into a STRICT JSON format.
CRITICAL RULES:
1. Output ONLY valid JSON. 
2. Do NOT wrap the JSON in markdown blocks (e.g., no ```json). Just output the raw JSON object.
3. Keep values concise, using underscores '_' instead of spaces for multi-word values.
4. If a field cannot be determined, use "Unknown" or the current date for dates.
EXTRACT THE FOLLOWING FIELDS: "date" (YYYYMMDD), "company", "doctype", "category", "subject"."#;

fn clean_json_response(text: &str) -> String {
    text.trim()
        .trim_start_matches("```json")
        .trim_start_matches("```")
        .trim_end_matches("```")
        .trim()
    .to_string()
}

fn parse_json_response(text: &str) -> Result<HashMap<String, serde_json::Value>, String> {
    let clean_json = clean_json_response(text);
    serde_json::from_str::<HashMap<String, serde_json::Value>>(&clean_json)
        .map_err(|e| format!("Failed to parse AI response as JSON: {}", e))
}

fn guess_mime(ext: &str) -> &'static str {
    match ext.to_lowercase().as_str() {
        ".jpg" | ".jpeg" => "image/jpeg",
        ".png" => "image/png",
        ".gif" => "image/gif",
        ".webp" => "image/webp",
        ".bmp" => "image/bmp",
        ".tiff" | ".tif" => "image/tiff",
        ".pdf" => "application/pdf",
        _ => "application/octet-stream",
    }
}

fn file_extension(path: &str) -> String {
    std::path::Path::new(path)
        .extension()
        .map(|e| format!(".{}", e.to_string_lossy()))
        .unwrap_or_default()
}

pub async fn extract_metadata_text(
    text: &str,
    config: &AiConfig,
) -> Result<DocumentMetadata, String> {
    let provider = &config.provider;
    let start = Instant::now();
    info!("AI text extraction: provider={}, model={}", provider, config.model);

    let result = match provider.as_str() {
        "gemini" => gemini_text_extract(text, config).await,
        "openai" => openai_text_extract(text, config).await,
        "anthropic" => anthropic_text_extract(text, config).await,
        "ollama" | "xai" | "custom" => openai_compat_text_extract(text, config).await,
        other => Err(format!("Unknown provider: {}", other)),
    };

    info!("AI text extraction completed in {:?}", start.elapsed());
    result
}

pub async fn extract_metadata_vision(
    file_buffers: &[(String, Vec<u8>)],
    config: &AiConfig,
) -> Result<DocumentMetadata, String> {
    let provider = &config.provider;
    let start = Instant::now();
    info!("AI vision extraction: provider={}, files={}", provider, file_buffers.len());

    let result = match provider.as_str() {
        "gemini" => gemini_vision_extract(file_buffers, config).await,
        "openai" => openai_vision_extract(file_buffers, config).await,
        "anthropic" => anthropic_vision_extract(file_buffers, config).await,
        "ollama" | "xai" | "custom" => openai_compat_vision_extract(file_buffers, config).await,
        other => Err(format!("Unknown provider: {}", other)),
    };

    info!("AI vision extraction completed in {:?}", start.elapsed());
    result
}

fn resolve_ai_config(config: &AiConfig) -> HashMap<String, String> {
    let mut cfg = HashMap::new();
    cfg.insert("provider".to_string(), config.provider.clone());
    cfg.insert("api_key".to_string(), config.api_key.clone());
    cfg.insert("model".to_string(), config.model.clone());
    cfg.insert("gemini_model".to_string(), config.gemini_model.clone());
    cfg.insert("gemini_api_key".to_string(), config.gemini_api_key.clone());
    cfg.insert("gemini_base_url".to_string(), config.gemini_base_url.clone());
    cfg.insert("custom_model".to_string(), config.custom_model.clone());
    cfg.insert("custom_base_url".to_string(), config.custom_base_url.clone());
    cfg.insert("temperature".to_string(), config.temperature.to_string());
    cfg.insert("timeout".to_string(), config.timeout.to_string());
    cfg
}

async fn gemini_text_extract(text: &str, config: &AiConfig) -> Result<DocumentMetadata, String> {
    let api_key = config.gemini_api_key.trim();
    if api_key.is_empty() {
        return Err("Gemini API key is required".to_string());
    }
    let model = if config.gemini_model.trim().is_empty() {
        GEMINI_DEFAULT_MODEL.to_string()
    } else {
        config.gemini_model.clone()
    };

    let client = Client::builder()
        .timeout(std::time::Duration::from_secs(config.timeout))
        .build()
        .map_err(|e| e.to_string())?;

    let url = format!(
        "{}/v1beta/models/{}:generateContent?key={}",
        GEMINI_API_BASE, model, api_key
    );

    let body = serde_json::json!({
        "contents": [{
            "parts": [{
                "text": format!("{}\n\nDocument text:\n{}\n\nExtract metadata JSON.", SYSTEM_PROMPT, text)
            }]
        }],
        "generationConfig": {
            "responseMimeType": "application/json",
            "temperature": config.temperature
        }
    });

    let resp = client
        .post(&url)
        .json(&body)
        .send()
        .await
        .map_err(|e| e.to_string())?;

    let text_resp = resp.text().await.map_err(|e| e.to_string())?;
    let json: serde_json::Value = serde_json::from_str(&text_resp).map_err(|e| e.to_string())?;

    let candidates = json["candidates"].as_array().ok_or("No candidates in response")?;
    let parts = candidates[0]["content"]["parts"].as_array().ok_or("No content parts")?;
    let result_text = parts[0]["text"].as_str().ok_or("No text in response")?;

    let parsed = parse_json_response(result_text)?;
    Ok(data_to_metadata(&parsed))
}

async fn gemini_vision_extract(
    file_buffers: &[(String, Vec<u8>)],
    config: &AiConfig,
) -> Result<DocumentMetadata, String> {
    let api_key = config.gemini_api_key.trim();
    if api_key.is_empty() {
        return Err("Gemini API key is required for vision extraction".to_string());
    }
    let model = if config.gemini_model.trim().is_empty() {
        GEMINI_DEFAULT_MODEL.to_string()
    } else {
        config.gemini_model.clone()
    };

    let client = Client::builder()
        .timeout(std::time::Duration::from_secs(config.timeout.max(60)))
        .build()
        .map_err(|e| e.to_string())?;

    let url = format!(
        "{}/v1beta/models/{}:generateContent?key={}",
        GEMINI_API_BASE, model, api_key
    );

    let mut contents = Vec::new();
    contents.push(serde_json::json!({"text": SYSTEM_PROMPT}));

    for (path, buffer) in file_buffers {
        let ext = file_extension(path);
        let mime = guess_mime(&ext);
        let b64 = general_purpose::STANDARD.encode(buffer);
        contents.push(serde_json::json!({
            "inlineData": {
                "mimeType": mime,
                "data": b64
            }
        }));
    }

    let body = serde_json::json!({
        "contents": [{"parts": contents}],
        "generationConfig": {
            "responseMimeType": "application/json",
            "temperature": config.temperature
        }
    });

    let resp = client
        .post(&url)
        .json(&body)
        .send()
        .await
        .map_err(|e| e.to_string())?;

    let text_resp = resp.text().await.map_err(|e| e.to_string())?;
    let json: serde_json::Value = serde_json::from_str(&text_resp).map_err(|e| e.to_string())?;

    let candidates = json["candidates"].as_array().ok_or("No candidates in response")?;
    let parts = candidates[0]["content"]["parts"].as_array().ok_or("No content parts")?;
    let result_text = parts[0]["text"].as_str().ok_or("No text in response")?;

    let parsed = parse_json_response(result_text)?;
    Ok(data_to_metadata(&parsed))
}

async fn openai_text_extract(text: &str, config: &AiConfig) -> Result<DocumentMetadata, String> {
    let api_key = config.api_key.trim();
    if api_key.is_empty() {
        return Err("OpenAI API key is required".to_string());
    }
    let model = if config.model.trim().is_empty() {
        "gpt-4o-mini".to_string()
    } else {
        config.model.clone()
    };

    let client = Client::builder()
        .timeout(std::time::Duration::from_secs(config.timeout))
        .build()
        .map_err(|e| e.to_string())?;

    let url = format!("{}/v1/chat/completions", OPENAI_API_BASE);

    let body = serde_json::json!({
        "model": model,
        "temperature": config.temperature,
        "messages": [
            {"role": "system", "content": SYSTEM_PROMPT},
            {"role": "user", "content": format!("Document text:\n\n{}\n\nExtract metadata JSON.", text)}
        ],
        "response_format": {"type": "json_object"}
    });

    let resp = client
        .post(&url)
        .bearer_auth(api_key)
        .json(&body)
        .send()
        .await
        .map_err(|e| e.to_string())?;

    let json: serde_json::Value = resp.json().await.map_err(|e| e.to_string())?;
    let result_text = json["choices"][0]["message"]["content"]
        .as_str()
        .ok_or("No content in response")?;

    let parsed = parse_json_response(result_text)?;
    Ok(data_to_metadata(&parsed))
}

async fn openai_vision_extract(
    file_buffers: &[(String, Vec<u8>)],
    config: &AiConfig,
) -> Result<DocumentMetadata, String> {
    let api_key = config.api_key.trim();
    if api_key.is_empty() {
        return Err("OpenAI API key is required for vision extraction".to_string());
    }
    let model = if config.model.trim().is_empty() {
        "gpt-4o".to_string()
    } else {
        config.model.clone()
    };

    let client = Client::builder()
        .timeout(std::time::Duration::from_secs(config.timeout.max(60)))
        .build()
        .map_err(|e| e.to_string())?;

    let url = format!("{}/v1/chat/completions", OPENAI_API_BASE);

    let mut content_parts = Vec::new();
    content_parts.push(serde_json::json!({"type": "text", "text": SYSTEM_PROMPT}));

    for (path, buffer) in file_buffers {
        let ext = file_extension(path);
        let mime = guess_mime(&ext);
        let b64 = general_purpose::STANDARD.encode(buffer);
        content_parts.push(serde_json::json!({
            "type": "image_url",
            "image_url": {"url": format!("data:{};base64,{}", mime, b64), "detail": "auto"}
        }));
    }

    let body = serde_json::json!({
        "model": model,
        "temperature": config.temperature,
        "messages": [{"role": "user", "content": content_parts}],
        "response_format": {"type": "json_object"},
        "max_tokens": 1024
    });

    let resp = client
        .post(&url)
        .bearer_auth(api_key)
        .json(&body)
        .send()
        .await
        .map_err(|e| e.to_string())?;

    let json: serde_json::Value = resp.json().await.map_err(|e| e.to_string())?;
    let result_text = json["choices"][0]["message"]["content"]
        .as_str()
        .ok_or("No content in response")?;

    let parsed = parse_json_response(result_text)?;
    Ok(data_to_metadata(&parsed))
}

async fn anthropic_text_extract(text: &str, config: &AiConfig) -> Result<DocumentMetadata, String> {
    let api_key = config.api_key.trim();
    if api_key.is_empty() {
        return Err("Anthropic API key is required".to_string());
    }
    let model = if config.model.trim().is_empty() {
        "claude-3-5-haiku-latest".to_string()
    } else {
        config.model.clone()
    };

    let client = Client::builder()
        .timeout(std::time::Duration::from_secs(config.timeout))
        .build()
        .map_err(|e| e.to_string())?;

    let url = "https://api.anthropic.com/v1/messages".to_string();

    let body = serde_json::json!({
        "model": model,
        "max_tokens": 1024,
        "system": SYSTEM_PROMPT,
        "messages": [{"role": "user", "content": format!("Document text:\n\n{}\n\nExtract metadata JSON.", text)}]
    });

    let resp = client
        .post(&url)
        .header("x-api-key", api_key)
        .header("anthropic-version", "2023-06-01")
        .json(&body)
        .send()
        .await
        .map_err(|e| e.to_string())?;

    let json: serde_json::Value = resp.json().await.map_err(|e| e.to_string())?;
    let result_text = json["content"][0]["text"].as_str().ok_or("No text in response")?;

    let parsed = parse_json_response(result_text)?;
    Ok(data_to_metadata(&parsed))
}

async fn anthropic_vision_extract(
    file_buffers: &[(String, Vec<u8>)],
    config: &AiConfig,
) -> Result<DocumentMetadata, String> {
    let api_key = config.api_key.trim();
    if api_key.is_empty() {
        return Err("Anthropic API key is required for vision extraction".to_string());
    }
    let model = if config.model.trim().is_empty() {
        "claude-sonnet-4-20250514".to_string()
    } else {
        config.model.clone()
    };

    let client = Client::builder()
        .timeout(std::time::Duration::from_secs(config.timeout.max(60)))
        .build()
        .map_err(|e| e.to_string())?;

    let url = "https://api.anthropic.com/v1/messages".to_string();

    let mut content_parts = Vec::new();

    for (path, buffer) in file_buffers {
        let ext = file_extension(path);
        let mime = guess_mime(&ext);
        let b64 = general_purpose::STANDARD.encode(buffer);
        content_parts.push(serde_json::json!({
            "type": "image",
            "source": {"type": "base64", "media_type": mime, "data": b64}
        }));
    }

    content_parts.push(serde_json::json!({"type": "text", "text": SYSTEM_PROMPT}));

    let body = serde_json::json!({
        "model": model,
        "max_tokens": 1024,
        "messages": [{"role": "user", "content": content_parts}]
    });

    let resp = client
        .post(&url)
        .header("x-api-key", api_key)
        .header("anthropic-version", "2023-06-01")
        .json(&body)
        .send()
        .await
        .map_err(|e| e.to_string())?;

    let json: serde_json::Value = resp.json().await.map_err(|e| e.to_string())?;
    let result_text = json["content"][0]["text"].as_str().ok_or("No text in response")?;

    let parsed = parse_json_response(result_text)?;
    Ok(data_to_metadata(&parsed))
}

async fn openai_compat_text_extract(text: &str, config: &AiConfig) -> Result<DocumentMetadata, String> {
    let base_url = config.custom_base_url.trim();
    if base_url.is_empty() {
        return Err("Custom API base URL is required".to_string());
    }
    let model = if config.custom_model.trim().is_empty() {
        "llama3.2".to_string()
    } else {
        config.custom_model.clone()
    };
    let api_key = config.api_key.trim();

    let client = Client::builder()
        .timeout(std::time::Duration::from_secs(config.timeout))
        .build()
        .map_err(|e| e.to_string())?;

    let url = format!("{}/v1/chat/completions", base_url.trim_end_matches('/'));

    let body = serde_json::json!({
        "model": model,
        "temperature": config.temperature,
        "messages": [
            {"role": "system", "content": SYSTEM_PROMPT},
            {"role": "user", "content": format!("Document text:\n\n{}\n\nExtract metadata JSON.", text)}
        ],
        "response_format": {"type": "json_object"}
    });

    let resp = client
        .post(&url)
        .bearer_auth(api_key)
        .json(&body)
        .send()
        .await
        .map_err(|e| e.to_string())?;

    let json: serde_json::Value = resp.json().await.map_err(|e| e.to_string())?;
    let result_text = json["choices"][0]["message"]["content"]
        .as_str()
        .ok_or("No content in response")?;

    let parsed = parse_json_response(result_text)?;
    Ok(data_to_metadata(&parsed))
}

async fn openai_compat_vision_extract(
    file_buffers: &[(String, Vec<u8>)],
    config: &AiConfig,
) -> Result<DocumentMetadata, String> {
    let base_url = config.custom_base_url.trim();
    if base_url.is_empty() {
        return Err("Custom API base URL is required for vision extraction".to_string());
    }
    let model = if config.custom_model.trim().is_empty() {
        "llama3.2".to_string()
    } else {
        config.custom_model.clone()
    };
    let api_key = config.api_key.trim();

    let client = Client::builder()
        .timeout(std::time::Duration::from_secs(config.timeout.max(60)))
        .build()
        .map_err(|e| e.to_string())?;

    let url = format!("{}/v1/chat/completions", base_url.trim_end_matches('/'));

    let mut content_parts = Vec::new();
    content_parts.push(serde_json::json!({"type": "text", "text": SYSTEM_PROMPT}));

    for (path, buffer) in file_buffers {
        let ext = file_extension(path);
        let mime = guess_mime(&ext);
        let b64 = general_purpose::STANDARD.encode(buffer);
        content_parts.push(serde_json::json!({
            "type": "image_url",
            "image_url": {"url": format!("data:{};base64,{}", mime, b64), "detail": "auto"}
        }));
    }

    let body = serde_json::json!({
        "model": model,
        "temperature": config.temperature,
        "messages": [{"role": "user", "content": content_parts}],
        "response_format": {"type": "json_object"},
        "max_tokens": 1024
    });

    let resp = client
        .post(&url)
        .bearer_auth(api_key)
        .json(&body)
        .send()
        .await
        .map_err(|e| e.to_string())?;

    let json: serde_json::Value = resp.json().await.map_err(|e| e.to_string())?;
    let result_text = json["choices"][0]["message"]["content"]
        .as_str()
        .ok_or("No content in response")?;

    let parsed = parse_json_response(result_text)?;
    Ok(data_to_metadata(&parsed))
}

fn data_to_metadata(data: &HashMap<String, serde_json::Value>) -> DocumentMetadata {
    let confidence = data
        .get("confidence")
        .and_then(|v| v.as_f64())
        .unwrap_or(0.0)
        .clamp(0.0, 1.0);

    DocumentMetadata {
        company_name: data
            .get("company")
            .and_then(|v| v.as_str())
            .unwrap_or("Unknown")
            .to_string(),
        document_date: data
            .get("date")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string(),
        document_type: data
            .get("doctype")
            .and_then(|v| v.as_str())
            .unwrap_or("Unknown")
            .to_string(),
        category: data
            .get("category")
            .and_then(|v| v.as_str())
            .unwrap_or("Unknown")
            .to_string(),
        subject: data
            .get("subject")
            .and_then(|v| v.as_str())
            .unwrap_or("Unknown")
            .to_string(),
        confidence,
        invoice_number: data
            .get("invoice_number")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string(),
        total_amount: data
            .get("total_amount")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string(),
    }
}

pub async fn test_connection(
    provider: &str,
    api_key: &str,
    model: &str,
) -> TestConnectionResult {
    let start = Instant::now();

    if api_key.is_empty() && provider != "ollama" {
        return TestConnectionResult {
            success: false,
            message: "API key is empty".to_string(),
            latency_ms: 0,
            provider: provider.to_string(),
        };
    }

    let result = match provider {
        "gemini" => test_gemini_connection(api_key, model, start).await,
        "openai" => test_openai_connection(api_key, start).await,
        "anthropic" => test_anthropic_connection(api_key, start).await,
        "ollama" => test_ollama_connection(start).await,
        "xai" => test_xai_connection(api_key, start).await,
        "custom" => test_custom_connection(start).await,
        other => TestConnectionResult {
            success: false,
            message: format!("Unknown provider: {}", other),
            latency_ms: start.elapsed().as_millis(),
            provider: other.to_string(),
        },
    };

    result
}

async fn test_gemini_connection(api_key: &str, model: &str, start: Instant) -> TestConnectionResult {
    let model_name = if model.trim().is_empty() {
        GEMINI_DEFAULT_MODEL
    } else {
        model
    };
    let client = Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .unwrap_or_default();

    let url = format!(
        "{}/v1beta/models/{}:generateContent?key={}",
        GEMINI_API_BASE, model_name, api_key
    );

    let body = serde_json::json!({
        "contents": [{"parts": [{"text": "OK"}]}],
        "generationConfig": {"maxOutputTokens": 1}
    });

    match client.post(&url).json(&body).send().await {
        Ok(resp) => {
            let ms = start.elapsed().as_millis();
            if resp.status().is_success() {
                TestConnectionResult {
                    success: true,
                    message: format!("Connected ({}ms)", ms),
                    latency_ms: ms,
                    provider: "gemini".to_string(),
                }
            } else {
                TestConnectionResult {
                    success: false,
                    message: format!("Gemini returned status {}", resp.status()),
                    latency_ms: ms,
                    provider: "gemini".to_string(),
                }
            }
        }
        Err(e) => TestConnectionResult {
            success: false,
            message: format!("Failed: {}", e),
            latency_ms: start.elapsed().as_millis(),
            provider: "gemini".to_string(),
        },
    }
}

async fn test_openai_connection(api_key: &str, start: Instant) -> TestConnectionResult {
    let client = Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .unwrap_or_default();

    let url = format!("{}/v1/models", OPENAI_API_BASE);

    match client
        .get(&url)
        .bearer_auth(api_key)
        .send()
        .await
    {
        Ok(resp) => {
            let ms = start.elapsed().as_millis();
            if resp.status().is_success() {
                TestConnectionResult {
                    success: true,
                    message: format!("Connected ({}ms)", ms),
                    latency_ms: ms,
                    provider: "openai".to_string(),
                }
            } else {
                TestConnectionResult {
                    success: false,
                    message: format!("OpenAI returned status {}", resp.status()),
                    latency_ms: ms,
                    provider: "openai".to_string(),
                }
            }
        }
        Err(e) => TestConnectionResult {
            success: false,
            message: format!("Failed: {}", e),
            latency_ms: start.elapsed().as_millis(),
            provider: "openai".to_string(),
        },
    }
}

async fn test_anthropic_connection(api_key: &str, start: Instant) -> TestConnectionResult {
    let client = Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .unwrap_or_default();

    let url = "https://api.anthropic.com/v1/models".to_string();

    match client
        .get(&url)
        .header("x-api-key", api_key)
        .header("anthropic-version", "2023-06-01")
        .send()
        .await
    {
        Ok(resp) => {
            let ms = start.elapsed().as_millis();
            if resp.status().is_success() {
                TestConnectionResult {
                    success: true,
                    message: format!("Connected ({}ms)", ms),
                    latency_ms: ms,
                    provider: "anthropic".to_string(),
                }
            } else {
                TestConnectionResult {
                    success: false,
                    message: format!("Anthropic returned status {}", resp.status()),
                    latency_ms: ms,
                    provider: "anthropic".to_string(),
                }
            }
        }
        Err(e) => TestConnectionResult {
            success: false,
            message: format!("Failed: {}", e),
            latency_ms: start.elapsed().as_millis(),
            provider: "anthropic".to_string(),
        },
    }
}

async fn test_ollama_connection(start: Instant) -> TestConnectionResult {
    let client = Client::builder()
        .timeout(std::time::Duration::from_secs(5))
        .build()
        .unwrap_or_default();

    let url = "http://localhost:11434/api/tags".to_string();

    match client.get(&url).send().await {
        Ok(resp) => {
            let ms = start.elapsed().as_millis();
            if resp.status().is_success() {
                if let Ok(json) = resp.json::<serde_json::Value>().await {
                    let count = json.get("models").and_then(|v| v.as_array()).map(|a| a.len()).unwrap_or(0);
                    TestConnectionResult {
                        success: true,
                        message: format!("Connected ({} models, {}ms)", count, ms),
                        latency_ms: ms,
                        provider: "ollama".to_string(),
                    }
                } else {
                    TestConnectionResult {
                        success: true,
                        message: format!("Connected ({}ms)", ms),
                        latency_ms: ms,
                        provider: "ollama".to_string(),
                    }
                }
            } else {
                TestConnectionResult {
                    success: false,
                    message: format!("Ollama returned status {}", resp.status()),
                    latency_ms: ms,
                    provider: "ollama".to_string(),
                }
            }
        }
        Err(e) => TestConnectionResult {
            success: false,
            message: format!("Failed: {}", e),
            latency_ms: start.elapsed().as_millis(),
            provider: "ollama".to_string(),
        },
    }
}

async fn test_xai_connection(api_key: &str, start: Instant) -> TestConnectionResult {
    let client = Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .unwrap_or_default();

    let url = format!("{}/v1/models", XAI_API_BASE);

    match client
        .get(&url)
        .bearer_auth(api_key)
        .send()
        .await
    {
        Ok(resp) => {
            let ms = start.elapsed().as_millis();
            if resp.status().is_success() {
                TestConnectionResult {
                    success: true,
                    message: format!("Connected ({}ms)", ms),
                    latency_ms: ms,
                    provider: "xai".to_string(),
                }
            } else {
                TestConnectionResult {
                    success: false,
                    message: format!("xAI returned status {}", resp.status()),
                    latency_ms: ms,
                    provider: "xai".to_string(),
                }
            }
        }
        Err(e) => TestConnectionResult {
            success: false,
            message: format!("Failed: {}", e),
            latency_ms: start.elapsed().as_millis(),
            provider: "xai".to_string(),
        },
    }
}

async fn test_custom_connection(start: Instant) -> TestConnectionResult {
    TestConnectionResult {
        success: false,
        message: "Custom provider connection test requires base URL configuration".to_string(),
        latency_ms: start.elapsed().as_millis(),
        provider: "custom".to_string(),
    }
}
