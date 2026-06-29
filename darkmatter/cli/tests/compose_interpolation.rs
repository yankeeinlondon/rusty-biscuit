mod common;

use common::md_cmd;
use predicates::prelude::*;

#[test]
fn test_compose_frontmatter_interpolation_basic() {
    md_cmd()
        .args(["compose", "-"])
        .write_stdin("---\nbase: /docs\nspec: \"{{base}}/spec.md\"\n---\nSpec: {{spec}}")
        .assert()
        .success()
        .stdout(predicate::str::contains("Spec: /docs/spec.md"));
}

#[test]
fn test_compose_frontmatter_interpolation_nested_state() {
    md_cmd()
        .args([
            "compose",
            "-",
            "--state",
            r#"{"meta":{"base":"/root","author":"Parent"}}"#,
        ])
        .write_stdin("---\nmeta:\n  author: Local\nspec: \"{{meta.base}}/spec.md\"\n---\n{{spec}}")
        .assert()
        .success()
        .stdout(predicate::str::contains("/root/spec.md"));
}

#[test]
fn test_compose_frontmatter_interpolation_ctx_in_frontmatter_only() {
    // ctx.today referenced only in frontmatter — must still resolve
    md_cmd()
        .args(["compose", "-"])
        .write_stdin("---\nstamp: \"{{ctx.today}}\"\n---\nDate: {{stamp}}")
        .assert()
        .success()
        // The date should not be empty
        .stdout(predicate::str::contains("Date: ").and(predicate::str::contains("Date: \n").not()));
}

// =============================================================================
//                FRONTMATTER FALLBACK INTERPOLATION TESTS
// =============================================================================

#[test]
fn test_compose_frontmatter_double_pipe_fallback() {
    // || in frontmatter interpolation should work the same as | (fallback operator).
    // When the variable is empty, the fallback value should be used.
    md_cmd()
        .args(["compose", "-"])
        .write_stdin(
            "---\nplan: \"\"\nresolved: '{{plan || \"plan.md\"}}'\n---\nFile: {{resolved}}",
        )
        .assert()
        .success()
        .stdout(predicate::str::contains("File: plan.md"));
}

#[test]
fn test_compose_frontmatter_double_pipe_with_set_value() {
    // When --set provides a non-empty value, it should take precedence over the fallback
    md_cmd()
        .args(["compose", "-", "--set", r#"{"plan":"custom.md"}"#])
        .write_stdin(
            "---\nplan: \"\"\nresolved: '{{plan || \"plan.md\"}}'\n---\nFile: {{resolved}}",
        )
        .assert()
        .success()
        .stdout(predicate::str::contains("File: custom.md"));
}

#[test]
fn test_compose_frontmatter_nested_quotes_in_interpolation() {
    // Regression test: double quotes inside {{ }} expressions in YAML
    // frontmatter values (e.g., {{ plan || "plan.md" }}) should not break
    // YAML parsing. The frontmatter parser protects expressions before parsing.
    md_cmd()
        .args([
            "compose",
            "-",
            "--set",
            r#"{"topic":"refactor","phase":1}"#,
            "--frontmatter",
        ])
        .write_stdin(
            "---\ntopic: \"\"\nplan: \"\"\nresolved: \"prefix/{{topic}}/{{plan || \"plan.md\"}}\"\n---\nBody: {{topic}}",
        )
        .assert()
        .success()
        .stdout(predicate::str::contains("resolved: prefix/refactor/plan.md"))
        .stdout(predicate::str::contains("Body: refactor"))
        // Must NOT produce double frontmatter
        .stdout(predicate::str::contains("---\n---").not());
}

/// The real-errors reference failure, through the binary (non-TTY → optimistic
/// render keeps SGR/OSC8): a present-but-missing file reference is
/// authoring-fatal, so `md compose` exits non-zero and renders the cause-driven
/// report — root-cause headline, the receiving key, and the focused excerpt that
/// includes the involved keys (`spec`, `iteration`) but not unrelated ones.
///
/// Composing a real file (not stdin) so the `frontmatter()` call resolves its
/// path argument against the file's directory.
#[test]
fn test_compose_invalid_file_reference_reports_cause_not_mechanism() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("prompt.md");
    std::fs::write(
        &path,
        "---\nagent: \"codex\"\nspec: \"does-not-exist-spec.md\"\niteration: \"{{ frontmatter(spec, 'review_iterations') ? frontmatter(spec, 'review_iterations') : 1 }}\"\n---\n# Body\n",
    )
    .unwrap();

    md_cmd()
        .args(["compose", path.to_str().unwrap()])
        .assert()
        .failure()
        // Root cause, not the mechanism word.
        .stderr(predicate::str::contains("invalid file path"))
        .stderr(predicate::str::contains("transform failed").not())
        // Names the receiving frontmatter key and the involved sibling key.
        .stderr(predicate::str::contains("iteration"))
        .stderr(predicate::str::contains("spec:"))
        // Links the prompt file by name.
        .stderr(predicate::str::contains("prompt.md"));
}
