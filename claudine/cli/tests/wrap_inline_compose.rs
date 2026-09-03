//! Integration tests: inline-compose validation, frontmatter preservation, write-back, and dry-run behavior.
//!
//! Split out of the `wrap_commands.rs` god file; shared fixtures live in
//! `common::wrap`.

#[cfg(unix)]
use chrono::Local;
use std::fs;
use tempfile::tempdir;
mod common;
#[cfg(unix)]
use common::wrap::*;
use common::strip_ansi;
#[cfg(unix)]
use common::{augmented_path, write_executable};

#[test]
fn inline_compose_requires_positional_arg() {
    let assert = assert_cmd::Command::cargo_bin("claudine").unwrap()
        .env("NO_COLOR", "1")
        .args(["inline-compose"])
        .assert()
        .code(2);

    let stderr = String::from_utf8_lossy(&assert.get_output().stderr);
    let plain = strip_ansi(&stderr);
    assert!(plain.contains("ARG"), "usage should show ARG positional");
}

#[test]
fn inline_compose_rejects_missing_prompt_property() {
    let workspace = tempdir().unwrap();
    let md_file = workspace.path().join("test.md");
    fs::write(&md_file, "---\ntitle: No prompt\n---\nBody\n").unwrap();

    assert_cmd::Command::cargo_bin("claudine").unwrap()
        .env("NO_COLOR", "1")
        .args(["inline-compose", md_file.to_str().unwrap()])
        .assert()
        .code(1);
}

#[cfg(unix)]
#[test]
fn inline_compose_resolves_env_agent_in_prompt_template() {
    // `{{env.AGENT}}` in the inline-compose `prompt` frontmatter must
    // resolve to the chosen provider's slug after eager target resolution.
    // Uses Goose because its `-t <prompt>` argv delivery is easy to
    // capture, and its plain-stdout output works without structured streams.
    let workspace = tempdir().unwrap();
    let path_dir = workspace.path().join("bin");
    fs::create_dir_all(&path_dir).unwrap();
    seed_minimal_config(workspace.path());

    let md_file = workspace.path().join("inline.md");
    fs::write(
        &md_file,
        "---\nprompt: 'pick: {{env.AGENT}}'\nagent: goose\n---\noriginal body\n",
    )
    .unwrap();

    let captured_args = workspace.path().join("captured_args.txt");
    write_executable(
        &path_dir.join("goose"),
        r#"#!/bin/sh
printf '%s\n' "$@" > "$CLAUDINE_CAPTURED_ARGS"
printf 'updated body content\n'
exit 0
"#,
    );

    assert_cmd::Command::cargo_bin("claudine").unwrap()
        .current_dir(workspace.path())
        .env("NO_COLOR", "1")
        .env("CLAUDINE_RENDEZVOUS_REPORT", "false")
        .env("HOME", workspace.path())
        .env("PATH", &path_dir)
        .env("CLAUDINE_CAPTURED_ARGS", &captured_args)
        .args(["inline-compose", md_file.to_str().unwrap()])
        .assert()
        .success();

    let argv = fs::read_to_string(&captured_args).unwrap();
    assert!(
        argv.contains("pick: goose"),
        "{{{{env.AGENT}}}} should resolve during inline-compose prompt \
         rendering; argv was: {argv:?}"
    );
}

#[cfg(unix)]
#[test]
fn inline_compose_rejects_empty_captured_output() {
    let workspace = tempdir().unwrap();
    let path_dir = workspace.path().join("bin");
    fs::create_dir_all(&path_dir).unwrap();
    seed_minimal_config(workspace.path());

    let md_file = workspace.path().join("test.md");
    fs::write(
        &md_file,
        "---\nprompt: Generate content\n---\nOriginal body\n",
    )
    .unwrap();

    // Agent that produces no replacement body.
    write_executable(
        &path_dir.join("codex"),
        r#"#!/bin/sh
exit 0
"#,
    );

    {
        let assert = assert_cmd::Command::cargo_bin("claudine").unwrap()
            .current_dir(workspace.path())
            .env("NO_COLOR", "1")
            .env("CLAUDINE_RENDEZVOUS_REPORT", "false")
            .env("HOME", workspace.path())
            .env("PATH", &path_dir)
            .args(["inline-compose", "--codex", md_file.to_str().unwrap()])
            .assert();

        let stderr = String::from_utf8_lossy(&assert.get_output().stderr).to_string();
        let plain = strip_ansi(&stderr);
        assert!(
            plain.contains("valid replacement body") || plain.contains("empty response"),
            "should report invalid captured output; stderr was: {plain}"
        );
    }
}

