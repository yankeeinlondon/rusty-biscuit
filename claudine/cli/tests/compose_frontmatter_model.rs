#![cfg(unix)]

//! L1: a document's frontmatter `model:` reaches the provider launch even when
//! Claudine's compiled model catalog does not know the id
//! (`fixes/2026-09-08-frontmatter-matters`).
//!
//! `minimax/MiniMax-M3` is the regression input: an OpenCode provider the user
//! configures in `opencode.jsonc`, which can never be in the compiled
//! `opencode/*` baseline. `opencode/gpt-5.2-codex` is the control, an id the
//! baseline carries.

use std::fs;
use std::path::PathBuf;

mod common;
use common::{CliProcessFixture, strip_ansi, write, write_executable};

const UNRECOGNIZED_MODEL: &str = "minimax/MiniMax-M3";
const BASELINE_MODEL: &str = "opencode/gpt-5.2-codex";
const CATALOG_NOTICE: &str = "[model] frontmatter model";

/// A fake `opencode` that appends every launch's argv and `MODEL` to
/// `CLAUDINE_LAUNCH_FILE`, touches `<file>.models` when asked for its model
/// listing, and emits a minimal structured stream so the run completes.
fn stage_recording_opencode(fixture: &CliProcessFixture) -> PathBuf {
    write_executable(
        &fixture.bin_dir().join("opencode"),
        r##"#!/bin/sh
if [ "$1" = "models" ]; then
  : > "$CLAUDINE_LAUNCH_FILE.models"
  printf '%s\n' '[]'
  exit 0
fi
{
  printf -- '--- invocation ---\n'
  printf 'ARG=%s\n' "$@"
  printf 'MODEL=%s\n' "$MODEL"
} >> "$CLAUDINE_LAUNCH_FILE"
printf '%s\n' '{"type":"init","session_id":"conv","model":"stub"}'
printf '%s\n' '{"type":"step_start","sessionID":"conv"}'
printf '%s\n' '{"type":"text","text":"ok"}'
printf '%s\n' '{"type":"finish","sessionID":"conv"}'
exit 0
"##,
    );
    fixture.cwd().join("opencode-launch.txt")
}

fn write_prompt(fixture: &CliProcessFixture, model: &str) -> PathBuf {
    let path = fixture.cwd().join("prompt.md");
    write(
        &path,
        &format!("---\nagent: opencode\nmodel: {model}\n---\n\nSay hi.\n"),
    );
    path
}

fn run(fixture: &CliProcessFixture, launch_file: &PathBuf, args: &[&str]) -> (bool, String) {
    let assert = fixture
        .command()
        .env("CLAUDINE_LAUNCH_FILE", launch_file)
        .env_remove("MODEL")
        .env_remove("OPENCODE_MODEL")
        .args(args)
        .assert();
    let output = assert.get_output();
    let stderr = strip_ansi(&String::from_utf8_lossy(&output.stderr));
    (output.status.success(), stderr)
}

fn launched_with(launch_file: &PathBuf, model: &str) -> bool {
    let log = fs::read_to_string(launch_file).unwrap_or_default();
    log.contains(&format!("ARG=--model\nARG={model}\n")) && log.contains(&format!("MODEL={model}\n"))
}

#[test]
fn frontmatter_model_outside_the_catalog_reaches_argv_and_env_with_a_warning() {
    let fixture = CliProcessFixture::named("compose-fm-model-forwarded");
    let launch_file = stage_recording_opencode(&fixture);
    let prompt = write_prompt(&fixture, UNRECOGNIZED_MODEL);

    let (ok, stderr) = run(&fixture, &launch_file, &["compose", prompt.to_str().unwrap()]);

    assert!(ok, "compose should succeed; stderr:\n{stderr}");
    assert!(
        launched_with(&launch_file, UNRECOGNIZED_MODEL),
        "child must receive the frontmatter model on argv and in MODEL; launch log:\n{}",
        fs::read_to_string(&launch_file).unwrap_or_default()
    );
    let notices = stderr.matches(CATALOG_NOTICE).count();
    assert_eq!(
        notices, 1,
        "exactly one catalog notice expected; stderr:\n{stderr}"
    );
    assert!(
        stderr.contains(&format!("'{UNRECOGNIZED_MODEL}'")) && stderr.contains("OpenCode"),
        "the notice must name the model and the provider; stderr:\n{stderr}"
    );
}

