//! Unified error types for the AutoRename-Revived v4.0.0 backend.
//!
//! Every command surface converts these into user-friendly `String` messages,
//! but the internal pipeline works with typed errors so failure paths are
//! explicit and testable.

use std::io;
use std::path::PathBuf;

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

    /// A retryable transient failure (network timeout, rate limit).
    #[error("Transient error: {0}")]
    Transient(String),

    /// File already exists at the destination.
    #[error("Collision: {0}")]
    Collision(String),
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

    /// Attach context to an error, preserving the original cause.
    pub fn with_context(self, ctx: impl AsRef<str>) -> Self {
        let prefix = ctx.as_ref();
        match self {
            AppError::Io(e) => AppError::Io(io::Error::new(e.kind(), format!("{prefix}: {e}"))),
            AppError::Network(e) => AppError::Network(format!("{prefix}: {e}")),
            AppError::AiProvider(e) => AppError::AiProvider(format!("{prefix}: {e}")),
            AppError::AiApi { code, message } => AppError::AiApi {
                code,
                message: format!("{prefix}: {message}"),
            },
            AppError::Config(e) => AppError::Config(format!("{prefix}: {e}")),
            AppError::PathSafety(e) => AppError::PathSafety(format!("{prefix}: {e}")),
            AppError::UnsupportedFileType(e) => {
                AppError::UnsupportedFileType(format!("{prefix}: {e}"))
            }
            AppError::Internal(e) => AppError::Internal(format!("{prefix}: {e}")),
            AppError::Transient(e) => AppError::Transient(format!("{prefix}: {e}")),
            AppError::Collision(e) => AppError::Collision(format!("{prefix}: {e}")),
            AppError::Cancelled => AppError::Cancelled,
            AppError::Json(e) => AppError::Json(e),
        }
    }

    /// Category label for telemetry/logging.
    pub fn category(&self) -> &'static str {
        match self {
            AppError::Io(_) => "io",
            AppError::Json(_) => "json",
            AppError::Network(_) => "network",
            AppError::AiProvider(_) => "ai_provider",
            AppError::AiApi { .. } => "ai_api",
            AppError::Config(_) => "config",
            AppError::PathSafety(_) => "path_safety",
            AppError::UnsupportedFileType(_) => "unsupported_file",
            AppError::Cancelled => "cancelled",
            AppError::Internal(_) => "internal",
            AppError::Transient(_) => "transient",
            AppError::Collision(_) => "collision",
        }
    }
}

/// Convenience alias used throughout the pipeline.
pub type AppResult<T> = Result<T, AppError>;

/// A collision record: source path and the existing destination that blocked the rename.
#[derive(Debug, Clone)]
pub struct CollisionInfo {
    pub source: PathBuf,
    pub destination: PathBuf,
}