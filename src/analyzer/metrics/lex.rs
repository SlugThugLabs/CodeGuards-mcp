//! The code-only source view and the raw lexical scan.
//!
//! [`CodeView`] is the tokenizer's `code_only` output with per-line offsets; [`scan`]
//! additionally recovers string/numeric literals and comment text from the raw
//! source.

use super::literals::{LiteralKind, LiteralSite};
use crate::analyzer::tokenizer::tokenize_source;
use regex::Regex;
use std::sync::LazyLock;

/// A lazily compiled regex. `None` only if the literal pattern fails to compile,
/// which keeps a malformed pattern from reaching an `unwrap`/`expect` call.
pub(super) type LazyRegex = LazyLock<Option<Regex>>;

/// Borrow the compiled regex out of a [`LazyRegex`].
pub(super) fn re(slot: &LazyRegex) -> Option<&Regex> {
    slot.as_ref()
}

// ── Code/literal lexer ──────────────────────────────────────────────────────

/// The code-only view of a source file: one entry per line, plus a joined text
/// and per-line byte offsets for mapping a match back to its line.
pub(super) struct CodeView {
    pub(super) lines: Vec<String>,
    pub(super) starts: Vec<usize>,
    pub(super) text: String,
}

impl CodeView {
    #[must_use]
    pub(super) fn build(source: &str) -> Self {
        let mut lines = Vec::new();
        let mut starts = Vec::new();
        let mut text = String::new();
        for stripped in tokenize_source(source) {
            starts.push(text.len());
            text.push_str(&stripped.code_only);
            text.push('\n');
            lines.push(stripped.code_only);
        }
        Self {
            lines,
            starts,
            text,
        }
    }

    /// 1-based line containing `offset`.
    #[must_use]
    pub(super) fn line_at(&self, offset: usize) -> usize {
        match self.starts.binary_search(&offset) {
            Ok(index) => index + 1,
            Err(index) => index.max(1),
        }
    }

    /// Byte offset where a 1-based line starts.
    #[must_use]
    pub(super) fn line_start(&self, line: usize) -> usize {
        self.starts
            .get(line.saturating_sub(1))
            .copied()
            .unwrap_or(0)
    }
}

/// Literals and comment texts recovered by one pass over the raw source.
pub(super) struct Scan {
    pub(super) literals: Vec<LiteralSite>,
    /// `(line, trimmed comment text)` for every non-empty comment fragment.
    pub(super) comments: Vec<(usize, String)>,
}

/// Lexes the raw source into literals and comment texts, tracking strings,
/// characters, raw strings and both comment styles so that `//` inside a URL
/// string or a digit inside a comment is not mistaken for code.
#[must_use]
pub(super) fn scan(source: &str) -> Scan {
    let bytes = source.as_bytes();
    let raw_lines: Vec<&str> = source.lines().collect();
    let rust_like = looks_rust_like(source);
    let mut literals = Vec::new();
    let mut comments = Vec::new();
    let mut cursor = 0usize;
    let mut line = 1usize;

    while cursor < bytes.len() {
        let byte = bytes[cursor];

        if byte == b'\n' {
            line += 1;
            cursor += 1;
            continue;
        }
        if byte == b'/' && bytes.get(cursor + 1) == Some(&b'/') {
            let start = cursor + 2;
            let end = find_byte_from(bytes, start, b'\n').unwrap_or(bytes.len());
            push_comment(&mut comments, line, source.get(start..end).unwrap_or(""));
            cursor = end;
            continue;
        }
        if byte == b'/' && bytes.get(cursor + 1) == Some(&b'*') {
            if let Some(end) = find_pair_from(bytes, cursor + 2, *b"*/") {
                let interior = source.get(cursor + 2..end).unwrap_or("");
                for (offset, part) in interior.split('\n').enumerate() {
                    push_comment(&mut comments, line + offset, part);
                }
                line += interior.matches('\n').count();
                cursor = end + 2;
            } else {
                cursor = bytes.len();
            }
            continue;
        }
        // A `#` comment (Python/PyYAML) — but not a Rust attribute `#[…]`/`#![…]`.
        if byte == b'#' && !matches!(bytes.get(cursor + 1).copied(), Some(b'[' | b'!')) {
            let start = cursor + 1;
            let end = find_byte_from(bytes, start, b'\n').unwrap_or(bytes.len());
            push_comment(&mut comments, line, source.get(start..end).unwrap_or(""));
            cursor = end;
            continue;
        }
        if let Some((next, body_start, body_end)) = scan_quoted(bytes, cursor, rust_like) {
            literals.push(LiteralSite {
                line,
                kind: LiteralKind::String,
                value: source.get(body_start..body_end).unwrap_or("").to_string(),
                context: raw_line(&raw_lines, line),
            });
            line += source
                .get(cursor..next)
                .map_or(0, |consumed| consumed.matches('\n').count());
            cursor = next;
            continue;
        }
        if let Some(next) = scan_number_at(bytes, cursor) {
            literals.push(LiteralSite {
                line,
                kind: LiteralKind::Numeric,
                value: source.get(cursor..next).unwrap_or("").to_string(),
                context: raw_line(&raw_lines, line),
            });
            cursor = next;
            continue;
        }
        cursor += 1;
    }

    Scan { literals, comments }
}

