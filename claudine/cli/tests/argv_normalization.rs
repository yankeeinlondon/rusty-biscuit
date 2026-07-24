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

/// Rule 4 + Rule 3 headline: `compose <file> --<provider-bool> key=val --help`
/// must render the compose-tier help screen, not a clap error.
///
/// Before this feature, clap emitted the misleading
/// `tip: to pass '--help' as a value, use '-- --help'` diagnostic because
/// the greedy positional collector swallowed `--help`. The first fix
/// landed only Rule 1 and Rule 3 — that suppressed the clap tip but
/// surfaced a new "expected at most one file reference" Claudine error
/// because `--` buried `--help` in the trailing raw-value bucket, which
/// the downstream positional parser then misclassified as a second file
/// reference. Rule 4 closes the loop by hoisting `--help` to position 1,
/// so the root `cli.help` handler fires and the grouped help screen
/// renders.
///
/// The assertion suite covers three user-visible guarantees:
///
/// 1. the process exits successfully;
/// 2. Claudine's own help content appears (the command group heading
///    and the `compose` entry are both visible);
/// 3. neither the original clap tip nor the secondary Claudine error
///    leak into the output.
#[test]
fn headline_compose_with_interleaved_flag_renders_help() {
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
        .assert()
        .success();

    let output = assert.get_output().clone();
    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);
    let combined = format!("{stderr}{stdout}");
    let plain = strip_ansi(&combined);

    assert!(
        plain.contains("Compose a Markdown document"),
        "expected Claudine's help output to include the `compose` \
         command description; got: {plain}"
    );
    assert!(
        plain.contains("Composition"),
        "expected Claudine's help output to include the `Composition` \
         command group heading; got: {plain}"
    );
    assert!(
        !plain.contains("tip: to pass '--help' as a value"),
        "normalizer must suppress the misleading clap help tip; \
         got: {plain}"
    );
    assert!(
        !plain.contains("unexpected argument '--help'"),
        "normalizer must keep `--help` from tripping clap's greedy \
         positional collection; got: {plain}"
    );
    assert!(
        !plain.contains("expected at most one file reference"),
        "Rule 4 must hoist `--help` off the trailing raw-value bucket \
         so the downstream positional parser never misclassifies it \
         as a second file reference; got: {plain}"
    );
}

/// Rule 4 simpler form: `compose <file> --help` (no interleaved
/// setters) must also render the help screen.
#[test]
fn headline_compose_with_trailing_help_renders_help() {
    let workspace = tempdir().unwrap();
    let fixture = write_fixture(workspace.path(), "simple.md");

    let assert = cargo_bin_cmd!("claudine")
        .env("NO_COLOR", "1")
        .args(["compose", fixture.to_str().unwrap(), "--help"])
        .assert()
        .success();

    let output = assert.get_output().clone();
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    let combined = format!("{stderr}{stdout}");
    let plain = strip_ansi(&combined);

    assert!(
        plain.contains("Compose a Markdown document"),
        "trailing `--help` on compose must render Claudine help; \
         got: {plain}"
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
        combined.contains("Claude"),
        "dry-run should resolve to Claude after fuzzy match \
         (`cl` → `claude`); got: {combined}"
    );
    // The composed body on stdout is the defining dry-run marker.
    assert!(
        stdout.contains("Hello."),
        "dry-run should render the composed body to stdout; got: {combined}"
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
        combined.contains("Claude"),
        "plain setter path should still route to the chosen provider; \
         got: {combined}"
    );
    // The composed body on stdout is the defining dry-run marker.
    assert!(
        stdout.contains("Hello."),
        "dry-run should render the composed body to stdout; got: {combined}"
    );
}