#[cfg(unix)]
#[test]
fn inline_compose_preserves_frontmatter() {
    let workspace = tempdir().unwrap();
    let path_dir = workspace.path().join("bin");
    fs::create_dir_all(&path_dir).unwrap();
    seed_minimal_config(workspace.path());

    let md_file = workspace.path().join("test.md");
    let original = "---\nprompt: Generate content\nlast_updated: 2026-01-01\n---\nOriginal body\n";
    fs::write(&md_file, original).unwrap();

    // Agent returns frontmatter + body on stdout. Claudine should strip the
    // accidental frontmatter wrapper and preserve the original source frontmatter.
    let provider_output =
        "---\nprompt: CHANGED\nlast_updated: 2099-01-01\n---\nNew body from agent\n";
    let escaped = provider_output.replace('\'', "'\\''");

    write_executable(
        &path_dir.join("goose"),
        &format!("#!/bin/sh\nprintf '%s' '{}'\nexit 0\n", escaped),
    );

    let assert = assert_cmd::Command::cargo_bin("claudine").unwrap()
        .current_dir(workspace.path())
        .env("NO_COLOR", "1")
        .env("CLAUDINE_RENDEZVOUS_REPORT", "false")
        .env("HOME", workspace.path())
        .env("PATH", &path_dir)
        .args(["inline-compose", "--goose", md_file.to_str().unwrap()])
        .assert();

    let final_content = fs::read_to_string(&md_file).unwrap();
    assert!(
        final_content.contains("prompt: Generate content"),
        "original frontmatter prompt should be preserved; file: {final_content}"
    );

    let today = Local::now().format("%Y-%m-%d").to_string();
    assert!(
        final_content.contains(&format!("last_updated: {today}")),
        "last_updated should be today; file: {final_content}"
    );

    // Body should be updated
    assert!(
        final_content.contains("New body from agent"),
        "body should be from agent; file: {final_content}"
    );

    let stderr = String::from_utf8_lossy(&assert.get_output().stderr).to_string();
    let plain = strip_ansi(&stderr);
    assert!(
        plain.contains("Preserved original frontmatter"),
        "should report Claudine-managed frontmatter preservation; stderr was: {plain}"
    );
}

