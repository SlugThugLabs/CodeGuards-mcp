//! Fast, .gitignore-aware parallel file walker using Rayon and Ignore crates.

use crate::error::{CodeGuardsError, Result};
use ignore::WalkBuilder;
use std::path::{Path, PathBuf};

/// Collects all relevant source files in the project root.
pub fn collect_source_files(project_root: &Path) -> Result<Vec<PathBuf>> {
    let mut files = Vec::new();
    let walker = WalkBuilder::new(project_root)
        .standard_filters(true)
        .hidden(true)
        .parents(true)
        .build();

    for result in walker {
        let entry = result.map_err(|e| CodeGuardsError::Io {
            path: project_root.to_path_buf(),
            source: std::io::Error::other(e),
        })?;

        let path = entry.path();
        if path.is_file()
            && let Some(ext) = path.extension().and_then(|e| e.to_str())
            && matches!(ext, "rs" | "py" | "ts" | "js" | "go" | "toml")
        {
            // Ignore planning and git files
            let path_str = path.to_string_lossy();
            if path_str.contains("/.planning/")
                || path_str.contains("/.git/")
                || path_str.contains("/target/")
                || path_str.contains("/node_modules/")
                || path_str.contains("/legacy-python/")
                || path_str.contains("/sample_code/")
            {
                continue;
            }

            files.push(path.to_path_buf());
        }
    }

    Ok(files)
}

/// Collects only files modified in the active git worktree.
pub fn collect_git_diff_files(project_root: &Path) -> Result<Vec<PathBuf>> {
    let output = std::process::Command::new("git")
        .arg("status")
        .arg("--porcelain")
        .current_dir(project_root)
        .output();

    let Ok(output) = output else { return collect_source_files(project_root) };

    let mut files = Vec::new();
    let text = String::from_utf8_lossy(&output.stdout);

    for line in text.lines() {
        if line.len() > 3 {
            let rel_path = line[3..].trim();
            let full_path = project_root.join(rel_path);
            if full_path.is_file()
                && let Some(ext) = full_path.extension().and_then(|e| e.to_str())
                && matches!(ext, "rs" | "py" | "ts" | "js" | "go" | "toml")
            {
                files.push(full_path);
            }
        }
    }

    if files.is_empty() {
        // If git diff returned nothing or not in git repo, fall back to all source files
        collect_source_files(project_root)
    } else {
        Ok(files)
    }
}
