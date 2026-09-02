//! CodeGuards error taxonomy.

use std::path::PathBuf;
use thiserror::Error;

/// Core error type for CodeGuards operations.
#[derive(Debug, Error)]
pub enum CodeGuardsError {
    #[error("I/O error at {path}: {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    #[error("Sandbox violation: path {path} is unsafe (outside project root or accesses sensitive system paths)")]
    SandboxViolation { path: PathBuf },

    #[error("JSON serialization/deserialization error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("TOML parsing error in {path}: {source}")]
    TomlParse {
        path: PathBuf,
        #[source]
        source: toml::de::Error,
    },

    #[error("Architecture contract error: {0}")]
    Contract(String),

    #[error("Guard test schema validation error for {test_id}: {reason}")]
    InvalidGuardTest { test_id: String, reason: String },

    #[error("Exception verification failed: {0}")]
    ExceptionInvalid(String),

    #[error("Project not found at: {0}")]
    ProjectNotFound(PathBuf),

    #[error("Internal error: {0}")]
    Internal(String),
}

/// Convenience result type alias.
pub type Result<T> = std::result::Result<T, CodeGuardsError>;
