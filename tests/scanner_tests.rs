#[cfg(test)]
mod tests {
    use codeguards_mcp::analyzer::{count_code_lines, find_debug_prints, find_unwrap_expect_calls};
    use codeguards_mcp::contract::ArchitectureContract;
    use codeguards_mcp::guards::run_guard_checks;
    use codeguards_mcp::library::catalog::GuardCatalog;
    use codeguards_mcp::storage::ProjectExceptions;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn test_tokenizer_code_line_count() {
        let code = r#"
// This is a comment
/* Multiline
   Comment */
fn main() {
    let x = 42; // inline comment
    let s = "string with // comment inside";
}
"#;
        assert_eq!(count_code_lines(code), 4);
    }

    #[test]
    fn test_tokenizer_unwrap_detection() {
        let code = r#"
fn test_unwraps() {
    // let c = opt.unwrap();
    let real = opt.unwrap();
    let s = "str.unwrap()";
}
"#;
        let unwraps = find_unwrap_expect_calls(code);
        assert_eq!(unwraps.len(), 1);
        assert_eq!(unwraps[0].0, 4); // Line 4 is the real unwrap
    }

    #[test]
    fn test_tokenizer_debug_print_detection() {
        let code = r#"
fn run() {
    // dbg!("ignored");
    console.log("real debug print");
    dbg!(x);
}
"#;
        let prints = find_debug_prints(code);
        assert_eq!(prints.len(), 2);
    }

    #[test]
    fn test_guard_runner_full_pass_and_fail() {
        let dir = tempdir().unwrap();
        let src_dir = dir.path().join("src");
        fs::create_dir_all(&src_dir).unwrap();

        let clean_file = src_dir.join("clean.rs");
        fs::write(&clean_file, "pub fn clean() -> Result<(), ()> { Ok(()) }\n").unwrap();

        let dirty_file = src_dir.join("dirty.rs");
        fs::write(&dirty_file, "pub fn dirty() { let x = Some(1).unwrap(); }\n").unwrap();

        let contract = ArchitectureContract {
            enforce: vec!["no_unwrap".to_string(), "source_limits".to_string()],
            ..Default::default()
        };

        let builtins = codeguards_mcp::library::builtins::get_builtin_guard_tests();
        let catalog = GuardCatalog::from_definitions(
            &builtins
                .into_iter()
                .map(|d| (d, std::path::PathBuf::new()))
                .collect::<Vec<_>>(),
        );
        let exceptions = ProjectExceptions::default();

        let report = run_guard_checks(dir.path(), &[clean_file, dirty_file], &contract, &catalog, &exceptions).unwrap();

        assert!(!report.is_pass());
        assert_eq!(report.violations.len(), 1);
        assert_eq!(report.violations[0].guard_id, "languages/rust/no-unwrap");
    }
}
