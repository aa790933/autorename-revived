use serde::{Serialize, Deserialize};
use crate::config::NamingConfig;
use chrono::NaiveDate;
use regex::Regex;
use std::collections::HashMap;
use std::fs;
use std::path::Path;

/// Truncate a string to max_len characters (char-boundary safe).
fn truncate_str(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        return s.to_string();
    }
    let mut end = max_len;
    while end > 0 && !s.is_char_boundary(end) {
        end -= 1;
    }
    s[..end].to_string()
}

/// Simple random helpers (no external crate needed).
fn rand_u32() -> u32 {
    use std::collections::hash_map::RandomState;
    use std::hash::{BuildHasher, Hasher};
    let s = RandomState::new();
    let mut h = s.build_hasher();
    h.write_u64(unsafe { std::ptr::read(&h as *const _ as *const u64) });
    h.finish() as u32
}

fn rand_u16() -> u16 {
    rand_u32() as u16
}

fn rand_u48() -> u64 {
    rand_u32() as u64 | ((rand_u32() as u64) << 32)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileResult {
    pub file: String,
    pub status: String,
    pub new_name: Option<String>,
    pub new_path: Option<String>,
    pub error: Option<String>,
    pub warnings: Vec<String>,
    pub company: Option<String>,
    pub date: Option<String>,
    pub doc_type: Option<String>,
    pub provider: Option<String>,
    pub model: Option<String>,
    #[serde(default)]
    pub suggestion_names: Vec<String>,
    #[serde(default)]
    pub suggestion_languages: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchResult {
    pub success: bool,
    pub total: usize,
    pub completed: usize,
    pub skipped: usize,
    pub failed: usize,
    pub files: Vec<FileResult>,
    pub dry_run: bool,
    pub batch_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UndoFileResult {
    pub old_path: String,
    pub new_path: String,
    pub status: String,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UndoResult {
    pub success: bool,
    pub restored: usize,
    pub failed: usize,
    pub files: Vec<UndoFileResult>,
    pub batch_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UndoBatch {
    pub batch_id: String,
    pub timestamp: String,
    pub source: String,
    pub undone: bool,
    pub files: Vec<UndoEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UndoEntry {
    pub old_path: String,
    pub new_path: String,
    pub timestamp: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UndoHistory {
    pub version: u32,
    pub batches: Vec<UndoBatch>,
}

impl UndoHistory {
    pub fn new() -> Self {
        Self {
            version: 2,
            batches: Vec::new(),
        }
    }

    pub fn load(path: &Path) -> Result<Self, String> {
        if !path.exists() {
            return Ok(Self::new());
        }
        let content = fs::read_to_string(path).map_err(|e| e.to_string())?;
        let data: serde_json::Value = serde_json::from_str(&content).map_err(|e| e.to_string())?;

        if let Ok(history) = serde_json::from_value::<UndoHistory>(data.clone()) {
            return Ok(history);
        }

        if let Some(arr) = data.as_array() {
            let files: Vec<UndoEntry> = arr
                .iter()
                .filter_map(|v| serde_json::from_value(v.clone()).ok())
                .collect();
            return Ok(UndoHistory {
                version: 2,
                batches: vec![UndoBatch {
                    batch_id: "migrated-v1".to_string(),
                    timestamp: String::new(),
                    source: "cli".to_string(),
                    undone: false,
                    files,
                }],
            });
        }

        if let Some(obj) = data.as_object() {
            if obj.get("version").and_then(|v| v.as_u64()) == Some(2) {
                if let Ok(history) = serde_json::from_value::<UndoHistory>(data) {
                    return Ok(history);
                }
            }
        }

        Ok(Self::new())
    }

    pub fn save(&self, path: &Path) -> Result<(), String> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        let content = serde_json::to_string_pretty(self).map_err(|e| e.to_string())?;
        fs::write(path, content).map_err(|e| e.to_string())?;
        Ok(())
    }
}

pub fn sanitize_filename(name: &str, max_length: usize) -> String {
    if name.is_empty() {
        return String::from("_");
    }

    let invalid_fs_chars = Regex::new(r#"[\x00-\x1f\\/:*?\"<>|]"#).unwrap();
    let unicode_control = Regex::new(r#"[\u200b-\u200f\u2028-\u202f\u2060-\u2064\ufeff\u00ad]"#).unwrap();
    let gibberish_hex = Regex::new(r#"\b[0-9]*[a-f][0-9a-f]{11,}\b"#).unwrap();
    let multi_sep = Regex::new(r#"[_ \-]{2,}"#).unwrap();

    let reserved_names = [
        "con", "prn", "aux", "nul", "com1", "com2", "com3", "com4", "com5", "com6",
        "com7", "com8", "com9", "lpt1", "lpt2", "lpt3", "lpt4", "lpt5", "lpt6",
        "lpt7", "lpt8", "lpt9",
    ];

    let cleaned = unicode_control.replace_all(name, "");
    let cleaned = invalid_fs_chars.replace_all(&cleaned, "_");
    let cleaned = gibberish_hex.replace_all(&cleaned, "_");
    let cleaned = multi_sep.replace_all(&cleaned, "_");
    let cleaned = cleaned.trim_matches(' ').trim_matches('.');

    let parts: Vec<&str> = cleaned.split('_').filter(|s| !s.is_empty()).collect();
    let cleaned = parts.join("_");
    let cleaned = if cleaned.is_empty() {
        String::from("_")
    } else {
        cleaned
    };

    let cleaned = if cleaned.starts_with('.') {
        format!("_{}", &cleaned[1..])
    } else {
        cleaned
    };

    let cleaned = if cleaned.len() > max_length {
        // Use char-boundary-aware truncation to avoid panicking on multi-byte UTF-8
        let mut end = max_length;
        while end > 0 && !cleaned.is_char_boundary(end) {
            end -= 1;
        }
        cleaned[..end].to_string()
    } else {
        cleaned
    };

    let stem = cleaned.rsplit('.').nth(1).unwrap_or(&cleaned);
    let stem_lower = stem.to_lowercase();
    let cleaned = if reserved_names.contains(&stem_lower.as_str()) {
        format!("_{}", cleaned)
    } else {
        cleaned
    };

    cleaned
}

pub fn resolve_safe_path(directory: &str, filename: &str) -> Result<String, String> {
    let dir_path = Path::new(directory).canonicalize().unwrap_or_else(|_| Path::new(directory).to_path_buf());
    let safe_filename = sanitize_filename(filename, 128);
    let safe_name = Path::new(&safe_filename);
    let resolved = dir_path.join(safe_name);

    let resolved_str = resolved.to_string_lossy().to_string();

    let dir_str = dir_path.to_string_lossy().to_string();
    if !resolved_str.starts_with(&dir_str) {
        return Err(format!("Path traversal blocked: {}", filename));
    }

    if resolved_str.len() > 260 {
        return Err(format!(
            "Full path exceeds 260 characters: {} chars",
            resolved_str.len()
        ));
    }

    Ok(resolved_str)
}

pub fn parse_document_date(date_str: &str) -> Option<String> {
    if date_str.is_empty() {
        return None;
    }

    // Accept YYYY-MM-DD format directly (new schema standard)
    if let Ok(parsed) = chrono::NaiveDate::parse_from_str(date_str, r#"%Y-%m-%d"#) {
        return Some(parsed.format(r#"%Y-%m-%d"#).to_string());
    }

    // Accept YYYYMMDD format (legacy)
    let compact = date_str.replace('-', "").replace('/', "").replace(' ', "");
    if compact.len() == 8 && compact.chars().all(|c| c.is_ascii_digit()) {
        let y = &compact[0..4];
        let m = &compact[4..6];
        let d = &compact[6..8];
        return Some(format!("{}-{}-{}", y, m, d));
    }

    // Accept MM/DD/YYYY format
    if let Ok(parsed) = chrono::NaiveDate::parse_from_str(date_str, r#"%m/%d/%Y"#) {
        return Some(parsed.format(r#"%Y-%m-%d"#).to_string());
    }

    // Accept DD/MM/YYYY format
    if let Ok(parsed) = chrono::NaiveDate::parse_from_str(date_str, r#"%d/%m/%Y"#) {
        return Some(parsed.format(r#"%Y-%m-%d"#).to_string());
    }

    None
}

/// Validates extracted metadata and returns warnings/errors.
/// Returns (is_error, warnings) where is_error means the metadata is unusable.
pub fn validate_metadata(
    company: &str,
    doctype: &str,
    date_str: &str,
    subject: &str,
    is_unreadable: bool,
) -> (bool, Vec<String>) {
    let mut warnings = Vec::new();
    let mut is_error = false;

    if is_unreadable {
        warnings.push("Document was flagged as unreadable by AI".to_string());
        is_error = true;
    }

    if company.is_empty() && doctype.is_empty() && date_str.is_empty() {
        warnings.push("AI returned no usable metadata — using defaults".to_string());
        is_error = true;
    }

    // Validate date format: must be YYYY-MM-DD or empty
    if !date_str.is_empty() {
        let date_re = Regex::new(r#"^\d{4}-\d{2}-\d{2}$"#).unwrap();
        if !date_re.is_match(date_str) {
            warnings.push(format!("Invalid date format '{}' — expected YYYY-MM-DD", date_str));
        }
    }

    // Validate subject length (max 30 chars)
    if subject.len() > 30 {
        warnings.push(format!("Subject truncated from {} to 30 chars", subject.len()));
    }

    (is_error, warnings)
}

pub fn harmonize_company_name(
    name: &str,
    harmonized_companies: &[HashMap<String, serde_json::Value>],
) -> String {
    if name.is_empty() || harmonized_companies.is_empty() {
        return name.to_string();
    }

    let mut lookup: HashMap<String, String> = HashMap::new();
    for entry in harmonized_companies {
        if let Some(company_name) = entry.get("name").and_then(|v| v.as_str()) {
            let normalized = company_name.to_lowercase();
            lookup.insert(normalized, company_name.to_string());
            if let Some(variations) = entry.get("variations").and_then(|v| v.as_array()) {
                for var in variations {
                    if let Some(v_str) = var.as_str() {
                        lookup.insert(v_str.to_lowercase(), company_name.to_string());
                    }
                }
            }
        }
    }

    let name_lower = name.to_lowercase();
    if let Some(harmonized) = lookup.get(&name_lower) {
        return harmonized.clone();
    }

    name.to_string()
}

pub fn generate_filename(
    company: &str,
    doctype: &str,
    date_str: &str,
    subject: &str,
    config: &NamingConfig,
    original_filename: &str,
    is_unreadable: bool,
) -> String {
    let suffix = Path::new(original_filename)
        .extension()
        .map(|e| format!(".{}", e.to_string_lossy()))
        .unwrap_or_default();

    // If the document is unreadable, return a predefined error filename
    if is_unreadable {
        let uuid = format!(
            "{:08x}-{:04x}-{:04x}-{:04x}-{:012x}",
            rand_u32(), rand_u16(), rand_u16(), rand_u16(), rand_u48()
        );
        return format!("Error_Unreadable_File_{}{}", &uuid[..8], suffix);
    }

    let has_content = !company.is_empty() || !doctype.is_empty() || !date_str.is_empty();
    let template = if has_content {
        &config.template
    } else {
        &config.fallback
    };

    let date_formatted = if date_str.is_empty() {
        String::new()
    } else if let Ok(parsed) = NaiveDate::parse_from_str(date_str, r#"%Y-%m-%d"#) {
        parsed.format(r#"%Y-%m-%d"#).to_string()
    } else if date_str.len() == 8 && date_str.chars().all(|c| c.is_ascii_digit()) {
        let y = &date_str[0..4];
        let m = &date_str[4..6];
        let d = &date_str[6..8];
        format!("{}-{}-{}", y, m, d)
    } else {
        String::new()
    };

    let clean_company = sanitize_filename(company, 32);
    let clean_doctype = sanitize_filename(doctype, 24);
    // Truncate subject to 30 chars max (char-boundary safe)
    let subject_truncated = truncate_str(subject, 30);
    let clean_subject = sanitize_filename(&subject_truncated, 32);

    let original_stem = Path::new(original_filename)
        .file_stem()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| "file".to_string());

    let seq_width = config.sequence_zerofill as usize;

    let fields: HashMap<String, String> = [
        ("date".to_string(), date_formatted),
        ("company".to_string(), clean_company.clone()),
        ("doctype".to_string(), clean_doctype.clone()),
        ("subject".to_string(), clean_subject.clone()),
        ("original".to_string(), original_stem.clone()),
        ("sequence".to_string(), format!("_{:0width$}", 1, width = seq_width)),
    ]
    .into_iter()
    .collect();

    let mut result = template.to_string();
    for (key, val) in &fields {
        result = result.replace(&format!("{{{}}}", key), val);
    }

    // If template had no matching placeholders or result is empty, use fallback
    if result == template.as_str() || result.trim().is_empty() {
        let fallback_fields: HashMap<String, String> = [
            ("date".to_string(), fields["date"].clone()),
            ("company".to_string(), if clean_company.is_empty() { "Unknown".to_string() } else { clean_company }),
            ("doctype".to_string(), if clean_doctype.is_empty() { "Doc".to_string() } else { clean_doctype }),
            ("subject".to_string(), if clean_subject.is_empty() { "Unknown".to_string() } else { clean_subject }),
            ("original".to_string(), original_stem),
            ("sequence".to_string(), format!("_{:0width$}", 1, width = seq_width)),
        ]
        .into_iter()
        .collect();
        result = config.fallback.to_string();
        for (key, val) in &fallback_fields {
            result = result.replace(&format!("{{{}}}", key), val);
        }
    }

    // Clean up double underscores/separators in the final result
    let multi_sep = Regex::new(r#"_{2,}"#).unwrap();
    let result = multi_sep.replace_all(&result, "_");
    let result = result.trim_matches('_').trim_matches(' ').trim_matches('.').to_string();
    let result = if result.is_empty() { "Unknown".to_string() } else { result };

    let avail = config.max_length as usize - suffix.len();
    if avail < 4 {
        let mut end = avail.min(result.len());
        while end > 0 && !result.is_char_boundary(end) {
            end -= 1;
        }
        format!("{}{}", &result[..end], suffix)
    } else {
        format!("{}{}", result, suffix)
    }
}

pub fn ensure_unique_filename(directory: &str, filename: &str, zerofill: u32) -> String {
    let path = Path::new(directory).join(filename);
    if !path.exists() {
        return filename.to_string();
    }

    let path_obj = Path::new(filename);
    let stem = path_obj
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or(filename);
    let ext = path_obj
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| format!(".{}", e))
        .unwrap_or_default();

    let seq_width = zerofill as usize;
    let mut counter = 2u32;
    loop {
        let seq = format!("_{:0width$}", counter, width = seq_width);
        let new_name = if stem.ends_with("_01") {
            let base = stem.trim_end_matches("_01");
            format!("{}{}{}", base, seq, ext)
        } else {
            format!("{}{}{}", stem, seq, ext)
        };
        let new_path = Path::new(directory).join(&new_name);
        if !new_path.exists() {
            return new_name;
        }
        counter += 1;
        if counter > 9999 {
            // Append a timestamp-based suffix as a last resort to guarantee uniqueness
            let ts = chrono::Local::now().format(r#"%Y%m%dT%H%M%S"#);
            let fallback = format!("{}_{ts}{}", stem, ext);
            return fallback;
        }
    }
}

pub fn apply_rename(
    old_path: &str,
    new_path: &str,
    dry_run: bool,
) -> Result<(), String> {
    if dry_run {
        return Ok(());
    }
    fs::rename(old_path, new_path).map_err(|e| e.to_string())?;
    Ok(())
}

pub fn save_rename_to_history(
    old_path: &str,
    new_path: &str,
    history_path: &Path,
    batch_id: &str,
) -> Result<(), String> {
    let mut history = UndoHistory::load(history_path)?;
    let timestamp = chrono::Local::now().to_rfc3339();

    let target_batch = history.batches.iter_mut().find(|b| {
        b.batch_id == batch_id && !b.undone
    });

    if let Some(batch) = target_batch {
        batch.files.push(UndoEntry {
            old_path: Path::new(old_path).to_string_lossy().to_string(),
            new_path: Path::new(new_path).to_string_lossy().to_string(),
            timestamp: timestamp.clone(),
        });
    } else {
        let batch_id = if batch_id.is_empty() {
            format!("gui-{}", chrono::Local::now().format(r#"%Y%m%dT%H%M%S"#))
        } else {
            batch_id.to_string()
        };
        history.batches.push(UndoBatch {
            batch_id,
            timestamp: timestamp.clone(),
            source: "gui".to_string(),
            undone: false,
            files: vec![UndoEntry {
                old_path: Path::new(old_path).to_string_lossy().to_string(),
                new_path: Path::new(new_path).to_string_lossy().to_string(),
                timestamp: timestamp.clone(),
            }],
        });
    }

    while history.batches.len() > 100 {
        history.batches.remove(0);
    }

    history.save(history_path)?;
    Ok(())
}

pub fn undo_last_rename(
    history_path: &Path,
    batch_id: &str,
) -> Result<UndoResult, String> {
    let mut history = UndoHistory::load(history_path)?;

    if history.batches.is_empty() {
        return Ok(UndoResult {
            success: false,
            restored: 0,
            failed: 0,
            files: Vec::new(),
            batch_id: None,
        });
    }

    let Some((batch_idx, file_idx)) = (if !batch_id.is_empty() {
        history.batches.iter().rposition(|b| b.batch_id == batch_id && !b.undone).and_then(|bi| {
            let fi = history.batches[bi].files.len().saturating_sub(1);
            if fi < history.batches[bi].files.len() {
                Some((bi, fi))
            } else {
                None
            }
        })
    } else {
        history.batches.iter().rposition(|b| !b.undone && !b.files.is_empty()).and_then(|bi| {
            let fi = history.batches[bi].files.len().saturating_sub(1);
            Some((bi, fi))
        })
    }) else {
        return Ok(UndoResult {
            success: false,
            restored: 0,
            failed: 0,
            files: Vec::new(),
            batch_id: None,
        });
    };

    let entry = history.batches[batch_idx].files[file_idx].clone();
    let result_batch_id = history.batches[batch_idx].batch_id.clone();

    if !Path::new(&entry.new_path).exists() {
        return Ok(UndoResult {
            success: false,
            restored: 0,
            failed: 1,
            files: vec![UndoFileResult {
                old_path: entry.old_path.clone(),
                new_path: entry.new_path.clone(),
                status: "failed".to_string(),
                error: Some("Renamed file no longer exists".to_string()),
            }],
            batch_id: Some(result_batch_id),
        });
    }

    match fs::rename(&entry.new_path, &entry.old_path) {
        Ok(_) => {
            history.batches[batch_idx].files.remove(file_idx);
            if history.batches[batch_idx].files.is_empty() {
                history.batches[batch_idx].undone = true;
            }
            history.save(history_path)?;
            Ok(UndoResult {
                success: true,
                restored: 1,
                failed: 0,
                files: vec![UndoFileResult {
                    old_path: entry.old_path,
                    new_path: entry.new_path,
                    status: "restored".to_string(),
                    error: None,
                }],
                batch_id: Some(result_batch_id),
            })
        }
        Err(e) => Ok(UndoResult {
            success: false,
            restored: 0,
            failed: 1,
            files: vec![UndoFileResult {
                old_path: entry.old_path,
                new_path: entry.new_path,
                status: "failed".to_string(),
                error: Some(e.to_string()),
            }],
            batch_id: Some(result_batch_id),
        }),
    }
}


