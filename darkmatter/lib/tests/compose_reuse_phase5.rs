//! Cross-pass compose reuse regression tests (perf-followup Phase 5).
//!
//! Guards Finding 35.1 (the effective-state / context hash is hoisted to
//! once-per-transclusion-phase and threaded into every directive's cache key)
//! by proving composed output stays byte-identical when a single phase resolves
//! many `::file` directives, and that the shared state key still isolates
//! documents whose frontmatter differs.

use darkmatter::markdown::Markdown;
use darkmatter::markdown::compose::{ComposeOperation, ComposeOptions};
use std::fs;
use tempfile::tempdir;

fn transclude_only(root: &std::path::Path) -> ComposeOptions {
    ComposeOptions::new()
        .with_source_file(root)
        .only(&[
            ComposeOperation::Interpolation,
            ComposeOperation::BlockTransclusion,
        ])
}

/// 35.1: one transclusion phase resolving many `::file` directives to the same
/// child produces the child content once per directive, in order. The
/// phase-wide state/context hash is captured once and reused for every
/// directive's cache key; a regression that recomputed a different value per
/// directive (or corrupted the shared key) would change this output.
#[test]
fn many_file_directives_in_one_phase_compose_byte_identically() {
    let dir = tempdir().unwrap();
    let root = dir.path().join("root.md");
    let child = dir.path().join("child.md");

    fs::write(&child, "Reusable body.\n").unwrap();
    fs::write(
        &root,
        "::file child.md\n\n::file child.md\n\n::file child.md\n\n::file child.md\n",
    )
    .unwrap();

    let md = Markdown::try_from(root.as_path()).unwrap();
    let (composed, _report) = md.compose_with(transclude_only(&root)).unwrap();
    let text = composed.content();

    assert_eq!(
        text.matches("Reusable body.").count(),
        4,
        "each of the four ::file directives must contribute the child body once: {text}"
    );

    // Determinism: re-composing the same source yields byte-identical output.
    let md2 = Markdown::try_from(root.as_path()).unwrap();
    let (composed2, _report2) = md2.compose_with(transclude_only(&root)).unwrap();
    assert_eq!(text, composed2.content());
}

/// 35.1 / Findings 7,16: the hoisted state hash is still part of the cache key,
/// so two parents that transclude the same child but carry different frontmatter
/// state do not cross-contaminate. The child interpolates an inherited value, so
/// a shared-but-wrong key would surface the wrong parent's value.
#[test]
fn differing_parent_state_does_not_reuse_child_output() {
    let dir = tempdir().unwrap();
    let child = dir.path().join("child.md");
    fs::write(&child, "Value is {{ label }}.\n").unwrap();

    let compose = |root: &std::path::Path| {
        let md = Markdown::try_from(root).unwrap();
        let opts = ComposeOptions::new().with_source_file(root).only(&[
            ComposeOperation::Interpolation,
            ComposeOperation::BlockTransclusion,
        ]);
        md.compose_with(opts).unwrap().0.content().to_string()
    };

    // The child inherits the parent frontmatter as external state (see
    // `build_child_external_state`), so `{{ label }}` resolves to the parent's
    // value. The two parents differ only in that value.
    let root_a = dir.path().join("root_a.md");
    fs::write(&root_a, "---\nlabel: alpha\n---\n\n::file child.md\n").unwrap();
    let root_b = dir.path().join("root_b.md");
    fs::write(&root_b, "---\nlabel: beta\n---\n\n::file child.md\n").unwrap();

    let out_a = compose(&root_a);
    let out_b = compose(&root_b);

    assert!(out_a.contains("Value is alpha."), "root_a output: {out_a}");
    assert!(out_b.contains("Value is beta."), "root_b output: {out_b}");
    assert_ne!(out_a, out_b, "differing parent state must not reuse output");
}
