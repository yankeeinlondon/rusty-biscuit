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

use crate::common;
use common::completion::{
    fake_home, run_complete, run_complete_with_home,
    seed_cargo_workspace_members as seed_cargo_workspace, write_file,
};
use common::CliProcessFixture;
#[cfg(unix)]
use common::{strip_ansi, write_executable};

// ============================================================================
// Non-interactive MissingProperties surface
// ============================================================================

#[cfg(unix)]
#[test]
fn compose_and_inline_bare_sidecar_advisory_render_once_and_silent_suppresses_it() {
    let fixture = CliProcessFixture::named("compose-schema-cli");
    let sidecar = fixture.cwd().join("schema.yaml");
    fs::write(
        &sidecar,
        "source_marker: string(required)\nspec: 'file(eager; required)'\ncaller_spec: 'file(eager; required)'\n",
    )
    .unwrap();
    let md_file = fixture.cwd().join("plan.md");
    fs::write(
        &md_file,
        "---\n$schema: ./schema.yaml\ntitle: Hello\n---\nPlan.\n",
    )
    .unwrap();
    let inline_file = fixture.cwd().join("inline.md");
    let inline_source =
        "---\n$schema: ./schema.yaml\nprompt: Update the body.\n---\nOriginal body.\n";
    fs::write(&inline_file, inline_source).unwrap();
    // `compose` ignores the stub's edit; `inline-compose` requires it, because
    // the agent is the writer.
    common::InlineAgentStub::new(&inline_file)
        .body("Updated body.\n")
        .install(fixture.bin_dir(), "goose");

    let run = |subcommand: &str, file: &std::path::Path, silent: bool| {
        let mut command = fixture.command();
        command.args([subcommand, "--goose"]);
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
    let fixture = CliProcessFixture::named("compose-schema-cli");
    let count_path = fixture.cwd().join("call-count.txt");

    let md_file = fixture.cwd().join("plan.md");
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
        &fixture.bin_dir().join("goose"),
        &format!(
            "#!/bin/sh\necho touched >> {count}\nexit 0\n",
            count = count_path.display()
        ),
    );

    let assert = fixture.command()
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

/// A required-property violation is a validation boundary even though reading a
/// missing property is `null`: the run stops before any `start` action and
/// before the provider, under the existing recovery policy. A document without
/// `initialize` fails eagerly with no lifecycle event; one with `initialize`
/// routes the failure once through its `blocked`/`finalize`, where `err` is
/// readable and an absent property is `null` — never a lifecycle error.
#[cfg(unix)]
#[test]
fn a_required_property_violation_stops_before_lifecycle_actions_under_existing_recovery_policy() {
    const STACKS: &str = r#"start:
  stack:
    - action: {append_line: ["events.log", "start"]}
blocked:
  stack:
    - when: "plan"
      action: {append_line: ["events.log", "guard over an absent property"]}
    - action: {append_line: ["events.log", "blocked {{ err.code }} plan=[{{ plan }}]"]}
finalize:
  stack:
    - action: {append_line: ["events.log", "finalize"]}
"#;
    for (case, initialize, expected) in [
        ("eager", "", vec![]),
        (
            "staged",
            "initialize:\n  stack:\n    - action: {append_line: [\"events.log\", \"initialize\"]}\n",
            vec!["initialize", "blocked composition.missing_properties plan=[]", "finalize"],
        ),
    ] {
        let fixture = CliProcessFixture::named("compose-schema-cli");
        fixture.initialize_repository();
        let count_path = fixture.cwd().join("call-count.txt");
        let md_file = fixture.cwd().join("plan.md");
        fs::write(
            &md_file,
            format!("---\n$schema:\n  topic: 'string(required)'\n{initialize}{STACKS}---\nPlan for {{{{topic}}}}.\n"),
        )
        .unwrap();
        write_executable(
            &fixture.bin_dir().join("goose"),
            &format!("#!/bin/sh\necho touched >> {}\nexit 0\n", count_path.display()),
        );

        let assert = fixture.command()
            .args(["compose", "--goose", md_file.to_str().unwrap()])
            .assert()
            .failure();

        let plain = strip_ansi(&String::from_utf8_lossy(&assert.get_output().stderr));
        assert!(plain.to_lowercase().contains("missing properties"), "{case}: {plain}");
        assert!(!plain.contains("undefined variable") && !plain.contains("unknown root"), "{case}: {plain}");
        assert!(!count_path.exists(), "{case}: no provider session may launch");
        let log = fs::read_to_string(fixture.cwd().join("events.log")).unwrap_or_default();
        assert_eq!(log.lines().collect::<Vec<_>>(), expected, "{case}: {plain}");
        assert!(!fixture.audio_spool().exists(), "{case}: no audio is published");
    }
}

/// AC9a / AC13, compose half: a required property authored as a conditional
/// expression is judged *after* the document's own expression has had its
/// chance. With no caller value the expression yields `null`, which — with
/// Interactive Mode denied on a non-TTY — is a launch error naming the
/// property, not a wrong-type failure and not a silent launch. Supplying the
/// input the expression depends on makes the same run succeed.
#[cfg(unix)]
#[test]
fn compose_conditional_required_property_fails_before_launch_when_interaction_is_denied() {
    let fixture = CliProcessFixture::named("compose-schema-cli");
    let count_path = fixture.cwd().join("call-count.txt");

    let md_file = fixture.cwd().join("plan.md");
    fs::write(
        &md_file,
        concat!(
            "---\n",
            "$schema:\n",
            "  spec: 'string'\n",
            "  foo: 'string(required)'\n",
            "foo: \"{{ spec ? spec + '-derived' : null }}\"\n",
            "---\n",
            "Plan for {{foo}}.\n",
        ),
    )
    .unwrap();
    write_executable(
        &fixture.bin_dir().join("goose"),
        &format!(
            "#!/bin/sh\necho touched >> {count}\nexit 0\n",
            count = count_path.display()
        ),
    );

    let run = |args: &[&str]| {
        let mut command = fixture.command();
        command
            .args(["compose", "--goose", md_file.to_str().unwrap()])
            .args(args)
            .assert()
    };

    let assert = run(&[]).failure();
    let plain = strip_ansi(&String::from_utf8_lossy(&assert.get_output().stderr));
    assert!(
        plain.to_lowercase().contains("missing properties"),
        "a null-resolving required property is a missing-property gap, not a type \
         failure; stderr:\n{plain}"
    );
    assert!(plain.contains("foo"), "stderr:\n{plain}");
    assert!(
        !count_path.exists(),
        "compose must never start with a required property unsatisfied"
    );

    run(&["spec=alpha"]).success();
    assert!(
        fs::read_to_string(&count_path).unwrap().contains("touched"),
        "the resolvable expression must neither prompt nor fail"
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
    let fixture = CliProcessFixture::named("compose-schema-cli");
    let count_path = fixture.cwd().join("call-count.txt");

    let md_file = fixture.cwd().join("plan.md");
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
        &fixture.bin_dir().join("goose"),
        &format!(
            "#!/bin/sh\necho touched >> {count}\nexit 0\n",
            count = count_path.display()
        ),
    );

    let assert = fixture.command()
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
    let fixture = CliProcessFixture::named("compose-schema-cli");

    let md_file = fixture.cwd().join("plan.md");
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
        &fixture.bin_dir().join("goose"),
        "#!/bin/sh\ncat > /dev/null\nexit 0\n",
    );

    fixture.command()
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
    let fixture = CliProcessFixture::named("compose-schema-cli");
    let count_path = fixture.cwd().join("call-count.txt");

    let md_file = fixture.cwd().join("plan.md");
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
        &fixture.bin_dir().join("goose"),
        &format!(
            "#!/bin/sh\necho touched >> {count}\nexit 0\n",
            count = count_path.display()
        ),
    );

    let assert = fixture.command()
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
    let fixture = CliProcessFixture::named("compose-schema-cli");
    let count_path = fixture.cwd().join("call-count.txt");

    let md_file = fixture.cwd().join("plan.md");
    fs::write(
        &md_file,
        "---\n$schema:\n    spec: file(required, match(**/*spec*.md))\nspec: \"x\"\n---\nPlan.\n",
    )
    .unwrap();

    write_executable(
        &fixture.bin_dir().join("goose"),
        &format!(
            "#!/bin/sh\necho touched >> {count}\nexit 0\n",
            count = count_path.display()
        ),
    );

    let assert = fixture.command()
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
    let fixture = CliProcessFixture::named("compose-schema-cli");

    let md_file = fixture.cwd().join("plan.md");
    fs::write(
        &md_file,
        "---\n$schema:\n    spec: file(required, match(**/*spec*.md))\nspec: \"x\"\n---\nPlan.\n",
    )
    .unwrap();

    write_executable(
        &fixture.bin_dir().join("goose"),
        "#!/bin/sh\ncat > /dev/null\nexit 0\n",
    );

    let assert = fixture.command()
        .env_remove("NO_COLOR")
        .env("FORCE_COLOR", "1")
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
    let fixture = CliProcessFixture::named("compose-schema-cli");

    let md_file = fixture.cwd().join("plan.md");
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
        &fixture.bin_dir().join("goose"),
        "#!/bin/sh\ncat > /dev/null\nexit 0\n",
    );

    fixture.command()
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
    let fixture = CliProcessFixture::named("compose-schema-cli");

    let md_file = fixture.cwd().join("plan.md");
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
        &fixture.bin_dir().join("goose"),
        "#!/bin/sh\ncat > /dev/null\nexit 0\n",
    );

    fixture.command()
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
    let fixture = CliProcessFixture::named("compose-schema-cli");
    let count_path = fixture.cwd().join("call-count.txt");

    let md_file = fixture.cwd().join("loopy.md");
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
        &fixture.bin_dir().join("goose"),
        &format!(
            "#!/bin/sh\necho touched >> {count}\nexit 0\n",
            count = count_path.display()
        ),
    );

    let assert = fixture.command()
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
    let fixture = CliProcessFixture::named("compose-schema-cli");
    let count_path = fixture.cwd().join("call-count.txt");

    let md_file = fixture.cwd().join("inline.md");
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
        &fixture.bin_dir().join("goose"),
        &format!(
            "#!/bin/sh\necho touched >> {count}\nexit 0\n",
            count = count_path.display()
        ),
    );

    let assert = fixture.command()
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
    // CompositionError (not a raw Darkmatter MarkdownError) when an eager
    // property is missing — same contract as direct compose. (A
    // required-but-not-eager property is deferred to the agent at inline
    // launch, so only `eager` is a launch gate here.)
    let fixture = CliProcessFixture::named("compose-schema-cli");
    let count_path = fixture.cwd().join("call-count.txt");

    let md_file = fixture.cwd().join("inline.md");
    fs::write(
        &md_file,
        r#"---
$schema:
  topic: 'string(required;eager)'
prompt: 'Generate a plan for {{topic}}'
---
ignored body
"#,
    )
    .unwrap();

    write_executable(
        &fixture.bin_dir().join("goose"),
        &format!(
            "#!/bin/sh\necho touched >> {count}\nexit 0\n",
            count = count_path.display()
        ),
    );

    let assert = fixture.command()
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
    let fixture = CliProcessFixture::named("compose-schema-cli");
    write_shell_whitelist(fixture.cwd(), &["echo"]);

    let md_file = fixture.cwd().join("plan.md");
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
        &fixture.bin_dir().join("goose"),
        "#!/bin/sh\ncat > /dev/null\nexit 0\n",
    );

    fixture.command()
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
    let fixture = CliProcessFixture::named("compose-schema-cli");
    write_shell_whitelist(fixture.cwd(), &["echo"]);
    let count_path = fixture.cwd().join("call-count.txt");

    let md_file = fixture.cwd().join("plan.md");
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
        &fixture.bin_dir().join("goose"),
        &format!(
            "#!/bin/sh\necho touched >> {count}\nexit 0\n",
            count = count_path.display()
        ),
    );

    let assert = fixture.command()
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
    let fixture = CliProcessFixture::named("compose-schema-cli");
    write_shell_whitelist(fixture.cwd(), &["echo"]);
    let count_path = fixture.cwd().join("call-count.txt");

    let md_file = fixture.cwd().join("inline.md");
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
        &fixture.bin_dir().join("goose"),
        &format!(
            "#!/bin/sh\necho touched >> {count}\nexit 0\n",
            count = count_path.display()
        ),
    );

    let assert = fixture.command()
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
    let fixture = CliProcessFixture::named("compose-schema-cli");

    let md_file = fixture.cwd().join("plan.md");
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
        &fixture.bin_dir().join("goose"),
        "#!/bin/sh\ncat > /dev/null\nexit 0\n",
    );

    let assert = fixture.command()
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
    let fixture = CliProcessFixture::named("compose-schema-cli");

    let md_file = fixture.cwd().join("plan.md");
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
        &fixture.bin_dir().join("goose"),
        "#!/bin/sh\ncat > /dev/null\nexit 0\n",
    );

    let assert = fixture.command()
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
    let fixture = CliProcessFixture::named("compose-schema-cli");

    // The eager `review` input exists; the lazy `plan` output names a path that
    // does NOT exist yet (this run would create it).
    fs::write(
        fixture.cwd().join("design-review.md"),
        "# Review\n",
    )
    .unwrap();

    let md_file = fixture.cwd().join("plan-review.md");
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
        &fixture.bin_dir().join("goose"),
        "#!/bin/sh\ncat > /dev/null\nexit 0\n",
    );

    fixture.command()
        .args(["compose", "--goose", md_file.to_str().unwrap()])
        .assert()
        .success();
}

