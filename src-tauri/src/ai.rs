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
    #[serde(default)]
    pub system_prompt: String,
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
            system_prompt: String::new(),
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

const VISION_USER_PROMPT: &str = r#"STEP 1 — FULL DOCUMENT COMPREHENSION: Study the entire document/image thoroughly. Identify what this specific document is about (the actual subject/topic, not just the type), who issued/signed it, when it was officially issued/signed/published (distinguishing from background reference dates like laws or decrees mentioned in the body), and its administrative type and domain category.

STEP 2 — ACCURATE TEMPLATE POPULATION: Only after achieving full comprehension, extract the metadata fields in JSON format. Focus on the document's OWN issuance/publication date (typically in footer, signature block, or official header — NOT reference dates to background laws). The subject field should capture the specific project or topic name, NEVER generic words like 'Tender', 'Notice', 'Work', or 'Report'.

Do NOT output anything except the JSON object. If you CANNOT read or see the document provided, return 'ERROR_CANNOT_SEE_FILE' in the 'subject' field."#;

fn build_system_prompt(language: &str, prompt_template: &str) -> String {
    if language.eq_ignore_ascii_case("English") {
        return prompt_template.to_string();
    }

    format!(
        "{}: Output all metadata field values in {} language. For doctype, use the {} translation of document type names (e.g., Invoice, Receipt, Contract, etc.). For category, use the {} translation of category names (e.g., Finance, Personal, Work, etc.).",
        prompt_template,
        language,
        language,
        language
    )
}

fn build_vision_user_prompt(language: &str) -> String {
    if language.eq_ignore_ascii_case("English") {
        return VISION_USER_PROMPT.to_string();
    }

    format!(
        "STEP 1 — FULL DOCUMENT COMPREHENSION: Study the entire document/image thoroughly. Identify what this specific document is about (the actual subject/topic, not just the type), who issued/signed it, when it was officially issued/signed/published (distinguishing from background reference dates like laws or decrees mentioned in the body), and its administrative type and domain category.\n\nSTEP 2 — ACCURATE TEMPLATE POPULATION: Only after achieving full comprehension, extract the metadata fields in JSON format. Output all metadata field values in {} language. Do NOT output anything except the JSON object. If you CANNOT read or see the document provided, return 'ERROR_CANNOT_SEE_FILE' in the 'subject' field.",
        language
    )
}

pub fn get_all_languages(primary: &str, suggestions: &[String]) -> Vec<String> {
    let mut langs = vec![primary.to_string()];
    for s in suggestions {
        if !langs.iter().any(|l| l.eq_ignore_ascii_case(s)) {
            langs.push(s.clone());
        }
    }
    langs
}

fn build_text_prompt(system_prompt: &str, text: &str) -> String {
    format!("{}\n\nDocument text:\n{}\n\nAfter completing full document comprehension (Step 1), output the metadata JSON (Step 2).", system_prompt, text)
}

