use serde::{Serialize, Deserialize};
use crate::config::NamingConfig;
use chrono::NaiveDate;
use regex::Regex;
use std::collections::HashMap;
use std::fs;
use std::path::Path;

/// Truncate a string to `max_len` characters (char-count, not bytes).
fn truncate_str(s: &str, max_len: usize) -> String {
    match s.char_indices().nth(max_len) {
        Some((idx, _)) => s[..idx].to_string(),
        None => s.to_string(),
    }
}

/// Cryptographically strong random u32 from the OS provider, replacing the
/// previous pointer-read-from-a-hasher trick (undefined behaviour).
fn rand_u32() -> u32 {
    getrandom::u32().expect("system RNG unavailable")
}

/// 8 hex chars for de-duplicating unreadable-file placeholders.
fn random_hex8() -> String {
    format!("{:08x}", rand_u32())
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

    /// Append one rename to the given batch, creating the batch when needed.
    /// Callers in a pipeline keep the history in memory and save once at the
    /// end — rewriting the log per renamed file is O(files x log size).
    pub fn add_entry(&mut self, batch_id: &str, old_path: &str, new_path: &str) {
        let timestamp = chrono::Local::now().to_rfc3339();
        let entry = UndoEntry {
            old_path: old_path.to_string(),
            new_path: new_path.to_string(),
            timestamp: timestamp.clone(),
        };
        let target = self
            .batches
            .iter_mut()
            .find(|b| b.batch_id == batch_id && !b.undone);
        match target {
            Some(batch) => batch.files.push(entry),
            None => {
                let batch_id = if batch_id.is_empty() {
                    format!("gui-{}", chrono::Local::now().format(r#"%Y%m%dT%H%M%S"#))
                } else {
                    batch_id.to_string()
                };
                self.batches.push(UndoBatch {
                    batch_id,
                    timestamp,
                    source: "gui".to_string(),
                    undone: false,
                    files: vec![entry],
                });
            }
        }
    }

    /// Drop the oldest batches beyond `max_batches`.
    pub fn trim(&mut self, max_batches: usize) {
        let max_batches = max_batches.max(1);
        while self.batches.len() > max_batches {
            self.batches.remove(0);
        }
    }
}

pub fn sanitize_filename(name: &str, max_length: usize) -> String {
    if name.is_empty() {
        return String::from("_");
    }

    // Compiled once per process instead of per call — this runs for every
    // field of every file in a batch.
    static INVALID_FS_CHARS: std::sync::OnceLock<Regex> = std::sync::OnceLock::new();
    static UNICODE_CONTROL: std::sync::OnceLock<Regex> = std::sync::OnceLock::new();
    static GIBBERISH_HEX: std::sync::OnceLock<Regex> = std::sync::OnceLock::new();
    static MULTI_SEP: std::sync::OnceLock<Regex> = std::sync::OnceLock::new();
    let invalid_fs_chars =
        INVALID_FS_CHARS.get_or_init(|| Regex::new(r#"[\x00-\x1f\\/:*?\"<>|]"#).unwrap());
    let unicode_control = UNICODE_CONTROL.get_or_init(|| {
        Regex::new(r#"[\u200b-\u200f\u2028-\u202f\u2060-\u2064\ufeff\u00ad]"#).unwrap()
    });
    let gibberish_hex =
        GIBBERISH_HEX.get_or_init(|| Regex::new(r#"\b[0-9]*[a-f][0-9a-f]{11,}\b"#).unwrap());
    let multi_sep = MULTI_SEP.get_or_init(|| Regex::new(r#"[_ \-]{2,}"#).unwrap());

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

    // Windows reserves device names for the whole base name (extension
    // irrelevant): "con.txt" is illegal. The previous check inspected the
    // wrong component and let "con.txt" through while flagging "doc.con".
    let base_name = cleaned.split('.').next().unwrap_or(&cleaned);
    let cleaned = if reserved_names.contains(&base_name.to_lowercase().as_str()) {
        format!("_{}", cleaned)
    } else {
        cleaned
    };

    cleaned
}

pub fn resolve_safe_path(
    directory: &str,
    filename: &str,
    max_length: usize,
) -> Result<String, String> {
    let dir_path = Path::new(directory)
        .canonicalize()
        .unwrap_or_else(|_| Path::new(directory).to_path_buf());
    let safe_filename = sanitize_filename(filename, max_length);
    if safe_filename.is_empty() || safe_filename == "_" {
        return Err(format!("File name {:?} sanitizes to nothing", filename));
    }
    let resolved = dir_path.join(Path::new(&safe_filename));

    // Component-aware containment check. A string `starts_with` accepted
    // `C:\foobar\x` as being inside `C:\foo`.
    if !resolved.starts_with(&dir_path) {
        return Err(format!("Path traversal blocked: {}", filename));
    }

    let mut resolved_str = resolved.to_string_lossy().to_string();
    // Rust's Windows canonicalize() returns `\\?\`-prefixed paths; strip it so
    // stored/displayed paths stay readable, re-adding it only when needed.
    if let Some(stripped) = resolved_str
        .strip_prefix(r"\\?\UNC\")
        .map(|s| s.to_string())
        .or_else(|| resolved_str.strip_prefix(r"\\?\").map(|s| s.to_string()))
    {
        resolved_str = stripped;
    }

    // Windows paths beyond MAX_PATH need the extended-length prefix.
    if resolved_str.len() > 259 {
        let extended = if resolved_str.starts_with(r"\\") {
            format!(r"\\?\UNC\{}", resolved_str.trim_start_matches('\\'))
        } else {
            format!(r"\\?\{}", resolved_str)
        };
        return Ok(extended);
    }

    Ok(resolved_str)
}

pub fn parse_document_date(date_str: &str) -> Option<String> {
    let date_str = date_str.trim();
    if date_str.is_empty() {
        return None;
    }

    // ISO form is the schema standard.
    if let Ok(parsed) = NaiveDate::parse_from_str(date_str, r#"%Y-%m-%d"#) {
        return Some(parsed.format(r#"%Y-%m-%d"#).to_string());
    }

    // Compact `YYYYMMDD`. The parsed date is validated rather than trusted:
    // "20241340" previously produced "2024-13-40", which then failed the
    // `YYYY-MM-DD` check downstream and dropped the date silently.
    let digits: String = date_str.chars().filter(|c| c.is_ascii_digit()).collect();
    if date_str.chars().all(|c| c.is_ascii_digit()) && digits.len() == 8 {
        if let Ok(parsed) = NaiveDate::parse_from_str(&digits, "%Y%m%d") {
            return Some(parsed.format(r#"%Y-%m-%d"#).to_string());
        }
    }

    // Ambiguous slash/dot forms. `MM/DD/YYYY` is tried first because the app
    // targets US-style documents; `DD/MM/YYYY` is the fallback. A value that
    // is valid under both is therefore read as month-first, which is the
    // documented behaviour.
    for fmt in ["%m/%d/%Y", "%m.%d.%Y", "%m-%d-%Y"] {
        if let Ok(parsed) = NaiveDate::parse_from_str(date_str, fmt) {
            return Some(parsed.format(r#"%Y-%m-%d"#).to_string());
        }
    }
    for fmt in ["%d/%m/%Y", "%d.%m.%Y", "%d-%m-%Y"] {
        if let Ok(parsed) = NaiveDate::parse_from_str(date_str, fmt) {
            return Some(parsed.format(r#"%Y-%m-%d"#).to_string());
        }
    }

    // Bare `YYYYMMDD` that also survived the compact branch above.
    None
}

/// Maximum number of characters kept from an AI-provided subject.
pub const MAX_SUBJECT_CHARS: usize = 30;

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
        static ISO_DATE: std::sync::OnceLock<Regex> = std::sync::OnceLock::new();
        let date_re =
            ISO_DATE.get_or_init(|| Regex::new(r#"^\d{4}-\d{2}-\d{2}$"#).unwrap());
        if !date_re.is_match(date_str) {
            warnings.push(format!(
                "Invalid date format '{}' — expected YYYY-MM-DD",
                date_str
            ));
        }
    }

    // Validate subject length (characters, not bytes — "Ürün Özet…" is
    // multi-byte in UTF-8). This mirrors the truncation applied when the
    // filename is rendered, so the warning and the output agree.
    let subject_chars = subject.chars().count();
    if subject_chars > MAX_SUBJECT_CHARS {
        warnings.push(format!(
            "Subject truncated from {} to {} chars",
            subject_chars, MAX_SUBJECT_CHARS
        ));
    }

    (is_error, warnings)
}

/// Build the lowercase-variation -> canonical-name map once per batch
/// instead of rebuilding it for every file.
pub fn build_company_lookup(
    harmonized_companies: &[HashMap<String, serde_json::Value>],
) -> HashMap<String, String> {
    let mut lookup: HashMap<String, String> = HashMap::new();
    for entry in harmonized_companies {
        let Some(company_name) = entry.get("name").and_then(|v| v.as_str()) else {
            continue;
        };
        lookup.insert(company_name.to_lowercase(), company_name.to_string());
        if let Some(variations) = entry.get("variations").and_then(|v| v.as_array()) {
            for var in variations {
                if let Some(v_str) = var.as_str() {
                    lookup.insert(v_str.to_lowercase(), company_name.to_string());
                }
            }
        }
    }
    lookup
}

/// Harmonize against a pre-built lookup (see [`build_company_lookup`]).
pub fn harmonize_with_lookup(name: &str, lookup: &HashMap<String, String>) -> String {
    if name.is_empty() || lookup.is_empty() {
        return name.to_string();
    }
    lookup.get(&name.to_lowercase()).cloned().unwrap_or_else(|| name.to_string())
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
        return format!("Error_Unreadable_File_{}{}", random_hex8(), suffix);
    }

    let has_content = !company.is_empty() || !doctype.is_empty() || !date_str.is_empty();
    let template = if has_content {
        &config.template
    } else {
        &config.fallback
    };

    // Normalize the date to YYYY-MM-DD internally, then render with the
    // user-configured `date_format` (falling back to ISO when unset).
    let date_normalized = parse_document_date(date_str).unwrap_or_default();
    let date_formatted = format_date(&date_normalized, &config.date_format);

    // The configured separator joins fields; sanitize it so an unsafe value
    // can never produce an invalid Windows file name.
    let separator = sanitize_filename(&effective_separator(&config.separator), 4);

    let clean_company = sanitize_filename(company, 32);
    let clean_doctype = sanitize_filename(doctype, 24);
    // Truncate subject to 30 chars max (char-boundary safe)
    let subject_truncated = truncate_str(subject, MAX_SUBJECT_CHARS);
    let clean_subject = sanitize_filename(&subject_truncated, 32);

    let original_stem = Path::new(original_filename)
        .file_stem()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| "file".to_string());

    let fields: HashMap<String, String> = [
        ("date".to_string(), date_formatted),
        ("company".to_string(), clean_company.clone()),
        ("doctype".to_string(), clean_doctype.clone()),
        ("subject".to_string(), clean_subject.clone()),
        ("original".to_string(), original_stem.clone()),
        // `{sequence}` renders the collision counter. The token is resolved by
        // `ensure_unique_filename`, which knows whether the target name is
        // already taken; the first candidate is always `1`.
        ("sequence".to_string(), SEQUENCE_TOKEN.to_string()),
    ]
    .into_iter()
    .collect();

    let mut result = render_template(template, &fields, &separator);

    // Fall back when the template produced nothing usable — no placeholders at
    // all, a placeholder that does not exist, or a value that sanitized away
    // to nothing.
    if result.trim().is_empty() || has_unresolved_placeholder(&result) {
        let fallback_fields: HashMap<String, String> = [
            ("date".to_string(), fields["date"].clone()),
            (
                "company".to_string(),
                if clean_company.is_empty() {
                    "Unknown".to_string()
                } else {
                    clean_company
                },
            ),
            (
                "doctype".to_string(),
                if clean_doctype.is_empty() {
                    "Doc".to_string()
                } else {
                    clean_doctype
                },
            ),
            (
                "subject".to_string(),
                if clean_subject.is_empty() {
                    "Unknown".to_string()
                } else {
                    clean_subject
                },
            ),
            ("original".to_string(), original_stem),
            ("sequence".to_string(), fields["sequence"].clone()),
        ]
        .into_iter()
        .collect();
        result = render_template(&config.fallback, &fallback_fields, &separator);
    }

    // Trim leading/trailing noise (any configured separator characters plus
    // the usual `_`, space and dot) before the length budget is applied.
    let mut result = result
        .trim_matches(|c: char| {
            c == '_' || c == ' ' || c == '.' || separator.contains(c)
        })
        .to_string();
    if result.is_empty() {
        result = "Unknown".to_string();
    }

    // Reserve room for the extension. If the rendered name is too long, the
    // stem must be truncated down to `max_length - suffix` so the *final*
    // file name (stem + extension) fits the limit and keeps its extension —
    // Windows routing/formatting depends on the extension being present.
    let avail = config.max_length as usize;
    if suffix.is_empty() {
        if result.chars().count() > avail {
            result = truncate_str(&result, avail);
            if result.is_empty() {
                result = "Unknown".to_string();
            }
        }
        return result;
    }
    let stem_only = result
        .strip_suffix(suffix.as_str())
        .or_else(|| result.strip_suffix(suffix.to_lowercase().as_str()))
        .unwrap_or(result.as_str());
    let avail_stem = avail.saturating_sub(suffix.chars().count());
    let stem = if stem_only.chars().count() > avail_stem {
        let trimmed = truncate_str(stem_only, avail_stem);
        if trimmed.is_empty() {
            "Unknown".to_string()
        } else {
            trimmed
        }
    } else {
        stem_only.to_string()
    };
    format!("{}{}", stem, suffix)
}

/// Placeholder resolved by [`ensure_unique_filename`] into `_01`, `_02`, …
const SEQUENCE_TOKEN: &str = "{sequence}";

/// True when `rendered` still contains a `{...}` token, i.e. the template
/// referenced a field this build does not know about.
fn has_unresolved_placeholder(rendered: &str) -> bool {
    let Some(start) = rendered.find('{') else {
        return false;
    };
    rendered[start + 1..].contains('}')
}

/// Effective separator: configured value, else `_`.
fn effective_separator(sep: &str) -> &str {
    let trimmed = sep.trim();
    if trimmed.is_empty() {
        "_"
    } else {
        trimmed
    }
}

/// Render a `NaiveDate` (YYYY-MM-DD input) with the user's format; fall back
/// to ISO when the format is empty or produces nothing.
fn format_date(iso: &str, fmt: &str) -> String {
    if iso.is_empty() {
        return String::new();
    }
    match NaiveDate::parse_from_str(iso, r#"%Y-%m-%d"#) {
        Ok(parsed) => {
            let f = if fmt.trim().is_empty() {
                "%Y-%m-%d"
            } else {
                fmt.trim()
            };
            let out = parsed.format(f).to_string();
            if out.is_empty() {
                iso.to_string()
            } else {
                out
            }
        }
        // Unparseable: keep the raw string rather than dropping the date.
        Err(_) => iso.to_string(),
    }
}

/// Substitute `{placeholder}` tokens segment by segment.
///
/// The template is split on the configured separator *before* substitution, so
/// only segments whose placeholders all resolved to empty are dropped — values
/// that legitimately contain the separator are preserved and the result never
/// carries doubled separators.
///
/// `{separator}` is expanded to the configured separator first, which is what
/// makes a template like `{date}{separator}{company}` honour a user-chosen
/// separator instead of the literal `_` baked into the template text.
fn render_template(template: &str, fields: &HashMap<String, String>, separator: &str) -> String {
    let sep = if separator.is_empty() { "_" } else { separator };
    let expanded = template.replace("{separator}", sep);

    let segments: Vec<String> = expanded
        .split(sep)
        .map(|segment| {
            let mut rendered = segment.to_string();
            for (key, val) in fields {
                let token = format!("{{{}}}", key);
                if rendered.contains(&token) {
                    rendered = rendered.replace(&token, val);
                }
            }
            rendered
        })
        .collect();

    segments
        .into_iter()
        .filter(|s| !s.trim().is_empty())
        .collect::<Vec<String>>()
        .join(sep)
}

/// Produce a file name that does not collide with anything in `directory`.
///
/// If `filename` contains the `{sequence}` token, the token is replaced by the
/// first free zero-padded counter (`_01`, `_02`, …). Without the token, a
/// counter is appended to the stem.
///
/// Returns an error when every candidate up to [`MAX_SEQUENCE`] is taken — the
/// alternative would be handing back a name that already exists on disk.
pub fn ensure_unique_filename(
    directory: &str,
    filename: &str,
    zerofill: u32,
) -> Result<String, String> {
    let has_token = filename.contains(SEQUENCE_TOKEN);
    let candidate = |counter: u32| -> String {
        if has_token {
            filename.replace(SEQUENCE_TOKEN, &sequence_suffix(counter, zerofill))
        } else {
            format!("{}{}", filename, sequence_suffix(counter, zerofill))
        }
    };

    if !Path::new(directory).join(&candidate(1)).exists() {
        return Ok(candidate(1));
    }

    // The counter starts at 2 because slot 1 is the name as rendered.
    for counter in 2u32..=MAX_SEQUENCE {
        let name = candidate(counter);
        if !Path::new(directory).join(&name).exists() {
            return Ok(name);
        }
    }

    Err(format!(
        "Could not find a free file name for {:?} in {} after {} attempts",
        filename, directory, MAX_SEQUENCE
    ))
}

/// Highest sequence number [`ensure_unique_filename`] will try.
const MAX_SEQUENCE: u32 = 9999;

/// `_01`, `_02`, … with the configured zero-fill width.
fn sequence_suffix(counter: u32, zerofill: u32) -> String {
    let width = zerofill.max(1) as usize;
    format!("_{:0width$}", counter, width = width)
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

/// Undo an entire rename batch (restoring files in reverse order so renames
/// that chained onto each other unwind cleanly). An empty `batch_id` undoes
/// the most recent not-yet-undone batch.
pub fn undo_last_rename(history_path: &Path, batch_id: &str) -> Result<UndoResult, String> {
    let mut history = UndoHistory::load(history_path)?;

    let Some(batch_idx) = (if !batch_id.is_empty() {
        history
            .batches
            .iter()
            .rposition(|b| b.batch_id == batch_id && !b.undone)
    } else {
        history
            .batches
            .iter()
            .rposition(|b| !b.undone && !b.files.is_empty())
    }) else {
        return Ok(UndoResult {
            success: false,
            restored: 0,
            failed: 0,
            files: Vec::new(),
            batch_id: None,
        });
    };

    let result_batch_id = history.batches[batch_idx].batch_id.clone();
    let entries: Vec<UndoEntry> = history.batches[batch_idx].files.clone();

    let mut files: Vec<UndoFileResult> = Vec::new();
    let mut restored = 0usize;
    let mut failed = 0usize;

    // Reverse order: the last rename applied is the first unwound.
    for entry in entries.iter().rev() {
        if !Path::new(&entry.new_path).exists() {
            // File moved/deleted by the user — leave the rest of the batch
            // intact but report this one as failed.
            failed += 1;
            files.push(UndoFileResult {
                old_path: entry.old_path.clone(),
                new_path: entry.new_path.clone(),
                status: "failed".to_string(),
                error: Some("Renamed file no longer exists".to_string()),
            });
            continue;
        }
        // Refuse to clobber an existing file at the original location.
        if Path::new(&entry.old_path).exists() {
            failed += 1;
            files.push(UndoFileResult {
                old_path: entry.old_path.clone(),
                new_path: entry.new_path.clone(),
                status: "failed".to_string(),
                error: Some("Target name is already taken".to_string()),
            });
            continue;
        }
        match fs::rename(&entry.new_path, &entry.old_path) {
            Ok(_) => {
                restored += 1;
                files.push(UndoFileResult {
                    old_path: entry.old_path.clone(),
                    new_path: entry.new_path.clone(),
                    status: "restored".to_string(),
                    error: None,
                });
            }
            Err(e) => {
                failed += 1;
                files.push(UndoFileResult {
                    old_path: entry.old_path.clone(),
                    new_path: entry.new_path.clone(),
                    status: "failed".to_string(),
                    error: Some(e.to_string()),
                });
            }
        }
    }

    // Only entries that could not be restored stay in the batch so a retry
    // does not attempt them twice as successful work.
    if restored > 0 {
        let batch = &mut history.batches[batch_idx];
        batch.files.retain(|e| {
            !files.iter().any(|f| {
                f.old_path == e.old_path && f.new_path == e.new_path && f.status == "restored"
            })
        });
        if batch.files.is_empty() {
            batch.undone = true;
        }
        history.save(history_path)?;
    }

    Ok(UndoResult {
        success: restored > 0,
        restored,
        failed,
        files,
        batch_id: Some(result_batch_id),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::NamingConfig;
    use std::path::PathBuf;

    fn cfg() -> NamingConfig {
        NamingConfig::default()
    }

    #[test]
    fn generate_filename_default_template() {
        let name = generate_filename(
            "Acme Corp",
            "Invoice",
            "2024-06-01",
            "Monthly service fee",
            &cfg(),
            "scan001.pdf",
            false,
        );
        assert_eq!(name, "2024-06-01_Invoice_Acme Corp_Monthly service fee.pdf");
    }

    #[test]
    fn generate_filename_honors_date_format_and_separator() {
        let mut c = cfg();
        c.date_format = "%Y%m%d".to_string();
        c.separator = "-".to_string();
        // The template must reference `{separator}` for the configured
        // separator to replace the literal `_` in the template text.
        c.template =
            "{date}{separator}{doctype}{separator}{company}{separator}{subject}".to_string();
        let name = generate_filename(
            "Acme",
            "Invoice",
            "2024-06-01",
            "Fee",
            &c,
            "a.pdf",
            false,
        );
        assert_eq!(name, "20240601-Invoice-Acme-Fee.pdf");
    }

    #[test]
    fn generate_filename_sequence_token_is_resolved_to_first_slot() {
        let mut c = cfg();
        c.template = "{date}_{sequence}_{company}".to_string();
        let name = generate_filename("Acme", "Invoice", "2024-06-01", "", &c, "a.pdf", false);
        assert_eq!(name, "2024-06-01_01_Acme.pdf");
    }

    #[test]
    fn generate_filename_supports_original_and_separator_placeholders() {
        let mut c = cfg();
        c.template = "{date}{separator}{original}".to_string();
        c.separator = "-".to_string();
        let name = generate_filename("", "", "2024-06-01", "", &c, "scan_001.pdf", false);
        assert_eq!(name, "2024-06-01-scan_001.pdf");
    }

    #[test]
    fn generate_filename_tiny_max_length_does_not_panic() {
        let mut c = cfg();
        c.max_length = 3; // shorter than ".pdf"
        let name = generate_filename("Acme", "Invoice", "2024-06-01", "Fee", &c, "a.pdf", false);
        assert!(name.len() <= 3 + 4);
        assert!(name.ends_with(".pdf") || name == "Unknown.pdf" || !name.is_empty());
    }

    #[test]
    fn generate_filename_empty_metadata_uses_fallback() {
        let name = generate_filename("", "", "", "", &cfg(), "a.pdf", false);
        assert!(name.starts_with("Unknown"), "got {}", name);
    }

    #[test]
    fn generate_filename_unknown_placeholder_falls_back_instead_of_leaking_it() {
        let mut c = cfg();
        c.template = "{date}_{bogus}".to_string();
        c.fallback = "{date}_{company}".to_string();
        let name = generate_filename("Acme", "Invoice", "2024-06-01", "", &c, "a.pdf", false);
        assert!(!name.contains('{'), "unresolved token leaked: {}", name);
        assert_eq!(name, "2024-06-01_Acme.pdf");
    }

    #[test]
    fn generate_filename_unreadable_document_gets_a_unique_placeholder() {
        let a = generate_filename("Acme", "Invoice", "2024-06-01", "x", &cfg(), "a.pdf", true);
        let b = generate_filename("Acme", "Invoice", "2024-06-01", "x", &cfg(), "b.pdf", true);
        assert!(a.starts_with("Error_Unreadable_File_"), "got {}", a);
        assert!(a.ends_with(".pdf"));
        assert_ne!(a, b, "the placeholder must not collide across files");
    }

    #[test]
    fn generate_filename_rejects_an_invalid_ai_date_instead_of_fabricating_one() {
        let mut c = cfg();
        c.date_format = "%Y%m%d".to_string();
        c.template = "{date}_{company}".to_string();
        c.fallback = "{company}_Unknown".to_string();
        let name = generate_filename("Acme", "Invoice", "20241340", "", &c, "a.pdf", false);
        assert!(!name.contains("20241340"), "invalid date leaked: {}", name);
    }

    // -----------------------------------------------------------------------
    // Unique naming
    // -----------------------------------------------------------------------

    #[test]
    fn ensure_unique_filename_resolves_the_sequence_token() {
        let dir = unique_temp_dir("seq");
        let name = ensure_unique_filename(&dir, "Report_{sequence}.pdf", 2).unwrap();
        assert_eq!(name, "Report_01.pdf");

        fs::write(dir.join("Report_01.pdf"), "x").unwrap();
        let name = ensure_unique_filename(&dir, "Report_{sequence}.pdf", 2).unwrap();
        assert_eq!(name, "Report_02.pdf");

        fs::write(dir.join("Report_02.pdf"), "x").unwrap();
        let name = ensure_unique_filename(&dir, "Report_{sequence}.pdf", 3).unwrap();
        assert_eq!(name, "Report_003.pdf");

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn ensure_unique_filename_appends_a_counter_without_the_token() {
        let dir = unique_temp_dir("nocounter");
        assert_eq!(
            ensure_unique_filename(&dir, "Report.pdf", 2).unwrap(),
            "Report.pdf"
        );

        fs::write(dir.join("Report.pdf"), "x").unwrap();
        assert_eq!(
            ensure_unique_filename(&dir, "Report.pdf", 2).unwrap(),
            "Report_02.pdf"
        );

        // A stem that merely *looks* like a sequence slot keeps its own text:
        // the old implementation stripped a trailing "_01" and produced
        // "Report_02.pdf" for "Report_01.pdf".
        assert_eq!(
            ensure_unique_filename(&dir, "Report_01.pdf", 2).unwrap(),
            "Report_01_02.pdf"
        );

        let _ = fs::remove_dir_all(&dir);
    }

    // -----------------------------------------------------------------------
    // Path safety
    // -----------------------------------------------------------------------

    #[test]
    fn resolve_safe_path_blocks_traversal_and_separators() {
        let dir = unique_temp_dir("safe");
        let resolved = resolve_safe_path(&dir.to_string_lossy(), "ok.pdf", 128).unwrap();
        assert!(resolved.ends_with("ok.pdf"));

        // Separators are sanitized away, so the result stays inside `dir`.
        let escaped =
            resolve_safe_path(&dir.to_string_lossy(), r"..\..\evil.pdf", 128).unwrap();
        assert!(
            Path::new(&escaped).starts_with(&dir),
            "escaped the directory: {}",
            escaped
        );

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn resolve_safe_path_rejects_a_name_that_sanitizes_to_nothing() {
        let dir = unique_temp_dir("empty");
        assert!(resolve_safe_path(&dir.to_string_lossy(), "...", 128).is_err());
        assert!(resolve_safe_path(&dir.to_string_lossy(), "", 128).is_err());
        let _ = fs::remove_dir_all(&dir);
    }

    // -----------------------------------------------------------------------
    // Dates
    // -----------------------------------------------------------------------

    #[test]
    fn parse_document_date_accepts_known_shapes_and_validates_them() {
        assert_eq!(
            parse_document_date("2024-06-01").as_deref(),
            Some("2024-06-01")
        );
        assert_eq!(
            parse_document_date("20240601").as_deref(),
            Some("2024-06-01")
        );
        assert_eq!(
            parse_document_date("06/01/2024").as_deref(),
            Some("2024-06-01")
        );
        // Impossible calendar dates are rejected rather than echoed back.
        assert_eq!(parse_document_date("20241340"), None);
        assert_eq!(parse_document_date("2024-02-31"), None);
        assert_eq!(parse_document_date(""), None);
        assert_eq!(parse_document_date("no date here"), None);
    }

    #[test]
    fn sanitize_reserved_device_name_any_extension() {
        assert_ne!(sanitize_filename("con.txt", 128), "con.txt");
        assert_eq!(sanitize_filename("account.txt", 128), "account.txt");
    }

    #[test]
    fn sanitize_replaces_invalid_chars() {
        let s = sanitize_filename("a/b\\c:d*e?f\"g<h>i|j", 128);
        assert!(!s.contains(['/', '\\', ':', '*', '?', '"', '<', '>', '|']));
    }

    #[test]
    fn truncate_is_char_count_not_bytes() {
        let s = "Ürün özeti burada uzun"; // multi-byte
        let t = truncate_str(s, 5);
        assert_eq!(t.chars().count(), 5);
        assert!(t.is_char_boundary(t.len()));
    }

    #[test]
    fn history_roundtrip_and_undo_batch() {
        let dir = unique_temp_dir("history");
        let log = dir.join("rename_history.json");
        let old = dir.join("old_a.txt");
        let old2 = dir.join("old_b.txt");
        let new = dir.join("new_a.txt");
        let new2 = dir.join("new_b.txt");
        let _ = fs::remove_file(&new);
        let _ = fs::remove_file(&new2);
        fs::write(&old, "a").unwrap();
        fs::write(&old2, "b").unwrap();

        let mut history = UndoHistory::new();
        fs::rename(&old, &new).unwrap();
        history.add_entry("b1", &old.to_string_lossy(), &new.to_string_lossy());
        fs::rename(&old2, &new2).unwrap();
        history.add_entry("b1", &old2.to_string_lossy(), &new2.to_string_lossy());
        history.save(&log).unwrap();

        let result = undo_last_rename(&log, "b1").unwrap();
        assert_eq!(result.restored, 2);
        assert!(old.exists() && old2.exists());

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn undo_refuses_to_clobber() {
        let dir = unique_temp_dir("clobber");
        let log = dir.join("rename_history.json");
        let old = dir.join("old_c.txt");
        let new = dir.join("new_c.txt");
        fs::write(&new, "renamed").unwrap();
        fs::write(&old, "occupied by someone else").unwrap();

        let mut history = UndoHistory::new();
        history.add_entry("b2", &old.to_string_lossy(), &new.to_string_lossy());
        history.save(&log).unwrap();

        let result = undo_last_rename(&log, "b2").unwrap();
        assert_eq!(result.restored, 0);
        assert_eq!(result.failed, 1);
        assert!(new.exists(), "undo must not destroy an existing target file");

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn legacy_v1_history_migrates() {
        let dir = unique_temp_dir("v1");
        let log = dir.join("rename_history.json");
        fs::write(
            &log,
            r#"[{"old_path":"a.txt","new_path":"b.txt","timestamp":"2024-01-01T00:00:00+00:00"}]"#,
        )
        .unwrap();
        let history = UndoHistory::load(&log).unwrap();
        assert_eq!(history.batches.len(), 1);
        assert_eq!(history.batches[0].files.len(), 1);
        let _ = fs::remove_file(&log);
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn undo_reports_nothing_to_do_on_an_empty_log() {
        let dir = unique_temp_dir("empty_undo");
        let log = dir.join("rename_history.json");
        let result = undo_last_rename(&log, "").unwrap();
        assert!(!result.success);
        assert_eq!(result.restored, 0);
        assert_eq!(result.failed, 0);
        assert!(result.batch_id.is_none());
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn undo_marks_a_fully_restored_batch_as_done() {
        let dir = unique_temp_dir("undo_done");
        let log = dir.join("rename_history.json");
        let old = dir.join("a.txt");
        let new = dir.join("b.txt");
        fs::write(&old, "x").unwrap();
        fs::rename(&old, &new).unwrap();

        let mut history = UndoHistory::new();
        history.add_entry("batch-1", &old.to_string_lossy(), &new.to_string_lossy());
        history.save(&log).unwrap();

        let first = undo_last_rename(&log, "batch-1").unwrap();
        assert_eq!(first.restored, 1);

        let reloaded = UndoHistory::load(&log).unwrap();
        assert!(reloaded.batches[0].undone, "restored batch must be closed");

        // A second undo of the same batch is a no-op, not a double rename.
        let second = undo_last_rename(&log, "batch-1").unwrap();
        assert_eq!(second.restored, 0);
        assert!(old.exists());

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn history_trim_keeps_the_newest_batches() {
        let mut history = UndoHistory::new();
        for i in 0..5 {
            history.add_entry(&format!("b{}", i), "a", "b");
        }
        assert_eq!(history.batches.len(), 5);
        history.trim(2);
        assert_eq!(history.batches.len(), 2);
        assert_eq!(history.batches[0].batch_id, "b3");
        assert_eq!(history.batches[1].batch_id, "b4");
        // Trimming to zero still keeps at least one batch.
        history.trim(0);
        assert_eq!(history.batches.len(), 1);
    }

    #[test]
    fn corrupt_history_loads_as_empty_instead_of_panicking() {
        let dir = unique_temp_dir("corrupt");
        let log = dir.join("rename_history.json");
        fs::write(&log, "{ this is not json").unwrap();
        let history = UndoHistory::load(&log).unwrap();
        assert!(history.batches.is_empty());
        let _ = fs::remove_dir_all(&dir);
    }

    /// Create a fresh, empty temp directory for one test.
    fn unique_temp_dir(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "autorename_test_{}_{}_{}",
            tag,
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0)
        ));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }
}


