use crate::ai::AiConfig;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use tauri_plugin_store::StoreBuilder;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocumentConfig {
    pub vision: String,
    pub vision_provider: String,
    #[serde(default = "default_text_quality_threshold")]
    pub text_quality_threshold: f64,
}

fn default_text_quality_threshold() -> f64 {
    0.2
}

impl Default for DocumentConfig {
    fn default() -> Self {
        Self {
            vision: "auto".to_string(),
            vision_provider: "gemini".to_string(),
            text_quality_threshold: 0.2,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NamingConfig {
    pub template: String,
    pub fallback: String,
    pub date_format: String,
    pub separator: String,
    pub max_length: u32,
    pub sequence_zerofill: u32,
    #[serde(default = "default_primary_language")]
    pub primary_language: String,
    #[serde(default = "default_suggestion_languages")]
    pub suggestion_languages: Vec<String>,
}

fn default_primary_language() -> String {
    "English".to_string()
}

fn default_suggestion_languages() -> Vec<String> {
    Vec::new()
}

impl Default for NamingConfig {
    fn default() -> Self {
        Self {
            template: "{date}_{doctype}_{company}_{subject}".to_string(),
            fallback: "{date}_{doctype}_{company}_Unknown".to_string(),
            date_format: String::from("%Y-%m-%d"),
            separator: String::from("_"),
            max_length: 128,
            sequence_zerofill: 2,
            primary_language: default_primary_language(),
            suggestion_languages: default_suggestion_languages(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UndoConfig {
    pub enabled: bool,
    /// Custom history-log location. Empty (the default) means "inside the
    /// settings directory", which keeps portable builds self-contained and
    /// avoids scattering a dot-folder through the user's home directory.
    #[serde(default)]
    pub log_path: String,
    /// Maximum number of *batches* kept in the log.
    pub max_entries: u32,
}

impl Default for UndoConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            log_path: String::new(),
            max_entries: 100,
        }
    }
}

/// Backup configuration for config file rotation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackupConfig {
    /// Keep N previous versions of settings.json on save.
    pub keep: u32,
    /// Directory for backup archives. Empty = settings directory.
    pub dir: String,
}

impl Default for BackupConfig {
    fn default() -> Self {
        Self {
            keep: 3,
            dir: String::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub ai: AiConfig,
    pub document: DocumentConfig,
    pub naming: NamingConfig,
    pub undo: UndoConfig,
    pub backup: BackupConfig,
    pub harmonized_companies: Vec<HashMap<String, serde_json::Value>>,
    pub debug: bool,
    pub max_workers: u32,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            ai: AiConfig::default(),
            document: DocumentConfig::default(),
            naming: NamingConfig::default(),
            undo: UndoConfig::default(),
            backup: BackupConfig::default(),
            harmonized_companies: Vec::new(),
            debug: false,
            max_workers: 4,
        }
    }
}

pub fn get_store_path(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    crate::portable::settings_path(app)
}

pub fn get_settings_directory(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    crate::portable::settings_dir(app)
}

pub fn ensure_settings_directory(app: &tauri::AppHandle) -> Result<(), String> {
    let dir = get_settings_directory(app)?;
    if !dir.exists() {
        std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    }
    Ok(())
}

/// Resolve the undo log location.
///
/// An empty `undo.log_path` (the default) puts the log in the settings
/// directory, so portable builds keep their history next to the executable.
/// A configured value wins and gets `~` expansion; a value that names a
/// directory (existing directory, or a trailing separator) has the default
/// file name appended.
pub fn resolve_undo_log_path(app: &tauri::AppHandle, config: &AppConfig) -> Result<PathBuf, String> {
    let custom = config.undo.log_path.trim();
    if custom.is_empty() {
        return crate::portable::undo_log_path(app);
    }

    let path = expand_tilde(custom);
    let looks_like_dir = custom.ends_with('\\')
        || custom.ends_with('/')
        || path.is_dir();
    if looks_like_dir {
        return Ok(path.join(DEFAULT_UNDO_LOG_NAME));
    }
    Ok(path)
}

/// File name used when the undo log lives in the settings directory.
pub const DEFAULT_UNDO_LOG_NAME: &str = "rename_history.json";

/// Expand a leading `~` to the user's home directory.
///
/// Used for `undo.log_path` so a saved `~/...` value resolves the way the
/// default documents it.
pub fn expand_tilde(path: &str) -> PathBuf {
    let trimmed = path.trim();
    if trimmed == "~" {
        return dirs_home().unwrap_or_else(|| PathBuf::from(trimmed));
    }
    if let Some(rest) = trimmed.strip_prefix("~/").or_else(|| trimmed.strip_prefix("~\\")) {
        if let Some(home) = dirs_home() {
            return home.join(rest);
        }
    }
    PathBuf::from(trimmed)
}

fn dirs_home() -> Option<PathBuf> {
    std::env::var_os("USERPROFILE")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(PathBuf::from))
}

/// Resolve `${VAR}` / `$VAR` placeholders from the process environment.
///
/// A single regex pass over the original text: substituted values are never
/// re-scanned, so a key containing a literal `$` (or a variable that expands
/// to text with `$` in it) is not mangled or double-expanded.
fn resolve_env_vars(value: &str) -> String {
    if !value.contains('$') {
        return value.to_string();
    }
    let re = regex::Regex::new(r"\$\{(\w+)\}|\$(\w+)").unwrap();
    re.replace_all(value, |caps: &regex::Captures| {
        let name = caps
            .get(1)
            .or_else(|| caps.get(2))
            .map(|m| m.as_str())
            .unwrap_or("");
        match std::env::var(name) {
            Ok(val) => val,
            // Unset (or a bare `$` followed by a non-identifier): keep as-is.
            Err(_) => caps[0].to_string(),
        }
    })
    .to_string()
}

fn resolve_config_env_vars(config: &mut AppConfig) {
    config.ai.api_key = resolve_env_vars(&config.ai.api_key);
    config.ai.base_url = resolve_env_vars(&config.ai.base_url);
    config.ai.gemini_base_url = resolve_env_vars(&config.ai.gemini_base_url);
    config.ai.custom_base_url = resolve_env_vars(&config.ai.custom_base_url);
    config.ai.ollama_base_url = resolve_env_vars(&config.ai.ollama_base_url);
}

/// Read the persisted config *without* resolving `${VAR}` placeholders.
///
/// Saving must round-trip exactly what is on disk: `load_config` substitutes
/// real secrets into `api_key`, and writing that back would silently replace a
/// `${GEMINI_API_KEY}` reference with the literal key in plaintext.
fn load_raw_config(app: &tauri::AppHandle) -> Result<AppConfig, String> {
    let store_path = get_store_path(app)?;
    if !store_path.exists() {
        return Ok(AppConfig::default());
    }
    let store = StoreBuilder::new(app, store_path)
        .build()
        .map_err(|e| e.to_string())?;
    match store.get("config") {
        // A malformed/partial file degrades to defaults instead of failing the
        // whole load, matching the previous behaviour.
        Some(saved) => Ok(serde_json::from_value::<AppConfig>(saved).unwrap_or_default()),
        None => Ok(AppConfig::default()),
    }
}

pub async fn load_config(app: tauri::AppHandle) -> Result<AppConfig, String> {
    let mut cfg = load_raw_config(&app)?;
    resolve_config_env_vars(&mut cfg);
    Ok(cfg)
}

pub async fn save_config(app: tauri::AppHandle, config: &AppConfig) -> Result<(), String> {
    let store_path = get_store_path(&app)?;
    rotate_backup(&store_path, &config.backup)?;
    let store = StoreBuilder::new(&app, store_path)
        .build()
        .map_err(|e| e.to_string())?;
    store.set(
        "config",
        serde_json::to_value(config).map_err(|e| e.to_string())?,
    );
    store.save().map_err(|e| e.to_string())?;
    Ok(())
}

/// Rotate backup copies of the settings file before each save.
///
/// Keeps `backup.keep` numbered snapshots (settings.json.1, .2, …)
/// in the backup directory (or the settings directory when empty).
fn rotate_backup(settings_path: &PathBuf, backup: &BackupConfig) -> Result<(), String> {
    if backup.keep == 0 || !settings_path.exists() {
        return Ok(());
    }
    let backup_dir = if backup.dir.trim().is_empty() {
        settings_path
            .parent()
            .map(|p| p.to_path_buf())
            .unwrap_or_else(|| PathBuf::from("."))
    } else {
        let p = expand_tilde(&backup.dir);
        if !p.exists() {
            std::fs::create_dir_all(&p).map_err(|e| e.to_string())?;
        }
        p
    };
    let base = settings_path
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| "settings.json".to_string());
    // Shift existing backups down (3 → 4, 2 → 3, …) then copy current → .1
    for i in (1..backup.keep).rev() {
        let src = backup_dir.join(format!("{base}.{i}"));
        let dst = backup_dir.join(format!("{base}.{}", i + 1));
        if src.exists() {
            let _ = std::fs::rename(src, dst);
        }
    }
    let _ = std::fs::copy(settings_path, backup_dir.join(format!("{base}.1")));
    Ok(())
}

pub async fn save_config_batch(
    app: tauri::AppHandle,
    updates: Vec<(String, String)>,
) -> Result<ConfigBatchResult, String> {
    // Start from the *raw* persisted config so `${VAR}` references survive a
    // save instead of being replaced by their resolved values.
    let mut config = load_raw_config(&app)?;
    let mut saved = 0u32;
    let mut failed = 0u32;
    let mut errors = Vec::new();

    for (key, value) in updates {
        match apply_config_update(&mut config, &key, &value) {
            Ok(()) => saved += 1,
            Err(e) => {
                failed += 1;
                errors.push(e);
            }
        }
    }

    // Persist even when some updates failed: a partial save is better than
    // silently dropping the valid ones, and the result reports the failures.
    if saved > 0 {
        save_config(app, &config).await?;
    }

    Ok(ConfigBatchResult {
        success: failed == 0,
        saved,
        failed,
        errors,
    })
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigBatchResult {
    pub success: bool,
    pub saved: u32,
    pub failed: u32,
    pub errors: Vec<String>,
}

/// AI providers the backend can route to. Kept next to `ai::get_model_name`
/// so the settings UI, `validate_config` and this validator cannot drift.
pub const KNOWN_PROVIDERS: [&str; 6] =
    ["gemini", "openai", "anthropic", "ollama", "xai", "custom"];

/// Placeholders `document::render_template` actually substitutes. Anything
/// else is rejected at save time so a typo cannot silently produce "Unknown".
pub const KNOWN_PLACEHOLDERS: [&str; 7] = [
    "{date}",
    "{company}",
    "{doctype}",
    "{subject}",
    "{original}",
    "{sequence}",
    "{separator}",
];

/// Config keys the settings UI may write, as `section.field`. Used by tests to
/// guarantee the frontend form and this validator stay in sync.
pub fn config_key_is_known(key: &str) -> bool {
    let Some((section, field)) = key.split_once('.') else {
        return false;
    };
    match section {
        "ai" => matches!(
            field,
            "provider"
                | "api_key"
                | "model"
                | "gemini_model"
                | "base_url"
                | "gemini_base_url"
                | "custom_model"
                | "custom_base_url"
                | "ollama_base_url"
                | "temperature"
                | "timeout"
                | "system_prompt"
        ),
        "document" => matches!(field, "vision" | "vision_provider" | "text_quality_threshold"),
        "naming" => matches!(
            field,
            "template"
                | "fallback"
                | "date_format"
                | "separator"
                | "max_length"
                | "sequence_zerofill"
                | "primary_language"
                | "suggestion_languages"
        ),
        "undo" => matches!(field, "enabled" | "log_path" | "max_entries"),
        "backup" => matches!(field, "keep" | "dir"),
        "_general" => matches!(field, "debug" | "max_workers"),
        _ => false,
    }
}

pub fn apply_config_update(
    config: &mut AppConfig,
    key: &str,
    value: &str,
) -> Result<(), String> {
    let parts: Vec<&str> = key.splitn(2, '.').collect();
    if parts.len() != 2 {
        return Err(format!("Invalid config key format: {}", key));
    }
    let section = parts[0];
    let field = parts[1];

    match section {
        "ai" => {
            let ai = &mut config.ai;
            match field {
                "provider" => {
                    let v = value.trim();
                    if !KNOWN_PROVIDERS.contains(&v) {
                        return Err(format!(
                            "Invalid AI provider: {} (expected one of {})",
                            v,
                            KNOWN_PROVIDERS.join(", ")
                        ));
                    }
                    ai.provider = v.to_string();
                }
                "api_key" => ai.api_key = value.to_string(),
                "model" => ai.model = value.to_string(),
                "gemini_model" => ai.gemini_model = value.to_string(),
                "base_url" => ai.base_url = value.to_string(),
                "gemini_base_url" => ai.gemini_base_url = value.to_string(),
                "custom_model" => ai.custom_model = value.to_string(),
                "custom_base_url" => ai.custom_base_url = value.to_string(),
                "ollama_base_url" => ai.ollama_base_url = value.to_string(),
                "temperature" => {
                    let v: f64 = value
                        .trim()
                        .parse()
                        .map_err(|_| format!("Invalid ai.temperature: {}", value))?;
                    if !v.is_finite() || !(0.0..=2.0).contains(&v) {
                        return Err("ai.temperature must be between 0 and 2".into());
                    }
                    ai.temperature = v;
                }
                "timeout" => {
                    let v: u64 = value
                        .trim()
                        .parse()
                        .map_err(|_| format!("Invalid ai.timeout: {}", value))?;
                    if !(5..=600).contains(&v) {
                        return Err("ai.timeout must be between 5 and 600 seconds".into());
                    }
                    ai.timeout = v;
                }
                "system_prompt" => ai.system_prompt = value.to_string(),
                _ => {
                    return Err(format!("Unknown AI config field: {}", field));
                }
            }
        }
        "document" => {
            let document = &mut config.document;
            match field {
                "vision" => {
                    let v = value.trim();
                    if !["auto", "true", "false"].contains(&v) {
                        return Err(format!(
                            "Invalid document.vision: {} (expected auto, true or false)",
                            v
                        ));
                    }
                    document.vision = v.to_string();
                }
                "vision_provider" => {
                    let v = value.trim();
                    if !v.is_empty() && !KNOWN_PROVIDERS.contains(&v) {
                        return Err(format!(
                            "Invalid document.vision_provider: {} (expected one of {})",
                            v,
                            KNOWN_PROVIDERS.join(", ")
                        ));
                    }
                    document.vision_provider = v.to_string();
                }
                "text_quality_threshold" => {
                    let v: f64 = value
                        .trim()
                        .parse()
                        .map_err(|_| format!("Invalid document.text_quality_threshold: {}", value))?;
                    if !v.is_finite() || !(0.0..=1.0).contains(&v) {
                        return Err("document.text_quality_threshold must be between 0 and 1".into());
                    }
                    document.text_quality_threshold = v;
                }
                _ => {
                    return Err(format!("Unknown document config field: {}", field));
                }
            }
        }
        "naming" => {
            let naming = &mut config.naming;
            match field {
                "template" | "fallback" => {
                    let unknown: Vec<String> = extract_placeholders(value)
                        .into_iter()
                        .filter(|p| !KNOWN_PLACEHOLDERS.contains(&p.as_str()))
                        .collect();
                    if !unknown.is_empty() {
                        return Err(format!(
                            "Unknown naming placeholder(s) in {}: {} (valid: {})",
                            field,
                            unknown.join(", "),
                            KNOWN_PLACEHOLDERS.join(", ")
                        ));
                    }
                    if field == "template" {
                        naming.template = value.to_string();
                    } else {
                        naming.fallback = value.to_string();
                    }
                }
                "date_format" => {
                    if !is_valid_strftime(value) {
                        return Err(format!("Invalid naming.date_format: {}", value));
                    }
                    naming.date_format = value.to_string();
                }
                "separator" => {
                    if is_invalid_separator(value) {
                        return Err(format!(
                            "Invalid naming.separator: {:?} must not contain characters that \
                             are forbidden in file names",
                            value
                        ));
                    }
                    naming.separator = value.to_string();
                }
                "max_length" => {
                    let v: u32 = parse_num(value, "naming.max_length")?;
                    if !(16..=255).contains(&v) {
                        return Err("naming.max_length must be between 16 and 255".into());
                    }
                    naming.max_length = v;
                }
                "sequence_zerofill" => {
                    let v: u32 = parse_num(value, "naming.sequence_zerofill")?;
                    if !(1..=9).contains(&v) {
                        return Err("naming.sequence_zerofill must be between 1 and 9".into());
                    }
                    naming.sequence_zerofill = v;
                }
                "primary_language" => naming.primary_language = value.to_string(),
                "suggestion_languages" => {
                    naming.suggestion_languages = if value.is_empty() {
                        Vec::new()
                    } else {
                        value
                            .split(',')
                            .map(|s| s.trim().to_string())
                            .filter(|s| !s.is_empty())
                            .collect()
                    };
                }
                _ => {
                    return Err(format!("Unknown naming config field: {}", field));
                }
            }
        }
        "undo" => {
            let undo = &mut config.undo;
            match field {
                "enabled" => {
                    undo.enabled = parse_bool(value).ok_or_else(|| {
                        format!("Invalid undo.enabled: {} (expected true or false)", value)
                    })?;
                }
                "log_path" => undo.log_path = value.to_string(),
                "max_entries" => {
                    let v: u32 = parse_num(value, "undo.max_entries")?;
                    if v == 0 || v > 100_000 {
                        return Err("undo.max_entries must be between 1 and 100000".into());
                    }
                    undo.max_entries = v;
                }
                _ => {
                    return Err(format!("Unknown undo config field: {}", field));
                }
            }
        }
        "backup" => {
            let backup = &mut config.backup;
            match field {
                "keep" => {
                    let v: u32 = parse_num(value, "backup.keep")?;
                    if v == 0 || v > 100 {
                        return Err("backup.keep must be between 1 and 100".into());
                    }
                    backup.keep = v;
                }
                "dir" => backup.dir = value.to_string(),
                _ => {
                    return Err(format!("Unknown backup config field: {}", field));
                }
            }
        }
        "_general" => match field {
            "debug" => {
                config.debug = parse_bool(value).ok_or_else(|| {
                    format!("Invalid _general.debug: {} (expected true or false)", value)
                })?;
            }
            "max_workers" => {
                let v: u32 = parse_num(value, "max_workers")?;
                if v == 0 || v > 32 {
                    return Err("max_workers must be between 1 and 32".into());
                }
                config.max_workers = v;
            }
            _ => {
                return Err(format!("Unknown general config field: {}", field));
            }
        },
        _ => {
            return Err(format!("Unknown config section: {}", section));
        }
    }
    Ok(())
}

/// Parse a number, rejecting empty/non-numeric input instead of silently
/// keeping the previous value.
fn parse_num<T: std::str::FromStr>(value: &str, key: &str) -> Result<T, String>
where
    T::Err: std::fmt::Display,
{
    value
        .trim()
        .parse::<T>()
        .map_err(|e| format!("Invalid {}: {:?} ({})", key, value, e))
}

fn parse_bool(value: &str) -> Option<bool> {
    match value.trim().to_ascii_lowercase().as_str() {
        "true" | "1" | "yes" | "on" => Some(true),
        "false" | "0" | "no" | "off" => Some(false),
        _ => None,
    }
}

/// `{...}` tokens inside a naming template.
fn extract_placeholders(template: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut rest = template;
    while let Some(start) = rest.find('{') {
        let after = &rest[start..];
        match after.find('}') {
            Some(end) => {
                out.push(after[..=end].to_string());
                rest = &after[end + 1..];
            }
            None => break,
        }
    }
    out
}

/// Reject separators that would produce illegal file names.
///
/// An empty value is allowed and means "use the default `_`" , the UI field
/// can therefore be cleared to reset it, which `document::effective_separator`
/// already handles.
fn is_invalid_separator(value: &str) -> bool {
    value.chars().any(|c| {
        matches!(c, '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|') || c.is_control()
    })
}

/// Light validation for a `chrono`/`strftime` style date format: it must be
/// non-empty, contain at least one real directive, and every `%` must be
/// followed by a recognized directive (with optional `-`, `_`, `0` or `^`
/// modifier) or `%%`.
fn is_valid_strftime(value: &str) -> bool {
    if value.trim().is_empty() {
        return false;
    }
    let mut chars = value.chars().peekable();
    let mut has_directive = false;
    while let Some(c) = chars.next() {
        if c != '%' {
            continue;
        }
        // `%%` is a literal percent sign, not a directive.
        if chars.peek() == Some(&'%') {
            chars.next();
            continue;
        }
        // Padding/case modifiers may precede the directive character.
        while matches!(chars.peek(), Some('-' | '_' | '0' | '^')) {
            chars.next();
        }
        let Some(directive) = chars.next() else {
            return false;
        };
        if !is_strftime_directive(directive) {
            return false;
        }
        has_directive = true;
    }
    has_directive
}

/// Directives accepted by `chrono::format::strftime`.
fn is_strftime_directive(d: char) -> bool {
    matches!(
        d,
        'Y' | 'C'
            | 'y'
            | 'm'
            | 'b'
            | 'B'
            | 'h'
            | 'd'
            | 'e'
            | 'a'
            | 'A'
            | 'w'
            | 'u'
            | 'U'
            | 'W'
            | 'G'
            | 'g'
            | 'V'
            | 'j'
            | 'D'
            | 'x'
            | 'F'
            | 'v'
            | 'H'
            | 'k'
            | 'I'
            | 'l'
            | 'P'
            | 'p'
            | 'M'
            | 'S'
            | 'f'
            | 'R'
            | 'T'
            | 'X'
            | 'r'
            | 'Z'
            | 'z'
            | ':'
            | '+'
            | 's'
            | 't'
            | 'n'
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn config() -> AppConfig {
        AppConfig::default()
    }

    #[test]
    fn every_provider_the_backend_routes_to_is_accepted() {
        for provider in KNOWN_PROVIDERS {
            let mut c = config();
            apply_config_update(&mut c, "ai.provider", provider).unwrap();
            assert_eq!(c.ai.provider, provider);
        }
    }

    #[test]
    fn provider_is_trimmed_and_validated() {
        let mut c = config();
        apply_config_update(&mut c, "ai.provider", "  openai  ").unwrap();
        assert_eq!(c.ai.provider, "openai");

        assert!(apply_config_update(&mut c, "ai.provider", "gpt5").is_err());
        // The rejected value must not have been written.
        assert_eq!(c.ai.provider, "openai");
    }

    #[test]
    fn template_placeholders_cover_everything_render_template_substitutes() {
        let mut c = config();
        for placeholder in KNOWN_PLACEHOLDERS {
            let template = format!("{{date}}_{}", placeholder);
            apply_config_update(&mut c, "naming.template", &template)
                .unwrap_or_else(|e| panic!("{} rejected: {}", placeholder, e));
        }
        assert!(apply_config_update(&mut c, "naming.template", "{date}_{bogus}").is_err());
        assert!(apply_config_update(&mut c, "naming.fallback", "{unknown}").is_err());
    }

    #[test]
    fn separator_allows_empty_but_rejects_illegal_characters() {
        let mut c = config();
        apply_config_update(&mut c, "naming.separator", "").unwrap();
        assert_eq!(c.naming.separator, "");

        apply_config_update(&mut c, "naming.separator", "-").unwrap();
        assert_eq!(c.naming.separator, "-");

        for bad in ["/", "\\", ":", "*", "?", "\"", "<", ">", "|"] {
            assert!(
                apply_config_update(&mut c, "naming.separator", bad).is_err(),
                "{:?} should be rejected",
                bad
            );
        }
    }

    #[test]
    fn date_format_validation_accepts_real_directives_and_rejects_junk() {
        for good in ["%Y%m%d", "%Y-%m-%d", "%d/%m/%Y", "%Y%m%d_%H%M%S", "100%%%Y"] {
            assert!(is_valid_strftime(good), "{} should be valid", good);
        }
        for bad in ["", "   ", "literal-only", "%", "%Q", "%Y%"] {
            assert!(!is_valid_strftime(bad), "{:?} should be invalid", bad);
        }
    }

    #[test]
    fn numeric_ranges_are_enforced() {
        let mut c = config();

        assert!(apply_config_update(&mut c, "ai.temperature", "-1").is_err());
        assert!(apply_config_update(&mut c, "ai.temperature", "3").is_err());
        assert!(apply_config_update(&mut c, "ai.temperature", "nan").is_err());
        assert!(apply_config_update(&mut c, "ai.temperature", "0.5").is_ok());

        assert!(apply_config_update(&mut c, "ai.timeout", "1").is_err());
        assert!(apply_config_update(&mut c, "ai.timeout", "30").is_ok());

        assert!(apply_config_update(&mut c, "naming.max_length", "8").is_err());
        assert!(apply_config_update(&mut c, "naming.max_length", "128").is_ok());

        assert!(apply_config_update(&mut c, "naming.sequence_zerofill", "0").is_err());
        assert!(apply_config_update(&mut c, "naming.sequence_zerofill", "3").is_ok());

        assert!(apply_config_update(&mut c, "undo.max_entries", "0").is_err());
        assert!(apply_config_update(&mut c, "max_workers", "0").is_err());
        assert!(apply_config_update(&mut c, "max_workers", "33").is_err());
        assert!(apply_config_update(&mut c, "max_workers", "8").is_ok());

        assert!(apply_config_update(&mut c, "document.text_quality_threshold", "1.5").is_err());
        assert!(apply_config_update(&mut c, "document.text_quality_threshold", "0.2").is_ok());
    }

    #[test]
    fn unknown_keys_are_rejected() {
        let mut c = config();
        assert!(apply_config_update(&mut c, "nope", "x").is_err());
        assert!(apply_config_update(&mut c, "ai.nope", "x").is_err());
        assert!(apply_config_update(&mut c, "nope.field", "x").is_err());
        assert!(apply_config_update(&mut c, "document.nope", "x").is_err());
        assert!(apply_config_update(&mut c, "undo.nope", "x").is_err());
    }

    #[test]
    fn suggestion_languages_are_split_and_trimmed() {
        let mut c = config();
        apply_config_update(&mut c, "naming.suggestion_languages", " French , Arabic, ").unwrap();
        assert_eq!(c.naming.suggestion_languages, vec!["French", "Arabic"]);

        apply_config_update(&mut c, "naming.suggestion_languages", "").unwrap();
        assert!(c.naming.suggestion_languages.is_empty());
    }

    #[test]
    fn booleans_accept_common_spellings() {
        let mut c = config();
        apply_config_update(&mut c, "undo.enabled", "off").unwrap();
        assert!(!c.undo.enabled);
        apply_config_update(&mut c, "undo.enabled", "TRUE").unwrap();
        assert!(c.undo.enabled);
        assert!(apply_config_update(&mut c, "undo.enabled", "maybe").is_err());
    }

    #[test]
    fn vision_mode_and_provider_are_validated() {
        let mut c = config();
        for mode in ["auto", "true", "false"] {
            apply_config_update(&mut c, "document.vision", mode).unwrap();
            assert_eq!(c.document.vision, mode);
        }
        assert!(apply_config_update(&mut c, "document.vision", "sometimes").is_err());

        apply_config_update(&mut c, "document.vision_provider", "").unwrap();
        apply_config_update(&mut c, "document.vision_provider", "openai").unwrap();
        assert!(apply_config_update(&mut c, "document.vision_provider", "gpt5").is_err());
    }

    #[test]
    fn env_placeholders_are_substituted_in_one_pass_only() {
        // `$VAR` without braces is supported too.
        // Only the two branches that need no environment mutation are
        // exercised here: setting process environment variables from a
        // parallel test harness is racy, and the substitution itself is a
        // pure single-pass regex replace.
        assert_eq!(
            resolve_env_vars("${AUTORENAME_DEFINITELY_UNSET_VAR}"),
            "${AUTORENAME_DEFINITELY_UNSET_VAR}"
        );
        assert_eq!(
            resolve_env_vars("$AUTORENAME_DEFINITELY_UNSET_VAR"),
            "$AUTORENAME_DEFINITELY_UNSET_VAR"
        );
        assert_eq!(resolve_env_vars("plain"), "plain");
        assert_eq!(resolve_env_vars(""), "");
    }

    #[test]
    fn tilde_expansion_maps_to_the_home_directory() {
        let home = dirs_home().expect("test environment must expose a home directory");
        assert_eq!(expand_tilde("~"), home);
        // Both separators after `~` are accepted, whichever the host uses.
        assert_eq!(
            expand_tilde("~/logs/history.json"),
            home.join("logs").join("history.json")
        );
        assert_eq!(
            expand_tilde("~\\logs\\history.json"),
            home.join("logs").join("history.json")
        );
        // Relative and absolute paths pass through untouched.
        assert_eq!(
            expand_tilde("logs/history.json"),
            PathBuf::from("logs/history.json")
        );
        assert_eq!(expand_tilde("/tmp/h.json"), PathBuf::from("/tmp/h.json"));
    }

    #[test]
    fn default_undo_log_path_is_empty_meaning_settings_directory() {
        assert!(UndoConfig::default().log_path.is_empty());
    }

    #[test]
    fn known_key_helper_matches_the_validator() {
        for key in [
            "ai.provider",
            "ai.api_key",
            "ai.system_prompt",
            "document.vision",
            "document.vision_provider",
            "naming.template",
            "naming.suggestion_languages",
            "undo.log_path",
            "_general.max_workers",
        ] {
            assert!(config_key_is_known(key), "{} should be known", key);
        }
        for key in ["ai.nope", "nope.field", "no-dot", "_general.nope"] {
            assert!(!config_key_is_known(key), "{} should be unknown", key);
        }
    }

    #[test]
    fn config_round_trips_through_json() {
        let mut c = config();
        apply_config_update(&mut c, "ai.provider", "ollama").unwrap();
        apply_config_update(&mut c, "naming.template", "{date}_{original}").unwrap();
        let json = serde_json::to_value(&c).unwrap();
        let back: AppConfig = serde_json::from_value(json).unwrap();
        assert_eq!(back.ai.provider, "ollama");
        assert_eq!(back.naming.template, "{date}_{original}");
    }

    #[test]
    fn missing_optional_fields_fall_back_to_defaults() {
        // A config file written by an older version lacks these keys.
        let partial = serde_json::json!({
            "ai": {
                "provider": "gemini",
                "api_key": "",
                "model": "gpt-4o-mini",
                "gemini_model": "gemini-2.0-flash",
                "gemini_base_url": "",
                "custom_model": "",
                "custom_base_url": "",
                "temperature": 0.0,
                "timeout": 30
            },
            "document": { "vision": "auto", "vision_provider": "gemini" },
            "naming": {
                "template": "{date}",
                "fallback": "{date}",
                "date_format": "%Y%m%d",
                "separator": "_",
                "max_length": 128,
                "sequence_zerofill": 2
            },
            "undo": { "enabled": true, "max_entries": 100 },
            "harmonized_companies": [],
            "debug": false,
            "max_workers": 4
        });
        let parsed: AppConfig = serde_json::from_value(partial).unwrap();
        assert_eq!(parsed.document.text_quality_threshold, 0.2);
        assert!(parsed.undo.log_path.is_empty());
        assert_eq!(parsed.naming.primary_language, "English");
        assert!(parsed.naming.suggestion_languages.is_empty());
    }
}
