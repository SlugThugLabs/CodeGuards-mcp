//! Parallel guard execution runner.
//!
//! Dynamically routes active contract rules through the guard-tests catalog.

use crate::analyzer::{
    count_code_lines, extract_imported_modules, find_debug_prints, find_unwrap_expect_calls,
};
use crate::contract::ArchitectureContract;
use crate::error::Result;
use crate::library::catalog::{GuardCatalog, GuardCatalogEntry};
use crate::storage::ProjectExceptions;
use crate::types::{GuardReport, Severity, Violation};
use rayon::prelude::*;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::Instant;

/// Runs all applicable guards across the target list of files.
///
/// # Errors
///
/// Every built-in guard reports violations inside the returned [`GuardReport`]
/// instead of failing, so this currently always returns `Ok`; the `Result` is
/// part of the guard-check contract so callers can propagate evaluation failures
/// introduced by future guards.
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

    // Saturating: elapsed milliseconds only overflow u64 after ~585 million years,
    // so this never actually clamps — it just removes a panic path.
    let duration_ms = u64::try_from(start.elapsed().as_millis()).unwrap_or(u64::MAX);

    let (passed_tests, unevaluated_tests) = classify_rules(contract, catalog);

    Ok(GuardReport {
        project_root: project_root.to_path_buf(),
        total_files_checked: files.len(),
        violations,
        passed_tests,
        unevaluated_tests,
        active_exceptions: exceptions.exceptions.clone(),
        duration_ms,
    })
}

/// Guard ids that have an implementation wired into the dispatcher in
/// [`evaluate_file_guards`].
///
/// This is the single source of truth for "can this guard actually run".
/// `tests/guard_registry_parity.rs` fails if it drifts from the built-in catalog.
pub const IMPLEMENTED_GUARDS: &[&str] = &[
    "complexity/source-limits",
    "languages/rust/no-unwrap",
    "hygiene/no-debug-prints",
    "structural/layer-dependencies",
];

/// Guard ids that are catalogued but have no handler yet.
///
/// Enabling one of these evaluates nothing. They are reported as unevaluated
/// rather than passed, so a missing implementation cannot look like a success.
pub const UNIMPLEMENTED_GUARDS: &[&str] = &[
    "hygiene/no-secrets",
    "hygiene/no-duplicates",
    "structural/docs-drift",
    "structural/manifest-dependencies",
    "languages/rust/unsafe-policy",
    "languages/rust/runtime-leak",
    "languages/rust/tracing-instrument",
];

/// Whether `guard_id` has a handler wired into the dispatcher.
#[must_use]
pub fn is_implemented(guard_id: &str) -> bool {
    IMPLEMENTED_GUARDS.contains(&guard_id)
}

/// Splits the contract's enforced rules into what a handler will evaluate and
/// what will be skipped.
///
/// `structural/layer-dependencies` has a handler but only runs when the contract
/// declares `allowed_dependencies`, so without those it is unevaluated too.
#[must_use]
fn classify_rules(
    contract: &ArchitectureContract,
    catalog: &GuardCatalog,
) -> (Vec<String>, Vec<String>) {
    let mut evaluated = Vec::new();
    let mut skipped = Vec::new();

    for rule in &contract.enforce {
        let Some(guard) = catalog.resolve(rule) else {
            continue;
        };
        let will_run = if guard.id == "structural/layer-dependencies" {
            !contract.allowed_dependencies.is_empty()
        } else {
            is_implemented(&guard.id)
        };
        if will_run {
            evaluated.push(rule.clone());
        } else {
            skipped.push(rule.clone());
        }
    }

    (evaluated, skipped)
}

/// Per-file state shared by the individual guard checks in [`evaluate_file_guards`].
struct FileContext<'a> {
    file: &'a Path,
    rel_file: &'a Path,
    rel_str: &'a str,
    content: &'a str,
    is_test: bool,
    rule_name: &'a str,
    guard: &'a GuardCatalogEntry,
    exceptions: &'a ProjectExceptions,
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
    let Ok(content) = fs::read_to_string(file) else {
        return violations;
    };

    let rel_file = file.strip_prefix(project_root).unwrap_or(file);
    let rel_str = rel_file.to_string_lossy();

    // Iterate through all active rules declared in contract.enforce
    for rule_name in &contract.enforce {
        let Some(guard) = catalog.resolve(rule_name) else {
            continue;
        };

        let ctx = FileContext {
            file,
            rel_file,
            rel_str: &rel_str,
            content: &content,
            is_test: rel_str.contains("test") || rel_str.contains("tests/"),
            rule_name,
            guard,
            exceptions,
        };

        match guard.id.as_str() {
            // ── Complexity: source_limits ──
            "complexity/source-limits" => check_source_limits(&ctx, contract, &mut violations),
            // ── Language/Rust: no_unwrap ──
            "languages/rust/no-unwrap" => check_no_unwrap(&ctx, &mut violations),
            // ── Hygiene: no_debug_prints ──
            "hygiene/no-debug-prints" => check_debug_prints(&ctx, &mut violations),
            // ── Structural: layer_dependencies ──
            "structural/layer-dependencies" if !contract.allowed_dependencies.is_empty() => {
                check_layer_dependencies(&ctx, contract, &mut violations);
            }
            _ => {}
        }
    }

    violations
}

