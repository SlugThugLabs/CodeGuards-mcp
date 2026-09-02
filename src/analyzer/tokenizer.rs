//! Unified pure-Rust code scanner and tokenizer.
//!
//! Strips comments, docstrings, and string literals in a single clean pass.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum LexState {
    Normal,
    LineComment,
    BlockComment { depth: usize },
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
                        state = LexState::BlockComment { depth: 1 };
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
                LexState::BlockComment { depth } => {
                    if b == b'/' && next == Some(b'*') {
                        state = LexState::BlockComment { depth: depth + 1 };
                        i += 2;
                        continue;
                    } else if b == b'*' && next == Some(b'/') {
                        if depth == 1 {
                            state = LexState::Normal;
                        } else {
                            state = LexState::BlockComment { depth: depth - 1 };
                        }
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
        if code.starts_with("use ") {
            let path_part = code["use ".len()..]
                .trim_matches(';')
                .trim();
            imports.push((line.line_number, path_part.to_string()));
        }
        // Python: import foo or from foo import bar
        else if code.starts_with("import ") {
            let mod_name = code["import ".len()..].split_whitespace().next().unwrap_or("");
            imports.push((line.line_number, mod_name.to_string()));
        } else if code.starts_with("from ") {
            let mod_name = code["from ".len()..].split_whitespace().next().unwrap_or("");
            imports.push((line.line_number, mod_name.to_string()));
        }
    }

    imports
}
