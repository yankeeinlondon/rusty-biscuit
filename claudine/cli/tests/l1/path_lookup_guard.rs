//! Filesystem canonicalization in the `claudine` binary goes through
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

/// A direct call whose result leaves the function; it converts to the shared
/// helper.
const fn onward(path: &'static str, item: &'static str, reason: &'static str) -> Exception {
    Exception { rule: Rule::Canonicalize, kind: Kind::Temporary, path, item, operation: ".canonicalize()", count: 1, reason }
}

const DEDUPE: &str = "dedupe set: every entry is canonicalized the same way and only membership is read";
const SAME_FILE: &str = "both operands are canonicalized identically and only the \"same file\" verdict leaves";

const EXCEPTIONS: &[Exception] = &[
    private(
        "claudine/cli/src/commands/wrap/provider_overlay.rs",
        "copy_entry",
        "fs::canonicalize",
        1,
        "cycle detection: every visited directory is canonicalized the same way and only membership is read",
    ),
    private("claudine/cli/src/commands/wrap/runaway_guard.rs", "resolve_guard_inputs", ".canonicalize()", 2, SAME_FILE),
    private("claudine/cli/src/completion/composition/compose.rs", "gather_repo_dirs", "std::fs::canonicalize", 1, DEDUPE),
    private("claudine/cli/src/completion/composition/compose.rs", "render_entry_word", "std::fs::canonicalize", 1, DEDUPE),
    private(
        "claudine/cli/src/completion/composition/setter_value.rs",
        "gather_committed_root",
        "std::fs::canonicalize",
        1,
        DEDUPE,
    ),
    private("claudine/cli/src/completion/setter_value.rs", "gather_value_candidates", "std::fs::canonicalize", 1, DEDUPE),
    private("claudine/cli/src/completion/operation_file.rs", "gather_candidates", "std::fs::canonicalize", 1, DEDUPE),
    private(
        "claudine/cli/src/completion/schema_completion/mod.rs",
        "in_repository_spelling",
        ".canonicalize()",
        2,
        "the file and its root are canonicalized alike; only the relative suffix leaves, rejoined to the authored root",
    ),
    onward("claudine/cli/src/commands/compose/interrupt.rs", "format_user_interrupt_message", "a file URL in the interrupt notice"),
    onward("claudine/cli/src/completion/autocomplete_ui.rs", "path_label", "a completion display label"),
    onward("claudine/cli/src/completion/autocomplete_ui.rs", "file_href", "a completion file URL"),
];

#[test]
fn production_source_canonicalizes_and_reads_home_only_through_the_shared_helpers() {
    // Spelled for CI's test-input index, which does not count the `#[path]`
    // include above (`source-inputs` in Cargo.toml).
    let _ = include_str!("../../../../darkmatter/cli/tests/common/path_lookup_guard.rs");
    let _ = include_str!("../../../../darkmatter/cli/tests/common/source_scan.rs");
    engine::assert_guarded(
        "claudine/cli",
        &biscuit_test_harness::manifest_dir!(),
        &[Rule::Canonicalize, Rule::HomeLookup],
        EXCEPTIONS,
    );
}