#[cfg(unix)]
#[test]
fn compose_missing_eager_review_aborts_without_launching_provider() {
    let fixture = CliProcessFixture::named("compose-schema-cli");
    let count_path = fixture.cwd().join("call-count.txt");

    // Same prompt, but the eager `review` input is absent from disk. The lazy
    // `plan` output is still a not-yet-existing path; only the missing eager
    // `review` may cause the failure.
    let md_file = fixture.cwd().join("plan-review.md");
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
        &fixture.bin_dir().join("goose"),
        &format!(
            "#!/bin/sh\necho touched >> {count}\nexit 0\n",
            count = count_path.display()
        ),
    );

    let assert = fixture.command()
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
    let checkout = biscuit_test_harness::manifest_dir!()
        .parent()
        .and_then(std::path::Path::parent)
        .expect("claudine CLI crate should live two levels below the repository root")
        .to_path_buf();

    // The subject is a shipped prompt's plan expression, so a frozen copy of
    // `prompts/plan.md` (intentionally not kept in step with the live prompt)
    // is the artifact under test — but the two launch directories it is
    // exercised from must be repositories this test owns, not the rusty-biscuit
    // checkout. Both files are copied at their checkout-relative paths, so every
    // relative reference in them resolves identically.
    let fixture = CliProcessFixture::named("compose-schema-cli");
    fixture.initialize_repository();
    let root = fixture.cwd().to_path_buf();
    common::prompt_staging::stage_frozen_prompts(&root.join("prompts"), &["plan.md"]);
    let relative = "claudine/cli/tests/fixtures/shipped_plan_route/spec.md";
    common::write(
        &root.join(relative),
        &std::fs::read_to_string(checkout.join(relative))
            .unwrap_or_else(|error| panic!("shipped artifact {relative}: {error}")),
    );
    // `--dry-run` never reaches the provider, but discovery still has to find
    // one; the stub keeps that off the host's installed `claude`, and it fails
    // loudly if the dry run ever does launch it.
    common::write_dry_run_provider_stub(fixture.bin_dir(), "claude");

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
        let assert = fixture
            .command_builder()
            // The invariance claim *is* the launch directory, so each run is
            // pinned to one of the two the copy above staged.
            .ambient_context(launch_dir)
            .build()
            .env("CLAUDE_CODE_EXIT", "0")
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

