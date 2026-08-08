use serde::{Serialize, Deserialize};
use crate::config::NamingConfig;
use chrono::NaiveDate;
use regex::Regex;
use std::collections::HashMap;
use std::fs;
use std::path::Path;

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
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchResult {
    pub success: bool,
    pub total: usize,
    pub renamed: usize,
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
    let gibberish_hex = Regex::new(r#"\b[0-9a-f]{8,}\b"#).unwrap();
    let gibberish_long_num = Regex::new(r#"\b\d{6,}\b"#).unwrap();
    let _leading_trailing = Regex::new(r#"^[\W_]+|[\W_]+$"#).unwrap();
    let _multi_sep = Regex::new(r#"[_ \-]{2,}"#).unwrap();

    let reserved_names = [
        "con", "prn", "aux", "nul", "com1", "com2", "com3", "com4", "com5", "com6",
        "com7", "com8", "com9", "lpt1", "lpt2", "lpt3", "lpt4", "lpt5", "lpt6",
        "lpt7", "lpt8", "lpt9",
    ];

    let cleaned = unicode_control.replace_all(name, "");
    let cleaned = invalid_fs_chars.replace_all(&cleaned, "_");
    let cleaned = gibberish_hex.replace_all(&cleaned, "_");
    let cleaned = gibberish_long_num.replace_all(&cleaned, "_");
    let cleaned = cleaned.trim_matches(' ').trim_matches('.');

    let parts: Vec<&str> = cleaned.split('_').filter(|s| !s.is_empty()).collect();
    let mut cleaned = parts.join("_");
    if cleaned.is_empty() {
        cleaned = String::from("_");
    }

    if cleaned.starts_with('.') {
        cleaned = format!("_{}", &cleaned[1..]);
    }

    if cleaned.len() > max_length {
        cleaned = cleaned[..max_length].to_string();
    }

    let stem = cleaned.rsplit('.').nth(1).unwrap_or(&cleaned);
    let stem_lower = stem.to_lowercase();
    if reserved_names.contains(&stem_lower.as_str()) {
        cleaned = format!("_{}", cleaned);
    }

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

    let cleaned = date_str.replace('-', "").replace('/', "").replace(' ', "");
    if cleaned.len() == 8 && cleaned.chars().all(|c| c.is_ascii_digit()) {
        return Some(cleaned);
    }

    if let Ok(parsed) = chrono::NaiveDate::parse_from_str(date_str, r#"%Y-%m-%d"#) {
        return Some(parsed.format(r#"%Y%m%d"#).to_string());
    }

    if let Ok(parsed) = chrono::NaiveDate::parse_from_str(date_str, r#"%m/%d/%Y"#) {
        return Some(parsed.format(r#"%Y%m%d"#).to_string());
    }

    if let Ok(parsed) = chrono::NaiveDate::parse_from_str(date_str, r#"%d/%m/%Y"#) {
        return Some(parsed.format(r#"%Y%m%d"#).to_string());
    }

    None
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
    config: &NamingConfig,
    original_filename: &str,
) -> String {
    let suffix = Path::new(original_filename)
        .extension()
        .map(|e| format!(".{}", e.to_string_lossy()))
        .unwrap_or_else(|| String::from(".pdf"));

    let has_content = !company.is_empty() || !doctype.is_empty() || !date_str.is_empty() || !original_filename.is_empty();
    let template = if has_content {
        &config.template
    } else {
        &config.fallback
    };

    let date_formatted = if date_str.len() == 8 && date_str.chars().all(|c| c.is_ascii_digit()) {
        date_str.to_string()
    } else if let Ok(parsed) = NaiveDate::parse_from_str(date_str, r#"%Y-%m-%d"#) {
        parsed.format(&config.date_format).to_string()
    } else {
        "00000000".to_string()
    };

    let clean_company = sanitize_filename(company, 48);
    let clean_doctype = sanitize_filename(doctype, 48);

    let fields: HashMap<String, String> = [
        ("date".to_string(), date_formatted.clone()),
        ("company".to_string(), if clean_company.is_empty() { "Unknown".to_string() } else { clean_company.clone() }),
        ("doctype".to_string(), if clean_doctype.is_empty() { "Doc".to_string() } else { clean_doctype.clone() }),
        ("category".to_string(), if clean_company.is_empty() { "Unknown".to_string() } else { clean_company.clone() }),
        ("subject".to_string(), if company.is_empty() && doctype.is_empty() { "Unknown".to_string() } else { company.to_string() }),
        ("original".to_string(), Path::new(original_filename).file_stem().map(|s| s.to_string_lossy().to_string()).unwrap_or_else(|| "file".to_string())),
        ("sequence".to_string(), "01".to_string()),
    ]
    .into_iter()
    .collect();

    let mut result = template.to_string();
    for (key, val) in &fields {
        result = result.replace(&format!("{{{}}}", key), val);
    }

    if result == template.as_str() || result.is_empty() {
        let fallback_fields: HashMap<String, String> = [
            ("date".to_string(), date_formatted.clone()),
            ("company".to_string(), if clean_company.is_empty() { "Unknown".to_string() } else { clean_company.clone() }),
            ("doctype".to_string(), if clean_doctype.is_empty() { "Doc".to_string() } else { clean_doctype.clone() }),
        ]
        .into_iter()
        .collect();
        result = config.fallback.to_string();
        for (key, val) in &fallback_fields {
            result = result.replace(&format!("{{{}}}", key), val);
        }
    }

    let avail = config.max_length as usize - suffix.len();
    if avail < 4 {
        format!("{}{}", &result[..avail.min(result.len())], suffix)
    } else {
        format!("{}{}", result, suffix)
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
    let batches = &mut history.batches;

    if batches.is_empty() {
        return Ok(UndoResult {
            success: false,
            restored: 0,
            failed: 0,
            files: Vec::new(),
            batch_id: None,
        });
    }

    let Some((batch_idx, file_idx)) = (if !batch_id.is_empty() {
        batches.iter().rposition(|b| b.batch_id == batch_id && !b.undone).and_then(|bi| {
            let fi = batches[bi].files.len().saturating_sub(1);
            if fi < batches[bi].files.len() {
                Some((bi, fi))
            } else {
                None
            }
        })
    } else {
        batches.iter().rposition(|b| !b.undone && !b.files.is_empty()).and_then(|bi| {
            let fi = batches[bi].files.len().saturating_sub(1);
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

    let entry = batches[batch_idx].files[file_idx].clone();

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
            batch_id: Some(batches[batch_idx].batch_id.clone()),
        });
    }

    match fs::rename(&entry.new_path, &entry.old_path) {
        Ok(_) => {
            batches[batch_idx].files.remove(file_idx);
            if batches[batch_idx].files.is_empty() {
                batches[batch_idx].undone = true;
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
                batch_id: Some(batches[batch_idx].batch_id.clone()),
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
            batch_id: Some(batches[batch_idx].batch_id.clone()),
        }),
    }
}


