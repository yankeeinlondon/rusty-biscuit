//! Filesystem canonicalization in the `darkmatter` library goes through
//! `biscuit_file::canonicalize_simplified` wherever the result leaves a
//! private comparison. The guard engine and its self-tests live in
//! `cli/tests/common/path_lookup_guard.rs`.

#[path = "../../../cli/tests/common/path_lookup_guard.rs"]
mod engine;

use engine::{Exception, Kind, Rule};

const fn private(path: &'static str, item: &'static str, count: usize, reason: &'static str) -> Exception {
    Exception { rule: Rule::Canonicalize, kind: Kind::Invariant, path, item, operation: "std::fs::canonicalize", count, reason }
}

const EXCEPTIONS: &[Exception] = &[
    private(
        "darkmatter/lib/src/markdown/compose/link_normalization.rs",
        "in_context_spelling",
        2,
        "both the target and the anchor are canonicalized; only the relative suffix leaves, rejoined to the authored anchor",
    ),
    private(
        "darkmatter/lib/src/markdown/compose/shell_expansion/mod.rs",
        "cache_key",
        1,
        "the shell cache's only key producer; a spelling mismatch costs a re-run, never a wrong result",
    ),
    private(
        "darkmatter/lib/src/effects/fs_write.rs",
        "canonicalize_with_missing_tail",
        1,
        "both the write target and its boundary go through this function; only the containment verdict and the lexical path leave",
    ),
    Exception {
        rule: Rule::Canonicalize,
        kind: Kind::Invariant,
        path: "darkmatter/lib/src/markdown/output/terminal.rs",
        item: "ImageRenderer::render_image",
        operation: ".canonicalize()",
        count: 2,
        reason: "traversal check: the image and its base are canonicalized alike and only the verdict (and a tracing warning) leaves",
    },
];

#[test]
fn production_source_canonicalizes_only_through_the_shared_helper() {
    // Spelled for CI's test-input index, which does not count the `#[path]`
    // include above (`source-inputs` in Cargo.toml).
    let _ = include_str!("../../../cli/tests/common/path_lookup_guard.rs");
    let _ = include_str!("../../../cli/tests/common/source_scan.rs");
    engine::assert_guarded(
        "darkmatter/lib",
        &biscuit_test_harness::manifest_dir!(),
        &[Rule::Canonicalize],
        EXCEPTIONS,
    );
}
