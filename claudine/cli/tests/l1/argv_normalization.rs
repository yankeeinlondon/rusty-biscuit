//! End-to-end integration tests for the pre-clap argv normalization layer.
//!
//! These tests drive the compiled `claudine` binary so we exercise the
//! actual path from `std::env::args_os()` through `argv::normalize` into
//! clap. Unit coverage for the rewrite rules lives alongside the
//! implementation in `claudine/cli/src/argv/`; this suite proves that
//! the normalization shows up at the binary entrypoint and does not
//! disturb unaffected commands.
//!
//! It also proves the executable's composition help selection, which runs on
//! the original argv before normalization: command-specific screens, the
//! validation they bypass, plain mode, and forwarding of `--help` after `--`.
//!
//! Feature: `2026-04-17-cli-pre-processing`.

use std::fs;

use crate::common;
use common::{CliProcessFixture, strip_ansi};

/// Minimal frontmatter-only markdown fixture that composition can resolve.
const FIXTURE_MD: &str = "---\ntitle: argv normalization fixture\n---\n\nHello.\n";

fn write_fixture(dir: &std::path::Path, name: &str) -> std::path::PathBuf {
    let path = dir.join(name);
    fs::write(&path, FIXTURE_MD).unwrap();
    path
}

/// One help invocation's channels, ANSI-stripped and kept apart so each
/// assertion names the channel it is about.
struct HelpRun {
    stdout: String,
    stderr: String,
}

impl HelpRun {
    fn from_output(output: &std::process::Output) -> Self {
        Self {
            stdout: strip_ansi(&String::from_utf8_lossy(&output.stdout)),
            stderr: strip_ansi(&String::from_utf8_lossy(&output.stderr)),
        }
    }
}

/// Run `args` with the fixture's default command and require exit 0.
fn run_help(cli: &CliProcessFixture, args: &[&str]) -> HelpRun {
    run_help_with(cli.command(), args)
}

fn run_help_with(mut command: assert_cmd::Command, args: &[&str]) -> HelpRun {
    let output = command.args(args).output().unwrap();
    let help = HelpRun::from_output(&output);
    assert!(
        output.status.success(),
        "`claudine {}` must exit 0; status {:?}\nstdout: {}\nstderr: {}",
        args.join(" "),
        output.status.code(),
        help.stdout,
        help.stderr
    );
    help
}

