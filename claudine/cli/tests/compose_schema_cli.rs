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
//! - Eager caller file setters anchor before frontmatter expressions from
//!   both repository-root and package-area launch directories.

#[cfg(unix)]
use std::fs;
#[cfg(unix)]
use tempfile::tempdir;

mod common;
use common::completion::{
    run_complete, seed_cargo_workspace_members as seed_cargo_workspace, write_file,
};
#[cfg(unix)]
use common::{augmented_path, strip_ansi, write_executable};

// ============================================================================
// Non-interactive MissingProperties surface
// ============================================================================

#[cfg(unix)]
#[test]
fn compose_and_inline_bare_sidecar_advisory_render_once_and_silent_suppresses_it() {
    let workspace = tempdir().unwrap();
    let path_dir = workspace.path().join("bin");
    fs::create_dir_all(&path_dir).unwrap();
    let sidecar = workspace.path().join("schema.yaml");
    fs::write(
        &sidecar,
        "source_marker: string(required)\nspec: 'file(eager; required)'\ncaller_spec: 'file(eager; required)'\n",
    )
    .unwrap();
    let md_file = workspace.path().join("plan.md");
    fs::write(
        &md_file,
        "---\n$schema: ./schema.yaml\ntitle: Hello\n---\nPlan.\n",
    )
    .unwrap();
    let inline_file = workspace.path().join("inline.md");
    let inline_source =
        "---\n$schema: ./schema.yaml\nprompt: Update the body.\n---\nOriginal body.\n";
    fs::write(&inline_file, inline_source).unwrap();
    write_executable(
        &path_dir.join("goose"),
        "#!/bin/sh\nprintf 'Updated body.\\n'\nexit 0\n",
    );

    let run = |subcommand: &str, file: &std::path::Path, silent: bool| {
        let mut command = assert_cmd::Command::cargo_bin("claudine").unwrap();
        command
            .env("NO_COLOR", "1")
            .env("CLAUDINE_RENDEZVOUS_REPORT", "false")
            .env("HOME", workspace.path())
            .env("PATH", augmented_path(&path_dir))
            .current_dir(workspace.path())
            .args([subcommand, "--goose"]);
        if silent {
            command.arg("--silent");
        }
        command.arg(file).assert().success().get_output().stderr.clone()
    };

    for (subcommand, file) in [("compose", &md_file), ("inline-compose", &inline_file)] {
        if subcommand == "inline-compose" {
            fs::write(file, inline_source).unwrap();
        }
        let stderr = strip_ansi(&String::from_utf8_lossy(&run(subcommand, file, false)));
        assert_eq!(
            stderr
                .matches("looks like a SimplifiedSchema but has no envelope")
                .count(),
            1,
            "the {subcommand} warning must render exactly once; stderr:\n{stderr}"
        );
        assert!(
            stderr.contains(&sidecar.display().to_string()),
            "{subcommand} stderr:\n{stderr}"
        );

        if subcommand == "inline-compose" {
            assert!(
                fs::read_to_string(file).unwrap().contains("Updated body."),
                "inline-compose must still persist the provider result"
            );
            fs::write(file, inline_source).unwrap();
        }
        let silent_stderr =
            strip_ansi(&String::from_utf8_lossy(&run(subcommand, file, true)));
        assert!(
            !silent_stderr.contains("looks like a SimplifiedSchema but has no envelope"),
            "--silent must suppress the {subcommand} warning; stderr:\n{silent_stderr}"
        );
    }
}

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

    let assert = assert_cmd::Command::cargo_bin("claudine").unwrap()
        .env("NO_COLOR", "1")
        .env("HOME", workspace.path())
        .env("PATH", augmented_path(&path_dir))
        .current_dir(workspace.path())
        .args(["compose", "--goose", md_file.to_str().unwrap()])
        .assert()
        .failure();

    let stderr = String::from_utf8_lossy(&assert.get_output().stderr);
    let plain = strip_ansi(&stderr);
    // Strong assertion: error must be Claudine's typed CompositionError
    // surface, not Darkmatter's raw MarkdownError. Catches regressions
    // where the preflight compose pass runs before schema validation.
    assert!(
        plain.contains("CompositionError"),
        "expected typed CompositionError surface (not raw MarkdownError); stderr:\n{plain}"
    );
    assert!(
        plain.to_lowercase().contains("missing properties"),
        "expected a missing-properties report; stderr:\n{plain}"
    );
    assert!(
        !plain.contains("MarkdownError"),
        "raw MarkdownError leaked instead of typed Claudine error; stderr:\n{plain}"
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
fn compose_frontmatter_interactive_true_still_reports_missing_on_non_tty() {
    // A document with `interactive: true` frontmatter selects interactive
    // session mode by default, but schema collection depends on TTY signals,
    // not on the resolved session interactivity. When stdin/stderr are piped,
    // the missing required property must surface as a typed MissingProperties
    // report without hanging or prompting.
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
interactive: true
---
Plan for {{topic}}.
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

    let assert = assert_cmd::Command::cargo_bin("claudine").unwrap()
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
        plain.contains("CompositionError"),
        "expected typed CompositionError surface (not raw MarkdownError); stderr:\n{plain}"
    );
    assert!(
        plain.to_lowercase().contains("missing properties"),
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

    assert_cmd::Command::cargo_bin("claudine").unwrap()
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

    let assert = assert_cmd::Command::cargo_bin("claudine").unwrap()
        .env("NO_COLOR", "1")
        .env("HOME", workspace.path())
        .env("PATH", augmented_path(&path_dir))
        .current_dir(workspace.path())
        .args(["compose", "--goose", md_file.to_str().unwrap()])
        .assert()
        .failure();

    let stderr = String::from_utf8_lossy(&assert.get_output().stderr);
    let plain = strip_ansi(&stderr);
    // Strong assertion: error must be Claudine's typed CompositionError
    // surface, not Darkmatter's raw MarkdownError.
    assert!(
        plain.contains("CompositionError"),
        "expected typed CompositionError surface (not raw MarkdownError); stderr:\n{plain}"
    );
    assert!(
        plain.to_lowercase().contains("schema validation"),
        "expected a schema validation error; stderr:\n{plain}"
    );
    assert!(
        !plain.contains("MarkdownError"),
        "raw MarkdownError leaked instead of typed Claudine error; stderr:\n{plain}"
    );
    assert!(
        !count_path.exists(),
        "no provider session should have been launched on hard validation; stub recorded a call"
    );
}

// ============================================================================
// SchemaParse focused excerpt + OSC8 link (real-errors review-2 medium)
// ============================================================================

#[cfg(unix)]
#[test]
fn compose_schema_grammar_error_reports_invalid_schema_without_launching_provider() {
    // A bad constraint separator (`,` instead of `;`) in the `$schema.spec`
    // type-and-constraint string is a grammar error. It must surface as the
    // typed `invalid schema` report (not a path-focused `schema load failed`),
    // name the offending property, and never launch the provider.
    let workspace = tempdir().unwrap();
    let path_dir = workspace.path().join("bin");
    fs::create_dir_all(&path_dir).unwrap();
    let count_path = workspace.path().join("call-count.txt");

    let md_file = workspace.path().join("plan.md");
    fs::write(
        &md_file,
        "---\n$schema:\n    spec: file(required, match(**/*spec*.md))\nspec: \"x\"\n---\nPlan.\n",
    )
    .unwrap();

    write_executable(
        &path_dir.join("goose"),
        &format!(
            "#!/bin/sh\necho touched >> {count}\nexit 0\n",
            count = count_path.display()
        ),
    );

    let assert = assert_cmd::Command::cargo_bin("claudine").unwrap()
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
        plain.contains("invalid schema"),
        "expected the `invalid schema` (SchemaParse) header; stderr:\n{plain}"
    );
    assert!(
        plain.contains("spec"),
        "expected the offending property name `spec`; stderr:\n{plain}"
    );
    assert!(
        !plain.to_lowercase().contains("schema load failed"),
        "grammar error must not collapse into path-focused `schema load failed`; stderr:\n{plain}"
    );
    assert!(
        !count_path.exists(),
        "no provider session should have been launched on a schema parse error"
    );
}

