//! `real_` tier: the primary behavioral regression guard for the OpenCode YOLO
//! subagent hang (`fixes/2026-06-26-opencode-yolo/spec.md`, acceptance #1).
//!
//! Every other test for this fix is Level-1 assembly coverage: it asserts the
//! *shape* of the argv / `OPENCODE_CONFIG_CONTENT` Claudine builds (see
//! `cli/src/commands/wrap/composition/tests.rs` and
//! `cli/src/commands/wrap/profile/tests/apply_yolo.rs`). None of them run
//! OpenCode, so none can prove the load-bearing assumption: that OpenCode
//! actually applies the injected `permission` block (`"*": "allow"` plus the
//! explicit `external_directory` / `doom_loop` guards) to a **child / Task
//! session**, not just the parent. That asymmetry was the whole bug — the
//! parent's `--dangerously-skip-permissions` grant never reached the subagent,
//! so a subagent tool call against a path outside the worktree fell back to the
//! `"ask"` default and hung forever in a non-interactive `opencode run`.
//!
//! This test runs the real `claudine compose --opencode --yolo` against the
//! installed, authenticated OpenCode binary with a prompt that forces a
//! subagent to write then read a path **outside** the worktree (under the
//! system temp dir). It asserts the run *completes* instead of stalling at the
//! `external_directory` gate.
//!
//! ## Why it is gated, and how to run it
//!
//! Spawning a real provider session costs tokens and needs credentials, so the
//! test is **opt-in** and matches the package's existing `real_` convention
//! (`contract/tests/real_provider.rs`): it skips unless `CLAUDINE_CONTRACT_REAL=1`
//! is set, and skips if the `opencode` binary is not on `PATH`. The model is
//! taken from `CLAUDINE_REAL_OPENCODE_MODEL` (falling back to `OPENCODE_MODEL`);
//! if neither is set the test skips, because OpenCode has no default
//! non-interactive model.
//!
//! Run it with:
//!
//! ```sh
//! CLAUDINE_CONTRACT_REAL=1 \
//!   CLAUDINE_REAL_OPENCODE_MODEL=opencode/glm-5 \
//!   cargo test -p claudine-cli --test real_opencode_yolo_subagent -- --nocapture
//! ```
//!
//! or via the package recipe `just test-real` (which sets `CLAUDINE_CONTRACT_REAL=1`
//! and runs the `real_` filterset across the package's real-tier binaries).

#![cfg(unix)]

use std::fs;
use std::path::{Path, PathBuf};
use std::time::Duration;
use tempfile::tempdir;

mod common;
use common::init_git_repo;

/// The opt-in env gate, mirroring `contract/tests/real_provider.rs::real_enabled`.
fn real_enabled() -> bool {
    if std::env::var("CLAUDINE_CONTRACT_REAL").as_deref() != Ok("1") {
        eprintln!(
            "skipping real_opencode_yolo_subagent (set CLAUDINE_CONTRACT_REAL=1 to run)"
        );
        return false;
    }
    true
}

/// Mirrors the binary-presence check in `contract/tests/real_provider.rs`.
fn binary_on_path(binary: &str) -> bool {
    let Some(path) = std::env::var_os("PATH") else {
        return false;
    };
    std::env::split_paths(&path).any(|dir| dir.join(binary).is_file())
}

/// The model to drive the run with. OpenCode has no default non-interactive
/// model, so a run with no model selected fails preflight — that is a Claudine
/// behavior already covered at L1, not what this test exercises. Skip cleanly
/// when no model is available.
fn opencode_model() -> Option<String> {
    std::env::var("CLAUDINE_REAL_OPENCODE_MODEL")
        .ok()
        .or_else(|| std::env::var("OPENCODE_MODEL").ok())
        .filter(|m| !m.trim().is_empty())
}

/// Build a minimal claudine monorepo-shaped git worktree so claudine resolves a
/// repo root and launches OpenCode from it. The external path the subagent
/// touches lives under the **system temp dir**, deliberately outside this tree,
/// so OpenCode's `external_directory` guard is the gate under test. Returns the
/// repo root (where the prompt file lives).
fn seed_repo(workspace: &Path) -> Option<PathBuf> {
    let repo_root = workspace.join("repo");
    let pkg_src = repo_root.join("claudine/lib/src");
    fs::create_dir_all(&pkg_src).unwrap();
    fs::write(
        repo_root.join("Cargo.toml"),
        "[workspace]\nresolver = \"2\"\nmembers = [\"claudine/lib\"]\n",
    )
    .unwrap();
    fs::write(
        repo_root.join("claudine/lib/Cargo.toml"),
        "[package]\nname = \"claudine\"\nversion = \"0.1.0\"\nedition = \"2024\"\n",
    )
    .unwrap();
    fs::write(pkg_src.join("lib.rs"), "").unwrap();

    if !init_git_repo(&repo_root) {
        eprintln!("skipping real_opencode_yolo_subagent (git init unavailable)");
        return None;
    }
    Some(repo_root)
}