/// Collapse every whitespace run to one space: clap wraps help to the
/// terminal width, so a phrase may break at a host-dependent point.
fn flatten(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// The contract of a selected composition help screen: `command`'s own
/// compact clap help on stdout, nothing on stderr, never the root overview.
fn assert_composition_help(command: &str, help: &HelpRun) {
    assert!(help.stderr.is_empty(), "help must leave stderr empty; got: {}", help.stderr);
    assert!(!help.stdout.trim().is_empty(), "help must print to stdout");
    let flat = flatten(&help.stdout);
    let usage = format!("Usage: claudine {command} [OPTIONS] <ARG>...");
    assert!(flat.contains(&usage), "expected `{usage}`; got: {}", help.stdout);

    let file = if command == "sequence" { "sequence.md" } else { "prompt.md" };
    let example = format!("claudine {command} {file} name=Ken -- --help");
    for explanation in [
        "One required file reference, then optional `key=value` setters (overriding `--set`) and arguments for the provider",
        example.as_str(),
        "Everything after `--` is forwarded to the provider unchanged",
        "Provider-native options are not listed above.",
    ] {
        assert!(
            flat.contains(explanation),
            "`{command}` help must explain `{explanation}`; got: {}",
            help.stdout
        );
    }

    let tokens: Vec<&str> = flat.split(' ').collect();
    for option in [
        "--help",
        "--provider",
        "--set",
        "--dry-run",
        "--dry-run-shell",
        "--max-iterations",
        "--on-rate-limit",
    ] {
        assert!(
            tokens.contains(&option),
            "`{command}` help must list `{option}`; got: {}",
            help.stdout
        );
    }
    for root_heading in ["Shared Resources:", "Composition:"] {
        assert!(
            !help.stdout.contains(root_heading),
            "`{command}` help must not be the grouped root overview \
             (found `{root_heading}`); got: {}",
            help.stdout
        );
    }
}

/// The grouped root overview, as printed by `claudine --help`.
fn assert_root_overview(help: &HelpRun) {
    assert!(help.stderr.is_empty(), "root help must leave stderr empty; got: {}", help.stderr);
    for expected in ["Usage: claudine <command> [options]", "Shared Resources:", "Composition:"] {
        assert!(
            help.stdout.contains(expected),
            "root overview must contain `{expected}`; got: {}",
            help.stdout
        );
    }
    assert!(
        !help.stdout.contains("Usage: claudine compose"),
        "root overview must not be the compose screen; got: {}",
        help.stdout
    );
}

// ──────────────────────────────────────────────────────────────────────
// Headline integration cases (spec: "Testing → Integration tests")
// ──────────────────────────────────────────────────────────────────────

/// Headline regression: `compose <file> --<provider-bool> key=val --help`
/// renders the compose command's own help screen.
///
/// Help is selected from the original argv, before normalization and the
/// ownership partition, so the greedy positional collector never sees
/// `--help`: no clap value tip, no unexpected-argument error, and no
/// second-file-reference error, and the screen is `compose`'s usage rather
/// than the grouped root overview.
#[test]
fn headline_compose_with_interleaved_flag_renders_help() {
    let cli = CliProcessFixture::named("argv-normalization");
    let fixture = write_fixture(cli.cwd(), "greet.md");

    let help = run_help(
        &cli,
        &["compose", fixture.to_str().unwrap(), "--gemini", "name=Ken", "--help"],
    );

    assert_composition_help("compose", &help);
    assert!(
        help.stdout.contains("Compose a Markdown document"),
        "expected the `compose` command description; got: {}",
        help.stdout
    );
    for misleading in [
        "tip: to pass '--help' as a value",
        "unexpected argument '--help'",
        "expected at most one file reference",
    ] {
        assert!(
            !help.stdout.contains(misleading),
            "compose help must not contain `{misleading}`; got: {}",
            help.stdout
        );
    }
}

/// Simpler form: `compose <file> --help` (no interleaved setters) renders
/// the compose command's own help screen.
#[test]
fn headline_compose_with_trailing_help_renders_help() {
    let cli = CliProcessFixture::named("argv-normalization");
    let fixture = write_fixture(cli.cwd(), "simple.md");

    let help = run_help(&cli, &["compose", fixture.to_str().unwrap(), "--help"]);

    assert_composition_help("compose", &help);
}

/// Rule 2 fuzzy match: `--provider cl` should canonicalise to `claude`
/// before clap parses, so the dry-run command completes successfully and
/// reports Claude as the selected provider.
#[test]
fn headline_compose_with_fuzzy_provider_resolves_to_claude() {
    let cli = CliProcessFixture::named("argv-normalization");
    let fixture = write_fixture(cli.cwd(), "fuzzy.md");

    let output = cli
        .command()
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
    let cli = CliProcessFixture::named("argv-normalization");
    let fixture = write_fixture(cli.cwd(), "plain.md");

    let output = cli
        .command()
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
    let cli = CliProcessFixture::named("argv-normalization");
    let fixture = write_fixture(cli.cwd(), "clarify.md");
    let output = cli
        // Escape: fake-only PATH. The dry run must not find a host `gemini`,
        // whose real path would then reach the assertions below.
        .command_builder()
        .fake_only_path()
        .build()
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
    let cli = CliProcessFixture::named("argv-normalization");
    let fixture = write_fixture(cli.cwd(), "setter-first.md");
    let output = cli
        // Escape: fake-only PATH. The dry run must not find a host `gemini`,
        // whose real path would then reach the assertions below.
        .command_builder()
        .fake_only_path()
        .build()
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
/// must not touch it. Completion-only variables are cleared so an invoking
/// shell cannot redirect this subprocess into clap's completion bootstrap.
#[test]
fn passthrough_version_flag_still_prints_version_string() {
    let cli = CliProcessFixture::named("argv-passthrough-version");
    let output = cli
        .command()
        .env_remove("COMPLETE")
        .env_remove("_CLAP_COMPLETE_INDEX")
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
    assert!(output.stderr.is_empty(), "`--version` must leave stderr empty");
}

/// `claudine --help` (root, no subcommand) must continue to render the
/// grouped help screen through Claudine's custom help handler.
#[test]
fn passthrough_root_help_renders_custom_help_screen() {
    let cli = CliProcessFixture::named("argv-passthrough-root-help");
    let help = run_help(&cli, &["--help"]);
    assert_root_overview(&help);

    let stdout = help.stdout;
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
    let cli = CliProcessFixture::named("argv-subcommand-help");
    for sub in [
        "completions",
        "hooks",
        "skills",
        "sync",
        "providers",
        "agents",
    ] {
        let output = cli
            .command()
            .args([sub, "--help"])
            .assert()
            .success()
            .get_output()
            .clone();

        let stdout = strip_ansi(&String::from_utf8_lossy(&output.stdout));
        assert!(output.stderr.is_empty(), "`claudine {sub} --help` must leave stderr empty");
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
    let cli = CliProcessFixture::named("argv-hooks-describe");
    let output = cli
        .command()
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
// `claudine/cli/src/argv/` proves that guard at the function boundary.
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
    let cli = CliProcessFixture::named("argv-complete-env");
    let output = cli
        .command()
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
    let cli = CliProcessFixture::named("argv-complete-env-wrapper");
    // Without the `maybe_complete()` hook, this invocation would fall
    // through to the wrapper launch path and either spawn the Claude
    // Code binary or fail with a "claude not found" error on stderr.
    // The short-circuit is what keeps shell completion setup cheap.
    let output = cli
        .command()
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
    let cli = CliProcessFixture::named("argv-normalization");
    let fixture = write_fixture(cli.cwd(), "near-miss.md");

    let assert = cli
        .command()
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
    let cli = CliProcessFixture::named("argv-normalization");
    let fixture = write_fixture(cli.cwd(), "near-miss.md");

    let assert = cli
        .command()
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

// ──────────────────────────────────────────────────────────────────────
// Composition command help
//
// `-h`/`--help` before the first `--` and after a composition command prints
// that command's compact clap help and exits 0 before normalization, the
// ownership partition, configuration, or provider launch. A help token before
// the command prints the grouped root overview instead.
// ──────────────────────────────────────────────────────────────────────

/// Run `claudine <command> <tail>` and require the composition help contract.
///
/// The tail names no file or a file that does not exist, so a passing screen
/// also proves help reads no input.
fn assert_help_without_input(command: &str, tail: &[&str]) {
    let cli = CliProcessFixture::named("composition-help");
    let mut args = vec![command];
    args.extend_from_slice(tail);
    let help = run_help_with(
        cli.command_builder()
            // Escape: fake-only PATH. Only the empty fixture `bin` is
            // searchable, so this screen proves help needs no installed provider.
            .fake_only_path()
            .build(),
        &args,
    );
    assert_composition_help(command, &help);
    assert!(
        !cli.home().join(".claudine/config.json").exists(),
        "help must not create user configuration"
    );
}

macro_rules! composition_help_cases {
    ($command:literal: $short:ident, $long:ident, $short_missing:ident, $long_missing:ident) => {
        #[test]
        fn $short() {
            assert_help_without_input($command, &["-h"]);
        }

        #[test]
        fn $long() {
            assert_help_without_input($command, &["--help"]);
        }

        #[test]
        fn $short_missing() {
            assert_help_without_input($command, &["missing.md", "-h"]);
        }

        #[test]
        fn $long_missing() {
            assert_help_without_input($command, &["missing.md", "--help"]);
        }
    };
}

composition_help_cases!("compose":
    compose_short_help_without_file,
    compose_long_help_without_file,
    compose_short_help_with_missing_file,
    compose_long_help_with_missing_file
);
composition_help_cases!("inline-compose":
    inline_compose_short_help_without_file,
    inline_compose_long_help_without_file,
    inline_compose_short_help_with_missing_file,
    inline_compose_long_help_with_missing_file
);
composition_help_cases!("sequence":
    sequence_short_help_without_file,
    sequence_long_help_without_file,
    sequence_short_help_with_missing_file,
    sequence_long_help_with_missing_file
);

/// A help request bypasses validation that would reject the same arguments
/// without it. The matching no-help failures are proved in process by the
/// `composition_help` and `rule4_help_hoist` unit tests.
macro_rules! help_bypasses_validation {
    ($name:ident, $command:literal, [$($arg:literal),+ $(,)?]) => {
        #[test]
        fn $name() {
            let cli = CliProcessFixture::named("composition-help-bypass");
            let help = run_help(&cli, &[$command, $($arg),+]);
            assert_composition_help($command, &help);
        }
    };
}

help_bypasses_validation!(help_bypasses_unknown_provider, "compose",
    ["--provider", "nope", "prompt.md", "-h"]);
help_bypasses_validation!(help_bypasses_missing_provider_value, "compose",
    ["--provider", "--help"]);
help_bypasses_validation!(help_bypasses_malformed_setter_json, "compose",
    ["--set", "{broken", "prompt.md", "--help"]);
help_bypasses_validation!(help_bypasses_dry_run_shell_without_dry_run, "inline-compose",
    ["--dry-run-shell", "prompt.md", "-h"]);
help_bypasses_validation!(help_bypasses_conflicting_interactive_switches, "sequence",
    ["-i", "--no-interactive", "sequence.md", "--help"]);
help_bypasses_validation!(help_bypasses_unknown_switch_before_file, "compose",
    ["--claud", "prompt.md", "--help"]);

// Output-volume and perf switches do not suppress help, and `--perf` adds
// no report on stderr.
help_bypasses_validation!(help_ignores_quiet, "compose", ["--quiet", "prompt.md", "-h"]);
help_bypasses_validation!(help_ignores_silent, "inline-compose", ["--silent", "--help"]);
help_bypasses_validation!(help_ignores_perf, "sequence", ["--perf", "sequence.md", "--help"]);

/// A help token before the composition command asks for the root overview,
/// even when the composition arguments after it are invalid.
#[test]
fn root_help_before_composition_command_renders_root_overview() {
    let cli = CliProcessFixture::named("composition-help-root");
    let help = run_help(&cli, &["--help", "compose", "--provider", "nope"]);
    assert_root_overview(&help);
}

#[test]
fn bare_invocation_renders_root_overview() {
    let cli = CliProcessFixture::named("argv-bare");
    let help = run_help(&cli, &[]);
    assert_root_overview(&help);
}

macro_rules! admin_help {
    ($name:ident, $command:literal) => {
        #[test]
        fn $name() {
            let cli = CliProcessFixture::named("argv-admin-help");
            let help = run_help(&cli, &[$command, "--help"]);
            assert!(help.stderr.is_empty(), "stderr: {}", help.stderr);
            let usage = concat!("Usage: claudine ", $command, " [OPTIONS]");
            assert!(help.stdout.contains(usage), "expected `{usage}`; got: {}", help.stdout);
        }
    };
}

admin_help!(mcp_help_renders_clap_screen, "mcp");
admin_help!(actions_help_renders_clap_screen, "actions");

/// Wrapper help stays the wrapper's own screen and launches nothing.
#[test]
fn wrapper_help_renders_wrapper_screen() {
    let cli = CliProcessFixture::named("argv-wrapper-help");
    let help = run_help(&cli, &["claude", "--help"]);
    assert!(help.stderr.is_empty(), "stderr: {}", help.stderr);
    for expected in [
        "Usage: claudine claude [OPTIONS] [ARGS]...",
        "Arguments forwarded to the wrapped provider CLI",
    ] {
        assert!(help.stdout.contains(expected), "expected `{expected}`; got: {}", help.stdout);
    }
}

/// Run `args` with color forced on, returning raw (unstripped) stdout.
///
/// clap's renderer honors `CLICOLOR_FORCE`; Claudine's own terminal honors
/// `FORCE_COLOR`. The fixture's `NO_COLOR=1` is removed on this child only,
/// since it outranks both.
fn forced_color_stdout(args: &[&str]) -> String {
    let cli = CliProcessFixture::named("composition-help-color");
    let mut command = cli.command();
    command.env_remove("NO_COLOR").env("CLICOLOR_FORCE", "1").env("FORCE_COLOR", "1");
    let output = command.args(args).output().unwrap();
    assert!(output.status.success(), "`claudine {}` must exit 0", args.join(" "));
    assert!(
        output.stderr.is_empty(),
        "help must leave stderr empty; got: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout).unwrap()
}

macro_rules! plain_help {
    ($name:ident, [$($arg:literal),+], $usage:literal) => {
        #[test]
        fn $name() {
            let stdout = forced_color_stdout(&[$($arg),+]);
            assert!(stdout.contains($usage), "expected `{}`; got: {stdout}", $usage);
            assert_eq!(stdout, strip_ansi(&stdout), "`--plain` must remove every escape");
        }
    };
}

plain_help!(plain_before_composition_command_disables_color,
    ["--plain", "compose", "-h"], "Usage: claudine compose");
plain_help!(plain_after_composition_command_disables_color,
    ["sequence", "--help", "--plain"], "Usage: claudine sequence");
plain_help!(plain_root_help_disables_color, ["--plain", "--help"], "Usage: claudine <command>");

/// Controls for the plain cases: with color forced and no `--plain`, both
/// renderers emit escapes, so the plain assertions above are load-bearing.
#[test]
fn forced_color_composition_help_contains_escapes() {
    let stdout = forced_color_stdout(&["compose", "-h"]);
    assert!(stdout.contains('\u{1b}'), "forced color must style compose help; got: {stdout}");
    assert!(strip_ansi(&stdout).contains("Usage: claudine compose"));
}

#[test]
fn forced_color_root_help_contains_escapes() {
    let stdout = forced_color_stdout(&["--help"]);
    assert!(stdout.contains('\u{1b}'), "forced color must style root help; got: {stdout}");
}

/// A document that would mutate itself, append a lifecycle log, and launch
/// `goose` if it ran.
#[cfg(feature = "test-fixtures")]
const TRAP_DOC: &str = "---\nagent: goose\nprompt: Say hello.\ninitialize:\n  - append_line: [help-lifecycle.log, ran]\n---\n\nOriginal body.\n";

/// `inline-compose` rewrites its input when it runs, so a help request on a
/// real, launchable document must leave no trace: no provider launch, no
/// lifecycle effect, unchanged bytes, no configuration, no audio.
#[cfg(feature = "test-fixtures")]
fn assert_inline_help_has_no_effects(help_token: &str) {
    let cli = CliProcessFixture::named("composition-help-trap");
    fs::copy(
        biscuit_test_harness::bin_exe!("claudine-fake-goose"),
        cli.bin_dir().join(format!("goose{}", std::env::consts::EXE_SUFFIX)),
    )
    .expect("the fake provider is built with `test-fixtures`");
    let marker = cli.cwd().join("provider-launched.txt");
    let document = cli.cwd().join("doc.md");
    fs::write(&document, TRAP_DOC).unwrap();

    let mut command = cli.command();
    command.env("PROVIDER_MARKER", &marker);
    let help = run_help_with(command, &["inline-compose", "doc.md", help_token]);

    assert_composition_help("inline-compose", &help);
    assert!(!marker.exists(), "help must not launch the provider");
    assert!(!cli.cwd().join("help-lifecycle.log").exists(), "help must not run `initialize`");
    assert_eq!(fs::read_to_string(&document).unwrap(), TRAP_DOC, "help must not touch the input");
    assert!(
        !cli.home().join(".claudine/config.json").exists(),
        "help must not create user configuration"
    );
    assert!(!cli.audio_spool().exists(), "help must publish no audio");
}

#[cfg(feature = "test-fixtures")]
#[test]
fn inline_compose_long_help_on_real_document_has_no_effects() {
    assert_inline_help_has_no_effects("--help");
}

#[cfg(feature = "test-fixtures")]
#[test]
fn inline_compose_short_help_on_real_document_has_no_effects() {
    assert_inline_help_has_no_effects("-h");
}

/// After the authored `--`, `--help` belongs to the provider: the composition
/// executes for real and the provider receives exactly one `--help`, without
/// the separator.
#[cfg(feature = "test-fixtures")]
#[test]
fn help_after_separator_is_forwarded_to_provider() {
    let cli = CliProcessFixture::named("composition-help-forward");
    cli.seed_user_config();
    fs::copy(
        biscuit_test_harness::bin_exe!("claudine-fake-completion"),
        cli.bin_dir().join(format!("claude{}", std::env::consts::EXE_SUFFIX)),
    )
    .expect("the fake provider is built with `test-fixtures`");
    let record = cli.workspace_path().join("provider-record");
    fs::create_dir(&record).unwrap();
    let prompt = write_fixture(cli.cwd(), "prompt.md");

    let output = cli
        .command()
        .env("FAKE_COMPLETION_PROVIDER", "claude")
        .env("FAKE_COMPLETION_DIR", &record)
        .args(["compose", prompt.to_str().unwrap(), "--claude", "--", "--help"])
        .output()
        .unwrap();
    let run = HelpRun::from_output(&output);
    assert!(output.status.success(), "stdout: {}\nstderr: {}", run.stdout, run.stderr);

    let argv: Vec<String> =
        serde_json::from_slice(&fs::read(record.join("argv.json")).unwrap()).unwrap();
    assert_eq!(
        argv.iter().filter(|arg| *arg == "--help").count(),
        1,
        "the provider must receive exactly one `--help`; got: {argv:?}"
    );
    assert!(!argv.iter().any(|arg| arg == "--"), "the authored `--` is consumed; got: {argv:?}");
    assert!(run.stdout.contains("review-answer"), "expected provider output; got: {}", run.stdout);
    assert!(
        !run.stdout.contains("Usage: claudine compose"),
        "a forwarded `--help` must not print composition help; got: {}",
        run.stdout
    );
}
