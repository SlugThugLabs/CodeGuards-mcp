//! Loader, seeder, and creator for ~/.slugthug/codeguards/tests/.

pub mod builtins;
pub mod catalog;

use crate::error::{CodeGuardsError, Result};
use crate::library::builtins::get_builtin_guard_tests;
use crate::library::catalog::GuardCatalog;
use crate::types::GuardTestDefinition;
use crate::util::get_tests_dir;
use std::fs;
use std::path::{Path, PathBuf};

/// Ensures the global tests directory is initialized and seeded with built-in tests.
pub fn ensure_test_library_seeded() -> Result<GuardCatalog> {
    let tests_root = get_tests_dir();
    fs::create_dir_all(&tests_root).map_err(|e| CodeGuardsError::Io {
        path: tests_root.clone(),
        source: e,
    })?;

    let builtins = get_builtin_guard_tests();
    let mut definitions = Vec::new();

    for def in builtins {
        let category_dir = def.category.as_str();
        let slug_name = def.name.replace('_', "-");
        let file_name_final = format!("{slug_name}.guard.json");

        let target_dir = tests_root.join(category_dir);
        fs::create_dir_all(&target_dir).map_err(|e| CodeGuardsError::Io {
            path: target_dir.clone(),
            source: e,
        })?;

        let test_file = target_dir.join(file_name_final);
        if !test_file.exists() {
            let json_str = serde_json::to_string_pretty(&def)?;
            fs::write(&test_file, json_str).map_err(|e| CodeGuardsError::Io {
                path: test_file.clone(),
                source: e,
            })?;
        }

        definitions.push((def, test_file));
    }

    // Load any user/custom tests from disk
    load_all_tests_and_save_catalog(&tests_root)
}

/// Scans the tests directory, loads all definitions, and writes catalog.json.
pub fn load_all_tests_and_save_catalog(tests_root: &Path) -> Result<GuardCatalog> {
    let mut definitions = Vec::new();

    for entry in walkdir::WalkDir::new(tests_root)
        .into_iter()
        .filter_map(std::result::Result::ok)
    {
        let path = entry.path();
        if path.is_file() && path.extension().is_some_and(|ext| ext == "json") {
            if path.file_name().is_some_and(|n| n == "catalog.json") {
                continue;
            }
            if let Ok(content) = fs::read_to_string(path)
                && let Ok(def) = serde_json::from_str::<GuardTestDefinition>(&content)
            {
                definitions.push((def, path.to_path_buf()));
            }
        }
    }

    let catalog = GuardCatalog::from_definitions(&definitions);
    let catalog_file = tests_root.join("catalog.json");
    let catalog_json = serde_json::to_string_pretty(&catalog)?;
    fs::write(&catalog_file, catalog_json).map_err(|e| CodeGuardsError::Io {
        path: catalog_file,
        source: e,
    })?;

    Ok(catalog)
}

/// Creates a new custom guard test with duplicate prevention.
pub fn create_custom_guard_test(
    tests_root: &Path,
    def: GuardTestDefinition,
    force: bool,
) -> Result<PathBuf> {
    let catalog_file = tests_root.join("catalog.json");
    let catalog = if catalog_file.exists() {
        let content = fs::read_to_string(&catalog_file).map_err(|e| CodeGuardsError::Io {
            path: catalog_file.clone(),
            source: e,
        })?;
        serde_json::from_str::<GuardCatalog>(&content)?
    } else {
        GuardCatalog::default()
    };

    if !force
        && let Some(reason) = catalog.find_potential_duplicate(&def.name, &def.tags)
    {
        return Err(CodeGuardsError::InvalidGuardTest {
            test_id: def.id.clone(),
            reason: format!("Duplicate guard prevented: {reason}. Use --force to override."),
        });
    }

    let target_dir = tests_root.join(&def.category);
    fs::create_dir_all(&target_dir).map_err(|e| CodeGuardsError::Io {
        path: target_dir.clone(),
        source: e,
    })?;

    let slug_name = def.name.replace('_', "-");
    let target_file = target_dir.join(format!("{slug_name}.guard.json"));

    let json_str = serde_json::to_string_pretty(&def)?;
    fs::write(&target_file, json_str).map_err(|e| CodeGuardsError::Io {
        path: target_file.clone(),
        source: e,
    })?;

    // Refresh catalog.json
    load_all_tests_and_save_catalog(tests_root)?;

    Ok(target_file)
}