#[test]
fn frontmatter_model_in_the_catalog_launches_without_a_notice() {
    let fixture = CliProcessFixture::named("compose-fm-model-baseline");
    let launch_file = stage_recording_opencode(&fixture);
    let prompt = write_prompt(&fixture, BASELINE_MODEL);

    let (ok, stderr) = run(&fixture, &launch_file, &["compose", prompt.to_str().unwrap()]);

    assert!(ok, "compose should succeed; stderr:\n{stderr}");
    assert!(launched_with(&launch_file, BASELINE_MODEL));
    assert!(
        !stderr.contains(CATALOG_NOTICE),
        "a baseline model must not warn; stderr:\n{stderr}"
    );
}

#[test]
fn silent_suppresses_the_catalog_notice_but_not_the_model() {
    let fixture = CliProcessFixture::named("compose-fm-model-silent");
    let launch_file = stage_recording_opencode(&fixture);
    let prompt = write_prompt(&fixture, UNRECOGNIZED_MODEL);

    let (ok, stderr) = run(
        &fixture,
        &launch_file,
        &["compose", "--silent", prompt.to_str().unwrap()],
    );

    assert!(ok, "compose should succeed; stderr:\n{stderr}");
    assert!(launched_with(&launch_file, UNRECOGNIZED_MODEL));
    assert!(!stderr.contains(CATALOG_NOTICE), "stderr:\n{stderr}");
}

#[test]
fn dry_run_reports_the_launch_model_and_the_notice_without_a_refresh() {
    let fixture = CliProcessFixture::named("compose-fm-model-dry-run");
    let launch_file = stage_recording_opencode(&fixture);
    let prompt = write_prompt(&fixture, UNRECOGNIZED_MODEL);

    let (ok, stderr) = run(
        &fixture,
        &launch_file,
        &["compose", "--dry-run", prompt.to_str().unwrap()],
    );

    assert!(ok, "dry-run should succeed; stderr:\n{stderr}");
    let model_row = stderr
        .lines()
        .find(|line| line.contains("Model"))
        .unwrap_or_default();
    assert!(
        model_row.contains(UNRECOGNIZED_MODEL),
        "the Model row must show the launch model; stderr:\n{stderr}"
    );
    assert!(
        stderr.contains(CATALOG_NOTICE),
        "dry-run must print the same notice as the live run; stderr:\n{stderr}"
    );
    assert!(!launch_file.exists(), "dry-run must not launch the provider");
    assert!(
        !launch_file.with_extension("txt.models").exists(),
        "dry-run must not refresh the provider's model listing"
    );
}

#[test]
fn sequence_launches_every_step_with_the_document_model() {
    let fixture = CliProcessFixture::named("sequence-fm-model");
    let launch_file = stage_recording_opencode(&fixture);
    let sequence = fixture.cwd().join("seq.md");
    write(
        &sequence,
        &format!(
            "---\nagent: opencode\nmodel: {UNRECOGNIZED_MODEL}\nsequence:\n  - alpha\n  - beta\n---\n\nStep {{{{state}}}}\n"
        ),
    );

    let (ok, stderr) = run(&fixture, &launch_file, &["sequence", sequence.to_str().unwrap()]);

    assert!(ok, "sequence should succeed; stderr:\n{stderr}");
    let log = fs::read_to_string(&launch_file).unwrap_or_default();
    assert_eq!(
        log.matches("--- invocation ---").count(),
        2,
        "two steps, two launches; log:\n{log}"
    );
    assert_eq!(
        log.matches(&format!("ARG=--model\nARG={UNRECOGNIZED_MODEL}\n")).count(),
        2,
        "every step must carry the document model; log:\n{log}"
    );
    assert_eq!(
        stderr.matches(CATALOG_NOTICE).count(),
        1,
        "one notice per provider, not per step; stderr:\n{stderr}"
    );
}
