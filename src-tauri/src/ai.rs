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
    #[serde(default, rename = "company_name", alias = "company")]
    pub company_name: String,
    #[serde(default, rename = "date", alias = "document_date")]
    pub document_date: String,
    #[serde(default, rename = "doctype", alias = "document_type")]
    pub document_type: String,
    #[serde(default)]
    pub category: String,
    #[serde(default)]
    pub subject: String,
    #[serde(default)]
    pub confidence: f64,
    #[serde(default)]
    pub invoice_number: String,
    #[serde(default)]
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

Your PRIMARY directive: Extract the following fields from the document or image. NEVER return null, empty strings, or "Unknown" for fields you can reasonably infer. Use your best judgment to deduce missing information from visual context.

EXTRACT THESE FIELDS:
- "date": Document date in YYYYMMDD format. Scan for dates anywhere in the document. If no explicit date, deduce from file context or use the most plausible date. NEVER leave empty.
- "company": The company name, sender, brand, or main entity mentioned. If the document is from/to a specific organization, extract it. If none found, summarize the main entity in 1 word.
- "doctype": One of: Invoice, Receipt, Contract, Report, ID, Image, Email, Letter, Form, Bill, Memo, Certificate. Guess based on visual layout and content patterns.
- "category": One of: Finance, Personal, Work, Legal, Medical, Education, Receipt, Invoice, Utility, Tax. Choose the most fitting category.
- "subject": A very brief 2-3 word summary of the document content (e.g., "Server_Hosting", "Q3_Earnings", "Travel_Expense"). If you CANNOT read or see the document provided, return "ERROR_CANNOT_SEE_FILE" in the subject field.

CRITICAL RULES:
1. Output ONLY valid JSON — no markdown fences, no code blocks, just the raw JSON object.
2. Use underscores '_' instead of spaces in all values.
3. If a field cannot be determined with high confidence, use "Unknown" as a last resort, NOT null or empty string.
4. The JSON MUST contain ALL five required fields: date, company, doctype, category, subject."#;

const VISION_USER_PROMPT: &str = "Analyze the provided document or image above. Extract all required metadata fields (date, company, doctype, category, subject) in JSON format. Do NOT output anything except the JSON object. If you CANNOT read or see the document provided, return 'ERROR_CANNOT_SEE_FILE' in the 'subject' field.";

fn gemini_response_schema() -> serde_json::Value {
    serde_json::json!({
        "type": "OBJECT",
        "properties": {
            "date": {
                "type": "STRING",
                "description": "Document date in YYYYMMDD format. Extract from document or deduce from context. Never empty."
            },
            "company": {
                "type": "STRING",
                "description": "Company name, sender, or brand. If none, summarize the main entity in 1 word."
            },
            "doctype": {
                "type": "STRING",
                "description": "One of: Invoice, Receipt, Contract, Report, ID, Image, Email, Letter, Form, Bill, Memo, Certificate."
            },
            "category": {
                "type": "STRING",
                "description": "One of: Finance, Personal, Work, Legal, Medical, Education, Receipt, Invoice, Utility, Tax."
            },
            "subject": {
                "type": "STRING",
                "description": "Very brief 2-3 word summary of content (e.g., Server_Hosting, Q3_Earnings)."
            }
        },
        "required": ["date", "company", "doctype", "category", "subject"]
    })
}

fn clean_json_response(text: &str) -> String {
    let cleaned = text
        .trim()
        .trim_start_matches("```json")
        .trim_start_matches("```")
        .trim_end_matches("```")
        .trim()
        .to_string();
    cleaned
}

fn extract_json_braces(text: &str) -> Option<String> {
    let start = text.find('{')?;
    let mut depth = 0;
    for (i, ch) in text[start..].chars().enumerate() {
        match ch {
            '{' => depth += 1,
            '}' => {
                depth -= 1;
                if depth == 0 {
                    return Some(text[start..start + i + 1].to_string());
                }
            }
            _ => {}
        }
    }
    None
}

fn parse_json_response(text: &str) -> Result<HashMap<String, serde_json::Value>, String> {
    let clean_json = clean_json_response(text);

    // Attempt 1: strict serde_json parse on cleaned text
    if let Ok(parsed) = serde_json::from_str::<HashMap<String, serde_json::Value>>(&clean_json) {
        return Ok(parsed);
    }

    // Attempt 2: try extracting JSON braces and parsing that
    if let Some(block) = extract_json_braces(&clean_json) {
        if let Ok(parsed) = serde_json::from_str::<HashMap<String, serde_json::Value>>(&block) {
            return Ok(parsed);
        }
    }

    // Attempt 3: parse the raw trimmed text directly
    if let Ok(parsed) = serde_json::from_str::<HashMap<String, serde_json::Value>>(text.trim()) {
        return Ok(parsed);
    }

    // Attempt 4: regex-based fallback — try to extract key-value pairs from raw text
    tracing::warn!(
        "JSON parsing failed for AI response; falling back to regex extraction. Raw (first 500 chars): {}",
        &text[..text.len().min(500)]
    );
    let regex_data = parse_with_regex_fallback(text);
    if !regex_data.is_empty() {
        tracing::info!("Regex fallback extracted {} fields: {:?}", regex_data.len(), regex_data.keys().collect::<Vec<_>>());
        return Ok(regex_data);
    }

    Err(format!(
        "Failed to parse AI response as JSON. Raw response (first 500 chars): {}",
        &text[..text.len().min(500)]
    ))
}

