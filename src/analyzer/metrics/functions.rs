//! Function spans and doc-comment detection, for Rust `fn` and Python `def`.

use super::lex::{CodeView, LazyRegex, re};
use regex::Regex;
use std::sync::LazyLock;

/// A function or method definition and its source extent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FunctionSpan {
    /// Function name.
    pub name: String,
    /// 1-based line of the signature.
    pub start_line: usize,
    /// 1-based line of the closing brace, or of the trailing `;` for a
    /// declaration without a body.
    pub end_line: usize,
    /// Parameters at brace-nesting depth zero, comma-separated (a trailing comma
    /// is ignored). Includes `self`/`&self` for methods, matching Python's
    /// `_count_params`.
    pub param_count: usize,
    /// Rust: the signature was preceded by `pub`. Python: the name does not
    /// start with `_`.
    pub is_pub: bool,
    /// Rust: a `///` or `#[doc]` comment precedes the item (attributes and blank
    /// lines between are skipped, up to five lines). Python: the first statement
    /// in the body is a string literal (a docstring).
    pub has_doc: bool,
}

static RE_RUST_FN: LazyRegex =
    LazyLock::new(|| Regex::new(r"\bfn\s+([A-Za-z_][A-Za-z0-9_]*)").ok());
static RE_PY_DEF: LazyRegex = LazyLock::new(|| {
    Regex::new(r"(?:^|\n)[ \t]*(?:async[ \t]+)?def[ \t]+([A-Za-z_][A-Za-z0-9_]*)[ \t]*\(").ok()
});
static RE_PUB: LazyRegex = LazyLock::new(|| Regex::new(r"\bpub\b").ok());

// ── function_spans ───────────────────────────────────────────────────────

/// Extracts function spans (Rust `fn`, Python `def`).
///
/// Spans are sorted by start line. Rust functions at minimum; a function whose
/// body cannot be located is still reported with the extent of its signature.
#[must_use]
pub fn function_spans(source: &str) -> Vec<FunctionSpan> {
    let mut spans = rust_function_spans(source);
    spans.extend(python_function_spans(source));
    spans.sort_by(|left, right| {
        left.start_line
            .cmp(&right.start_line)
            .then_with(|| left.name.cmp(&right.name))
    });
    spans
}

fn rust_function_spans(source: &str) -> Vec<FunctionSpan> {
    let mut spans = Vec::new();
    let Some(re_fn) = re(&RE_RUST_FN) else {
        return spans;
    };
    let code = CodeView::build(source);
    for captures in re_fn.captures_iter(&code.text) {
        let Some(name_match) = captures.get(1) else {
            continue;
        };
        let Some(keyword_match) = captures.get(0) else {
            continue;
        };
        let start_line = code.line_at(keyword_match.start());
        let prefix = code
            .text
            .get(code.line_start(start_line)..keyword_match.start())
            .unwrap_or("");
        let is_pub = re(&RE_PUB).is_some_and(|pattern| pattern.is_match(prefix));
        let Some(open) = find_open_paren(&code.text, keyword_match.end()) else {
            continue;
        };
        let Some(close) = find_matching(&code.text, open, b'(', b')') else {
            continue;
        };
        let param_count = count_params(code.text.get(open + 1..close).unwrap_or(""));
        let end_line = match find_body_start(&code.text, close + 1) {
            Some(BodyStart::Brace(index)) => find_matching(&code.text, index, b'{', b'}')
                .map_or_else(|| code.line_at(index), |end| code.line_at(end)),
            Some(BodyStart::Semicolon(index)) => code.line_at(index),
            None => code.line_at(close),
        };
        spans.push(FunctionSpan {
            name: name_match.as_str().to_string(),
            start_line,
            end_line,
            param_count,
            is_pub,
            has_doc: has_doc_comment(source, start_line),
        });
    }
    spans
}

fn python_function_spans(source: &str) -> Vec<FunctionSpan> {
    let mut spans = Vec::new();
    let Some(re_def) = re(&RE_PY_DEF) else {
        return spans;
    };
    let code = CodeView::build(source);
    let raw_lines: Vec<&str> = source.lines().collect();
    for captures in re_def.captures_iter(&code.text) {
        let Some(name_match) = captures.get(1) else {
            continue;
        };
        let Some(full_match) = captures.get(0) else {
            continue;
        };
        let name = name_match.as_str().to_string();
        let start_line = code.line_at(name_match.start());
        let open = full_match.end().saturating_sub(1);
        let Some(close) = find_matching(&code.text, open, b'(', b')') else {
            continue;
        };
        let param_count = count_params(code.text.get(open + 1..close).unwrap_or(""));
        let signature_end = code.line_at(close);
        let end_line = python_block_end(&raw_lines, start_line, signature_end);
        spans.push(FunctionSpan {
            name: name.clone(),
            start_line,
            end_line,
            param_count,
            is_pub: !name.starts_with('_'),
            has_doc: has_python_docstring(&raw_lines, signature_end),
        });
    }
    spans
}

