//! Numeric and string literals.

use super::lex::scan;

/// Whether a literal is numeric or a quoted string.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LiteralKind {
    /// A numeric literal, including sign, radix prefix and suffix (`-30i64`).
    Numeric,
    /// A quoted string's inner text, with escapes left as written.
    String,
}

/// A numeric or string literal together with its location and line context.
///
/// Python's `magic_numbers` guard skips the values `0, 1, -1, 2, 10, 100, 1000`
/// and lines that are imports, constants or assertions; `hardcoded_values`
/// matches URLs, IPs, ports and timeouts and skips `const`/`static`/`import`
/// lines. Those are **guard policies** — this primitive reports every literal.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LiteralSite {
    /// 1-based line where the literal starts.
    pub line: usize,
    /// Literal kind.
    pub kind: LiteralKind,
    /// Literal text: digits/sign/suffix for numerics, inner text for strings.
    pub value: String,
    /// The trimmed raw source line containing the literal.
    pub context: String,
}

// ── extract_literals ─────────────────────────────────────────────────────

/// Numeric and string literals in source order, each with its line and context.
///
/// Comments and string bodies are excluded from numeric scanning, so a port
/// number inside a comment is not a literal; conversely a number inside a string
/// is part of that string literal. Guard-level policy (magic-number skips,
/// hardcoded-value patterns) is deliberately not applied here.
#[must_use]
pub fn extract_literals(source: &str) -> Vec<LiteralSite> {
    scan(source).literals
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn numeric_and_string_literals_carry_line_and_context() {
        let source = "let code = 8080;\nlet url = \"https://example.com\";\n";
        let literals = extract_literals(source);
        assert_eq!(literals.len(), 2, "got {literals:?}");
        assert_eq!(literals[0].kind, LiteralKind::Numeric);
        assert_eq!(literals[0].value, "8080");
        assert_eq!(literals[0].line, 1);
        assert!(literals[0].context.contains("8080"));
        assert_eq!(literals[1].kind, LiteralKind::String);
        assert_eq!(literals[1].value, "https://example.com");
        assert_eq!(literals[1].line, 2);
    }

    #[test]
    fn digits_inside_strings_are_part_of_the_string_not_numeric() {
        let source = "let s = \"port 8080\";\n";
        let literals = extract_literals(source);
        assert_eq!(literals.len(), 1, "got {literals:?}");
        assert_eq!(literals[0].kind, LiteralKind::String);
    }

    #[test]
    fn literals_inside_comments_are_ignored() {
        assert!(extract_literals("// 8080 and \"x\"\n").is_empty());
        assert!(extract_literals("# 8080 and \"x\"\n").is_empty());
    }

    #[test]
    fn signed_and_suffixed_numbers_are_single_literals() {
        let literals = extract_literals("let t = -30i64;\n");
        assert_eq!(literals.len(), 1, "got {literals:?}");
        assert_eq!(literals[0].value, "-30i64");
    }

    #[test]
    fn rust_lifetimes_are_not_string_literals() {
        let source = "fn f<'a>(x: &'a str) -> &'a str { x }\n";
        assert!(
            extract_literals(source).is_empty(),
            "lifetimes must not be read as literals"
        );
    }

    #[test]
    fn rust_char_literal_is_reported_as_a_string() {
        let literals = extract_literals("let c = 'x';\n");
        assert_eq!(literals.len(), 1, "got {literals:?}");
        assert_eq!(literals[0].kind, LiteralKind::String);
        assert_eq!(literals[0].value, "x");
    }

    // ── find_stub_markers ───────────────────────────────────────────────
}
