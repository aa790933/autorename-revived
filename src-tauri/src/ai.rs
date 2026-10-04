use base64::{engine::general_purpose, Engine as _};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::Instant;
use tracing::info;

const GEMINI_DEFAULT_MODEL: &str = "gemini-2.5-flash";
const GEMINI_API_BASE: &str = "https://generativelanguage.googleapis.com";
const OPENAI_API_BASE: &str = "https://api.openai.com";
const ANTHROPIC_API_BASE: &str = "https://api.anthropic.com";
const XAI_API_BASE: &str = "https://api.x.ai";
const OLLAMA_DEFAULT_BASE: &str = "http://localhost:11434";

fn default_system_prompt() -> String {
    r#"You are an advanced, domain-agnostic Universal Document & File Intelligence Engine.
Your task is to analyze ANY input file (Invoices, Checks, IDs, Contracts, Tenders, Receipts, Medical Reports, Letters, Photos, Screenshots, Code, etc.) and extract strict, precise metadata for automated file renaming.

CORE EXTRACTION RULES:
1. DOMAIN ADAPTABILITY:
   - For Financial/Administrative (Invoices, Receipts, Checks): Extract the vendor/bank/issuer, the exact document type, and the item/service/check-number.
   - For Identity/Legal (IDs, Passports, Contracts, Decrees): Extract the issuing authority/person, type, and identifier/subject.
    - For Visual Media (Photos, Screenshots, Artwork): Set document_nature to 'Photo' or 'Screenshot', and extract a 2-4 word visual description as specific_subject.
   - For Technical/Academic (Reports, Papers, Code): Extract the main topic, organization, and document structure.

2. STRICT SEPARATION & NO REDUNDANCY:
   - 'document_nature' is the administrative/structural category (e.g., Invoice, Check, Contract, ID_Card, Photo).
   - 'specific_subject' is the UNIQUE context or topic.
   - RULE: 'specific_subject' MUST NEVER contain any words present in 'document_nature' or 'issuer_entity_short'.

3. ENTITY CLEANING:
   - Extract short, functional entity names. Strip legal prefixes/suffixes (e.g., use 'Amazon' instead of 'Amazon.com Services LLC', use 'DTP_Tebessa' instead of full official header).

4. STRICT DATE FORMATTING:
   - Output 'date_YYYY_MM_DD' in YYYY-MM-DD. Always pick the primary effective/issuance date (e.g., invoice date over due date). If no date exists (e.g., a photo), leave as empty string."#.to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiConfig {
    pub provider: String,
    pub api_key: String,
    pub model: String,
    pub gemini_model: String,
    /// OpenAI-compatible endpoint for `openai` / `anthropic` / `xai` providers.
    /// Used as a fallback for `custom` when `custom_base_url` is empty.
    #[serde(default)]
    pub base_url: String,
    pub gemini_base_url: String,
    pub custom_model: String,
    /// Dedicated endpoint for the `custom` provider (takes priority over `base_url`).
    pub custom_base_url: String,
    /// Dedicated endpoint for the `ollama` provider (takes priority over `base_url`).
    #[serde(default)]
    pub ollama_base_url: String,
    pub temperature: f64,
    pub timeout: u64,
    #[serde(default = "default_system_prompt")]
    pub system_prompt: String,
}

impl Default for AiConfig {
    fn default() -> Self {
        Self {
            provider: "gemini".to_string(),
            api_key: String::new(),
            model: "gpt-4o-mini".to_string(),
            gemini_model: GEMINI_DEFAULT_MODEL.to_string(),
            base_url: String::new(),
            gemini_base_url: String::new(),
            custom_model: String::new(),
            custom_base_url: String::new(),
            ollama_base_url: String::new(),
            temperature: 0.0,
            timeout: 30,
            system_prompt: default_system_prompt(),
        }
    }
}