#[cfg(unix)]
#[test]
fn compose_schema_grammar_error_force_color_highlights_line_and_links_file() {
    // With color forced on (optimistic terminal), the SchemaParse report must
    // append the frontmatter excerpt with the offending `$schema.spec` line
    // highlighted and an OSC8 link to the prompt file. The excerpt is TTY-gated
    // but `FORCE_COLOR=1` lifts the gate even under a piped stderr.
    let workspace = tempdir().unwrap();
    let path_dir = workspace.path().join("bin");
    fs::create_dir_all(&path_dir).unwrap();

    let md_file = workspace.path().join("plan.md");
    fs::write(
        &md_file,
        "---\n$schema:\n    spec: file(required, match(**/*spec*.md))\nspec: \"x\"\n---\nPlan.\n",
    )
    .unwrap();

    write_executable(
        &path_dir.join("goose"),
        "#!/bin/sh\ncat > /dev/null\nexit 0\n",
    );

    let assert = assert_cmd::Command::cargo_bin("claudine").unwrap()
        .env_remove("NO_COLOR")
        .env("FORCE_COLOR", "1")
        .env("HOME", workspace.path())
        .env("PATH", augmented_path(&path_dir))
        .current_dir(workspace.path())
        .args(["compose", "--goose", md_file.to_str().unwrap()])
        .assert()
        .failure();

    let stderr = String::from_utf8_lossy(&assert.get_output().stderr);
    let plain = strip_ansi(&stderr);

    // The appended YAML excerpt reproduces the offending frontmatter line.
    assert!(
        plain.contains("spec: file(required, match(**/*spec*.md))"),
        "expected the appended frontmatter excerpt with the offending line; stderr:\n{plain}"
    );
    // OSC8 hyperlink wrapping the prompt path survives under FORCE_COLOR.
    assert!(
        stderr.contains("\u{1b}]8;;"),
        "expected an OSC8 link to the prompt file under FORCE_COLOR; raw stderr:\n{stderr}"
    );
    assert!(
        stderr.contains("plan.md"),
        "expected the prompt file name in the linked report; raw stderr:\n{stderr}"
    );
}

// ============================================================================
// Composition-tolerant pre-validation (review-3 regression)
// ============================================================================

#[cfg(unix)]
#[test]
fn compose_template_value_for_required_enum_does_not_fail_pre_validation() {
    // Regression test for the high finding in
    // `features/2026-05-15-schemas/review-3.md`: a schema-constrained
    // value supplied as a template expression (`{{ env.AGENT }}`) must
    // pass pre-validation, because Darkmatter's compose pipeline can
    // resolve the template into a valid enum member before the
    // prepare-time validator runs. Previously the pre-validator rejected
    // the raw string `"{{ env.AGENT }}"` as not matching the enum.
    let workspace = tempdir().unwrap();
    let path_dir = workspace.path().join("bin");
    fs::create_dir_all(&path_dir).unwrap();

    let md_file = workspace.path().join("plan.md");
    fs::write(
        &md_file,
        r#"---
$schema:
  runtime_agent: 'enum(goose; required)'
runtime_agent: '{{ env.AGENT }}'
---
Plan for {{runtime_agent}}.
"#,
    )
    .unwrap();

    // Provider stub that reads the prompt and exits 0 — proves the
    // composition reached the launch step (i.e. pre-validation and
    // preflight did not abort).
    write_executable(
        &path_dir.join("goose"),
        "#!/bin/sh\ncat > /dev/null\nexit 0\n",
    );

    assert_cmd::Command::cargo_bin("claudine").unwrap()
        .env("NO_COLOR", "1")
        .env("HOME", workspace.path())
        .env("PATH", augmented_path(&path_dir))
        .current_dir(workspace.path())
        .args(["compose", "--goose", md_file.to_str().unwrap()])
        .assert()
        .success();
}

