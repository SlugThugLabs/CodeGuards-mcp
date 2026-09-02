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

#[cfg(test)]
mod tests {
    use super::*;

    fn def(id: &str, name: &str, tags: &[&str], aliases: &[&str]) -> GuardTestDefinition {
        GuardTestDefinition {
            id: id.to_string(),
            name: name.to_string(),
            category: "structural".to_string(),
            version: "1.0.0".to_string(),
            summary: "test guard".to_string(),
            tags: tags.iter().map(|s| s.to_string()).collect(),
            aliases: aliases.iter().map(|s| s.to_string()).collect(),
            engine: "builtin".to_string(),
            default_params: BTreeMap::new(),
            remediation: "fix it".to_string(),
        }
    }

    fn catalog(defs: &[GuardTestDefinition]) -> GuardCatalog {
        let defs: Vec<(GuardTestDefinition, PathBuf)> =
            defs.iter().map(|d| (d.clone(), PathBuf::from("/x"))).collect();
        GuardCatalog::from_definitions(&defs)
    }

    #[test]
    fn resolve_by_id_name_and_alias_case_insensitive() {
        let defs = vec![def("structural/layer-deps", "layer-deps", &["layers"], &["layers", "deps"])];
        let cat = catalog(&defs);
        assert!(cat.resolve("structural/layer-deps").is_some());
        assert!(cat.resolve("LAYER-DEPS").is_some(), "name lookup must be case-insensitive");
        assert!(cat.resolve("DEPS").is_some(), "alias lookup must be case-insensitive");
        assert_eq!(cat.resolve("deps").unwrap().id, "structural/layer-deps");
    }

    #[test]
    fn resolve_unknown_query_returns_none() {
        let defs = vec![def("structural/x", "x", &[], &[])];
        let cat = catalog(&defs);
        assert!(cat.resolve("nonexistent").is_none());
    }

    #[test]
    fn duplicate_exact_name_match_detected() {
        let defs = vec![def("structural/layers", "layers", &["layers"], &["layer-deps"])];
        let cat = catalog(&defs);
        assert!(cat.find_potential_duplicate("layers", &[]).is_some(), "exact name match");
        assert!(
            cat.find_potential_duplicate("LAYER-DEPS", &[]).is_some(),
            "alias match must be detected"
        );
    }

    #[test]
    fn duplicate_tag_overlap_detected_at_three_shared_tags() {
        let defs = vec![def("structural/layers", "layers", &["layers", "imports", "deps"], &[])];
        let cat = catalog(&defs);
        // Three shared tags -> duplicate.
        let tags = vec!["LAYERS".to_string(), "imports".to_string(), "deps".to_string()];
        assert!(cat.find_potential_duplicate("fresh-name", &tags).is_some());
        // Two shared tags -> not a duplicate.
        let tags = vec!["LAYERS".to_string(), "imports".to_string()];
        assert!(cat.find_potential_duplicate("fresh-name", &tags).is_none());
    }

    #[test]
    fn duplicate_none_when_distinct() {
        let defs = vec![def("structural/layers", "layers", &["layers", "imports"], &[])];
        let cat = catalog(&defs);
        let tags = vec!["secrets".to_string(), "crypto".to_string(), "tokens".to_string()];
        assert!(cat.find_potential_duplicate("secret-scanner", &tags).is_none());
    }
}
