//! Unified pure-Rust code scanner and tokenizer.
//!
//! Strips comments, docstrings, and string literals in a single clean pass.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum LexState {
    Normal,
    LineComment,
    BlockComment,
    StringLiteral { escaped: bool },
    CharLiteral { escaped: bool },
}

/// A line in a source file, split into raw content and code-only content (comments/strings stripped).
#[derive(Debug, Clone)]
pub struct StrippedLine {
    pub line_number: usize,
    pub raw: String,
    pub code_only: String,
    pub has_code_token: bool,
}

/// Tokenizes source text and strips all non-executable tokens (comments, strings).
pub fn tokenize_source(source: &str) -> Vec<StrippedLine> {
    let mut state = LexState::Normal;
    let mut lines = Vec::new();

    for (idx, raw_line) in source.lines().enumerate() {
        let mut code_buf = String::new();
        let bytes = raw_line.as_bytes();
        let mut i = 0;
        let mut has_code = false;

        while i < bytes.len() {
            let b = bytes[i];
            let next = bytes.get(i + 1).copied();

            match state {
                LexState::Normal => {
                    if b == b'/' && next == Some(b'/') {
                        state = LexState::LineComment;
                        break;
                    } else if b == b'/' && next == Some(b'*') {
                        state = LexState::BlockComment;
                        i += 2;
                        continue;
                    } else if b == b'"' {
                        state = LexState::StringLiteral { escaped: false };
                        has_code = true;
                    } else if b == b'\'' {
                        state = LexState::CharLiteral { escaped: false };
                        has_code = true;
                    } else {
                        if !b.is_ascii_whitespace() {
                            has_code = true;
                        }
                        code_buf.push(b as char);
                    }
                }
                LexState::LineComment => break,
                LexState::BlockComment => {
                    if b == b'*' && next == Some(b'/') {
                        state = LexState::Normal;
                        i += 2;
                        continue;
                    }
                }
                LexState::StringLiteral { escaped } => {
                    if escaped {
                        state = LexState::StringLiteral { escaped: false };
                    } else if b == b'\\' {
                        state = LexState::StringLiteral { escaped: true };
                    } else if b == b'"' {
                        state = LexState::Normal;
                    }
                }
                LexState::CharLiteral { escaped } => {
                    if escaped {
                        state = LexState::CharLiteral { escaped: false };
                    } else if b == b'\\' {
                        state = LexState::CharLiteral { escaped: true };
                    } else if b == b'\'' {
                        state = LexState::Normal;
                    }
                }
            }
            i += 1;
        }

        if state == LexState::LineComment {
            state = LexState::Normal;
        }

        lines.push(StrippedLine {
            line_number: idx + 1,
            raw: raw_line.to_string(),
            code_only: code_buf,
            has_code_token: has_code,
        });
    }

    lines
}

/// Counts non-comment, non-empty code lines in source text.
#[must_use]
pub fn count_code_lines(source: &str) -> usize {
    tokenize_source(source)
        .into_iter()
        .filter(|l| l.has_code_token)
        .count()
}

/// Checks if a source file contains unwrap() or expect() calls outside comments and strings.
#[must_use]
pub fn find_unwrap_expect_calls(source: &str) -> Vec<(usize, String)> {
    let mut results = Vec::new();
    for line in tokenize_source(source) {
        if line.code_only.contains(".unwrap()") {
            results.push((line.line_number, "bare .unwrap() call detected".to_string()));
        } else if line.code_only.contains(".expect(") {
            results.push((line.line_number, "bare .expect() call detected".to_string()));
        }
    }
    results
}

/// Checks for debug prints like dbg!, console.log outside comments/strings.
#[must_use]
pub fn find_debug_prints(source: &str) -> Vec<(usize, String)> {
    let mut results = Vec::new();
    let debug_patterns = ["dbg!", "console.log"];

    for line in tokenize_source(source) {
        for pat in debug_patterns {
            if line.code_only.contains(pat) && !line.raw.contains("tracing::") {
                results.push((line.line_number, format!("debug statement '{pat}' found")));
            }
        }
    }

    results
}