#[cfg(unix)]
#[test]
fn inline_compose_harvests_and_refreshes_authorized_response_frontmatter() {
    let workspace = tempdir().unwrap();
    let path_dir = workspace.path().join("bin");
    fs::create_dir_all(&path_dir).unwrap();
    seed_minimal_config(workspace.path());

    let md_file = workspace.path().join("ap.md");
    let prompt_bytes = concat!(
        "prompt: |-\n",
        "    Inventory the wireless access points.  \n",
        "\n",
        "    Preserve literal \\\"quoted\\\" identifiers.\n",
    );
    let original = format!(
        concat!(
            "---\n",
            "{}",
            "response_frontmatter:\n",
            "  - access_points\n",
            "  - generated_by\n",
            "last_updated: '2026-01-01'\n",
            "---\n",
            "Original inventory.\n",
        ),
        prompt_bytes
    );
    fs::write(&md_file, &original).unwrap();

    let run_count = workspace.path().join("run-count");
    let captured_args = workspace.path().join("captured-args");
    write_executable(
        &path_dir.join("goose"),
        r#"#!/bin/sh
printf '%s\n' "$@" > "$CLAUDINE_CAPTURED_ARGS"
if [ -f "$CLAUDINE_RUN_COUNT" ]; then
  printf '%s\n' '---' 'access_points:' '  - Office-v2' '  - Studio-v2' 'generated_by: obedient-stub-v2' '---' 'Refreshed access-point inventory.'
else
  : > "$CLAUDINE_RUN_COUNT"
  printf '%s\n' '---' 'access_points:' '  - Office-v1' '  - Studio-v1' 'generated_by: obedient-stub-v1' '---' 'Initial access-point inventory.'
fi
exit 0
"#,
    );

    let run = || {
        assert_cmd::Command::cargo_bin("claudine")
            .unwrap()
            .current_dir(workspace.path())
            .env("NO_COLOR", "1")
            .env("CLAUDINE_RENDEZVOUS_REPORT", "false")
            .env("HOME", workspace.path())
            .env("PATH", &path_dir)
            .env("CLAUDINE_RUN_COUNT", &run_count)
            .env("CLAUDINE_CAPTURED_ARGS", &captured_args)
            .args(["inline-compose", "--goose", md_file.to_str().unwrap()])
            .assert()
            .success()
    };

    let first = run();
    let first_stderr = strip_ansi(&String::from_utf8_lossy(&first.get_output().stderr));
    assert!(first_stderr.contains("Inserted response frontmatter property \"access_points\""));
    assert!(first_stderr.contains("Inserted response frontmatter property \"generated_by\""));
    let first_content = fs::read_to_string(&md_file).unwrap();
    assert!(first_content.contains(prompt_bytes));
    assert!(first_content.contains("Office-v1"));
    assert!(first_content.contains("generated_by: obedient-stub-v1"));

    let delivered_prompt = fs::read_to_string(&captured_args).unwrap();
    assert!(delivered_prompt.contains("Allowed response frontmatter properties"));
    assert!(delivered_prompt.contains("access_points"));
    assert!(delivered_prompt.contains("generated_by"));

    let second = run();
    let second_stderr = strip_ansi(&String::from_utf8_lossy(&second.get_output().stderr));
    assert!(second_stderr.contains("Refreshed response frontmatter property \"access_points\""));
    assert!(second_stderr.contains("Refreshed response frontmatter property \"generated_by\""));
    let final_content = fs::read_to_string(&md_file).unwrap();
    assert!(final_content.contains(prompt_bytes));
    assert!(final_content.contains("Office-v2"));
    assert!(final_content.contains("Studio-v2"));
    assert!(final_content.contains("generated_by: obedient-stub-v2"));
    assert!(!final_content.contains("Office-v1"));
    assert!(!final_content.contains("obedient-stub-v1"));
    assert_eq!(final_content.matches("access_points:").count(), 1);
    assert_eq!(final_content.matches("generated_by:").count(), 1);
    assert!(final_content.contains("Refreshed access-point inventory."));

    let today = Local::now().format("%Y-%m-%d").to_string();
    assert!(final_content.contains(&format!("last_updated: '{today}'")));
    let markdown: darkmatter::markdown::Markdown = final_content.into();
    let options = claudine::composition::closure::inline_hash_options();
    let stored = darkmatter::markdown::hash::StoredHash::parse(
        markdown.frontmatter().as_map().get("hash").unwrap(),
        "hash",
    )
    .unwrap();
    assert_eq!(stored.kind, darkmatter::markdown::hash::MdHashKind::Simple);
    let comparison = markdown.compare_hash(&stored, &options).unwrap();
    assert!(!comparison.frontmatter_changed && !comparison.body_changed);
}

