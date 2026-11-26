//! Integration tests: sequence composition dry-run/live agent-state rendering.
//!
//! Split out of the `wrap_commands.rs` god file; shared fixtures live in
//! `common::wrap`.

use std::fs;
use crate::common;
#[cfg(unix)]
use claudine::provider::{PROVIDERS_DISPLAY_ORDER, Provider, provider_info};
use common::{CliProcessFixture, strip_ansi, write_executable};

/// End-to-end: for every wrapped provider, verify that
/// `claudine sequence compose.md --<provider> --dry-run` runs cleanly
/// through the composition pipeline with a trivial markdown body.
///
/// Regression guard for the composition-path drift: if a provider's
/// `apply_entrypoint` / `apply_non_interactive_flags` / `prompt_delivery`
/// chain silently bails when the prompt arrives via the composition
/// body, this test fails.
#[cfg(unix)]
mod sequence_composition_dry_run_for_every_provider {
    use super::*;

    macro_rules! provider_tests {
        ($($test:ident => $provider:ident),* $(,)?) => {
            $(#[test] fn $test() { assert_sequence_dry_run_composes(Provider::$provider); })*

            /// A provider added to the compiled set must get its own test.
            #[test]
            fn every_row_has_a_listed_test() {
                assert_eq!([$(Provider::$provider),*].as_slice(), PROVIDERS_DISPLAY_ORDER.as_slice());
            }
        };
    }

    provider_tests! {
        claude => Claude,
        codex => Codex,
        gemini => Gemini,
        goose => Goose,
        kimi => KimiCode,
        opencode => OpenCode,
        qwen => QwenCode,
        kilo => Kilo,
        pi => Pi,
        antigravity => Antigravity,
    }

    fn assert_sequence_dry_run_composes(provider: Provider) {
        let provider_slug = provider.as_slug();
        let fixture = CliProcessFixture::named("wrap-sequence-composition");
        fixture.seed_user_config();

        write_executable(
            &fixture.bin_dir().join(provider_info(provider).binary),
            "#!/bin/sh\nexit 0\n",
        );

        let compose_file = fixture.cwd().join("compose.md");
        fs::write(
            &compose_file,
            "---\nsequence:\n  - step_one\n---\ncomposed body text\n",
        )
        .unwrap();

        let output = fixture
            .command_builder()
            // Only the staged stub may resolve: the subject is that every
            // provider composes, not that the host has one installed.
            .fake_only_path()
            .build()
            .env("OPENCODE_MODEL", "test-model")
            .args([
                "sequence",
                "compose.md",
                &format!("--{provider_slug}"),
                "--dry-run",
            ])
            .output()
            .unwrap();

        assert!(
            output.status.success(),
            "`claudine sequence compose.md --{provider_slug} --dry-run` failed: stderr={}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
}

/// Sequence preflight must audit lifecycle shell commands for every step
/// before the first provider starts. A built-in-blacklisted command cannot be
/// widened by YOLO and must fail before the step status or provider launch.
#[cfg(unix)]
#[test]
fn sequence_preflight_rejects_blacklisted_lifecycle_shell_before_launch() {
    let fixture = CliProcessFixture::named("wrap-sequence-composition");
    fixture.seed_user_config();

    let sentinel = fixture.cwd().join("provider-ran.flag");
    write_executable(
        &fixture.bin_dir().join("goose"),
        &format!("#!/bin/sh\n: > '{}'\nexit 0\n", sentinel.display()),
    );

    let compose_file = fixture.cwd().join("compose.md");
    fs::write(
        &compose_file,
        "---\nsequence:\n  - step_one\nsuccess:\n  - shell: { command: cargo metadata }\n---\nSEQUENCE_BODY\n",
    )
    .unwrap();

    let output = fixture
        .command()
        .args(["sequence", "compose.md", "--goose", "--yolo"])
        .output()
        .unwrap();

    let stderr = strip_ansi(&String::from_utf8_lossy(&output.stderr));
    assert!(!output.status.success(), "blacklisted command must fail");
    assert!(
        stderr.contains("cargo metadata") && stderr.contains("blacklisted"),
        "failure must identify the lifecycle command; stderr:\n{stderr}"
    );
    assert!(
        !stderr.contains("[1/1]"),
        "failure must occur during fleet-wide preflight; stderr:\n{stderr}"
    );
    assert!(!sentinel.exists(), "provider must not launch");
}

/// The sequence-level YOLO flag applies to lifecycle commands during the
/// fleet-wide preflight, allowing a non-blacklisted command without a TTY.
#[cfg(unix)]
#[test]
fn sequence_yolo_approves_lifecycle_shell_during_preflight() {
    let fixture = CliProcessFixture::named("wrap-sequence-composition");
    fixture.seed_user_config();

    write_executable(&fixture.bin_dir().join("goose"), "#!/bin/sh\nexit 0\n");
    let lifecycle_marker = fixture.cwd().join("lifecycle-ran.flag");
    let compose_file = fixture.cwd().join("compose.md");
    fs::write(
        &compose_file,
        format!(
            "---\nsequence:\n  - step_one\nsuccess:\n  - shell: touch '{}'\n---\nSEQUENCE_BODY\n",
            lifecycle_marker.display()
        ),
    )
    .unwrap();

    let output = fixture
        .command()
        .args(["sequence", "compose.md", "--goose", "--yolo"])
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "YOLO lifecycle run must succeed; stderr:\n{}",
        strip_ansi(&String::from_utf8_lossy(&output.stderr))
    );
    assert!(
        lifecycle_marker.exists(),
        "approved lifecycle command must execute"
    );
}

// ---------------------------------------------------------------------------
// Sequence dry run: the passive check reports every task and previews each
// composed body
// ---------------------------------------------------------------------------

/// `claudine sequence --dry-run` over a multi-step sequence previews each
/// step's composed body on **stdout** in order, reports each task on
/// **stderr**, and never launches the provider (sentinel absent).
#[cfg(unix)]
#[test]
fn sequence_dry_run_concatenates_bodies_with_dividers() {
    let fixture = CliProcessFixture::named("wrap-sequence-composition");
    fixture.seed_user_config();

    let sentinel = fixture.cwd().join("provider-ran.flag");
    write_executable(
        &fixture.bin_dir().join("goose"),
        &format!("#!/bin/sh\ntouch '{}'\nexit 0\n", sentinel.display()),
    );

    // Every step composes the same document body, so the body marker
    // appears once per step on stdout.
    let compose_file = fixture.cwd().join("compose.md");
    fs::write(
        &compose_file,
        "---\nsequence:\n  - step_one\n  - step_two\n  - step_three\n---\nSEQUENCE_BODY_XYZZY\n",
    )
    .unwrap();

    let output = fixture
        .command()
        .args(["sequence", "compose.md", "--goose", "--dry-run"])
        .output()
        .unwrap();

    let stdout = strip_ansi(&String::from_utf8_lossy(&output.stdout));
    let stderr = strip_ansi(&String::from_utf8_lossy(&output.stderr));
    assert_eq!(output.status.code(), Some(0), "a clean sequence exits 0; stderr:\n{stderr}");
    assert_eq!(
        stdout.matches("SEQUENCE_BODY_XYZZY").count(),
        3,
        "stdout should contain all three composed bodies; stdout was:\n{stdout}"
    );
    for task in ["task step_one (sequence[0])", "task step_two (sequence[1])", "task step_three (sequence[2])"] {
        assert!(stderr.contains(task), "the report names `{task}`; stderr:\n{stderr}");
    }
    assert!(stderr.contains("Previewed 3 of 3 tasks."), "stderr:\n{stderr}");
    assert!(!stdout.contains("Dry run"), "the report never reaches stdout:\n{stdout}");
    assert!(!sentinel.exists(), "provider must not execute under sequence --dry-run");
}

/// `--quiet` and `--silent` have no effect on the sequence dry run: the
/// report and the composed bodies render regardless.
#[cfg(unix)]
#[test]
fn sequence_dry_run_quiet_and_silent_are_no_op() {
    for flag in ["--quiet", "--silent"] {
        let fixture = CliProcessFixture::named("wrap-sequence-composition");
        fixture.seed_user_config();

        write_executable(&fixture.bin_dir().join("goose"), "#!/bin/sh\nexit 0\n");

        let compose_file = fixture.cwd().join("compose.md");
        fs::write(
            &compose_file,
            "---\nsequence:\n  - step_one\n  - step_two\n---\nSEQUENCE_BODY_XYZZY\n",
        )
        .unwrap();

        let output = fixture
            .command()
            .args(["sequence", "compose.md", "--goose", "--dry-run", flag])
            .output()
            .unwrap();

        let stdout = strip_ansi(&String::from_utf8_lossy(&output.stdout));
        let stderr = strip_ansi(&String::from_utf8_lossy(&output.stderr));
        assert_eq!(output.status.code(), Some(0), "sequence dry-run {flag} should succeed; stderr:\n{stderr}");
        assert_eq!(
            stdout.matches("SEQUENCE_BODY_XYZZY").count(),
            2,
            "{flag} must not suppress the composed bodies; stdout was:\n{stdout}"
        );
        assert!(
            stderr.contains("Dry run of compose.md") && stderr.contains("Previewed 2 of 2 tasks."),
            "{flag} must not suppress the report; stderr was:\n{stderr}"
        );
    }
}

/// The `--json` report of `claudine sequence compose.md --dry-run` with only
/// `installed` providers on `PATH`, as `(exit code, finding codes)`. Asserts
/// that no provider ever launched.
#[cfg(unix)]
fn sequence_dry_run_codes(frontmatter: &str, installed: &[&str], args: &[&str]) -> (Option<i32>, Vec<String>) {
    let fixture = CliProcessFixture::named("wrap-sequence-composition");
    fixture.seed_user_config();

    // Any installed provider writes a sentinel; under --dry-run it must never
    // run, regardless of how the provider resolves.
    let sentinel = fixture.cwd().join("provider-ran.flag");
    for slug in installed {
        write_executable(
            &fixture.bin_dir().join(slug),
            &format!("#!/bin/sh\ntouch '{}'\nexit 0\n", sentinel.display()),
        );
    }

    fs::write(
        fixture.cwd().join("compose.md"),
        format!("---\nsequence:\n  - step_one\n  - step_two\n{frontmatter}---\nSEQ_BODY_MARKER\n"),
    )
    .unwrap();

    let output = fixture
        .command_builder()
        // Restrict PATH to the fake bin so only `installed` providers count.
        .fake_only_path()
        .build()
        .args(["sequence", "compose.md", "--dry-run", "--json"])
        .args(args)
        .output()
        .unwrap();

    assert!(!sentinel.exists(), "provider must not execute under sequence --dry-run");
    let envelope: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap_or_else(|error| {
        panic!("not a JSON envelope ({error}):\n{}", String::from_utf8_lossy(&output.stdout))
    });
    let codes = envelope["diagnostics"]
        .as_array()
        .unwrap()
        .iter()
        .map(|finding| finding["code"].as_str().unwrap().to_string())
        .collect();
    (output.status.code(), codes)
}

/// `text` with its whitespace runs collapsed, so a wrapped row matches.
fn flat(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn count(codes: &[String], code: &str) -> usize {
    codes.iter().filter(|candidate| *candidate == code).count()
}

/// A `$schema`-required property the frontmatter does not satisfy is
/// reported once, as the document's input; every task whose body reads it
/// waits for it (its preview is not checked) rather than repeating it, and
/// no provider launches.
#[cfg(unix)]
#[test]
fn sequence_dry_run_reports_a_missing_document_input_once() {
    let (code, codes) =
        sequence_dry_run_codes("$schema:\n  topic: 'string(required)'\n", &["goose"], &["--goose"]);
    assert_eq!(code, Some(2), "{codes:?}");
    assert_eq!(count(&codes, "composition.missing_properties"), 1, "{codes:?}");
    assert_eq!(count(&codes, "dm.schema.missing_required"), 0, "no second finding per task: {codes:?}");
    assert_eq!(count(&codes, "dm.check.not_checked"), 2, "each task's preview waits: {codes:?}");
}

/// No frontmatter `agent` and no explicit provider: with no TTY, every step
/// reports that it has no provider, which a non-interactive run fails on.
#[cfg(unix)]
#[test]
fn sequence_dry_run_no_agent_renders_state_per_step() {
    let (code, codes) = sequence_dry_run_codes("", &["goose"], &[]);
    assert_eq!(code, Some(2), "{codes:?}");
    assert_eq!(count(&codes, "composition.provider_unresolved"), 2, "one per step: {codes:?}");
}

/// A scalar invalid `agent` (`agent: not-real`) is reported per step as an
/// unknown provider.
#[cfg(unix)]
#[test]
fn sequence_dry_run_single_invalid_renders_state_per_step() {
    let (code, codes) = sequence_dry_run_codes("agent: not-real\n", &["claude"], &[]);
    assert_eq!(code, Some(2), "{codes:?}");
    assert_eq!(count(&codes, "composition.provider_unknown"), 2, "one per step: {codes:?}");
}

/// A single valid-but-not-installed `agent` (`agent: gemini`, only `claude`
/// installed) is reported per step as not installed.
#[cfg(unix)]
#[test]
fn sequence_dry_run_single_not_installed_renders_state_per_step() {
    let (code, codes) = sequence_dry_run_codes("agent: gemini\n", &["claude"], &[]);
    assert_eq!(code, Some(2), "{codes:?}");
    assert_eq!(count(&codes, "composition.provider_not_installed"), 2, "one per step: {codes:?}");
}

/// A frontmatter `agent` list resolving to zero installed providers — here an
/// all-invalid list, which is deterministic regardless of which providers the
/// host has installed — is reported per step as not installed, never as the
/// scalar unknown-provider finding.
#[cfg(unix)]
#[test]
fn sequence_dry_run_zero_installed_list_renders_state_per_step() {
    let (code, codes) = sequence_dry_run_codes("agent: [not-real, also-fake]\n", &["claude"], &[]);
    assert_eq!(code, Some(2), "{codes:?}");
    assert_eq!(count(&codes, "composition.provider_not_installed"), 2, "one per step: {codes:?}");
    assert_eq!(count(&codes, "composition.provider_unknown"), 0, "{codes:?}");
}

// ---------------------------------------------------------------------------
// Live (non-dry-run) sequence agent-resolution gate
//
// The dry run reports a prompting state as a failure for a non-interactive
// run; the real `sequence` command must honor that. In a no-TTY session every
// prompting state aborts with the same styled `AgentResolutionFailed`
// message — never auto-running a substitute provider through the legacy
// favorite/default resolver. These tests are the live counterpart to the
// `sequence_dry_run_*_renders_state_per_step` tests above.
// ---------------------------------------------------------------------------

/// Run `claudine sequence compose.md` (live: no `--dry-run`, no explicit
/// provider) over a two-step sequence whose frontmatter carries `agent_line`,
/// with only `installed` providers on PATH and **no TTY** (piped empty stdin).
///
/// Each installed provider records a launch by writing a sentinel via the
/// POSIX shell builtin `: > file` — claudine restricts the child PATH to the
/// fake bin, so an external `touch` would not resolve, but a redirect always
/// does. Returns `(exit_code, ansi_stripped_stderr, provider_ran)`.
#[cfg(unix)]
fn run_sequence_live_agent_state(agent_line: &str, installed: &[&str]) -> (i32, String, bool) {
    let fixture = CliProcessFixture::named("wrap-sequence-composition");
    fixture.seed_user_config();

    let sentinel = fixture.cwd().join("provider-ran.flag");
    for slug in installed {
        write_executable(
            &fixture.bin_dir().join(slug),
            &format!("#!/bin/sh\n: > '{}'\nexit 0\n", sentinel.display()),
        );
    }

    let compose_file = fixture.cwd().join("compose.md");
    fs::write(
        &compose_file,
        format!("---\nsequence:\n  - step_one\n  - step_two\n{agent_line}---\nSEQ_BODY_MARKER\n"),
    )
    .unwrap();

    let stdin_file = fixture.cwd().join("empty-stdin.txt");
    fs::write(&stdin_file, "").unwrap();

    let output = fixture
        .command_builder()
        // Restrict PATH to the fake bin so only `installed` providers count.
        .fake_only_path()
        .build()
        .pipe_stdin(&stdin_file)
        .unwrap()
        .args(["sequence", "compose.md"])
        .output()
        .unwrap();

    (
        output.status.code().unwrap_or(-1),
        strip_ansi(&String::from_utf8_lossy(&output.stderr)),
        sentinel.exists(),
    )
}

/// No frontmatter `agent` and no explicit provider in a no-TTY session: the
/// live run aborts with the no-agent breakdown instead of auto-picking the
/// favorite/default and launching a provider.
#[cfg(unix)]
#[test]
fn sequence_live_no_agent_aborts_without_launching_provider() {
    let (code, stderr, ran) = run_sequence_live_agent_state("", &["claude"]);
    assert!(!ran, "no provider may launch for a no-agent live sequence");
    assert_eq!(code, 1, "no-agent live sequence must abort; stderr:\n{stderr}");
    assert!(
        stderr.contains("agent resolution failed"),
        "abort must surface the structured agent-resolution error; stderr:\n{stderr}"
    );
    assert!(
        flat(&stderr).contains("no provider for this task; a non-interactive run fails here"),
        "abort body must be the same no-agent finding the dry run shows; stderr:\n{stderr}"
    );
}

/// A shell-only sequence launches no provider, so it needs none: with no
/// `agent` in a no-TTY session it runs its shell step instead of aborting.
/// `--yolo` approves the shell at pre-flight.
#[cfg(unix)]
#[test]
fn sequence_live_shell_only_no_agent_runs_headless() {
    let fixture = CliProcessFixture::named("wrap-sequence-composition");
    fixture.seed_user_config();
    write_executable(&fixture.bin_dir().join("claude"), "#!/bin/sh\nexit 0\n");

    let sentinel = fixture.cwd().join("shell-ran.flag");
    fs::write(
        fixture.cwd().join("compose.md"),
        format!(
            "---\nsequence:\n  - name: stage\n    shell: \": > '{}'\"\n---\nSEQ_BODY_MARKER\n",
            sentinel.display()
        ),
    )
    .unwrap();
    let stdin_file = fixture.cwd().join("empty-stdin.txt");
    fs::write(&stdin_file, "").unwrap();

    let output = fixture
        .command()
        .pipe_stdin(&stdin_file)
        .unwrap()
        .args(["sequence", "compose.md", "--yolo"])
        .output()
        .unwrap();

    let stderr = strip_ansi(&String::from_utf8_lossy(&output.stderr));
    assert!(sentinel.exists(), "the shell step did not run; stderr:\n{stderr}");
    assert_eq!(output.status.code(), Some(0), "must run; stderr:\n{stderr}");
    assert!(!stderr.contains("agent resolution failed"), "stderr:\n{stderr}");
}

/// A scalar invalid `agent` (`agent: not-real`) aborts live in no-TTY mode
/// with the imperative `Invalid Agent:` message — it must NOT fall back to the
/// only installed provider.
#[cfg(unix)]
#[test]
fn sequence_live_single_invalid_aborts_without_launching_provider() {
    let (code, stderr, ran) = run_sequence_live_agent_state("agent: not-real\n", &["claude"]);
    assert!(!ran, "no provider may launch for an invalid-agent live sequence");
    assert_eq!(code, 1, "invalid-agent live sequence must abort; stderr:\n{stderr}");
    assert!(
        stderr.contains("agent resolution failed"),
        "abort must surface the structured agent-resolution error; stderr:\n{stderr}"
    );
    assert!(
        flat(&stderr).contains("`not-real` (compose.md agent) is not a known provider"),
        "abort body must name the invalid hint; stderr:\n{stderr}"
    );
}

/// A single valid-but-not-installed `agent` (`agent: gemini`, only `claude`
/// installed) aborts live in no-TTY mode rather than substituting `claude`.
#[cfg(unix)]
#[test]
fn sequence_live_single_not_installed_aborts_without_launching_provider() {
    let (code, stderr, ran) = run_sequence_live_agent_state("agent: gemini\n", &["claude"]);
    assert!(!ran, "no provider may launch for a not-installed-agent live sequence");
    assert_eq!(
        code, 1,
        "not-installed-agent live sequence must abort; stderr:\n{stderr}"
    );
    assert!(
        stderr.contains("agent resolution failed"),
        "abort must surface the structured agent-resolution error; stderr:\n{stderr}"
    );
    assert!(
        flat(&stderr).contains("the Gemini provider (compose.md agent) is not installed"),
        "abort body must be the not-installed finding; stderr:\n{stderr}"
    );
}

/// A frontmatter `agent` list resolving to zero installed providers
/// (`agent: [not-real, also-fake]`) aborts live in no-TTY mode.
#[cfg(unix)]
#[test]
fn sequence_live_zero_installed_list_aborts_without_launching_provider() {
    let (code, stderr, ran) =
        run_sequence_live_agent_state("agent: [not-real, also-fake]\n", &["claude"]);
    assert!(!ran, "no provider may launch for a zero-installed-list live sequence");
    assert_eq!(
        code, 1,
        "zero-installed-list live sequence must abort; stderr:\n{stderr}"
    );
    assert!(
        stderr.contains("agent resolution failed"),
        "abort must surface the structured agent-resolution error; stderr:\n{stderr}"
    );
    assert!(
        flat(&stderr).contains("no provider listed in compose.md agent is installed"),
        "abort body must be the zero-installed-list finding; stderr:\n{stderr}"
    );
}

/// Counterpart guard: an auto-selectable state (`agent: claude`, installed)
/// must NOT abort — the gate only fires for prompting states. The provider
/// launches and the run succeeds.
#[cfg(unix)]
#[test]
fn sequence_live_auto_selectable_launches_provider() {
    let (code, stderr, ran) = run_sequence_live_agent_state("agent: claude\n", &["claude"]);
    assert!(
        ran,
        "an auto-selectable agent must launch the provider; stderr:\n{stderr}"
    );
    assert_eq!(
        code, 0,
        "auto-selectable live sequence must succeed; stderr:\n{stderr}"
    );
    assert!(
        !stderr.contains("agent resolution failed"),
        "auto-selectable state must not trip the resolution gate; stderr:\n{stderr}"
    );
}

/// `--silent` suppresses status chatter but must not suppress the
/// agent-resolution abort: a no-TTY invalid-agent live sequence still aborts
/// with the styled message (mirrors the direct-compose `--silent` guarantee).
#[cfg(unix)]
#[test]
fn sequence_live_silent_does_not_suppress_agent_resolution_abort() {
    let fixture = CliProcessFixture::named("wrap-sequence-composition");
    fixture.seed_user_config();

    let sentinel = fixture.cwd().join("provider-ran.flag");
    write_executable(
        &fixture.bin_dir().join("claude"),
        &format!("#!/bin/sh\n: > '{}'\nexit 0\n", sentinel.display()),
    );

    let compose_file = fixture.cwd().join("compose.md");
    fs::write(
        &compose_file,
        "---\nsequence:\n  - step_one\nagent: not-real\n---\nSEQ_BODY_MARKER\n",
    )
    .unwrap();

    let stdin_file = fixture.cwd().join("empty-stdin.txt");
    fs::write(&stdin_file, "").unwrap();

    let output = fixture
        .command_builder()
        // Restrict PATH to the fake bin so only `installed` providers count.
        .fake_only_path()
        .build()
        .pipe_stdin(&stdin_file)
        .unwrap()
        .args(["sequence", "compose.md", "--silent"])
        .output()
        .unwrap();

    let stderr = strip_ansi(&String::from_utf8_lossy(&output.stderr));
    assert!(!sentinel.exists(), "no provider may launch under the abort");
    assert_eq!(output.status.code(), Some(1), "must abort; stderr:\n{stderr}");
    assert!(
        stderr.contains("agent resolution failed") && flat(&stderr).contains("is not a known provider"),
        "--silent must not suppress the agent-resolution abort message; stderr:\n{stderr}"
    );
}

// ---------------------------------------------------------------------------
// Phase 6 — cross-cutting hardening (stdout/stderr discipline, error
// surfaces, quiet/silent matrix)
// ---------------------------------------------------------------------------
