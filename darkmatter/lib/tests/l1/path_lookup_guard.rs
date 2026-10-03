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

/// A direct call whose result leaves the function; it converts to the shared
/// helper.
const fn onward(
    path: &'static str,
    item: &'static str,
    operation: &'static str,
    count: usize,
    reason: &'static str,
) -> Exception {
    Exception { rule: Rule::Canonicalize, kind: Kind::Temporary, path, item, operation, count, reason }
}

const PREFLIGHT_KEY: &str =
    "a preflight source key compared with the keys the other preflight producers write; all must share one spelling";
const SOURCE_CONTEXT: &str = "the `SourceContext` path shown in diagnostics";

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
    onward(
        "darkmatter/lib/src/markdown/compose/link_resolve.rs",
        "resolve_absolute",
        "std::fs::canonicalize",
        1,
        "the returned link target",
    ),
    onward(
        "darkmatter/lib/src/markdown/compose/file_links/discovery.rs",
        "canonicalize",
        "std::fs::canonicalize",
        1,
        "the local wrapper's result is rendered as `display_path` and `component_root`",
    ),
    onward(
        "darkmatter/lib/src/markdown/compose/context/report.rs",
        "ComposeWarning::from_schema_advisory",
        "std::fs::canonicalize",
        1,
        "the path a compose report shows",
    ),
    onward("darkmatter/lib/src/markdown/mod.rs", "Markdown::source_context_for_errors", ".canonicalize()", 1, SOURCE_CONTEXT),
    onward("darkmatter/lib/src/markdown/mod.rs", "Markdown::full_source_context_for_errors", ".canonicalize()", 1, SOURCE_CONTEXT),
    onward("darkmatter/lib/src/markdown/mod.rs", "Markdown::loaded_source_context_for_errors", ".canonicalize()", 1, SOURCE_CONTEXT),
    onward("darkmatter/lib/src/markdown/mod.rs", "Markdown::try_from", ".canonicalize()", 1, SOURCE_CONTEXT),
    onward(
        "darkmatter/lib/src/markdown/compose/transclusion/resolver.rs",
        "resolve_file_reference",
        "std::fs::canonicalize",
        1,
        "`LocalTarget.canonical`, compared with document identities the helper already builds (mixed producers)",
    ),
    onward(
        "darkmatter/lib/src/markdown/compose/cache/hashing.rs",
        "compose_cache_key",
        "std::fs::canonicalize",
        1,
        "a compose cache key shared with other producers (`source_id`)",
    ),
    onward(
        "darkmatter/lib/src/markdown/compose/pipeline/mod.rs",
        "Markdown::run_compose_pipeline_internal",
        "std::fs::canonicalize",
        1,
        PREFLIGHT_KEY,
    ),
    onward(
        "darkmatter/lib/src/markdown/compose/preflight/collect.rs",
        "collect_recursive",
        "std::fs::canonicalize",
        1,
        PREFLIGHT_KEY,
    ),
    onward("darkmatter/lib/src/markdown/compose/preflight/mod.rs", "canonical_key", "std::fs::canonicalize", 1, PREFLIGHT_KEY),
    onward(
        "darkmatter/lib/src/markdown/schemas/resolve.rs",
        "canonical_path",
        ".canonicalize()",
        1,
        "`referenced_files`, shown in `TriggerPayloadCycle` errors and compared by trigger discovery",
    ),
    onward(
        "darkmatter/lib/src/markdown/schemas/triggers/assemble.rs",
        "canonicalize",
        ".canonicalize()",
        1,
        "compared with `referenced_files` across the trigger-discovery seam",
    ),
    onward(
        "darkmatter/lib/src/markdown/compose/expression/path_projection.rs",
        "strip_prefix_any_spelling",
        "std::fs::canonicalize",
        2,
        "compares a raw path against a canonical root (mixed spellings)",
    ),
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
