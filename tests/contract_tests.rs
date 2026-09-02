#[cfg(test)]
mod tests {
    use codeguards_mcp::contract::{parse_frontmatter, validate_architecture};
    use codeguards_mcp::library::catalog::GuardCatalog;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn test_parse_frontmatter_valid() {
        let content = r#"+++
modules = ["server", "storage"]
layers = ["transport", "storage"]
enforce = ["no_unwrap", "source_limits"]

[allowed_dependencies]
server = ["storage"]
storage = []
+++

# Architecture Title
Some body text.
"#;

        let (contract, body) = parse_frontmatter(content).unwrap();
        assert_eq!(contract.modules, vec!["server", "storage"]);
        assert_eq!(contract.layers, vec!["transport", "storage"]);
        assert_eq!(contract.enforce, vec!["no_unwrap", "source_limits"]);
        assert_eq!(contract.allowed_dependencies.get("server").unwrap(), &vec!["storage"]);
        assert!(body.contains("Some body text."));
    }

    #[test]
    fn test_validate_architecture_with_missing_guards() {
        let dir = tempdir().unwrap();
        let plan_dir = dir.path().join(".planning");
        fs::create_dir_all(&plan_dir).unwrap();

        let arch_content = r#"+++
modules = ["server"]
enforce = ["custom_payment_rule"]
+++
"#;
        fs::write(plan_dir.join("ARCHITECTURE.md"), arch_content).unwrap();

        let catalog = GuardCatalog::default();
        let result = validate_architecture(dir.path(), &catalog).unwrap();

        assert!(!result.is_valid);
        assert_eq!(result.missing_guards, vec!["custom_payment_rule"]);
        assert_eq!(result.errors.len(), 1);
    }
}
