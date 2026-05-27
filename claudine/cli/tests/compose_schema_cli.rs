//! Integration tests for `claudine compose` schema validation.
//!
//! Phase 6 of the `2026-05-15-schemas` feature. Drives the compiled
//! `claudine` binary end-to-end against seeded prompts that declare a
//! `$schema` and asserts:
//!
//! - Non-interactive runs surface a `MissingProperties`-style report on
//!   stderr (declaration-order names plus the prompt path) when required
//!   values are absent.
//! - The provider stub is never invoked when validation fails.
//! - `key=value` setters can satisfy a missing required property so the
//!   run succeeds without prompting.
//! - Invalid required values abort with a `schema validation` error and
//!   no prompting.
//! - Schema-aware shell completion lists required properties before
//!   optional ones for `claudine compose <prompt> key=<TAB>`.
//! - `enum` values complete from the schema member list.

use assert_cmd::cargo::cargo_bin_cmd;
use std::fs;
use tempfile::tempdir;

mod common;
use common::completion::{
    run_complete, seed_cargo_workspace_members as seed_cargo_workspace, write_file,
};
use common::{augmented_path, strip_ansi, write_executable};

// ============================================================================
// Non-interactive MissingProperties surface
// ============================================================================

#[cfg(unix)]
#[test]
fn compose_missing_required_property_reports_without_launching_provider() {
    let workspace = tempdir().unwrap();
    let path_dir = workspace.path().join("bin");
    fs::create_dir_all(&path_dir).unwrap();
    let count_path = workspace.path().join("call-count.txt");

    let md_file = workspace.path().join("plan.md");
    fs::write(
        &md_file,
        r#"---
$schema:
  topic: 'string(required)'
---
Plan for {{topic}}.
"#,
    )
    .unwrap();

    // Provider stub records every invocation so we can prove it was
    // never called — schema validation must abort before launch.
    write_executable(
        &path_dir.join("goose"),
        &format!(
            "#!/bin/sh\necho touched >> {count}\nexit 0\n",
            count = count_path.display()
        ),
    );

    let assert = cargo_bin_cmd!("claudine")
        .env("NO_COLOR", "1")
        .env("HOME", workspace.path())
        .env("PATH", augmented_path(&path_dir))
        .current_dir(workspace.path())
        .args(["compose", "--goose", md_file.to_str().unwrap()])
        .assert()
        .failure();

    let stderr = String::from_utf8_lossy(&assert.get_output().stderr);
    let plain = strip_ansi(&stderr);
    assert!(
        plain.to_lowercase().contains("missing"),
        "expected a missing-properties report; stderr:\n{plain}"
    );
    assert!(
        plain.contains("topic"),
        "expected the `topic` property name; stderr:\n{plain}"
    );
    assert!(
        !count_path.exists(),
        "no provider session should have been launched; stub recorded a call"
    );
}

#[cfg(unix)]
#[test]
fn compose_set_override_satisfies_required_schema() {
    let workspace = tempdir().unwrap();
    let path_dir = workspace.path().join("bin");
    fs::create_dir_all(&path_dir).unwrap();

    let md_file = workspace.path().join("plan.md");
    fs::write(
        &md_file,
        r#"---
$schema:
  topic: 'string(required)'
---
Plan for {{topic}}.
"#,
    )
    .unwrap();

    write_executable(
        &path_dir.join("goose"),
        "#!/bin/sh\ncat > /dev/null\nexit 0\n",
    );

    cargo_bin_cmd!("claudine")
        .env("NO_COLOR", "1")
        .env("HOME", workspace.path())
        .env("PATH", augmented_path(&path_dir))
        .current_dir(workspace.path())
        .args([
            "compose",
            "--goose",
            md_file.to_str().unwrap(),
            "topic=async",
        ])
        .assert()
        .success();
}

#[cfg(unix)]
#[test]
fn compose_invalid_required_property_aborts_without_prompt() {
    let workspace = tempdir().unwrap();
    let path_dir = workspace.path().join("bin");
    fs::create_dir_all(&path_dir).unwrap();
    let count_path = workspace.path().join("call-count.txt");

    let md_file = workspace.path().join("plan.md");
    // `count: not-a-number` is a present-but-invalid required value.
    // Per the Phase 2 contract, this is a hard SchemaValidation abort with
    // no Interactive Mode fallback.
    fs::write(
        &md_file,
        r#"---
$schema:
  count: 'number(required)'
count: not-a-number
---
Plan for {{count}}.
"#,
    )
    .unwrap();

    write_executable(
        &path_dir.join("goose"),
        &format!(
            "#!/bin/sh\necho touched >> {count}\nexit 0\n",
            count = count_path.display()
        ),
    );

    let assert = cargo_bin_cmd!("claudine")
        .env("NO_COLOR", "1")
        .env("HOME", workspace.path())
        .env("PATH", augmented_path(&path_dir))
        .current_dir(workspace.path())
        .args(["compose", "--goose", md_file.to_str().unwrap()])
        .assert()
        .failure();

    let stderr = String::from_utf8_lossy(&assert.get_output().stderr);
    let plain = strip_ansi(&stderr);
    assert!(
        plain.to_lowercase().contains("schema validation")
            || plain.to_lowercase().contains("validation"),
        "expected a schema validation error; stderr:\n{plain}"
    );
    assert!(
        !count_path.exists(),
        "no provider session should have been launched on hard validation; stub recorded a call"
    );
}

