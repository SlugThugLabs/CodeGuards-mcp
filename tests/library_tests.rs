#[cfg(test)]
mod tests {
    use codeguards_mcp::library::{create_custom_guard_test, load_all_tests_and_save_catalog};
    use codeguards_mcp::types::GuardTestDefinition;
    use std::collections::BTreeMap;
    use tempfile::tempdir;

    #[test]
    fn test_catalog_resolution_and_aliases() {
        let dir = tempdir().unwrap();
        let tests_root = dir.path();

        let def = GuardTestDefinition {
            id: "complexity/source-limits".to_string(),
            name: "source_limits".to_string(),
            category: "complexity".to_string(),
            version: "1.0.0".to_string(),
            summary: "Max line ceiling".to_string(),
            tags: vec!["lines".to_string(), "size".to_string()],
            aliases: vec!["small_files_limit".to_string(), "max_file_lines".to_string()],
            engine: "source_limits".to_string(),
            default_params: BTreeMap::new(),
            remediation: "Split file".to_string(),
        };

        create_custom_guard_test(tests_root, def, false).unwrap();

        let catalog = load_all_tests_and_save_catalog(tests_root).unwrap();
        assert_eq!(catalog.total_tests, 1);

        // Resolve by ID
        assert!(catalog.resolve("complexity/source-limits").is_some());
        // Resolve by name
        assert!(catalog.resolve("source_limits").is_some());
        // Resolve by alias
        assert!(catalog.resolve("small_files_limit").is_some());
        assert!(catalog.resolve("max_file_lines").is_some());
    }

    #[test]
    fn test_duplicate_prevention() {
        let dir = tempdir().unwrap();
        let tests_root = dir.path();

        let def1 = GuardTestDefinition {
            id: "hygiene/no-secrets".to_string(),
            name: "no_secrets".to_string(),
            category: "hygiene".to_string(),
            version: "1.0.0".to_string(),
            summary: "Detects secrets".to_string(),
            tags: vec!["security".to_string(), "secrets".to_string(), "keys".to_string()],
            aliases: vec!["secret_scanner".to_string()],
            engine: "no_secrets".to_string(),
            default_params: BTreeMap::new(),
            remediation: "Extract secret".to_string(),
        };

        create_custom_guard_test(tests_root, def1, false).unwrap();

        // Attempting to create duplicate by alias should fail
        let def2 = GuardTestDefinition {
            id: "hygiene/secret-scanner".to_string(),
            name: "secret_scanner".to_string(),
            category: "hygiene".to_string(),
            version: "1.0.0".to_string(),
            summary: "Scans secrets".to_string(),
            tags: vec!["security".to_string(), "secrets".to_string(), "keys".to_string()],
            aliases: vec![],
            engine: "no_secrets".to_string(),
            default_params: BTreeMap::new(),
            remediation: "Extract secret".to_string(),
        };

        let result = create_custom_guard_test(tests_root, def2, false);
        assert!(result.is_err());
    }
}
