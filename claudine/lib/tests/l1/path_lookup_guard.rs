//! Filesystem canonicalization in the `claudine` library goes through
//! `biscuit_file::canonicalize_simplified` wherever the result leaves a
//! private comparison, and its home directory comes from
//! `biscuit_file::home_dir`, never `dirs::home_dir` or `std::env::home_dir`
//! (one reader, so config reads and writes agree on one home). The guard engine and its self-tests live
//! in `darkmatter/cli/tests/common/path_lookup_guard.rs`.

#[path = "../../../../darkmatter/cli/tests/common/path_lookup_guard.rs"]
mod engine;

use engine::{Exception, Kind, Rule};

const fn private(path: &'static str, item: &'static str, operation: &'static str, count: usize, reason: &'static str) -> Exception {
    Exception { rule: Rule::Canonicalize, kind: Kind::Invariant, path, item, operation, count, reason }
}

const SAME_FILE: &str = "both operands are canonicalized identically and only the \"same file\" verdict leaves";

const EXCEPTIONS: &[Exception] = &[
    private(
        "claudine/lib/src/composition/lifecycle/control.rs",
        "proxy_handoff_allowed",
        "std::fs::canonicalize",
        2,
        SAME_FILE,
    ),
    private(
        "claudine/lib/src/composition/resolve.rs",
        "with_prompt_magic_roots",
        "fs::canonicalize",
        2,
        "both the local root and the home are canonicalized; only the \"local root is home\" verdict leaves",
    ),
    private("claudine/lib/src/dispatch/loader.rs", "load_claudine_config", ".canonicalize()", 2, SAME_FILE),
    private(
        "claudine/lib/src/linking/hashing.rs",
        "hash_skill_dir",
        "fs::canonicalize",
        1,
        "the walk root only; the hash takes paths relative to it, so the root's spelling never enters it",
    ),
];

#[test]
fn production_source_canonicalizes_and_reads_home_only_through_the_shared_helpers() {
    // Spelled for CI's test-input index, which does not count the `#[path]`
    // include above (`source-inputs` in Cargo.toml).
    let _ = include_str!("../../../../darkmatter/cli/tests/common/path_lookup_guard.rs");
    let _ = include_str!("../../../../darkmatter/cli/tests/common/source_scan.rs");
    engine::assert_guarded(
        "claudine/lib",
        &biscuit_test_harness::manifest_dir!(),
        &[Rule::Canonicalize, Rule::HomeLookup],
        EXCEPTIONS,
    );
}