/// Incident 2: a user prompt's `match(^**/*spec*.md)` completes a spec in
/// the repository the user launched from, spelled as it will resolve there.
#[test]
fn completion_file_match_reads_the_caret_prefix_from_the_launch_repository() {
    let ws = common::TestWorkspace::named("complete-schema-caret-match");
    assert!(common::init_git_repo(ws.path()));
    write_file(
        &ws.path().join("fixes/2026-09-29-ts-review-improvements/spec.md"),
        "# fix\n",
    );
    write_file(&ws.path().join("fixes/2026-09-01-other/spec.md"), "# other\n");
    let home = fake_home(ws.path());
    let prompt = home.join(".claudine/prompts/implement.md");
    write_file(
        &prompt,
        "---\n$schema:\n    - spec: file(required;eager;match(^**/*spec*.md))\n---\nImplement {{ spec }}.\n",
    );

    let got = run_complete_with_home(
        ws.path(),
        &home,
        &["compose", prompt.to_str().unwrap(), "spec=ts-review"],
    );
    assert_eq!(got, ["spec='fixes/2026-09-29-ts-review-improvements/spec.md'"]);
}

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

/// Criterion 26 for completion: an authored directory name is
/// case-sensitive in an absolute pattern as in a bare one, so `DOCS` does not
/// offer `docs/a.md`, even on a case-insensitive filesystem where it opens.
#[test]
fn completion_file_match_directory_names_are_case_sensitive() {
    assert_completion_judges_directory_spelling("complete-schema-file-match-directory-case", "docs", "DOCS", None);
}