#[cfg(unix)]
#[test]
fn inline_compose_reports_property_and_unclassified_frontmatter_drift() {
    for (current, expected_notice) in [
        (
            "---\nprompt: Generate content\nadded: value\n---\nOriginal body\n",
            "Frontmatter property \"added\" changed on disk during the run — restored the authored value",
        ),
        (
            "---\nprompt: [\n---\nOriginal body\n",
            "could not be compared property by property — restored the authored frontmatter",
        ),
    ] {
        let workspace = tempdir().unwrap();
        let path_dir = workspace.path().join("bin");
        fs::create_dir_all(&path_dir).unwrap();
        seed_minimal_config(workspace.path());

        let md_file = workspace.path().join("doc.md");
        fs::write(
            &md_file,
            "---\nprompt: Generate content\n---\nOriginal body\n",
        )
        .unwrap();
        write_executable(
            &path_dir.join("goose"),
            r#"#!/bin/sh
printf '%s' "$CLAUDINE_DRIFT_CONTENT" > "$CLAUDINE_DRIFT_TARGET"
printf 'Replacement body\n'
exit 0
"#,
        );

        let assert = assert_cmd::Command::cargo_bin("claudine")
            .unwrap()
            .current_dir(workspace.path())
            .env("NO_COLOR", "1")
            .env("CLAUDINE_RENDEZVOUS_REPORT", "false")
            .env("HOME", workspace.path())
            .env("PATH", &path_dir)
            .env("CLAUDINE_DRIFT_CONTENT", current)
            .env("CLAUDINE_DRIFT_TARGET", &md_file)
            .args(["inline-compose", "--goose", md_file.to_str().unwrap()])
            .assert()
            .success();

        let stderr = strip_ansi(&String::from_utf8_lossy(&assert.get_output().stderr));
        let normalized_stderr = stderr.split_whitespace().collect::<Vec<_>>().join(" ");
        assert!(
            normalized_stderr.contains(expected_notice),
            "expected drift notice {expected_notice:?}; stderr was:\n{stderr}"
        );
        assert!(
            !stderr.contains("The document body changed on disk during the run"),
            "frontmatter-only drift must not produce a body notice; stderr was:\n{stderr}"
        );
    }
}

#[cfg(unix)]
#[test]
fn inline_compose_reports_canonical_value_and_body_drift() {
    let workspace = tempdir().unwrap();
    let path_dir = workspace.path().join("bin");
    fs::create_dir_all(&path_dir).unwrap();
    seed_minimal_config(workspace.path());

    let md_file = workspace.path().join("doc.md");
    let original = concat!(
        "---\n",
        "prompt: |-\n",
        "  First\n",
        "  Second\n",
        "title: Mine\n",
        "---\n",
        "Original body\n",
    );
    fs::write(&md_file, original).unwrap();
    write_executable(
        &path_dir.join("goose"),
        r#"#!/bin/sh
printf '%s' "$CLAUDINE_DRIFT_CONTENT" > "$CLAUDINE_DRIFT_TARGET"
printf 'Replacement body\n'
exit 0
"#,
    );

    let drifted = concat!(
        "---\n",
        "prompt: \"First\\nSecond\"\n",
        "title: Theirs\n",
        "---\n",
        "Changed body\n",
    );
    let assert = assert_cmd::Command::cargo_bin("claudine")
        .unwrap()
        .current_dir(workspace.path())
        .env("NO_COLOR", "1")
        .env("CLAUDINE_RENDEZVOUS_REPORT", "false")
        .env("HOME", workspace.path())
        .env("PATH", &path_dir)
        .env("CLAUDINE_DRIFT_CONTENT", drifted)
        .env("CLAUDINE_DRIFT_TARGET", &md_file)
        .args(["inline-compose", "--goose", md_file.to_str().unwrap()])
        .assert()
        .success();

    let stderr = strip_ansi(&String::from_utf8_lossy(&assert.get_output().stderr));
    let normalized_stderr = stderr.split_whitespace().collect::<Vec<_>>().join(" ");
    assert!(normalized_stderr.contains(
        "Frontmatter property \"title\" changed on disk during the run — restored the authored value"
    ));
    assert!(normalized_stderr.contains(
        "The document body changed on disk during the run — applied the captured replacement body"
    ));
    assert!(!normalized_stderr.contains("agent changed"));

    let written = fs::read_to_string(md_file).unwrap();
    assert!(written.contains("prompt: |-\n  First\n  Second\n"));
    assert!(written.contains("title: Mine\n"));
    assert!(written.ends_with("---\nReplacement body\n"));
}

