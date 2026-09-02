#[cfg(test)]
mod tests {
    use codeguards_mcp::error::CodeGuardsError;
    use codeguards_mcp::types::{GuardReport, Severity, Violation};
    use codeguards_mcp::util::{compute_exception_token, hash_project_path, validate_safe_path};
    use std::path::{Path, PathBuf};

    #[test]
    fn test_validate_safe_path() {
        let safe = Path::new("/tmp/test_project");
        assert!(validate_safe_path(safe).is_ok());

        let unsafe_path = Path::new("/root/.ssh/id_rsa");
        assert!(matches!(
            validate_safe_path(unsafe_path),
            Err(CodeGuardsError::SandboxViolation { .. })
        ));
    }

    #[test]
    fn test_hash_project_path_deterministic() {
        let path = Path::new("/root/projects/slugaudit");
        let hash1 = hash_project_path(path);
        let hash2 = hash_project_path(path);
        assert_eq!(hash1, hash2);
        assert_eq!(hash1.len(), 12);
    }

    #[test]
    fn test_compute_exception_token_deterministic() {
        let file = Path::new("src/server.rs");
        let token1 = compute_exception_token(file, "complexity/source-limits", "transport scaffolding");
        let token2 = compute_exception_token(file, "complexity/source-limits", "transport scaffolding");
        assert_eq!(token1, token2);
        assert_eq!(token1.len(), 5);
    }

    #[test]
    fn test_guard_report_pass_logic() {
        let mut report = GuardReport::default();
        assert!(report.is_pass());

        report.violations.push(Violation {
            guard_id: "hygiene/no-secrets".to_string(),
            file: PathBuf::from("src/config.rs"),
            line: Some(10),
            message: "Secret detected".to_string(),
            severity: Severity::Error,
            fix_suggestion: None,
            rule_reference: None,
        });

        assert!(!report.is_pass());
        assert_eq!(report.error_count(), 1);
        assert_eq!(report.warning_count(), 0);
    }
}