/// Criterion 26 for completion with Unicode aliases that lowercasing does not
/// reveal (`ς` for a stored `Σ`, `ß` for a stored `SS`). Where the alias
/// does not open, the checks still run.
#[test]
fn completion_file_match_directory_names_reject_unicode_case_aliases() {
    for (label, stored, authored) in
        [("complete-schema-file-match-sigma", "Σ", "ς"), ("complete-schema-file-match-sharp-s", "SS", "ß")]
    {
        assert_completion_judges_directory_spelling(label, stored, authored, None);
    }
}

/// Criterion 26 for completion below a traversal-only ancestor (mode
/// `0111`): being unable to list it must not offer the mismatched `DOCS`.
#[cfg(unix)]
#[test]
fn completion_file_match_directory_names_below_a_traversal_only_ancestor_are_case_sensitive() {
    assert_completion_judges_directory_spelling(
        "complete-schema-file-match-traversal-only",
        "locked/anchor/docs",
        "locked/anchor/DOCS",
        Some("locked"),
    );
}

/// Shipped `__complete` offers nothing for absolute and bare patterns
/// spelling the stored directory `stored` as `authored`, and offers the file
/// for the stored spelling. With `traversal_only`, that directory is set to
/// mode `0111` first.
fn assert_completion_judges_directory_spelling(label: &str, stored: &str, authored: &str, traversal_only: Option<&str>) {
    let ws = common::TestWorkspace::named(label);
    seed_cargo_workspace(ws.path(), &["pkg"]);
    write_file(&ws.path().join(stored).join("a.md"), "# A\n");
    common::fs_capability::probe_directory_alias(ws.path(), stored, authored);
    let root = biscuit_file::to_portable_string(ws.path());
    let offered = |pattern: &str| {
        write_file(
            &ws.path().join("prompts").join("plan.md"),
            &format!("---\n$schema:\n  spec: \"file(match('{pattern}'))\"\n---\nSee {{{{spec}}}}.\n"),
        );
        run_complete(ws.path(), &["compose", "prompts/plan.md", "spec="])
            .into_iter()
            .filter(|candidate| candidate.contains("a.md"))
            .collect::<Vec<_>>()
    };
    #[cfg(unix)]
    let _guard = match traversal_only {
        Some(dir) => match TraversalOnly::new(&ws.path().join(dir)) {
            Some(guard) => Some(guard),
            None => return,
        },
        None => None,
    };

    for mismatched in [format!("{root}/{authored}/*.md"), format!("{authored}/*.md")] {
        assert_eq!(offered(&mismatched), Vec::<String>::new(), "`{mismatched}`");
    }
    let suffix = format!("{stored}/a.md'");
    let mut exact_patterns = vec![format!("{root}/{stored}/*.md")];
    // A bare pattern's walk starts at the workspace and cannot list a
    // traversal-only folder, so it offers nothing there by design.
    if traversal_only.is_none() {
        exact_patterns.push(format!("{stored}/*.md"));
    }
    for exact in exact_patterns {
        let got = offered(&exact);
        assert!(got.iter().any(|candidate| candidate.ends_with(&suffix)), "`{exact}`: {got:?}");
    }
}

