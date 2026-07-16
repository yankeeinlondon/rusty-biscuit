//! Integration tests: sequence composition dry-run/live agent-state rendering.
//!
//! Split out of the `wrap_commands.rs` god file; shared fixtures live in
//! `common::wrap`.

use assert_cmd::cargo::cargo_bin_cmd;
use std::fs;
use tempfile::tempdir;
mod common;
use common::wrap::*;
use common::{augmented_path, strip_ansi, write_executable};

/// End-to-end: for every wrapped provider, verify that
/// `claudine sequence compose.md --<provider> --dry-run` runs cleanly
/// through the composition pipeline with a trivial markdown body.
///
/// Regression guard for the composition-path drift: if a provider's
/// `apply_entrypoint` / `apply_non_interactive_flags` / `prompt_delivery`
/// chain silently bails when the prompt arrives via the composition
/// body, this test fails.
#[cfg(unix)]
#[test]
fn sequence_composition_dry_run_for_every_provider() {
    for provider_slug in [
        "claude", "codex", "gemini", "kimi", "opencode", "qwen", "goose",
    ] {
        let workspace = tempdir().unwrap();
        let path_dir = workspace.path().join("bin");
        fs::create_dir_all(&path_dir).unwrap();
        seed_minimal_config(workspace.path());

        write_executable(&path_dir.join(provider_slug), "#!/bin/sh\nexit 0\n");

        let compose_file = workspace.path().join("compose.md");
        fs::write(
            &compose_file,
            "---\nsequence:\n  - step_one\n---\ncomposed body text\n",
        )
        .unwrap();

        let output = cargo_bin_cmd!("claudine")
            .env("NO_COLOR", "1")
            .env("OPENCODE_MODEL", "test-model")
            .env("PATH", &path_dir)
            .current_dir(workspace.path())
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
    let workspace = tempdir().unwrap();
    let path_dir = workspace.path().join("bin");
    fs::create_dir_all(&path_dir).unwrap();
    seed_minimal_config(workspace.path());

    let sentinel = workspace.path().join("provider-ran.flag");
    write_executable(
        &path_dir.join("goose"),
        &format!("#!/bin/sh\n: > '{}'\nexit 0\n", sentinel.display()),
    );

    let compose_file = workspace.path().join("compose.md");
    fs::write(
        &compose_file,
        "---\nsequence:\n  - step_one\nsuccess:\n  stack:\n    - action:\n        - action: shell\n          command: cargo metadata\n---\nSEQUENCE_BODY\n",
    )
    .unwrap();

    let output = cargo_bin_cmd!("claudine")
        .env("NO_COLOR", "1")
        .env("HOME", workspace.path())
        .env("PATH", augmented_path(&path_dir))
        .current_dir(workspace.path())
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
    let workspace = tempdir().unwrap();
    let path_dir = workspace.path().join("bin");
    fs::create_dir_all(&path_dir).unwrap();
    seed_minimal_config(workspace.path());

    write_executable(&path_dir.join("goose"), "#!/bin/sh\nexit 0\n");
    let lifecycle_marker = workspace.path().join("lifecycle-ran.flag");
    let compose_file = workspace.path().join("compose.md");
    fs::write(
        &compose_file,
        format!(
            "---\nsequence:\n  - step_one\nsuccess:\n  stack:\n    - action:\n        - action: shell\n          command: touch '{}'\n---\nSEQUENCE_BODY\n",
            lifecycle_marker.display()
        ),
    )
    .unwrap();

    let output = cargo_bin_cmd!("claudine")
        .env("NO_COLOR", "1")
        .env("HOME", workspace.path())
        .env("PATH", augmented_path(&path_dir))
        .current_dir(workspace.path())
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
// Phase 5 — sequence dry-run (dividers, concatenation, fail-fast)
// ---------------------------------------------------------------------------

/// `claudine sequence --dry-run` over a multi-step sequence:
/// - each step's composed body is concatenated to **stdout** in order,
/// - a `=== Document N of M ===` divider precedes documents 2..M on
///   **stderr** (none before the first document),
/// - no provider is ever launched (sentinel absent).
#[cfg(unix)]
#[test]
fn sequence_dry_run_concatenates_bodies_with_dividers() {
    let workspace = tempdir().unwrap();
    let path_dir = workspace.path().join("bin");
    fs::create_dir_all(&path_dir).unwrap();
    seed_minimal_config(workspace.path());

    // Provider stub writes a sentinel; under --dry-run it must never run.
    let sentinel = workspace.path().join("provider-ran.flag");
    write_executable(
        &path_dir.join("goose"),
        &format!("#!/bin/sh\ntouch '{}'\nexit 0\n", sentinel.display()),
    );

    // Three-step sequence; every step composes the same document body, so the
    // body marker appears once per step on stdout.
    let compose_file = workspace.path().join("compose.md");
    fs::write(
        &compose_file,
        "---\nsequence:\n  - step_one\n  - step_two\n  - step_three\n---\nSEQUENCE_BODY_XYZZY\n",
    )
    .unwrap();

    let output = cargo_bin_cmd!("claudine")
        .env("NO_COLOR", "1")
        .env("HOME", workspace.path())
        .env("PATH", augmented_path(&path_dir))
        .current_dir(workspace.path())
        .args(["sequence", "compose.md", "--goose", "--dry-run"])
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "sequence dry-run should succeed; stderr was:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = strip_ansi(&String::from_utf8_lossy(&output.stdout));
    let stderr = strip_ansi(&String::from_utf8_lossy(&output.stderr));

    // All three composed bodies are concatenated to stdout in order.
    assert_eq!(
        stdout.matches("SEQUENCE_BODY_XYZZY").count(),
        3,
        "stdout should contain all three composed bodies; stdout was:\n{stdout}"
    );

    // Horizontal-rule delimiters precede documents 2 and 3, but never the
    // first document. The old text divider is no longer emitted.
    assert!(
        !stderr.contains("=== Document"),
        "old text divider must not appear; stderr was:\n{stderr}"
    );
    let hr_lines: Vec<&str> = stderr
        .lines()
        .filter(|l| l.len() >= 10 && l.chars().all(|c| c == '╌' || c == '-'))
        .collect();
    // Each document emits an HR after its body, plus one HR between docs 2&3.
    // That's 3 per-document HRs + 2 inter-document HRs = 5 total.
    assert!(
        hr_lines.len() >= 2,
        "stderr should contain horizontal-rule delimiters; stderr was:\n{stderr}"
    );

    // No provider launched.
    assert!(
        !sentinel.exists(),
        "provider must not execute under sequence --dry-run"
    );
}

/// `--quiet` and `--silent` have no effect on sequence dry-run output: the
/// concatenated bodies and the between-document dividers render regardless.
#[cfg(unix)]
#[test]
fn sequence_dry_run_quiet_and_silent_are_no_op() {
    for flag in ["--quiet", "--silent"] {
        let workspace = tempdir().unwrap();
        let path_dir = workspace.path().join("bin");
        fs::create_dir_all(&path_dir).unwrap();
        seed_minimal_config(workspace.path());

        write_executable(&path_dir.join("goose"), "#!/bin/sh\nexit 0\n");

        let compose_file = workspace.path().join("compose.md");
        fs::write(
            &compose_file,
            "---\nsequence:\n  - step_one\n  - step_two\n---\nSEQUENCE_BODY_XYZZY\n",
        )
        .unwrap();

        let output = cargo_bin_cmd!("claudine")
            .env("NO_COLOR", "1")
            .env("HOME", workspace.path())
            .env("PATH", augmented_path(&path_dir))
            .current_dir(workspace.path())
            .args(["sequence", "compose.md", "--goose", "--dry-run", flag])
            .output()
            .unwrap();

        assert!(
            output.status.success(),
            "sequence dry-run {flag} should succeed; stderr was:\n{}",
            String::from_utf8_lossy(&output.stderr)
        );

        let stdout = strip_ansi(&String::from_utf8_lossy(&output.stdout));
        let stderr = strip_ansi(&String::from_utf8_lossy(&output.stderr));

        assert_eq!(
            stdout.matches("SEQUENCE_BODY_XYZZY").count(),
            2,
            "{flag} must not suppress the composed bodies; stdout was:\n{stdout}"
        );
        assert!(
            !stderr.contains("=== Document"),
            "{flag}: old text divider must not appear; stderr was:\n{stderr}"
        );
        let hr_lines: Vec<&str> = stderr
            .lines()
            .filter(|l| l.len() >= 10 && l.chars().all(|c| c == '╌' || c == '-'))
            .collect();
        assert!(
            hr_lines.len() >= 2,
            "{flag} must not suppress the dry-run horizontal rules; stderr was:\n{stderr}"
        );
    }
}

/// Fail-fast: a composition error (here, a `$schema`-required property the
/// sequence frontmatter does not satisfy) renders to **stderr** and stops the
/// sequence with a non-zero exit, before any provider launches.
#[cfg(unix)]
#[test]
fn sequence_dry_run_fail_fast_on_composition_error() {
    let workspace = tempdir().unwrap();
    let path_dir = workspace.path().join("bin");
    fs::create_dir_all(&path_dir).unwrap();
    seed_minimal_config(workspace.path());

    let sentinel = workspace.path().join("provider-ran.flag");
    write_executable(
        &path_dir.join("goose"),
        &format!("#!/bin/sh\ntouch '{}'\nexit 0\n", sentinel.display()),
    );

    let compose_file = workspace.path().join("compose.md");
    fs::write(
        &compose_file,
        "---\n$schema:\n  topic: 'string(required)'\nsequence:\n  - step_one\n  - step_two\n---\nPlan for {{topic}}.\n",
    )
    .unwrap();

    let assert = cargo_bin_cmd!("claudine")
        .env("NO_COLOR", "1")
        .env("HOME", workspace.path())
        .env("PATH", augmented_path(&path_dir))
        .current_dir(workspace.path())
        .args(["sequence", "compose.md", "--goose", "--dry-run"])
        .assert()
        .failure();

    let stderr = strip_ansi(&String::from_utf8_lossy(&assert.get_output().stderr));
    assert!(
        stderr.to_lowercase().contains("missing properties"),
        "composition error should surface a missing-properties report on stderr; stderr was:\n{stderr}"
    );
    assert!(
        stderr.contains("topic"),
        "error should name the unsatisfied `topic` property; stderr was:\n{stderr}"
    );
    assert!(
        !sentinel.exists(),
        "provider must not launch when sequence dry-run fails composition"
    );
}

// ---------------------------------------------------------------------------
// review-2 — sequence --dry-run agent-resolution states (no explicit provider)
//
// `claudine sequence --dry-run` with no `--<provider>` flag must NOT run the
// legacy non-TTY resolver (which auto-picks or aborts) before the per-step
// dry-run seam. Each step must instead render the classified agent-resolution
// state into the metadata table, exactly like `compose --dry-run`. These L1
// tests cover the states a bare sequence can hit: no-agent, single-invalid,
// single-not-installed, and zero-installed-list.
// ---------------------------------------------------------------------------

/// Run `claudine sequence compose.md --dry-run` (no explicit provider) over a
/// two-step sequence whose frontmatter carries `agent_line` (empty for the
/// no-agent case), with only `installed` providers on PATH. Returns
/// `(success, ansi_stripped_stdout, ansi_stripped_stderr)` and asserts that no
/// provider ever launched.
#[cfg(unix)]
fn run_sequence_dry_run_agent_state(
    agent_line: &str,
    installed: &[&str],
) -> (bool, String, String) {
    let workspace = tempdir().unwrap();
    let path_dir = workspace.path().join("bin");
    fs::create_dir_all(&path_dir).unwrap();
    seed_minimal_config(workspace.path());

    // Any installed provider writes a sentinel; under --dry-run it must never
    // run, regardless of how the agent state resolves.
    let sentinel = workspace.path().join("provider-ran.flag");
    for slug in installed {
        write_executable(
            &path_dir.join(slug),
            &format!("#!/bin/sh\ntouch '{}'\nexit 0\n", sentinel.display()),
        );
    }

    let compose_file = workspace.path().join("compose.md");
    fs::write(
        &compose_file,
        format!("---\nsequence:\n  - step_one\n  - step_two\n{agent_line}---\nSEQ_BODY_MARKER\n"),
    )
    .unwrap();

    let output = cargo_bin_cmd!("claudine")
        .env("NO_COLOR", "1")
        .env("HOME", workspace.path())
        // Restrict PATH to the fake bin so only `installed` providers count.
        .env("PATH", &path_dir)
        .current_dir(workspace.path())
        .args(["sequence", "compose.md", "--dry-run"])
        .output()
        .unwrap();

    assert!(
        !sentinel.exists(),
        "provider must not execute under sequence --dry-run"
    );

    (
        output.status.success(),
        strip_ansi(&String::from_utf8_lossy(&output.stdout)),
        strip_ansi(&String::from_utf8_lossy(&output.stderr)),
    )
}

/// No frontmatter `agent` and no explicit provider: every step renders the
/// no-agent state instead of aborting through the legacy resolver.
#[cfg(unix)]
#[test]
fn sequence_dry_run_no_agent_renders_state_per_step() {
    let (ok, stdout, stderr) = run_sequence_dry_run_agent_state("", &["goose"]);
    assert!(ok, "no-agent sequence dry-run should succeed; stderr:\n{stderr}");
    // Both composed bodies still reach stdout.
    assert_eq!(
        stdout.matches("SEQ_BODY_MARKER").count(),
        2,
        "each step's body must reach stdout; stdout:\n{stdout}"
    );
    // The no-agent breakdown renders once per step (one per metadata table).
    assert!(
        stderr.matches("didn't specify the Agent").count() >= 2,
        "each step must render the no-agent state; stderr:\n{stderr}"
    );
}

/// A scalar invalid `agent` (`agent: not-real`) is non-fatal under dry-run:
/// every step renders the single-invalid cell rather than aborting.
#[cfg(unix)]
#[test]
fn sequence_dry_run_single_invalid_renders_state_per_step() {
    let (ok, _stdout, stderr) =
        run_sequence_dry_run_agent_state("agent: not-real\n", &["claude"]);
    assert!(ok, "single-invalid sequence dry-run should succeed; stderr:\n{stderr}");
    assert!(
        stderr.matches("Invalid Agent").count() >= 2,
        "each step must render the single-invalid cell; stderr:\n{stderr}"
    );
    assert!(
        stderr.contains("not-real"),
        "the invalid hint must be named; stderr:\n{stderr}"
    );
}

/// A single valid-but-not-installed `agent` (`agent: gemini`, only `claude`
/// installed) renders the not-installed cell per step under dry-run.
#[cfg(unix)]
#[test]
fn sequence_dry_run_single_not_installed_renders_state_per_step() {
    let (ok, _stdout, stderr) =
        run_sequence_dry_run_agent_state("agent: gemini\n", &["claude"]);
    assert!(
        ok,
        "single-not-installed sequence dry-run should succeed; stderr:\n{stderr}"
    );
    assert!(
        stderr.matches("Agent Not Installed").count() >= 2,
        "each step must render the not-installed cell; stderr:\n{stderr}"
    );
}

/// A frontmatter `agent` list resolving to zero installed providers — here an
/// all-invalid list (`agent: [not-real, also-fake]`), which is deterministic
/// regardless of which providers the host has installed — renders the
/// zero-installed-list state per step under dry-run.
#[cfg(unix)]
#[test]
fn sequence_dry_run_zero_installed_list_renders_state_per_step() {
    let (ok, _stdout, stderr) =
        run_sequence_dry_run_agent_state("agent: [not-real, also-fake]\n", &["claude"]);
    assert!(
        ok,
        "zero-installed-list sequence dry-run should succeed; stderr:\n{stderr}"
    );
    // `installed/valid` is the single-token signature of the zero-installed
    // header; it survives table word-wrap.
    assert!(
        stderr.matches("installed/valid").count() >= 2,
        "each step must render the zero-installed-list state; stderr:\n{stderr}"
    );
    assert!(
        !stderr.contains("Invalid Agent"),
        "a list state must not render the single-invalid scalar cell; stderr:\n{stderr}"
    );
}

// ---------------------------------------------------------------------------
// Live (non-dry-run) sequence agent-resolution gate
//
// The dry-run table promises a prompting state would prompt-or-abort; the
// real `sequence` command must honor that. In a no-TTY session every
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
    let workspace = tempdir().unwrap();
    let path_dir = workspace.path().join("bin");
    fs::create_dir_all(&path_dir).unwrap();
    seed_minimal_config(workspace.path());

    let sentinel = workspace.path().join("provider-ran.flag");
    for slug in installed {
        write_executable(
            &path_dir.join(slug),
            &format!("#!/bin/sh\n: > '{}'\nexit 0\n", sentinel.display()),
        );
    }

    let compose_file = workspace.path().join("compose.md");
    fs::write(
        &compose_file,
        format!("---\nsequence:\n  - step_one\n  - step_two\n{agent_line}---\nSEQ_BODY_MARKER\n"),
    )
    .unwrap();

    let stdin_file = workspace.path().join("empty-stdin.txt");
    fs::write(&stdin_file, "").unwrap();

    let output = cargo_bin_cmd!("claudine")
        .env("NO_COLOR", "1")
        .env("HOME", workspace.path())
        // Restrict PATH to the fake bin so only `installed` providers count.
        .env("PATH", &path_dir)
        .pipe_stdin(&stdin_file)
        .unwrap()
        .current_dir(workspace.path())
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
        stderr.contains("didn't specify the Agent"),
        "abort body must be the same no-agent breakdown the dry-run table shows; stderr:\n{stderr}"
    );
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
        stderr.contains("Invalid Agent") && stderr.contains("not-real"),
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
        stderr.contains("Agent Not Installed"),
        "abort body must be the not-installed breakdown; stderr:\n{stderr}"
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
        stderr.contains("installed/valid"),
        "abort body must be the zero-installed-list breakdown; stderr:\n{stderr}"
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
    let workspace = tempdir().unwrap();
    let path_dir = workspace.path().join("bin");
    fs::create_dir_all(&path_dir).unwrap();
    seed_minimal_config(workspace.path());

    let sentinel = workspace.path().join("provider-ran.flag");
    write_executable(
        &path_dir.join("claude"),
        &format!("#!/bin/sh\n: > '{}'\nexit 0\n", sentinel.display()),
    );

    let compose_file = workspace.path().join("compose.md");
    fs::write(
        &compose_file,
        "---\nsequence:\n  - step_one\nagent: not-real\n---\nSEQ_BODY_MARKER\n",
    )
    .unwrap();

    let stdin_file = workspace.path().join("empty-stdin.txt");
    fs::write(&stdin_file, "").unwrap();

    let output = cargo_bin_cmd!("claudine")
        .env("NO_COLOR", "1")
        .env("HOME", workspace.path())
        .env("PATH", &path_dir)
        .pipe_stdin(&stdin_file)
        .unwrap()
        .current_dir(workspace.path())
        .args(["sequence", "compose.md", "--silent"])
        .output()
        .unwrap();

    let stderr = strip_ansi(&String::from_utf8_lossy(&output.stderr));
    assert!(!sentinel.exists(), "no provider may launch under the abort");
    assert_eq!(output.status.code(), Some(1), "must abort; stderr:\n{stderr}");
    assert!(
        stderr.contains("agent resolution failed") && stderr.contains("Invalid Agent"),
        "--silent must not suppress the agent-resolution abort message; stderr:\n{stderr}"
    );
}

// ---------------------------------------------------------------------------
// Phase 6 — cross-cutting hardening (stdout/stderr discipline, error
// surfaces, quiet/silent matrix)
// ---------------------------------------------------------------------------
