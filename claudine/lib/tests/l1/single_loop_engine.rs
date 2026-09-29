//! R7 guard (`2026-09-20-lifecycle-handoff-gaps`): the loop is post-checked and
//! has one implementation. The pre-checked `execute_loop` and
//! `execute_loop_with_config` were deleted; these tests fail if either, or any
//! other `execute_loop*` entry point, returns to the library or CLI sources.

use std::collections::BTreeSet;
use std::path::Path;

/// Every `execute_loop*` identifier the sources may contain: the one engine,
/// its private catch-protocol helper, and the CLI's dispatch between a loop
/// and a single run.
const ALLOWED: [&str; 3] = [
    "execute_loop_with_lifecycle",
    "execute_loop_catch_protocol",
    "execute_loop_or_single",
];

/// `execute_loop*` identifiers in every `.rs` file under `root`, each with the
/// first file it appears in.
fn execute_loop_identifiers(root: &Path) -> Vec<(String, String)> {
    let mut found = Vec::new();
    let mut files = 0;
    for entry in walkdir::WalkDir::new(root) {
        let entry = entry.unwrap_or_else(|e| panic!("walk {}: {e}", root.display()));
        if entry.path().extension().is_none_or(|ext| ext != "rs") {
            continue;
        }
        files += 1;
        let text = std::fs::read_to_string(entry.path())
            .unwrap_or_else(|e| panic!("read {}: {e}", entry.path().display()));
        let identifiers: BTreeSet<&str> = text
            .split(|c: char| !(c.is_alphanumeric() || c == '_'))
            .filter(|word| word.starts_with("execute_loop"))
            .collect();
        for identifier in identifiers {
            found.push((identifier.to_string(), entry.path().display().to_string()));
        }
    }
    // Non-vacuity: a wrong root would scan nothing and pass.
    assert!(files > 50, "scanned only {files} source file(s) under {}", root.display());
    found
}

#[test]
fn no_second_loop_engine_in_lib_or_cli_sources() {
    let lib = execute_loop_identifiers(&biscuit_test_harness::manifest_dir!().join("src"));
    let cli = execute_loop_identifiers(&biscuit_test_harness::manifest_dir!().join("../cli/src"));

    let unexpected: Vec<_> = lib
        .iter()
        .chain(&cli)
        .filter(|(identifier, _)| !ALLOWED.contains(&identifier.as_str()))
        .collect();
    assert!(
        unexpected.is_empty(),
        "a loop entry point other than `execute_loop_with_lifecycle` exists (R7): {unexpected:?}"
    );
    // The allowed set is live, so a stale entry cannot mask a rename.
    for allowed in ALLOWED {
        assert!(
            lib.iter().chain(&cli).any(|(identifier, _)| identifier == allowed),
            "`{allowed}` no longer appears; update ALLOWED"
        );
    }
}

#[test]
fn composition_exports_exactly_one_loop_entry_point() {
    // Compile-time: the one engine is reachable where callers import it.
    #[allow(unused_imports)]
    use claudine::composition::execute_loop_with_lifecycle;

    let composition = std::fs::read_to_string(
        biscuit_test_harness::manifest_dir!().join("src/composition/mod.rs"),
    )
    .expect("read composition/mod.rs");
    let reexported: Vec<&str> = composition
        .split(|c: char| !(c.is_alphanumeric() || c == '_'))
        .filter(|word| word.starts_with("execute_loop"))
        .collect();
    assert_eq!(reexported, ["execute_loop_with_lifecycle"], "composition/mod.rs re-exports");

    // `looping` re-exports `engine::*`, so every `pub fn` there is exported too.
    let engine = std::fs::read_to_string(
        biscuit_test_harness::manifest_dir!().join("src/composition/looping/engine.rs"),
    )
    .expect("read looping/engine.rs");
    let public: Vec<&str> = engine
        .lines()
        .filter_map(|line| line.trim_start().strip_prefix("pub fn "))
        .map(|rest| rest.split(|c: char| !(c.is_alphanumeric() || c == '_')).next().unwrap_or(""))
        .collect();
    assert_eq!(public, ["execute_loop_with_lifecycle"], "looping/engine.rs public functions");
}
