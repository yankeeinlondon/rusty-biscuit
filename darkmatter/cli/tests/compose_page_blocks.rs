mod common;

use common::CliProcessFixture;
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
    let fixture =
        CliProcessFixture::named("test_compose_motivating_spec_ternary_resolves_when_spec_present");
    fixture.write_file("cwd/plan.md", "# Plan\n");
    fixture.write_file("cwd/spec.md", "# Spec\n");
    fixture.write_file("cwd/template.md", MOTIVATING_SPEC_TERNARY);

    // `file_exists(possible_spec)` is true → the ternary resolves `spec` to the
    // path, the optional `file` field validates, and `::block when="spec"`
    // renders its body. Run from the document directory so the `file`-typed
    // schema fields resolve their references against the sibling files.
    fixture
        .command_builder()
        .ambient_context(fixture.cwd())
        .build()
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
    let fixture =
        CliProcessFixture::named("test_compose_motivating_spec_ternary_absent_when_spec_missing");
    fixture.write_file("cwd/plan.md", "# Plan\n");
    // No spec.md sibling.
    fixture.write_file("cwd/template.md", MOTIVATING_SPEC_TERNARY);

    // `file_exists(possible_spec)` is false → the ternary resolves `spec` to the
    // empty string. Decision A treats an empty non-required `file` field as
    // absent (compose still succeeds) and `::block when="spec"` is excluded.
    fixture
        .command_builder()
        .ambient_context(fixture.cwd())
        .build()
        .arg("compose")
        .arg("template.md")
        .assert()
        .success()
        .stdout(predicate::str::contains("Spec resolved:").not());
}
