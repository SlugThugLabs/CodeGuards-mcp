//! Per-module fan-out and fan-in across a set of source files.

use crate::analyzer::tokenizer::extract_imported_modules;
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

/// One module's dependency connections.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct FanOut {
    /// Distinct import specifications in this module, internal and external.
    pub imports: usize,
    /// Distinct modules in the input set that import this one.
    pub imported_by: usize,
}

// ── module_fan_out ───────────────────────────────────────────────────────

/// Per-module fan-out and fan-in across a set of source files.
///
/// `files` maps a module identifier (typically a project-relative path) to its
/// source text. `imports` counts the module's distinct import specifications
/// (internal and external, as Python's `unique_imports` does). `imported_by`
/// counts input modules that resolve to this one.
///
/// Resolution is the same non-AST heuristic the Python analyzer uses: an import
/// resolves to the module whose path components are the longest suffix matching
/// the import's leading components (so `use crate::db::profiles::X` resolves to
/// `src/db/profiles.rs`). Single-component names match by trailing component.
#[must_use]
pub fn module_fan_out(files: &[(String, String)]) -> BTreeMap<String, FanOut> {
    let modules: Vec<(String, Vec<String>)> = files
        .iter()
        .map(|(name, _)| (name.clone(), module_components(name)))
        .collect();
    let mut result: BTreeMap<String, FanOut> = files
        .iter()
        .map(|(name, _)| (name.clone(), FanOut::default()))
        .collect();

    for (name, source) in files {
        let mut imports: BTreeSet<String> = BTreeSet::new();
        for (_, path) in extract_imported_modules(source) {
            let components = import_components(&path);
            if !components.is_empty() {
                imports.insert(components.join("::"));
            }
        }
        if let Some(entry) = result.get_mut(name) {
            entry.imports = imports.len();
        }

        let mut targets: BTreeSet<String> = BTreeSet::new();
        for spec in &imports {
            let components: Vec<String> = spec.split("::").map(ToString::to_string).collect();
            if let Some(target) = resolve_module(&components, &modules)
                && target != *name
            {
                targets.insert(target);
            }
        }
        for target in targets {
            if let Some(entry) = result.get_mut(&target) {
                entry.imported_by += 1;
            }
        }
    }

    result
}

/// Path components of a module name, with the extension stripped.
fn module_components(name: &str) -> Vec<String> {
    let without_extension = Path::new(name.trim()).with_extension("");
    split_components(&without_extension.to_string_lossy(), false)
}

/// Identifier components of an import specification, dropping aliases and
/// grouped-import remains (`use crate::a::{B, C}` → `crate::a`).
fn import_components(spec: &str) -> Vec<String> {
    let head = spec.split_whitespace().next().unwrap_or("");
    let mut components = split_components(head, true);
    while components
        .first()
        .is_some_and(|first| matches!(first.as_str(), "crate" | "super" | "self"))
    {
        components.remove(0);
    }
    components
}

/// Splits on `/`, `\` and `::` (and `.` when `dots` is set), keeping only
/// identifier-like components.
fn split_components(raw: &str, dots: bool) -> Vec<String> {
    let mut components = Vec::new();
    let mut current = String::new();
    let mut characters = raw.chars().peekable();
    while let Some(character) = characters.next() {
        let is_separator = if character == ':' {
            if characters.peek() == Some(&':') {
                characters.next();
                true
            } else {
                false
            }
        } else {
            character == '/' || character == '\\' || (dots && character == '.')
        };
        if is_separator {
            push_component(&mut components, &current);
            current.clear();
        } else {
            current.push(character);
        }
    }
    push_component(&mut components, &current);
    components
}

/// Keeps `current` if it is a non-empty identifier-like component.
fn push_component(components: &mut Vec<String>, current: &str) {
    let is_identifier = !current.is_empty()
        && current
            .chars()
            .next()
            .is_some_and(|first| first.is_alphabetic() || first == '_')
        && current
            .chars()
            .all(|character| character.is_alphanumeric() || character == '_');
    if is_identifier {
        components.push(current.to_string());
    }
}

/// Longest suffix of `module` that is a prefix of `import`.
fn match_score(module: &[String], import: &[String]) -> usize {
    let mut suffix = module;
    loop {
        if suffix.len() <= import.len() && suffix == &import[..suffix.len()] {
            return suffix.len();
        }
        if suffix.is_empty() {
            return 0;
        }
        suffix = &suffix[1..];
    }
}

/// The input module an import resolves to, if any. Ties keep the first module in
/// name order for determinism.
fn resolve_module(import: &[String], modules: &[(String, Vec<String>)]) -> Option<String> {
    let mut best: Option<(String, usize)> = None;
    for (name, components) in modules {
        let score = match_score(components, import);
        if score == 0 {
            continue;
        }
        if best.as_ref().is_none_or(|(_, previous)| score > *previous) {
            best = Some((name.clone(), score));
        }
    }
    best.map(|(name, _)| name)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fan_out_counts_imports_and_imported_by() {
        let files = vec![
            (
                "src/a.rs".to_string(),
                "use crate::b::thing;\nuse crate::c::other;\nuse std::fmt;\n".to_string(),
            ),
            ("src/b.rs".to_string(), "use crate::a::alpha;\n".to_string()),
            ("src/c.rs".to_string(), String::new()),
        ];
        let fan_out = module_fan_out(&files);
        assert_eq!(fan_out["src/a.rs"].imports, 3);
        assert_eq!(fan_out["src/b.rs"].imports, 1);
        assert_eq!(fan_out["src/a.rs"].imported_by, 1);
        assert_eq!(fan_out["src/b.rs"].imported_by, 1);
        assert_eq!(fan_out["src/c.rs"].imported_by, 1);
    }

    #[test]
    fn grouped_and_nested_imports_resolve_to_the_module() {
        let files = vec![
            ("src/analyzer/tokenizer.rs".to_string(), String::new()),
            (
                "src/guards/runner.rs".to_string(),
                "use crate::analyzer::tokenizer::{tokenize_source};\n".to_string(),
            ),
        ];
        let fan_out = module_fan_out(&files);
        assert_eq!(fan_out["src/analyzer/tokenizer.rs"].imported_by, 1);
    }

    #[test]
    fn empty_input_yields_empty_map() {
        assert!(module_fan_out(&[]).is_empty());
    }

    // ── test_region_lines / has_test_module ─────────────────────────────
}