/// Sets a directory to mode `0o111` (traversal only) and restores `0o755` on
/// drop. `None` when this user can still list it (a privileged user).
#[cfg(unix)]
struct TraversalOnly(std::path::PathBuf);

#[cfg(unix)]
impl TraversalOnly {
    fn new(dir: &std::path::Path) -> Option<Self> {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(dir, fs::Permissions::from_mode(0o111)).expect("chmod 111");
        let guard = Self(dir.to_path_buf());
        match fs::read_dir(dir) {
            Err(error) if error.kind() == std::io::ErrorKind::PermissionDenied => Some(guard),
            other => {
                eprintln!("skipping: {} is still listable after chmod 111 ({other:?})", dir.display());
                None
            }
        }
    }
}

#[cfg(unix)]
impl Drop for TraversalOnly {
    fn drop(&mut self) {
        use std::os::unix::fs::PermissionsExt;
        let _ = fs::set_permissions(&self.0, fs::Permissions::from_mode(0o755));
    }
}

// ============================================================================
// Caller-supplied file references (2026-09-27-union-partial-file-completion)
// ============================================================================

/// A `goose` stub that counts its launches, plus the `events.log` path the
/// fixtures' `initialize` stacks append to.
#[cfg(unix)]
fn caller_file_fixture(label: &str) -> (CliProcessFixture, std::path::PathBuf, std::path::PathBuf) {
    let fixture = CliProcessFixture::named(label);
    let count_path = fixture.cwd().join("call-count.txt");
    write_executable(
        &fixture.bin_dir().join("goose"),
        &format!("#!/bin/sh\necho touched >> {}\nexit 0\n", count_path.display()),
    );
    let spec = fixture.cwd().join("features/2026-06-30-style/spec.md");
    fs::create_dir_all(spec.parent().unwrap()).unwrap();
    fs::write(&spec, "---\ntitle: Style\n---\nSpec body.\n").unwrap();
    let events = fixture.cwd().join("events.log");
    (fixture, count_path, events)
}