// ============================================================================
// Invalid optional setter — drop-and-retry through the CLI
// ============================================================================

#[cfg(unix)]
#[test]
fn compose_invalid_optional_setter_is_dropped_and_run_succeeds() {
    // Regression test for the high-priority finding in
    // `features/2026-05-15-schemas/review-1.md`: an invalid optional value
    // supplied via `key=value` must be elided from the run's overrides on
    // the drop-and-retry pass — not only from source frontmatter — so the
    // composition continues with a warning instead of failing.
    let workspace = tempdir().unwrap();
    let path_dir = workspace.path().join("bin");
    fs::create_dir_all(&path_dir).unwrap();

    let md_file = workspace.path().join("plan.md");
    fs::write(
        &md_file,
        r#"---
$schema:
  topic: 'string(required)'
  count: 'number'
---
Plan for {{topic}}.
"#,
    )
    .unwrap();

    write_executable(
        &path_dir.join("goose"),
        "#!/bin/sh\ncat > /dev/null\nexit 0\n",
    );

    assert_cmd::Command::cargo_bin("claudine").unwrap()
        .env("NO_COLOR", "1")
        .env("HOME", workspace.path())
        .env("PATH", augmented_path(&path_dir))
        .current_dir(workspace.path())
        .args([
            "compose",
            "--goose",
            md_file.to_str().unwrap(),
            "topic=async",
            "count=bad",
        ])
        .assert()
        .success();
}

// ============================================================================
// Loop documents route through the schema-aware prepare wrapper
// ============================================================================

#[cfg(unix)]
#[test]
fn compose_loop_missing_required_surfaces_typed_missing_properties() {
    // Loop documents must surface the same typed `MissingProperties`
    // report as non-loop documents instead of a generic Darkmatter
    // compose failure. Policy: missing required values inside loops fail
    // as MissingProperties; interactive collection is not driven inside
    // loops (see `compose.rs` loop closure comment).
    let workspace = tempdir().unwrap();
    let path_dir = workspace.path().join("bin");
    fs::create_dir_all(&path_dir).unwrap();
    let count_path = workspace.path().join("call-count.txt");

    let md_file = workspace.path().join("loopy.md");
    fs::write(
        &md_file,
        r#"---
$schema:
  topic: 'string(required)'
loop:
  max: 2
  condition: '{{ _loop_count < 1 }}'
---
Plan for {{topic}}.
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

    let assert = assert_cmd::Command::cargo_bin("claudine").unwrap()
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
        plain.contains("CompositionError"),
        "expected typed CompositionError surface for loop docs; stderr:\n{plain}"
    );
    assert!(
        plain.to_lowercase().contains("missing properties"),
        "expected typed MissingProperties report inside loop; stderr:\n{plain}"
    );
    assert!(
        !plain.contains("MarkdownError"),
        "raw MarkdownError leaked for loop doc instead of typed Claudine error; stderr:\n{plain}"
    );
    assert!(
        plain.contains("topic"),
        "expected `topic` property name in report; stderr:\n{plain}"
    );
    assert!(
        !count_path.exists(),
        "provider must not be launched when a loop iteration fails schema validation"
    );
}

// ============================================================================
// inline-compose contract precedence over schema scrub
// ============================================================================