/// Pushes a comment fragment unless it is only whitespace.
fn push_comment(comments: &mut Vec<(usize, String)>, line: usize, text: &str) {
    let trimmed = text.trim();
    if !trimmed.is_empty() {
        comments.push((line, trimmed.to_string()));
    }
}

/// The trimmed raw line at `line` (1-based), or an empty string past the end.
#[must_use]
pub(super) fn raw_line(raw_lines: &[&str], line: usize) -> String {
    raw_lines
        .get(line.saturating_sub(1))
        .map_or_else(String::new, |text| text.trim().to_string())
}

/// Whether the source looks like Rust. Used only to disambiguate `'…'`, which is
/// a character/lifetime in Rust and a string in Python/JS.
fn looks_rust_like(source: &str) -> bool {
    source.contains("fn ")
        || source.contains("impl ")
        || source.contains("#[")
        || source.contains("pub ")
}

/// Recognizes a quoted literal at `index`, returning `(index_after, body_start, body_end)`.
fn scan_quoted(bytes: &[u8], index: usize, rust_like: bool) -> Option<(usize, usize, usize)> {
    match bytes.get(index).copied()? {
        b'"' => scan_double_quoted(bytes, index),
        b'`' => {
            let body_start = index + 1;
            let body_end = find_byte_from(bytes, body_start, b'`')?;
            Some((body_end + 1, body_start, body_end))
        }
        b'\'' => scan_single_quoted(bytes, index, rust_like),
        b'r' | b'b' => scan_raw_string(bytes, index),
        _ => None,
    }
}

/// Double-quoted string, including Python's triple-quoted form.
fn scan_double_quoted(bytes: &[u8], index: usize) -> Option<(usize, usize, usize)> {
    if bytes.get(index + 1) == Some(&b'"') && bytes.get(index + 2) == Some(&b'"') {
        let body_start = index + 3;
        let mut probe = body_start;
        while probe + 2 < bytes.len() {
            if bytes[probe] == b'"' && bytes[probe + 1] == b'"' && bytes[probe + 2] == b'"' {
                return Some((probe + 3, body_start, probe));
            }
            probe += 1;
        }
        return None;
    }
    let body_start = index + 1;
    let mut probe = body_start;
    let mut escaped = false;
    while probe < bytes.len() {
        let byte = bytes[probe];
        if escaped {
            escaped = false;
        } else if byte == b'\\' {
            escaped = true;
        } else if byte == b'"' {
            return Some((probe + 1, body_start, probe));
        }
        probe += 1;
    }
    None
}

/// Rust raw string `r"…"` / `r#"…"#` / `br#"…"#`.
fn scan_raw_string(bytes: &[u8], index: usize) -> Option<(usize, usize, usize)> {
    let mut probe = index;
    if bytes.get(probe) == Some(&b'b') {
        probe += 1;
    }
    if bytes.get(probe) != Some(&b'r') {
        return None;
    }
    probe += 1;
    let mut hashes = 0usize;
    while bytes.get(probe) == Some(&b'#') {
        hashes += 1;
        probe += 1;
    }
    if bytes.get(probe) != Some(&b'"') {
        return None;
    }
    let body_start = probe + 1;
    let mut scan_at = body_start;
    while scan_at < bytes.len() {
        if bytes[scan_at] == b'"' {
            let terminated = (0..hashes).all(|h| bytes.get(scan_at + 1 + h) == Some(&b'#'));
            if terminated {
                return Some((scan_at + 1 + hashes, body_start, scan_at));
            }
        }
        scan_at += 1;
    }
    None
}

