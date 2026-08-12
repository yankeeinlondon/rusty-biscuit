//! Level 2 end-to-end tests for the two lifecycle action forms — **positional**
//! and **key/value** (feature `2026-06-26-positional-and-key-value`, Phase 6).
//!
//! Drives the real `claudine compose --goose <doc>` binary inside a real tmux
//! pane with a fake provider on `PATH`, and asserts the **externally observable**
//! results the new grammar must produce:
//!
//! - **mixed-form success stack** — a `when:`-gated stack mixing a positional
//!   multi-arg `set_frontmatter`, a positional communication action, and a
//!   key/value `shell` action runs end-to-end; event-time interpolation
//!   resolves and the side-effect writes the expected frontmatter to a target
//!   document.
//! - **typed argument write-through** — `set_frontmatter: ["state.md", "ready",
//!   "{{ true }}"]` writes a YAML boolean `true` (not the string `"true"`), and
//!   `merge_frontmatter: ["state.md", "{{ payload }}"]` merges the object stored
//!   in `payload`, proving the whole-value typed-resolution escape hatch reaches
//!   the side-effect engine.
//! - **key/value literal default** — a key/value `{ action: message, message:
//!   "doc.title" }` sends the **literal** string `doc.title`, while the
//!   `{{ doc.title }}` equivalent resolves the value. This is the breaking
//!   change the spec calls out: key/value string parameters are literal by
//!   default.
//!
//! Why files and tmux: the lifecycle side effects write to real files
//! (`state.md`, `events.log`) in a git-repo workspace whose mutation root is the
//! workspace root, so the assertions read deterministic on-disk state rather
//! than the capped, scrollback-free pane. The communication-channel assertion
//! (literal default) reads the visible pane because `message` targets stderr.
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
use biscuit_test_harness::tmux::{TmuxHarness, kill_session_by_name};
use serial_test::serial;
use std::fs;
use std::path::Path;
use std::sync::atomic::{AtomicU32, Ordering};
use std::time::{Duration, Instant};
use tempfile::tempdir;
use test_toolkit::{Backend, Level, require_level};

/// A staged compose run: a git-repo workspace, a fake `goose` provider on
/// `PATH`, the prompt document, the side-effect log the lifecycle stacks write
/// their event markers to, and the target `state.md` the side-effects mutate.
struct Staged {
    workspace: tempfile::TempDir,
    bin_dir: std::path::PathBuf,
    md_file: std::path::PathBuf,
    events_log: std::path::PathBuf,
    state_file: std::path::PathBuf,
}

/// Write a fake `goose` that exits 0 (drives the `success` event) and appends a
/// `provider-ran` line so a test can confirm the provider ran. It drains stdin
/// (`cat > /dev/null`) like the real wrappers expect.
fn write_goose(bin_dir: &Path, events_log: &Path) {
    write_executable(
        &bin_dir.join("goose"),
        &format!(
            "#!/bin/sh\ncat > /dev/null\nprintf 'provider-ran\\n' >> {log}\nexit 0\n",
            log = events_log.display(),
        ),
    );
}

/// Stage a workspace with the given prompt document and a seeded `state.md`
/// (the side-effect target). `state.md` carries a minimal frontmatter block so
/// the `set_frontmatter`/`merge_frontmatter` verbs (which load the document
/// before mutating it) have a parseable file to read.
fn stage(doc: &str) -> Staged {
    let workspace = tempdir().unwrap();
    let bin_dir = workspace.path().join("bin");
    fs::create_dir_all(&bin_dir).unwrap();
    seed_minimal_config(workspace.path());
    // A git repo so the effect engine's mutation root resolves to the workspace
    // root and the side-effect files land somewhere the test can read.
    assert!(init_git_repo(workspace.path()), "git init failed");

    let events_log = workspace.path().join("events.log");
    write_goose(&bin_dir, &events_log);

    let state_file = workspace.path().join("state.md");
    fs::write(&state_file, "---\nstatus: pending\n---\nstate body\n").unwrap();

    let md_file = workspace.path().join("doc.md");
    fs::write(&md_file, doc).unwrap();

    Staged {
        workspace,
        bin_dir,
        md_file,
        events_log,
        state_file,
    }
}