fn gemini_response_schema() -> serde_json::Value {
    serde_json::json!({
        "type": "OBJECT",
        "properties": {
            "date": {
                "type": "STRING",
                "description": "Document date in YYYYMMDD format. The date the document was issued, signed, or made effective — not publication or copyright dates."
            },
            "company": {
                "type": "STRING",
                "description": "Full legal name of the issuing/sending organization. Include ministry, department, or brand names. If none, summarize the main entity in 1 word."
            },
            "doctype": {
                "type": "STRING",
                "description": "One of: Invoice, Receipt, Contract, Tender, Bid, Agreement, Report, ID, Image, Email, Letter, Form, Bill, Memo, Certificate, Permit, License, Order, Statement. Match the document's evident purpose."
            },
            "category": {
                "type": "STRING",
                "description": "One of: Finance, Personal, Work, Legal, Medical, Education, Receipt, Invoice, Utility, Tax, Tender, Contract, Government, Other. The broadest fitting category."
            },
            "subject": {
                "type": "STRING",
                "description": "3-5 word summary of the document's specific subject. Include project names, contract numbers, or specific topics. Use underscores between words."
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

    if let Ok(parsed) = serde_json::from_str::<HashMap<String, serde_json::Value>>(&clean_json) {
        return Ok(parsed);
    }

    if let Some(block) = extract_json_braces(&clean_json) {
        if let Ok(parsed) = serde_json::from_str::<HashMap<String, serde_json::Value>>(&block) {
            return Ok(parsed);
        }
    }

    if let Ok(parsed) = serde_json::from_str::<HashMap<String, serde_json::Value>>(text.trim()) {
        return Ok(parsed);
    }

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

    let re = regex::Regex::new(r#""(\w+)""\s*:\s*"([^"]*)""#).unwrap();
    for caps in re.captures_iter(text) {
        let key = caps[1].to_lowercase();
        let val = caps[2].trim().to_string();
        data.insert(key, serde_json::Value::String(val));
    }

    let num_re = regex::Regex::new(r#""(\w+)""\s*:\s*([0-9]+\.?[0-9]*)"#).unwrap();
    for caps in num_re.captures_iter(text) {
        let key = caps[1].to_lowercase();
        if let Ok(num) = caps[2].parse::<f64>() {
            data.insert(key, serde_json::Value::Number(serde_json::Number::from_f64(num).unwrap_or(serde_json::Number::from(0))));
        }
    }

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
    language: &str,
) -> Result<DocumentMetadata, String> {
    let provider = &config.provider;
    let start = Instant::now();
    info!("AI text extraction: provider={}, model={}, language={}", provider, config.model, language);

    let sys_prompt = build_system_prompt(language, &config.system_prompt);
    let result = match provider.as_str() {
        "gemini" => gemini_text_extract(text, config, &sys_prompt).await,
        "openai" => openai_text_extract(text, config, &sys_prompt).await,
        "anthropic" => anthropic_text_extract(text, config, &sys_prompt).await,
        "ollama" | "xai" | "custom" => openai_compat_text_extract(text, config, &sys_prompt).await,
        other => Err(format!("Unknown provider: {}", other)),
    };

    info!("AI text extraction completed in {:?}", start.elapsed());
    result
}

pub async fn extract_metadata_vision(
    file_buffers: &[(String, Vec<u8>)],
    config: &AiConfig,
    language: &str,
) -> Result<DocumentMetadata, String> {
    let provider = &config.provider;
    let start = Instant::now();
    info!("AI vision extraction: provider={}, files={}, language={}", provider, file_buffers.len(), language);

    let sys_prompt = build_system_prompt(language, &config.system_prompt);
    let user_prompt = build_vision_user_prompt(language);
    let result = match provider.as_str() {
        "gemini" => gemini_vision_extract(file_buffers, config, &sys_prompt, &user_prompt).await,
        "openai" => openai_vision_extract(file_buffers, config, &sys_prompt, &user_prompt).await,
        "anthropic" => anthropic_vision_extract(file_buffers, config, &sys_prompt, &user_prompt).await,
        "ollama" | "xai" | "custom" => openai_compat_vision_extract(file_buffers, config, &sys_prompt, &user_prompt).await,
        other => Err(format!("Unknown provider: {}", other)),
    };

    info!("AI vision extraction completed in {:?}", start.elapsed());
    result
}

pub async fn extract_metadata_text_multi(
    text: &str,
    config: &AiConfig,
    languages: &[String],
) -> Vec<DocumentMetadata> {
    let mut results = Vec::new();
    for lang in languages {
        if crate::is_cancelled() {
            break;
        }
        match extract_metadata_text(text, config, lang).await {
            Ok(m) => results.push(m),
            Err(e) => {
                tracing::warn!("AI text extraction failed for language {}: {}", lang, e);
            }
        }
    }
    results
}

pub async fn extract_metadata_vision_multi(
    file_buffers: &[(String, Vec<u8>)],
    config: &AiConfig,
    languages: &[String],
) -> Vec<DocumentMetadata> {
    let mut results = Vec::new();
    for lang in languages {
        if crate::is_cancelled() {
            break;
        }
        match extract_metadata_vision(file_buffers, config, lang).await {
            Ok(m) => results.push(m),
            Err(e) => {
                tracing::warn!("AI vision extraction failed for language {}: {}", lang, e);
            }
        }
    }
    results
}


pub fn get_model_name(config: &AiConfig) -> String {
    match config.provider.as_str() {
        "gemini" => config.gemini_model.clone(),
        "openai" => config.model.clone(),
        "anthropic" => config.model.clone(),
        "ollama" | "xai" | "custom" => config.custom_model.clone(),
        _ => String::new(),
    }
}

async fn gemini_text_extract(text: &str, config: &AiConfig, sys_prompt: &str) -> Result<DocumentMetadata, String> {
    let api_key = config.api_key.trim();
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
                "text": build_text_prompt(sys_prompt, text)
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

    tracing::debug!("Gemini text response (status={}, truncated): {}", status_code, &text_resp[..text_resp.len().min(500)]);
    let result_text = parse_gemini_response(&text_resp)?;

    let parsed = parse_json_response(&result_text)?;
    Ok(data_to_metadata(&parsed))
}

async fn gemini_vision_extract(
    file_buffers: &[(String, Vec<u8>)],
    config: &AiConfig,
    sys_prompt: &str,
    user_prompt: &str,
) -> Result<DocumentMetadata, String> {
    let api_key = config.api_key.trim();
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

    let mut parts: Vec<serde_json::Value> = Vec::new();

    parts.push(serde_json::json!({ "text": sys_prompt }));

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

    parts.push(serde_json::json!({ "text": user_prompt }));

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

    tracing::debug!("Gemini vision response (status={}, truncated): {}", status_code, &text_resp[..text_resp.len().min(500)]);
    let result_text = parse_gemini_response(&text_resp)?;

    let parsed = parse_json_response(&result_text)?;

    if let Some(subject_val) = parsed.get("subject") {
        if let Some(subject_str) = subject_val.as_str() {
            if subject_str.contains("ERROR_CANNOT_SEE_FILE") {
                tracing::error!("Gemini reported it cannot see the file. Base64 data may be empty or MIME type mismatch.");
            }
        }
    }

    Ok(data_to_metadata(&parsed))
}

async fn openai_text_extract(text: &str, config: &AiConfig, sys_prompt: &str) -> Result<DocumentMetadata, String> {
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
            {"role": "system", "content": sys_prompt},
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
    sys_prompt: &str,
    user_prompt: &str,
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
    content_parts.push(serde_json::json!({"type": "text", "text": sys_prompt}));
    content_parts.push(serde_json::json!({"type": "text", "text": user_prompt}));

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

async fn anthropic_text_extract(text: &str, config: &AiConfig, sys_prompt: &str) -> Result<DocumentMetadata, String> {
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
        "system": sys_prompt,
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
    sys_prompt: &str,
    user_prompt: &str,
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

    content_parts.push(serde_json::json!({"type": "text", "text": sys_prompt}));
    content_parts.push(serde_json::json!({"type": "text", "text": user_prompt}));

    let body = serde_json::json!({
        "model": model,
        "max_tokens": 1024,
        "system": sys_prompt,
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

async fn openai_compat_text_extract(text: &str, config: &AiConfig, sys_prompt: &str) -> Result<DocumentMetadata, String> {
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
            {"role": "system", "content": sys_prompt},
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
    sys_prompt: &str,
    user_prompt: &str,
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
    content_parts.push(serde_json::json!({"type": "text", "text": sys_prompt}));
    content_parts.push(serde_json::json!({"type": "text", "text": user_prompt}));

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