/// Single-quoted literal: a Rust character (`'x'`, `'\n'`) or a Python/JS string.
/// Rust lifetimes carry no closing quote on the line and are therefore skipped.
fn scan_single_quoted(
    bytes: &[u8],
    index: usize,
    rust_like: bool,
) -> Option<(usize, usize, usize)> {
    if bytes.get(index + 1) == Some(&b'\\') {
        let mut probe = index + 2;
        let mut escaped = true;
        while probe < bytes.len() {
            let byte = bytes[probe];
            if escaped {
                escaped = false;
            } else if byte == b'\\' {
                escaped = true;
            } else if byte == b'\'' {
                return Some((probe + 1, index + 1, probe));
            } else if byte == b'\n' {
                return None;
            }
            probe += 1;
        }
        return None;
    }
    if bytes.get(index + 2) == Some(&b'\'') && bytes.get(index + 1).is_some_and(|b| *b != b'\n') {
        return Some((index + 3, index + 1, index + 2));
    }
    if rust_like {
        return None;
    }
    let line_end = find_byte_from(bytes, index + 1, b'\n').unwrap_or(bytes.len());
    let mut probe = index + 1;
    let mut escaped = false;
    while probe < line_end {
        let byte = bytes[probe];
        if escaped {
            escaped = false;
        } else if byte == b'\\' {
            escaped = true;
        } else if byte == b'\'' {
            return Some((probe + 1, index + 1, probe));
        }
        probe += 1;
    }
    None
}

/// Recognizes a numeric literal starting at `index`, returning the index after it.
fn scan_number_at(bytes: &[u8], index: usize) -> Option<usize> {
    let byte = bytes.get(index).copied()?;
    let negative = byte == b'-' && bytes.get(index + 1).is_some_and(u8::is_ascii_digit);
    if !byte.is_ascii_digit() && !negative {
        return None;
    }
    if negative {
        if !is_sign_boundary(bytes, index) {
            return None;
        }
    } else {
        let previous = if index == 0 {
            None
        } else {
            bytes.get(index - 1).copied()
        };
        if previous.is_some_and(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'.') {
            return None;
        }
    }
    Some(consume_number(bytes, index))
}

/// Consumes sign, radix prefix, digits, fraction, exponent and suffix.
fn consume_number(bytes: &[u8], start: usize) -> usize {
    let mut cursor = start;
    if bytes.get(cursor) == Some(&b'-') {
        cursor += 1;
    }
    if bytes.get(cursor) == Some(&b'0')
        && matches!(
            bytes.get(cursor + 1).copied(),
            Some(b'x' | b'X' | b'b' | b'B' | b'o' | b'O')
        )
    {
        cursor += 2;
        while bytes
            .get(cursor)
            .is_some_and(|b| b.is_ascii_alphanumeric() || *b == b'_')
        {
            cursor += 1;
        }
        return cursor;
    }
    while bytes
        .get(cursor)
        .is_some_and(|b| b.is_ascii_digit() || *b == b'_')
    {
        cursor += 1;
    }
    if bytes.get(cursor) == Some(&b'.') && bytes.get(cursor + 1).is_some_and(u8::is_ascii_digit) {
        cursor += 1;
        while bytes
            .get(cursor)
            .is_some_and(|b| b.is_ascii_digit() || *b == b'_')
        {
            cursor += 1;
        }
    }
    if matches!(bytes.get(cursor).copied(), Some(b'e' | b'E')) {
        let mut probe = cursor + 1;
        if matches!(bytes.get(probe).copied(), Some(b'+' | b'-')) {
            probe += 1;
        }
        let digits_start = probe;
        while bytes.get(probe).is_some_and(u8::is_ascii_digit) {
            probe += 1;
        }
        if probe > digits_start {
            cursor = probe;
        }
    }
    if bytes.get(cursor).is_some_and(u8::is_ascii_alphabetic) {
        while bytes
            .get(cursor)
            .is_some_and(|b| b.is_ascii_alphanumeric() || *b == b'_')
        {
            cursor += 1;
        }
    }
    cursor
}

/// Whether the character before a signed number is an operator, bracket or start
/// of line — matching Python's `-?\d` behaviour without treating `x-1` as `-1`.
fn is_sign_boundary(bytes: &[u8], index: usize) -> bool {
    let mut probe = index;
    while probe > 0 {
        probe -= 1;
        let byte = bytes[probe];
        if byte.is_ascii_whitespace() {
            continue;
        }
        return matches!(
            byte,
            b'=' | b'('
                | b'['
                | b'{'
                | b','
                | b':'
                | b';'
                | b'+'
                | b'-'
                | b'*'
                | b'/'
                | b'<'
                | b'>'
                | b'&'
                | b'|'
                | b'!'
                | b'?'
        );
    }
    true
}

/// First index at or after `from` holding `needle`.
fn find_byte_from(bytes: &[u8], from: usize, needle: u8) -> Option<usize> {
    (from..bytes.len()).find(|index| bytes[*index] == needle)
}

/// First index at or after `from` where `needle` (a two-byte ASCII sequence) begins.
fn find_pair_from(bytes: &[u8], from: usize, needle: [u8; 2]) -> Option<usize> {
    (from..bytes.len().saturating_sub(1))
        .find(|index| bytes[*index] == needle[0] && bytes[*index + 1] == needle[1])
}
