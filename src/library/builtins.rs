//! Built-in guard test definitions embedded directly into the binary.

use crate::types::GuardTestDefinition;
use std::collections::BTreeMap;

/// Returns all standard built-in guard tests.
#[must_use]
pub fn get_builtin_guard_tests() -> Vec<GuardTestDefinition> {
    vec![
        // ── Structural Guards ──
        GuardTestDefinition {
            id: "structural/docs-drift".to_string(),
            name: "docs_drift".to_string(),
            category: "structural".to_string(),
            version: "1.0.0".to_string(),
            summary: "Verifies bidirectional agreement between ARCHITECTURE.md and codebase reality"
                .to_string(),
            tags: vec![
                "architecture".to_string(),
                "docs".to_string(),
                "drift".to_string(),
                "module-map".to_string(),
            ],
            aliases: vec!["docs_drift".to_string(), "architecture_drift".to_string()],
            engine: "docs_drift".to_string(),
            default_params: BTreeMap::new(),
            remediation: "Update .planning/ARCHITECTURE.md or remove unlisted rogue files in src/."
                .to_string(),
        },
        GuardTestDefinition {
            id: "structural/layer-dependencies".to_string(),
            name: "layer_dependencies".to_string(),
            category: "structural".to_string(),
            version: "1.0.0".to_string(),
            summary: "Enforces strict DAG import hierarchy based on allowed_dependencies".to_string(),
            tags: vec![
                "layers".to_string(),
                "imports".to_string(),
                "dependencies".to_string(),
                "dag".to_string(),
            ],
            aliases: vec![
                "layer_dependencies".to_string(),
                "forbidden_imports".to_string(),
                "dependency_guard".to_string(),
            ],
            engine: "layer_dependencies".to_string(),
            default_params: BTreeMap::new(),
            remediation: "Refactor imported dependency through its declared public interface or update layer bounds in ARCHITECTURE.md.".to_string(),
        },
        GuardTestDefinition {
            id: "structural/manifest-dependencies".to_string(),
            name: "manifest_dependencies".to_string(),
            category: "structural".to_string(),
            version: "1.0.0".to_string(),
            summary: "Verifies Cargo.toml / package.json dependency whitelists and layer bounds".to_string(),
            tags: vec!["cargo".to_string(), "manifest".to_string(), "dependencies".to_string()],
            aliases: vec!["manifest_dependencies".to_string(), "crate_dependencies".to_string()],
            engine: "manifest_dependencies".to_string(),
            default_params: BTreeMap::new(),
            remediation: "Remove forbidden external crate from Cargo.toml or move logic to the appropriate infrastructure layer.".to_string(),
        },

        // ── Complexity Guards ──
        GuardTestDefinition {
            id: "complexity/source-limits".to_string(),
            name: "source_limits".to_string(),
            category: "complexity".to_string(),
            version: "1.0.0".to_string(),
            summary: "Enforces strict production code line ceiling per file (default 400 lines)".to_string(),
            tags: vec!["loc".to_string(), "lines".to_string(), "file-size".to_string(), "small-files".to_string()],
            aliases: vec![
                "source_limits".to_string(),
                "file_size_guard".to_string(),
                "small_files_limit".to_string(),
                "max_file_lines".to_string(),
            ],
            engine: "source_limits".to_string(),
            default_params: {
                let mut p = BTreeMap::new();
                p.insert("max_lines".to_string(), serde_json::json!(400));
                p
            },
            remediation: "Split large module into focused cohesive submodules under a directory module.".to_string(),
        },

        // ── Hygiene Guards ──
        GuardTestDefinition {
            id: "hygiene/no-secrets".to_string(),
            name: "no_secrets".to_string(),
            category: "hygiene".to_string(),
            version: "1.0.0".to_string(),
            summary: "Detects hardcoded API keys, private tokens, and credentials".to_string(),
            tags: vec!["security".to_string(), "secrets".to_string(), "tokens".to_string(), "keys".to_string()],
            aliases: vec!["no_secrets".to_string(), "no_hardcoded_secrets".to_string(), "secret_scanner".to_string()],
            engine: "no_secrets".to_string(),
            default_params: BTreeMap::new(),
            remediation: "Extract hardcoded secret into environment variable or configuration file.".to_string(),
        },
        GuardTestDefinition {
            id: "hygiene/no-duplicates".to_string(),
            name: "no_duplicates".to_string(),
            category: "hygiene".to_string(),
            version: "1.0.0".to_string(),
            summary: "Disallows duplicate test function names and duplicated git commit subjects".to_string(),
            tags: vec!["dedupe".to_string(), "tests".to_string(), "git".to_string()],
            aliases: vec!["no_duplicates".to_string(), "check_no_duplicates".to_string()],
            engine: "no_duplicates".to_string(),
            default_params: BTreeMap::new(),
            remediation: "Rename duplicate test function to describe specific unique behavior.".to_string(),
        },
        GuardTestDefinition {
            id: "hygiene/no-debug-prints".to_string(),
            name: "no_debug_prints".to_string(),
            category: "hygiene".to_string(),
            version: "1.0.0".to_string(),
            summary: "Rejects leftover debugging statements in production".to_string(),
            tags: vec!["logging".to_string(), "debug".to_string(), "prints".to_string()],
            aliases: vec!["no_debug_prints".to_string(), "debug_statements".to_string()],
            engine: "no_debug_prints".to_string(),
            default_params: BTreeMap::new(),
            remediation: "Replace debug print statement with structured tracing or remove it before committing.".to_string(),
        },

        // ── Rust Language Guards ──
        GuardTestDefinition {
            id: "languages/rust/no-unwrap".to_string(),
            name: "no_unwrap".to_string(),
            category: "languages/rust".to_string(),
            version: "1.0.0".to_string(),
            summary: "Prohibits bare .unwrap() and .expect() calls in production Rust source code".to_string(),
            tags: vec!["rust".to_string(), "errors".to_string(), "unwrap".to_string(), "safety".to_string()],
            aliases: vec!["no_unwrap".to_string(), "forbid_unwrap".to_string()],
            engine: "rust_no_unwrap".to_string(),
            default_params: BTreeMap::new(),
            remediation: "Use ? error propagation with thiserror or return a structured Result<T, E>.".to_string(),
        },
        GuardTestDefinition {
            id: "languages/rust/tracing-instrument".to_string(),
            name: "tracing_instrument".to_string(),
            category: "languages/rust".to_string(),
            version: "1.0.0".to_string(),
            summary: "Requires #[tracing::instrument] on public API functions".to_string(),
            tags: vec!["rust".to_string(), "tracing".to_string(), "observability".to_string()],
            aliases: vec!["tracing_instrument".to_string(), "require_tracing".to_string()],
            engine: "rust_tracing_instrument".to_string(),
            default_params: BTreeMap::new(),
            remediation: "Add #[tracing::instrument] above public entrypoint function.".to_string(),
        },
        GuardTestDefinition {
            id: "languages/rust/unsafe-policy".to_string(),
            name: "unsafe_policy".to_string(),
            category: "languages/rust".to_string(),
            version: "1.0.0".to_string(),
            summary: "Enforces #![forbid(unsafe_code)] or strict bounds on unsafe blocks".to_string(),
            tags: vec!["rust".to_string(), "unsafe".to_string(), "security".to_string()],
            aliases: vec!["unsafe_policy".to_string(), "forbid_unsafe".to_string()],
            engine: "rust_unsafe_policy".to_string(),
            default_params: BTreeMap::new(),
            remediation: "Remove unsafe block or add crate-level #![forbid(unsafe_code)] directive.".to_string(),
        },
        GuardTestDefinition {
            id: "languages/rust/runtime-leak".to_string(),
            name: "runtime_leak".to_string(),
            category: "languages/rust".to_string(),
            version: "1.0.0".to_string(),
            summary: "Prevents async runtime crates (tokio, wasmtime, rayon) from leaking into pure domain/contracts".to_string(),
            tags: vec!["rust".to_string(), "tokio".to_string(), "leak".to_string(), "architecture".to_string()],
            aliases: vec!["runtime_leak".to_string(), "tokio_leak_guard".to_string()],
            engine: "rust_runtime_leak".to_string(),
            default_params: BTreeMap::new(),
            remediation: "Isolate async runtime calls into the transport or infrastructure layer.".to_string(),
        },
    ]
}
