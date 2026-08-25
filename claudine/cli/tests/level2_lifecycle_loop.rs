//! Level 2 loop-gate ordering tests (Finding 5, coverage #2 and #4).
//!
//! Drives the real `claudine compose --goose <doc>` binary end-to-end inside a
//! real tmux pane (the L2 resource) with a fake provider, and asserts the
//! externally observable ordered side-effect log written by the loop's
//! lifecycle concerns and per-iteration stacks.
//!
//! ## Coverage #2 — loop gate ordering + per-iteration finalize count
//!
//! A looping document (`until: "phase > N"`, `action: "increment(phase)"`) with
//! per-event `finalize`/`start`/`initialize` stacks plus a `loop`-gate stack
//! that records the *current* `phase` proves:
//!
//! - exactly **one** `initialize` across the whole loop (later iterations
//!   re-enter at `start`, not `initialize`);
//! - exactly **N** `finalize` markers for an N-iteration loop;
//! - the loop-gate concern fires **before** the condition is evaluated and
//!   **before** the per-iteration `increment` mutation — it records the
//!   *pre-increment* `phase` (gate:1, gate:2, gate:3 for a 3-iteration loop).
//!
//! ## Coverage #4 — blocked-first-iteration edge case
//!
//! A looping document whose pre-flight fails on the first iteration routes to
//! `blocked` → `finalize`, then with the default `fail_fast: true` exits
//! **before** the loop gate (no `gate` marker, no iteration).
//!
//! ## Loop-path lifecycle wiring (Finding 5, fixed)
//!
//! `compose/prep.rs::build_and_run_loop` now parses the lifecycle config from
//! the document's **full** composed frontmatter (via
//! `loop_engine::build_loop_seed_with_lifecycle`) rather than from the
//! control-variable-only seed, so every lifecycle event block and the `loop:`
//! gate concerns survive into the loop runner. These tests assert the
//! spec-correct ordered markers directly.
//!
//! ## Synchronization
//!
//! Completion is detected by polling the side-effect file (the deterministic,
//! full-history barrier) rather than the capped, scrollback-free pane.
//!
//! ## Skip-clean
//!
//! `TmuxHarness::available()` is checked via `require_level!(Level::L2, ...)`,
//! which skips when tmux is absent. `BISCUIT_TEST_LEVEL_REQUIRED=2` flips a
//! missing backend into a hard failure. Run via `just test-l2`.

#![cfg(unix)]

mod common;
use common::wrap::seed_minimal_config;
use common::{augmented_path, init_git_repo, write_executable};

use biscuit_test_harness::TerminalHarness;
use biscuit_test_harness::tmux::{TmuxHarness, kill_session_by_name, spawn_shell_session};
use serial_test::serial;
use std::fs;
use std::path::Path;
use std::sync::atomic::{AtomicU32, Ordering};
use std::time::{Duration, Instant};
use tempfile::tempdir;
use test_toolkit::{Backend, Level, require_level};

struct Staged {
    workspace: tempfile::TempDir,
    bin_dir: std::path::PathBuf,
    md_file: std::path::PathBuf,
    events_log: std::path::PathBuf,
}

/// Write a fake `goose` that always exits 0 and records `provider-ran`.
fn write_goose(bin_dir: &Path, events_log: &Path) {
    write_executable(
        &bin_dir.join("goose"),
        &format!(
            "#!/bin/sh\ncat > /dev/null\nprintf 'provider-ran\\n' >> {log}\nexit 0\n",
            log = events_log.display(),
        ),
    );
}

fn stage(doc: &str) -> Staged {
    let workspace = tempdir().unwrap();
    let bin_dir = workspace.path().join("bin");
    fs::create_dir_all(&bin_dir).unwrap();
    seed_minimal_config(workspace.path());
    assert!(init_git_repo(workspace.path()), "git init failed");

    let events_log = workspace.path().join("events.log");
    write_goose(&bin_dir, &events_log);

    let md_file = workspace.path().join("doc.md");
    fs::write(&md_file, doc).unwrap();

    Staged {
        workspace,
        bin_dir,
        md_file,
        events_log,
    }
}

fn event_lines(staged: &Staged) -> Vec<String> {
    fs::read_to_string(&staged.events_log)
        .unwrap_or_default()
        .lines()
        .map(|l| l.trim().to_string())
        .filter(|l| !l.is_empty())
        .collect()
}

