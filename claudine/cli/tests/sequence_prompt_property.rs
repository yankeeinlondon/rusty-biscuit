#![cfg(unix)]

//! Integration tests for `claudine sequence` inline `prompt`-property
//! behavior.
//!
//! Split out of `sequence_cli.rs`: covers the `prompt` frontmatter
//! property running each step inline, and the hard rejection of
//! `interactive: true` and non-string `prompt` properties.

use std::fs;
mod common;
use common::{CliProcessFixture, strip_ansi, write_executable};

// ============================================================================
// Inline mode: a `prompt` frontmatter property switches each step from
// compose (body-as-prompt) to inline-compose (prompt-as-prompt + body
// write-back), mirroring the top-level `inline-compose` command.
// ============================================================================

#[cfg(unix)]
#[test]
fn sequence_with_prompt_property_runs_each_step_inline() {
    // When the source document declares a `prompt` frontmatter property,
    // every sequence step must run as an inline composition: the agent
    // prompt is the composed `prompt` (with per-step `{{state}}`
    // interpolation), NOT the document body, and the provider's output
    // replaces the body on disk. Regression target: the sequence
    // orchestrator previously hardcoded compose mode and sent the body.
    let fixture = CliProcessFixture::named("sequence-prompt-property");
    let prompts_path = fixture.cwd().join("all-prompts.txt");
    let count_path = fixture.cwd().join("call-count.txt");

    let md_file = fixture.cwd().join("seq.md");
    fs::write(
        &md_file,
        r#"---
prompt: |-
  Work on state {{state}}.
sequence:
  - alpha
  - beta
---
SENTINEL ORIGINAL BODY — must never be sent as the agent prompt.
"#,
    )
    .unwrap();

    // Goose stub: record the `-t <prompt>` argument (the agent prompt) for
    // every invocation, then edit the document the way a file-aware inline
    // agent does so each step reaches its closure.
    common::InlineAgentStub::new(&md_file)
        .prelude(
            "prev=''\n\
             for arg in \"$@\"; do\n\
             if [ \"$prev\" = '-t' ]; then\n\
             printf -- '--- invocation ---\\n' >> \"$CLAUDINE_PROMPTS_FILE\"\n\
             printf '%s\\n' \"$arg\" >> \"$CLAUDINE_PROMPTS_FILE\"\n\
             fi\n\
             prev=\"$arg\"\n\
             done\n\
             count=0\n\
             if [ -f \"$CLAUDINE_COUNT_FILE\" ]; then\n\
             IFS= read -r count < \"$CLAUDINE_COUNT_FILE\"\n\
             fi\n\
             count=$((count + 1))\n\
             printf '%s' \"$count\" > \"$CLAUDINE_COUNT_FILE\"\n",
        )
        .body_expression("\"Body produced by inline step $count\n\"")
        .install(fixture.bin_dir(), "goose");

    let assert = fixture
        .command()
        .env("CLAUDINE_PROMPTS_FILE", &prompts_path)
        .env("CLAUDINE_COUNT_FILE", &count_path)
        .args(["sequence", "--goose", md_file.to_str().unwrap()])
        .assert()
        .success();

    // Both steps ran.
    let calls = fs::read_to_string(&count_path).unwrap();
    assert_eq!(calls.trim(), "2", "both inline steps should run");

    let captured = fs::read_to_string(&prompts_path).unwrap();
    // The agent prompt is the composed `prompt` frontmatter, per step.
    assert!(
        captured.contains("Work on state alpha"),
        "step 1 agent prompt should be the composed `prompt` with state=alpha; captured:\n{captured}"
    );
    assert!(
        captured.contains("Work on state beta"),
        "step 2 agent prompt should be the composed `prompt` with state=beta; captured:\n{captured}"
    );
    // Core regression assertion: the body must NOT have been sent as the
    // agent prompt (that is the old compose behavior).
    assert!(
        !captured.contains("SENTINEL ORIGINAL BODY"),
        "the document body must never be sent as the agent prompt in inline mode; captured:\n{captured}"
    );

    // Inline closure rewrote the document body with the provider output and
    // preserved the original frontmatter (`prompt`, `sequence`). The last
    // write-back (step 2) wins on disk.
    let final_doc = fs::read_to_string(&md_file).unwrap();
    assert!(
        final_doc.contains("Body produced by inline step 2"),
        "the document body should be replaced by the final step's output; doc:\n{final_doc}"
    );
    assert!(
        !final_doc.contains("SENTINEL ORIGINAL BODY"),
        "the original body should have been replaced; doc:\n{final_doc}"
    );
    assert!(
        final_doc.contains("prompt:") && final_doc.contains("sequence:"),
        "inline closure must preserve the original frontmatter; doc:\n{final_doc}"
    );

    let stderr = String::from_utf8_lossy(&assert.get_output().stderr);
    let plain = strip_ansi(&stderr);
    assert!(
        plain.contains("2 succeeded"),
        "summary should record both inline steps succeeding; stderr:\n{plain}"
    );
}

