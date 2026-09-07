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
    common::InlineAgentStub::new(&md_file)
        .prelude("printf '%s\\n' \"$@\" > \"$CLAUDINE_CAPTURED_ARGS\"\n")
        .body("updated body content\n")
        .install(&path_dir, "goose");

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

/// AC7: an agent that returns a summary without editing the file fails, and the
/// document is byte-identical to the pre-run snapshot.
#[cfg(unix)]
#[test]
fn inline_compose_rejects_a_document_the_agent_never_updated() {
    let workspace = tempdir().unwrap();
    let path_dir = workspace.path().join("bin");
    fs::create_dir_all(&path_dir).unwrap();
    seed_minimal_config(workspace.path());

    let md_file = workspace.path().join("test.md");
    let original = "---\nprompt: Generate content\n---\nOriginal body\n";
    fs::write(&md_file, original).unwrap();

    // An agent that talks about the work without doing it.
    write_executable(
        &path_dir.join("codex"),
        "#!/bin/sh\nprintf 'I reviewed the document and it looks fine.\\n'\nexit 0\n",
    );

    let assert = assert_cmd::Command::cargo_bin("claudine").unwrap()
        .current_dir(workspace.path())
        .env("NO_COLOR", "1")
        .env("CLAUDINE_RENDEZVOUS_REPORT", "false")
        .env("HOME", workspace.path())
        .env("PATH", &path_dir)
        .args(["inline-compose", "--codex", md_file.to_str().unwrap()])
        .assert()
        .failure();

    let plain = strip_ansi(&String::from_utf8_lossy(&assert.get_output().stderr));
    assert!(
        plain.contains("did not update"),
        "should report the untouched document; stderr was: {plain}"
    );
    assert_eq!(
        fs::read_to_string(&md_file).unwrap(),
        original,
        "a refused candidate must leave the document byte-identical"
    );
}

/// AC6: the agent's authored frontmatter survives, the three caller-owned
/// properties are restored with one warning each, and the stamp is fresh.
#[cfg(unix)]
#[test]
fn inline_compose_preserves_frontmatter_and_restores_owned_properties() {
    let workspace = tempdir().unwrap();
    let path_dir = workspace.path().join("bin");
    fs::create_dir_all(&path_dir).unwrap();
    seed_minimal_config(workspace.path());

    let md_file = workspace.path().join("test.md");
    let original =
        "---\nprompt: Generate content\nowner: Human\nlast_updated: '2026-01-01'\n---\nOriginal body\n";
    fs::write(&md_file, original).unwrap();

    // A disobedient agent: it rewrites `prompt` and `last_updated` (both owned)
    // alongside the property it was legitimately asked to set.
    write_executable(
        &path_dir.join("goose"),
        &format!(
            "#!/bin/sh\nprintf '%s' {document} > {target}\nprintf 'Rewrote the body.\\n'\nexit 0\n",
            document = common::sh_quote(concat!(
                "---\n",
                "prompt: CHANGED\n",
                "owner: Human\n",
                "researched_by: goose\n",
                "last_updated: '2099-01-01'\n",
                "---\n",
                "New body from agent\n",
            )),
            target = common::sh_quote(&md_file.display().to_string()),
        ),
    );

    let assert = assert_cmd::Command::cargo_bin("claudine").unwrap()
        .current_dir(workspace.path())
        .env("NO_COLOR", "1")
        .env("CLAUDINE_RENDEZVOUS_REPORT", "false")
        .env("HOME", workspace.path())
        .env("PATH", &path_dir)
        .args(["inline-compose", "--goose", md_file.to_str().unwrap()])
        .assert()
        .success();

    let final_content = fs::read_to_string(&md_file).unwrap();
    assert!(
        final_content.contains("prompt: Generate content"),
        "the authored `prompt` must be restored; file: {final_content}"
    );
    assert!(
        !final_content.contains("CHANGED"),
        "the agent's `prompt` must not survive; file: {final_content}"
    );
    assert!(
        final_content.contains("researched_by: goose"),
        "the agent's own frontmatter must survive; file: {final_content}"
    );

    let today = Local::now().format("%Y-%m-%d").to_string();
    assert!(
        final_content.contains(&format!("last_updated: '{today}'")),
        "last_updated should be today; file: {final_content}"
    );
    assert!(
        final_content.contains("New body from agent"),
        "body should be from agent; file: {final_content}"
    );

    let plain = strip_ansi(&String::from_utf8_lossy(&assert.get_output().stderr));
    let normalized = plain.split_whitespace().collect::<Vec<_>>().join(" ");
    for property in ["prompt", "last_updated"] {
        assert!(
            normalized.contains(&format!(
                "The agent changed the caller-owned property \"{property}\" — restored the authored value"
            )),
            "expected one restore warning for `{property}`; stderr was:\n{plain}"
        );
    }
}

