//! Filesystem canonicalization in `biscuit-file` goes through
//! `canonicalize_simplified`, which is also the one direct `dunce` call. The
//! guard engine and its self-tests live in darkmatter-cli
//! (`tests/common/path_lookup_guard.rs`).

#[path = "../../../../darkmatter/cli/tests/common/path_lookup_guard.rs"]
mod engine;

use engine::{Exception, Kind, Rule};

const EXCEPTIONS: &[Exception] = &[
    Exception {
        rule: Rule::Canonicalize,
        kind: Kind::Invariant,
        path: "biscuit-file/lib/src/path_text.rs",
        item: "canonicalize_simplified",
        operation: "dunce::canonicalize",
        count: 1,
        reason: "the shared helper itself: `dunce` drops a verbatim prefix only where the legacy spelling names the same file",
    },
    Exception {
        rule: Rule::Canonicalize,
        kind: Kind::Temporary,
        path: "biscuit-file/lib/src/file_reference/resolve.rs",
        item: "validate_containment",
        operation: "dunce::canonicalize",
        count: 2,
        reason: "same function as the helper, called directly; the candidate's canonical path is carried in the escape error, \
                 so it converts to `canonicalize_simplified`",
    },
    Exception {
        rule: Rule::Canonicalize,
        kind: Kind::Temporary,
        path: "biscuit-file/lib/src/file_reference/glob/roots.rs",
        item: "spelled_as_stored",
        operation: "std::fs::canonicalize",
        count: 1,
        reason: "only the final component's name is compared, so the prefix cannot matter; converts to \
                 `canonicalize_simplified` rather than keeping an exception",
    },
];

#[test]
fn production_source_canonicalizes_only_through_the_shared_helper() {
    // Spelled for CI's test-input index, which does not count the `#[path]`
    // include above (`source-inputs` in Cargo.toml).
    let _ = include_str!("../../../../darkmatter/cli/tests/common/path_lookup_guard.rs");
    let _ = include_str!("../../../../darkmatter/cli/tests/common/source_scan.rs");
    engine::assert_guarded(
        "biscuit-file/lib",
        &biscuit_test_harness::manifest_dir!(),
        &[Rule::Canonicalize],
        EXCEPTIONS,
    );
}