fn parse_gemini_response(text_resp: &str) -> Result<String, String> {
    let json: serde_json::Value =
        serde_json::from_str(text_resp).map_err(|e| format!("Failed to parse API response: {}", e))?;

    if let Some(error) = json.get("error") {
        let message = error
            .get("message")
            .and_then(|v| v.as_str())
            .unwrap_or("Unknown API error");
        let code = error.get("code").and_then(|v| v.as_i64()).unwrap_or(0);
        return Err(format!("Gemini API error (code {}): {}", code, message));
    }

    let candidates = json["candidates"]
        .as_array()
        .ok_or("No candidates in response — the model may not exist or the request failed")?;
    let parts = candidates[0]["content"]["parts"]
        .as_array()
        .ok_or("No content parts in response")?;
    let result_text = parts[0]["text"]
        .as_str()
        .ok_or("No text field in response")?;

    Ok(result_text.to_string())
}

/// Regex-based fallback parser. If the AI response is not strictly valid JSON,
/// this function attempts to extract individual fields by pattern-matching
/// common JSON-like structures in the raw text.
fn parse_with_regex_fallback(text: &str) -> HashMap<String, serde_json::Value> {
    let mut data = HashMap::new();

    // Match "key": "value" patterns (handles both "key":"value" and "key": "value")
    let re = regex::Regex::new(r#""(\w+)""\s*:\s*"([^"]*)""#).unwrap();
    for caps in re.captures_iter(text) {
        let key = caps[1].to_lowercase();
        let val = caps[2].trim().to_string();
        data.insert(key, serde_json::Value::String(val));
    }

    // Match "key": number patterns
    let num_re = regex::Regex::new(r#""(\w+)""\s*:\s*([0-9]+\.?[0-9]*)"#).unwrap();
    for caps in num_re.captures_iter(text) {
        let key = caps[1].to_lowercase();
        if let Ok(num) = caps[2].parse::<f64>() {
            data.insert(key, serde_json::Value::Number(serde_json::Number::from_f64(num).unwrap_or(serde_json::Number::from(0))));
        }
    }

    // If we still have nothing, try to extract date-like patterns
    if data.is_empty() {
        let date_re = regex::Regex::new(r"(\d{4})[-/\.](\d{1,2})[-/\.](\d{1,2})").unwrap();
        if let Some(caps) = date_re.captures(text) {
            let date = format!("{}{}{}", &caps[1], &caps[2], &caps[3]);
            data.insert("date".to_string(), serde_json::Value::String(date));
        }
    }

    data
}

/// Determines the MIME type from a file path using mime_guess for accuracy.
/// Falls back to extension-based matching if mime_guess returns nothing.
fn guess_mime(path: &str) -> String {
    let mime = mime_guess::from_path(path).first();
    match mime {
        Some(m) => m.to_string(),
        None => {
            let ext = file_extension(path);
            match ext.to_lowercase().as_str() {
                ".jpg" | ".jpeg" => "image/jpeg".to_string(),
                ".png" => "image/png".to_string(),
                ".gif" => "image/gif".to_string(),
                ".webp" => "image/webp".to_string(),
                ".bmp" => "image/bmp".to_string(),
                ".tiff" | ".tif" => "image/tiff".to_string(),
                ".pdf" => "application/pdf".to_string(),
                _ => "application/octet-stream".to_string(),
            }
        }
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

    let base_url = if config.gemini_base_url.trim().is_empty() {
        GEMINI_API_BASE.to_string()
    } else {
        config.gemini_base_url.trim().to_string()
    };

    let client = Client::builder()
        .timeout(std::time::Duration::from_secs(config.timeout))
        .build()
        .map_err(|e| e.to_string())?;

    let url = format!(
        "{}/v1beta/models/{}:generateContent?key={}",
        base_url, model, api_key
    );

    let body = serde_json::json!({
        "contents": [{
            "parts": [{
                "text": format!("{}\n\nDocument text:\n{}\n\nExtract metadata JSON.", SYSTEM_PROMPT, text)
            }]
        }],
        "generationConfig": {
            "responseMimeType": "application/json",
            "responseSchema": gemini_response_schema(),
            "temperature": 0.1,
            "maxOutputTokens": 2048
        }
    });

    let resp = client
        .post(&url)
        .json(&body)
        .send()
        .await
        .map_err(|e| e.to_string())?;

    let status_code = resp.status();
    let text_resp = resp.text().await.map_err(|e| e.to_string())?;

    println!("RAW GEMINI HTTP STATUS: {}", status_code);
    println!("RAW GEMINI RESPONSE: {}", &text_resp[..text_resp.len().min(2000)]);

    tracing::info!("Gemini text response (truncated): {}", &text_resp[..text_resp.len().min(500)]);
    let result_text = parse_gemini_response(&text_resp)?;

    let parsed = parse_json_response(&result_text)?;
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

    let base_url = if config.gemini_base_url.trim().is_empty() {
        GEMINI_API_BASE.to_string()
    } else {
        config.gemini_base_url.trim().to_string()
    };

    let client = Client::builder()
        .timeout(std::time::Duration::from_secs(config.timeout.max(60)))
        .build()
        .map_err(|e| e.to_string())?;

    let url = format!(
        "{}/v1beta/models/{}:generateContent?key={}",
        base_url, model, api_key
    );

    tracing::info!(
        "Gemini vision extract: model={}, files={}",
        model,
        file_buffers.len()
    );

    // Build the multi-modal parts array: system prompt + all files + user prompt
    let mut parts: Vec<serde_json::Value> = Vec::new();

    // 1. System prompt as first text part
    parts.push(serde_json::json!({ "text": SYSTEM_PROMPT }));

    // 2. File data as inline_data parts (Base64 encoded)
    for (path, buffer) in file_buffers {
        if buffer.is_empty() {
            tracing::warn!("Empty file buffer for path: {}", path);
            continue;
        }

        let mime = guess_mime(path);
        let b64 = general_purpose::STANDARD.encode(buffer);

        tracing::info!(
            "Gemini vision: file={}, mime_type={}, buffer_size={}, base64_length={}",
            path,
            mime,
            buffer.len(),
            b64.len()
        );

        parts.push(serde_json::json!({
            "inlineData": {
                "mimeType": mime,
                "data": b64
            }
        }));
    }

    // 3. Explicit user instruction as final text part
    parts.push(serde_json::json!({ "text": VISION_USER_PROMPT }));

    let body = serde_json::json!({
        "contents": [{ "parts": parts }],
        "generationConfig": {
            "responseMimeType": "application/json",
            "responseSchema": gemini_response_schema(),
            "temperature": 0.1,
            "maxOutputTokens": 2048
        }
    });

    let body_str = serde_json::to_string(&body).unwrap_or_default();
    tracing::debug!("Gemini vision request body (truncated): {}", &body_str[..body_str.len().min(500)]);

    let resp = client
        .post(&url)
        .json(&body)
        .send()
        .await
        .map_err(|e| e.to_string())?;

    let status_code = resp.status();
    let text_resp = resp.text().await.map_err(|e| e.to_string())?;

    println!("RAW GEMINI HTTP STATUS: {}", status_code);
    println!("RAW GEMINI RESPONSE: {}", &text_resp[..text_resp.len().min(2000)]);

    tracing::info!("Gemini vision response (truncated): {}", &text_resp[..text_resp.len().min(500)]);
    let result_text = parse_gemini_response(&text_resp)?;

    let parsed = parse_json_response(&result_text)?;

    // Log if subject contains ERROR_CANNOT_SEE_FILE
    if let Some(subject_val) = parsed.get("subject") {
        if let Some(subject_str) = subject_val.as_str() {
            if subject_str.contains("ERROR_CANNOT_SEE_FILE") {
                tracing::error!("Gemini reported it cannot see the file. Base64 data may be empty or MIME type mismatch.");
            }
        }
    }

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
        let mime = guess_mime(path);
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
        let mime = guess_mime(path);
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
        let mime = guess_mime(path);
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

    let meta = DocumentMetadata {
        company_name: extract_str_field(data, "company", "Unknown"),
        document_date: extract_str_field(data, "date", ""),
        document_type: extract_str_field(data, "doctype", "Unknown"),
        category: extract_str_field(data, "category", "Unknown"),
        subject: extract_str_field(data, "subject", "Unknown"),
        confidence,
        invoice_number: extract_str_field(data, "invoice_number", ""),
        total_amount: extract_str_field(data, "total_amount", ""),
    };

    tracing::debug!(
        "Mapped metadata: date={}, company={}, doctype={}, category={}, subject={}",
        meta.document_date, meta.company_name, meta.document_type, meta.category, meta.subject
    );

    meta
}

/// Safely extracts a string field from JSON data.
/// Handles missing keys, null values, and whitespace-only strings.
/// Returns `default_value` if the field is absent, null, or empty.
fn extract_str_field(
    data: &HashMap<String, serde_json::Value>,
    key: &str,
    default_value: &str,
) -> String {
    data.get(key)
        .and_then(|v| v.as_str())
        .filter(|s| !s.trim().is_empty())
        .map(|s| s.trim().to_string())
        .unwrap_or_else(|| default_value.to_string())
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