/// Last line of a Python block: the last line indented deeper than the `def`.
fn python_block_end(raw_lines: &[&str], def_line: usize, signature_end: usize) -> usize {
    let Some(def_text) = raw_lines.get(def_line.saturating_sub(1)) else {
        return signature_end;
    };
    let def_indent = def_text.len() - def_text.trim_start().len();
    let mut end = signature_end;
    let mut index = signature_end;
    while let Some(text) = raw_lines.get(index) {
        let trimmed = text.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            index += 1;
            continue;
        }
        let indent = text.len() - text.trim_start().len();
        if indent <= def_indent {
            break;
        }
        end = index + 1;
        index += 1;
    }
    end
}

/// Whether the first statement after a Python signature is a docstring.
fn has_python_docstring(raw_lines: &[&str], signature_end: usize) -> bool {
    let mut index = signature_end;
    while let Some(text) = raw_lines.get(index) {
        let trimmed = text.trim();
        if trimmed.is_empty() {
            index += 1;
            continue;
        }
        return trimmed.starts_with("\"\"\"")
            || trimmed.starts_with("'''")
            || trimmed.starts_with('"')
            || trimmed.starts_with('\'');
    }
    false
}

/// Where a Rust function body begins.
enum BodyStart {
    Brace(usize),
    Semicolon(usize),
}

/// First `{` (a body) or `;` (a declaration) after the parameter list.
fn find_body_start(text: &str, from: usize) -> Option<BodyStart> {
    let bytes = text.as_bytes();
    let mut depth = 0i64;
    let mut index = from;
    while index < bytes.len() {
        match bytes[index] {
            b'(' | b'[' => depth += 1,
            b')' | b']' => depth -= 1,
            b'{' if depth == 0 => return Some(BodyStart::Brace(index)),
            b';' if depth == 0 => return Some(BodyStart::Semicolon(index)),
            _ => {}
        }
        index += 1;
    }
    None
}

/// First `(` that is not inside a generic angle-bracket list, so a bound such as
/// `T: Fn(u8)` does not masquerade as the parameter list.
fn find_open_paren(text: &str, from: usize) -> Option<usize> {
    let bytes = text.as_bytes();
    let mut angles = 0i64;
    let mut index = from;
    while index < bytes.len() {
        match bytes[index] {
            b'<' => angles += 1,
            b'>' if index > 0 && bytes[index - 1] != b'-' => angles -= 1,
            b'(' if angles <= 0 => return Some(index),
            b'\n' if angles <= 0 => {}
            _ => {}
        }
        index += 1;
    }
    None
}

/// Index of the `close_byte` matching the `open_byte` at `open`.
fn find_matching(text: &str, open: usize, open_byte: u8, close_byte: u8) -> Option<usize> {
    let bytes = text.as_bytes();
    let mut depth = 0i64;
    let mut index = open;
    while index < bytes.len() {
        if bytes[index] == open_byte {
            depth += 1;
        } else if bytes[index] == close_byte {
            depth -= 1;
            if depth <= 0 {
                return Some(index);
            }
        }
        index += 1;
    }
    None
}

/// Counts comma-separated parameters at bracket depth zero. A trailing comma is
/// ignored; depth is clamped so stray `>`/`)` cannot go negative.
fn count_params(params: &str) -> usize {
    let trimmed = params.trim().trim_end_matches(',');
    if trimmed.is_empty() {
        return 0;
    }
    let mut depth = 0i64;
    let mut count = 1usize;
    for character in trimmed.chars() {
        match character {
            '<' | '(' | '[' | '{' => depth += 1,
            '>' | ')' | ']' | '}' => depth = (depth - 1).max(0),
            ',' if depth == 0 => count += 1,
            _ => {}
        }
    }
    count
}

// ── has_doc_comment ────────────────────────────────────

