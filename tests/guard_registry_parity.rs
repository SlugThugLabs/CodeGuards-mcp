//! Fails if the guard catalog and the dispatcher's registry drift apart.
//!
//! The catalog is what a project can enable; the registry is what actually runs.
//! When those disagree, an enabled guard silently evaluates nothing — and before
//! this test existed, the run reported it as *passed*. Any new catalog entry
//! must therefore be either implemented or declared unimplemented.

use codeguards_mcp::guards::runner::{IMPLEMENTED_GUARDS, UNIMPLEMENTED_GUARDS, is_implemented};
use codeguards_mcp::library::builtins::get_builtin_guard_tests;

/// Every guard offered in the catalog is accounted for one way or the other.
#[test]
fn every_catalogued_guard_is_classified() {
    for def in get_builtin_guard_tests() {
        let known = is_implemented(&def.id) || UNIMPLEMENTED_GUARDS.contains(&def.id.as_str());
        assert!(
            known,
            "guard `{}` is in the catalog but neither implemented nor declared \
             unimplemented — an enabled rule would evaluate nothing. Wire it or list it \
             in UNIMPLEMENTED_GUARDS.",
            def.id
        );
    }
}

/// A guard cannot be both implemented and declared missing.
#[test]
fn implemented_and_unimplemented_are_disjoint() {
    for id in UNIMPLEMENTED_GUARDS {
        assert!(
            !is_implemented(id),
            "guard `{id}` is listed as both implemented and unimplemented"
        );
    }
}

/// The registry must not name guards the catalog does not offer.
#[test]
fn registry_ids_exist_in_the_catalog() {
    let catalogued: Vec<String> = get_builtin_guard_tests()
        .into_iter()
        .map(|def| def.id)
        .collect();

    for id in IMPLEMENTED_GUARDS.iter().chain(UNIMPLEMENTED_GUARDS) {
        assert!(
            catalogued.iter().any(|c| c == id),
            "guard `{id}` is in the registry but not in the built-in catalog"
        );
    }
}
