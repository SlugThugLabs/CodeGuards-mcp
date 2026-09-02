#[cfg(test)]
mod tests {
    use codeguards_mcp::storage::ProjectExceptions;
    use std::path::Path;
    use tempfile::tempdir;

    #[test]
    fn test_exception_lifecycle() {
        let dir = tempdir().unwrap();
        let project_path = dir.path();

        let mut exceptions = ProjectExceptions::load(project_path).unwrap();
        assert!(exceptions.exceptions.is_empty());

        let file = Path::new("src/server/transport.rs");
        let entry = exceptions
            .add_exception(file, "complexity/source-limits", "transport scaffolding")
            .unwrap();

        assert_eq!(entry.token.len(), 5);
        assert_eq!(exceptions.exceptions.len(), 1);

        // Verify lookup
        assert!(exceptions.is_exception_valid(file, "complexity/source-limits", &entry.token));
        // Wrong token fails
        assert!(!exceptions.is_exception_valid(file, "complexity/source-limits", "99999"));
        // Wrong guard fails
        assert!(!exceptions.is_exception_valid(file, "hygiene/no-secrets", &entry.token));
        // Wrong file fails
        assert!(!exceptions.is_exception_valid(
            Path::new("src/other.rs"),
            "complexity/source-limits",
            &entry.token
        ));

        // Revoke
        assert!(exceptions.revoke(&entry.token).unwrap());
        assert!(!exceptions.is_exception_valid(file, "complexity/source-limits", &entry.token));
    }
}
