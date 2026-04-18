//! End-to-end integration tests for the pre-clap argv normalization layer.
//!
//! These tests drive the compiled `claudine` binary so we exercise the
//! actual path from `std::env::args_os()` through `argv::normalize` into
//! clap. Unit coverage for the rewrite rules lives alongside the
//! implementation in `claudine/cli/src/argv.rs`; this suite proves that
//! the normalization shows up at the binary entrypoint and does not
//! disturb unaffected commands.
//!
//! Feature: `2026-04-17-cli-pre-processing`.

use assert_cmd::cargo::cargo_bin_cmd;
use std::fs;
use tempfile::tempdir;

mod common;
use common::strip_ansi;

/// Minimal frontmatter-only markdown fixture that composition can resolve.
const FIXTURE_MD: &str = "---\ntitle: argv normalization fixture\n---\n\nHello.\n";

fn write_fixture(dir: &std::path::Path, name: &str) -> std::path::PathBuf {
    let path = dir.join(name);
    fs::write(&path, FIXTURE_MD).unwrap();
    path
}

// ──────────────────────────────────────────────────────────────────────
// Headline integration cases (spec: "Testing → Integration tests")
// ──────────────────────────────────────────────────────────────────────

/// Rule 3 insertion: `compose <file> --<provider-bool> key=val --help`.
///
/// Before this feature, clap emitted the misleading
/// `tip: to pass '--help' as a value, use '-- --help'` diagnostic because
/// the greedy positional collector swallowed `--help`. After normalization,
/// Rule 1 canonicalises `--gemini` to `--provider gemini` and Rule 3
/// inserts a `--` before `name=Ken`, so clap never hits the misleading
/// "unexpected argument" arm. The specific exit behavior of compose is
/// out of scope; the assertion we care about is that the pre-existing
/// clap tip is gone from stderr.
#[test]
fn headline_compose_with_interleaved_flag_no_longer_trips_clap_help_tip() {
    let workspace = tempdir().unwrap();
    let fixture = write_fixture(workspace.path(), "greet.md");

    let assert = cargo_bin_cmd!("claudine")
        .env("NO_COLOR", "1")
        .args([
            "compose",
            fixture.to_str().unwrap(),
            "--gemini",
            "name=Ken",
            "--help",
        ])
        .assert();

    let output = assert.get_output().clone();
    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);
    let combined = format!("{stderr}{stdout}");
    let plain = strip_ansi(&combined);

    assert!(
        !plain.contains("tip: to pass '--help' as a value"),
        "normalizer must suppress the misleading clap help tip; got: {plain}"
    );
    assert!(
        !plain.contains("unexpected argument '--help'"),
        "normalizer must keep `--help` from tripping clap's greedy \
         positional collection; got: {plain}"
    );
}

/// Rule 2 fuzzy match: `--provider cl` should canonicalise to `claude`
/// before clap parses, so the dry-run command completes successfully and
/// reports Claude as the selected provider.
#[test]
fn headline_compose_with_fuzzy_provider_resolves_to_claude() {
    let workspace = tempdir().unwrap();
    let fixture = write_fixture(workspace.path(), "fuzzy.md");

    let output = cargo_bin_cmd!("claudine")
        .env("NO_COLOR", "1")
        .args([
            "compose",
            "--provider",
            "cl",
            fixture.to_str().unwrap(),
            "--dry-run",
        ])
        .assert()
        .success()
        .get_output()
        .clone();

    let stdout = strip_ansi(&String::from_utf8_lossy(&output.stdout));
    let stderr = strip_ansi(&String::from_utf8_lossy(&output.stderr));
    let combined = format!("{stderr}{stdout}");

    assert!(
        combined.contains("AGENT=claude"),
        "dry-run should report `AGENT=claude` after fuzzy match \
         (`cl` → `claude`); got: {combined}"
    );
    assert!(
        combined.contains("[DRY RUN]"),
        "dry-run header should appear in output; got: {combined}"
    );
}

/// Common case (no flags between positional + setter) must remain
/// behaviorally identical — Rule 3 does not fire, Rule 1/2 rewrite nothing.
#[test]
fn headline_compose_with_plain_setter_behaves_as_before() {
    let workspace = tempdir().unwrap();
    let fixture = write_fixture(workspace.path(), "plain.md");

    let output = cargo_bin_cmd!("claudine")
        .env("NO_COLOR", "1")
        .args([
            "compose",
            fixture.to_str().unwrap(),
            "key=val",
            "--provider",
            "claude",
            "--dry-run",
        ])
        .assert()
        .success()
        .get_output()
        .clone();

    let stdout = strip_ansi(&String::from_utf8_lossy(&output.stdout));
    let stderr = strip_ansi(&String::from_utf8_lossy(&output.stderr));
    let combined = format!("{stderr}{stdout}");

    assert!(
        combined.contains("AGENT=claude"),
        "plain setter path should still route to the chosen provider; \
         got: {combined}"
    );
    assert!(
        combined.contains("[DRY RUN]"),
        "dry-run header should appear in output; got: {combined}"
    );
}

// ──────────────────────────────────────────────────────────────────────
// Pass-through regression cases (spec: "Pass-through guarantees")
// ──────────────────────────────────────────────────────────────────────

/// `--version` is a root-level flag with no subcommand; the normalizer
/// must not touch it.
#[test]
fn passthrough_version_flag_still_prints_version_string() {
    let output = cargo_bin_cmd!("claudine")
        .env("NO_COLOR", "1")
        .arg("--version")
        .assert()
        .success()
        .get_output()
        .clone();

    let stdout = strip_ansi(&String::from_utf8_lossy(&output.stdout));
    assert!(
        stdout.starts_with("claudine "),
        "expected `claudine <version>` on stdout; got: {stdout}"
    );
}

/// `claudine --help` (root, no subcommand) must continue to render the
/// grouped help screen through Claudine's custom help handler.
#[test]
fn passthrough_root_help_renders_custom_help_screen() {
    let output = cargo_bin_cmd!("claudine")
        .env("NO_COLOR", "1")
        .arg("--help")
        .assert()
        .success()
        .get_output()
        .clone();

    let stdout = strip_ansi(&String::from_utf8_lossy(&output.stdout));
    assert!(
        stdout.contains("Claudine"),
        "custom help screen should contain the `Claudine` title; \
         got: {stdout}"
    );
    assert!(
        stdout.contains("claudine"),
        "custom help screen should include the binary name; \
         got: {stdout}"
    );
}

/// `claudine hooks --describe` exercises a non-composition subcommand
/// path. Rule 3 is gated on composition subcommands and must not fire
/// here; Rules 1 and 2 have nothing to rewrite.
#[test]
fn passthrough_hooks_describe_still_runs() {
    let output = cargo_bin_cmd!("claudine")
        .env("NO_COLOR", "1")
        .args(["hooks", "--describe"])
        .assert()
        .success()
        .get_output()
        .clone();

    let stdout = strip_ansi(&String::from_utf8_lossy(&output.stdout));
    assert!(
        stdout.contains("Response Schema") || stdout.contains("Return Schema"),
        "hooks --describe should still render its schema output; \
         got: {stdout}"
    );
}

// `COMPLETE` completion-mode guard is covered by
// `normalize_is_noop_when_complete_env_is_set` inside `claudine/cli/src/argv.rs`
// rather than here; exercising it through the compiled binary would
// trigger `clap_complete::CompleteEnv::complete()` and short-circuit the
// whole CLI before our normalizer runs, which defeats the purpose of
// the regression.