/// Phase 4 dry-run: `inline-compose --dry-run` runs the full composition
/// pipeline up to (but not including) provider launch, leaves the source
/// file byte-identical (no write-back, `last_updated` untouched), and prints
/// the composed prompt — what *would* be sent — to stdout.
#[cfg(unix)]
#[test]
fn inline_compose_dry_run_leaves_file_unchanged_and_prints_prompt() {
    let workspace = tempdir().unwrap();
    let path_dir = workspace.path().join("bin");
    fs::create_dir_all(&path_dir).unwrap();
    seed_minimal_config(workspace.path());

    let md_file = workspace.path().join("test.md");
    let original =
        "---\nprompt: Generate the documentation\nlast_updated: 2026-01-01\n---\nOriginal body\n";
    fs::write(&md_file, original).unwrap();

    // Provider binary writes a sentinel file when it runs; under --dry-run it
    // must never launch, so the sentinel must be absent afterwards.
    let sentinel = workspace.path().join("provider-ran.flag");
    write_executable(
        &path_dir.join("goose"),
        &format!(
            "#!/bin/sh\ntouch '{}'\nprintf 'New body from agent\\n'\nexit 0\n",
            sentinel.display()
        ),
    );

    let assert = assert_cmd::Command::cargo_bin("claudine").unwrap()
        .env("NO_COLOR", "1")
        .env("HOME", workspace.path())
        .env("PATH", augmented_path(&path_dir))
        .args([
            "inline-compose",
            "--goose",
            "--dry-run",
            md_file.to_str().unwrap(),
        ])
        .assert()
        .success();

    // Source file is byte-identical: no write-back, `last_updated` untouched.
    let final_content = fs::read_to_string(&md_file).unwrap();
    assert_eq!(
        final_content, original,
        "inline-compose --dry-run must not mutate the source file"
    );

    // Provider never launched.
    assert!(
        !sentinel.exists(),
        "provider must not execute under inline-compose --dry-run"
    );

    // Composed prompt (what would be sent) is on stdout.
    let stdout = String::from_utf8_lossy(&assert.get_output().stdout).to_string();
    assert!(
        strip_ansi(&stdout).contains("Generate the documentation"),
        "stdout should carry the composed prompt; stdout was:\n{stdout}"
    );
}