/// AC6 / AC9b: the agent authors frontmatter and body directly into the file
/// across runs. The authored `prompt` bytes never move, the stamp stays
/// coherent with what was written, and a second run refreshes rather than
/// duplicating the agent's own properties.
#[cfg(unix)]
#[test]
fn inline_compose_keeps_agent_written_frontmatter_across_runs() {
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
            "last_updated: '2026-01-01'\n",
            "---\n",
            "Original inventory.\n",
        ),
        prompt_bytes
    );
    fs::write(&md_file, &original).unwrap();

    let run_count = workspace.path().join("run-count");
    let captured_args = workspace.path().join("captured-args");
    // A file-aware agent: it rewrites the whole document, keeping the authored
    // `prompt` node verbatim and setting the two properties it was asked for.
    let agent_document = |version: &str| {
        format!(
            concat!(
                "---\n",
                "{}",
                "access_points:\n",
                "  - Office-{}\n",
                "  - Studio-{}\n",
                "generated_by: obedient-stub-{}\n",
                "last_updated: '2026-01-01'\n",
                "---\n",
                "{} access-point inventory.\n",
            ),
            prompt_bytes,
            version,
            version,
            version,
            if version == "v1" { "Initial" } else { "Refreshed" }
        )
    };
    write_executable(
        &path_dir.join("goose"),
        &format!(
            "#!/bin/sh\n\
             printf '%s\\n' \"$@\" > \"$CLAUDINE_CAPTURED_ARGS\"\n\
             if [ -f \"$CLAUDINE_RUN_COUNT\" ]; then\n\
             printf '%s' {v2} > {document}\n\
             else\n\
             : > \"$CLAUDINE_RUN_COUNT\"\n\
             printf '%s' {v1} > {document}\n\
             fi\n\
             printf 'Updated the inventory.\\n'\n\
             exit 0\n",
            document = common::sh_quote(&md_file.display().to_string()),
            v1 = common::sh_quote(&agent_document("v1")),
            v2 = common::sh_quote(&agent_document("v2")),
        ),
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

    run();
    let first_content = fs::read_to_string(&md_file).unwrap();
    assert!(first_content.contains(prompt_bytes), "{first_content}");
    assert!(first_content.contains("Office-v1"));
    assert!(first_content.contains("generated_by: obedient-stub-v1"));

    let delivered_prompt = fs::read_to_string(&captured_args).unwrap();
    assert!(delivered_prompt.contains("Never modify the `prompt`, `hash`, or `last_updated`"));
    assert!(!delivered_prompt.contains("If the prompt asks you to add or update frontmatter properties"));

    run();
    let final_content = fs::read_to_string(&md_file).unwrap();
    assert!(final_content.contains(prompt_bytes), "{final_content}");
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

/// The 2026-09-01 drift semantics invert: on-disk frontmatter the agent wrote
/// is the deliverable, not drift to restore. A structurally broken document is
/// still refused, and refusal never stamps.
#[cfg(unix)]
#[test]
fn inline_compose_keeps_agent_frontmatter_and_refuses_a_malformed_document() {
    for (agent_document, expectation) in [
        (
            "---\nprompt: Generate content\nadded: value\n---\nReplacement body\n",
            Ok("added: value"),
        ),
        (
            "---\nprompt: [\n---\nReplacement body\n",
            Err("could not reconcile the inline document"),
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
printf '%s' "$CLAUDINE_AGENT_DOCUMENT" > "$CLAUDINE_AGENT_TARGET"
printf 'Wrote the document.\n'
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
            .env("CLAUDINE_AGENT_DOCUMENT", agent_document)
            .env("CLAUDINE_AGENT_TARGET", &md_file)
            .args(["inline-compose", "--goose", md_file.to_str().unwrap()])
            .assert();

        let stderr = strip_ansi(&String::from_utf8_lossy(&assert.get_output().stderr));
        let written = fs::read_to_string(&md_file).unwrap();
        match expectation {
            Ok(kept) => {
                assert!(
                    written.contains(kept),
                    "the agent's frontmatter must be kept, not restored; file:\n{written}"
                );
                assert!(written.contains("hash:"), "an accepted run stamps; file:\n{written}");
            }
            Err(message) => {
                assert!(
                    stderr.contains(message),
                    "expected {message:?}; stderr was:\n{stderr}"
                );
                assert!(
                    !written.contains("hash:"),
                    "a refused run must not stamp; file:\n{written}"
                );
                // AC17: a closure parse failure rolls the guard's baseline back
                // atomically, so the malformed text the agent left is gone.
                assert_eq!(
                    written, "---\nprompt: Generate content\n---\nOriginal body\n",
                    "a parse failure must restore the captured baseline"
                );
            }
        }
        assert!(
            !stderr.contains("changed on disk during the run"),
            "the retired drift wording must be gone; stderr was:\n{stderr}"
        );
    }
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

/// AC5: the agent writes the file and its final response is the run summary.
/// Neither the interstitial narration nor the summary itself reaches the body.
#[cfg(unix)]
#[test]
#[serial_test::serial]
fn inline_compose_writes_the_agents_file_and_reports_only_the_final_summary() {
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

    // Claude stream-json stub: narrate, call a tool, narrate again, write the
    // document, then emit the FINAL summary. Each narration block is a separate
    // text-only assistant message, so without the accumulator reset all of them
    // would be reported as the summary.
    write_executable(
        &path_dir.join("claude"),
        &format!(
            r##"#!/bin/sh
printf '%s\n' '{{"type":"system","subtype":"init","session_id":"claude-final","model":"claude-sonnet-4"}}'
printf '%s\n' '{{"type":"assistant","message":{{"content":[{{"type":"text","text":"Let me read the research documents first."}}]}}}}'
printf '%s\n' '{{"type":"tool_use","name":"read_file","input":{{"path":"research.md"}}}}'
printf '%s\n' '{{"type":"assistant","message":{{"content":[{{"type":"text","text":"Now let me write the document."}}]}}}}'
printf '%s' {agent_document} > '{document}'
printf '%s\n' '{{"type":"tool_use","name":"write_file","input":{{"path":"doc.md"}}}}'
printf '%s\n' '{{"type":"assistant","message":{{"content":[{{"type":"text","text":"I researched the handsets and wrote the document."}}]}}}}'
printf '%s\n' '{{"type":"result","subtype":"success","stop_reason":"end_turn","num_turns":3,"duration_ms":100,"usage":{{"input_tokens":3,"output_tokens":60}}}}'
"##,
            document = md_file.display(),
            agent_document = common::sh_quote(concat!(
                "---\n",
                "prompt: Generate the document body.\n",
                "---\n",
                "# Final Document\n",
                "\n",
                "This is the only content that belongs in the body.\n",
            )),
        ),
    );

    let assert = assert_cmd::Command::cargo_bin("claudine").unwrap()
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
        "the agent's file is the deliverable; doc:\n{final_doc}"
    );
    for narration in [
        "Let me read the research documents first.",
        "Now let me write the document.",
        "I researched the handsets and wrote the document.",
        "Original placeholder body.",
    ] {
        assert!(
            !final_doc.contains(narration),
            "{narration:?} must not reach the body; doc:\n{final_doc}"
        );
    }
    assert!(
        final_doc.contains("prompt: Generate the document body."),
        "original frontmatter should be preserved; doc:\n{final_doc}"
    );

    // Narration is never reported as the run's outcome. Publishing the summary
    // itself into run output is the lifecycle-routing step (plan Phase 6).
    let stderr = strip_ansi(&String::from_utf8_lossy(&assert.get_output().stderr));
    let stdout = strip_ansi(&String::from_utf8_lossy(&assert.get_output().stdout));
    assert!(
        stdout.contains("I researched the handsets and wrote the document."),
        "the final response must be published as the run summary; stdout was:\n{stdout}"
    );
    assert!(
        !stderr.contains("Let me read the research documents first."),
        "narration must not be reported as the summary; stderr was:\n{stderr}"
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
    // `eager`: an inline run tolerates a required-but-not-eager gap at launch
    // (the agent supplies it), so only an eager property is a launch error.
    let original =
        "---\n$schema:\n  topic: 'string(required;eager)'\nprompt: Plan {{topic}}\nagent: goose\n---\nOriginal body\n";
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

// -- file-aware inline launch (2026-09-05 inline flow, Phase 4) -----------

/// A Goose stub that records its argv and environment, edits the active
/// document the way a file-aware agent does, and returns a summary.
#[cfg(unix)]
/// A Goose stub that records its launch and writes the document.
///
/// `frontmatter_additions` are the properties this fixture's agent is expected
/// to supply — the completion verdict enforces the document's `$schema` once
/// the run finishes, so a launch-surface fixture has to leave a *satisfied*
/// document behind or it would be asserting on a failed run.
fn write_recording_goose(
    path_dir: &std::path::Path,
    document: &std::path::Path,
    frontmatter_additions: &str,
) {
    common::InlineAgentStub::new(document)
        .prelude(RECORDING_PRELUDE)
        .frontmatter_additions(frontmatter_additions)
        .body("updated body content\n")
        .summary("Recorded the launch and updated the document.")
        .install(path_dir, "goose");
}

#[cfg(unix)]
const RECORDING_PRELUDE: &str = "printf '%s\\n' \"$@\" > \"$CLAUDINE_CAPTURED_ARGS\"\n\
     printf 'GOOSE_MODE=%s\\n' \"${GOOSE_MODE-unset}\" > \"$CLAUDINE_CAPTURED_ENV\"\n";

#[cfg(unix)]
fn inline_compose_cmd(workspace: &std::path::Path, path_dir: &std::path::Path) -> assert_cmd::Command {
    let mut cmd = assert_cmd::Command::cargo_bin("claudine").unwrap();
    cmd.current_dir(workspace)
        .env("NO_COLOR", "1")
        .env("CLAUDINE_RENDEZVOUS_REPORT", "false")
        .env("HOME", workspace)
        .env("PATH", path_dir)
        .env("CLAUDINE_CAPTURED_ARGS", workspace.join("captured_args.txt"))
        .env("CLAUDINE_CAPTURED_ENV", workspace.join("captured_env.txt"));
    cmd
}

/// AC5 / AC19: the delivered prompt names the document's exact native
/// absolute path (spaces intact, nothing escaped) and carries the file-aware
/// guardrails, and Goose is launched in its writable posture.
#[cfg(unix)]
#[test]
fn inline_compose_delivers_the_native_document_path_and_file_aware_guardrails() {
    let workspace = tempdir().unwrap();
    let path_dir = workspace.path().join("bin");
    fs::create_dir_all(&path_dir).unwrap();
    seed_minimal_config(workspace.path());
    let docs = workspace.path().join("my docs");
    fs::create_dir_all(&docs).unwrap();
    let md_file = docs.join("voip notes.md");
    write_recording_goose(
        &path_dir,
        &md_file,
        "researched_by: goose\nproducts:\n  handset: Yealink\n",
    );
    fs::write(
        &md_file,
        concat!(
            "---\n",
            "$schema:\n",
            "  prompt: 'string(required;eager)'\n",
            "  last_updated: 'string(required)'\n",
            "  researched_by: 'string(required)'\n",
            "  products: 'object(required)'\n",
            "prompt: Research VoIP handsets\n",
            "agent: goose\n",
            "---\n",
            "original body\n",
        ),
    )
    .unwrap();

    inline_compose_cmd(workspace.path(), &path_dir)
        .args(["inline-compose", md_file.to_str().unwrap()])
        .assert()
        .success();

    let argv = fs::read_to_string(workspace.path().join("captured_args.txt")).unwrap();
    // The host may spell the temp dir with or without its `/private` symlink
    // prefix; the document identity Claudine resolved is one of the two.
    let spellings = [
        md_file.clone(),
        md_file.canonicalize().unwrap_or_else(|_| md_file.clone()),
    ];
    let expected_span = spellings
        .iter()
        .map(|path| format!("`{}`", path.display()))
        .find(|span| argv.contains(&format!("**Document:** {span}")))
        .unwrap_or_else(|| panic!("the prompt header must name the native absolute path; argv was: {argv}"));
    assert!(
        argv.contains(&format!("The document you are updating is {expected_span}.")),
        "guardrails must be bound to the same path; argv was: {argv}"
    );
    assert!(argv.contains("Never modify the `prompt`, `hash`, or `last_updated`"), "{argv}");
    assert!(argv.contains("summary of what you did"), "{argv}");
    assert!(argv.contains("| `products` | `object(required)` | absent | required |"), "{argv}");
    assert!(argv.contains("| `prompt` | `string(required;eager)` | present | required |"), "{argv}");
    assert!(!argv.contains("Return the replacement Markdown body"), "{argv}");
    assert!(!argv.contains("\\\\"), "backslashes must never be doubled: {argv}");

    let env = fs::read_to_string(workspace.path().join("captured_env.txt")).unwrap();
    assert_eq!(env.trim(), "GOOSE_MODE=auto", "Goose's minimum writable posture");
}

/// AC3 / AC13: an eager `prompt` supplied by the caller satisfies the launch
/// gate, becomes the delivered prompt, and is never written to the file.
#[cfg(unix)]
#[test]
fn inline_compose_uses_a_caller_supplied_prompt_without_persisting_it() {
    let workspace = tempdir().unwrap();
    let path_dir = workspace.path().join("bin");
    fs::create_dir_all(&path_dir).unwrap();
    seed_minimal_config(workspace.path());

    let md_file = workspace.path().join("doc.md");
    write_recording_goose(&path_dir, &md_file, "products:\n  handset: Yealink\n");
    let authored = concat!(
        "---\n",
        "$schema:\n",
        "  prompt: 'string(required;eager)'\n",
        "  products: 'object(required)'\n",
        "agent: goose\n",
        "---\n",
        "original body\n",
    );
    fs::write(&md_file, authored).unwrap();

    inline_compose_cmd(workspace.path(), &path_dir)
        .args([
            "inline-compose",
            md_file.to_str().unwrap(),
            "prompt=Transient research request",
        ])
        .assert()
        .success();

    let argv = fs::read_to_string(workspace.path().join("captured_args.txt")).unwrap();
    assert!(argv.contains("Transient research request"), "{argv}");
    let after = fs::read_to_string(&md_file).unwrap();
    assert!(!after.contains("Transient research request"), "transient prompt persisted: {after}");
    assert!(!after.contains("\nprompt:"), "no prompt property may be authored into the file: {after}");
}

/// AC3: with `prompt` absent and no caller value, a non-TTY run fails before
/// launch naming `prompt` — through the schema, so it is collectable on a TTY.
#[cfg(unix)]
#[test]
fn inline_compose_without_an_eager_prompt_fails_before_launch_naming_it() {
    let workspace = tempdir().unwrap();
    let path_dir = workspace.path().join("bin");
    fs::create_dir_all(&path_dir).unwrap();
    seed_minimal_config(workspace.path());

    let md_file = workspace.path().join("doc.md");
    write_recording_goose(&path_dir, &md_file, "");
    fs::write(
        &md_file,
        "---\n$schema:\n  prompt: 'string(required;eager)'\n  products: 'object(required)'\nagent: goose\n---\nbody\n",
    )
    .unwrap();

    let assert = inline_compose_cmd(workspace.path(), &path_dir)
        .args(["inline-compose", md_file.to_str().unwrap()])
        .assert()
        .failure();
    let stderr = strip_ansi(&String::from_utf8_lossy(&assert.get_output().stderr));
    assert!(stderr.contains("prompt"), "must name the missing eager property: {stderr}");
    assert!(
        !stderr.contains("products"),
        "a required-but-not-eager property is never a launch gap for inline-compose: {stderr}"
    );
    assert!(
        !workspace.path().join("captured_args.txt").exists(),
        "the provider must not launch"
    );
}

/// AC19: an explicit approval deny refuses before any provider spawns and is
/// never widened to bypass.
#[cfg(unix)]
#[test]
fn inline_compose_refuses_an_explicit_write_deny_before_spawn() {
    let workspace = tempdir().unwrap();
    let path_dir = workspace.path().join("bin");
    fs::create_dir_all(&path_dir).unwrap();
    seed_minimal_config(workspace.path());

    let md_file = workspace.path().join("doc.md");
    write_recording_goose(&path_dir, &md_file, "");
    fs::write(&md_file, "---\nprompt: Update me\nagent: goose\n---\nbody\n").unwrap();

    let assert = inline_compose_cmd(workspace.path(), &path_dir)
        .env("GOOSE_MODE", "chat")
        .args(["inline-compose", md_file.to_str().unwrap()])
        .assert()
        .failure();
    let stderr = strip_ansi(&String::from_utf8_lossy(&assert.get_output().stderr));
    assert!(stderr.contains("GOOSE_MODE=chat"), "must name the denial: {stderr}");
    assert!(stderr.contains("explicitly denies file edits"), "{stderr}");
    assert!(
        !workspace.path().join("captured_args.txt").exists(),
        "the provider must not launch under a denied posture"
    );
    assert_eq!(fs::read_to_string(&md_file).unwrap(), "---\nprompt: Update me\nagent: goose\n---\nbody\n");
}
