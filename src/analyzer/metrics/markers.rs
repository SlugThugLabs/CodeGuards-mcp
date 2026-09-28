//! Stub, action-item and swallowed-error markers.

use super::lex::{CodeView, LazyRegex, raw_line, re, scan};
use crate::analyzer::tokenizer::tokenize_source;
use regex::Regex;
use std::sync::LazyLock;

/// What kind of marker was found.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MarkerKind {
    /// `todo!(…)`
    Todo,
    /// `unimplemented!(…)`
    Unimplemented,
    /// `unreachable!(…)` — reported for every call; Python's guard only rejects
    /// ones carrying a placeholder message, so that policy belongs to the guard.
    Unreachable,
    /// `panic!("…")` whose message is a placeholder phrase.
    PanicNotImplemented,
    /// Python `raise NotImplementedError`.
    NotImplementedError,
    /// `TODO`/`FIXME`/`HACK`/`ACTION` in a comment.
    ActionItem,
    /// `let _ = …`
    DiscardedResult,
    /// `_ => {}` or `Err(_) => {}`
    EmptyMatchArm,
    /// `if let Err(_)`
    IgnoredError,
    /// Empty `catch {}` / `except: pass`.
    EmptyCatch,
}

/// A textual marker at a specific line.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MarkerSite {
    /// 1-based line of the marker.
    pub line: usize,
    /// Which marker was found.
    pub kind: MarkerKind,
    /// The matched text (macro call, keyword, or arm).
    pub text: String,
    /// The trimmed raw line (for comment markers, the comment text).
    pub context: String,
}