/// Run `claudine compose --goose <doc>` in a real tmux pane and block until the
/// run settles (line count stable, or `expected_min_lines` reached), then
/// return the captured pane. Bounded well under the nextest slow-timeout so a
/// non-producing run never hangs the suite.
fn run_loop_in_tmux(staged: &Staged, expected_min_lines: usize) -> String {
    static SEQ: AtomicU32 = AtomicU32::new(0);
    let seq = SEQ.fetch_add(1, Ordering::Relaxed);

    let session = format!("biscuit_l2_lcloop_{}_{seq}", std::process::id());
    spawn_shell_session(&session, 180, 60).expect("failed to spawn tmux session");

    let mut harness = TmuxHarness::attach(&session);
    let _ = biscuit_test_harness::wait_for_prompt(&mut harness);

    let claudine = common::claudine_bin();
    let sentinel = format!("L2_LOOP_DONE_{seq}");
    let env_prefix = format!(
        "NO_COLOR='1' HOME='{home}' PATH='{path}' ",
        home = staged.workspace.path().display(),
        path = augmented_path(&staged.bin_dir).to_string_lossy(),
    );
    let cmd = format!(
        "cd {ws} && {env_prefix}{claudine} compose --goose {md} ; echo {sentinel}",
        ws = staged.workspace.path().display(),
        md = staged.md_file.display(),
    );
    harness
        .send_command_with_env(&cmd, &[])
        .expect("send compose command");

    // Settle on either the expected line count or a stable count (the run
    // finished without reaching the target — e.g. a fail-fast blocked loop).
    let deadline = Instant::now() + Duration::from_secs(20);
    let mut last_count = 0usize;
    let mut stable_since: Option<Instant> = None;
    while Instant::now() < deadline {
        let count = event_lines(staged).len();
        if count >= expected_min_lines {
            std::thread::sleep(Duration::from_millis(150));
            break;
        }
        if count == last_count {
            match stable_since {
                Some(since) if since.elapsed() >= Duration::from_millis(800) => break,
                Some(_) => {}
                None => stable_since = Some(Instant::now()),
            }
        } else {
            stable_since = None;
            last_count = count;
        }
        std::thread::sleep(Duration::from_millis(100));
    }

    let pane = harness.capture().map(|f| f.plain).unwrap_or_default();
    kill_session_by_name(&session);
    pane
}

/// Coverage #2: a 3-iteration loop fires `initialize` exactly once, fires
/// `finalize` once per iteration (3 total), and the loop-gate concern observes
/// pre-mutation (pre-increment) `phase` on each gate pass.
#[test]
#[serial(level2_lifecycle_loop)]
fn level2_lifecycle_loop_gate_order_and_finalize_count() {
    require_level!(Level::L2, TmuxHarness::available(), Backend::Tmux);

    // phase starts at 1; `until: phase > 2` exits after the gate sees phase 3.
    // Iterations: phase 1, 2, 3 → 3 finalize markers; gate records 1, 2, 3
    // (pre-increment) since concerns fire before the per-iteration mutation.
    let doc = r#"---
title: lifecycle loop
phase: 1
loop:
  until: "phase > 2"
  action: "increment(phase)"
  max: 10
  stack:
    - action: {append_line: ["events.log", "gate:{{phase}}"]}
initialize:
  stack:
    - action: {append_line: ["events.log", "initialize"]}
start:
  stack:
    - action: {append_line: ["events.log", "start"]}
finalize:
  stack:
    - action: {append_line: ["events.log", "finalize"]}
---
Phase {{phase}}
"#;
    let staged = stage(doc);
    // 1 initialize + 3*(start + provider-ran + finalize + gate) = 13 markers.
    // `events.log` records `provider-ran` too, so the settle target must count
    // it or the poll breaks mid-run before the terminal `gate:3` lands.
    let pane = run_loop_in_tmux(&staged, 13);

    let lines = event_lines(&staged);
    let lifecycle: Vec<&String> = lines.iter().filter(|l| *l != "provider-ran").collect();

    let count = |needle: &str| lifecycle.iter().filter(|l| **l == needle).count();
    assert_eq!(
        count("initialize"),
        1,
        "initialize must fire exactly once across the whole loop; got {lines:?}; pane:\n{pane}"
    );
    assert_eq!(
        count("start"),
        3,
        "start must re-fire each iteration (3 total); got {lines:?}; pane:\n{pane}"
    );
    assert_eq!(
        count("finalize"),
        3,
        "an N-iteration loop fires exactly N finalize events; got {lines:?}; pane:\n{pane}"
    );

    let gates: Vec<&String> = lifecycle
        .iter()
        .copied()
        .filter(|l| l.starts_with("gate:"))
        .collect();
    assert_eq!(
        gates,
        vec!["gate:1", "gate:2", "gate:3"],
        "the loop-gate concern fires before the increment and records the \
         pre-mutation phase each pass; got {lines:?}; pane:\n{pane}"
    );

    assert_eq!(
        lifecycle.first().map(|s| s.as_str()),
        Some("initialize"),
        "initialize is the first lifecycle event; got {lines:?}"
    );
    let first_finalize = lifecycle.iter().position(|l| **l == "finalize");
    let first_gate = lifecycle.iter().position(|l| l.starts_with("gate:"));
    assert!(
        matches!((first_finalize, first_gate), (Some(f), Some(g)) if f < g),
        "the first loop-gate concern fires after the first finalize \
         (gate is the post-finalize iterate-again gate); got {lines:?}"
    );
}