#[cfg(unix)]
#[test]
#[serial_test::serial]
fn inline_compose_writes_only_final_response_not_narration() {
    // Regression: with a structured provider that narrates between tool
    // calls, inline-compose must write ONLY the agent's final response (the
    // output text after the last tool call) into the document body. The
    // interstitial "Let me read…/Now let me write…" narration must never
    // leak into the artifact.
    let workspace = tempdir().unwrap();
    let path_dir = workspace.path().join("bin");
    let fake_home = workspace.path().join("home");
    fs::create_dir_all(&path_dir).unwrap();
    fs::create_dir_all(&fake_home).unwrap();
    seed_minimal_config(&fake_home);

    let md_file = workspace.path().join("doc.md");
    fs::write(
        &md_file,
        "---\nprompt: Generate the document body.\n---\nOriginal placeholder body.\n",
    )
    .unwrap();

    // Claude stream-json stub: narrate, call a tool, narrate again, call a
    // second tool, then emit the FINAL response. Each narration block is a
    // separate text-only assistant message, so without the fix all of them
    // accumulate into `assistant_text` and leak into the body.
    write_executable(
        &path_dir.join("claude"),
        r##"#!/bin/sh
printf '%s\n' '{"type":"system","subtype":"init","session_id":"claude-final","model":"claude-sonnet-4"}'
printf '%s\n' '{"type":"assistant","message":{"content":[{"type":"text","text":"Let me read the research documents first."}]}}'
printf '%s\n' '{"type":"tool_use","name":"read_file","input":{"path":"research.md"}}'
printf '%s\n' '{"type":"assistant","message":{"content":[{"type":"text","text":"Now let me write the document."}]}}'
printf '%s\n' '{"type":"tool_use","name":"write_file","input":{"path":"doc.md"}}'
printf '%s\n' '{"type":"assistant","message":{"content":[{"type":"text","text":"# Final Document\n\nThis is the only content that belongs in the body."}]}}'
printf '%s\n' '{"type":"result","subtype":"success","stop_reason":"end_turn","num_turns":3,"duration_ms":100,"usage":{"input_tokens":3,"output_tokens":60}}'
"##,
    );

    assert_cmd::Command::cargo_bin("claudine").unwrap()
        .current_dir(workspace.path())
        .env("NO_COLOR", "1")
        .env("CLAUDINE_RENDEZVOUS_REPORT", "false")
        .env("HOME", &fake_home)
        .env("PATH", &path_dir)
        .args(["inline-compose", "--claude", md_file.to_str().unwrap()])
        .assert()
        .success();

    let final_doc = fs::read_to_string(&md_file).unwrap();
    assert!(
        final_doc.contains("This is the only content that belongs in the body."),
        "the final response should be written to the body; doc:\n{final_doc}"
    );
    assert!(
        !final_doc.contains("Let me read the research documents first."),
        "first narration block leaked into the body; doc:\n{final_doc}"
    );
    assert!(
        !final_doc.contains("Now let me write the document."),
        "second narration block leaked into the body; doc:\n{final_doc}"
    );
    assert!(
        !final_doc.contains("Original placeholder body."),
        "the original body should have been replaced; doc:\n{final_doc}"
    );
    // Frontmatter is preserved through the inline rewrite.
    assert!(
        final_doc.contains("prompt: Generate the document body."),
        "original frontmatter should be preserved; doc:\n{final_doc}"
    );
}

#[cfg(unix)]
#[test]
fn inline_compose_no_overwrite_on_failure() {
    let workspace = tempdir().unwrap();
    let path_dir = workspace.path().join("bin");
    fs::create_dir_all(&path_dir).unwrap();
    seed_minimal_config(workspace.path());

    let md_file = workspace.path().join("test.md");
    let original = "---\nprompt: Generate content\n---\nOriginal body\n";
    fs::write(&md_file, original).unwrap();

    // Agent that exits with error and does not modify the file
    write_executable(
        &path_dir.join("codex"),
        r#"#!/bin/sh
exit 1
"#,
    );

    let _assert = assert_cmd::Command::cargo_bin("claudine").unwrap()
        .current_dir(workspace.path())
        .env("NO_COLOR", "1")
        .env("CLAUDINE_RENDEZVOUS_REPORT", "false")
        .env("HOME", workspace.path())
        .env("PATH", &path_dir)
        .args(["inline-compose", "--codex", md_file.to_str().unwrap()])
        .assert();

    // File should not have been modified
    let final_content = fs::read_to_string(&md_file).unwrap();
    assert_eq!(
        final_content, original,
        "file should not be modified on agent failure"
    );
}

/// `--quiet` and `--silent` have no effect on `inline-compose --dry-run`
/// output: the composed prompt still lands on stdout, the metadata on stderr,
/// and the source file is left byte-identical.
#[cfg(unix)]
#[test]
fn inline_compose_dry_run_quiet_and_silent_are_no_op() {
    for flag in ["--quiet", "--silent"] {
        let workspace = tempdir().unwrap();
        let path_dir = workspace.path().join("bin");
        fs::create_dir_all(&path_dir).unwrap();
        seed_minimal_config(workspace.path());

        let md_file = workspace.path().join("doc.md");
        let original = "---\nprompt: PROMPT_MARKER_QQQ\nagent: goose\n---\nOriginal body\n";
        fs::write(&md_file, original).unwrap();

        write_executable(&path_dir.join("goose"), "#!/bin/sh\nexit 0\n");

        let output = assert_cmd::Command::cargo_bin("claudine").unwrap()
            .env("NO_COLOR", "1")
            .env("HOME", workspace.path())
            .env("PATH", augmented_path(&path_dir))
            .args([
                "inline-compose",
                "--goose",
                "--dry-run",
                flag,
                md_file.to_str().unwrap(),
            ])
            .output()
            .unwrap();

        assert!(
            output.status.success(),
            "inline-compose dry-run {flag} should succeed; stderr was:\n{}",
            String::from_utf8_lossy(&output.stderr)
        );
        let stdout = strip_ansi(&String::from_utf8_lossy(&output.stdout));

        assert!(
            stdout.contains("PROMPT_MARKER_QQQ"),
            "{flag} must not suppress the composed prompt on stdout; stdout was:\n{stdout}"
        );
        assert_eq!(
            fs::read_to_string(&md_file).unwrap(),
            original,
            "{flag} inline-compose --dry-run must not mutate the source file"
        );
    }
}

