//! Source metrics primitives: function spans, nesting, literals, markers, fan-out.
//!
//! Every primitive here reports lexical **facts** about source text and applies
//! no policy. Thresholds stay in guard definitions (`default_params`), so a
//! guard decides what counts as a violation. The semantics mirror the Python
//! implementation under `legacy-python/`, which is the specification for these
//! checks; the Python defaults observed while porting are noted per primitive.
//!
//! ## Comment vs. code matching
//!
//! The primitives use two different views of the source, and picking the wrong
//! one is the difference between a real finding and a false positive:
//!
//! * **Rust/Python markers** (`todo!`, `unimplemented!`, `unreachable!`,
//!   `panic!("…")`, `raise NotImplementedError`, `let _ =`, `_ => {}`) are
//!   matched against the tokenizer's `code_only` view, which strips comments and
//!   string bodies. A `todo!()` inside a doc comment or string must not be
//!   reported.
//! * **Comment markers** (`TODO`, `FIXME`, `HACK`, `ACTION`) live *inside*
//!   comments, so they are matched against the comment text recovered by the
//!   lexer. Matching them against `code_only` would find nothing.

mod fan_out;
mod functions;
mod lex;
mod literals;
mod markers;
mod nesting;

use self::lex::CodeView;

pub use fan_out::{FanOut, module_fan_out};
pub use functions::{FunctionSpan, function_spans, has_doc_comment};
pub use literals::{LiteralKind, LiteralSite, extract_literals};
pub use markers::{
    MarkerKind, MarkerSite, find_action_markers, find_stub_markers, find_swallowed_errors,
};
pub use nesting::{NestingSite, nesting_depth};

// ── test_region_lines ────────────────────────────────────────────────────

/// Line ranges (inclusive, 1-based) of `#[cfg(test)] mod … { … }` blocks.
///
/// This is what lets a guard treat in-file test code the way `clippy.toml`
/// does: `.unwrap()` inside such a block is test code even though the file
/// lives under `src/`. A `#[cfg(test)]` attribute on a non-block item (for
/// example `#[cfg(test)] use super::*;`) yields no range.
#[must_use]
pub fn test_region_lines(source: &str) -> Vec<(usize, usize)> {
    let code = CodeView::build(source);
    let mut regions = Vec::new();
    let mut index = 0usize;
    while index < code.lines.len() {
        if !code.lines[index].contains("#[cfg(test)]") {
            index += 1;
            continue;
        }
        let Some(open_line) = attributed_block_open(&code, index) else {
            index += 1;
            continue;
        };
        if let Some(end_line) = block_end(&code, open_line) {
            regions.push((index + 1, end_line + 1));
            index = end_line + 1;
        } else {
            index = open_line + 1;
        }
    }
    regions
}

/// The line holding the `{` of a block introduced by `#[cfg(test)]` at `index`.
///
/// Aborts when a `;` appears first (the attribute is on a statement, not a
/// block) and requires the declaration to read as a `mod`/`impl` block.
fn attributed_block_open(code: &CodeView, index: usize) -> Option<usize> {
    let mut cursor = index;
    let mut open_line = None;
    while let Some(text) = code.lines.get(cursor) {
        if text.contains(';') {
            return None;
        }
        if text.contains('{') {
            open_line = Some(cursor);
            break;
        }
        cursor += 1;
    }
    let open_line = open_line?;
    let mut declaration = String::new();
    for line in index..=open_line {
        declaration.push_str(code.lines.get(line).map_or("", String::as_str));
        declaration.push(' ');
    }
    if declaration.contains("mod ")
        || declaration.contains("mod\t")
        || declaration.contains("impl ")
    {
        Some(open_line)
    } else {
        None
    }
}

/// Line of the `}` closing the block opened at `open_line`.
fn block_end(code: &CodeView, open_line: usize) -> Option<usize> {
    let mut depth = 0i64;
    let mut started = false;
    for index in open_line..code.lines.len() {
        for character in code.lines[index].chars() {
            match character {
                '{' => {
                    depth += 1;
                    started = true;
                }
                '}' => depth -= 1,
                _ => {}
            }
            if started && depth <= 0 {
                return Some(index);
            }
        }
    }
    None
}

/// Whether the source contains a `#[cfg(test)] mod … { … }` block.
#[must_use]
pub fn has_test_module(source: &str) -> bool {
    !test_region_lines(source).is_empty()
}

// ── Test-path classification ────────────────────────────────────────────────

/// Language test-file suffixes, mirroring Python's `_is_test_path`.
const TEST_FILE_SUFFIXES: [&str; 7] = [
    "_test.py",
    "_test.rs",
    "_tests.rs",
    ".test.ts",
    ".test.js",
    ".spec.ts",
    ".spec.js",
];

/// Whether a path looks like a test file, mirroring Python's `_is_test_path`.
///
/// Matches `tests/`/`test/` directories, `test_` prefixes and the language test
/// suffixes (`_test.rs`, `.spec.ts`, …). Crucially this is component-based, not
/// a substring test: `src/latest.rs` and `src/attestation.rs` are **not** test
/// files even though they contain the letters `test`.
#[must_use]
pub fn is_test_path(path: &str) -> bool {
    let normalized = path.replace('\\', "/");
    let name = normalized.rsplit('/').next().unwrap_or(&normalized);
    if normalized.contains("/tests/")
        || normalized.contains("/test/")
        || normalized.starts_with("tests/")
        || normalized.starts_with("test/")
    {
        return true;
    }
    if name.starts_with("test_") {
        return true;
    }
    TEST_FILE_SUFFIXES
        .iter()
        .any(|suffix| name.ends_with(suffix))
}

#[cfg(test)]
const TEST_MODULE_SOURCE: &str = "\
pub fn production() -> u8 {
    let x: Option<u8> = None;
    x.unwrap()
}

#[cfg(test)]
mod tests {
    #[test]
    fn t() {
        let y: Option<u8> = None;
        y.unwrap();
    }
}
";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_module_range_covers_the_whole_block() {
        assert_eq!(test_region_lines(TEST_MODULE_SOURCE), vec![(6, 13)]);
        assert!(has_test_module(TEST_MODULE_SOURCE));
    }

    #[test]
    fn multiple_test_modules_are_reported() {
        let source = "\
#[cfg(test)]
mod a {
    fn one() {}
}

#[cfg(test)]
mod b {
    fn two() {}
}
";
        assert_eq!(test_region_lines(source), vec![(1, 4), (6, 9)]);
    }

    #[test]
    fn cfg_test_on_a_statement_is_not_a_region() {
        let source = "#[cfg(test)]\nuse super::*;\n\nfn real() {}\n";
        assert!(test_region_lines(source).is_empty());
        assert!(!has_test_module(source));
    }

    #[test]
    fn cfg_test_text_inside_a_string_is_not_a_region() {
        let source = "let s = \"#[cfg(test)] mod tests {\";\n";
        assert!(test_region_lines(source).is_empty());
    }

    // ── has_doc_comment ─────────────────────────────────────────────────

    #[test]
    fn test_paths_follow_directory_and_suffix_conventions() {
        assert!(is_test_path("tests/foo.rs"));
        assert!(is_test_path("src/tests/foo.rs"));
        assert!(is_test_path("src/foo_test.rs"));
        assert!(is_test_path("test_helper.py"));
        assert!(is_test_path("components/Widget.spec.ts"));
    }

    #[test]
    fn production_paths_containing_test_letters_are_not_tests() {
        assert!(!is_test_path("src/latest.rs"));
        assert!(!is_test_path("src/attestation.rs"));
        assert!(!is_test_path("src/contract/mod.rs"));
    }
}
