//! Catalog management, indexing, and anti-duplication registry for guard tests.

use crate::types::GuardTestDefinition;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::PathBuf;

/// Global catalog index stored at ~/.slugthug/codeguards/tests/catalog.json
#[derive(Debug, Clone, Default, Serialize, Deserialize, JsonSchema)]
pub struct GuardCatalog {
    pub version: String,
    pub total_tests: usize,
    pub tests: BTreeMap<String, GuardCatalogEntry>,
    pub alias_map: BTreeMap<String, String>,
}

/// Lightweight summary of a guard test in catalog.json.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct GuardCatalogEntry {
    pub id: String,
    pub name: String,
    pub category: String,
    pub summary: String,
    pub tags: Vec<String>,
    pub aliases: Vec<String>,
    pub file_path: PathBuf,
}

impl GuardCatalog {
    /// Builds a catalog from an array of guard test definitions.
    #[must_use]
    pub fn from_definitions(defs: &[(GuardTestDefinition, PathBuf)]) -> Self {
        let mut tests = BTreeMap::new();
        let mut alias_map = BTreeMap::new();

        for (def, path) in defs {
            let entry = GuardCatalogEntry {
                id: def.id.clone(),
                name: def.name.clone(),
                category: def.category.clone(),
                summary: def.summary.clone(),
                tags: def.tags.clone(),
                aliases: def.aliases.clone(),
                file_path: path.clone(),
            };

            // Register primary ID and name
            alias_map.insert(def.id.to_lowercase(), def.id.clone());
            alias_map.insert(def.name.to_lowercase(), def.id.clone());

            // Register all aliases
            for alias in &def.aliases {
                alias_map.insert(alias.to_lowercase(), def.id.clone());
            }

            tests.insert(def.id.clone(), entry);
        }

        Self {
            version: "1.0.0".to_string(),
            total_tests: tests.len(),
            tests,
            alias_map,
        }
    }

    /// Looks up a guard test by ID, name, or alias.
    #[must_use]
    pub fn resolve(&self, query: &str) -> Option<&GuardCatalogEntry> {
        let key = query.to_lowercase();
        let canonical_id = self.alias_map.get(&key)?;
        self.tests.get(canonical_id)
    }

    /// Checks if a proposed new test is a potential duplicate of an existing test.
    #[must_use]
    pub fn find_potential_duplicate(&self, name: &str, tags: &[String]) -> Option<String> {
        // Direct alias or name match
        if let Some(entry) = self.resolve(name) {
            return Some(format!(
                "Exact name/alias match with existing guard: '{}' ({})",
                entry.id, entry.summary
            ));
        }

        // Tag overlap match (>2 shared tags)
        for entry in self.tests.values() {
            let shared_tags: usize = tags
                .iter()
                .filter(|t| entry.tags.iter().any(|et| et.eq_ignore_ascii_case(t)))
                .count();
            if shared_tags >= 3 {
                return Some(format!(
                    "High tag overlap ({shared_tags} matching tags) with existing guard: '{}' ({})",
                    entry.id, entry.summary
                ));
            }
        }

        None
    }
}