/// Error surface (inline-compose): an unsatisfied `$schema` required property
/// under `--dry-run` renders to **stderr**, exits **non-zero**, leaves stdout
/// clean, and never mutates the source file.
#[cfg(unix)]
#[test]
fn inline_compose_dry_run_schema_error_to_stderr_with_clean_stdout() {
    let workspace = tempdir().unwrap();
    let path_dir = workspace.path().join("bin");
    fs::create_dir_all(&path_dir).unwrap();
    seed_minimal_config(workspace.path());

    let md_file = workspace.path().join("doc.md");
    let original =
        "---\n$schema:\n  topic: 'string(required)'\nprompt: Plan {{topic}}\nagent: goose\n---\nOriginal body\n";
    fs::write(&md_file, original).unwrap();

    write_executable(&path_dir.join("goose"), "#!/bin/sh\nexit 0\n");

    let output = assert_cmd::Command::cargo_bin("claudine").unwrap()
        .env("NO_COLOR", "1")
        .env("HOME", workspace.path())
        .env("PATH", augmented_path(&path_dir))
        .args([
            "inline-compose",
            "--goose",
            "--dry-run",
            md_file.to_str().unwrap(),
        ])
        .output()
        .unwrap();

    assert!(
        !output.status.success(),
        "schema-error dry-run must exit non-zero"
    );
    assert!(
        output.stdout.is_empty(),
        "stdout must stay clean on error; stdout was:\n{}",
        String::from_utf8_lossy(&output.stdout)
    );
    let stderr = strip_ansi(&String::from_utf8_lossy(&output.stderr));
    assert!(
        stderr.to_lowercase().contains("missing properties") && stderr.contains("topic"),
        "missing-properties error naming `topic` must appear on stderr; stderr was:\n{stderr}"
    );
    assert_eq!(
        fs::read_to_string(&md_file).unwrap(),
        original,
        "inline-compose --dry-run must not mutate the source file on error"
    );
}

// ---------------------------------------------------------------------------
// OpenCode model resolution (Phase 7 integration tests)
// ---------------------------------------------------------------------------

#[cfg(unix)]
mod opencode_model_integration {
    use super::*;

    #[test]
    fn no_model_provided_renders_blockquote_without_text_above() {
        let workspace = tempdir().unwrap();
        let path_dir = workspace.path().join("bin");
        fs::create_dir_all(&path_dir).unwrap();

        write_executable(
            &path_dir.join("opencode"),
            r#"#!/bin/sh
exit 0
"#,
        );

        let assert = assert_cmd::Command::cargo_bin("claudine").unwrap()
            .env("NO_COLOR", "1")
            .env("HOME", workspace.path())
            .env("PATH", &path_dir)
            .args(["opencode", "summarize"])
            .assert()
            .code(1);

        let stderr = String::from_utf8_lossy(&assert.get_output().stderr).to_string();
        let plain = strip_ansi(&stderr);

        assert!(plain.contains("No model specified!"));
        assert!(plain.contains("OPENCODE_MODEL"));
        assert!(plain.contains("--model"));
        assert!(plain.contains("opencode models"));

        let block_quote_start = plain.find("┃").unwrap();
        let before_block = &plain[..block_quote_start];
        let trimmed_before = before_block.trim();
        assert!(
            trimmed_before.is_empty(),
            "no text should appear above the BlockQuote; got: '{trimmed_before}'"
        );
    }