/// Extracts imported modules from source lines across Rust (`use crate::...`),
/// Python (`import ...`, `from ... import`), and JS/TS (`import ... from '...'`).
#[must_use]
pub fn extract_imported_modules(source: &str) -> Vec<(usize, String)> {
    let mut imports = Vec::new();

    for line in tokenize_source(source) {
        let code = line.code_only.trim();

        // Rust: use crate::foo::bar; or use foo::bar;
        if let Some(stripped) = code.strip_prefix("use ") {
            let path_part = stripped.trim_matches(';').trim();
            imports.push((line.line_number, path_part.to_string()));
        }
        // Python: import foo or from foo import bar
        else if let Some(stripped) = code.strip_prefix("import ") {
            let mod_name = stripped.split_whitespace().next().unwrap_or("");
            imports.push((line.line_number, mod_name.to_string()));
        } else if let Some(stripped) = code.strip_prefix("from ") {
            let mod_name = stripped.split_whitespace().next().unwrap_or("");
            imports.push((line.line_number, mod_name.to_string()));
        }
    }

    imports
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── Tokenizer state-machine negative cases ──────────────────────────

    #[test]
    fn commented_out_rust_imports_are_not_extracted() {
        let src = "// use crate::secret;\n// use super::danger;\nuse crate::real;\n";
        let imports = extract_imported_modules(src);
        assert_eq!(imports.len(), 1, "only the live import must count: {imports:?}");
        assert_eq!(imports[0].1, "crate::real");
    }

    #[test]
    fn commented_out_python_imports_are_not_extracted() {
        let src = "# import evil\n# from evil import bad\nimport good\nfrom good import thing\n";
        let imports = extract_imported_modules(src);
        assert_eq!(imports.len(), 2, "only live imports must count: {imports:?}");
        assert_eq!(imports[0].1, "good");
        assert_eq!(imports[1].1, "good");
    }

    #[test]
    fn block_comment_imports_are_not_extracted() {
        let src = "/* use crate::hidden;\n   import phantom */\nuse crate::live;\n";
        let imports = extract_imported_modules(src);
        assert_eq!(imports.len(), 1, "block-comment imports must be stripped: {imports:?}");
        assert_eq!(imports[0].1, "crate::live");
    }

    #[test]
    fn block_comment_closes_at_first_terminator_like_rust_compilers() {
        // Rust/C/JS block comments are NOT nestable: the first `*/` ends the
        // comment. Everything after it is real code, even if a second `*/`
        // appears later. (C# is the notable exception; we never scan C#.)
        let src = "/* outer /* inner */ still code */ let real = 1;\n";
        let lines = tokenize_source(src);
        assert!(
            lines[0].code_only.contains("let real = 1;"),
            "code after the first */ must be visible: {:?}",
            lines[0].code_only
        );
        assert!(
            !lines[0].code_only.contains("inner"),
            "comment interior must be stripped: {:?}",
            lines[0].code_only
        );
    }

    #[test]
    fn block_comment_spanning_lines_strips_interior_and_restores_after() {
        let src = "let a = 1;\n/* multi\nline comment */\nlet b = 2;\n";
        let lines = tokenize_source(src);
        assert!(lines[0].code_only.contains("let a = 1;"));
        assert!(lines[1].code_only.is_empty());
        assert!(lines[2].code_only.is_empty());
        assert!(lines[3].code_only.contains("let b = 2;"));
    }

    #[test]
    fn unwrap_inside_string_literal_is_not_detected() {
        let src = "let msg = \"call .unwrap() here\";\nlet s = \"a \\\".expect(x)\\\" b\";\n";
        assert!(find_unwrap_expect_calls(src).is_empty());
    }

    #[test]
    fn unwrap_inside_comment_is_not_detected() {
        let src = "// let x = opt.unwrap();\n/* .expect(panics) */\n";
        assert!(find_unwrap_expect_calls(src).is_empty());
    }

    #[test]
    fn unwrap_in_real_code_is_detected_with_line_numbers() {
        let src = "fn a() {}\nlet x = opt.unwrap();\nlet y = r.expect(\"nope\");\n";
        let hits = find_unwrap_expect_calls(src);
        assert_eq!(hits.len(), 2);
        assert_eq!(hits[0].0, 2);
        assert_eq!(hits[1].0, 3);
    }

    #[test]
    fn escaped_quote_in_string_does_not_terminate_early() {
        let src = "let s = \"he said \\\"hi\\\"\"; let x = ok.unwrap();\n";
        let lines = tokenize_source(src);
        // The real code after the string must still be visible.
        assert!(
            lines[0].code_only.contains("let x = ok.unwrap();"),
            "escaped quote must keep string open until the true end: {:?}",
            lines[0].code_only
        );
    }

    #[test]
    fn char_literal_with_escaped_quote() {
        let src = "let c = '\\''; let d = 'x';\n";
        let lines = tokenize_source(src);
        assert!(
            lines[0].code_only.contains("let d = "),
            "escaped char must parse: {:?}",
            lines[0].code_only
        );
    }

    #[test]
    fn code_after_line_comment_is_stripped() {
        let src = "let a = 1; // trailing .unwrap() in comment\n";
        let lines = tokenize_source(src);
        assert!(!lines[0].code_only.contains("unwrap"));
        assert!(lines[0].code_only.contains("let a = 1;"));
    }

    #[test]
    fn doc_comment_with_import_example_is_not_extracted() {
        let src = "/// Example: use crate::demo;\npub fn f() {}\n";
        let imports = extract_imported_modules(src);
        assert!(imports.is_empty(), "doc-comment example must not count: {imports:?}");
    }

    #[test]
    fn cfg_test_imports_are_extracted_like_all_imports() {
        // Documented behavior: #[cfg(test)] imports are real imports too;
        // the guard layer decides whether to count them.
        let src = "#[cfg(test)]\nuse super::inner;\n";
        let imports = extract_imported_modules(src);
        assert_eq!(imports.len(), 1);
        assert_eq!(imports[0].1, "super::inner");
    }

    // ── count_code_lines ─────────────────────────────────────────────────

    #[test]
    fn count_code_lines_ignores_comments_strings_and_blank() {
        let src = "// only comment\n\n/* block */\nlet real = \"a string\";\n";
        assert_eq!(count_code_lines(src), 1);
    }

    #[test]
    fn string_only_line_counts_as_code() {
        // A line that is *only* a string literal still has a code token.
        assert_eq!(count_code_lines("\"just a string\";\n"), 1);
    }
}