/// Whether the item starting on `start_line` carries a doc comment.
///
/// Rust convention, matching Python's `rust_extract_missing_docs`: walk up to
/// five lines back, skipping blank lines and `#[…]` attributes; a `///`, `/**`
/// or `#[doc…]` line counts, and any other non-blank line ends the search. A
/// multi-line `/** … */` block is not recognized (Python does not recognize it
/// either).
#[must_use]
pub fn has_doc_comment(source: &str, start_line: usize) -> bool {
    let lines: Vec<&str> = source.lines().collect();
    if start_line == 0 || start_line > lines.len() {
        return false;
    }
    let mut index = start_line - 1;
    let mut checked = 0usize;
    while index > 0 && checked < 5 {
        let previous = lines[index - 1].trim();
        if previous.starts_with("///")
            || previous.starts_with("/**")
            || previous.starts_with("#[doc")
        {
            return true;
        }
        if previous.starts_with("#[") || previous.is_empty() {
            index -= 1;
            checked += 1;
            continue;
        }
        return false;
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rust_spans_carry_bounds_params_visibility_and_docs() {
        let source = "\
/// Adds one.
pub fn add(a: u8, b: u8) -> u8 { a + b }

pub async fn go<T, U>(x: T, y: Vec<U>, z: HashMap<K, V>) {}

fn private() {}
";
        let spans = function_spans(source);
        assert_eq!(spans.len(), 3, "got {spans:?}");

        assert_eq!(spans[0].name, "add");
        assert_eq!(spans[0].start_line, 2);
        assert_eq!(spans[0].end_line, 2);
        assert_eq!(spans[0].param_count, 2);
        assert!(spans[0].is_pub);
        assert!(spans[0].has_doc, "leading /// must count as documentation");

        assert_eq!(spans[1].name, "go");
        assert_eq!(spans[1].param_count, 3);
        assert!(spans[1].is_pub);
        assert!(!spans[1].has_doc);

        assert_eq!(spans[2].name, "private");
        assert_eq!(spans[2].param_count, 0);
        assert!(!spans[2].is_pub);
    }

    #[test]
    fn generic_bound_parens_are_not_the_parameter_list() {
        let source = "pub fn run<T: Fn(u8) -> u8>(callback: T, count: usize) {}\n";
        let spans = function_spans(source);
        assert_eq!(spans.len(), 1, "got {spans:?}");
        assert_eq!(spans[0].param_count, 2);
    }

    #[test]
    fn multi_line_signature_span_covers_body() {
        let source = "pub fn multi(\n    a: u8,\n    b: u8,\n) -> u8 {\n    a + b\n}\n";
        let spans = function_spans(source);
        assert_eq!(spans.len(), 1, "got {spans:?}");
        assert_eq!(spans[0].param_count, 2);
        assert_eq!(spans[0].start_line, 1);
        assert_eq!(spans[0].end_line, 6);
    }

    #[test]
    fn trait_declaration_ends_at_semicolon() {
        let source = "pub trait T {\n    fn method(&self);\n}\n";
        let spans = function_spans(source);
        let method = spans
            .iter()
            .find(|span| span.name == "method")
            .expect("method span");
        assert_eq!(method.start_line, 2);
        assert_eq!(method.end_line, 2);
        assert_eq!(method.param_count, 1, "&self counts as a parameter");
    }

    #[test]
    fn python_def_span_uses_indentation_and_detects_docstring() {
        let source = "\
def outer(a, b):
    if a:
        return b
    return a
";
        let spans = function_spans(source);
        assert_eq!(spans.len(), 1, "got {spans:?}");
        assert_eq!(spans[0].name, "outer");
        assert_eq!(spans[0].start_line, 1);
        assert_eq!(spans[0].end_line, 4);
        assert_eq!(spans[0].param_count, 2);
        assert!(spans[0].is_pub);
        assert!(!spans[0].has_doc);

        let documented = "def documented():\n    \"\"\"Doc.\"\"\"\n    return 1\n";
        let spans = function_spans(documented);
        assert!(spans[0].has_doc, "docstring must count as documentation");
    }

    // ── nesting_depth ───────────────────────────────────────────────────

    #[test]
    fn doc_comment_is_seen_through_attributes() {
        let source = "/// Doc.\n#[inline]\npub fn f() {}\n";
        assert!(has_doc_comment(source, 3));
    }

    #[test]
    fn doc_comment_search_stops_at_other_code() {
        let source = "let x = 1;\npub fn f() {}\n";
        assert!(!has_doc_comment(source, 2));
        assert!(!has_doc_comment("pub fn f() {}\n", 1));
        assert!(!has_doc_comment(source, 99));
    }

    // ── is_test_path ────────────────────────────────────────────────────
}