/// Run `claudine compose --goose [extra] <doc>` inside a real tmux pane and wait
/// for the run to finish (the terminal `done_marker` lands in `events.log`) or
/// the deadline elapses. Returns the captured pane text.
fn run_compose_in_tmux(staged: &Staged, done_marker: &str, extra_claudine_args: &str) -> String {
    static SEQ: AtomicU32 = AtomicU32::new(0);
    let seq = SEQ.fetch_add(1, Ordering::Relaxed);

    let session = format!("biscuit_l2_actforms_{}_{seq}", std::process::id());
    let shell = biscuit_test_harness::detect_shell();
    let spawned = std::process::Command::new("tmux")
        .args([
            "new-session",
            "-d",
            "-s",
            &session,
            "-x",
            "180",
            "-y",
            "60",
            &format!("{shell} -l"),
        ])
        .status()
        .map(|s| s.success())
        .unwrap_or(false);
    assert!(spawned, "failed to spawn tmux session");

    let mut harness = TmuxHarness::attach(&session);
    let _ = biscuit_test_harness::wait_for_prompt(&mut harness);

    let claudine = common::claudine_bin();
    let sentinel = format!("L2_AF_DONE_{seq}");
    let env_prefix = format!(
        "NO_COLOR='1' HOME='{home}' PATH='{path}' ",
        home = staged.workspace.path().display(),
        path = augmented_path(&staged.bin_dir).to_string_lossy(),
    );
    let extra = if extra_claudine_args.is_empty() {
        String::new()
    } else {
        format!(" {extra_claudine_args}")
    };
    let cmd = format!(
        "cd {ws} && {env_prefix}{claudine} compose --goose{extra} {md} ; echo {sentinel}",
        ws = staged.workspace.path().display(),
        md = staged.md_file.display(),
    );
    harness
        .send_command_with_env(&cmd, &[])
        .expect("send compose command");

    let deadline = Instant::now() + Duration::from_secs(30);
    while Instant::now() < deadline {
        if event_lines(staged).iter().any(|l| l == done_marker) {
            std::thread::sleep(Duration::from_millis(150));
            break;
        }
        std::thread::sleep(Duration::from_millis(100));
    }

    let pane = harness.capture().map(|f| f.plain).unwrap_or_default();
    kill_session_by_name(&session);
    pane
}

/// Read `events.log` as an ordered `Vec` of trimmed non-empty lines.
fn event_lines(staged: &Staged) -> Vec<String> {
    fs::read_to_string(&staged.events_log)
        .unwrap_or_default()
        .lines()
        .map(|l| l.trim().to_string())
        .filter(|l| !l.is_empty())
        .collect()
}

/// Read the post-run contents of the side-effect target `state.md`.
fn state_contents(staged: &Staged) -> String {
    fs::read_to_string(&staged.state_file).unwrap_or_default()
}

/// The spec's motivating acceptance shape, end-to-end: a `when:`-gated `success`
/// stack mixing a positional multi-arg `set_frontmatter`, a positional
/// communication action (`stderr`), and a key/value `shell` action. Proves the
/// mixed-form stack parses, the gate evaluates, event-time interpolation
/// resolves (`{{ doc.phase }}`/`{{ doc.title }}`), the positional side-effect
/// writes the interpolated frontmatter to `state.md`, and the key/value shell
/// action runs (`--yolo` auto-approves it at pre-flight).
#[test]
#[serial(level2_lifecycle)]
fn level2_action_forms_mixed_success_stack_writes_frontmatter() {
    require_level!(Level::L2, TmuxHarness::available(), Backend::Tmux);

    let doc = r#"---
title: ACTFORMS_TITLE
phase: phase-6
start:
  stack:
    - action: {append_line: ["events.log", "start"]}
success:
  stack:
    - when: "file_exists('state.md')"
      action:
        - set_frontmatter: ["state.md", "status", "production ready in {{ doc.phase }}"]
        - stderr: "SUCCESS_COMM {{ doc.title }}"
        - action: shell
          command: "echo shell-ran >> events.log"
    - action: {append_line: ["events.log", "success"]}
finalize:
  stack:
    - action: {append_line: ["events.log", "finalize"]}
---
Body
"#;
    let staged = stage(doc);
    // `--yolo` auto-approves the lifecycle `shell` action during pre-flight; the
    // non-interactive pane has no TTY to prompt on.
    let pane = run_compose_in_tmux(&staged, "finalize", "--yolo");

    let lines = event_lines(&staged);
    let lifecycle: Vec<&String> = lines.iter().filter(|l| *l != "provider-ran").collect();
    assert_eq!(
        lifecycle,
        vec!["start", "shell-ran", "success", "finalize"],
        "mixed-form success stack must run the key/value shell action then the \
         trailing markers in order; events.log was {lines:?}; pane:\n{pane}"
    );

    // The positional multi-arg `set_frontmatter` wrote the interpolated value to
    // the target document's frontmatter (event-time `{{ doc.phase }}` resolved).
    let state = state_contents(&staged);
    assert!(
        state.contains("status: production ready in phase-6"),
        "positional set_frontmatter must write the interpolated status to \
         state.md; state.md was:\n{state}\npane:\n{pane}"
    );

    // The positional communication action fired with `{{ doc.title }}` resolved.
    assert!(
        pane.contains("SUCCESS_COMM ACTFORMS_TITLE"),
        "positional stderr communication must fire with doc.title resolved; \
         pane:\n{pane}"
    );
}

