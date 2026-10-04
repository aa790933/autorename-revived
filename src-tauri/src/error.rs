//! Unified error types for the AutoRename-Revived v4.0.0 backend.
//!
//! Every command surface converts these into user-friendly `String` messages,
//! but the internal pipeline works with typed errors so failure paths are
//! explicit and testable.

use std::io;

/// Top-level application error.
///
/// `From` conversions let library code (IO, JSON, …) bubble up without
/// `.expect()` or `unwrap()` sprinkled through call sites.
#[derive(Debug, thiserror::Error)]
pub enum AppError {
    /// IO failure (file read, rename, directory creation, …).
    #[error("I/O error: {0}")]
    Io(#[from] io::Error),

    /// JSON serialisation / deserialisation failure.
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    /// A network request to an AI provider failed.
    #[error("Network error: {0}")]
    Network(String),

    /// The AI provider returned an unexpected or unparseable response.
    #[error("AI provider error: {0}")]
    AiProvider(String),

    /// The AI provider rejected the request (auth, quota, content policy).
    #[error("AI API error ({code}): {message}")]
    AiApi { code: u16, message: String },

    /// A configuration value is missing, malformed, or out of range.
    #[error("Configuration error: {0}")]
    Config(String),

    /// Path traversal or unsafe filename was blocked.
    #[error("Path safety error: {0}")]
    PathSafety(String),

    /// The requested file type is not supported.
    #[error("Unsupported file type: {0}")]
    UnsupportedFileType(String),

    /// The operation was cancelled by the user.
    #[error("Operation cancelled")]
    Cancelled,

    /// An internal/unexpected error (should not normally occur).
    #[error("Internal error: {0}")]
    Internal(String),
}

impl AppError {
    /// Build a network error from a `reqwest` error, preserving the status
    /// code when one is available.
    pub fn from_reqwest(e: reqwest::Error, context: &str) -> Self {
        let msg = if let Some(status) = e.status() {
            format!("{} (HTTP {})", context, status.as_u16())
        } else {
            format!("{}: {}", context, e)
        };
        Self::Network(msg)
    }

    /// Build an AI API error from a status code and message.
    pub fn ai_api(status: u16, message: impl Into<String>) -> Self {
        Self::AiApi {
            code: status,
            message: message.into(),
        }
    }

    /// Convert to a plain `String` for Tauri command return values.
    pub fn to_string_lossy(&self) -> String {
        self.to_string()
    }
}

/// Convenience alias used throughout the pipeline.
pub type AppResult<T> = Result<T, AppError>;