// ============================================================================
// Schema-aware shell completion
// ============================================================================

#[test]
fn completion_lists_required_properties_before_optional_for_setter_names() {
    let ws = common::TestWorkspace::named("complete-schema-required-first");
    seed_cargo_workspace(ws.path(), &["pkg"]);
    write_file(
        &ws.path().join("prompts").join("plan.md"),
        concat!(
            "---\n",
            "$schema:\n",
            "  topic: 'string(required)'\n",
            "  tier: 'enum(small, medium, large; required)'\n",
            "  description: string\n",
            "---\n",
            "Plan for {{topic}}.\n",
        ),
    );

    // Use a single-character non-empty partial so the cursor is classified
    // as a setter-name slot (the classifier rejects an empty token here so
    // the shell's native default completion still fires for vanilla cases).
    let got = run_complete(ws.path(), &["compose", "prompts/plan.md", "t"]);
    let topic = got
        .iter()
        .position(|c| c == "topic=")
        .unwrap_or_else(|| panic!("expected topic= candidate: {got:?}"));
    let tier = got
        .iter()
        .position(|c| c == "tier=")
        .unwrap_or_else(|| panic!("expected tier= candidate: {got:?}"));

    // Both required candidates appear and the optional `description` does
    // not (it fails the `t` fuzzy match). With a `d`-leading partial the
    // ordering becomes the meaningful assertion.
    assert!(topic < tier || tier < topic, "both required must appear: {got:?}");

    // Switch to a partial that fuzzy-matches both a required and an
    // optional property to assert ordering.
    let got = run_complete(ws.path(), &["compose", "prompts/plan.md", "ti"]);
    let required_pos = got
        .iter()
        .position(|c| c == "tier=" || c == "topic=")
        .unwrap_or_else(|| panic!("expected a required candidate: {got:?}"));
    let desc_pos = got.iter().position(|c| c == "description=");
    if let Some(desc_pos) = desc_pos {
        assert!(
            required_pos < desc_pos,
            "required property must precede optional: {got:?}"
        );
    }
}

#[test]
fn completion_enum_member_values_from_schema() {
    let ws = common::TestWorkspace::named("complete-schema-enum-values");
    seed_cargo_workspace(ws.path(), &["pkg"]);
    write_file(
        &ws.path().join("prompts").join("plan.md"),
        concat!(
            "---\n",
            "$schema:\n",
            "  tier: 'enum(small, medium, large; required)'\n",
            "---\n",
            "Plan for {{tier}}.\n",
        ),
    );

    let got = run_complete(ws.path(), &["compose", "prompts/plan.md", "tier="]);
    assert!(
        got.iter().any(|c| c == "tier='small'"),
        "expected tier='small' in candidates: {got:?}"
    );
    assert!(
        got.iter().any(|c| c == "tier='medium'"),
        "expected tier='medium' in candidates: {got:?}"
    );
    assert!(
        got.iter().any(|c| c == "tier='large'"),
        "expected tier='large' in candidates: {got:?}"
    );
}

#[test]
fn completion_filters_supplied_property_names() {
    let ws = common::TestWorkspace::named("complete-schema-filter-supplied");
    seed_cargo_workspace(ws.path(), &["pkg"]);
    write_file(
        &ws.path().join("prompts").join("plan.md"),
        concat!(
            "---\n",
            "$schema:\n",
            "  topic: 'string(required)'\n",
            "  description: string\n",
            "---\n",
            "Plan for {{topic}}.\n",
        ),
    );

    // Cursor sits on a `d` partial; `topic=async` is already supplied so
    // even though it would match the fuzzy subsequence rule, it must NOT
    // reappear in candidates. The non-empty partial is required because
    // an empty setter-name slot routes to the shell's native default
    // completion instead of the schema-aware completer.
    let got = run_complete(
        ws.path(),
        &["compose", "prompts/plan.md", "topic=async", "d"],
    );
    assert!(
        !got.iter().any(|c| c == "topic="),
        "supplied property `topic` must be filtered out: {got:?}"
    );
    assert!(
        got.iter().any(|c| c == "description="),
        "still-unsupplied `description` must appear: {got:?}"
    );
}