/// R4 / R7.6: a caller value typed at the launch root and judged by the
/// post-`initialize` verdict names the launch directory, never the document's
/// `prompts/` directory.
///
/// The arms disagree on `spec`'s array shape and the discriminant is only
/// known after composition, so the early supplied-file pass has no single glob
/// and the value reaches the late verdict. Darkmatter resolved it in document
/// context there and reported `…/prompts`.
#[cfg(unix)]
#[test]
fn compose_late_caller_file_verdict_names_the_launch_directory() {
    let (fixture, count_path, events) = caller_file_fixture("compose-late-caller-file");
    let prompts = fixture.cwd().join("prompts");
    fs::create_dir_all(&prompts).unwrap();
    let md_file = prompts.join("plan.md");
    fs::write(
        &md_file,
        concat!(
            "---\n",
            "$schema:\n",
            "  - kind: 'literal(feature)'\n",
            "    spec: 'file(required;match(**/*spec*.md);eager)'\n",
            "  - kind: 'literal(note)'\n",
            "    spec: 'file(required;match(**/*note*.md);eager)[]'\n",
            "kind: \"{{ 'feature' }}\"\n",
            "initialize:\n",
            "  stack:\n",
            "    - action: {append_line: [\"events.log\", \"initialize\"]}\n",
            "---\n",
            "Spec: {{spec}}\n",
        ),
    )
    .unwrap();

    let run = |spec: &str| {
        let output = fixture
            .command()
            .args(["compose", "--goose", "prompts/plan.md", &format!("spec={spec}")])
            // Keep the long temporary paths on one line.
            .env("COLUMNS", "1000")
            .output()
            .unwrap();
        (output.status.success(), strip_ansi(&String::from_utf8_lossy(&output.stderr)))
    };

    // Control: a spec the launch directory resolves composes and launches.
    let (ok, plain) = run("features/2026-06-30-style/spec.md");
    assert!(ok, "a resolvable spec should compose; stderr:\n{plain}");
    assert!(count_path.exists(), "the provider should launch; stderr:\n{plain}");
    fs::remove_file(&count_path).unwrap();
    fs::remove_file(&events).unwrap();

    let (ok, plain) = run("fix");
    assert!(!ok, "`fix` names no file; stderr:\n{plain}");
    let launch_dir = fixture.cwd().canonicalize().unwrap();
    assert!(
        plain.contains(&format!(
            "/spec: no existing file matched reference `fix` while resolving from `{}`",
            launch_dir.display()
        )),
        "the verdict should name the launch directory; stderr:\n{plain}"
    );
    assert!(
        !plain.contains(&format!("`{}`", launch_dir.join("prompts").display())),
        "the verdict must not name the document directory; stderr:\n{plain}"
    );
    assert!(
        fs::read_to_string(&events).is_ok_and(|log| log.contains("initialize")),
        "the failure must come from the post-initialize verdict; stderr:\n{plain}"
    );
    assert!(!count_path.exists(), "no provider should launch; stderr:\n{plain}");
}

/// R2: a caller's eager `file(match)` value that names no file fails before
/// the document's `initialize` runs, for a union whose applicable arm is only
/// clear once a templated sibling is ignored (C1), and for the D1 shape, where
/// both arms declare the property.
#[cfg(unix)]
#[test]
fn compose_unresolved_caller_file_fails_before_initialize_when_non_interactive() {
    let (fixture, count_path, events) = caller_file_fixture("compose-early-caller-file");
    let prompts = fixture.cwd().join("prompts");
    fs::create_dir_all(&prompts).unwrap();
    let initialize = "initialize:\n  stack:\n    - action: {append_line: [\"events.log\", \"initialize\"]}\n";
    for (label, schema) in [
        (
            "templated sibling",
            "$schema:\n  - spec: 'file(required;match(**/*spec*.md);eager)'\n    doc: file\n  \
             - design: 'file(required;match(**/*design*.md))'\n    doc: file\n\
             doc: \"{{spec || design}}\"\n",
        ),
        (
            "D1 two-tree union",
            "$schema:\n  - kind: 'literal(feature)'\n    spec: 'file(required;eager;match(**/features/**/spec.md))'\n  \
             - kind: 'literal(fix)'\n    spec: 'file(required;eager;match(**/fixes/**/spec.md))'\n",
        ),
    ] {
        let md_file = prompts.join("plan.md");
        fs::write(&md_file, format!("---\n{schema}{initialize}---\nSpec: {{{{spec}}}}\n")).unwrap();

        // Control: a resolvable literal passes the early check and runs
        // `initialize`, so the check below is the only thing that stops it.
        // The path is relative to the launch directory, which the document's
        // `prompts/` directory is not.
        let output = fixture
            .command()
            .args(["compose", "--goose", "prompts/plan.md", "spec=features/2026-06-30-style/spec.md"])
            .output()
            .unwrap();
        let plain = strip_ansi(&String::from_utf8_lossy(&output.stderr));
        assert!(output.status.success(), "{label}: control should compose; stderr:\n{plain}");
        assert!(events.exists(), "{label}: control should run initialize; stderr:\n{plain}");
        fs::remove_file(&events).unwrap();
        fs::remove_file(&count_path).unwrap();

        let output = fixture
            .command()
            .args(["compose", "--goose", "prompts/plan.md", "spec=no-such-spec"])
            .env("COLUMNS", "1000")
            .output()
            .unwrap();
        let plain = strip_ansi(&String::from_utf8_lossy(&output.stderr));
        assert!(!output.status.success(), "{label}: stderr:\n{plain}");
        assert!(
            plain.contains(&format!(
                "/spec: no existing file matched reference `no-such-spec` while resolving from `{}`",
                fixture.cwd().canonicalize().unwrap().display()
            )),
            "{label}: expected the caller-origin failure; stderr:\n{plain}"
        );
        assert!(!events.exists(), "{label}: initialize must not run; stderr:\n{plain}");
        assert!(!count_path.exists(), "{label}: no provider should launch; stderr:\n{plain}");
    }
}