/// Complexity guard: flags files whose code-line count exceeds the configured limit.
fn check_source_limits(
    ctx: &FileContext<'_>,
    contract: &ArchitectureContract,
    violations: &mut Vec<Violation>,
) {
    let max_lines = usize::try_from(
        contract
            .guard_settings
            .get("source_limits")
            .and_then(|v| v.get("max_lines"))
            .and_then(serde_json::Value::as_u64)
            .unwrap_or(400),
    )
    .ok()
    .unwrap_or(usize::MAX);

    let code_lines = count_code_lines(ctx.content);
    if code_lines > max_lines
        && !has_valid_exception(ctx.file, &ctx.guard.id, ctx.content, ctx.exceptions)
    {
        violations.push(Violation {
            guard_id: ctx.guard.id.clone(),
            file: ctx.rel_file.to_path_buf(),
            line: Some(1),
            message: format!("File has {code_lines} code lines (exceeds limit of {max_lines})"),
            severity: Severity::Error,
            fix_suggestion: Some(ctx.guard.summary.clone()),
            rule_reference: Some(format!(
                ".planning/ARCHITECTURE.md [enforce: {}]",
                ctx.rule_name
            )),
        });
    }
}

/// Rust language guard: flags bare `.unwrap()`/`.expect()` in non-test sources.
fn check_no_unwrap(ctx: &FileContext<'_>, violations: &mut Vec<Violation>) {
    if !ctx.is_test && ctx.rel_file.extension().is_some_and(|ext| ext == "rs") {
        for (line, msg) in find_unwrap_expect_calls(ctx.content) {
            if !has_valid_exception(ctx.file, &ctx.guard.id, ctx.content, ctx.exceptions) {
                violations.push(Violation {
                    guard_id: ctx.guard.id.clone(),
                    file: ctx.rel_file.to_path_buf(),
                    line: Some(line),
                    message: msg,
                    severity: Severity::Error,
                    fix_suggestion: Some(
                        "Use '?' error propagation with thiserror or return a Result<T, E>."
                            .to_string(),
                    ),
                    rule_reference: Some(format!(
                        ".planning/ARCHITECTURE.md [enforce: {}]",
                        ctx.rule_name
                    )),
                });
            }
        }
    }
}

/// Hygiene guard: flags leftover debug print statements in non-test sources.
fn check_debug_prints(ctx: &FileContext<'_>, violations: &mut Vec<Violation>) {
    if !ctx.is_test {
        for (line, msg) in find_debug_prints(ctx.content) {
            if !has_valid_exception(ctx.file, &ctx.guard.id, ctx.content, ctx.exceptions) {
                violations.push(Violation {
                    guard_id: ctx.guard.id.clone(),
                    file: ctx.rel_file.to_path_buf(),
                    line: Some(line),
                    message: msg,
                    severity: Severity::Error,
                    fix_suggestion: Some(
                        "Replace debug print with structured tracing::info/debug or remove before committing."
                            .to_string(),
                    ),
                    rule_reference: Some(format!(
                        ".planning/ARCHITECTURE.md [enforce: {}]",
                        ctx.rule_name
                    )),
                });
            }
        }
    }
}

/// Structural guard: flags imports that cross a declared layer boundary.
fn check_layer_dependencies(
    ctx: &FileContext<'_>,
    contract: &ArchitectureContract,
    violations: &mut Vec<Violation>,
) {
    let imported_modules = extract_imported_modules(ctx.content);
    for (source_layer, allowed) in &contract.allowed_dependencies {
        if ctx.rel_str.contains(&format!("src/{source_layer}/"))
            || ctx.rel_str.starts_with(&format!("{source_layer}/"))
        {
            for (line_num, import_path) in &imported_modules {
                for target_layer in contract.allowed_dependencies.keys() {
                    let crosses_boundary = target_layer != source_layer
                        && !allowed.contains(target_layer)
                        && (import_path.contains(&format!("crate::{target_layer}"))
                            || import_path.starts_with(target_layer));
                    if crosses_boundary
                        && !has_valid_exception(
                            ctx.file,
                            &ctx.guard.id,
                            ctx.content,
                            ctx.exceptions,
                        )
                    {
                        violations.push(Violation {
                            guard_id: ctx.guard.id.clone(),
                            file: ctx.rel_file.to_path_buf(),
                            line: Some(*line_num),
                            message: format!(
                                "Illegal import of '{target_layer}' from '{source_layer}' (import: '{import_path}')"
                            ),
                            severity: Severity::Error,
                            fix_suggestion: Some(format!(
                                "Module '{source_layer}' cannot depend on '{target_layer}'. Refactor access through declared layer boundary."
                            )),
                            rule_reference: Some(format!(
                                ".planning/ARCHITECTURE.md [allowed_dependencies: {source_layer}]"
                            )),
                        });
                    }
                }
            }
        }
    }
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
        if line.contains("codeguard-exception:")
            && line.contains("token=")
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
