//! Maximum nesting depth per function.

use super::functions::{FunctionSpan, function_spans};
use super::lex::CodeView;

/// A function's maximum nesting depth.
///
/// Python's `deep_nesting` default limit is `max_depth: 4`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NestingSite {
    /// Function name.
    pub name: String,
    /// 1-based line of the signature.
    pub start_line: usize,
    /// Maximum nesting depth observed in the span. Rust: brace depth, counting
    /// the function body itself as depth 1. Python: indentation level, one level
    /// per four spaces or per tab.
    pub max_depth: usize,
    /// 1-based line where `max_depth` was first reached.
    pub line: usize,
}

// ── nesting_depth ────────────────────────────────────────────────────────

/// Maximum nesting depth per function.
///
/// Rust spans use brace depth (the body itself is depth 1). Python spans use
/// indentation, one level per four spaces or per tab. Reported as a fact: the
/// Python guard rejects depth above `max_depth` (default 4).
#[must_use]
pub fn nesting_depth(source: &str) -> Vec<NestingSite> {
    let code = CodeView::build(source);
    let raw_lines: Vec<&str> = source.lines().collect();
    function_spans(source)
        .into_iter()
        .map(|span| {
            if is_python_signature(&raw_lines, span.start_line) {
                python_nesting(&raw_lines, &span)
            } else {
                brace_nesting(&code, &span)
            }
        })
        .collect()
}

/// Whether the source line at `line` is a Python `def`.
fn is_python_signature(raw_lines: &[&str], line: usize) -> bool {
    raw_lines.get(line.saturating_sub(1)).is_some_and(|text| {
        let trimmed = text.trim_start();
        trimmed.starts_with("def ") || trimmed.starts_with("async def ")
    })
}

fn brace_nesting(code: &CodeView, span: &FunctionSpan) -> NestingSite {
    let mut depth = 0usize;
    let mut maximum = 0usize;
    let mut maximum_line = span.start_line;
    let end = span.end_line.min(code.lines.len());
    for index in span.start_line.saturating_sub(1)..end {
        for character in code.lines[index].chars() {
            match character {
                '{' => {
                    depth += 1;
                    if depth > maximum {
                        maximum = depth;
                        maximum_line = index + 1;
                    }
                }
                '}' => depth = depth.saturating_sub(1),
                _ => {}
            }
        }
    }
    NestingSite {
        name: span.name.clone(),
        start_line: span.start_line,
        max_depth: maximum,
        line: maximum_line,
    }
}

fn python_nesting(raw_lines: &[&str], span: &FunctionSpan) -> NestingSite {
    let mut maximum = 0usize;
    let mut maximum_line = span.start_line;
    let end = span.end_line.min(raw_lines.len());
    for (index, text) in raw_lines
        .iter()
        .enumerate()
        .take(end)
        .skip(span.start_line.saturating_sub(1))
    {
        let trimmed = text.trim_start();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        let tabs = text
            .chars()
            .take_while(|character| *character == '\t')
            .count();
        let spaces = text.len() - trimmed.len();
        let level = if tabs > 0 { tabs } else { spaces / 4 };
        if level > maximum {
            maximum = level;
            maximum_line = index + 1;
        }
    }
    NestingSite {
        name: span.name.clone(),
        start_line: span.start_line,
        max_depth: maximum,
        line: maximum_line,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn brace_nesting_is_per_function() {
        let source = "\
fn flat() {
    let x = 1;
}

fn nested() {
    if true {
        for _ in 0..1 {
        }
    }
}
";
        let sites = nesting_depth(source);
        let flat = sites.iter().find(|site| site.name == "flat").expect("flat");
        let nested = sites
            .iter()
            .find(|site| site.name == "nested")
            .expect("nested");
        assert_eq!(flat.max_depth, 1);
        assert_eq!(nested.max_depth, 3);
    }

    #[test]
    fn braces_inside_strings_do_not_add_depth() {
        let source = "fn f() {\n    let s = \"{{{\";\n}\n";
        let sites = nesting_depth(source);
        assert_eq!(sites[0].max_depth, 1);
    }

    #[test]
    fn python_nesting_counts_indentation() {
        let source = "def f():\n    if x:\n        return 1\n";
        let sites = nesting_depth(source);
        let f = sites.iter().find(|site| site.name == "f").expect("f");
        assert_eq!(f.max_depth, 2);
    }

    // ── extract_literals ────────────────────────────────────────────────
}