// ============================================================================
// Frontmatter `interactive: true` is hard-rejected for sequence
// (2026-06-14-interactive, review-1 medium finding)
// ============================================================================

#[cfg(unix)]
#[test]
fn sequence_rejects_interactive_true_frontmatter_via_cli() {
    // A sequence document that authors `interactive: true` must be rejected
    // up front with the sequence-specific diagnostic, before any provider
    // step is launched. This exercises the rendered CLI error surface (not
    // just the unit-level `reject_sequence_interactive`).
    let fixture = CliProcessFixture::named("sequence-prompt-property");
    let count_path = fixture.cwd().join("call-count.txt");

    let md_file = fixture.cwd().join("seq.md");
    fs::write(
        &md_file,
        "---\ninteractive: true\nsequence:\n  - alpha\n  - beta\n---\nStep {{state}}.\n",
    )
    .unwrap();

    // Provider stub records every invocation so the test can prove no step
    // ever launched.
    write_executable(
        &fixture.bin_dir().join("goose"),
        &format!(
            "#!/bin/sh\necho touched >> {count}\nexit 0\n",
            count = count_path.display()
        ),
    );

    let assert = fixture
        .command()
        .args(["sequence", "--goose", md_file.to_str().unwrap()])
        .assert()
        .failure();

    let stderr = String::from_utf8_lossy(&assert.get_output().stderr);
    let plain = strip_ansi(&stderr);
    // The styled BlockError word-wraps the body, so tokens like
    // `--interactive` and `inline-compose` may break across lines at hyphens,
    // with the frame's `┃` border glyph reinserted at each wrapped line.
    // Strip whitespace and the border glyph before substring-matching so wrap
    // points don't defeat the assertion.
    let collapsed: String = plain
        .chars()
        .filter(|c| !c.is_whitespace() && *c != '┃')
        .collect();
    // The rendered diagnostic must name `interactive: true`, point to the
    // compose/inline-compose commands, and mention the `--interactive`
    // single-run override.
    assert!(
        collapsed.contains("interactive:true"),
        "error should quote the rejected `interactive: true` key; stderr:\n{plain}"
    );
    assert!(
        collapsed.contains("inline-compose"),
        "error should point to compose / inline-compose for dialog prompts; stderr:\n{plain}"
    );
    assert!(
        collapsed.contains("--interactive"),
        "error should mention the --interactive single-run override; stderr:\n{plain}"
    );
    assert!(
        !count_path.exists(),
        "no provider step should launch when interactive: true is rejected"
    );
}

#[cfg(unix)]
#[test]
fn sequence_rejects_non_string_prompt_property() {
    // A `prompt` frontmatter property that is present but not a string is
    // an inline-mode contract violation and must be rejected up front with
    // the same typed error `inline-compose` raises — before any step runs.
    let fixture = CliProcessFixture::named("sequence-prompt-property");
    let count_path = fixture.cwd().join("call-count.txt");

    let md_file = fixture.cwd().join("seq.md");
    fs::write(
        &md_file,
        "---\nprompt: 42\nsequence:\n  - alpha\n---\nBody\n",
    )
    .unwrap();

    write_executable(
        &fixture.bin_dir().join("goose"),
        &format!(
            "#!/bin/sh\necho touched >> {count}\nexit 0\n",
            count = count_path.display()
        ),
    );

    let assert = fixture
        .command()
        .args(["sequence", "--goose", md_file.to_str().unwrap()])
        .assert()
        .failure();

    let stderr = String::from_utf8_lossy(&assert.get_output().stderr);
    let plain = strip_ansi(&stderr);
    assert!(
        plain.to_lowercase().contains("prompt") && plain.contains("number"),
        "expected a prompt wrong-type error naming the `number` type; stderr:\n{plain}"
    );
    assert!(
        !count_path.exists(),
        "no provider session should launch when the prompt property is invalid"
    );
}
