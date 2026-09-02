//! Pure-Rust tokenizer that strips comments, docstrings, and string literals.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TokenizerState {
    Normal,
    LineComment,
    BlockComment { depth: usize },
    StringLiteral { escaped: bool },
    CharLiteral { escaped: bool },
}

/// Counts non-comment, non-empty code lines in source text.
#[must_use]
pub fn count_code_lines(source: &str) -> usize {
    let mut state = TokenizerState::Normal;
    let mut code_lines = 0;

    for line in source.lines() {
        let mut has_code_token = false;
        let bytes = line.as_bytes();
        let mut i = 0;

        while i < bytes.len() {
            let b = bytes[i];
            let next = bytes.get(i + 1).copied();

            match state {
                TokenizerState::Normal => {
                    if b == b'/' && next == Some(b'/') {
                        state = TokenizerState::LineComment;
                        break;
                    } else if b == b'/' && next == Some(b'*') {
                        state = TokenizerState::BlockComment { depth: 1 };
                        i += 2;
                        continue;
                    } else if b == b'"' {
                        state = TokenizerState::StringLiteral { escaped: false };
                        has_code_token = true;
                    } else if b == b'\'' {
                        state = TokenizerState::CharLiteral { escaped: false };
                        has_code_token = true;
                    } else if !b.is_ascii_whitespace() {
                        has_code_token = true;
                    }
                }
                TokenizerState::LineComment => {
                    break;
                }
                TokenizerState::BlockComment { depth } => {
                    if b == b'/' && next == Some(b'*') {
                        state = TokenizerState::BlockComment { depth: depth + 1 };
                        i += 2;
                        continue;
                    } else if b == b'*' && next == Some(b'/') {
                        if depth == 1 {
                            state = TokenizerState::Normal;
                        } else {
                            state = TokenizerState::BlockComment { depth: depth - 1 };
                        }
                        i += 2;
                        continue;
                    }
                }
                TokenizerState::StringLiteral { escaped } => {
                    if escaped {
                        state = TokenizerState::StringLiteral { escaped: false };
                    } else if b == b'\\' {
                        state = TokenizerState::StringLiteral { escaped: true };
                    } else if b == b'"' {
                        state = TokenizerState::Normal;
                    }
                }
                TokenizerState::CharLiteral { escaped } => {
                    if escaped {
                        state = TokenizerState::CharLiteral { escaped: false };
                    } else if b == b'\\' {
                        state = TokenizerState::CharLiteral { escaped: true };
                    } else if b == b'\'' {
                        state = TokenizerState::Normal;
                    }
                }
            }
            i += 1;
        }

        if state == TokenizerState::LineComment {
            state = TokenizerState::Normal;
        }

        if has_code_token {
            code_lines += 1;
        }
    }

    code_lines
}

/// Checks if a source file contains unwrap() or expect() calls outside comments and strings.
#[must_use]
pub fn find_unwrap_expect_calls(source: &str) -> Vec<(usize, String)> {
    let mut results = Vec::new();
    let mut state = TokenizerState::Normal;

    for (line_idx, line) in source.lines().enumerate() {
        let line_num = line_idx + 1;
        let bytes = line.as_bytes();
        let mut i = 0;
        let mut clean_line = String::new();

        while i < bytes.len() {
            let b = bytes[i];
            let next = bytes.get(i + 1).copied();

            match state {
                TokenizerState::Normal => {
                    if b == b'/' && next == Some(b'/') {
                        break;
                    } else if b == b'/' && next == Some(b'*') {
                        state = TokenizerState::BlockComment { depth: 1 };
                        i += 2;
                        continue;
                    } else if b == b'"' {
                        state = TokenizerState::StringLiteral { escaped: false };
                    } else if b == b'\'' {
                        state = TokenizerState::CharLiteral { escaped: false };
                    } else {
                        clean_line.push(b as char);
                    }
                }
                TokenizerState::LineComment => break,
                TokenizerState::BlockComment { depth } => {
                    if b == b'*' && next == Some(b'/') {
                        if depth == 1 {
                            state = TokenizerState::Normal;
                        } else {
                            state = TokenizerState::BlockComment { depth: depth - 1 };
                        }
                        i += 2;
                        continue;
                    }
                }
                TokenizerState::StringLiteral { escaped } => {
                    if escaped {
                        state = TokenizerState::StringLiteral { escaped: false };
                    } else if b == b'\\' {
                        state = TokenizerState::StringLiteral { escaped: true };
                    } else if b == b'"' {
                        state = TokenizerState::Normal;
                    }
                }
                TokenizerState::CharLiteral { escaped } => {
                    if escaped {
                        state = TokenizerState::CharLiteral { escaped: false };
                    } else if b == b'\\' {
                        state = TokenizerState::CharLiteral { escaped: true };
                    } else if b == b'\'' {
                        state = TokenizerState::Normal;
                    }
                }
            }
            i += 1;
        }

        if state == TokenizerState::LineComment {
            state = TokenizerState::Normal;
        }

        if clean_line.contains(".unwrap()") {
            results.push((line_num, "bare .unwrap() call detected".to_string()));
        } else if clean_line.contains(".expect(") {
            results.push((line_num, "bare .expect() call detected".to_string()));
        }
    }

    results
}

/// Checks for debug prints like println!, dbg!, console.log outside comments/strings.
#[must_use]
pub fn find_debug_prints(source: &str) -> Vec<(usize, String)> {
    let mut results = Vec::new();
    let debug_patterns = ["dbg!", "console.log", "print(", "println!"];

    for (line_idx, line) in source.lines().enumerate() {
        let trimmed = line.trim_start();
        if trimmed.starts_with("//") || trimmed.starts_with("/*") || trimmed.starts_with('*') {
            continue;
        }

        for pat in debug_patterns {
            if line.contains(pat) && !line.contains("tracing::") {
                results.push((line_idx + 1, format!("debug statement '{pat}' found")));
            }
        }
    }

    results
}
