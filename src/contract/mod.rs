//! Architecture contract parser and validator for .planning/ARCHITECTURE.md.

use crate::error::{CodeGuardsError, Result};
use crate::library::catalog::GuardCatalog;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

/// TOML frontmatter extracted from .planning/ARCHITECTURE.md.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, JsonSchema)]
pub struct ArchitectureContract {
    #[serde(default)]
    pub modules: Vec<String>,
    #[serde(default)]
    pub layers: Vec<String>,
    #[serde(default)]
    pub allowed_dependencies: BTreeMap<String, Vec<String>>,
    #[serde(default)]
    pub enforce: Vec<String>,
    #[serde(default)]
    pub guard_settings: BTreeMap<String, serde_json::Value>,
}

/// Validation verdict returned by validate_architecture.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ValidationResult {
    pub is_valid: bool,
    pub contract: Option<ArchitectureContract>,
    pub ready_guards: Vec<String>,
    pub missing_guards: Vec<String>,
    pub errors: Vec<String>,
    pub warnings: Vec<String>,
}

/// Extracts TOML frontmatter delimited by `+++` fences from markdown content.
pub fn parse_frontmatter(content: &str) -> Result<(ArchitectureContract, &str)> {
    let trimmed = content.trim_start();
    if !trimmed.starts_with("+++") {
        return Ok((ArchitectureContract::default(), content));
    }

    let rest = &trimmed[3..];
    let end_idx = rest
        .find("+++")
        .ok_or_else(|| CodeGuardsError::Contract("Unclosed '+++' TOML frontmatter fence in ARCHITECTURE.md".to_string()))?;

    let toml_str = &rest[..end_idx];
    let body = &rest[end_idx + 3..];

    let contract: ArchitectureContract = toml::from_str(toml_str).map_err(|e| CodeGuardsError::TomlParse {
        path: PathBuf::from(".planning/ARCHITECTURE.md"),
        source: e,
    })?;

    Ok((contract, body))
}

/// Loads and parses .planning/ARCHITECTURE.md from a project directory.
pub fn load_architecture(project_root: &Path) -> Result<ArchitectureContract> {
    let arch_file = project_root.join(".planning").join("ARCHITECTURE.md");
    if !arch_file.exists() {
        return Err(CodeGuardsError::Contract(format!(
            "Missing architecture contract at {}",
            arch_file.display()
        )));
    }

    let content = fs::read_to_string(&arch_file).map_err(|e| CodeGuardsError::Io {
        path: arch_file,
        source: e,
    })?;

    let (contract, _) = parse_frontmatter(&content)?;
    Ok(contract)
}

/// Validates .planning/ARCHITECTURE.md against disk reality and the guard-test catalog.
pub fn validate_architecture(
    project_root: &Path,
    catalog: &GuardCatalog,
) -> Result<ValidationResult> {
    let mut errors = Vec::new();
    let mut warnings = Vec::new();
    let mut ready_guards = Vec::new();
    let mut missing_guards = Vec::new();

    let contract = match load_architecture(project_root) {
        Ok(c) => c,
        Err(e) => {
            errors.push(format!("Failed to load ARCHITECTURE.md: {e}"));
            return Ok(ValidationResult {
                is_valid: false,
                contract: None,
                ready_guards,
                missing_guards,
                errors,
                warnings,
            });
        }
    };

    // 1. Verify declared modules exist under src/
    let src_dir = project_root.join("src");
    if src_dir.is_dir() {
        for module in &contract.modules {
            let rs_file = src_dir.join(format!("{module}.rs"));
            let mod_dir = src_dir.join(module);
            if !rs_file.exists() && !mod_dir.exists() {
                warnings.push(format!(
                    "Declared module '{module}' does not exist on disk under src/ ({rs_file:?} or {mod_dir:?})"
                ));
            }
        }
    }

    // 2. Cross-check enforce items against catalog
    for enforce_item in &contract.enforce {
        if let Some(entry) = catalog.resolve(enforce_item) {
            ready_guards.push(entry.id.clone());
        } else {
            missing_guards.push(enforce_item.clone());
            errors.push(format!(
                "Required guard test '{enforce_item}' is not found in ~/.slugthug/codeguards/tests/. Use create_guard_test to generate it."
            ));
        }
    }

    let is_valid = errors.is_empty();

    Ok(ValidationResult {
        is_valid,
        contract: Some(contract),
        ready_guards,
        missing_guards,
        errors,
        warnings,
    })
}