/// Mixed-order regression from `just clarify`: provider flag before the
/// setter, then more boolean flags after the setter. The command should
/// still parse all flags and succeed under `--dry-run`.
#[test]
fn headline_compose_with_setter_then_late_flags_preserves_flag_semantics() {
    let workspace = tempdir().unwrap();
    let fixture = write_fixture(workspace.path(), "clarify.md");
    let empty_path = workspace.path().join("empty-path");
    fs::create_dir(&empty_path).unwrap();

    let output = cargo_bin_cmd!("claudine")
        .env("NO_COLOR", "1")
        .env("TERM_WIDTH", "120")
        .env("PATH", empty_path)
        .args([
            "compose",
            fixture.to_str().unwrap(),
            "--gemini",
            "doc=@pkg/feature/spec.md",
            "-y",
            "-i",
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
        combined.contains("Gemini"),
        "expected provider selection to survive mixed ordering; got: {combined}"
    );
    // `-y` surfaces in the dry-run metadata table's YOLO row.
    assert!(
        combined.contains("YOLO") && combined.contains("true"),
        "expected trailing `-y` to remain a Claudine flag; got: {combined}"
    );
    // `-i` is not surfaced in the dry-run output, but a `-i` that failed to
    // normalize would be misclassified as a second positional file reference
    // and abort before any output. The rendered dry-run body therefore proves
    // `-i` was parsed as the interactive flag, not swallowed as a positional.
    assert!(
        stdout.contains("Hello."),
        "expected trailing `-i` to parse as a flag and the dry-run body to \
         render to stdout; got: {combined}"
    );
}

/// Setter-first form should also work: later flags must be hoisted ahead of
/// the setter rather than disappearing into the positional bucket.
#[test]
fn headline_compose_with_setter_before_late_flags_preserves_flag_semantics() {
    let workspace = tempdir().unwrap();
    let fixture = write_fixture(workspace.path(), "setter-first.md");
    let empty_path = workspace.path().join("empty-path");
    fs::create_dir(&empty_path).unwrap();

    let output = cargo_bin_cmd!("claudine")
        .env("NO_COLOR", "1")
        .env("TERM_WIDTH", "120")
        .env("PATH", empty_path)
        .args([
            "compose",
            fixture.to_str().unwrap(),
            "doc=@pkg/feature/spec.md",
            "--gemini",
            "-y",
            "-i",
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
        combined.contains("Gemini"),
        "expected provider selection to survive setter-first ordering; got: {combined}"
    );
    // `-y` surfaces in the dry-run metadata table's YOLO row.
    assert!(
        combined.contains("YOLO") && combined.contains("true"),
        "expected `-y` after a setter to remain a Claudine flag; got: {combined}"
    );
    // `-i` is not surfaced in the dry-run output, but a `-i` that failed to
    // normalize would be misclassified as a second positional file reference
    // and abort before any output. The rendered dry-run body therefore proves
    // `-i` was parsed as the interactive flag, not swallowed as a positional.
    assert!(
        stdout.contains("Hello."),
        "expected `-i` after a setter to parse as a flag and the dry-run body \
         to render to stdout; got: {combined}"
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

/// Non-composition subcommands must accept `--help` and render clap's
/// per-subcommand help screen. The root `Cli` sets `disable_help_flag = true`,
/// which clap propagates to subcommands; `parse_cli_from` re-injects an
/// `ArgAction::Help` arg on every non-wrapper subcommand to compensate.
#[test]
fn non_composition_subcommands_accept_help_flag() {
    for sub in [
        "completions",
        "hooks",
        "skills",
        "sync",
        "providers",
        "agents",
    ] {
        let output = cargo_bin_cmd!("claudine")
            .env("NO_COLOR", "1")
            .args([sub, "--help"])
            .assert()
            .success()
            .get_output()
            .clone();

        let stdout = strip_ansi(&String::from_utf8_lossy(&output.stdout));
        assert!(
            stdout.contains("Usage:") && stdout.contains("--help"),
            "`claudine {sub} --help` should render clap's per-subcommand \
             help screen with a Usage line and a --help entry; got: {stdout}"
        );
    }
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

// ──────────────────────────────────────────────────────────────────────
// Feature: `2026-04-17-file-completion` (Phase 1.4)
//
// The pre-clap argv normalizer has a `COMPLETE` env-var guard that makes
// `normalize_inner` a strict no-op under completion mode. The unit test
// `normalize_is_noop_when_complete_env_is_set` inside
// `claudine/cli/src/argv.rs` proves that guard at the function boundary.
//
// Phase 1.4 adds the complementary binary-level proof: once
// `completion::maybe_complete()` runs in `main.rs` before argv
// normalization, setting `COMPLETE=<shell>` must exit via
// `CompleteEnv::complete()` with a registration snippet on stdout — it
// must *not* fall through into argv normalization, wrapper parsing,
// config loading, or any runtime error path. If it ever does, users who
// have completions wired up will see spurious CLI chatter on every
// shell startup.
// ──────────────────────────────────────────────────────────────────────

/// Verify that setting `COMPLETE=<shell>` short-circuits the binary at
/// the `maybe_complete()` hook. A completion subprocess must:
///
/// 1. exit `success`;
/// 2. emit a registration snippet on stdout (the clap_complete handshake
///    always names the binary, `claudine`, in the generated script);
/// 3. leave stderr completely clean — normalization, config checks,
///    telemetry init, and wrapper launch must all be skipped.
///
/// The wrapper-argv form (`COMPLETE=bash claudine claude --some-flag`)
/// is also exercised to prove the short-circuit beats the
/// wrapper-launch branch specifically. Without the `maybe_complete()`
/// hook in `main.rs`, the wrapper form would try to spawn the Claude
/// Code CLI and the assertion would fail noisily.
#[test]
fn complete_env_short_circuits_before_argv_normalization() {
    let output = cargo_bin_cmd!("claudine")
        .env("NO_COLOR", "1")
        .env("COMPLETE", "bash")
        .assert()
        .success()
        .get_output()
        .clone();

    let stdout = strip_ansi(&String::from_utf8_lossy(&output.stdout));
    let stderr = strip_ansi(&String::from_utf8_lossy(&output.stderr));
    assert!(
        stdout.contains("claudine"),
        "COMPLETE=bash registration must mention the binary name; \
         got stdout: {stdout}",
    );
    assert!(
        stderr.trim().is_empty(),
        "COMPLETE=bash subprocess must not touch stderr; \
         got stderr: {stderr}",
    );
}

#[test]
fn complete_env_short_circuits_wrapper_argv_without_launching_provider() {
    // Without the `maybe_complete()` hook, this invocation would fall
    // through to the wrapper launch path and either spawn the Claude
    // Code binary or fail with a "claude not found" error on stderr.
    // The short-circuit is what keeps shell completion setup cheap.
    let output = cargo_bin_cmd!("claudine")
        .env("NO_COLOR", "1")
        .env("COMPLETE", "bash")
        .args(["claude", "--some-passthrough-flag"])
        .assert()
        .success()
        .get_output()
        .clone();

    let stderr = strip_ansi(&String::from_utf8_lossy(&output.stderr));
    assert!(
        stderr.trim().is_empty(),
        "COMPLETE=bash wrapper subprocess must never reach wrapper \
         launch; got stderr: {stderr}",
    );
}

/// `spec.md`: `--claud` (near-miss of `--claude`) is intentionally NOT
/// rewritten by Rule 1. Clap must surface the standard unknown-argument
/// error so the user sees a diagnostic that points at the token they
/// actually typed instead of a silent rewrite. This closes the
/// acceptance-criterion loop end-to-end.
#[test]
fn non_owned_flag_after_file_is_forwarded_to_agent() {
    // With provider-argument forwarding, a switch Claudine does not own —
    // including a near-miss like `--claud` — placed *after* the composition
    // file starts the agent tail and is forwarded verbatim rather than
    // rejected by clap. The dry-run "Provider args" row audits the tail.
    let workspace = tempdir().unwrap();
    let fixture = write_fixture(workspace.path(), "near-miss.md");

    let assert = cargo_bin_cmd!("claudine")
        .env("NO_COLOR", "1")
        .args([
            "compose",
            fixture.to_str().unwrap(),
            "--codex",
            "--dry-run",
            "--claud",
        ])
        .assert()
        .success();

    let output = assert.get_output().clone();
    let stderr = strip_ansi(&String::from_utf8_lossy(&output.stderr));
    let stdout = strip_ansi(&String::from_utf8_lossy(&output.stdout));
    let combined = format!("{stderr}{stdout}");

    assert!(
        combined.contains("Provider args") && combined.contains("--claud"),
        "a non-owned flag after the file must be forwarded to the agent and \
         shown in the dry-run Provider args row; got: {combined}"
    );
    assert!(
        !combined.contains("unexpected argument"),
        "clap must not reject a forwarded non-owned flag; got: {combined}"
    );
}

#[test]
fn non_owned_flag_before_file_errors_with_ordering_guidance() {
    // The ordering rule: an unowned switch before the composition file is a
    // partition error with targeted guidance, not a silent guess.
    let workspace = tempdir().unwrap();
    let fixture = write_fixture(workspace.path(), "near-miss.md");

    let assert = cargo_bin_cmd!("claudine")
        .env("NO_COLOR", "1")
        .args(["compose", "--claud", fixture.to_str().unwrap()])
        .assert()
        .failure();

    let output = assert.get_output().clone();
    let stderr = strip_ansi(&String::from_utf8_lossy(&output.stderr));
    let stdout = strip_ansi(&String::from_utf8_lossy(&output.stdout));
    let combined = format!("{stderr}{stdout}");

    assert!(
        combined.contains("--claud") && combined.contains("before the composition file"),
        "an unowned switch before the file must surface ordering guidance; got: {combined}"
    );
}
