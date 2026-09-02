//! Project storage manager and user-authorized exception store.

use crate::error::{CodeGuardsError, Result};
use crate::types::ExceptionEntry;
use crate::util::{compute_exception_token, get_project_storage_dir};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

/// Project-specific exceptions registry stored at ~/.slugthug/codeguards/projects/<id>/exceptions.json
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ProjectExceptions {
    pub project_path: PathBuf,
    pub version: String,
    pub exceptions: Vec<ExceptionEntry>,
}

impl ProjectExceptions {
    /// Loads the exceptions registry for a given project, or empty if not present.
    pub fn load(project_path: &Path) -> Result<Self> {
        let storage_dir = get_project_storage_dir(project_path);
        let file_path = storage_dir.join("exceptions.json");

        if !file_path.exists() {
            return Ok(Self {
                project_path: project_path.to_path_buf(),
                version: "1.0.0".to_string(),
                exceptions: Vec::new(),
            });
        }

        let content = fs::read_to_string(&file_path).map_err(|e| CodeGuardsError::Io {
            path: file_path.clone(),
            source: e,
        })?;

        let registry = serde_json::from_str::<Self>(&content)?;
        Ok(registry)
    }

    /// Saves the exceptions registry to ~/.slugthug/codeguards/projects/<id>/exceptions.json.
    pub fn save(&self) -> Result<PathBuf> {
        let storage_dir = get_project_storage_dir(&self.project_path);
        fs::create_dir_all(&storage_dir).map_err(|e| CodeGuardsError::Io {
            path: storage_dir.clone(),
            source: e,
        })?;

        let file_path = storage_dir.join("exceptions.json");
        let content = serde_json::to_string_pretty(self)?;
        fs::write(&file_path, content).map_err(|e| CodeGuardsError::Io {
            path: file_path.clone(),
            source: e,
        })?;

        Ok(file_path)
    }

    /// Adds a user-authorized exception and returns the generated token.
    pub fn add_exception(
        &mut self,
        file: &Path,
        guard_id: &str,
        reason: &str,
    ) -> Result<ExceptionEntry> {
        let token = compute_exception_token(file, guard_id, reason);

        // Deduplicate
        self.exceptions.retain(|e| e.token != token);

        let entry = ExceptionEntry {
            token: token.clone(),
            file: file.to_path_buf(),
            guard_id: guard_id.to_string(),
            reason: reason.to_string(),
            granted_at: chrono::Utc::now().to_rfc3339(),
        };

        self.exceptions.push(entry.clone());
        self.save()?;

        Ok(entry)
    }

    /// Revokes an exception by token.
    pub fn revoke(&mut self, token: &str) -> Result<bool> {
        let before_len = self.exceptions.len();
        self.exceptions.retain(|e| e.token != token);
        let removed = self.exceptions.len() < before_len;

        if removed {
            self.save()?;
        }

        Ok(removed)
    }

    /// Validates whether a file and guard violation is covered by an active exception token.
    #[must_use]
    pub fn is_exception_valid(&self, file: &Path, guard_id: &str, token: &str) -> bool {
        self.exceptions.iter().any(|e| {
            e.token == token
                && e.guard_id.eq_ignore_ascii_case(guard_id)
                && (e.file == file || file.ends_with(&e.file))
        })
    }
}