static RE_ACTION: LazyRegex = LazyLock::new(|| Regex::new(r"\b(TODO|FIXME|HACK|ACTION)\b").ok());
static RE_TODO: LazyRegex = LazyLock::new(|| Regex::new(r"\btodo!\s*\(").ok());
static RE_UNIMPLEMENTED: LazyRegex = LazyLock::new(|| Regex::new(r"\bunimplemented!\s*\(").ok());
static RE_UNREACHABLE: LazyRegex = LazyLock::new(|| Regex::new(r"\bunreachable!\s*\(").ok());
static RE_PANIC_STR: LazyRegex = LazyLock::new(|| Regex::new(r#"panic!\s*\(\s*"([^"]*)""#).ok());
static RE_NOT_IMPLEMENTED: LazyRegex =
    LazyLock::new(|| Regex::new(r"\braise\s+NotImplementedError\b").ok());
static RE_PLACEHOLDER_WORDS: LazyRegex = LazyLock::new(|| {
    Regex::new(r"(?i)(not implemented|todo|unimplemented|stub|placeholder|replace me)").ok()
});
static RE_LET_UNDERSCORE: LazyRegex = LazyLock::new(|| Regex::new(r"\blet\s+_\s*=").ok());
static RE_EMPTY_ARM: LazyRegex = LazyLock::new(|| Regex::new(r"_\s*=>\s*\{\s*\}").ok());
static RE_EMPTY_ERR_ARM: LazyRegex =
    LazyLock::new(|| Regex::new(r"Err\(\s*_\s*\)\s*=>\s*\{\s*\}").ok());
static RE_IF_LET_ERR: LazyRegex = LazyLock::new(|| Regex::new(r"\bif\s+let\s+Err\(\s*_\s*\)").ok());
static RE_BARE_EXCEPT: LazyRegex = LazyLock::new(|| Regex::new(r"except\s*:\s*pass").ok());
static RE_TYPED_EXCEPT: LazyRegex =
    LazyLock::new(|| Regex::new(r"except\s+[A-Za-z_][A-Za-z0-9_.]*\s*:\s*pass").ok());
static RE_EMPTY_CATCH: LazyRegex =
    LazyLock::new(|| Regex::new(r"catch\s*(?:\([^)]*\))?\s*\{\s*\}").ok());
static RE_EMPTY_PROMISE_CATCH: LazyRegex =
    LazyLock::new(|| Regex::new(r"\.catch\(\s*\(\s*\)\s*=>\s*\{\s*\}\s*\)").ok());

// ── find_stub_markers ────────────────────────────────────────────────────

/// Placeholder implementations: `todo!`, `unimplemented!`, `unreachable!`,
/// `panic!("not implemented")` and Python's `raise NotImplementedError`.
///
/// Matched against the **code-only** view, so a stub mentioned in a comment or
/// string is not reported. `unreachable!` is reported for every call — Python's
/// guard rejects only the ones carrying a placeholder message, and that
/// filtering belongs to the guard.
#[must_use]
pub fn find_stub_markers(source: &str) -> Vec<MarkerSite> {
    let mut sites = Vec::new();
    let raw_lines: Vec<&str> = source.lines().collect();
    for stripped in tokenize_source(source) {
        let line = stripped.line_number;
        let context = raw_line(&raw_lines, line);
        if re(&RE_TODO).is_some_and(|pattern| pattern.is_match(&stripped.code_only)) {
            sites.push(MarkerSite {
                line,
                kind: MarkerKind::Todo,
                text: "todo!()".to_string(),
                context: context.clone(),
            });
        }
        if re(&RE_UNIMPLEMENTED).is_some_and(|pattern| pattern.is_match(&stripped.code_only)) {
            sites.push(MarkerSite {
                line,
                kind: MarkerKind::Unimplemented,
                text: "unimplemented!()".to_string(),
                context: context.clone(),
            });
        }
        if re(&RE_UNREACHABLE).is_some_and(|pattern| pattern.is_match(&stripped.code_only)) {
            sites.push(MarkerSite {
                line,
                kind: MarkerKind::Unreachable,
                text: "unreachable!()".to_string(),
                context: context.clone(),
            });
        }
        if re(&RE_NOT_IMPLEMENTED).is_some_and(|pattern| pattern.is_match(&stripped.code_only)) {
            sites.push(MarkerSite {
                line,
                kind: MarkerKind::NotImplementedError,
                text: "raise NotImplementedError".to_string(),
                context: context.clone(),
            });
        }
        if stripped.code_only.contains("panic!")
            && let Some(captures) =
                re(&RE_PANIC_STR).and_then(|pattern| pattern.captures(&stripped.raw))
            && let Some(message) = captures.get(1)
            && re(&RE_PLACEHOLDER_WORDS).is_some_and(|pattern| pattern.is_match(message.as_str()))
        {
            sites.push(MarkerSite {
                line,
                kind: MarkerKind::PanicNotImplemented,
                text: captures
                    .get(0)
                    .map_or_else(|| "panic!".to_string(), |whole| whole.as_str().to_string()),
                context: context.clone(),
            });
        }
    }
    sites
}

// ── find_action_markers ──────────────────────────────────────────────────

/// `TODO` / `FIXME` / `HACK` / `ACTION` inside comments, in both `//` and `#`
/// styles.
///
/// Matched against **comment text**, which is where these markers live; a bare
/// keyword in a string literal is not a comment and is not reported. Python's
/// guard additionally requires a `(#123):` issue link (`require_issue` default
/// `true`) — that requirement is policy and stays in the guard.
#[must_use]
pub fn find_action_markers(source: &str) -> Vec<MarkerSite> {
    let mut sites = Vec::new();
    let Some(pattern) = re(&RE_ACTION) else {
        return sites;
    };
    for (line, comment) in scan(source).comments {
        for found in pattern.find_iter(&comment) {
            sites.push(MarkerSite {
                line,
                kind: MarkerKind::ActionItem,
                text: found.as_str().to_string(),
                context: comment.clone(),
            });
        }
    }
    sites
}

// ── find_swallowed_errors ────────────────────────────────────────────────

/// Error-swallowing constructs: `let _ =`, `_ => {}`, `Err(_) => {}`,
/// `if let Err(_)`, empty `catch {}` and Python's `except: pass`.
///
/// Matched against the **code-only** joined text (patterns may span lines), so
/// these shapes quoted in a comment or string are not reported. Python's guard
/// skips test files entirely — a guard-level decision.
#[must_use]
pub fn find_swallowed_errors(source: &str) -> Vec<MarkerSite> {
    let patterns: [(&'static LazyRegex, MarkerKind); 7] = [
        (&RE_LET_UNDERSCORE, MarkerKind::DiscardedResult),
        (&RE_EMPTY_ERR_ARM, MarkerKind::EmptyMatchArm),
        (&RE_EMPTY_ARM, MarkerKind::EmptyMatchArm),
        (&RE_IF_LET_ERR, MarkerKind::IgnoredError),
        (&RE_BARE_EXCEPT, MarkerKind::EmptyCatch),
        (&RE_TYPED_EXCEPT, MarkerKind::EmptyCatch),
        (&RE_EMPTY_CATCH, MarkerKind::EmptyCatch),
    ];
    let code = CodeView::build(source);
    let raw_lines: Vec<&str> = source.lines().collect();
    let mut sites = Vec::new();
    for (slot, kind) in patterns {
        let Some(pattern) = re(slot) else {
            continue;
        };
        for found in pattern.find_iter(&code.text) {
            let line = code.line_at(found.start());
            sites.push(MarkerSite {
                line,
                kind,
                text: found.as_str().trim().to_string(),
                context: raw_line(&raw_lines, line),
            });
        }
    }
    if let Some(pattern) = re(&RE_EMPTY_PROMISE_CATCH) {
        for found in pattern.find_iter(&code.text) {
            let line = code.line_at(found.start());
            sites.push(MarkerSite {
                line,
                kind: MarkerKind::EmptyCatch,
                text: found.as_str().trim().to_string(),
                context: raw_line(&raw_lines, line),
            });
        }
    }
    sites.sort_by(|left, right| {
        left.line
            .cmp(&right.line)
            .then_with(|| left.text.cmp(&right.text))
    });
    sites
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rust_stub_macros_are_detected() {
        let source = "fn f() {\n    todo!();\n    unimplemented!();\n    unreachable!();\n}\n";
        let markers = find_stub_markers(source);
        let kinds: Vec<MarkerKind> = markers.iter().map(|marker| marker.kind).collect();
        assert_eq!(
            kinds,
            vec![
                MarkerKind::Todo,
                MarkerKind::Unimplemented,
                MarkerKind::Unreachable
            ]
        );
        assert_eq!(markers[0].line, 2);
    }

    #[test]
    fn panic_placeholder_is_a_stub_but_a_real_message_is_not() {
        let stub = find_stub_markers("fn f() {\n    panic!(\"not implemented\");\n}\n");
        assert_eq!(stub.len(), 1, "got {stub:?}");
        assert_eq!(stub[0].kind, MarkerKind::PanicNotImplemented);

        let real = find_stub_markers("fn f() {\n    panic!(\"disk full\");\n}\n");
        assert!(real.is_empty(), "got {real:?}");
    }

    #[test]
    fn stubs_inside_comments_and_strings_are_ignored() {
        let source = "// todo!()\nlet s = \"unimplemented!()\";\n";
        assert!(find_stub_markers(source).is_empty());
    }

    #[test]
    fn python_not_implemented_error_is_a_stub() {
        let markers = find_stub_markers("def f():\n    raise NotImplementedError\n");
        assert_eq!(markers.len(), 1, "got {markers:?}");
        assert_eq!(markers[0].kind, MarkerKind::NotImplementedError);
        assert_eq!(markers[0].line, 2);
    }

    // ── find_action_markers ─────────────────────────────────────────────

    #[test]
    fn action_keywords_are_found_in_both_comment_styles() {
        let source = "// TODO: fix\n# FIXME: later\n/* HACK */\nlet x = 1; // ACTION\n";
        let markers = find_action_markers(source);
        let lines: Vec<usize> = markers.iter().map(|marker| marker.line).collect();
        assert_eq!(lines, vec![1, 2, 3, 4], "got {markers:?}");
        assert!(
            markers
                .iter()
                .all(|marker| marker.kind == MarkerKind::ActionItem)
        );
    }

    #[test]
    fn action_keywords_in_strings_are_not_comments() {
        assert!(find_action_markers("let s = \"TODO\";\n").is_empty());
    }

    #[test]
    fn rust_attributes_are_not_hash_comments() {
        let source = "#[derive(Debug)]\nstruct S;\n";
        assert!(find_action_markers(source).is_empty());
    }

    // ── find_swallowed_errors ───────────────────────────────────────────

    #[test]
    fn discarded_results_and_empty_arms_are_detected() {
        let source = "\
fn f() -> Result<(), E> {
    let _ = do_thing();
    match r {
        _ => {}
    }
    if let Err(_) = r {}
    Ok(())
}
";
        let markers = find_swallowed_errors(source);
        let kinds: Vec<MarkerKind> = markers.iter().map(|marker| marker.kind).collect();
        assert!(
            kinds.contains(&MarkerKind::DiscardedResult),
            "got {markers:?}"
        );
        assert!(
            kinds.contains(&MarkerKind::EmptyMatchArm),
            "got {markers:?}"
        );
        assert!(kinds.contains(&MarkerKind::IgnoredError), "got {markers:?}");
    }

    #[test]
    fn error_arms_and_empty_catches_are_detected() {
        let rust = "match r {\n    Err(_) => {}\n}\n";
        let markers = find_swallowed_errors(rust);
        assert_eq!(markers.len(), 1, "got {markers:?}");
        assert_eq!(markers[0].kind, MarkerKind::EmptyMatchArm);
        assert_eq!(markers[0].line, 2);

        let python = "try:\n    work()\nexcept:\n    pass\n";
        let markers = find_swallowed_errors(python);
        assert_eq!(markers.len(), 1, "got {markers:?}");
        assert_eq!(markers[0].kind, MarkerKind::EmptyCatch);

        let javascript = "try { work(); } catch (e) {}\n";
        let markers = find_swallowed_errors(javascript);
        assert_eq!(markers.len(), 1, "got {markers:?}");
        assert_eq!(markers[0].kind, MarkerKind::EmptyCatch);
    }

    #[test]
    fn swallowed_shapes_in_strings_are_ignored() {
        assert!(find_swallowed_errors("let s = \"let _ = x\";\n").is_empty());
    }

    // ── module_fan_out ──────────────────────────────────────────────────
}