#[cfg(unix)]
#[test]
fn inline_compose_wrong_type_prompt_takes_precedence_over_schema_scrub() {
    // Regression test for review-2 medium finding: with `$schema: { prompt:
    // string }` and a non-string `prompt`, the inline-specific
    // PromptPropertyWrongType contract must surface BEFORE the schema
    // scrub silently drops `prompt` as an invalid optional. The user
    // needs to see "must be a string, got number" — not "prompt missing".
    let workspace = tempdir().unwrap();
    let path_dir = workspace.path().join("bin");
    fs::create_dir_all(&path_dir).unwrap();
    let count_path = workspace.path().join("call-count.txt");

    let md_file = workspace.path().join("inline.md");
    fs::write(
        &md_file,
        r#"---
$schema:
  prompt: string
prompt: 123
---
ignored body
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

    let assert = assert_cmd::Command::cargo_bin("claudine").unwrap()
        .env("NO_COLOR", "1")
        .env("HOME", workspace.path())
        .env("PATH", augmented_path(&path_dir))
        .current_dir(workspace.path())
        .args(["inline-compose", "--goose", md_file.to_str().unwrap()])
        .assert()
        .failure();

    let stderr = String::from_utf8_lossy(&assert.get_output().stderr);
    let plain = strip_ansi(&stderr);
    assert!(
        plain.contains("must be a string"),
        "expected PromptPropertyWrongType message; stderr:\n{plain}"
    );
    assert!(
        plain.contains("got number"),
        "expected the wrong type name in the message; stderr:\n{plain}"
    );
    assert!(
        !plain.to_lowercase().contains("missing"),
        "must not report missing prompt when scrub would have dropped it; stderr:\n{plain}"
    );
    assert!(
        !count_path.exists(),
        "no provider session should have been launched"
    );
}

#[cfg(unix)]
#[test]
fn inline_compose_missing_required_surfaces_typed_missing_properties() {
    // Inline-compose schema validation must produce a typed
    // CompositionError (not a raw Darkmatter MarkdownError) when a
    // required property is missing — same contract as direct compose.
    let workspace = tempdir().unwrap();
    let path_dir = workspace.path().join("bin");
    fs::create_dir_all(&path_dir).unwrap();
    let count_path = workspace.path().join("call-count.txt");

    let md_file = workspace.path().join("inline.md");
    fs::write(
        &md_file,
        r#"---
$schema:
  topic: 'string(required)'
prompt: 'Generate a plan for {{topic}}'
---
ignored body
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

    let assert = assert_cmd::Command::cargo_bin("claudine").unwrap()
        .env("NO_COLOR", "1")
        .env("HOME", workspace.path())
        .env("PATH", augmented_path(&path_dir))
        .current_dir(workspace.path())
        .args(["inline-compose", "--goose", md_file.to_str().unwrap()])
        .assert()
        .failure();

    let stderr = String::from_utf8_lossy(&assert.get_output().stderr);
    let plain = strip_ansi(&stderr);
    assert!(
        plain.contains("CompositionError"),
        "expected typed CompositionError surface; stderr:\n{plain}"
    );
    assert!(
        plain.to_lowercase().contains("missing properties"),
        "expected MissingProperties report; stderr:\n{plain}"
    );
    assert!(
        !plain.contains("MarkdownError"),
        "raw MarkdownError leaked instead of typed error; stderr:\n{plain}"
    );
    assert!(
        plain.contains("topic"),
        "expected `topic` property name; stderr:\n{plain}"
    );
    assert!(
        !count_path.exists(),
        "no provider session should have been launched"
    );
}

// ============================================================================
// Post-shell-expansion schema validation (review-6 high finding)
// ============================================================================

/// Helper: write a `.darkmatter-shell-whitelist` file allowing the given
/// command prefixes so frontmatter `$(...)` expressions can execute under
/// claudine's preflight shell approval pipeline.
#[cfg(unix)]
fn write_shell_whitelist(home: &std::path::Path, prefixes: &[&str]) {
    let body: String = prefixes.iter().map(|p| format!("prefix {p}\n")).collect();
    fs::write(home.join(".darkmatter-shell-whitelist"), body).unwrap();
}

#[cfg(unix)]
#[test]
fn compose_shell_expanded_value_satisfying_schema_succeeds() {
    // Regression test for the high finding in `review-6.md`. A frontmatter
    // `$(...)` expression that resolves to a schema-valid enum member must
    // pass: Darkmatter defers compose-time validation for shell-bearing
    // values, and claudine's post-shell re-validation accepts the
    // resolved value.
    let workspace = tempdir().unwrap();
    let path_dir = workspace.path().join("bin");
    fs::create_dir_all(&path_dir).unwrap();
    write_shell_whitelist(workspace.path(), &["echo"]);

    let md_file = workspace.path().join("plan.md");
    fs::write(
        &md_file,
        r#"---
$schema:
  tier: 'enum(small, medium, large; required)'
tier: $(echo small)
---
Plan for tier {{tier}}.
"#,
    )
    .unwrap();

    write_executable(
        &path_dir.join("goose"),
        "#!/bin/sh\ncat > /dev/null\nexit 0\n",
    );

    assert_cmd::Command::cargo_bin("claudine").unwrap()
        .env("NO_COLOR", "1")
        .env("HOME", workspace.path())
        .env("PATH", augmented_path(&path_dir))
        .current_dir(workspace.path())
        .args(["compose", "--goose", md_file.to_str().unwrap()])
        .assert()
        .success();
}

#[cfg(unix)]
#[test]
fn compose_shell_expanded_value_violating_schema_aborts_without_launching_provider() {
    // The companion case: a `$(...)` expression that resolves to a value
    // outside the enum must be rejected by the post-shell validator, and
    // the provider stub must not be launched.
    let workspace = tempdir().unwrap();
    let path_dir = workspace.path().join("bin");
    fs::create_dir_all(&path_dir).unwrap();
    write_shell_whitelist(workspace.path(), &["echo"]);
    let count_path = workspace.path().join("call-count.txt");

    let md_file = workspace.path().join("plan.md");
    fs::write(
        &md_file,
        r#"---
$schema:
  tier: 'enum(small, medium, large; required)'
tier: $(echo huge)
---
Plan for tier {{tier}}.
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

    let assert = assert_cmd::Command::cargo_bin("claudine").unwrap()
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
        plain.to_lowercase().contains("schema validation"),
        "expected post-shell SchemaValidation error; stderr:\n{plain}"
    );
    assert!(
        !count_path.exists(),
        "no provider session should have been launched on post-shell invalid value"
    );
}

#[cfg(unix)]
#[test]
fn inline_compose_shell_expanded_value_violating_schema_aborts_without_launching_provider() {
    // Same contract for inline-compose: a `$(...)` expression that
    // resolves to a value outside the enum must be rejected by the
    // post-shell validator. Provider stub must not be launched.
    let workspace = tempdir().unwrap();
    let path_dir = workspace.path().join("bin");
    fs::create_dir_all(&path_dir).unwrap();
    write_shell_whitelist(workspace.path(), &["echo"]);
    let count_path = workspace.path().join("call-count.txt");

    let md_file = workspace.path().join("inline.md");
    fs::write(
        &md_file,
        r#"---
$schema:
  tier: 'enum(small, medium, large; required)'
prompt: 'Plan for {{tier}}'
tier: $(echo huge)
---
ignored body
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

    let assert = assert_cmd::Command::cargo_bin("claudine").unwrap()
        .env("NO_COLOR", "1")
        .env("HOME", workspace.path())
        .env("PATH", augmented_path(&path_dir))
        .current_dir(workspace.path())
        .args(["inline-compose", "--goose", md_file.to_str().unwrap()])
        .assert()
        .failure();

    let stderr = String::from_utf8_lossy(&assert.get_output().stderr);
    let plain = strip_ansi(&stderr);
    assert!(
        plain.to_lowercase().contains("schema validation"),
        "expected post-shell SchemaValidation for inline-compose; stderr:\n{plain}"
    );
    assert!(
        !count_path.exists(),
        "no provider session should have been launched"
    );
}

// ============================================================================
// Dropped-optional warning visibility (review-6 medium finding)
// ============================================================================

#[cfg(unix)]
#[test]
fn compose_invalid_optional_in_file_emits_visible_warning() {
    // Regression test for the medium finding in `review-6.md`. Optional
    // values that fail validation are silently elided by claudine, with
    // only a `tracing::warn!` event that is off by default. The CLI must
    // surface a user-visible stderr warning naming the dropped property.
    let workspace = tempdir().unwrap();
    let path_dir = workspace.path().join("bin");
    fs::create_dir_all(&path_dir).unwrap();

    let md_file = workspace.path().join("plan.md");
    fs::write(
        &md_file,
        r#"---
$schema:
  topic: 'string(required)'
  count: 'number'
topic: async
count: not-a-number
---
Plan for {{topic}}.
"#,
    )
    .unwrap();

    write_executable(
        &path_dir.join("goose"),
        "#!/bin/sh\ncat > /dev/null\nexit 0\n",
    );

    let assert = assert_cmd::Command::cargo_bin("claudine").unwrap()
        .env("NO_COLOR", "1")
        .env("HOME", workspace.path())
        .env("PATH", augmented_path(&path_dir))
        .current_dir(workspace.path())
        .args(["compose", "--goose", md_file.to_str().unwrap()])
        .assert()
        .success();

    let stderr = String::from_utf8_lossy(&assert.get_output().stderr);
    let plain = strip_ansi(&stderr);
    assert!(
        plain.contains("warning:"),
        "expected user-visible `warning:` line on stderr; stderr:\n{plain}"
    );
    assert!(
        plain.contains("count"),
        "expected dropped property name `count` in warning; stderr:\n{plain}"
    );
    assert!(
        plain.to_lowercase().contains("dropped"),
        "expected `dropped` in the warning message; stderr:\n{plain}"
    );
}

#[cfg(unix)]
#[test]
fn compose_invalid_optional_setter_emits_visible_warning() {
    // Setter-supplied invalid optional values should also surface a
    // user-visible stderr warning naming the dropped property.
    let workspace = tempdir().unwrap();
    let path_dir = workspace.path().join("bin");
    fs::create_dir_all(&path_dir).unwrap();

    let md_file = workspace.path().join("plan.md");
    fs::write(
        &md_file,
        r#"---
$schema:
  topic: 'string(required)'
  count: 'number'
---
Plan for {{topic}}.
"#,
    )
    .unwrap();

    write_executable(
        &path_dir.join("goose"),
        "#!/bin/sh\ncat > /dev/null\nexit 0\n",
    );

    let assert = assert_cmd::Command::cargo_bin("claudine").unwrap()
        .env("NO_COLOR", "1")
        .env("HOME", workspace.path())
        .env("PATH", augmented_path(&path_dir))
        .current_dir(workspace.path())
        .args([
            "compose",
            "--goose",
            md_file.to_str().unwrap(),
            "topic=async",
            "count=bad",
        ])
        .assert()
        .success();

    let stderr = String::from_utf8_lossy(&assert.get_output().stderr);
    let plain = strip_ansi(&stderr);
    assert!(
        plain.contains("warning:"),
        "expected user-visible `warning:` line on stderr; stderr:\n{plain}"
    );
    assert!(
        plain.contains("count"),
        "expected dropped property name `count` in warning; stderr:\n{plain}"
    );
}

// ============================================================================
// Motivating case: lazy `file` output + eager `file` input (eager-files spec)
// ============================================================================
//
// Mirrors `sniff/prompts/plan-review-implementation.md` after the migration in
// `features/2026-06-29-eager-files/spec.md`: `review` is an eager INPUT that
// must exist, `plan` is a lazy OUTPUT path this run is about to create. The
// pair proves the reported bug is fixed end-to-end through the compiled binary:
// a lazy `plan` pointing at a not-yet-existing file composes, while a missing
// eager `review` still aborts before the provider launches.

#[cfg(unix)]
#[test]
fn compose_lazy_plan_output_composes_with_present_eager_review() {
    let workspace = tempdir().unwrap();
    let path_dir = workspace.path().join("bin");
    fs::create_dir_all(&path_dir).unwrap();

    // The eager `review` input exists; the lazy `plan` output names a path that
    // does NOT exist yet (this run would create it).
    fs::write(
        workspace.path().join("design-review.md"),
        "# Review\n",
    )
    .unwrap();

    let md_file = workspace.path().join("plan-review.md");
    fs::write(
        &md_file,
        r#"---
$schema:
  review: 'file(eager; required; match(**/*review*.md))'
  plan: 'file'
  iteration: 'number'
review: design-review.md
plan: plan-1.md
iteration: 1
---
Implement {{plan}} from {{review}}.
"#,
    )
    .unwrap();

    write_executable(
        &path_dir.join("goose"),
        "#!/bin/sh\ncat > /dev/null\nexit 0\n",
    );

    assert_cmd::Command::cargo_bin("claudine").unwrap()
        .env("NO_COLOR", "1")
        .env("HOME", workspace.path())
        .env("PATH", augmented_path(&path_dir))
        .current_dir(workspace.path())
        .args(["compose", "--goose", md_file.to_str().unwrap()])
        .assert()
        .success();
}

#[cfg(unix)]
#[test]
fn compose_missing_eager_review_aborts_without_launching_provider() {
    let workspace = tempdir().unwrap();
    let path_dir = workspace.path().join("bin");
    fs::create_dir_all(&path_dir).unwrap();
    let count_path = workspace.path().join("call-count.txt");

    // Same prompt, but the eager `review` input is absent from disk. The lazy
    // `plan` output is still a not-yet-existing path; only the missing eager
    // `review` may cause the failure.
    let md_file = workspace.path().join("plan-review.md");
    fs::write(
        &md_file,
        r#"---
$schema:
  review: 'file(eager; required; match(**/*review*.md))'
  plan: 'file'
  iteration: 'number'
review: missing-review.md
plan: plan-1.md
iteration: 1
---
Implement {{plan}} from {{review}}.
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

    let assert = assert_cmd::Command::cargo_bin("claudine").unwrap()
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
        plain.contains("CompositionError"),
        "expected typed CompositionError surface; stderr:\n{plain}"
    );
    assert!(
        plain.to_lowercase().contains("schema validation"),
        "missing eager `review` must surface a schema validation error; stderr:\n{plain}"
    );
    assert!(
        plain.contains("review"),
        "expected the offending `review` property; stderr:\n{plain}"
    );
    assert!(
        !count_path.exists(),
        "no provider session should have been launched on a missing eager input"
    );
}

#[test]
fn compose_eager_spec_setter_anchors_before_plan_expression_from_root_and_area() {
    let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(std::path::Path::parent)
        .expect("claudine CLI crate should live two levels below the repository root")
        .to_path_buf();
    let home = tempfile::tempdir().unwrap();

    let area = root.join("claudine");
    let runs = [
        (
            root.as_path(),
            "prompts/plan.md",
            "spec=claudine/cli/tests/fixtures/shipped_plan_route/spec.md",
        ),
        (
            area.as_path(),
            "../prompts/plan.md",
            "spec=cli/tests/fixtures/shipped_plan_route/spec.md",
        ),
    ];

    for (launch_dir, prompt_arg, setter) in runs {
        let assert = assert_cmd::Command::cargo_bin("claudine")
            .unwrap()
            .env("NO_COLOR", "1")
            .env("HOME", home.path())
            .env("CLAUDE_CODE_EXIT", "0")
            .current_dir(launch_dir)
            .args(["compose", "--claude", "--dry-run", prompt_arg, setter])
            .assert()
            .success();
        let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
        assert!(
            stdout.contains(
                "Save the plan as \"claudine/cli/tests/fixtures/shipped_plan_route/plan.md\""
            ),
            "the complete target instruction must be launch-directory invariant; \
             launch_dir={}\nstdout:\n{stdout}",
            launch_dir.display()
        );
        assert!(
            !stdout.contains("prompts/cli/tests/fixtures/shipped_plan_route/plan.md"),
            "the plan expression must not retarget beneath the prompt directory; \
             launch_dir={}\nstdout:\n{stdout}",
            launch_dir.display()
        );
    }
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
            "  status: 'enum(draft, published)'\n",
            "---\n",
            "Plan for {{topic}}.\n",
        ),
    );

    // Use a single-character non-empty partial so the cursor is classified
    // as a setter-name slot (the classifier rejects an empty token here so
    // the shell's native default completion still fires for vanilla cases).
    // Both `t` and `s` fuzzy-match every property name in the schema, so
    // every candidate appears and the full required-then-optional plus
    // declaration-order contract can be asserted in one pass.
    let got = run_complete(ws.path(), &["compose", "prompts/plan.md", "t"]);
    let position = |needle: &str| {
        got.iter()
            .position(|c| c == needle)
            .unwrap_or_else(|| panic!("expected `{needle}` in candidates: {got:?}"))
    };
    let topic = position("topic=");
    let tier = position("tier=");
    let description = position("description=");
    let status = position("status=");

    // Required group precedes optional group.
    assert!(
        topic < description && topic < status,
        "required `topic=` must precede optional candidates: {got:?}",
    );
    assert!(
        tier < description && tier < status,
        "required `tier=` must precede optional candidates: {got:?}",
    );

    // Declaration order is preserved within the required group:
    // `topic` was authored before `tier`.
    assert!(
        topic < tier,
        "required group must preserve authored order `topic` before `tier`: {got:?}",
    );

    // Declaration order is preserved within the optional group:
    // `description` was authored before `status`.
    assert!(
        description < status,
        "optional group must preserve authored order `description` before `status`: {got:?}",
    );
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

// ============================================================================
// Schema-aware completion for `inline-compose` and `sequence`
// ============================================================================
//
// The completion engine routes the `compose`, `inline-compose`, and
// `sequence` subcommands through the same schema-aware completer; the
// `compose` tests above prove the wiring works end-to-end. Review-3
// requested explicit coverage proving the *other* two subcommands also
// route correctly, so a future regression that drops the schema-aware
// branch for inline-compose or sequence cannot land silently.

#[test]
fn completion_inline_compose_enum_values_from_schema() {
    let ws = common::TestWorkspace::named("complete-inline-schema-enum");
    seed_cargo_workspace(ws.path(), &["pkg"]);
    write_file(
        &ws.path().join("prompts").join("inline.md"),
        concat!(
            "---\n",
            "$schema:\n",
            "  tier: 'enum(small, medium, large; required)'\n",
            "prompt: 'Plan for {{tier}}'\n",
            "---\n",
            "ignored body\n",
        ),
    );

    let got = run_complete(
        ws.path(),
        &["inline-compose", "prompts/inline.md", "tier="],
    );
    assert!(
        got.iter().any(|c| c == "tier='small'"),
        "expected tier='small' from inline-compose schema completer: {got:?}"
    );
    assert!(
        got.iter().any(|c| c == "tier='large'"),
        "expected tier='large' from inline-compose schema completer: {got:?}"
    );
}

#[test]
fn completion_inline_compose_lists_required_properties_first() {
    let ws = common::TestWorkspace::named("complete-inline-required-first");
    seed_cargo_workspace(ws.path(), &["pkg"]);
    write_file(
        &ws.path().join("prompts").join("inline.md"),
        concat!(
            "---\n",
            "$schema:\n",
            "  topic: 'string(required)'\n",
            "  description: string\n",
            "prompt: 'Plan for {{topic}}'\n",
            "---\n",
            "ignored body\n",
        ),
    );

    let got = run_complete(ws.path(), &["inline-compose", "prompts/inline.md", "t"]);
    assert!(
        got.iter().any(|c| c == "topic="),
        "expected required `topic=` candidate via inline-compose: {got:?}"
    );

    // With a fuzzy `d` partial both required and optional could match;
    // the required `description` is absent in this fixture, but a `d`
    // partial against the optional `description` still returns it,
    // proving schema-aware property-name completion fires for the
    // inline-compose path.
    let got_desc = run_complete(ws.path(), &["inline-compose", "prompts/inline.md", "d"]);
    assert!(
        got_desc.iter().any(|c| c == "description="),
        "expected optional `description=` candidate via inline-compose: {got_desc:?}"
    );
}

#[test]
fn completion_sequence_enum_values_from_schema() {
    // The sequence completer reads the per-step prompt file's schema in
    // exactly the same way as compose/inline-compose. To keep the test
    // hermetic we declare a single inline step whose prompt file lives
    // alongside the sequence document.
    let ws = common::TestWorkspace::named("complete-sequence-schema-enum");
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
    // The sequence document itself is the file the user references on
    // the command line. The completer reads its $schema (if any), so
    // declaring the schema directly on the sequence document is the
    // simplest fixture.
    write_file(
        &ws.path().join("sequences").join("plan.seq.md"),
        concat!(
            "---\n",
            "$schema:\n",
            "  tier: 'enum(small, medium, large; required)'\n",
            "sequence:\n",
            "  - prompts/plan.md\n",
            "---\n",
            "ignored body\n",
        ),
    );

    let got = run_complete(
        ws.path(),
        &["sequence", "sequences/plan.seq.md", "tier="],
    );
    assert!(
        got.iter().any(|c| c == "tier='small'"),
        "expected tier='small' from sequence schema completer: {got:?}"
    );
    assert!(
        got.iter().any(|c| c == "tier='medium'"),
        "expected tier='medium' from sequence schema completer: {got:?}"
    );
    assert!(
        got.iter().any(|c| c == "tier='large'"),
        "expected tier='large' from sequence schema completer: {got:?}"
    );
}

#[test]
fn completion_file_bare_falls_back_to_default_glob() {
    let ws = common::TestWorkspace::named("complete-schema-bare-file");
    seed_cargo_workspace(ws.path(), &["pkg"]);
    write_file(
        &ws.path().join("prompts").join("plan.md"),
        concat!(
            "---\n",
            "$schema:\n",
            "  cover: file\n",
            "---\n",
            "Cover at {{cover}}.\n",
        ),
    );
    write_file(&ws.path().join("readme.md"), "# Readme\n");
    write_file(&ws.path().join("draft.txt"), "plain text\n");

    let got = run_complete(ws.path(), &["compose", "prompts/plan.md", "cover="]);
    assert!(
        got.iter().any(|c| c == "cover='readme.md'"),
        "bare file property must surface default-glob markdown: {got:?}"
    );
    assert!(
        !got.iter().any(|c| c.contains("draft.txt")),
        "non-markdown file must not surface in default glob: {got:?}"
    );
}

#[test]
fn completion_file_array_first_file_uses_default_glob() {
    let ws = common::TestWorkspace::named("complete-schema-file-array-first");
    seed_cargo_workspace(ws.path(), &["pkg"]);
    write_file(
        &ws.path().join("prompts").join("plan.md"),
        concat!(
            "---\n",
            "$schema:\n",
            "  attachments: file[]\n",
            "---\n",
            "Attachments: {{attachments}}.\n",
        ),
    );
    write_file(&ws.path().join("notes.md"), "# Notes\n");

    let got = run_complete(ws.path(), &["compose", "prompts/plan.md", "attachments="]);
    assert!(
        got.iter().any(|c| c == "attachments='notes.md'"),
        "file[] first file must complete from default glob: {got:?}"
    );
}

#[test]
fn completion_file_array_trailing_comma_reopens_completion() {
    let ws = common::TestWorkspace::named("complete-schema-file-array-comma");
    seed_cargo_workspace(ws.path(), &["pkg"]);
    write_file(
        &ws.path().join("prompts").join("plan.md"),
        concat!(
            "---\n",
            "$schema:\n",
            "  attachments: file[]\n",
            "---\n",
            "Attachments: {{attachments}}.\n",
        ),
    );
    write_file(&ws.path().join("a.md"), "# A\n");
    write_file(&ws.path().join("b.md"), "# B\n");

    let got = run_complete(ws.path(), &["compose", "prompts/plan.md", "attachments=a.md,"]);
    assert!(
        got.iter().any(|c| c == "attachments='a.md,b.md'"),
        "trailing comma must append a new default-glob file: {got:?}"
    );
    assert!(
        !got.iter().any(|c| c == "attachments='a.md,a.md'"),
        "already-selected file must be excluded: {got:?}"
    );
}

#[test]
fn completion_file_array_continuation_filters_by_active_partial() {
    let ws = common::TestWorkspace::named("complete-schema-file-array-partial");
    seed_cargo_workspace(ws.path(), &["pkg"]);
    write_file(
        &ws.path().join("prompts").join("plan.md"),
        concat!(
            "---\n",
            "$schema:\n",
            "  attachments: file[]\n",
            "---\n",
            "Attachments: {{attachments}}.\n",
        ),
    );
    write_file(&ws.path().join("alpha.md"), "# A\n");
    write_file(&ws.path().join("beta.md"), "# B\n");

    let got = run_complete(
        ws.path(),
        &["compose", "prompts/plan.md", "attachments=alpha.md,b"],
    );
    assert!(
        got.iter().any(|c| c == "attachments='alpha.md,beta.md'"),
        "active partial must filter continuation candidates: {got:?}"
    );
    assert!(
        !got.iter().any(|c| c == "attachments='alpha.md,alpha.md'"),
        "prior file must stay excluded: {got:?}"
    );
}

#[test]
fn completion_file_array_unclosed_quote_round_trips() {
    let ws = common::TestWorkspace::named("complete-schema-file-array-quote");
    seed_cargo_workspace(ws.path(), &["pkg"]);
    write_file(
        &ws.path().join("prompts").join("plan.md"),
        concat!(
            "---\n",
            "$schema:\n",
            "  attachments: file[]\n",
            "---\n",
            "Attachments: {{attachments}}.\n",
        ),
    );
    write_file(&ws.path().join("a.md"), "# A\n");
    write_file(&ws.path().join("b.md"), "# B\n");

    let got = run_complete(ws.path(), &["compose", "prompts/plan.md", "attachments='a.md,b"]);
    assert!(
        got.iter().any(|c| c == "attachments='a.md,b.md'"),
        "unclosed quote must produce a closed single-quoted candidate: {got:?}"
    );
}

#[test]
fn completion_file_array_literal_comma_filename_is_unsupported() {
    // Documents the known limitation: the comma-list parser splits on
    // every top-level comma, so a filename that itself contains a comma
    // cannot be expressed in the exclusion set. The test only asserts
    // non-panic and that some candidate is produced.
    let ws = common::TestWorkspace::named("complete-schema-file-array-comma-limit");
    seed_cargo_workspace(ws.path(), &["pkg"]);
    write_file(
        &ws.path().join("prompts").join("plan.md"),
        concat!(
            "---\n",
            "$schema:\n",
            "  attachments: file[]\n",
            "---\n",
            "Attachments: {{attachments}}.\n",
        ),
    );
    write_file(&ws.path().join("a,b.md"), "# Comma\n");
    write_file(&ws.path().join("b.md"), "# B\n");

    let got = run_complete(ws.path(), &["compose", "prompts/plan.md", "attachments='a,b.md"]);
    assert!(
        !got.is_empty(),
        "comma-named file edge case must produce candidates without panicking: {got:?}"
    );
}

// ============================================================================
// Schema-aware completion for `file(match(...))` values
// ============================================================================
//
// Properties declared as `file(match('*.ext'))` produce filesystem-path
// completion that's filtered to the matching glob. Verifies the CLI
// __complete surface emits `prop='relpath'` candidates for matching files
// and skips non-matching ones.

#[test]
fn completion_file_match_emits_matching_files_only() {
    let ws = common::TestWorkspace::named("complete-schema-file-match");
    seed_cargo_workspace(ws.path(), &["pkg"]);
    write_file(
        &ws.path().join("prompts").join("plan.md"),
        concat!(
            "---\n",
            "$schema:\n",
            "  cover: \"file(match('*.png'))\"\n",
            "---\n",
            "Cover at {{cover}}.\n",
        ),
    );
    // Seed two candidate assets — only the `.png` should appear.
    write_file(&ws.path().join("assets").join("cover.png"), "fake png");
    write_file(&ws.path().join("assets").join("notes.txt"), "fake txt");

    let got = run_complete(ws.path(), &["compose", "prompts/plan.md", "cover="]);
    assert!(
        got.iter().any(|c| c == "cover='assets/cover.png'"),
        "expected cover='assets/cover.png' from file(match()) completer: {got:?}"
    );
    assert!(
        !got.iter().any(|c| c.contains("notes.txt")),
        "non-matching file `notes.txt` must NOT appear: {got:?}"
    );
}

#[test]
fn completion_file_match_emits_path_qualified_glob_matches() {
    // Regression test for review-4 medium finding. A schema like
    // `file(match('src/**/*.rs'))` must match files by their relative
    // path, not just by basename. Previously the completer compared the
    // glob `src/**/*.rs` against `lib.rs` (the basename) and produced no
    // matches, even though Darkmatter validation accepts those same
    // values.
    let ws = common::TestWorkspace::named("complete-schema-file-match-pathglob");
    seed_cargo_workspace(ws.path(), &["pkg"]);
    write_file(
        &ws.path().join("prompts").join("plan.md"),
        concat!(
            "---\n",
            "$schema:\n",
            "  source_code: \"file(match('src/**/*.rs'))\"\n",
            "---\n",
            "Code at {{source_code}}.\n",
        ),
    );
    // Seed files inside and outside `src/` — only files under `src/`
    // should appear in the candidate set.
    write_file(&ws.path().join("src").join("lib.rs"), "// lib");
    write_file(&ws.path().join("src").join("inner").join("mod.rs"), "// mod");
    write_file(&ws.path().join("tests").join("integration.rs"), "// test");

    let got = run_complete(
        ws.path(),
        &["compose", "prompts/plan.md", "source_code="],
    );
    assert!(
        got.iter().any(|c| c == "source_code='src/lib.rs'"),
        "expected source_code='src/lib.rs' from path-qualified glob: {got:?}"
    );
    assert!(
        got.iter().any(|c| c == "source_code='src/inner/mod.rs'"),
        "expected source_code='src/inner/mod.rs' for recursive match: {got:?}"
    );
    assert!(
        !got.iter().any(|c| c.contains("tests/integration.rs")),
        "files outside `src/` must NOT match a path-qualified glob: {got:?}"
    );
}

#[test]
fn completion_file_match_honors_negated_path_qualified_glob() {
    // Regression test for review-4 medium finding (negated pattern half).
    // `!src/**/test_*.rs` must reject files under `src/` whose basename
    // begins with `test_`, while still accepting other `src/**/*.rs`
    // candidates.
    let ws = common::TestWorkspace::named("complete-schema-file-match-negation");
    seed_cargo_workspace(ws.path(), &["pkg"]);
    write_file(
        &ws.path().join("prompts").join("plan.md"),
        concat!(
            "---\n",
            "$schema:\n",
            "  source_code: \"file(match('src/**/*.rs', '!src/**/test_*.rs'))\"\n",
            "---\n",
            "Code at {{source_code}}.\n",
        ),
    );
    write_file(&ws.path().join("src").join("lib.rs"), "// lib");
    write_file(&ws.path().join("src").join("test_helpers.rs"), "// tests");
    write_file(
        &ws.path().join("src").join("inner").join("test_util.rs"),
        "// inner test",
    );

    let got = run_complete(
        ws.path(),
        &["compose", "prompts/plan.md", "source_code="],
    );
    assert!(
        got.iter().any(|c| c == "source_code='src/lib.rs'"),
        "expected source_code='src/lib.rs' to survive negation: {got:?}"
    );
    assert!(
        !got.iter().any(|c| c.contains("test_helpers.rs")),
        "negated pattern must reject `src/test_helpers.rs`: {got:?}"
    );
    assert!(
        !got.iter().any(|c| c.contains("test_util.rs")),
        "negated recursive pattern must reject `src/inner/test_util.rs`: {got:?}"
    );
}