    #[test]
    fn cli_model_proceeds_past_resolver() {
        let workspace = tempdir().unwrap();
        let path_dir = workspace.path().join("bin");
        fs::create_dir_all(&path_dir).unwrap();
        let args_path = workspace.path().join("args.txt");

        write_executable(
            &path_dir.join("opencode"),
            r#"#!/bin/sh
printf '%s\n' "$@" > "$CLAUDINE_ARGS_FILE"
exit 0
"#,
        );

        assert_cmd::Command::cargo_bin("claudine").unwrap()
            .env("NO_COLOR", "1")
            .env("PATH", &path_dir)
            .env("CLAUDINE_ARGS_FILE", &args_path)
            .args(["opencode", "--model", "test-model", "summarize"])
            .assert()
            .success();

        let args = fs::read_to_string(&args_path).unwrap();
        assert!(args.lines().any(|line| line == "test-model"));
    }

    #[test]
    fn invalid_model_error_shows_suggestions() {
        let workspace = tempdir().unwrap();
        let path_dir = workspace.path().join("bin");
        fs::create_dir_all(&path_dir).unwrap();

        write_executable(
            &path_dir.join("opencode"),
            r#"#!/bin/sh
printf 'Error: ProviderModelNotFoundError: model bad-model not found\nsuggestions: ["provider/a", "provider/b"]\n' >&2
exit 1
"#,
        );

        let assert = assert_cmd::Command::cargo_bin("claudine").unwrap()
            .env("NO_COLOR", "1")
            .env("HOME", workspace.path())
            .env("PATH", &path_dir)
            .env("OPENCODE_MODEL", "bad-model")
            .args(["opencode", "summarize"])
            .assert()
            .code(1);

        let stderr = String::from_utf8_lossy(&assert.get_output().stderr).to_string();
        let plain = strip_ansi(&stderr);

        assert!(
            plain.contains("Invalid model specified")
                || plain.contains("ProviderModelNotFoundError"),
            "expected model-not-found error in stderr; got:\n{plain}"
        );
        assert!(
            plain.contains("provider/a"),
            "expected suggestion 'provider/a' in stderr; got:\n{plain}"
        );
        assert!(
            plain.contains("provider/b"),
            "expected suggestion 'provider/b' in stderr; got:\n{plain}"
        );
    }

    #[test]
    fn config_file_model_resolves_successfully() {
        let workspace = tempdir().unwrap();
        let path_dir = workspace.path().join("bin");
        fs::create_dir_all(&path_dir).unwrap();
        let args_path = workspace.path().join("args.txt");
        let env_path = workspace.path().join("env.txt");

        let config_dir = workspace.path().join(".config/opencode");
        fs::create_dir_all(&config_dir).unwrap();
        fs::write(
            config_dir.join("config.json"),
            r#"{"model":"config-default-model"}"#,
        )
        .unwrap();

        write_executable(
            &path_dir.join("opencode"),
            r#"#!/bin/sh
printf '%s\n' "$@" > "$CLAUDINE_ARGS_FILE"
printf 'MODEL=%s\n' "$MODEL" > "$CLAUDINE_ENV_FILE"
exit 0
"#,
        );

        assert_cmd::Command::cargo_bin("claudine").unwrap()
            .env("NO_COLOR", "1")
            .env("HOME", workspace.path())
            .env("PATH", &path_dir)
            .env("CLAUDINE_ARGS_FILE", &args_path)
            .env("CLAUDINE_ENV_FILE", &env_path)
            .args(["opencode", "summarize"])
            .assert()
            .success();

        let env_lines = fs::read_to_string(&env_path).unwrap();
        assert!(env_lines.contains("MODEL=config-default-model"));

        let args = fs::read_to_string(&args_path).unwrap();
        assert!(
            !args.lines().any(|line| line == "--model"),
            "ConfigDefault should NOT push --model to child args"
        );
    }
}
