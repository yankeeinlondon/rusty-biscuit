mod common;

use common::md_cmd;
use predicates::prelude::*;

// =============================================================================
//        MOTIVATING `spec: file` FRONTMATTER TERNARY (RESOLUTION CONTEXT)
// =============================================================================

/// Builds the motivating document: an optional `spec` `file` field derived from
/// whether a sibling `spec.md` exists, gated downstream by `::block when="spec"`.
///
/// This is the end-to-end fixture for the resolution-context feature — the
/// read-side `file_exists` function must evaluate in the **frontmatter** pass
/// (it previously only worked in the body), and Decision A must let the empty
/// `else ''` branch satisfy the non-required `spec: file` schema field.
const MOTIVATING_SPEC_TERNARY: &str = "---\n\
$schema:\n\
\x20 plan: file(required)\n\
\x20 spec: file\n\
possible_spec: \"spec.md\"\n\
spec: \"{{ file_exists(possible_spec) ? possible_spec : '' }}\"\n\
plan: \"plan.md\"\n\
---\n\
# Task\n\
\n\
::block when=\"spec\"\n\
Spec resolved: {{ spec }}\n\
::end-block\n";

#[test]
fn test_compose_motivating_spec_ternary_resolves_when_spec_present() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("plan.md"), "# Plan\n").unwrap();
    std::fs::write(dir.path().join("spec.md"), "# Spec\n").unwrap();
    std::fs::write(dir.path().join("template.md"), MOTIVATING_SPEC_TERNARY).unwrap();

    // `file_exists(possible_spec)` is true → the ternary resolves `spec` to the
    // path, the optional `file` field validates, and `::block when="spec"`
    // renders its body. Run from the document directory so the `file`-typed
    // schema fields resolve their references against the sibling files.
    md_cmd()
        .current_dir(dir.path())
        .arg("compose")
        .arg("--frontmatter")
        .arg("template.md")
        .assert()
        .success()
        .stdout(predicate::str::contains("spec: spec.md"))
        .stdout(predicate::str::contains("Spec resolved: spec.md"));
}

#[test]
fn test_compose_motivating_spec_ternary_absent_when_spec_missing() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("plan.md"), "# Plan\n").unwrap();
    // No spec.md sibling.
    std::fs::write(dir.path().join("template.md"), MOTIVATING_SPEC_TERNARY).unwrap();

    // `file_exists(possible_spec)` is false → the ternary resolves `spec` to the
    // empty string. Decision A treats an empty non-required `file` field as
    // absent (compose still succeeds) and `::block when="spec"` is excluded.
    md_cmd()
        .current_dir(dir.path())
        .arg("compose")
        .arg("template.md")
        .assert()
        .success()
        .stdout(predicate::str::contains("Spec resolved:").not());
}
