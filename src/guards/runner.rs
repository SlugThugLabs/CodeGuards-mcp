//! Parallel guard execution runner.
//!
//! Dynamically routes active contract rules through the guard-tests catalog.

use crate::analyzer::{count_code_lines, extract_imported_modules, find_debug_prints, find_unwrap_expect_calls};
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

    let duration_ms = u64::try_from(start.elapsed().as_millis())
        .expect("duration should fit in u64");

    Ok(GuardReport {
        project_root: project_root.to_path_buf(),
        total_files_checked: files.len(),
        violations,
        passed_tests: contract.enforce.clone(),
        active_exceptions: exceptions.exceptions.clone(),
        duration_ms,
    })
}

/// Evaluates a single file against active contract rules and catalog definitions.
fn evaluate_file_guards(
    project_root: &Path,
    file: &Path,
    contract: &ArchitectureContract,
    catalog: &GuardCatalog,
    exceptions: &ProjectExceptions,
) -> Vec<Violation> {
    let mut violations = Vec::new();
    let Ok(content) = fs::read_to_string(file) else { return violations };

    let rel_file = file.strip_prefix(project_root).unwrap_or(file);
    let rel_str = rel_file.to_string_lossy();
    let is_test = rel_str.contains("test") || rel_str.contains("tests/");

    // Iterate through all active rules declared in contract.enforce
    for rule_name in &contract.enforce {
        let Some(guard_entry) = catalog.resolve(rule_name) else { continue };

        match guard_entry.id.as_str() {
            // ── Complexity: source_limits ──
            "complexity/source-limits" => {
                let max_lines = usize::try_from(contract
                  .guard_settings
                  .get("source_limits")
                  .and_then(|v| v.get("max_lines"))
                  .and_then(|v| v.as_u64())
                  .unwrap_or(400))
                  .expect("max_lines should fit in usize");

                let code_lines = count_code_lines(&content);
                if code_lines > max_lines && !has_valid_exception(file, &guard_entry.id, &content, exceptions) {
                    violations.push(Violation {
                        guard_id: guard_entry.id.clone(),
                        file: rel_file.to_path_buf(),
                        line: Some(1),
                        message: format!("File has {code_lines} code lines (exceeds limit of {max_lines})"),
                        severity: Severity::Error,
                        fix_suggestion: Some(guard_entry.summary.clone()),
                        rule_reference: Some(format!(".planning/ARCHITECTURE.md [enforce: {rule_name}]")),
                    });
                }
            }

            // ── Language/Rust: no_unwrap ──
            "languages/rust/no-unwrap" => {
                if !is_test && rel_str.ends_with(".rs") {
                    let unwraps = find_unwrap_expect_calls(&content);
                    for (line, msg) in unwraps {
                        if !has_valid_exception(file, &guard_entry.id, &content, exceptions) {
                            violations.push(Violation {
                                guard_id: guard_entry.id.clone(),
                                file: rel_file.to_path_buf(),
                                line: Some(line),
                                message: msg,
                                severity: Severity::Error,
                                fix_suggestion: Some("Use '?' error propagation with thiserror or return a Result<T, E>.".to_string()),
                                rule_reference: Some(format!(".planning/ARCHITECTURE.md [enforce: {rule_name}]")),
                            });
                        }
                    }
                }
            }

            // ── Hygiene: no_debug_prints ──
            "hygiene/no-debug-prints" => {
                if !is_test {
                    let prints = find_debug_prints(&content);
                    for (line, msg) in prints {
                        if !has_valid_exception(file, &guard_entry.id, &content, exceptions) {
                            violations.push(Violation {
                                guard_id: guard_entry.id.clone(),
                                file: rel_file.to_path_buf(),
                                line: Some(line),
                                message: msg,
                                severity: Severity::Error,
                                fix_suggestion: Some("Replace debug print with structured tracing::info/debug or remove before committing.".to_string()),
                                rule_reference: Some(format!(".planning/ARCHITECTURE.md [enforce: {rule_name}]")),
                            });
                        }
                    }
                }
            }

            // ── Structural: layer_dependencies ──
            "structural/layer-dependencies" => {
                if !contract.allowed_dependencies.is_empty() {
                    let imported_modules = extract_imported_modules(&content);
                    for (source_layer, allowed) in &contract.allowed_dependencies {
                        if rel_str.contains(&format!("src/{source_layer}/")) || rel_str.starts_with(&format!("{source_layer}/")) {
                            for (line_num, import_path) in &imported_modules {
                                for target_layer in contract.allowed_dependencies.keys() {
                                    if target_layer != source_layer && !allowed.contains(target_layer) {
                                        let target_match = format!("crate::{target_layer}");
                                        if (import_path.contains(&target_match) || import_path.starts_with(target_layer))
                                            && !has_valid_exception(file, &guard_entry.id, &content, exceptions)
                                        {
                                            violations.push(Violation {
                                                guard_id: guard_entry.id.clone(),
                                                file: rel_file.to_path_buf(),
                                                line: Some(*line_num),
                                                message: format!("Illegal import of '{target_layer}' from '{source_layer}' (import: '{import_path}')"),
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

            _ => {}
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
    // Scan up to first 50 lines to account for long license headers
    for line in content.lines().take(50) {
        if line.contains("codeguard-exception:") && line.contains("token=")
            && let Some(token_part) = line.split("token=").nth(1)
        {
            let token = token_part.split(';').next().unwrap_or("").trim();
            if exceptions.is_exception_valid(file, guard_id, token) {
                return true;
            }
        }
    }
    false
}
