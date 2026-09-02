//! Parallel guard execution runner.

use crate::analyzer::{count_code_lines, find_debug_prints, find_unwrap_expect_calls};
use crate::contract::ArchitectureContract;
use crate::error::Result;
use crate::library::catalog::GuardCatalog;
use crate::storage::ProjectExceptions;
use crate::types::{GuardReport, Severity, Violation};
use rayon::prelude::*;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::Instant;

/// Runs all applicable guards across the target list of files.
pub fn run_guard_checks(
    project_root: &Path,
    files: &[PathBuf],
    contract: &ArchitectureContract,
    catalog: &GuardCatalog,
    exceptions: &ProjectExceptions,
) -> Result<GuardReport> {
    let start = Instant::now();

    // Parallel evaluation over files
    let violations: Vec<Violation> = files
        .par_iter()
        .flat_map(|file| evaluate_file_guards(project_root, file, contract, catalog, exceptions))
        .collect();

    let duration_ms = start.elapsed().as_millis() as u64;

    Ok(GuardReport {
        project_root: project_root.to_path_buf(),
        total_files_checked: files.len(),
        violations,
        passed_tests: contract.enforce.clone(),
        active_exceptions: exceptions.exceptions.clone(),
        duration_ms,
    })
}

/// Evaluates a single file against active contract rules and built-in guards.
fn evaluate_file_guards(
    project_root: &Path,
    file: &Path,
    contract: &ArchitectureContract,
    _catalog: &GuardCatalog,
    exceptions: &ProjectExceptions,
) -> Vec<Violation> {
    let mut violations = Vec::new();
    let content = match fs::read_to_string(file) {
        Ok(c) => c,
        Err(_) => return violations,
    };

    let rel_file = file.strip_prefix(project_root).unwrap_or(file);
    let rel_str = rel_file.to_string_lossy();
    let is_test = rel_str.contains("test") || rel_str.contains("tests/");

    // 1. Complexity: source_limits
    if contract.enforce.iter().any(|g| g == "source_limits" || g == "small_files_limit") {
        let max_lines = contract
            .guard_settings
            .get("source_limits")
            .and_then(|v| v.get("max_lines"))
            .and_then(|v| v.as_integer())
            .unwrap_or(400) as usize;

        let code_lines = count_code_lines(&content);
        if code_lines > max_lines && !has_valid_exception(file, "complexity/source-limits", &content, exceptions) {
            violations.push(Violation {
                guard_id: "complexity/source-limits".to_string(),
                file: rel_file.to_path_buf(),
                line: Some(1),
                message: format!("File has {code_lines} code lines (exceeds limit of {max_lines})"),
                severity: Severity::Error,
                fix_suggestion: Some("Split module into smaller cohesive submodules under a directory module.".to_string()),
                rule_reference: Some(".planning/ARCHITECTURE.md [enforce: source_limits]".to_string()),
            });
        }
    }

    // 2. Language/Rust: no_unwrap
    if !is_test && contract.enforce.iter().any(|g| g == "no_unwrap" || g == "forbid_unwrap") {
        let unwraps = find_unwrap_expect_calls(&content);
        for (line, msg) in unwraps {
            if !has_valid_exception(file, "languages/rust/no-unwrap", &content, exceptions) {
                violations.push(Violation {
                    guard_id: "languages/rust/no-unwrap".to_string(),
                    file: rel_file.to_path_buf(),
                    line: Some(line),
                    message: msg,
                    severity: Severity::Error,
                    fix_suggestion: Some("Use '?' error propagation with thiserror or return a Result<T, E>.".to_string()),
                    rule_reference: Some(".planning/ARCHITECTURE.md [enforce: no_unwrap]".to_string()),
                });
            }
        }
    }

    // 3. Hygiene: no_debug_prints
    if !is_test && contract.enforce.iter().any(|g| g == "no_debug_prints") {
        let prints = find_debug_prints(&content);
        for (line, msg) in prints {
            if !has_valid_exception(file, "hygiene/no-debug-prints", &content, exceptions) {
                violations.push(Violation {
                    guard_id: "hygiene/no-debug-prints".to_string(),
                    file: rel_file.to_path_buf(),
                    line: Some(line),
                    message: msg,
                    severity: Severity::Error,
                    fix_suggestion: Some("Replace debug print with structured tracing::info/debug or remove before committing.".to_string()),
                    rule_reference: Some(".planning/ARCHITECTURE.md [enforce: no_debug_prints]".to_string()),
                });
            }
        }
    }

    // 4. Structural: layer_dependencies
    if !contract.allowed_dependencies.is_empty() {
        for (source_layer, allowed) in &contract.allowed_dependencies {
            if rel_str.contains(&format!("src/{source_layer}/")) || rel_str.starts_with(&format!("{source_layer}/")) {
                for (line_idx, line) in content.lines().enumerate() {
                    for (target_layer, _) in &contract.allowed_dependencies {
                        if target_layer != source_layer && !allowed.contains(target_layer) {
                            let forbidden_pat = format!("crate::{target_layer}");
                            if line.contains(&forbidden_pat) && !line.trim_start().starts_with("//") {
                                if !has_valid_exception(file, "structural/layer-dependencies", &content, exceptions) {
                                    violations.push(Violation {
                                        guard_id: "structural/layer-dependencies".to_string(),
                                        file: rel_file.to_path_buf(),
                                        line: Some(line_idx + 1),
                                        message: format!("Illegal import of '{target_layer}' from '{source_layer}'"),
                                        severity: Severity::Error,
                                        fix_suggestion: Some(format!("Module '{source_layer}' cannot depend on '{target_layer}'. Refactor access through declared layer boundary.")),
                                        rule_reference: Some(format!(".planning/ARCHITECTURE.md [allowed_dependencies: {source_layer}]")),
                                    });
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    violations
}

/// Helper that checks for inline exception headers: `// codeguard-exception: token=...; guard=...;`
fn has_valid_exception(
    file: &Path,
    guard_id: &str,
    content: &str,
    exceptions: &ProjectExceptions,
) -> bool {
    for line in content.lines().take(15) {
        if line.contains("codeguard-exception:") && line.contains("token=") {
            // Extract token
            if let Some(token_part) = line.split("token=").nth(1) {
                let token = token_part.split(';').next().unwrap_or("").trim();
                if exceptions.is_exception_valid(file, guard_id, token) {
                    return true;
                }
            }
        }
    }
    false
}