/// Typed-argument write-through, end-to-end. A whole-value `{{ true }}` argument
/// writes a YAML boolean (not the string `"true"`), and a whole-value
/// `{{ doc.payload }}` argument passes the object stored in frontmatter to
/// `merge_frontmatter`. Proves the spec's typed escape hatch reaches the
/// side-effect engine through the positional array form.
#[test]
#[serial(level2_lifecycle)]
fn level2_action_forms_typed_argument_write_through() {
    require_level!(Level::L2, TmuxHarness::available(), Backend::Tmux);

    let doc = r#"---
title: typed write-through
payload:
  owner: ken
success:
  stack:
    - action: {set_frontmatter: ["state.md", "ready", "{{ true }}"]}
    - action: {merge_frontmatter: ["state.md", "{{ doc.payload }}"]}
finalize:
  stack:
    - action: {append_line: ["events.log", "finalize"]}
---
Body
"#;
    let staged = stage(doc);
    let pane = run_compose_in_tmux(&staged, "finalize", "");

    let state = state_contents(&staged);
    // `{{ true }}` resolves to a typed boolean — YAML serializes a bare `true`,
    // never a quoted `"true"`/`'true'` string.
    assert!(
        state.contains("ready: true"),
        "whole-value `{{{{ true }}}}` must write a YAML boolean `ready: true`; \
         state.md was:\n{state}\npane:\n{pane}"
    );
    assert!(
        !state.contains("ready: \"true\"") && !state.contains("ready: 'true'"),
        "the typed boolean must not be stringified; state.md was:\n{state}\npane:\n{pane}"
    );
    // `{{ doc.payload }}` passes the object through to `merge_frontmatter`.
    assert!(
        state.contains("owner: ken"),
        "merge_frontmatter must merge the object stored in `payload`; \
         state.md was:\n{state}\npane:\n{pane}"
    );
}

/// Key/value literal default, end-to-end (the spec's breaking change). A
/// key/value communication action whose parameter is `doc.title` sends the
/// **literal** string `doc.title`, while the `{{ doc.title }}` equivalent
/// resolves the value. The `stderr` channel is used (its semantics are
/// identical to `message` for this rule) because it writes plain prose to the
/// pane deterministically, where `message` routes through statusful logging
/// that is suppressed in the non-verbose, piped test environment.
#[test]
#[serial(level2_lifecycle)]
fn level2_action_forms_keyvalue_literal_default() {
    require_level!(Level::L2, TmuxHarness::available(), Backend::Tmux);

    let doc = r#"---
title: LITDEFAULT_TITLE
success:
  stack:
    - action:
        action: stderr
        message: "LITERAL=doc.title"
    - action:
        action: stderr
        message: "INTERP={{ doc.title }}"
finalize:
  stack:
    - action: {append_line: ["events.log", "finalize"]}
---
Body
"#;
    let staged = stage(doc);
    let pane = run_compose_in_tmux(&staged, "finalize", "");

    // The literal-default key/value parameter is sent verbatim: `doc.title` is
    // the literal string, NOT the resolved title.
    assert!(
        pane.contains("LITERAL=doc.title"),
        "a key/value string parameter must be literal by default — `doc.title` \
         must reach stderr unresolved; pane:\n{pane}"
    );
    assert!(
        !pane.contains("LITERAL=LITDEFAULT_TITLE"),
        "the literal-default parameter must NOT resolve as an expression; \
         pane:\n{pane}"
    );
    // The `{{ doc.title }}` equivalent resolves the context value.
    assert!(
        pane.contains("INTERP=LITDEFAULT_TITLE"),
        "a `{{{{ doc.title }}}}` span must resolve the document title; pane:\n{pane}"
    );
}