/// A caller's file value resolves from the launch directory whatever the
/// schema shape: a single schema, a root union whose discriminator is settled,
/// and one whose arms both accept the value, each outside and inside a Git
/// repository, with the path spelled relative and absolute.
///
/// The document lives in `prompts/`, which holds no `fixes/` tree, so a value
/// judged from the document's directory fails the run.
#[cfg(unix)]
#[test]
fn compose_caller_file_resolves_from_the_launch_directory_for_every_schema_shape() {
    let (fixture, count_path, _events) = caller_file_fixture("compose-caller-file-shapes");
    let spec = fixture.cwd().join("fixes/x/spec.md");
    fs::create_dir_all(spec.parent().unwrap()).unwrap();
    fs::write(&spec, "---\ntitle: X\n---\nSpec body.\n").unwrap();
    let prompts = fixture.cwd().join("prompts");
    fs::create_dir_all(&prompts).unwrap();
    let union = concat!(
        "$schema:\n",
        "  - kind: 'literal(feature)'\n",
        "    spec: 'file(required;eager;match(**/features/**/spec.md))'\n",
        "  - kind: 'literal(fix)'\n",
        "    spec: 'file(required;eager;match(**/fixes/**/spec.md))'\n",
    );
    let shapes = [
        ("single schema", "$schema:\n  spec: 'file(required;eager;match(**/fixes/**/spec.md))'\n".to_string()),
        ("union with a settled discriminator", format!("{union}kind: fix\n")),
        ("undecided union", union.to_string()),
    ];
    let absolute = fixture.cwd().canonicalize().unwrap().join("fixes/x/spec.md");

    for repository in ["outside a repository", "inside a repository"] {
        if repository == "inside a repository" {
            fixture.initialize_repository();
        }
        for (shape, schema) in &shapes {
            fs::write(prompts.join("plan.md"), format!("---\n{schema}---\nSpec: {{{{spec}}}}\n")).unwrap();
            for value in ["fixes/x/spec.md".to_string(), absolute.display().to_string()] {
                let output = fixture
                    .command()
                    .args(["compose", "--goose", "prompts/plan.md", &format!("spec={value}")])
                    .env("COLUMNS", "1000")
                    .output()
                    .unwrap();
                let plain = strip_ansi(&String::from_utf8_lossy(&output.stderr));
                let label = format!("{shape}, {repository}, spec={value}");
                assert!(output.status.success(), "{label}: should compose; stderr:\n{plain}");
                assert!(count_path.exists(), "{label}: the provider should launch; stderr:\n{plain}");
                fs::remove_file(&count_path).unwrap();
            }
        }
    }
}

