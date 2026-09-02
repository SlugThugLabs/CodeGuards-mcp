#[cfg(test)]
mod tests {
    use codeguards_mcp::analyzer::extract_imported_modules;
    use codeguards_mcp::contract::ArchitectureContract;
    use codeguards_mcp::guards::run_guard_checks;
    use codeguards_mcp::library::catalog::GuardCatalog;
    use codeguards_mcp::storage::ProjectExceptions;
    use std::collections::BTreeMap;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn test_extract_imported_modules_multi_language() {
        let rust_code = r#"
// use crate::ignored;
use crate::storage::disk;
use crate::server::transport;
"#;
        let imports = extract_imported_modules(rust_code);
        assert_eq!(imports.len(), 2);
        assert!(imports[0].1.contains("crate::storage::disk"));

        let py_code = r#"
# import ignored
import os
from database import connection
"#;
        let py_imports = extract_imported_modules(py_code);
        assert_eq!(py_imports.len(), 2);
        assert_eq!(py_imports[0].1, "os");
        assert_eq!(py_imports[1].1, "database");
    }

    #[test]
    fn test_dynamic_catalog_rule_dispatch_and_layer_violation() {
        let dir = tempdir().unwrap();
        let src_transport = dir.path().join("src").join("transport");
        fs::create_dir_all(&src_transport).unwrap();

        // Transport illegally importing storage
        let file = src_transport.join("api.rs");
        fs::write(&file, "use crate::storage::db;\npub fn handle() {}\n").unwrap();

        let mut allowed = BTreeMap::new();
        allowed.insert("transport".to_string(), vec!["domain".to_string()]); // storage not allowed!
        allowed.insert("storage".to_string(), vec![]);

        let mut contract = ArchitectureContract::default();
        contract.enforce = vec!["layer_dependencies".to_string()];
        contract.allowed_dependencies = allowed;

        let catalog = codeguards_mcp::library::builtins::get_builtin_guard_tests();
        let catalog = GuardCatalog::from_definitions(
            &catalog
                .into_iter()
                .map(|d| (d, std::path::PathBuf::new()))
                .collect::<Vec<_>>(),
        );

        let exceptions = ProjectExceptions::default();
        let report = run_guard_checks(dir.path(), &[file.clone()], &contract, &catalog, &exceptions).unwrap();

        assert!(!report.is_pass());
        assert_eq!(report.violations.len(), 1);
        assert_eq!(report.violations[0].guard_id, "structural/layer-dependencies");
        assert!(report.violations[0].message.contains("Illegal import of 'storage'"));
    }
}