impl AiConfig {
    /// Returns the user's system prompt if non-empty, else the built-in default.
    pub fn effective_system_prompt(&self) -> String {
        let p = self.system_prompt.trim();
        if p.is_empty() {
            default_system_prompt()
        } else {
            self.system_prompt.clone()
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocumentMetadata {
    #[serde(default, rename = "issuer_entity_short", alias = "company")]
    pub company_name: String,
    #[serde(default, rename = "date_YYYY_MM_DD", alias = "date")]
    pub document_date: String,
    #[serde(default, rename = "document_nature", alias = "doctype")]
    pub document_type: String,
    #[serde(default, rename = "specific_subject", alias = "subject")]
    pub subject: String,
    #[serde(default)]
    pub is_unreadable_or_error: bool,
}

impl Default for DocumentMetadata {
    fn default() -> Self {
        Self {
            company_name: String::new(),
            document_date: String::new(),
            document_type: String::new(),
            subject: String::new(),
            is_unreadable_or_error: false,
        }
    }
}

impl DocumentMetadata {
    /// True when no field carries meaningful content.
    pub fn is_empty(&self) -> bool {
        self.company_name.is_empty()
            && self.document_date.is_empty()
            && self.document_type.is_empty()
            && self.subject.is_empty()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestConnectionResult {
    pub success: bool,
    pub message: String,
    pub latency_ms: u128,
    pub provider: String,
}

const VISION_USER_PROMPT: &str = r#"Analyze this document/image. Extract the following metadata fields as JSON:
- date_YYYY_MM_DD: Primary issuance/effective date in YYYY-MM-DD format. If no date exists, use empty string.
- issuer_entity_short: Short functional entity name (strip legal suffixes like 'LLC', 'Inc.'). If no issuer, use empty string.
- document_nature: Administrative/structural type (e.g., Invoice, Contract, ID_Card, Photo, Receipt, Letter, Report).
- specific_subject: 2-4 word description of the UNIQUE topic. Must NOT repeat words from document_nature or issuer_entity_short. If unreadable, describe what you see.
- is_unreadable_or_error: Set to true if the document cannot be read or understood.

Do NOT output anything except the JSON object. If you CANNOT read the document, set is_unreadable_or_error to true and provide best-effort values for the other fields."#;

fn build_system_prompt(language: &str, prompt_template: &str) -> String {
    if language.eq_ignore_ascii_case("English") {
        return prompt_template.to_string();
    }

    format!(
        "{}\n\nIMPORTANT: Output all metadata field values in {} language. For document_nature, use the {} translation of document type names (e.g., Invoice, Receipt, Contract, etc.).",
        prompt_template,
        language,
        language
    )
}

fn build_vision_user_prompt(language: &str) -> String {
    if language.eq_ignore_ascii_case("English") {
        return VISION_USER_PROMPT.to_string();
    }

    format!(
        "Analyze this document/image. Extract the following metadata fields as JSON (output all values in {} language):\n- date_YYYY_MM_DD: Primary issuance/effective date in YYYY-MM-DD format. If no date exists, use empty string.\n- issuer_entity_short: Short functional entity name (strip legal suffixes like 'LLC', 'Inc.'). If no issuer, use empty string.\n- document_nature: Administrative/structural type (e.g., Invoice, Contract, ID_Card, Photo, Receipt, Letter, Report).\n- specific_subject: 2-4 word description of the UNIQUE topic. Must NOT repeat words from document_nature or issuer_entity_short. If unreadable, describe what you see.\n- is_unreadable_or_error: Set to true if the document cannot be read or understood.\n\nDo NOT output anything except the JSON object. If you CANNOT read the document, set is_unreadable_or_error to true and provide best-effort values for the other fields.",
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
    format!(
        "{}\n\nDocument text:\n{}\n\nExtract the metadata JSON now.",
        system_prompt, text
    )
}

fn gemini_response_schema() -> serde_json::Value {
    serde_json::json!({
        "type": "OBJECT",
        "properties": {
            "date_YYYY_MM_DD": {
                "type": "STRING",
                "description": "Primary issuance/effective date in YYYY-MM-DD format. If no date exists, use empty string."
            },
            "issuer_entity_short": {
                "type": "STRING",
                "description": "Short functional entity name (strip legal suffixes like LLC, Inc.). If no issuer, use empty string."
            },
            "document_nature": {
                "type": "STRING",
                "description": "Administrative/structural document type (e.g., Invoice, Contract, ID_Card, Photo, Receipt, Letter, Report, Certificate, Tender)."
            },
            "specific_subject": {
                "type": "STRING",
                "description": "2-4 word description of the UNIQUE topic. Must NOT repeat words from document_nature or issuer_entity_short."
            },
            "is_unreadable_or_error": {
                "type": "BOOLEAN",
                "description": "Set to true if the document cannot be read or understood."
            }
        },
        "required": ["date_YYYY_MM_DD", "issuer_entity_short", "document_nature", "specific_subject", "is_unreadable_or_error"],
        "additionalProperties": false
    })
}

fn clean_json_response(text: &str) -> String {
    text.trim()
        .trim_start_matches("```json")
        .trim_start_matches("```")
        .trim_end_matches("```")
        .trim()
        .to_string()
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
        truncate_str_chars(text, 500)
    );
    let regex_data = parse_with_regex_fallback(text);
    if !regex_data.is_empty() {
        tracing::info!(
            "Regex fallback extracted {} fields: {:?}",
            regex_data.len(),
            regex_data.keys().collect::<Vec<_>>()
        );
        return Ok(regex_data);
    }

    Err(format!(
        "Failed to parse AI response as JSON. Raw response (first 500 chars): {}",
        truncate_str_chars(text, 500)
    ))
}

/// Truncate to `max` *characters* (not bytes). The slicing form
/// `&s[..s.len().min(max)]` panics when `max` lands inside a multi-byte
/// UTF-8 sequence — which this app hits constantly, since AI responses echo
/// Arabic, Hindi and Cyrillic document text.
fn truncate_str_chars(s: &str, max: usize) -> String {
    s.chars().take(max).collect()
}

/// Image media types the vision APIs accept as inline/base64 data.
///
/// `application/pdf` is deliberately absent: Anthropic rejects it as an image
/// block and OpenAI's `image_url` expects an image, so a PDF routed to vision
/// must go through Gemini (which accepts `application/pdf` inline).
const SUPPORTED_VISION_IMAGE_MIMES: [&str; 6] = [
    "image/png",
    "image/jpeg",
    "image/gif",
    "image/webp",
    "image/bmp",
    "image/tiff",
];

fn parse_gemini_response(text_resp: &str) -> Result<String, String> {
    let json: serde_json::Value = serde_json::from_str(text_resp)
        .map_err(|e| format!("Failed to parse API response: {}", e))?;

    if let Some(error) = json.get("error") {
        let message = error
            .get("message")
            .and_then(|v| v.as_str())
            .unwrap_or("Unknown API error");
        let code = error.get("code").and_then(|v| v.as_i64()).unwrap_or(0);
        return Err(format!("Gemini API error (code {}): {}", code, message));
    }

    let candidates = json["candidates"].as_array().ok_or(
        "No candidates in response — the model may not exist or the request failed",
    )?;
    if candidates.is_empty() {
        return Err(
            "Empty candidates array in response — the model may have refused the request or content was filtered"
                .to_string(),
        );
    }
    let parts = candidates[0]["content"]["parts"]
        .as_array()
        .ok_or("No content parts in response")?;
    if parts.is_empty() {
        return Err("Empty content parts array in response".to_string());
    }
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

    let re = regex::Regex::new(r#""(\w+)"\s*:\s*"([^"]*)""#).unwrap();
    for caps in re.captures_iter(text) {
        let key = caps[1].to_lowercase();
        let val = caps[2].trim().to_string();
        data.insert(key, serde_json::Value::String(val));
    }

    let num_re = regex::Regex::new(r#""(\w+)"\s*:\s*([0-9]+\.?[0-9]*)"#).unwrap();
    for caps in num_re.captures_iter(text) {
        let key = caps[1].to_lowercase();
        if let Ok(num) = caps[2].parse::<f64>() {
            data.insert(
                key,
                serde_json::Value::Number(
                    serde_json::Number::from_f64(num)
                        .unwrap_or(serde_json::Number::from(0)),
                ),
            );
        }
    }

    // Handle boolean fields
    let bool_re =
        regex::Regex::new(r#""(is_unreadable_or_error)"\s*:\s*(true|false)"#).unwrap();
    for caps in bool_re.captures_iter(text) {
        let key = caps[1].to_lowercase();
        let val = caps[2] == *"true";
        data.insert(key, serde_json::Value::Bool(val));
    }

    if data.is_empty() {
        let date_re = regex::Regex::new(r"(\d{4})[-/\.](\d{1,2})[-/\.](\d{1,2})").unwrap();
        if let Some(caps) = date_re.captures(text) {
            let date = format!("{}-{}-{}", &caps[1], &caps[2], &caps[3]);
            data.insert(
                "date_YYYY_MM_DD".to_string(),
                serde_json::Value::String(date),
            );
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

/// Builds a reqwest client with the configured timeout.
fn build_client(timeout_secs: u64) -> Result<Client, String> {
    Client::builder()
        .timeout(std::time::Duration::from_secs(timeout_secs))
        .build()
        .map_err(|e| e.to_string())
}

/// Effective request timeout: never below 60s for vision (large uploads).
fn request_timeout(config: &AiConfig) -> u64 {
    config.timeout.max(60)
}

// ---------------------------------------------------------------------------
// Provider base URL resolution (config override, else provider default)
//
// Each provider has a dedicated override field; `base_url` is the generic
// OpenAI-compatible override. The dedicated field always wins.
// ---------------------------------------------------------------------------

fn first_non_empty<'a>(candidates: impl IntoIterator<Item = &'a str>) -> Option<&'a str> {
    candidates
        .into_iter()
        .map(str::trim)
        .find(|s| !s.is_empty())
}

/// Trim, drop a trailing slash and a trailing `/v1` (callers re-add it).
fn normalize_base(url: &str) -> String {
    let trimmed = url.trim().trim_end_matches('/');
    trimmed
        .strip_suffix("/v1")
        .or_else(|| trimmed.strip_suffix("/V1"))
        .map(|s| s.trim_end_matches('/').to_string())
        .unwrap_or_else(|| trimmed.to_string())
}

fn gemini_base_url(config: &AiConfig) -> String {
    normalize_base(
        first_non_empty([config.gemini_base_url.as_str()]).unwrap_or(GEMINI_API_BASE),
    )
}

fn openai_base_url(config: &AiConfig) -> String {
    normalize_base(
        first_non_empty([config.base_url.as_str()]).unwrap_or(OPENAI_API_BASE),
    )
}

fn anthropic_base_url(config: &AiConfig) -> String {
    normalize_base(
        first_non_empty([config.base_url.as_str()]).unwrap_or(ANTHROPIC_API_BASE),
    )
}

fn xai_base_url(config: &AiConfig) -> String {
    normalize_base(first_non_empty([config.base_url.as_str()]).unwrap_or(XAI_API_BASE))
}

fn ollama_base_url(config: &AiConfig) -> String {
    normalize_base(
        first_non_empty([
            config.ollama_base_url.as_str(),
            config.base_url.as_str(),
        ])
        .unwrap_or(OLLAMA_DEFAULT_BASE),
    )
}

/// Resolve the OpenAI-compatible base URL for `ollama` / `xai` / `custom`.
/// The result never includes a trailing `/v1`; use [`chat_url`] to build the
/// chat completions endpoint.
fn compat_base_url(config: &AiConfig) -> Result<String, String> {
    let base = match config.provider.as_str() {
        "ollama" => ollama_base_url(config),
        "xai" => xai_base_url(config),
        _ => {
            let custom = first_non_empty([
                config.custom_base_url.as_str(),
                config.base_url.as_str(),
            ]);
            match custom {
                Some(url) => normalize_base(url),
                None => return Err("Custom API base URL is required".to_string()),
            }
        }
    };
    // Accept bases that already end in /v1 and strip it so chat_url can re-add.
    let base = base
        .strip_suffix("/v1")
        .or_else(|| base.strip_suffix("/V1"))
        .map(normalize_base)
        .unwrap_or(base);
    Ok(base)
}

/// Chat completions endpoint for an OpenAI-compatible provider.
fn chat_url(base: &str) -> String {
    format!("{}/v1/chat/completions", base)
}

/// Models listing endpoint for an OpenAI-compatible provider.
fn models_url(base: &str) -> String {
    format!("{}/v1/models", base)
}

/// Standard completion model per provider (used when the configured model is empty).
fn default_model_for(provider: &str) -> &'static str {
    match provider {
        "openai" => "gpt-4o-mini",
        "anthropic" => "claude-3-5-haiku-latest",
        "ollama" => "llama3.2",
        "xai" => "grok-3-beta",
        _ => "gpt-4o-mini",
    }
}

// ---------------------------------------------------------------------------
// Multi-language extraction (aligned results)
// ---------------------------------------------------------------------------

/// Extracts metadata via text for each requested language.
///
/// Result index `i` corresponds to `languages[i]`. If the primary-language
/// request (index 0) fails, an empty Vec is returned so the caller treats the
/// file as failed instead of silently renaming it with no metadata. A failed
/// *suggestion* language degrades to an unreadable placeholder, keeping
/// indices aligned for the caller.
pub async fn extract_metadata_text_multi(
    text: &str,
    config: &AiConfig,
    languages: &[String],
) -> Vec<DocumentMetadata> {
    let mut results = Vec::new();
    for (i, lang) in languages.iter().enumerate() {
        if crate::is_cancelled() {
            break;
        }
        match extract_metadata_text(text, config, lang).await {
            Ok(m) => {
                tracing::info!(
                    "Text extraction succeeded for lang '{}': company='{}', date='{}', type='{}', subject='{}'",
                    lang,
                    m.company_name,
                    m.document_date,
                    m.document_type,
                    m.subject
                );
                results.push(m);
            }
            Err(e) => {
                if i == 0 {
                    tracing::error!(
                        "AI text extraction FAILED for primary language '{}': {}",
                        lang,
                        e
                    );
                    return Vec::new();
                }
                tracing::warn!(
                    "AI text extraction failed for suggestion language '{}': {} — inserting placeholder",
                    lang,
                    e
                );
                results.push(DocumentMetadata {
                    is_unreadable_or_error: true,
                    ..Default::default()
                });
            }
        }
    }
    results
}

/// Vision counterpart of [`extract_metadata_text_multi`] (same alignment rules).
pub async fn extract_metadata_vision_multi(
    file_buffers: &[(String, Vec<u8>)],
    config: &AiConfig,
    languages: &[String],
) -> Vec<DocumentMetadata> {
    let mut results = Vec::new();
    for (i, lang) in languages.iter().enumerate() {
        if crate::is_cancelled() {
            break;
        }
        match extract_metadata_vision(file_buffers, config, lang).await {
            Ok(m) => {
                tracing::info!(
                    "Vision extraction succeeded for lang '{}': company='{}', date='{}', type='{}', subject='{}'",
                    lang,
                    m.company_name,
                    m.document_date,
                    m.document_type,
                    m.subject
                );
                results.push(m);
            }
            Err(e) => {
                if i == 0 {
                    tracing::error!(
                        "AI vision extraction FAILED for primary language '{}': {}",
                        lang,
                        e
                    );
                    return Vec::new();
                }
                tracing::warn!(
                    "AI vision extraction failed for suggestion language '{}': {} — inserting placeholder",
                    lang,
                    e
                );
                results.push(DocumentMetadata {
                    is_unreadable_or_error: true,
                    ..Default::default()
                });
            }
        }
    }
    results
}

// ---------------------------------------------------------------------------
// Public single-file entry points
// ---------------------------------------------------------------------------

pub async fn extract_metadata_text(
    text: &str,
    config: &AiConfig,
    language: &str,
) -> Result<DocumentMetadata, String> {
    let provider = config.provider.as_str();
    let start = Instant::now();
    info!(
        "AI text extraction: provider={}, model={}, language={}",
        provider,
        get_model_name(config),
        language
    );

    let sys_prompt = build_system_prompt(language, &config.effective_system_prompt());
    let result = match provider {
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
    let provider = config.provider.as_str();
    let start = Instant::now();
    info!(
        "AI vision extraction: provider={}, files={}, language={}",
        provider,
        file_buffers.len(),
        language
    );

    let sys_prompt = build_system_prompt(language, &config.effective_system_prompt());
    let user_prompt = build_vision_user_prompt(language);
    let result = match provider {
        "gemini" => gemini_vision_extract(file_buffers, config, &sys_prompt, &user_prompt).await,
        "openai" => openai_vision_extract(file_buffers, config, &sys_prompt, &user_prompt).await,
        "anthropic" => {
            anthropic_vision_extract(file_buffers, config, &sys_prompt, &user_prompt).await
        }
        "ollama" | "xai" | "custom" => {
            openai_compat_vision_extract(file_buffers, config, &sys_prompt, &user_prompt).await
        }
        other => Err(format!("Unknown provider: {}", other)),
    };

    info!("AI vision extraction completed in {:?}", start.elapsed());
    result
}

pub fn get_model_name(config: &AiConfig) -> String {
    match config.provider.as_str() {
        "gemini" => {
            let m = config.gemini_model.trim();
            if m.is_empty() {
                GEMINI_DEFAULT_MODEL.to_string()
            } else {
                m.to_string()
            }
        }
        "openai" | "anthropic" => {
            let m = config.model.trim();
            if m.is_empty() {
                default_model_for(config.provider.as_str()).to_string()
            } else {
                m.to_string()
            }
        }
        "ollama" | "xai" | "custom" => {
            let m = config.custom_model.trim();
            if m.is_empty() {
                default_model_for(config.provider.as_str()).to_string()
            } else {
                m.to_string()
            }
        }
        _ => String::new(),
    }
}

// ---------------------------------------------------------------------------
// Gemini
// ---------------------------------------------------------------------------

async fn gemini_text_extract(
    text: &str,
    config: &AiConfig,
    sys_prompt: &str,
) -> Result<DocumentMetadata, String> {
    let api_key = config.api_key.trim();
    if api_key.is_empty() {
        return Err("Gemini API key is required".to_string());
    }
    let model = get_model_name(config);
    let base_url = gemini_base_url(config);
    let client = build_client(config.timeout)?;

    let url = format!("{}/v1beta/models/{}:generateContent", base_url, model);

    let body = serde_json::json!({
        "contents": [{
            "role": "user",
            "parts": [{ "text": sys_prompt }, { "text": build_text_prompt("", text) }]
        }],
        "generationConfig": {
            "responseMimeType": "application/json",
            "responseSchema": gemini_response_schema(),
            "temperature": config.temperature,
            "maxOutputTokens": 8192
        }
    });

    // The key travels in the x-goog-api-key header, not the query string:
    // `?key=` values end up in proxy logs, crash dumps and browser history.
    let resp = client
        .post(&url)
        .header("x-goog-api-key", api_key)
        .json(&body)
        .send()
        .await
        .map_err(|e| e.to_string())?;

    let status_code = resp.status();
    let text_resp = resp.text().await.map_err(|e| e.to_string())?;

    tracing::debug!(
        "Gemini text response (status={}, truncated): {}",
        status_code,
        truncate_str_chars(&text_resp, 500)
    );
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
    let model = get_model_name(config);
    let base_url = gemini_base_url(config);
    let client = build_client(request_timeout(config))?;

    let url = format!("{}/v1beta/models/{}:generateContent", base_url, model);

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
        "contents": [{ "role": "user", "parts": parts }],
        "generationConfig": {
            "responseMimeType": "application/json",
            "responseSchema": gemini_response_schema(),
            "temperature": config.temperature,
            "maxOutputTokens": 8192
        }
    });

    let resp = client
        .post(&url)
        .header("x-goog-api-key", api_key)
        .json(&body)
        .send()
        .await
        .map_err(|e| e.to_string())?;

    let status_code = resp.status();
    let text_resp = resp.text().await.map_err(|e| e.to_string())?;

    tracing::debug!(
        "Gemini vision response (status={}, truncated): {}",
        status_code,
        truncate_str_chars(&text_resp, 500)
    );
    let result_text = parse_gemini_response(&text_resp)?;

    let parsed = parse_json_response(&result_text)?;

    if let Some(unreadable) = parsed
        .get("is_unreadable_or_error")
        .and_then(|v| v.as_bool())
    {
        if unreadable {
            tracing::warn!(
                "Gemini reported the document is unreadable. Base64 data may be empty or MIME type mismatch."
            );
        }
    }

    Ok(data_to_metadata(&parsed))
}

// ---------------------------------------------------------------------------
// OpenAI
// ---------------------------------------------------------------------------

/// Extracts the error message from an OpenAI-style error body, if present.
fn openai_error_message(json: &serde_json::Value) -> Option<String> {
    json.get("error")
        .and_then(|e| e.get("message"))
        .and_then(|m| m.as_str())
        .map(|s| s.to_string())
}

async fn openai_text_extract(
    text: &str,
    config: &AiConfig,
    sys_prompt: &str,
) -> Result<DocumentMetadata, String> {
    let api_key = config.api_key.trim();
    if api_key.is_empty() {
        return Err("OpenAI API key is required".to_string());
    }
    let model = get_model_name(config);
    let client = build_client(config.timeout)?;

    let url = format!("{}/v1/chat/completions", openai_base_url(config));

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

    let status = resp.status();
    let json: serde_json::Value = resp.json().await.map_err(|e| e.to_string())?;
    if !status.is_success() {
        return Err(match openai_error_message(&json) {
            Some(msg) => format!("OpenAI API error ({}): {}", status, msg),
            None => format!("OpenAI API returned status {}", status),
        });
    }
    let result_text = openai_compat_message_text(&json)?;

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
    let model = get_model_name(config);
    let client = build_client(request_timeout(config))?;

    let url = format!("{}/v1/chat/completions", openai_base_url(config));

    let mut content_parts = Vec::new();
    content_parts.push(serde_json::json!({"type": "text", "text": sys_prompt}));
    content_parts.push(serde_json::json!({"type": "text", "text": user_prompt}));

    for (path, buffer) in file_buffers {
        if buffer.is_empty() {
            tracing::warn!("Empty file buffer for path: {}", path);
            continue;
        }
        let mime = guess_mime(path);
        if !SUPPORTED_VISION_IMAGE_MIMES.contains(&mime.as_str()) {
            tracing::warn!(
                "Skipping {} for OpenAI vision: media type '{}' is not an accepted image type",
                path,
                mime
            );
            continue;
        }
        let b64 = general_purpose::STANDARD.encode(buffer);
        content_parts.push(serde_json::json!({
            "type": "image_url",
            "image_url": {"url": format!("data:{};base64,{}", mime, b64), "detail": "auto"}
        }));
    }
    if content_parts.len() == 2 {
        return Err(
            "No image could be sent to OpenAI vision (unsupported or empty media type)".to_string(),
        );
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

    let status = resp.status();
    let json: serde_json::Value = resp.json().await.map_err(|e| e.to_string())?;
    if !status.is_success() {
        return Err(match openai_error_message(&json) {
            Some(msg) => format!("OpenAI API error ({}): {}", status, msg),
            None => format!("OpenAI API returned status {}", status),
        });
    }
    let result_text = openai_compat_message_text(&json)?;

    let parsed = parse_json_response(result_text)?;
    Ok(data_to_metadata(&parsed))
}

/// Pull `choices[0].message.content` out of an OpenAI-compatible response.
///
/// Reasoning-capable models (and several OpenAI-compatible servers) may return
/// a null or empty `content` with the real answer in `reasoning_content`, or
/// split it across an array of content parts — both shapes are handled here
/// instead of failing the whole document.
fn openai_compat_message_text(json: &serde_json::Value) -> Result<&str, String> {
    let choices = json["choices"]
        .as_array()
        .ok_or("No choices in response")?;
    if choices.is_empty() {
        return Err(
            "Empty choices array in response — the API may have refused the request or content was filtered"
                .to_string(),
        );
    }
    let message = &choices[0]["message"];
    if let Some(text) = message["content"].as_str() {
        if !text.trim().is_empty() {
            return Ok(text);
        }
    }
    // Some servers return content as an array of typed parts.
    if let Some(parts) = message["content"].as_array() {
        for part in parts {
            if let Some(text) = part["text"].as_str() {
                if !text.trim().is_empty() {
                    return Ok(text);
                }
            }
        }
    }
    if let Some(reasoning) = message["reasoning_content"].as_str() {
        if !reasoning.trim().is_empty() {
            return Ok(reasoning);
        }
    }
    Err("No content in response".to_string())
}

// ---------------------------------------------------------------------------
// Anthropic
// ---------------------------------------------------------------------------

async fn anthropic_text_extract(
    text: &str,
    config: &AiConfig,
    sys_prompt: &str,
) -> Result<DocumentMetadata, String> {
    let api_key = config.api_key.trim();
    if api_key.is_empty() {
        return Err("Anthropic API key is required".to_string());
    }
    let model = get_model_name(config);
    let client = build_client(config.timeout)?;

    let url = format!("{}/v1/messages", anthropic_base_url(config));

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

    let status = resp.status();
    let json: serde_json::Value = resp.json().await.map_err(|e| e.to_string())?;
    if !status.is_success() {
        let msg = json
            .get("error")
            .and_then(|e| e.get("message"))
            .and_then(|m| m.as_str())
            .unwrap_or("unknown error");
        return Err(format!("Anthropic API error ({}): {}", status, msg));
    }
    let content_arr = json["content"]
        .as_array()
        .ok_or("No content array in response")?;
    if content_arr.is_empty() {
        return Err(
            "Empty content array in response — the API may have refused the request or content was filtered"
                .to_string(),
        );
    }
    let result_text = content_arr[0]["text"]
        .as_str()
        .ok_or("No text in response")?;

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
    let model = get_model_name(config);
    let client = build_client(request_timeout(config))?;

    let url = format!("{}/v1/messages", anthropic_base_url(config));

    // Anthropic accepts image blocks only; PDFs and other document types have
    // to go through a provider that takes them inline (Gemini). Sending an
    // unsupported media type makes the whole request fail with a 400, so the
    // unusable entries are dropped with a warning instead.
    let mut content_parts = Vec::new();
    for (path, buffer) in file_buffers {
        if buffer.is_empty() {
            tracing::warn!("Empty file buffer for path: {}", path);
            continue;
        }
        let mime = guess_mime(path);
        if !SUPPORTED_VISION_IMAGE_MIMES.contains(&mime.as_str()) {
            tracing::warn!(
                "Skipping {} for Anthropic vision: media type '{}' is not an accepted image type",
                path,
                mime
            );
            continue;
        }
        let b64 = general_purpose::STANDARD.encode(buffer);
        content_parts.push(serde_json::json!({
            "type": "image",
            "source": {"type": "base64", "media_type": mime, "data": b64}
        }));
    }
    if content_parts.is_empty() {
        return Err(
            "No image could be sent to Anthropic vision (unsupported or empty media type)"
                .to_string(),
        );
    }

    // `system` carries the system prompt; repeating it as a text block inside
    // the user turn duplicated the instructions in every request.
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

    let status = resp.status();
    let json: serde_json::Value = resp.json().await.map_err(|e| e.to_string())?;
    if !status.is_success() {
        let msg = json
            .get("error")
            .and_then(|e| e.get("message"))
            .and_then(|m| m.as_str())
            .unwrap_or("unknown error");
        return Err(format!("Anthropic API error ({}): {}", status, msg));
    }
    let content_arr = json["content"]
        .as_array()
        .ok_or("No content array in response")?;
    if content_arr.is_empty() {
        return Err(
            "Empty content array in response — the API may have refused the request or content was filtered"
                .to_string(),
        );
    }
    let result_text = content_arr[0]["text"]
        .as_str()
        .ok_or("No text in response")?;

    let parsed = parse_json_response(result_text)?;
    Ok(data_to_metadata(&parsed))
}

// ---------------------------------------------------------------------------
// OpenAI-compatible (Ollama / xAI / custom)
// ---------------------------------------------------------------------------

async fn openai_compat_text_extract(
    text: &str,
    config: &AiConfig,
    sys_prompt: &str,
) -> Result<DocumentMetadata, String> {
    let base_url = compat_base_url(config)?;
    let model = get_model_name(config);
    let client = build_client(config.timeout)?;

    let url = format!("{}/v1/chat/completions", base_url);

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
        .bearer_auth(config.api_key.trim())
        .json(&body)
        .send()
        .await
        .map_err(|e| e.to_string())?;

    let status = resp.status();
    let json: serde_json::Value = resp.json().await.map_err(|e| e.to_string())?;
    if !status.is_success() {
        return Err(match openai_error_message(&json) {
            Some(msg) => format!("API error ({}): {}", status, msg),
            None => format!("API returned status {}", status),
        });
    }
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
    let base_url = compat_base_url(config)?;
    let model = get_model_name(config);
    let client = build_client(request_timeout(config))?;

    let url = format!("{}/v1/chat/completions", base_url);

    let mut content_parts = Vec::new();
    content_parts.push(serde_json::json!({"type": "text", "text": sys_prompt}));
    content_parts.push(serde_json::json!({"type": "text", "text": user_prompt}));

    for (path, buffer) in file_buffers {
        if buffer.is_empty() {
            tracing::warn!("Empty file buffer for path: {}", path);
            continue;
        }
        let mime = guess_mime(path);
        if !SUPPORTED_VISION_IMAGE_MIMES.contains(&mime.as_str()) {
            tracing::warn!(
                "Skipping {} for vision: media type '{}' is not an accepted image type",
                path,
                mime
            );
            continue;
        }
        let b64 = general_purpose::STANDARD.encode(buffer);
        content_parts.push(serde_json::json!({
            "type": "image_url",
            "image_url": {"url": format!("data:{};base64,{}", mime, b64), "detail": "auto"}
        }));
    }
    if content_parts.len() == 2 {
        return Err("No image could be sent to the vision endpoint".to_string());
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
        .bearer_auth(config.api_key.trim())
        .json(&body)
        .send()
        .await
        .map_err(|e| e.to_string())?;

    let status = resp.status();
    let json: serde_json::Value = resp.json().await.map_err(|e| e.to_string())?;
    if !status.is_success() {
        return Err(match openai_error_message(&json) {
            Some(msg) => format!("API error ({}): {}", status, msg),
            None => format!("API returned status {}", status),
        });
    }
    let result_text = openai_compat_message_text(&json)?;

    let parsed = parse_json_response(result_text)?;
    Ok(data_to_metadata(&parsed))
}

// ---------------------------------------------------------------------------
// Response mapping
// ---------------------------------------------------------------------------

fn data_to_metadata(data: &HashMap<String, serde_json::Value>) -> DocumentMetadata {
    let is_unreadable = data
        .get("is_unreadable_or_error")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);

    let meta = DocumentMetadata {
        company_name: extract_str_field_any(data, &["issuer_entity_short", "company"], ""),
        document_date: extract_str_field_any(data, &["date_YYYY_MM_DD", "date"], ""),
        document_type: extract_str_field_any(data, &["document_nature", "doctype"], ""),
        subject: extract_str_field_any(data, &["specific_subject", "subject"], ""),
        is_unreadable_or_error: is_unreadable,
    };

    tracing::debug!(
        "Mapped metadata: date={}, issuer={}, nature={}, subject={}, unreadable={}",
        meta.document_date,
        meta.company_name,
        meta.document_type,
        meta.subject,
        meta.is_unreadable_or_error
    );

    meta
}

/// Tries multiple keys in order (current schema name first, then legacy aliases).
fn extract_str_field_any(
    data: &HashMap<String, serde_json::Value>,
    keys: &[&str],
    default_value: &str,
) -> String {
    for key in keys {
        if let Some(val) = data
            .get(*key)
            .and_then(|v| v.as_str())
            .filter(|s| !s.trim().is_empty())
        {
            return val.trim().to_string();
        }
    }
    default_value.to_string()
}

// ---------------------------------------------------------------------------
// Connection tests
// ---------------------------------------------------------------------------

/// Probe the configured provider endpoint with the given credentials.
///
/// Uses the live base-URL resolution of the extraction path (so overrides in
/// `AiConfig` are honored) and always contacts the endpoint — including the
/// `custom` provider — so a successful test means the endpoint actually
/// accepted the key.
pub async fn test_connection(config: &AiConfig) -> TestConnectionResult {
    let start = Instant::now();
    let provider = config.provider.as_str();
    let api_key = config.api_key.trim();

    if api_key.is_empty() && provider != "ollama" {
        return TestConnectionResult {
            success: false,
            message: "API key is empty".to_string(),
            latency_ms: 0,
            provider: provider.to_string(),
        };
    }

    let model = get_model_name(config);

    let result = match provider {
        "gemini" => test_gemini_connection(config, api_key, &model, start).await,
        "openai" => {
            test_models_list(config, "openai", &openai_base_url(config), api_key, start).await
        }
        "anthropic" => {
            test_anthropic_connection(api_key, anthropic_base_url(config), start).await
        }
        "ollama" => test_ollama_connection(&ollama_base_url(config), start).await,
        "xai" => test_models_list(config, "xai", &xai_base_url(config), api_key, start).await,
        "custom" => {
            test_custom_connection(config, api_key, &model, start).await
        }
        other => failure_result(
            other,
            format!("Unknown provider: {}", other),
            start,
        ),
    };
    result
}

fn failure_result(provider: &str, message: String, start: Instant) -> TestConnectionResult {
    TestConnectionResult {
        success: false,
        message,
        latency_ms: start.elapsed().as_millis(),
        provider: provider.to_string(),
    }
}

fn ok_result(provider: &str, message: String, start: Instant) -> TestConnectionResult {
    TestConnectionResult {
        success: true,
        message,
        latency_ms: start.elapsed().as_millis(),
        provider: provider.to_string(),
    }
}

async fn test_gemini_connection(
    config: &AiConfig,
    api_key: &str,
    model: &str,
    start: Instant,
) -> TestConnectionResult {
    let model_name = if model.trim().is_empty() {
        GEMINI_DEFAULT_MODEL
    } else {
        model
    };
    let base_url = gemini_base_url(config);
    let client = match build_client(10) {
        Ok(c) => c,
        Err(e) => return failure_result("gemini", e, start),
    };

    let url = format!("{}/v1beta/models/{}:generateContent", base_url, model_name);

    let body = serde_json::json!({
        "contents": [{"role": "user", "parts": [{"text": "OK"}]}],
        "generationConfig": {"maxOutputTokens": 1}
    });

    match client.post(&url).header("x-goog-api-key", api_key).json(&body).send().await {
        Ok(resp) => {
            let ms = start.elapsed().as_millis();
            if resp.status().is_success() {
                ok_result("gemini", format!("Connected ({}ms)", ms), start)
            } else {
                failure_result(
                    "gemini",
                    format!("Gemini returned status {}", resp.status()),
                    start,
                )
            }
        }
        Err(e) => failure_result("gemini", format!("Failed: {}", e), start),
    }
}

/// GET `{base}/v1/models` with a bearer token (OpenAI-compatible probe).
async fn test_models_list(
    config: &AiConfig,
    provider: &str,
    base_url: &str,
    api_key: &str,
    start: Instant,
) -> TestConnectionResult {
    let client = match build_client(10) {
        Ok(c) => c,
        Err(e) => return failure_result(provider, e, start),
    };

    let url = models_url(base_url);

    match client.get(&url).bearer_auth(api_key).send().await {
        Ok(resp) => {
            let ms = start.elapsed().as_millis();
            if resp.status().is_success() {
                // For OpenAI-compatible providers, verify the requested model
                // is actually available when the listing includes model ids.
                match resp.json::<serde_json::Value>().await {
                    Ok(json) => {
                        let ids: Vec<&str> = json
                            .get("data")
                            .and_then(|d| d.as_array())
                            .map(|arr| {
                                arr.iter()
                                    .filter_map(|m| m.get("id").and_then(|i| i.as_str()))
                                    .collect()
                            })
                            .unwrap_or_default();
                        let model = get_model_name(config);
                        if !model.is_empty() && !ids.is_empty() && !ids.contains(&model.as_str())
                        {
                            return failure_result(
                                provider,
                                format!(
                                    "Connected but model '{}' is not available at {} ({} models listed)",
                                    model, base_url, ids.len()
                                ),
                                start,
                            );
                        }
                        let count = if ids.is_empty() {
                            json.get("models")
                                .and_then(|v| v.as_array())
                                .map(|a| a.len())
                                .unwrap_or(0)
                        } else {
                            ids.len()
                        };
                        ok_result(provider, format!("Connected ({} models, {}ms)", count, ms), start)
                    }
                    Err(_) => ok_result(provider, format!("Connected ({}ms)", ms), start),
                }
            } else {
                failure_result(
                    provider,
                    format!("{} returned status {}", provider, resp.status()),
                    start,
                )
            }
        }
        Err(e) => failure_result(provider, format!("Failed: {}", e), start),
    }
}

async fn test_anthropic_connection(
    api_key: &str,
    base_url: String,
    start: Instant,
) -> TestConnectionResult {
    let client = match build_client(10) {
        Ok(c) => c,
        Err(e) => return failure_result("anthropic", e, start),
    };

    let url = format!("{}/v1/models", base_url);

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
                ok_result("anthropic", format!("Connected ({}ms)", ms), start)
            } else {
                failure_result(
                    "anthropic",
                    format!("Anthropic returned status {}", resp.status()),
                    start,
                )
            }
        }
        Err(e) => failure_result("anthropic", format!("Failed: {}", e), start),
    }
}

async fn test_ollama_connection(base_url: &str, start: Instant) -> TestConnectionResult {
    let client = match build_client(5) {
        Ok(c) => c,
        Err(e) => return failure_result("ollama", e, start),
    };

    let url = format!("{}/api/tags", base_url);

    match client.get(&url).send().await {
        Ok(resp) => {
            let ms = start.elapsed().as_millis();
            if resp.status().is_success() {
                if let Ok(json) = resp.json::<serde_json::Value>().await {
                    let count = json
                        .get("models")
                        .and_then(|v| v.as_array())
                        .map(|a| a.len())
                        .unwrap_or(0);
                    ok_result(
                        "ollama",
                        format!("Connected ({} models, {}ms)", count, ms),
                        start,
                    )
                } else {
                    ok_result("ollama", format!("Connected ({}ms)", ms), start)
                }
            } else {
                failure_result(
                    "ollama",
                    format!("Ollama returned status {}", resp.status()),
                    start,
                )
            }
        }
        Err(e) => failure_result("ollama", format!("Failed: {}", e), start),
    }
}

/// Custom provider test: performs a live request against the configured base
/// URL so success means the endpoint is reachable and accepted the key.
async fn test_custom_connection(
    config: &AiConfig,
    api_key: &str,
    model: &str,
    start: Instant,
) -> TestConnectionResult {
    let base_url = match compat_base_url(config) {
        Ok(url) => url,
        Err(e) => return failure_result("custom", e, start),
    };
    if model.trim().is_empty() {
        return failure_result(
            "custom",
            "Model name is required for custom providers".to_string(),
            start,
        );
    }
    let client = match build_client(15) {
        Ok(c) => c,
        Err(e) => return failure_result("custom", e, start),
    };

    let url = chat_url(&base_url);
    let body = serde_json::json!({
        "model": model,
        "messages": [{"role": "user", "content": "OK"}],
        "max_tokens": 1
    });

    match client.post(&url).bearer_auth(api_key).json(&body).send().await {
        Ok(resp) => {
            let status = resp.status();
            if status.is_success() {
                ok_result("custom", format!("Connected ({}ms)", start.elapsed().as_millis()), start)
            } else {
                let detail = resp
                    .text()
                    .await
                    .unwrap_or_default()
                    .chars()
                    .take(200)
                    .collect::<String>();
                failure_result(
                    "custom",
                    format!("Endpoint returned {} — {}", status, detail),
                    start,
                )
            }
        }
        Err(e) => failure_result("custom", format!("Failed: {}", e), start),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn json_response_survives_code_fences_and_surrounding_prose() {
        let fenced = "```json\n{\"issuer_entity_short\":\"Acme\"}\n```";
        assert_eq!(
            parse_json_response(fenced).unwrap()["issuer_entity_short"],
            serde_json::json!("Acme")
        );

        let chatty = "Sure! Here is the JSON:\n{\"issuer_entity_short\":\"Acme\"}\nHope that helps.";
        assert_eq!(
            parse_json_response(chatty).unwrap()["issuer_entity_short"],
            serde_json::json!("Acme")
        );
    }

    #[test]
    fn json_response_falls_back_to_regex_when_not_valid_json() {
        // Missing closing brace: the brace scanner cannot help, the regex can.
        let broken = "{\"issuer_entity_short\": \"Acme\", \"document_nature\": \"Invoice\"";
        let parsed = parse_json_response(broken).unwrap();
        assert_eq!(parsed["issuer_entity_short"], serde_json::json!("Acme"));
        assert_eq!(parsed["document_nature"], serde_json::json!("Invoice"));
    }

    #[test]
    fn truncate_str_chars_is_char_safe() {
        let arabic = "فاتورة";
        let out = truncate_str_chars(arabic, 3);
        assert_eq!(out.chars().count(), 3);
        // Byte slicing would have panicked here; char slicing must not.
        assert_eq!(truncate_str_chars("abc", 99), "abc");
    }

    #[test]
    fn data_to_metadata_accepts_current_schema_and_legacy_aliases() {
        let mut current = HashMap::new();
        current.insert(
            "issuer_entity_short".to_string(),
            serde_json::json!("Acme"),
        );
        current.insert("date_YYYY_MM_DD".to_string(), serde_json::json!("2024-01-05"));
        current.insert("document_nature".to_string(), serde_json::json!("Invoice"));
        current.insert("specific_subject".to_string(), serde_json::json!("Q3 fee"));
        let meta = data_to_metadata(&current);
        assert_eq!(meta.company_name, "Acme");
        assert_eq!(meta.document_date, "2024-01-05");
        assert_eq!(meta.document_type, "Invoice");
        assert_eq!(meta.subject, "Q3 fee");
        assert!(!meta.is_empty());

        let mut legacy = HashMap::new();
        legacy.insert("company".to_string(), serde_json::json!("  Acme  "));
        legacy.insert("date".to_string(), serde_json::json!("2024-01-05"));
        legacy.insert("doctype".to_string(), serde_json::json!("Receipt"));
        legacy.insert("subject".to_string(), serde_json::json!("Lunch"));
        let meta = data_to_metadata(&legacy);
        assert_eq!(meta.company_name, "Acme", "values must be trimmed");
        assert_eq!(meta.document_type, "Receipt");
        assert_eq!(meta.subject, "Lunch");
    }

    #[test]
    fn whitespace_only_fields_are_treated_as_missing() {
        let mut data = HashMap::new();
        data.insert("issuer_entity_short".to_string(), serde_json::json!("   "));
        data.insert("document_nature".to_string(), serde_json::json!("Invoice"));
        let meta = data_to_metadata(&data);
        assert_eq!(meta.company_name, "");
        assert_eq!(meta.document_type, "Invoice");
        assert!(!meta.is_empty());
    }

    #[test]
    fn openai_compat_message_text_handles_part_and_reasoning_shapes() {
        let parts = serde_json::json!({
            "choices": [{ "message": { "content": [
                { "type": "text", "text": "{\"a\":1}" }
            ] } }]
        });
        assert_eq!(openai_compat_message_text(&parts).unwrap(), "{\"a\":1}");

        let reasoning = serde_json::json!({
            "choices": [{ "message": { "content": null, "reasoning_content": "{\"b\":2}" } }]
        });
        assert_eq!(openai_compat_message_text(&reasoning).unwrap(), "{\"b\":2}");

        let empty = serde_json::json!({ "choices": [] });
        assert!(openai_compat_message_text(&empty).is_err());

        let no_choices = serde_json::json!({});
        assert!(openai_compat_message_text(&no_choices).is_err());
    }

    #[test]
    fn gemini_response_reports_api_errors_and_missing_candidates() {
        let api_error = serde_json::json!({
            "error": { "code": 400, "message": "API key not valid" }
        })
        .to_string();
        let err = parse_gemini_response(&api_error).unwrap_err();
        assert!(err.contains("400"), "got: {}", err);
        assert!(err.contains("API key not valid"), "got: {}", err);

        let no_candidates = serde_json::json!({ "promptFeedback": {} }).to_string();
        assert!(parse_gemini_response(&no_candidates).is_err());

        let ok = serde_json::json!({
            "candidates": [{ "content": { "parts": [{ "text": "{\"x\":1}" }] } }]
        })
        .to_string();
        assert_eq!(parse_gemini_response(&ok).unwrap(), "{\"x\":1}");
    }

    #[test]
    fn get_all_languages_deduplicates_case_insensitively_and_keeps_order() {
        let langs = get_all_languages(
            "English",
            &["french".to_string(), "English".to_string(), "French".to_string()],
        );
        // The primary language always comes first; later case-insensitive
        // duplicates of an already accepted language are dropped.
        assert_eq!(langs, vec!["English", "french"]);
    }
    #[test]
    fn model_name_falls_back_per_provider() {
        let mut config = AiConfig::default();
        assert_eq!(get_model_name(&config), GEMINI_DEFAULT_MODEL);

        config.provider = "openai".to_string();
        assert_eq!(get_model_name(&config), "gpt-4o-mini");

        config.provider = "anthropic".to_string();
        assert_eq!(get_model_name(&config), "claude-3-5-haiku-latest");

        config.provider = "ollama".to_string();
        assert_eq!(get_model_name(&config), "llama3.2");

        config.provider = "custom".to_string();
        config.custom_model = "my-model".to_string();
        assert_eq!(get_model_name(&config), "my-model");
    }

    #[test]
    fn base_url_normalisation_strips_trailing_slash_and_v1() {
        assert_eq!(normalize_base("http://localhost:11434/v1/"), "http://localhost:11434");
        assert_eq!(normalize_base("https://api.openai.com/"), "https://api.openai.com");
        assert_eq!(normalize_base("https://host/V1"), "https://host");
        assert_eq!(chat_url("https://host"), "https://host/v1/chat/completions");
    }

    #[test]
    fn compat_base_url_requires_a_custom_endpoint() {
        let mut config = AiConfig::default();
        config.provider = "custom".to_string();
        assert!(compat_base_url(&config).is_err());

        config.custom_base_url = "http://localhost:8000/v1".to_string();
        assert_eq!(compat_base_url(&config).unwrap(), "http://localhost:8000");
    }

    #[test]
    fn guess_mime_maps_documents_and_images() {
        assert_eq!(guess_mime("a.PNG"), "image/png");
        assert_eq!(guess_mime("b.pdf"), "application/pdf");
        assert_eq!(guess_mime("noext"), "application/octet-stream");
        // PDF is intentionally not an accepted inline image type.
        assert!(!SUPPORTED_VISION_IMAGE_MIMES.contains(&"application/pdf"));
    }

    #[test]
    fn system_prompt_falls_back_when_blank() {
        let mut config = AiConfig::default();
        config.system_prompt = "   ".to_string();
        assert_eq!(config.effective_system_prompt(), default_system_prompt());

        config.system_prompt = "custom".to_string();
        assert_eq!(config.effective_system_prompt(), "custom");
    }

    #[test]
    fn non_english_languages_get_an_output_language_instruction() {
        let english = build_system_prompt("English", "BASE");
        assert_eq!(english, "BASE");
        let arabic = build_system_prompt("Arabic", "BASE");
        assert!(arabic.starts_with("BASE"));
        assert!(arabic.contains("Arabic"));
    }
}