/// Coverage #4: a looping document whose pre-flight shell audit fails on
/// iteration 1 routes to `blocked` → `finalize`, then with the default
/// `fail_fast: true` exits before the loop gate. Assert: `blocked` and
/// `finalize` present, NO loop-gate marker, exactly one `finalize`, no
/// `start`, no provider invocation.
#[test]
#[serial(level2_lifecycle_loop)]
fn level2_lifecycle_loop_blocked_first_iteration_exits_before_gate() {
    require_level!(Level::L2, TmuxHarness::available(), Backend::Tmux);

    let doc = r#"---
title: lifecycle loop blocked
phase: 1
start:
  stack:
    - action: {shell: "rm -rf /tmp/nonexistent"}
loop:
  until: "phase > 2"
  action: "increment(phase)"
  max: 10
  stack:
    - action: {append_line: ["events.log", "gate:{{phase}}"]}
initialize:
  stack:
    - action: {append_line: ["events.log", "initialize"]}
blocked:
  stack:
    - action: {append_line: ["events.log", "blocked"]}
finalize:
  stack:
    - action: {append_line: ["events.log", "finalize"]}
---
Phase {{phase}}
"#;
    let staged = stage(doc);
    let pane = run_loop_in_tmux(&staged, 3);

    let lines = event_lines(&staged);
    assert!(
        lines.iter().any(|l| l == "blocked"),
        "the blocked event must fire when pre-flight fails on iteration 1; \
         got {lines:?}; pane:\n{pane}"
    );
    assert!(
        lines.iter().any(|l| l == "finalize"),
        "a blocked iteration must still reach finalize; got {lines:?}; pane:\n{pane}"
    );
    assert_eq!(
        lines.iter().filter(|l| **l == "finalize").count(),
        1,
        "the loop must not iterate past the blocked first iteration; exactly \
         one finalize; got {lines:?}; pane:\n{pane}"
    );
    assert!(
        !lines.iter().any(|l| l.starts_with("gate:")),
        "fail_fast (default) exits before the loop gate, so no gate concern \
         fires; got {lines:?}; pane:\n{pane}"
    );
    assert!(
        !lines.iter().any(|l| l == "start"),
        "start must not fire when pre-flight blocks; got {lines:?}; pane:\n{pane}"
    );
    assert!(
        !lines.iter().any(|l| l == "provider-ran"),
        "the provider must never run on the blocked path; got {lines:?}; pane:\n{pane}"
    );
}

