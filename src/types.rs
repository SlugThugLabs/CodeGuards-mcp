//! Core domain models and reporting types.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::PathBuf;

/// Severity level of a guard violation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Severity {
    Info,
    Warning,
    Error,
}

/// A specific violation detected in a file or project structure.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Violation {
    pub guard_id: String,
    pub file: PathBuf,
    pub line: Option<usize>,
    pub message: String,
    pub severity: Severity,
    pub fix_suggestion: Option<String>,
    pub rule_reference: Option<String>,
}

/// A verified user-authorized exception entry.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExceptionEntry {
    pub token: String,
    pub file: PathBuf,
    pub guard_id: String,
    pub reason: String,
    pub granted_at: String,
}

/// Aggregated report from running guard tests.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct GuardReport {
    pub project_root: PathBuf,
    pub total_files_checked: usize,
    pub violations: Vec<Violation>,
    pub passed_tests: Vec<String>,
    pub active_exceptions: Vec<ExceptionEntry>,
    pub duration_ms: u64,
}

impl GuardReport {
    #[must_use]
    pub fn is_pass(&self) -> bool {
        !self
            .violations
            .iter()
            .any(|v| v.severity == Severity::Error)
    }

    #[must_use]
    pub fn error_count(&self) -> usize {
        self.violations
            .iter()
            .filter(|v| v.severity == Severity::Error)
            .count()
    }

    #[must_use]
    pub fn warning_count(&self) -> usize {
        self.violations
            .iter()
            .filter(|v| v.severity == Severity::Warning)
            .count()
    }
}

/// Reusable Guard Test definition schema (.guard.json).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GuardTestDefinition {
    pub id: String,
    pub name: String,
    pub category: String,
    pub version: String,
    pub summary: String,
    pub tags: Vec<String>,
    pub aliases: Vec<String>,
    pub engine: String,
    #[serde(default)]
    pub default_params: BTreeMap<String, serde_json::Value>,
    pub remediation: String,
}