/// Acceptance criterion #1: a non-interactive OpenCode YOLO compose where a
/// **subagent** touches a path outside the working directory completes instead
/// of hanging at the `external_directory` permission gate.
///
/// The bounded `--step-timeout` / `--timeout` are the fail-fast safety net: if
/// the regression returns, the child stalls silently after
/// `permission_evaluated external_directory:...`, both stream and raw-byte
/// clocks go stale, and the step-timeout terminates the run with a non-zero
/// exit — the suite fails fast rather than blocking. A passing run completes
/// well within the budget because the injected permission block auto-allows the
/// child session's external access.
#[test]
#[serial_test::serial]
fn real_opencode_yolo_subagent_external_dir_completes() {
    if !real_enabled() {
        return;
    }
    if !binary_on_path("opencode") {
        eprintln!("skipping real_opencode_yolo_subagent (binary `opencode` not on PATH)");
        return;
    }
    let Some(model) = opencode_model() else {
        eprintln!(
            "skipping real_opencode_yolo_subagent \
             (set CLAUDINE_REAL_OPENCODE_MODEL or OPENCODE_MODEL to a non-interactive model)"
        );
        return;
    };

    let workspace = tempdir().unwrap();
    let Some(repo_root) = seed_repo(workspace.path()) else {
        return;
    };

    // The external scratch path is under the system temp dir (portable across
    // macOS / Linux / Windows), deliberately *outside* the worktree, so the
    // child session's read trips the `external_directory` guard — the exact
    // path that previously hung.
    let external_dir = std::env::temp_dir().join(format!(
        "claudine-real-yolo-{}-{}",
        std::process::id(),
        chrono::Utc::now().timestamp_nanos_opt().unwrap_or_default()
    ));
    fs::create_dir_all(&external_dir).unwrap();
    let external_file = external_dir.join("subagent-scratch.txt");
    let external_file_display = external_file.display().to_string();

    // The prompt forces the subagent (Task tool) to write then read back a path
    // outside the worktree, reproducing the spec's `just lint > /tmp/jl.log`
    // then read-`/tmp` sequence in a controlled way.
    let prompt_body = format!(
        "You are orchestrating a tiny task. Use your Task tool to dispatch ONE \
         subagent. Instruct that subagent to do exactly two filesystem steps and \
         nothing else:\n\
         1. Write the text `claudine-yolo-ok` to the absolute path \
         `{external_file_display}` (this path is OUTSIDE the current working \
         directory, on purpose).\n\
         2. Read that same file back and report its contents.\n\
         When the subagent returns, reply with the single word DONE and stop. Do \
         not ask for permission or confirmation; just perform the steps.\n",
    );
    let prompt_file = repo_root.join("prompts/subagent-external.md");
    fs::create_dir_all(prompt_file.parent().unwrap()).unwrap();
    fs::write(
        &prompt_file,
        format!("---\ntitle: real opencode yolo subagent\n---\n{prompt_body}"),
    )
    .unwrap();

    // Bounded so a regressed build fails fast instead of blocking the suite:
    // - `--step-timeout 90s`: a silently-blocked child emits no bytes, so both
    //   clocks go stale and this terminates the run.
    // - `--timeout 4m`: hard wall-clock backstop for the whole session.
    let assert = assert_cmd::Command::cargo_bin("claudine").unwrap()
        .current_dir(&repo_root)
        .env("NO_COLOR", "1")
        .env("OPENCODE_MODEL", &model)
        .args([
            "compose",
            "--opencode",
            "--yolo",
            "--no-interactive",
            "--step-timeout",
            "90s",
            "--timeout",
            "4m",
            prompt_file.to_str().unwrap(),
        ])
        .timeout(Duration::from_secs(5 * 60))
        .assert();

    let output = assert.get_output().clone();
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    let code = output.status.code();

    // The regression is a hang surfaced as a step-timeout termination. A
    // successful run exits 0; if the child stalled at `external_directory`, the
    // step-timeout fires and the run exits non-zero — fail with the captured
    // diagnostics so the asymmetry is obvious in CI logs.
    assert_eq!(
        code,
        Some(0),
        "non-interactive OpenCode YOLO compose with a subagent touching an external \
         directory must COMPLETE, not hang at the `external_directory` gate.\n\
         exit={code:?}\n--- stdout ---\n{stdout}\n--- stderr ---\n{stderr}"
    );

    // A timeout-driven termination leaves a recognizable marker; assert it is
    // absent so a future change that makes the run exit 0 *after* timing out
    // still fails this test.
    let combined = format!("{stdout}\n{stderr}");
    let lowered = combined.to_lowercase();
    assert!(
        !lowered.contains("step-timeout")
            && !lowered.contains("step timeout")
            && !lowered.contains("timed out"),
        "run reported a timeout, indicating the subagent hung at the permission gate:\n{combined}"
    );

    let _ = fs::remove_dir_all(&external_dir);
}
