//! Sandboxing and utility functions.

use crate::error::{CodeGuardsError, Result};
use std::path::{Path, PathBuf};

/// Validates that a path does not target sensitive system directories
/// (e.g., `/proc`, `/sys`, `/dev`, `~/.ssh`, `~/.aws`).
pub fn validate_safe_path(path: &Path) -> Result<PathBuf> {
    let canonical = match path.canonicalize() {
        Ok(p) => p,
        Err(_) => {
            // If path doesn't exist yet, normalize without canonicalize
            path.to_path_buf()
        }
    };

    let path_str = canonical.to_string_lossy();

    // Check blacklisted prefix patterns
    let blacklisted = [
        "/proc",
        "/sys",
        "/dev",
        "/.ssh",
        "/.aws",
        "/.gnupg",
        "/etc/shadow",
        "/etc/sudoers",
    ];

    for prefix in blacklisted {
        if path_str.contains(prefix) {
            return Err(CodeGuardsError::SandboxViolation {
                path: canonical,
            });
        }
    }

    Ok(canonical)
}

/// Generates a deterministic BLAKE3 hex hash of a project's root path.
#[must_use]
pub fn hash_project_path(project_path: &Path) -> String {
    let canonical = project_path
        .canonicalize()
        .unwrap_or_else(|_| project_path.to_path_buf());
    let mut hasher = blake3::Hasher::new();
    hasher.update(canonical.to_string_lossy().as_bytes());
    let hash = hasher.finalize();
    hash.to_hex()[..12].to_string()
}

/// Computes a deterministic 5-digit verification token for an exception.
#[must_use]
pub fn compute_exception_token(file: &Path, guard_id: &str, reason: &str) -> String {
    let mut hasher = blake3::Hasher::new();
    hasher.update(file.to_string_lossy().as_bytes());
    hasher.update(guard_id.as_bytes());
    hasher.update(reason.as_bytes());
    let hash = hasher.finalize();
    let bytes = hash.as_bytes();
    let num = u32::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]) % 90000 + 10000;
    num.to_string()
}

/// Returns the central ~/.slugthug directory.
pub fn get_slugthug_home() -> PathBuf {
    if let Ok(dir) = std::env::var("SLUGTHUG_HOME") {
        PathBuf::from(dir)
    } else if let Ok(home) = std::env::var("HOME") {
        PathBuf::from(home).join(".slugthug")
    } else {
        PathBuf::from("/root/.slugthug")
    }
}

/// Returns the central test library directory: ~/.slugthug/codeguards/tests/
pub fn get_tests_dir() -> PathBuf {
    get_slugthug_home().join("codeguards").join("tests")
}

/// Returns the project-specific storage directory: ~/.slugthug/codeguards/projects/<project_id>/
pub fn get_project_storage_dir(project_path: &Path) -> PathBuf {
    let hash = hash_project_path(project_path);
    let name = project_path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("unknown");
    let folder_name = format!("{name}-{hash}");
    get_slugthug_home()
        .join("codeguards")
        .join("projects")
        .join(folder_name)
}