/// Loop-path `initialize` control coverage: a looping document whose
/// `initialize` stack opts out via `skip` ends the run immediately — no
/// provider invocation, no `start`/`finalize`/`loop`-gate marker, no
/// iteration. This is the whole-document opt-out (spec.md:338), and the
/// loop driver (`execute_loop_with_lifecycle`) must honor it exactly like the
/// non-loop path. The existing loop tests prove `initialize` fires once but
/// never exercise a control *from* `initialize` in a loop document.
#[test]
#[serial(level2_lifecycle_loop)]
fn level2_lifecycle_loop_initialize_skip_ends_run_before_iterating() {
    require_level!(Level::L2, TmuxHarness::available(), Backend::Tmux);

    let doc = r#"---
title: lifecycle loop init skip
phase: 1
loop:
  until: "phase > 2"
  action: "increment(phase)"
  max: 10
  stack:
    - action: {append_line: ["events.log", "gate:{{phase}}"]}
initialize:
  stack:
    - action: {append_line: ["events.log", "initialize"]}
    - action: "skip"
start:
  stack:
    - action: {append_line: ["events.log", "start"]}
finalize:
  stack:
    - action: {append_line: ["events.log", "finalize"]}
---
Phase {{phase}}
"#;
    let staged = stage(doc);
    // Only the single `initialize` marker is expected; settle on a stable
    // count (the run exits cleanly without producing further markers).
    let pane = run_loop_in_tmux(&staged, 1);

    let lines = event_lines(&staged);
    assert!(
        lines.iter().any(|l| l == "initialize"),
        "the initialize stack runs up to the skip; got {lines:?}; pane:\n{pane}"
    );
    assert!(
        !lines.iter().any(|l| l == "start"),
        "skip ends the run before start; got {lines:?}; pane:\n{pane}"
    );
    assert!(
        !lines.iter().any(|l| l == "finalize"),
        "skip runs no finalize; got {lines:?}; pane:\n{pane}"
    );
    assert!(
        !lines.iter().any(|l| l.starts_with("gate:")),
        "skip runs no loop gate; got {lines:?}; pane:\n{pane}"
    );
    assert!(
        !lines.iter().any(|l| l == "provider-ran"),
        "the provider must never run after an initialize skip; got {lines:?}; pane:\n{pane}"
    );
}

/// Loop-gate `error(...)` coverage (Finding 3): a looping document whose
/// `loop:` gate stack ends in an explicit `error(...)` converts the loop's
/// final outcome to failure and exits the loop — even though `until: phase > 5`
/// would otherwise keep iterating. The error takes precedence over the
/// condition, so the per-iteration `"increment(phase)"` mutation is NOT applied
/// and no second iteration runs: exactly one `start`/`finalize`/`provider-ran`,
/// the gate marker recorded before the error, and a non-zero exit reported in
/// the pane.
#[test]
#[serial(level2_lifecycle_loop)]
fn level2_lifecycle_loop_gate_error_fails_and_exits() {
    require_level!(Level::L2, TmuxHarness::available(), Backend::Tmux);

    // phase starts at 1; `until: phase > 5` would run many iterations, but the
    // gate's `error(...)` aborts after the first pass. The gate records
    // `gate:1` (pre-increment phase) and then raises the error.
    let doc = r#"---
title: lifecycle loop gate error
phase: 1
loop:
  until: "phase > 5"
  action: "increment(phase)"
  max: 10
  stack:
    - action: {append_line: ["events.log", "gate:{{phase}}"]}
    - action: {error: "gate rejected final state"}
initialize:
  stack:
    - action: {append_line: ["events.log", "initialize"]}
start:
  stack:
    - action: {append_line: ["events.log", "start"]}
finalize:
  stack:
    - action: {append_line: ["events.log", "finalize"]}
---
Phase {{phase}}
"#;
    let staged = stage(doc);
    // 1 initialize + 1 start + 1 provider-ran + 1 finalize + 1 gate = 5 markers,
    // then the gate errors and the loop exits.
    let pane = run_loop_in_tmux(&staged, 5);

    let lines = event_lines(&staged);
    let count = |needle: &str| lines.iter().filter(|l| *l == needle).count();

    assert_eq!(
        count("initialize"),
        1,
        "initialize fires once; got {lines:?}; pane:\n{pane}"
    );
    assert_eq!(
        count("start"),
        1,
        "exactly one iteration runs before the gate error; got {lines:?}; pane:\n{pane}"
    );
    assert_eq!(
        count("provider-ran"),
        1,
        "the provider runs exactly once; got {lines:?}; pane:\n{pane}"
    );
    assert_eq!(
        count("finalize"),
        1,
        "exactly one finalize before the gate error exits the loop; got {lines:?}; pane:\n{pane}"
    );

    let gates: Vec<&String> = lines.iter().filter(|l| l.starts_with("gate:")).collect();
    assert_eq!(
        gates,
        vec!["gate:1"],
        "the gate records the pre-increment phase exactly once, then errors and \
         exits — no second iteration; got {lines:?}; pane:\n{pane}"
    );

    // The explicit gate error fails the composition: claudine reports a non-zero
    // outcome. The authored reason surfaces in the rendered error.
    assert!(
        pane.contains("gate rejected final state"),
        "the gate error reason must surface in the run output; pane:\n{pane}"
    );
}