/// Ruling D1 through `claudine compose`: a file outside an arm's contested
/// `match` glob rules that arm out, while a single schema's glob only suggests.
///
/// Only the `fix` arm requires `severity`, so an undecided union that fails on
/// a missing `severity` for a `fixes/…` path proves the `feature` arm, which
/// would otherwise accept it, was ruled out by its glob.
#[cfg(unix)]
#[test]
fn compose_contested_file_match_selects_and_rejects_union_arms() {
    let (fixture, count_path, _events) = caller_file_fixture("compose-contested-match");
    for tree in ["features", "fixes"] {
        let spec = fixture.cwd().join(tree).join("x/spec.md");
        fs::create_dir_all(spec.parent().unwrap()).unwrap();
        fs::write(&spec, "---\ntitle: X\n---\nSpec body.\n").unwrap();
    }
    let prompts = fixture.cwd().join("prompts");
    fs::create_dir_all(&prompts).unwrap();
    let union = concat!(
        "$schema:\n",
        "  - kind: 'literal(feature)'\n",
        "    spec: 'file(required;eager;match(**/features/**/spec.md))'\n",
        "  - kind: 'literal(fix)'\n",
        "    spec: 'file(required;eager;match(**/fixes/**/spec.md))'\n",
        "    severity: 'string(required)'\n",
    );
    let single = "$schema:\n  spec: 'file(required;eager;match(**/fixes/**/spec.md))'\n".to_string();
    let settled = format!("{union}kind: fix\nseverity: high\n");
    let run = |schema: &str, args: &[&str]| {
        fs::write(prompts.join("plan.md"), format!("---\n{schema}---\nSpec: {{{{spec}}}}\n")).unwrap();
        let _ = fs::remove_file(&count_path);
        let output = fixture
            .command()
            .args(["compose", "--goose", "prompts/plan.md"])
            .args(args)
            .env("COLUMNS", "1000")
            .output()
            .unwrap();
        let plain = strip_ansi(&String::from_utf8_lossy(&output.stderr));
        (output.status.success() && count_path.exists(), plain)
    };

    for (label, schema, args) in [
        ("single schema, matching path", single.as_str(), vec!["spec=fixes/x/spec.md"]),
        ("single schema, path outside its glob", single.as_str(), vec!["spec=features/x/spec.md"]),
        ("settled fix arm, fixes path", settled.as_str(), vec!["spec=fixes/x/spec.md"]),
        ("undecided union, features path", union, vec!["spec=features/x/spec.md"]),
        (
            "undecided union, fixes path with severity",
            union,
            vec!["spec=fixes/x/spec.md", "severity=high"],
        ),
    ] {
        let (launched, stderr) = run(schema, &args);
        assert!(launched, "{label}: the provider should launch; stderr:\n{stderr}");
    }

    let (launched, stderr) = run(&settled, &["spec=features/x/spec.md"]);
    assert!(!launched, "settled fix arm, features path: must not launch; stderr:\n{stderr}");
    assert!(stderr.contains("match(**/fixes/**/spec.md)"), "stderr:\n{stderr}");

    let (launched, stderr) = run(union, &["spec=fixes/x/spec.md"]);
    assert!(!launched, "undecided union, fixes path: must not launch; stderr:\n{stderr}");
    assert!(stderr.contains("severity"), "the fix arm applies; stderr:\n{stderr}");
    assert!(!stderr.contains("match(**/"), "the feature arm is ruled out; stderr:\n{stderr}");
}

#[cfg(unix)]
#[test]
fn compose_enforces_each_root_union_arm_match_before_provider_launch() {
    let (fixture, count_path, _events) = caller_file_fixture("compose-every-arm-match");
    for tree in ["features", "fixes"] {
        let spec = fixture.cwd().join(tree).join("x/spec.md");
        fs::create_dir_all(spec.parent().unwrap()).unwrap();
        fs::write(&spec, "---\ntitle: X\n---\nSpec body.\n").unwrap();
    }
    let prompts = fixture.cwd().join("prompts");
    fs::create_dir_all(&prompts).unwrap();
    fs::write(
        prompts.join("raw.json"),
        r#"{"$schema":"https://json-schema.org/draft/2020-12/schema","type":"object","properties":{"kind":{"const":"other"}},"required":["kind"]}"#,
    )
    .unwrap();
    for (name, schema) in [
        (
            "identical",
            "$schema:\n  - kind: 'literal(fix)'\n    spec: 'file(required;eager;match(**/fixes/**/spec.md))'\n  - kind: 'literal(other)'\n    spec: 'file(required;eager;match(**/fixes/**/spec.md))'\nkind: fix\n",
        ),
        (
            "one-declaration",
            "$schema:\n  - kind: 'literal(feature)'\n    spec: 'file(required;eager;match(**/fixes/**/spec.md))'\n  - kind: 'literal(fix)'\nkind: feature\n",
        ),
        (
            "mixed-schema",
            "$schema:\n  - kind: 'literal(fix)'\n    spec: 'file(required;eager;match(**/fixes/**/spec.md))'\n  - ./raw.json\nkind: fix\n",
        ),
    ] {
        fs::write(prompts.join("plan.md"), format!("---\n{schema}---\nSpec: {{{{spec}}}}\n")).unwrap();
        for (spec, accepted) in [("fixes/x/spec.md", true), ("features/x/spec.md", false)] {
            let _ = fs::remove_file(&count_path);
            let supplied = fixture.cwd().join(spec);
            let output = fixture
                .command()
                .args([
                    "compose",
                    "--goose",
                    "prompts/plan.md",
                    &format!("spec={}", supplied.display()),
                ])
                .env("COLUMNS", "1000")
                .output()
                .unwrap();
            let stderr = strip_ansi(&String::from_utf8_lossy(&output.stderr));
            assert_eq!(count_path.exists(), accepted, "{name}, {spec}: {stderr}");
            assert_eq!(output.status.success(), accepted, "{name}, {spec}: {stderr}");
            if !accepted {
                assert!(stderr.contains("match(**/fixes/**/spec.md)"), "{name}, {spec}: {stderr}");
            }
        }
    }
}
