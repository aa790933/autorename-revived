use crate::app_state::{AiProvider, ProviderSettings};
use base64::{engine::general_purpose, Engine as _};
use reqwest::Client;
use serde_json::{json, Value};
use std::time::Duration;
use tokio::fs;

pub async fn test_connection(settings: &ProviderSettings) -> Result<bool, String> {
    let client = Client::builder()
        .timeout(Duration::from_secs(10))
        .build()
        .map_err(|e| e.to_string())?;

    let api_key = settings.api_key.trim();
    if api_key.is_empty() {
        return Err("API key is required".to_string());
    }

    match settings.provider {
        AiProvider::Gemini => {
            let url = format!(
                "https://generativelanguage.googleapis.com/v1beta/models/{}:generateContent?key={}",
                settings.model, api_key
            );
            let body = json!({
                "contents": [{
                    "role": "user",
                    "parts": [{"text": "Respond with 'ok'"}]
                }],
                "generationConfig": {"maxOutputTokens": 1}
            });
            let res = client
                .post(&url)
                .json(&body)
                .send()
                .await
                .map_err(|e| e.to_string())?;
            let status = res.status();
            let _text = res.text().await.map_err(|e| e.to_string())?;
            Ok(status.is_success())
        }
        AiProvider::OpenAI => {
            let url = "https://api.openai.com/v1/chat/completions".to_string();
            let body = json!({
                "model": settings.model,
                "messages": [{"role": "user", "content": "Respond with 'ok'"}],
                "max_tokens": 1
            });
            let res = client
                .post(&url)
                .header("Authorization", format!("Bearer {}", api_key))
                .json(&body)
                .send()
                .await
                .map_err(|e| e.to_string())?;
            let status = res.status();
            let _text = res.text().await.map_err(|e| e.to_string())?;
            Ok(status.is_success())
        }
        AiProvider::Custom => {
            let base_url = settings.custom_base_url.trim();
            if base_url.is_empty() {
                return Err("Custom base URL is required".to_string());
            }
            let url = format!("{}/v1/chat/completions", base_url.trim_end_matches('/'));
            let body = json!({
                "model": settings.model,
                "messages": [{"role": "user", "content": "Respond with 'ok'"}],
                "max_tokens": 1
            });
            let res = client
                .post(&url)
                .header("Authorization", format!("Bearer {}", api_key))
                .json(&body)
                .send()
                .await
                .map_err(|e| e.to_string())?;
            let status = res.status();
            let _text = res.text().await.map_err(|e| e.to_string())?;
            Ok(status.is_success())
        }
    }
}

pub async fn extract_metadata(
    settings: &ProviderSettings,
    file_path: &str,
    mime_type: &str,
) -> Result<Value, String> {
    let client = Client::builder()
        .timeout(Duration::from_secs(60))
        .build()
        .map_err(|e| e.to_string())?;

    let api_key = settings.api_key.trim();
    if api_key.is_empty() {
        return Err("API key is required".to_string());
    }

    let file_bytes = fs::read(file_path).await.map_err(|e| e.to_string())?;
    let base64_data = general_purpose::STANDARD.encode(&file_bytes);

    let user_text = "Extract the company name, document date, and document type from this document. Return ONLY a valid JSON object with keys: company, date, doctype. Example: {\"company\": \"Acme Corp\", \"date\": \"2025-01-15\", \"doctype\": \"invoice\"}";

    let (url, body) = match settings.provider {
        AiProvider::Gemini => {
            let url = format!(
                "https://generativelanguage.googleapis.com/v1beta/models/{}:generateContent?key={}",
                settings.model, api_key
            );
            let body = json!({
                "contents": [{
                    "role": "user",
                    "parts": [
                        {"text": user_text},
                        {"inline_data": {"mimeType": mime_type, "data": base64_data}}
                    ]
                }],
                "generationConfig": {"maxOutputTokens": 500}
            });
            (url, body)
        }
        AiProvider::OpenAI => {
            let url = "https://api.openai.com/v1/chat/completions".to_string();
            let body = json!({
                "model": settings.model,
                "messages": [{
                    "role": "user",
                    "content": [
                        {"type": "text", "text": user_text},
                        {"type": "image_url", "image_url": {"url": format!("data:{};base64,{}", mime_type, base64_data)}}
                    ]
                }],
                "max_tokens": 500
            });
            (url, body)
        }
        AiProvider::Custom => {
            let base_url = settings.custom_base_url.trim();
            if base_url.is_empty() {
                return Err("Custom base URL is required".to_string());
            }
            let url = format!("{}/v1/chat/completions", base_url.trim_end_matches('/'));
            let body = json!({
                "model": settings.model,
                "messages": [{
                    "role": "user",
                    "content": [
                        {"type": "text", "text": user_text},
                        {"type": "image_url", "image_url": {"url": format!("data:{};base64,{}", mime_type, base64_data)}}
                    ]
                }],
                "max_tokens": 500
            });
            (url, body)
        }
    };

    let res = client
        .post(&url)
        .header("Authorization", format!("Bearer {}", api_key))
        .json(&body)
        .send()
        .await
        .map_err(|e| e.to_string())?;

    let status = res.status();
    let text = res.text().await.map_err(|e| e.to_string())?;

    if !status.is_success() {
        return Err(format!("API request failed ({}): {}", status.as_u16(), text));
    }

    let json_val: Value = serde_json::from_str(&text).map_err(|e| e.to_string())?;

    let content = match settings.provider {
        AiProvider::Gemini => {
            json_val["candidates"][0]["content"]["parts"][0]["text"]
                .as_str()
                .unwrap_or("")
                .to_string()
        }
        AiProvider::OpenAI | AiProvider::Custom => {
            json_val["choices"][0]["message"]["content"]
                .as_str()
                .unwrap_or("")
                .to_string()
        }
    };

    parse_llm_response(&content)
}

fn parse_llm_response(text: &str) -> Result<Value, String> {
    let trimmed = text.trim();
    let json_str = if trimmed.starts_with("```json") {
        trimmed.trim_start_matches("```json").trim_end_matches("```").trim()
    } else if trimmed.starts_with("```") {
        trimmed.trim_start_matches("```").trim_end_matches("```").trim()
    } else {
        trimmed
    };

    serde_json::from_str(json_str).map_err(|e| {
        let cleaned = json_str
            .lines()
            .filter(|l| l.trim().starts_with('{') || l.trim().starts_with('"') || l.trim() == "," || l.trim().starts_with('}'))
            .collect::<Vec<_>>()
            .join("\n");
        serde_json::from_str::<Value>(&cleaned)
            .map_err(|_| format!("Failed to parse LLM response: {}. Raw: {}", e, text))
    })
}

pub fn apply_naming_pattern(pattern: &str, metadata: &Value) -> String {
    let date = metadata.get("date").and_then(|v| v.as_str()).unwrap_or("unknown");
    let company = metadata.get("company").and_then(|v| v.as_str()).unwrap_or("unknown");
    let doctype = metadata.get("doctype").and_then(|v| v.as_str()).unwrap_or("document");

    pattern
        .replace("{date}", date)
        .replace("{company}", company)
        .replace("{doctype}", doctype)
}