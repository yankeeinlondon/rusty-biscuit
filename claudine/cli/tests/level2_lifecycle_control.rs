//! Level 2 lifecycle control-flow tests (Finding 5, coverage #3).
//!
//! Drives the real `claudine compose <provider> <doc>` binary end-to-end inside
//! a real tmux pane (the L2 resource) with fake providers, exercising the
//! `failure`-stack lifecycle control actions wired in Finding 3
//! (`harness_orch/loop_control.rs::dispatch_terminal_control` +
//! `composition/lifecycle_control.rs::decide_control`). Assertions are over the
//! externally observable ordered side-effect log plus, for the typed-error
//! controls, the visible error surface in the captured pane.
//!
//! ## Controls covered and their *actual* wired behavior
//!
//! - **Retry from `failure`** — `failure.stack` ending in `{retry: N}` re-invokes
//!   the provider; the provider runs `1 + N` times (the original attempt plus N
//!   retries), `failure` fires each attempt, the budget exhausts, then
//!   `finalize` fires once. Asserted via the `provider-ran` / `failure` line
//!   counts in `events.log`.
//! - **Retry from `finalize` (verify in `success`, recover in `finalize`)** — a
//!   `success.stack` raises `{error: "..."}` on a missing artifact, which routes
//!   through the `failure` event and carries an `err` into `finalize`; the `finalize.stack`
//!   `retry`s. The fake provider produces the artifact on its second invocation,
//!   so the retried attempt verifies clean and the terminal `finalize` runs its
//!   no-`err` branch. Exercises the `finalize` recovery surface and the
//!   success-downgrade `err` plumbing.
//! - **Resume from `failure` without a session id** — surfaces the typed
//!   `CompositionError::LifecycleResumeWithoutSession` (not a silent no-op),
//!   then `finalize` fires and the run exits. The fake provider never reports a
//!   session id, so this is the always-reached branch. Asserted via the typed
//!   error text in the pane and the `failure`→`finalize` markers.
//! - **Resume from `failure` with a session id** — a fake resume-capable Claude
//!   reports a stream session id on the failing first attempt; `{resume: "..."}`
//!   re-enters at `start`, invokes the provider with the resume argv/session id,
//!   delivers the follow-up prompt, then reaches `success`/`finalize`.
//! - **Defer from `failure`** — `defer` parses and dispatches, but its runtime
//!   home (the rendezvous deferred-execution scheduler) is not ready yet, so it
//!   surfaces the typed "not implemented" error; `failure` and `finalize` still
//!   fire. Asserted via `events.log` markers and the error text in the pane.
//!
//! - **Proxy from `failure`** — `failure.stack` ending in `{proxy: "@target.md"}`
//!   hands off to the target document, which runs its OWN lifecycle
//!   (`start`/`success`/`finalize`) once with its own provider exit. The source
//!   document's `failure`/`proxy` stack does NOT re-fire (no infinite loop), and
//!   the target's markers appear while the source's terminal `finalize` does
//!   not. Asserted via the ordered `events.log` markers. (Finding 5: the proxy
//!   runtime now re-parses the guard's lifecycle from the target's frontmatter
//!   on hand-off, so the target's events fire and the source's stack cannot
//!   re-trigger.)
//!
//! - **Proxy from `initialize`** — `initialize.stack` ending in `{proxy: "@target.md"}`
//!   hands off before the source's `start`. The target's own `initialize` fires
//!   and respects target-side `Skip`/`Proxy`/`Error` controls, including nested
//!   proxy cycle detection.
//!
//! - **Retry from `blocked`** — removed with the harness validation DSL;
//!   lifecycle `blocked` recovery actions now cover the same surface.
//!
//! - **Proxy from `blocked`** — removed with the harness validation DSL;
//!   proxy hand-off remains available from `initialize`, `start`, and `failure`.
//!
//! - **Defer from `blocked`** — accepted (flow control is universal) but, like
//!   `defer` everywhere, returns the typed "not implemented" error until the
//!   rendezvous deferred-execution backend lands.
//!
//! - **Top-level-before-stack ordering** — for `success` and `blocked`, top-level
//!   communication properties (`stderr`, `info`, `warn`, etc.) fire before any
//!   `stack:` side effects in the captured terminal output.
//!
//! - **`success.stack` downgrade** — when the stack ends in
//!   `{error: "downgraded"}`, the already-fired original top-level communication
//!   is preserved, the `failure` event fires next, and the terminal lifecycle
//!   state is failure. Asserted by marker ordering in the pane. Downgrade from
//!   `blocked.stack` is no longer supported because the shell-audit blocked
//!   path runs during composition preflight and does not process control
//!   actions from the blocked stack.
//!
//! ## Synchronization
//!
//! Completion is detected by polling the side-effect file for the terminal
//! `finalize` marker — the deterministic, full-history barrier — rather than the
//! capped, scrollback-free pane.
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
use rendezvous_core::local_endpoint::LocalEndpoint;
use rendezvous_core::local_endpoint::test_support::{endpoint_env_value, private_endpoint};
use tempfile::tempdir;
use test_toolkit::{Level, require_level};

struct Staged {
    workspace: tempfile::TempDir,
    bin_dir: std::path::PathBuf,
    md_file: std::path::PathBuf,
    events_log: std::path::PathBuf,
    rendezvous_endpoint: Option<LocalEndpoint>,
}

/// A fake `goose` that always exits non-zero (drives the `failure` event) and
/// records each invocation so retry counts are observable.
fn write_failing_goose(bin_dir: &Path, events_log: &Path) {
    write_executable(
        &bin_dir.join("goose"),
        &format!(
            "#!/bin/sh\ncat > /dev/null\nprintf 'provider-ran\\n' >> {log}\nexit 99\n",
            log = events_log.display(),
        ),
    );
}

/// A fake `goose` that always exits 0 (drives the `success` event) and records
/// each invocation so run counts are observable.
fn write_succeeding_goose(bin_dir: &Path, events_log: &Path) {
    write_executable(
        &bin_dir.join("goose"),
        &format!(
            "#!/bin/sh\ncat > /dev/null\nprintf 'provider-ran\\n' >> {log}\nexit 0\n",
            log = events_log.display(),
        ),
    );
}

/// A fake `claude` that fails once after reporting a session id, then succeeds
/// only when re-invoked through the provider's resume argv with the lifecycle
/// follow-up prompt on stdin.
fn write_resumable_claude(
    bin_dir: &Path,
    events_log: &Path,
    session_id: &str,
    follow_up: &str,
) {
    write_executable(
        &bin_dir.join("claude"),
        &format!(
            r#"#!/bin/sh
prompt=$(cat)
printf 'provider-ran\n' >> {log}
case " $* " in
  *" -r {session_id} "*)
    printf 'resume-session-ok\n' >> {log}
    case "$prompt" in
      *"{follow_up}"*) printf 'follow-up-ok\n' >> {log} ;;
      *) printf 'follow-up-missing:%s\n' "$prompt" >> {log} ;;
    esac
    printf '%s\n' '{{"type":"init","session_id":"{session_id}","model":"claude-test"}}'
    printf '%s\n' '{{"type":"assistant","content":[{{"type":"text","text":"resumed ok"}}]}}'
    exit 0
    ;;
  *)
    printf 'initial-prompt-ok\n' >> {log}
    printf '%s\n' '{{"type":"init","session_id":"{session_id}","model":"claude-test"}}'
    exit 99
    ;;
esac
"#,
            log = events_log.display(),
            session_id = session_id,
            follow_up = follow_up,
        ),
    );
}

fn stage_with_provider(doc: &str, succeeding: bool) -> Staged {
    let workspace = tempdir().unwrap();
    let bin_dir = workspace.path().join("bin");
    fs::create_dir_all(&bin_dir).unwrap();
    seed_minimal_config(workspace.path());
    assert!(init_git_repo(workspace.path()), "git init failed");

    let events_log = workspace.path().join("events.log");
    if succeeding {
        write_succeeding_goose(&bin_dir, &events_log);
    } else {
        write_failing_goose(&bin_dir, &events_log);
    }

    let md_file = workspace.path().join("doc.md");
    fs::write(&md_file, doc).unwrap();

    Staged {
        workspace,
        bin_dir,
        md_file,
        events_log,
        rendezvous_endpoint: None,
    }
}

/// Stage a document with a fake provider that always fails (drives `blocked` or
/// `failure` events).
fn stage(doc: &str) -> Staged {
    stage_with_provider(doc, false)
}

/// Stage a document with a fake provider that always succeeds (drives the
/// `success` event).
fn stage_success(doc: &str) -> Staged {
    stage_with_provider(doc, true)
}

/// Stage a source/target document pair for proxy hand-off tests.
///
/// `source_doc` is written to `doc.md` and is the document passed to
/// `claudine compose`. `target_doc` is written to `target.md`. The fake
/// provider's exit behavior is controlled by `succeeding`.
fn stage_proxy_pair(source_doc: &str, target_doc: &str, succeeding: bool) -> Staged {
    let workspace = tempdir().unwrap();
    let bin_dir = workspace.path().join("bin");
    fs::create_dir_all(&bin_dir).unwrap();
    seed_minimal_config(workspace.path());
    assert!(init_git_repo(workspace.path()), "git init failed");

    let events_log = workspace.path().join("events.log");
    if succeeding {
        write_succeeding_goose(&bin_dir, &events_log);
    } else {
        write_failing_goose(&bin_dir, &events_log);
    }

    let main_file = workspace.path().join("doc.md");
    fs::write(&main_file, source_doc).unwrap();
    fs::write(workspace.path().join("target.md"), target_doc).unwrap();

    Staged {
        workspace,
        bin_dir,
        md_file: main_file,
        events_log,
        rendezvous_endpoint: None,
    }
}

// Retained for when `defer` is wired to the rendezvous deferred-execution
// backend; currently unused because `defer` returns the not-implemented error.
#[allow(dead_code)]
struct RendezvousQueue {
    runtime: tokio::runtime::Runtime,
    handle: Option<rendezvous_daemon::server::ServerHandle>,
    endpoint: LocalEndpoint,
    node_id: String,
}

#[allow(dead_code)]
impl RendezvousQueue {
    fn spawn(workspace: &Path) -> Self {
        let endpoint = private_endpoint(workspace, "rendezvous");
        let runtime = tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .expect("tokio runtime");
        let config =
            rendezvous_daemon::server::DaemonConfig::with_data_dir(workspace.join("rv-data"))
                .without_networking();
        let handle = {
            let _enter = runtime.enter();
            rendezvous_daemon::local_transport::spawn_local_server(endpoint.clone(), config)
                .expect("spawn rendezvous daemon")
        };
        let node_id = handle.node_id();
        Self {
            runtime,
            handle: Some(handle),
            endpoint,
            node_id,
        }
    }

    fn entries(&self) -> Vec<rendezvous_core::SessionEntry> {
        self.runtime.block_on(async {
            let mut client = rendezvous_client::connect(&self.endpoint)
                .await
                .expect("connect rendezvous");
            let chunks = client
                .list_session_chunks(rendezvous_core::ListSessionChunksRequest {
                    owner_node_id: self.node_id.clone(),
                    session_id: "claudine-deferred-execution".to_string(),
                })
                .await
                .expect("list chunks")
                .into_inner();
            let mut entries = Vec::new();
            for chunk_id in chunks.chunk_ids {
                let listed = client
                    .list_chunk_entries(rendezvous_core::ListChunkEntriesRequest { chunk_id })
                    .await
                    .expect("list entries")
                    .into_inner();
                entries.extend(listed.entries);
            }
            entries
        })
    }
}

impl Drop for RendezvousQueue {
    fn drop(&mut self) {
        if let Some(handle) = self.handle.take() {
            let _ = self.runtime.block_on(handle.shutdown());
        }
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
/// terminal `finalize` marker lands (run finished) or the deadline elapses.
/// Returns the captured pane (for typed-error assertions).
fn run_in_tmux(staged: &Staged) -> String {
    run_provider_in_tmux_for(staged, "--goose", "finalize")
}

/// Like [`run_in_tmux`] but blocks until the named terminal marker lands in
/// `events.log` (or the deadline elapses). Used by the proxy test, whose
/// terminal marker is the target document's `target-finalize`.
fn run_in_tmux_for(staged: &Staged, done_marker: &str) -> String {
    run_provider_in_tmux_for(staged, "--goose", done_marker)
}

fn run_provider_in_tmux_for(staged: &Staged, provider_flag: &str, done_marker: &str) -> String {
    static SEQ: AtomicU32 = AtomicU32::new(0);
    let seq = SEQ.fetch_add(1, Ordering::Relaxed);

    let session = format!("biscuit_l2_lcctl_{}_{seq}", std::process::id());
    let shell = biscuit_test_harness::detect_shell();
    let spawned = std::process::Command::new("tmux")
        .args([
            "new-session",
            "-d",
            "-s",
            &session,
            "-x",
            "200",
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

    let claudine = env!("CARGO_BIN_EXE_claudine");
    let sentinel = format!("L2_CTL_DONE_{seq}");
    let env_prefix = format!(
        "NO_COLOR='1' HOME='{home}' PATH='{path}'{rendezvous} ",
        home = staged.workspace.path().display(),
        path = augmented_path(&staged.bin_dir).to_string_lossy(),
        rendezvous = staged
            .rendezvous_endpoint
            .as_ref()
            .map_or_else(String::new, |endpoint| {
                format!(
                    " RENDEZVOUS_ENDPOINT='{}'",
                    endpoint_env_value(endpoint).to_string_lossy()
                )
            }),
    );
    let cmd = format!(
        "cd {ws} && {env_prefix}{claudine} compose {provider_flag} {md} ; echo {sentinel}",
        ws = staged.workspace.path().display(),
        md = staged.md_file.display(),
    );
    harness
        .send_command_with_env(&cmd, &[])
        .expect("send compose command");

    let deadline = Instant::now() + Duration::from_secs(40);
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

/// Like [`run_in_tmux_for`] but appends caller `--set` positionals (e.g.
/// `spec=spec.md`) to the compose invocation. Used to prove those params
/// survive a `proxy` hand-off into the target document's re-materialization.
fn run_proxy_in_tmux_with_set(staged: &Staged, setters: &str, done_marker: &str) -> String {
    static SEQ: AtomicU32 = AtomicU32::new(0);
    let seq = SEQ.fetch_add(1, Ordering::Relaxed);

    let session = format!("biscuit_l2_lcctl_set_{}_{seq}", std::process::id());
    let shell = biscuit_test_harness::detect_shell();
    let spawned = std::process::Command::new("tmux")
        .args([
            "new-session",
            "-d",
            "-s",
            &session,
            "-x",
            "200",
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

    let claudine = env!("CARGO_BIN_EXE_claudine");
    let sentinel = format!("L2_CTL_DONE_{seq}");
    let env_prefix = format!(
        "NO_COLOR='1' HOME='{home}' PATH='{path}' ",
        home = staged.workspace.path().display(),
        path = augmented_path(&staged.bin_dir).to_string_lossy(),
    );
    let cmd = format!(
        "cd {ws} && {env_prefix}{claudine} compose --goose {md} {setters} ; echo {sentinel}",
        ws = staged.workspace.path().display(),
        md = staged.md_file.display(),
    );
    harness
        .send_command_with_env(&cmd, &[])
        .expect("send compose command");

    let deadline = Instant::now() + Duration::from_secs(40);
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

/// Like [`run_in_tmux_for`] but blocks until the `echo`-ed sentinel appears in
/// the captured pane (i.e. the compose command has exited, success or error),
/// then returns the pane. Used to assert on error output that writes nothing to
/// `events.log`.
fn run_compose_await_exit(staged: &Staged) -> String {
    static SEQ: AtomicU32 = AtomicU32::new(0);
    let seq = SEQ.fetch_add(1, Ordering::Relaxed);

    let session = format!("biscuit_l2_lcctl_exit_{}_{seq}", std::process::id());
    let shell = biscuit_test_harness::detect_shell();
    let spawned = std::process::Command::new("tmux")
        .args([
            "new-session", "-d", "-s", &session, "-x", "200", "-y", "60",
            &format!("{shell} -l"),
        ])
        .status()
        .map(|s| s.success())
        .unwrap_or(false);
    assert!(spawned, "failed to spawn tmux session");

    let mut harness = TmuxHarness::attach(&session);
    let _ = biscuit_test_harness::wait_for_prompt(&mut harness);

    let claudine = env!("CARGO_BIN_EXE_claudine");
    let sentinel = format!("L2_CTL_EXIT_{seq}");
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

    let deadline = Instant::now() + Duration::from_secs(40);
    let mut pane = String::new();
    while Instant::now() < deadline {
        pane = harness.capture().map(|f| f.plain).unwrap_or_default();
        // The sentinel is echoed only after compose exits; ignore the command
        // line itself (which also contains the literal token) by requiring it on
        // its own output line.
        if pane.lines().any(|l| l.trim() == sentinel) {
            break;
        }
        std::thread::sleep(Duration::from_millis(100));
    }
    kill_session_by_name(&session);
    pane
}

/// A proxy target whose body fails to compose (a bad `::file` transclusion)
/// must surface a *styled* error block, not a crude single-line `Error: …`.
/// Regression for the harness loop flattening the typed `BlockError` with
/// `eyre!("{e}")` on the re-materialization path.
#[test]
#[serial(level2_lifecycle_control)]
fn level2_lifecycle_initialize_proxy_compose_error_renders_styled_block() {
    require_level!(Level::L2, TmuxHarness::available(), "tmux");

    let source_doc = "---\ninitialize:\n  stack:\n    \
         - action: {proxy: '@target.md'}\n---\nsource body\n";
    // `::file` a non-existent partial: transclusion fails during the target's
    // re-materialization inside the harness loop.
    let target_doc = "---\ntitle: proxy target\n---\n::file _no_such_partial.md\n";
    let staged = stage_proxy_pair(source_doc, target_doc, true);

    let pane = run_compose_await_exit(&staged);

    // The styled `BlockError` renders the type name as its title (e.g.
    // "⤫ TransclusionError: …") inside a `┃`-bordered box. The crude fallback
    // (`log::error(report.to_string())`) prints a single un-bordered
    // "Error: Transclusion error: …" line instead — so the block title plus a
    // border glyph together prove the typed error survived to the walker.
    assert!(
        pane.contains("TransclusionError"),
        "the compose error must render as a typed block (not a crude line); pane:\n{pane}"
    );
    assert!(
        pane.contains('┃'),
        "the compose error must render inside a styled `┃`-bordered block; pane:\n{pane}"
    );
    assert!(
        !pane.contains("Error: Transclusion error"),
        "the crude single-line fallback must not be used; pane:\n{pane}"
    );
}

/// Retry from `failure`: `{retry: 2}` re-invokes the provider, so it runs 3 times
/// (original + 2 retries), `failure` fires each attempt, the budget exhausts,
/// then `finalize` fires exactly once.
#[test]
#[serial(level2_lifecycle_control)]
fn level2_lifecycle_failure_retry_reinvokes_provider_until_budget_exhausted() {
    require_level!(Level::L2, TmuxHarness::available(), "tmux");

    let doc = r#"---
title: lifecycle retry
failure:
  stack:
    - action: {append_line: ["events.log", "failure"]}
    - action: {retry: 2}
finalize:
  stack:
    - action: {append_line: ["events.log", "finalize"]}
---
Body
"#;
    let staged = stage(doc);
    let pane = run_in_tmux(&staged);

    let lines = event_lines(&staged);
    let provider_runs = lines.iter().filter(|l| **l == "provider-ran").count();
    let failures = lines.iter().filter(|l| **l == "failure").count();
    let finalizes = lines.iter().filter(|l| **l == "finalize").count();

    assert_eq!(
        provider_runs, 3,
        "{{retry: 2}} must invoke the provider 3 times (original + 2 retries); \
         got {lines:?}; pane:\n{pane}"
    );
    assert_eq!(
        failures, 3,
        "the failure event fires on each of the 3 attempts; got {lines:?}; pane:\n{pane}"
    );
    assert_eq!(
        finalizes, 1,
        "finalize fires exactly once after the retry budget is exhausted; \
         got {lines:?}; pane:\n{pane}"
    );
    // Ordering: the terminal finalize is the last marker.
    assert_eq!(
        lines.last().map(|s| s.as_str()),
        Some("finalize"),
        "finalize is the terminal marker; got {lines:?}"
    );
}

/// Verify-in-`success`, recover-in-`finalize`: a `success.stack` that detects a
/// missing artifact raises `{error: "..."}`, which routes through the `failure` event
/// and carries an `err` into `finalize`; the `finalize.stack` then `retry`s the
/// whole run. The fake provider creates the awaited artifact on its **second**
/// invocation, so the retried attempt verifies clean and the run ends in a
/// no-`err` `finalize`. Proves the `finalize` recovery surface and the
/// success-downgrade `err` plumbing end-to-end.
#[test]
#[serial(level2_lifecycle_control)]
fn level2_lifecycle_finalize_retry_recovers_success_verification_failure() {
    require_level!(Level::L2, TmuxHarness::available(), "tmux");

    let doc = r#"---
title: verify in success, recover in finalize
success:
  stack:
    - when: "!file_exists('attempt2.flag')"
      action:
        - {append_line: ["events.log", "verify-failed"]}
        - {error: "artifact missing"}
finalize:
  stack:
    - when: "err"
      action:
        - {append_line: ["events.log", "finalize-retry"]}
        - {retry: 1}
    - action: {append_line: ["events.log", "finalize-done"]}
---
Body
"#;
    let staged = stage_success(doc);
    // Replace the plain succeeding provider with one that creates the awaited
    // `attempt2.flag` on its SECOND invocation, so attempt 1 fails verification
    // (→ finalize retry) and attempt 2 verifies clean.
    let counter = staged.workspace.path().join(".attempt_counter");
    let flag = staged.workspace.path().join("attempt2.flag");
    write_executable(
        &staged.bin_dir.join("goose"),
        &format!(
            "#!/bin/sh\ncat > /dev/null\nprintf 'provider-ran\\n' >> {log}\n\
             if [ -f {counter} ]; then : > {flag}; fi\n: > {counter}\nexit 0\n",
            log = staged.events_log.display(),
            counter = counter.display(),
            flag = flag.display(),
        ),
    );

    let pane = run_in_tmux_for(&staged, "finalize-done");
    let lines = event_lines(&staged);
    let provider_runs = lines.iter().filter(|l| **l == "provider-ran").count();
    let verify_failed = lines.iter().filter(|l| **l == "verify-failed").count();
    let finalize_retry = lines.iter().filter(|l| **l == "finalize-retry").count();
    let finalize_done = lines.iter().filter(|l| **l == "finalize-done").count();

    assert_eq!(
        provider_runs, 2,
        "finalize {{retry: 1}} re-runs the provider exactly once (original + 1 retry); \
         got {lines:?}; pane:\n{pane}"
    );
    assert_eq!(
        verify_failed, 1,
        "the success verification fails only on the first attempt; got {lines:?}; pane:\n{pane}"
    );
    assert_eq!(
        finalize_retry, 1,
        "finalize recovers via retry exactly once (when `err` is present); \
         got {lines:?}; pane:\n{pane}"
    );
    assert_eq!(
        finalize_done, 1,
        "the retried attempt verifies clean, so the terminal finalize runs the \
         no-`err` branch once; got {lines:?}; pane:\n{pane}"
    );
    assert_eq!(
        lines.last().map(|s| s.as_str()),
        Some("finalize-done"),
        "the clean finalize is the terminal marker; got {lines:?}"
    );
}

/// Resume from `failure` without a session id surfaces the typed
/// `LifecycleResumeWithoutSession` error (not a silent no-op), then `finalize`
/// fires and the run exits. The fake provider never reports a session id, so
/// this branch is always reached.
#[test]
#[serial(level2_lifecycle_control)]
fn level2_lifecycle_failure_resume_without_session_surfaces_typed_error() {
    require_level!(Level::L2, TmuxHarness::available(), "tmux");

    let doc = r#"---
title: lifecycle resume
failure:
  stack:
    - action: {append_line: ["events.log", "failure"]}
    - action: {resume: "please finish the work"}
finalize:
  stack:
    - action: {append_line: ["events.log", "finalize"]}
---
Body
"#;
    let staged = stage(doc);
    let pane = run_in_tmux(&staged);

    let lines = event_lines(&staged);
    // The provider runs once (the original attempt); resume cannot proceed
    // without a session, so it does not re-invoke.
    assert_eq!(
        lines.iter().filter(|l| **l == "provider-ran").count(),
        1,
        "resume without a session must not re-invoke the provider; got {lines:?}; pane:\n{pane}"
    );
    assert!(
        lines.iter().any(|l| l == "failure") && lines.iter().any(|l| l == "finalize"),
        "failure then finalize must both fire; got {lines:?}; pane:\n{pane}"
    );
    // The typed error must be user-visible — not a silent no-op.
    assert!(
        pane.contains("resume") && pane.contains("session"),
        "the typed LifecycleResumeWithoutSession error must surface in the \
         terminal; pane:\n{pane}"
    );
}

/// Resume from `failure` with a provider-reported session id validates provider
/// resume support, re-invokes the provider with that session id, delivers the
/// lifecycle follow-up prompt, and starts a fresh second attempt lifecycle.
#[test]
#[serial(level2_lifecycle_control)]
fn level2_lifecycle_failure_resume_with_session_reinvokes_provider_with_follow_up() {
    require_level!(Level::L2, TmuxHarness::available(), "tmux");

    let follow_up = "please finish the resumed work";
    let doc = format!(
        r#"---
title: lifecycle resume success
start:
  stack:
    - action: {{append_line: ["events.log", "start"]}}
failure:
  stack:
    - action: {{append_line: ["events.log", "failure"]}}
    - action: {{resume: "{follow_up}"}}
success:
  stack:
    - action: {{append_line: ["events.log", "success"]}}
finalize:
  stack:
    - action: {{append_line: ["events.log", "finalize"]}}
---
Original body
"#
    );
    let workspace = tempdir().unwrap();
    let bin_dir = workspace.path().join("bin");
    fs::create_dir_all(&bin_dir).unwrap();
    seed_minimal_config(workspace.path());
    assert!(init_git_repo(workspace.path()), "git init failed");

    let events_log = workspace.path().join("events.log");
    let session_id = "lifecycle-session-123";
    write_resumable_claude(&bin_dir, &events_log, session_id, follow_up);

    let md_file = workspace.path().join("doc.md");
    fs::write(&md_file, doc).unwrap();
    let staged = Staged {
        workspace,
        bin_dir,
        md_file,
        events_log,
        rendezvous_endpoint: None,
    };
    let pane = run_provider_in_tmux_for(&staged, "--claude", "finalize");

    let lines = event_lines(&staged);
    assert_eq!(
        lines.iter().filter(|l| **l == "provider-ran").count(),
        2,
        "resume with a session must re-invoke the provider exactly once; \
         got {lines:?}; pane:\n{pane}"
    );
    assert!(
        lines.iter().any(|l| l == "resume-session-ok"),
        "the second provider invocation must receive the provider session id; \
         got {lines:?}; pane:\n{pane}"
    );
    assert!(
        lines.iter().any(|l| l == "follow-up-ok"),
        "the second provider invocation must receive the resume follow-up prompt; \
         got {lines:?}; pane:\n{pane}"
    );
    let expected = [
        "start",
        "provider-ran",
        "initial-prompt-ok",
        "failure",
        "start",
        "provider-ran",
        "resume-session-ok",
        "follow-up-ok",
        "success",
        "finalize",
    ];
    assert_eq!(
        lines, expected,
        "the resumed attempt must re-enter at start, then success, then finalize; pane:\n{pane}"
    );
    assert!(
        pane.contains("Claude session ID"),
        "the provider-reported session id must be surfaced in the real CLI path; pane:\n{pane}"
    );
}

/// `defer` from `failure` parses and dispatches, but its runtime home (the
/// rendezvous deferred-execution scheduler) is not ready, so it surfaces the
/// typed "not implemented" error. `failure` and `finalize` still fire (the
/// failure-dispatch Abort path runs `finalize` before propagating).
#[test]
#[serial(level2_lifecycle_control)]
fn level2_lifecycle_failure_defer_returns_not_implemented() {
    require_level!(Level::L2, TmuxHarness::available(), "tmux");

    let doc = r#"---
title: lifecycle defer
failure:
  stack:
    - action: {append_line: ["events.log", "failure"]}
    - action:
        action: defer
        delay: 5m
        reason: provider failed
finalize:
  stack:
    - action: {append_line: ["events.log", "finalize"]}
---
Body
"#;
    let staged = stage(doc);
    let pane = run_in_tmux(&staged);

    let lines = event_lines(&staged);
    assert_eq!(
        lines.iter().filter(|l| **l == "provider-ran").count(),
        1,
        "defer must not re-invoke the provider; got {lines:?}; pane:\n{pane}"
    );
    assert!(
        lines.iter().any(|l| l == "failure") && lines.iter().any(|l| l == "finalize"),
        "failure then finalize must both fire; got {lines:?}; pane:\n{pane}"
    );
    let lc = pane.to_lowercase();
    assert!(
        lc.contains("defer") && lc.contains("not implemented"),
        "the defer-not-implemented error must surface; pane:\n{pane}"
    );
}

/// A `goose` that exits 0 only when one of its arguments contains the target
/// document's body sentinel, and exits 99 otherwise. Goose delivers the prompt
/// via `-t <prompt>` args (not stdin), so the branch inspects `"$@"`. This lets
/// one fake provider on PATH fail the source document (driving `failure` →
/// `proxy`) and succeed the proxied target document (driving the target's
/// `success`).
fn write_proxy_goose(bin_dir: &Path, events_log: &Path, target_sentinel: &str) {
    write_executable(
        &bin_dir.join("goose"),
        &format!(
            "#!/bin/sh\nprintf 'provider-ran\\n' >> {log}\n\
             for a in \"$@\"; do\n  case \"$a\" in\n    *{sentinel}*) exit 0 ;;\n  esac\ndone\nexit 99\n",
            log = events_log.display(),
            sentinel = target_sentinel,
        ),
    );
}

/// Proxy from `failure`: the source document's `failure.stack` ends in
/// `{proxy: "@target.md"}`. The target runs its OWN lifecycle once
/// (`start`/`success`/`finalize`) with a clean provider exit. The source's
/// `failure`/`proxy` stack must NOT re-fire (no infinite loop), and the
/// target's markers must appear.
#[test]
#[serial(level2_lifecycle_control)]
fn level2_lifecycle_failure_proxy_runs_target_document_no_loop() {
    require_level!(Level::L2, TmuxHarness::available(), "tmux");

    let workspace = tempdir().unwrap();
    let bin_dir = workspace.path().join("bin");
    fs::create_dir_all(&bin_dir).unwrap();
    seed_minimal_config(workspace.path());
    assert!(init_git_repo(workspace.path()), "git init failed");

    let events_log = workspace.path().join("events.log");
    // The target document's body carries this sentinel; the goose exits 0 only
    // when it sees it on stdin (the proxied run), else 99 (the source run).
    let target_sentinel = "TARGET_BODY_MARKER";
    write_proxy_goose(&bin_dir, &events_log, target_sentinel);

    // Source document: provider fails → `failure` stack records a marker then
    // proxies to the target. Source has its own `finalize` that must NOT fire
    // (the run hands off before the source's terminal finalize).
    let main_doc = "---\ntitle: proxy source\nfailure:\n  stack:\n    \
         - action: {append_line: ['events.log', 'source-failure']}\n    \
         - action: {proxy: '@target.md'}\nfinalize:\n  stack:\n    \
         - action: {append_line: ['events.log', 'source-finalize']}\n---\nsource body\n";
    let main_file = workspace.path().join("main.md");
    fs::write(&main_file, main_doc).unwrap();

    // Target document: provider succeeds (stdin carries the sentinel) → its own
    // `start`/`success`/`finalize` fire exactly once.
    let target_doc = format!(
        "---\ntitle: proxy target\nstart:\n  stack:\n    \
         - action: {{append_line: ['events.log', 'target-start']}}\nsuccess:\n  stack:\n    \
         - action: {{append_line: ['events.log', 'target-success']}}\nfinalize:\n  stack:\n    \
         - action: {{append_line: ['events.log', 'target-finalize']}}\n---\n{target_sentinel}\n"
    );
    fs::write(workspace.path().join("target.md"), target_doc).unwrap();

    let staged = Staged {
        workspace,
        bin_dir,
        md_file: main_file,
        events_log,
        rendezvous_endpoint: None,
    };
    let pane = run_in_tmux_for(&staged, "target-finalize");

    let lines = event_lines(&staged);

    // The source failed once and proxied once — its failure stack must not
    // re-fire (the bug was an unbounded re-proxy loop).
    assert_eq!(
        lines.iter().filter(|l| **l == "source-failure").count(),
        1,
        "the source failure stack must fire exactly once (no re-proxy loop); \
         got {lines:?}; pane:\n{pane}"
    );
    // The target's own lifecycle fired.
    assert!(
        lines.iter().any(|l| l == "target-start"),
        "the target's own start must fire after proxy hand-off; got {lines:?}; pane:\n{pane}"
    );
    assert!(
        lines.iter().any(|l| l == "target-success"),
        "the target's own success must fire (its provider exits 0); got {lines:?}; pane:\n{pane}"
    );
    assert_eq!(
        lines.iter().filter(|l| **l == "target-finalize").count(),
        1,
        "the target finalize fires exactly once; got {lines:?}; pane:\n{pane}"
    );
    // The source's terminal finalize must NOT fire — the run handed off before
    // the source reached its own finalize, and the proxied target owns the
    // terminal lifecycle now.
    assert!(
        !lines.iter().any(|l| l == "source-finalize"),
        "the source finalize must not fire after handing off via proxy; \
         got {lines:?}; pane:\n{pane}"
    );
    // The provider ran exactly twice: once for the failing source, once for
    // the succeeding target. More than two would indicate a re-proxy loop.
    assert_eq!(
        lines.iter().filter(|l| **l == "provider-ran").count(),
        2,
        "the provider runs exactly twice (source fail + target success); \
         a higher count means the proxy looped; got {lines:?}; pane:\n{pane}"
    );
}

/// For a `success` event, top-level `stderr` fires before the stack's side
/// effects in the observable terminal output.
#[test]
#[serial(level2_lifecycle_control)]
fn level2_lifecycle_success_top_level_communication_fires_before_stack() {
    require_level!(Level::L2, TmuxHarness::available(), "tmux");

    let doc = r#"---
title: success ordering
success:
  stderr: "SUCCESS-TOP"
  stack:
    - action: {stderr: "SUCCESS-STACK"}
finalize:
  stack:
    - action: {append_line: ["events.log", "finalize"]}
---
Body
"#;
    let staged = stage_success(doc);
    let pane = run_in_tmux(&staged);

    let top_pos = pane
        .find("SUCCESS-TOP")
        .expect("success top-level stderr must appear in pane");
    let stack_pos = pane
        .find("SUCCESS-STACK")
        .expect("success stack stderr must appear in pane");
    assert!(
        top_pos < stack_pos,
        "top-level communication must fire before stack; pane:\n{pane}"
    );

    let lines = event_lines(&staged);
    assert!(
        lines.iter().any(|l| l == "finalize"),
        "finalize must fire; got {lines:?}"
    );
}

/// A `success.stack` ending in `{error: "downgraded"}` downgrades the run to
/// failure. The original success top-level communication still fires, the
/// failure event fires, and the final lifecycle state is failure.
#[test]
#[serial(level2_lifecycle_control)]
fn level2_lifecycle_success_stack_error_downgrades_to_failure_preserving_top_level() {
    require_level!(Level::L2, TmuxHarness::available(), "tmux");

    let doc = r#"---
title: success downgrade
success:
  stderr: "SUCCESS-TOP"
  stack:
    - action: {stderr: "SUCCESS-STACK"}
    - action: {error: "downgraded"}
failure:
  stderr: "FAILURE-TOP"
  stack:
    - action: {stderr: "FAILURE-STACK"}
finalize:
  stack:
    - action: {append_line: ["events.log", "finalize"]}
---
Body
"#;
    let staged = stage_success(doc);
    let pane = run_in_tmux(&staged);

    let markers = ["SUCCESS-TOP", "SUCCESS-STACK", "FAILURE-TOP", "FAILURE-STACK"];
    let positions: Vec<_> = markers
        .iter()
        .map(|m| {
            (
                *m,
                pane.find(m)
                    .unwrap_or_else(|| panic!("{m} must appear in pane; pane:\n{pane}")),
            )
        })
        .collect();
    assert!(
        positions.windows(2).all(|w| w[0].1 < w[1].1),
        "expected success top-level, success stack, failure top-level, failure stack \
         in that order; got {positions:?}; pane:\n{pane}"
    );

    let lines = event_lines(&staged);
    assert!(
        lines.iter().any(|l| l == "finalize"),
        "finalize must fire; got {lines:?}"
    );
}

/// For a `blocked` event, top-level `stderr` fires before the stack's side
/// effects in the observable terminal output. The blocked path is triggered
/// by a blacklisted shell command in `start.stack` failing composition
/// preflight shell audit.
#[test]
#[serial(level2_lifecycle_control)]
fn level2_lifecycle_blocked_top_level_communication_fires_before_stack() {
    require_level!(Level::L2, TmuxHarness::available(), "tmux");

    let doc = r#"---
title: blocked ordering
start:
  stack:
    - action: {shell: "rm -rf /tmp/nonexistent"}
blocked:
  stderr: "BLOCKED-TOP"
  stack:
    - action: {stderr: "BLOCKED-STACK"}
finalize:
  stack:
    - action: {append_line: ["events.log", "finalize"]}
---
Body
"#;
    let staged = stage(doc);
    let pane = run_in_tmux(&staged);

    let top_pos = pane
        .find("BLOCKED-TOP")
        .expect("blocked top-level stderr must appear in pane");
    let stack_pos = pane
        .find("BLOCKED-STACK")
        .expect("blocked stack stderr must appear in pane");
    assert!(
        top_pos < stack_pos,
        "top-level communication must fire before stack; pane:\n{pane}"
    );

    let lines = event_lines(&staged);
    assert!(
        lines.iter().any(|l| l == "finalize"),
        "finalize must fire; got {lines:?}"
    );
}

/// `initialize.stack` ending in `{proxy: "@target.md"}` hands off to the target
/// before the source's `start`. The target's own `initialize` fires, then its
/// normal lifecycle (`start`, `success`, `finalize`) runs. The source's
/// `start`/`success`/`finalize` do not fire.
#[test]
#[serial(level2_lifecycle_control)]
fn level2_lifecycle_initialize_proxy_runs_target_initialize() {
    require_level!(Level::L2, TmuxHarness::available(), "tmux");

    let source_doc = "---\ntitle: proxy source\ninitialize:\n  stack:\n    \
         - action: {append_line: ['events.log', 'source-init']}\n    \
         - action: {proxy: '@target.md'}\nstart:\n  stack:\n    \
         - action: {append_line: ['events.log', 'source-start']}\nsuccess:\n  stack:\n    \
         - action: {append_line: ['events.log', 'source-success']}\nfinalize:\n  stack:\n    \
         - action: {append_line: ['events.log', 'source-finalize']}\n---\nsource body\n";
    let target_doc = "---\ntitle: proxy target\ninitialize:\n  stack:\n    \
         - action: {append_line: ['events.log', 'target-init']}\nstart:\n  stack:\n    \
         - action: {append_line: ['events.log', 'target-start']}\nsuccess:\n  stack:\n    \
         - action: {append_line: ['events.log', 'target-success']}\nfinalize:\n  stack:\n    \
         - action: {append_line: ['events.log', 'target-finalize']}\n---\ntarget body\n";
    let staged = stage_proxy_pair(source_doc, target_doc, true);
    let pane = run_in_tmux_for(&staged, "target-finalize");

    let lines = event_lines(&staged);

    assert!(
        lines.iter().any(|l| l == "source-init"),
        "the source initialize must fire before the hand-off; got {lines:?}; pane:\n{pane}"
    );
    assert!(
        lines.iter().any(|l| l == "target-init"),
        "the target's own initialize must fire after proxy hand-off; got {lines:?}; pane:\n{pane}"
    );
    assert!(
        lines.iter().any(|l| l == "target-start"),
        "the target's own start must fire; got {lines:?}; pane:\n{pane}"
    );
    assert!(
        lines.iter().any(|l| l == "target-success"),
        "the target's own success must fire (its provider exits 0); got {lines:?}; pane:\n{pane}"
    );
    assert_eq!(
        lines.iter().filter(|l| **l == "target-finalize").count(),
        1,
        "the target finalize fires exactly once; got {lines:?}; pane:\n{pane}"
    );
    assert!(
        !lines.iter().any(|l| l == "source-start"),
        "the source start must not fire after handing off via proxy; got {lines:?}; pane:\n{pane}"
    );
    assert!(
        !lines.iter().any(|l| l == "source-success"),
        "the source success must not fire after handing off via proxy; got {lines:?}; pane:\n{pane}"
    );
    assert!(
        !lines.iter().any(|l| l == "source-finalize"),
        "the source finalize must not fire after handing off via proxy; got {lines:?}; pane:\n{pane}"
    );
}

/// Regression: a `proxy` hand-off must forward the caller's `--set` params and
/// launch-area file-ref anchor into the target's re-materialization. Both
/// documents declare `$schema: spec: file(required;eager)`, and the caller
/// passes `spec=spec.md`. Before the fix the target re-composed without the
/// `spec` override (and without the launch-area fallback dir), so its schema
/// validation failed — the run never reached the target's `success`.
#[test]
#[serial(level2_lifecycle_control)]
fn level2_lifecycle_initialize_proxy_forwards_set_params_to_target_schema() {
    require_level!(Level::L2, TmuxHarness::available(), "tmux");

    let source_doc = "---\n$schema:\n    spec: file(required;eager)\ninitialize:\n  stack:\n    \
         - action: {proxy: '@target.md'}\n---\nsource body\n";
    let target_doc = "---\n$schema:\n    spec: file(required;eager)\nsuccess:\n  stack:\n    \
         - action: {append_line: ['events.log', 'target-success']}\n---\ntarget body\n";
    let staged = stage_proxy_pair(source_doc, target_doc, true);
    // The `file(required;eager)` param resolves against the launch area (the
    // workspace we `cd` into), exercising the forwarded `file_ref_fallback_dir`.
    fs::write(staged.workspace.path().join("spec.md"), "---\nimplemented: true\n---\nspec\n")
        .unwrap();

    let pane = run_proxy_in_tmux_with_set(&staged, "spec=spec.md", "target-success");

    let lines = event_lines(&staged);
    assert!(
        lines.iter().any(|l| l == "target-success"),
        "the target's success must fire — the forwarded `spec` param must satisfy its \
         `$schema` after the proxy re-materialization; got {lines:?}; pane:\n{pane}"
    );
    assert!(
        !pane.contains("did not satisfy the schema"),
        "no schema-validation failure must surface for the proxy target; pane:\n{pane}"
    );
}

/// End-to-end precedence for `proxy.with`, through the shipped binary and the
/// normal `claudine compose` invocation path: target-authored frontmatter <
/// `with:` < the caller's `key=value`.
///
/// Both rungs are exercised in one run. `phase` is contested by all three
/// layers and the caller must win; `note` is contested by the target and the
/// router only and the router must win. Asserting them together is what makes
/// this a precedence test rather than two separate "a layer applies" tests —
/// an implementation that simply took the last writer would fail one of them.
///
/// The overlay is read through the *target's* own lifecycle stack, so this also
/// proves the values reach the document's late-bound event surface and not only
/// its body.
#[test]
#[serial(level2_lifecycle_control)]
fn level2_lifecycle_proxy_with_overlay_loses_to_a_caller_set_and_beats_the_target() {
    require_level!(Level::L2, TmuxHarness::available(), "tmux");

    let source_doc = "---\ntitle: router\ninitialize:\n  stack:\n    \
         - action: {action: proxy, target: '@target.md', with: {phase: 2, note: from-router}}\n\
         ---\nrouter body\n";
    let target_doc = "---\ntitle: target\nphase: 0\nnote: from-target\nsuccess:\n  stack:\n    \
         - action: {append_line: ['events.log', 'phase={{ phase }} note={{ note }}']}\n\
         finalize:\n  stack:\n    \
         - action: {append_line: ['events.log', 'target-finalize']}\n---\ntarget body\n";
    let staged = stage_proxy_pair(source_doc, target_doc, true);

    let pane = run_proxy_in_tmux_with_set(&staged, "phase=9", "target-finalize");

    let lines = event_lines(&staged);
    assert!(
        lines.iter().any(|l| l == "phase=9 note=from-router"),
        "the caller's `phase=9` must outrank the router's `with: {{phase: 2}}`, and the \
         router's `note` must outrank the target's authored `from-target`; \
         got {lines:?}; pane:\n{pane}"
    );
    assert!(
        !lines.iter().any(|l| l.contains("phase=2")),
        "a router must never silently replace an explicit caller value; \
         got {lines:?}; pane:\n{pane}"
    );
    assert!(
        !lines.iter().any(|l| l.contains("note=from-target")),
        "the overlay must reach the target's authored frontmatter; \
         got {lines:?}; pane:\n{pane}"
    );

    // `with:` is transient: neither document is rewritten by having carried an
    // overlay. A persisted overlay would silently edit a file the operator
    // never touched.
    assert_eq!(
        fs::read_to_string(&staged.md_file).unwrap(),
        source_doc,
        "the router's bytes must be unchanged; pane:\n{pane}"
    );
    assert_eq!(
        fs::read_to_string(staged.workspace.path().join("target.md")).unwrap(),
        target_doc,
        "the target's bytes must be unchanged; pane:\n{pane}"
    );
}

/// An `initialize` proxy hand-off must (a) announce the redirect with an INFO
/// line and (b) preview the *target* document's body as the agent prompt — not
/// the proxying source's body, which never reaches the agent.
#[test]
#[serial(level2_lifecycle_control)]
fn level2_lifecycle_initialize_proxy_reports_redirect_and_target_prompt() {
    require_level!(Level::L2, TmuxHarness::available(), "tmux");

    let source_doc = "---\ninitialize:\n  stack:\n    \
         - action: {proxy: '@target.md'}\nsuccess:\n  stack:\n    \
         - action: {append_line: ['events.log', 'source-success']}\n---\nSOURCEBODYMARKER\n";
    let target_doc = "---\nsuccess:\n  stack:\n    \
         - action: {append_line: ['events.log', 'target-success']}\n---\nTARGETBODYMARKER\n";
    let staged = stage_proxy_pair(source_doc, target_doc, true);

    let pane = run_in_tmux_for(&staged, "target-success");

    assert!(
        pane.contains("flow control redirected"),
        "an INFO line must announce the proxy hand-off; pane:\n{pane}"
    );
    assert!(
        pane.contains("TARGETBODYMARKER"),
        "the agent prompt must preview the proxied target's body; pane:\n{pane}"
    );
    assert!(
        !pane.contains("SOURCEBODYMARKER"),
        "the proxying source's body must not be previewed (it never runs); pane:\n{pane}"
    );
}

/// A proxied target's composed body must keep its authored line structure. The
/// re-materialization path must set `IncidentalNewlineMode::Preserve` (as
/// `prepare_direct` does); otherwise incidental single newlines are stripped and
/// an author's block-quoted list collapses onto one line in the agent prompt.
#[test]
#[serial(level2_lifecycle_control)]
fn level2_lifecycle_initialize_proxy_target_preserves_line_structure() {
    require_level!(Level::L2, TmuxHarness::available(), "tmux");

    let source_doc = "---\ninitialize:\n  stack:\n    \
         - action: {proxy: '@target.md'}\n---\nsource body\n";
    // A block-quoted unordered list: each item must stay on its own line.
    let target_doc = "---\nsuccess:\n  stack:\n    \
         - action: {append_line: ['events.log', 'target-done']}\n---\n\
         > - ALPHAITEM\n> - BETAITEM\n> - GAMMAITEM\n";
    let staged = stage_proxy_pair(source_doc, target_doc, true);

    let pane = run_in_tmux_for(&staged, "target-done");

    // Each list item renders on its own line, so no single rendered line holds
    // both the first and last item (the collapsed form is
    // "- ALPHAITEM - BETAITEM - GAMMAITEM" on one line).
    let collapsed = pane
        .lines()
        .any(|l| l.contains("ALPHAITEM") && l.contains("GAMMAITEM"));
    assert!(
        !collapsed,
        "the block-quoted list must keep each item on its own line (line structure \
         preserved on re-materialization); pane:\n{pane}"
    );
    assert!(
        pane.contains("ALPHAITEM") && pane.contains("GAMMAITEM"),
        "the agent-prompt preview must show the proxied target's list; pane:\n{pane}"
    );
}

/// A proxied target's lifecycle events must resolve `ctx.*` groups the
/// proxying *source* never referenced. The composition-start `ctx.*` snapshot is
/// demand-driven for the source, so it omits e.g. `ctx.os`; after a proxy the
/// guard drops that snapshot and the executor re-captures per expression. The
/// source references no `ctx.*`; the target's `success` stack references
/// `{{ctx.os}}`, which must render non-empty.
#[test]
#[serial(level2_lifecycle_control)]
fn level2_lifecycle_initialize_proxy_target_resolves_ctx_not_in_source() {
    require_level!(Level::L2, TmuxHarness::available(), "tmux");

    let source_doc = "---\ninitialize:\n  stack:\n    \
         - action: {proxy: '@target.md'}\n---\nsource body\n";
    let target_doc = "---\nsuccess:\n  stack:\n    - action:\n        \
         - {append_line: ['events.log', 'osmarker=[{{ctx.os}}]']}\n        \
         - {append_line: ['events.log', 'target-done']}\n---\ntarget body\n";
    let staged = stage_proxy_pair(source_doc, target_doc, true);

    let pane = run_in_tmux_for(&staged, "target-done");

    let lines = event_lines(&staged);
    let marker = lines
        .iter()
        .find(|l| l.starts_with("osmarker="))
        .unwrap_or_else(|| panic!("target success stack must run; got {lines:?}; pane:\n{pane}"));
    assert_ne!(
        marker, "osmarker=[]",
        "`ctx.os` must resolve in the proxied target even though the source \
         never referenced it (demand-driven snapshot re-capture); got `{marker}`; pane:\n{pane}"
    );
}

/// `initialize.stack` ending in `{proxy: "@target.md"}` hands off to a target
/// whose own `initialize.stack` ends in `skip`. The run exits cleanly with
/// no provider invocation and no source lifecycle beyond the proxy.
#[test]
#[serial(level2_lifecycle_control)]
fn level2_lifecycle_initialize_proxy_respects_target_skip() {
    require_level!(Level::L2, TmuxHarness::available(), "tmux");

    let source_doc = "---\ntitle: proxy source\ninitialize:\n  stack:\n    \
         - action: {proxy: '@target.md'}\nstart:\n  stack:\n    \
         - action: {append_line: ['events.log', 'source-start']}\n---\nsource body\n";
    let target_doc = "---\ntitle: proxy target\ninitialize:\n  stack:\n    \
         - action: {append_line: ['events.log', 'target-init']}\n    \
         - action: skip\n---\ntarget body\n";
    let staged = stage_proxy_pair(source_doc, target_doc, false);
    let pane = run_in_tmux_for(&staged, "target-init");

    let lines = event_lines(&staged);

    assert!(
        lines.iter().any(|l| l == "target-init"),
        "target initialize must partially run before skip; got {lines:?}; pane:\n{pane}"
    );
    assert_eq!(
        lines.iter().filter(|l| **l == "provider-ran").count(),
        0,
        "skip must prevent provider invocation; got {lines:?}; pane:\n{pane}"
    );
    assert!(
        !lines.iter().any(|l| l == "source-start"),
        "source start must not fire; got {lines:?}; pane:\n{pane}"
    );
}

/// `initialize.stack` ending in `{proxy: "@target.md"}` hands off to a target
/// whose own `initialize.stack` ends in `{error: "..."}`. The run routes to the
/// target's failure + finalize and surfaces the error.
#[test]
#[serial(level2_lifecycle_control)]
fn level2_lifecycle_initialize_proxy_respects_target_error() {
    require_level!(Level::L2, TmuxHarness::available(), "tmux");

    let source_doc = "---\ntitle: proxy source\ninitialize:\n  stack:\n    \
         - action: {proxy: '@target.md'}\n---\nsource body\n";
    let target_doc = "---\ntitle: proxy target\ninitialize:\n  stack:\n    \
         - action: {error: 'target init failed'}\nfailure:\n  stack:\n    \
         - action: {append_line: ['events.log', 'target-failure']}\nfinalize:\n  stack:\n    \
         - action: {append_line: ['events.log', 'target-finalize']}\n---\ntarget body\n";
    let staged = stage_proxy_pair(source_doc, target_doc, true);
    let pane = run_in_tmux_for(&staged, "target-finalize");

    let lines = event_lines(&staged);

    assert_eq!(
        lines.iter().filter(|l| **l == "provider-ran").count(),
        0,
        "target initialize error must prevent provider invocation; got {lines:?}; pane:\n{pane}"
    );
    assert!(
        lines.iter().any(|l| l == "target-failure"),
        "target failure must fire after initialize error; got {lines:?}; pane:\n{pane}"
    );
    assert!(
        lines.iter().any(|l| l == "target-finalize"),
        "target finalize must fire after initialize error; got {lines:?}; pane:\n{pane}"
    );
    assert!(
        pane.contains("target init failed"),
        "the explicit error reason must surface in the terminal; pane:\n{pane}"
    );
}

/// `initialize.stack` proxying back and forth between two documents is caught
/// by the cycle/hop-limit guard rather than looping forever.
#[test]
#[serial(level2_lifecycle_control)]
fn level2_lifecycle_initialize_proxy_cycle_guarded() {
    require_level!(Level::L2, TmuxHarness::available(), "tmux");

    let source_doc = "---\ntitle: proxy source\ninitialize:\n  stack:\n    \
         - action: {proxy: '@target.md'}\n---\nsource body\n";
    let target_doc = "---\ntitle: proxy target\ninitialize:\n  stack:\n    \
         - action: {append_line: ['events.log', 'target-init']}\n    \
         - action: {proxy: '@doc.md'}\n---\ntarget body\n";
    let staged = stage_proxy_pair(source_doc, target_doc, false);
    let pane = run_in_tmux_for(&staged, "target-init");

    let lines = event_lines(&staged);

    assert!(
        lines.iter().any(|l| l == "target-init"),
        "target initialize must run before the back-proxy; got {lines:?}; pane:\n{pane}"
    );
    assert!(
        pane.contains("proxy") && (pane.contains("cycle") || pane.contains("hop limit")),
        "the LifecycleProxyCycle error must surface in the terminal; pane:\n{pane}"
    );
}

/// Finding 5 (High): a proxy hand-off whose target then fails **harness-plan
/// parse** before provider launch must route the blocked pre-provider run
/// through the target's `blocked.stack` and `finalize.stack`, with the runtime
/// `err` payload available — not the legacy top-level-only `emit_blocked_or_err`
/// path that skipped the typed stacks, never fired `finalize`, and never exposed
/// `err`.
///
/// The source proxies from `initialize`. On hand-off the target re-parses its
/// own lifecycle (succeeds) and runs its `initialize` (proceeds), then
/// `parse_harness_plan` rejects the target's `timeout: "not a duration"` with a
/// typed `HarnessError`. The fix routes this through `blocked` → `finalize`
/// carrying `err_info`, so the target's `blocked.stack` observes
/// `err.kind`/`err.variant` and the `when: "err"` guard on `finalize.stack`
/// observes `err.msg`. The provider never launches.
#[test]
#[serial(level2_lifecycle_control)]
fn level2_lifecycle_proxy_target_harness_plan_failure_routes_blocked_finalize_with_err() {
    require_level!(Level::L2, TmuxHarness::available(), "tmux");

    let source_doc = "---\ntitle: proxy source\ninitialize:\n  stack:\n    \
         - action: {proxy: '@target.md'}\n---\nsource body\n";
    // `timeout: "not a duration"` passes lifecycle parse + the target's
    // `initialize`, then fails `parse_harness_plan` (a typed HarnessError)
    // before the provider launches. The target's blocked/finalize stacks read
    // the `err` payload routed in by the Finding-5 fix.
    let target_doc = "---\ntitle: proxy target\ntimeout: \"not a duration\"\ninitialize:\n  stack:\n    \
         - action: {append_line: ['events.log', 'target-init']}\nblocked:\n  stack:\n    \
         - action: {append_line: ['events.log', \"{{ 'blocked-err-kind=' + err.kind }}\"]}\n    \
         - action: {append_line: ['events.log', \"{{ 'blocked-err-variant=' + err.variant }}\"]}\nfinalize:\n  stack:\n    \
         - when: \"err\"\n      action: {append_line: ['events.log', \"{{ 'finalize-err-msg=' + err.msg }}\"]}\n    \
         - action: {append_line: ['events.log', 'target-finalize']}\n---\ntarget body\n";
    let staged = stage_proxy_pair(source_doc, target_doc, false);
    let pane = run_in_tmux_for(&staged, "target-finalize");

    let lines = event_lines(&staged);

    // The target's own initialize ran before the harness-plan parse failure.
    assert!(
        lines.iter().any(|l| l == "target-init"),
        "the target initialize must run before the harness-plan parse failure; \
         got {lines:?}; pane:\n{pane}"
    );
    // The provider never launched — this is a blocked pre-provider run.
    assert_eq!(
        lines.iter().filter(|l| **l == "provider-ran").count(),
        0,
        "a pre-provider harness-plan parse failure must not launch the provider; \
         got {lines:?}; pane:\n{pane}"
    );
    // The target's blocked.stack fired and observed the typed err payload. The
    // typed HarnessError (`InvalidTimeout` → `composition.lifecycle_invalid`) is
    // classifiable, so the deprecated `err.kind` alias now reads as its
    // `err.category` facet (`composition`), not the internal Rust type name.
    assert!(
        lines.iter().any(|l| l == "blocked-err-kind=composition"),
        "blocked.stack must fire and observe err.kind='composition' (alias of err.category); \
         got {lines:?}; pane:\n{pane}"
    );
    assert!(
        lines.iter().any(|l| l.starts_with("blocked-err-variant=")),
        "blocked.stack must observe a non-empty err.variant; got {lines:?}; pane:\n{pane}"
    );
    // The target's finalize.stack fired and its `when: "err"` guard was truthy,
    // so err.msg reached the failed-finalize stack.
    assert!(
        lines.iter().any(|l| l.starts_with("finalize-err-msg=") && l.len() > "finalize-err-msg=".len()),
        "finalize.stack `when: err` must be truthy and observe a non-empty err.msg; \
         got {lines:?}; pane:\n{pane}"
    );
    assert!(
        lines.iter().any(|l| l == "target-finalize"),
        "the target finalize must fire; got {lines:?}; pane:\n{pane}"
    );
}

/// A proxy hand-off whose target then fails **target lifecycle parse** surfaces
/// the typed diagnostic and fires no catch event on either document.
///
/// **Rewritten in Phase 7 of `features/2026-07-13-proxy-with`; the change is
/// intentional.** This test previously asserted the opposite — that the
/// *source's* `blocked`/`finalize` stacks fire, because "the guard still holds
/// the proxying document's lifecycle at the point target-lifecycle parse
/// fails". That was the drift, not the contract: a committed hand-off ends the
/// source, so firing its closure afterwards synthesizes a terminal signal for a
/// document that already handed off (R7 clean-handoff), and the target has no
/// lifecycle to catch with precisely because parsing it is what failed. The
/// coordinator now discards the source's config at the commit, so a boot
/// failure in this window surfaces as its own typed diagnostic — the same one
/// invoking the malformed target directly would produce.
#[test]
#[serial(level2_lifecycle_control)]
fn level2_lifecycle_proxy_target_lifecycle_parse_failure_fires_no_catch_events() {
    require_level!(Level::L2, TmuxHarness::available(), "tmux");

    let source_doc = "---\ntitle: proxy source\ninitialize:\n  stack:\n    \
         - action: {append_line: ['events.log', 'source-init']}\n    \
         - action: {proxy: '@target.md'}\nblocked:\n  stack:\n    \
         - action: {append_line: ['events.log', 'source-blocked']}\nfinalize:\n  stack:\n    \
         - action: {append_line: ['events.log', 'source-finalize']}\n---\nsource body\n";
    // The target's `success.stack` carries a malformed item (no `action` key),
    // which `parse_lifecycle_config` rejects with a typed CompositionError
    // (`LifecycleStackInvalidShape`) during the target's bootstrap read.
    let target_doc = "---\ntitle: proxy target\nsuccess:\n  stack:\n    \
         - when: \"true\"\n---\ntarget body\n";
    let staged = stage_proxy_pair(source_doc, target_doc, false);
    let pane = run_in_tmux_until_exit(&staged);

    let lines = event_lines(&staged);

    assert!(
        lines.iter().any(|l| l == "source-init"),
        "the source initialize must fire before the hand-off; got {lines:?}; pane:\n{pane}"
    );
    assert_eq!(
        lines.iter().filter(|l| **l == "provider-ran").count(),
        0,
        "a target lifecycle parse failure must not launch the provider; \
         got {lines:?}; pane:\n{pane}"
    );
    assert!(
        !lines.iter().any(|l| l == "source-blocked"),
        "the source handed off; its blocked stack must not fire for the \
         target's parse failure; got {lines:?}; pane:\n{pane}"
    );
    assert!(
        !lines.iter().any(|l| l == "source-finalize"),
        "the source handed off; its finalize must not be synthesized after the \
         fact; got {lines:?}; pane:\n{pane}"
    );
    assert!(
        pane.contains("stack"),
        "the typed lifecycle-stack diagnostic must be rendered; pane:\n{pane}"
    );
}

/// Review-6: a **post-`start`** setup failure must reach `failure` AND
/// `finalize` with `err` available — not just the legacy
/// `LifecycleRunGuard::drop` path, which never runs the typed stacks nor emits
/// `finalize`.
///
/// ## Post-start error injected and why it is genuinely triggerable
///
/// The frontmatter declares a runaway `exit_expressions` entry of `kind: regex`
/// whose pattern `[` is an unclosed character class. `start` and the pre-flight
/// pre-checks pass (the regex is not touched there), so the lifecycle reaches
/// `start`. The provider launch is then constructed and `execute_harness_attempt`
/// runs `resolve_guard_inputs` → `validate_exit_expressions`, which compiles the
/// regex and aborts with a typed `ConfigValidation` error **before any child is
/// spawned** (`runaway_guard.rs`'s "present-but-invalid = abort" contract). That
/// `Err` surfaces at the post-spawn `attempt_result?` site in
/// `run_harness_loop`, which the fix routes through
/// `emit_failure_finalize_with_err` (terminal is always `Failure` because
/// pre-flight already passed). The site wraps the underlying error via
/// `from_action_failure("harness_attempt", ...)`, so the stacks observe
/// `err.kind = "LifecycleAction"` / `err.variant = "harness_attempt"`.
///
/// Asserts the ordered markers prove: `start` fired → the provider never ran
/// (pre-spawn error) → `failure.stack` fired with the `err` interpolated →
/// `finalize.stack` fired with `err.msg`.
#[test]
#[serial(level2_lifecycle_control)]
fn level2_lifecycle_post_start_setup_failure_routes_failure_finalize_with_err() {
    require_level!(Level::L2, TmuxHarness::available(), "tmux");

    // `exit_expressions` with a malformed regex (`[` = unterminated character
    // class) validates-and-aborts inside `execute_harness_attempt`, after
    // `start` has fired and before the provider spawns.
    let doc = r#"---
title: post-start setup failure
exit_expressions:
  - patterns:
      - "["
    kind: regex
start:
  stack:
    - action: {append_line: ["events.log", "start"]}
failure:
  stack:
    - action: {append_line: ["events.log", "{{ 'failure-kind=' + err.kind }}"]}
    - action: {append_line: ["events.log", "{{ 'failure-variant=' + err.variant }}"]}
finalize:
  stack:
    - when: "err"
      action: {append_line: ["events.log", "{{ 'finalize-msg=' + err.msg }}"]}
    - action: {append_line: ["events.log", "finalize"]}
---
Body
"#;
    let staged = stage(doc);
    let pane = run_in_tmux(&staged);

    let lines = event_lines(&staged);

    // `start` fired before the setup failure.
    assert!(
        lines.iter().any(|l| l == "start"),
        "start must fire before the post-start setup failure; got {lines:?}; pane:\n{pane}"
    );
    // The error happens before the child spawns, so the provider never runs.
    assert_eq!(
        lines.iter().filter(|l| **l == "provider-ran").count(),
        0,
        "a pre-spawn guard-validation failure must not launch the provider; \
         got {lines:?}; pane:\n{pane}"
    );
    // The failure stack fired with the typed err payload (NOT the legacy
    // drop path, which never runs the failure stack). The post-spawn
    // `attempt_result?` site wraps the underlying error via
    // `from_action_failure("harness_attempt", ...)`, so err.kind is
    // `LifecycleAction` and err.variant is the `harness_attempt` verb.
    assert!(
        lines.iter().any(|l| l == "failure-kind=LifecycleAction"),
        "failure.stack must fire and observe err.kind='LifecycleAction'; \
         got {lines:?}; pane:\n{pane}"
    );
    assert!(
        lines.iter().any(|l| l == "failure-variant=harness_attempt"),
        "failure.stack must observe err.variant='harness_attempt'; \
         got {lines:?}; pane:\n{pane}"
    );
    // The finalize stack fired and its `when: "err"` guard was truthy.
    assert!(
        lines
            .iter()
            .any(|l| l.starts_with("finalize-msg=") && l.len() > "finalize-msg=".len()),
        "finalize.stack `when: err` must be truthy and observe a non-empty err.msg; \
         got {lines:?}; pane:\n{pane}"
    );
    assert!(
        lines.iter().any(|l| l == "finalize"),
        "the terminal finalize must fire; got {lines:?}; pane:\n{pane}"
    );
    // Ordering: start precedes failure precedes finalize.
    let pos = |needle: &str| lines.iter().position(|l| l == needle);
    let start_pos = pos("start").expect("start marker");
    let failure_pos = lines
        .iter()
        .position(|l| l.starts_with("failure-kind="))
        .expect("failure marker");
    let finalize_pos = pos("finalize").expect("finalize marker");
    assert!(
        start_pos < failure_pos && failure_pos < finalize_pos,
        "expected start → failure → finalize ordering; got {lines:?}; pane:\n{pane}"
    );
}

/// Run a compose in a real tmux pane and settle on either `expected_lines`
/// side-effect markers or a stable count (the run finished early). Unlike
/// [`run_in_tmux_for`], which breaks on the first sighting of one marker, this
/// cannot stop mid-run on a document whose terminal events fire once **per
/// iteration** — and it still returns when a buggy run produces fewer markers
/// than expected, which is what makes an equivalence mismatch observable rather
/// than a timeout.
fn run_until_settled(staged: &Staged, expected_lines: usize) -> String {
    static SEQ: AtomicU32 = AtomicU32::new(0);
    let seq = SEQ.fetch_add(1, Ordering::Relaxed);

    let session = format!("biscuit_l2_lcequiv_{}_{seq}", std::process::id());
    let shell = biscuit_test_harness::detect_shell();
    let spawned = std::process::Command::new("tmux")
        .args([
            "new-session",
            "-d",
            "-s",
            &session,
            "-x",
            "200",
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

    let claudine = env!("CARGO_BIN_EXE_claudine");
    let sentinel = format!("L2_EQUIV_DONE_{seq}");
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

    let deadline = Instant::now() + Duration::from_secs(40);
    let mut last_count = 0usize;
    let mut stable_since: Option<Instant> = None;
    while Instant::now() < deadline {
        let count = event_lines(staged).len();
        if count >= expected_lines {
            std::thread::sleep(Duration::from_millis(150));
            break;
        }
        if count == last_count {
            match stable_since {
                Some(since) if since.elapsed() >= Duration::from_millis(1200) => break,
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

/// A looping target document. Three iterations (`phase` 1, 2, 3), one
/// `initialize`, and a per-iteration `finalize` stamping the live `phase` so the
/// mutation sequence is observable from the side-effect log.
const EQUIV_LOOP_TARGET: &str = r#"---
title: proxy target loop
phase: 1
loop:
  until: "phase > 2"
  action: "increment(phase)"
  max: 10
initialize:
  stack:
    - action: {append_line: ["events.log", "target-init"]}
finalize:
  stack:
    - action: {append_line: ["events.log", "target-finalize:{{phase}}"]}
---
target body phase {{phase}}
"#;

/// A router with no `loop:` of its own that hands off to the looping target at
/// `initialize`.
const EQUIV_ROUTER: &str = r#"---
title: proxy router
initialize:
  stack:
    - action: {proxy: "@target.md"}
---
router body
"#;

/// **The motivating bug** (`features/2026-07-13-proxy-with/spec.md`): a proxied
/// target must execute exactly as it does when invoked directly.
///
/// A router with no `loop:` proxies at `initialize` to a looping target. Loop
/// vs single is decided at `cli/src/commands/compose/prep.rs` from the
/// **router's** frontmatter, before the router's `initialize` proxy fires — so
/// the routed run treats the target as a single-run document and executes one
/// provider attempt, while the direct run executes all three iterations.
///
/// ## Why this is `#[ignore]`d
///
/// It fails on the Phase 1 baseline **by design** — it is the reproduction, not
/// a regression guard. Loop ownership only follows document identity once the
/// coordinator (Phase 6) and loop-ownership move (Phase 10) land. **Phase 10
/// re-enables it**; it is the headline acceptance signal for the feature.
///
/// Deterministic by construction: a fake provider, a self-contained temporary
/// fixture, and no live Claude/Codex/Gemini service. The shipped
/// `prompts/implement.md` command remains a manual smoke case, never a CI
/// dependency.
#[test]
#[ignore = "reproduction of the motivating bug; Phase 10 (loop ownership follows \
            document identity) makes this pass and re-enables it"]
#[serial(level2_lifecycle_control)]
fn level2_lifecycle_initialize_proxy_to_looping_target_matches_direct_run() {
    require_level!(Level::L2, TmuxHarness::available(), "tmux");

    // 1 target-init + 3 provider-ran + 3 target-finalize = 7 markers per run.
    const EXPECTED_MARKERS: usize = 7;

    // Direct: the looping target IS the invoked document.
    let direct = stage_proxy_pair(EQUIV_LOOP_TARGET, EQUIV_LOOP_TARGET, true);
    let direct_pane = run_until_settled(&direct, EXPECTED_MARKERS);
    let direct_lines = event_lines(&direct);

    // Routed: the router is invoked and proxies to the same target at initialize.
    let routed = stage_proxy_pair(EQUIV_ROUTER, EQUIV_LOOP_TARGET, true);
    let routed_pane = run_until_settled(&routed, EXPECTED_MARKERS);
    let routed_lines = event_lines(&routed);

    let phases = |lines: &[String]| -> Vec<String> {
        lines
            .iter()
            .filter(|l| l.starts_with("target-finalize:"))
            .cloned()
            .collect()
    };
    let count = |lines: &[String], needle: &str| lines.iter().filter(|l| *l == needle).count();

    // The direct run is the contract the routed run must match. Assert it first
    // so a broken fixture is not misread as a routing bug.
    assert_eq!(
        phases(&direct_lines),
        vec!["target-finalize:1", "target-finalize:2", "target-finalize:3"],
        "fixture check: the target loops three times when invoked directly; \
         got {direct_lines:?}; pane:\n{direct_pane}"
    );

    assert_eq!(
        count(&routed_lines, "provider-ran"),
        count(&direct_lines, "provider-ran"),
        "iteration count must not depend on the route: the proxied target ran \
         {} provider attempts but the direct target ran {}; routed {routed_lines:?}; \
         pane:\n{routed_pane}",
        count(&routed_lines, "provider-ran"),
        count(&direct_lines, "provider-ran"),
    );
    assert_eq!(
        phases(&routed_lines),
        phases(&direct_lines),
        "the target's phase mutations must not depend on the route; \
         routed {routed_lines:?}; pane:\n{routed_pane}"
    );
    assert_eq!(
        count(&routed_lines, "target-init"),
        1,
        "the target's initialize fires exactly once on the routed run; \
         got {routed_lines:?}; pane:\n{routed_pane}"
    );
    assert_eq!(
        count(&routed_lines, "target-init"),
        count(&direct_lines, "target-init"),
        "the target initialize count must not depend on the route; \
         routed {routed_lines:?}; direct {direct_lines:?}"
    );
}

// ── Phase 7: staged bootstrap, narrow safety gate, stabilized reread ────────

/// A fake `goose` that records the prompt it was handed, so a test can assert
/// which read of the document produced the delivered body.
///
/// Goose takes the prompt on argv, not stdin, so both channels are recorded
/// onto one line and the caller matches against it.
fn write_prompt_recording_goose(bin_dir: &Path, events_log: &Path) {
    write_executable(
        &bin_dir.join("goose"),
        &format!(
            "#!/bin/sh\nstdin=$(cat)\nprintf 'prompt:%s %s\\n' \"$stdin\" \"$*\" >> {log}\n\
             printf 'provider-ran\\n' >> {log}\nexit 0\n",
            log = events_log.display(),
        ),
    );
}

/// Run `claudine compose --goose <doc>` in tmux and block until the shell
/// sentinel lands, rather than until an `events.log` marker does.
///
/// The gate tests below assert on what the run *refused* to do, so there is no
/// success marker to wait for and waiting on `events.log` would burn the whole
/// deadline on every run.
fn run_in_tmux_until_exit(staged: &Staged) -> String {
    static SEQ: AtomicU32 = AtomicU32::new(0);
    let seq = SEQ.fetch_add(1, Ordering::Relaxed);

    let session = format!("biscuit_l2_lcctl_exit_{}_{seq}", std::process::id());
    let shell = biscuit_test_harness::detect_shell();
    let spawned = std::process::Command::new("tmux")
        .args([
            "new-session",
            "-d",
            "-s",
            &session,
            "-x",
            "200",
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

    let claudine = env!("CARGO_BIN_EXE_claudine");
    let sentinel = format!("L2_CTL_EXIT_{seq}");
    let cmd = format!(
        "cd {ws} && NO_COLOR='1' HOME='{home}' PATH='{path}' {claudine} compose --goose {md} ; echo {sentinel}",
        ws = staged.workspace.path().display(),
        home = staged.workspace.path().display(),
        path = augmented_path(&staged.bin_dir).to_string_lossy(),
        md = staged.md_file.display(),
    );
    harness
        .send_command_with_env(&cmd, &[])
        .expect("send compose command");

    let deadline = Instant::now() + Duration::from_secs(40);
    let mut pane = String::new();
    while Instant::now() < deadline {
        pane = harness.capture().map(|f| f.plain).unwrap_or_default();
        // Two occurrences: the echoed command line and the shell's output.
        if pane.matches(sentinel.as_str()).count() >= 2 {
            break;
        }
        std::thread::sleep(Duration::from_millis(150));
    }
    kill_session_by_name(&session);
    pane
}

/// The narrow safety gate: a proxy target's `initialize` shell command is
/// approved **before** the evaluator dispatches it.
///
/// The target's `initialize` runs before the target's full pre-flight audit
/// can — the audit has to read the document `initialize` may rewrite. That
/// ordering must never mean "execute unapproved shell": the gate approves
/// every command `initialize` could select first, on its own.
///
/// `rm` is builtin-blacklisted, so a gate that ran would refuse it. Before the
/// staged boot the proxy route audited no lifecycle surface at all, so this
/// command reached `SystemShellRunner` and deleted the sentinel.
#[test]
#[serial(level2_lifecycle_control)]
fn level2_lifecycle_proxy_target_initialize_shell_is_gated_before_dispatch() {
    require_level!(Level::L2, TmuxHarness::available(), "tmux");

    let source_doc = "---\ntitle: gate source\ninitialize:\n  stack:\n    \
         - action: {append_line: ['events.log', 'source-init']}\n    \
         - action: {proxy: '@target.md'}\n---\nsource body\n";
    let target_doc = "---\ntitle: gate target\ninitialize:\n  stack:\n    \
         - action: {shell: 'rm sentinel.txt'}\nsuccess:\n  stack:\n    \
         - action: {append_line: ['events.log', 'target-success']}\n---\ntarget body\n";
    let staged = stage_proxy_pair(source_doc, target_doc, true);
    let sentinel = staged.workspace.path().join("sentinel.txt");
    fs::write(&sentinel, "intact").unwrap();

    let pane = run_in_tmux_until_exit(&staged);
    let lines = event_lines(&staged);

    assert!(
        lines.iter().any(|l| l == "source-init"),
        "the source initialize must fire before the hand-off; got {lines:?}; pane:\n{pane}"
    );
    assert!(
        sentinel.exists(),
        "the blacklisted `initialize` shell command must be refused by the narrow \
         gate before dispatch — it deleted the sentinel instead; pane:\n{pane}"
    );
    assert!(
        !lines.iter().any(|l| l == "provider-ran"),
        "a refused initialize command must stop the run before the provider \
         launches; got {lines:?}; pane:\n{pane}"
    );
    assert!(
        !lines.iter().any(|l| l == "target-success"),
        "no target terminal event may fire when the boot never completed; \
         got {lines:?}; pane:\n{pane}"
    );
}

/// The full post-stabilization audit covers a proxy target's later lifecycle
/// surfaces too — not only the ones the narrow gate scoped.
///
/// The gate deliberately skips `success`, so if the audit that follows the
/// stabilized reread did not run, this blacklisted `success` command would
/// reach the shell runner after the provider exited 0.
#[test]
#[serial(level2_lifecycle_control)]
fn level2_lifecycle_proxy_target_later_event_shell_is_audited_after_stabilization() {
    require_level!(Level::L2, TmuxHarness::available(), "tmux");

    let source_doc = "---\ntitle: audit source\ninitialize:\n  stack:\n    \
         - action: {append_line: ['events.log', 'source-init']}\n    \
         - action: {proxy: '@target.md'}\n---\nsource body\n";
    let target_doc = "---\ntitle: audit target\ninitialize:\n  stack:\n    \
         - action: {append_line: ['events.log', 'target-init']}\nsuccess:\n  stack:\n    \
         - action: {shell: 'rm sentinel.txt'}\n---\ntarget body\n";
    let staged = stage_proxy_pair(source_doc, target_doc, true);
    let sentinel = staged.workspace.path().join("sentinel.txt");
    fs::write(&sentinel, "intact").unwrap();

    let pane = run_in_tmux_until_exit(&staged);
    let lines = event_lines(&staged);

    assert!(
        lines.iter().any(|l| l == "target-init"),
        "the target's own initialize must fire; got {lines:?}; pane:\n{pane}"
    );
    assert!(
        sentinel.exists(),
        "the blacklisted `success` shell command must be refused by the full \
         post-stabilization audit; pane:\n{pane}"
    );
    assert!(
        !lines.iter().any(|l| l == "provider-ran"),
        "the audit runs before the provider launches; got {lines:?}; pane:\n{pane}"
    );
}

/// The stabilized reread: a proxy target that mutates its own frontmatter from
/// `initialize` delivers the **mutated** body to the provider.
///
/// The bootstrap read composed `phase: authored`; `initialize` then rewrote the
/// document on disk. Without the reread the run would deliver the body composed
/// before its own `initialize` ran — the document as it was, not as it is.
#[test]
#[serial(level2_lifecycle_control)]
fn level2_lifecycle_proxy_target_rereads_after_initialize_mutation() {
    require_level!(Level::L2, TmuxHarness::available(), "tmux");

    let source_doc = "---\ntitle: reread source\ninitialize:\n  stack:\n    \
         - action: {proxy: '@target.md'}\n---\nsource body\n";
    let target_doc = "---\ntitle: reread target\nphase: authored\ninitialize:\n  stack:\n    \
         - action: {append_line: ['events.log', 'target-init']}\n    \
         - action: {set_frontmatter: ['target.md', 'phase', 'stabilized']}\nfinalize:\n  stack:\n    \
         - action: {append_line: ['events.log', 'target-finalize']}\n---\nphase-is-{{ phase }}\n";
    let staged = stage_proxy_pair(source_doc, target_doc, true);
    write_prompt_recording_goose(&staged.bin_dir, &staged.events_log);

    let pane = run_in_tmux_for(&staged, "target-finalize");
    let lines = event_lines(&staged);

    assert!(
        lines
            .iter()
            .any(|l| l.starts_with("prompt:") && l.contains("phase-is-stabilized")),
        "the delivered prompt must come from the reread of the stabilized \
         target, not from the pre-initialize bootstrap read; got {lines:?}; pane:\n{pane}"
    );
    assert!(
        !lines.iter().any(|l| l.contains("phase-is-authored")),
        "the pre-initialize bootstrap body must never reach the provider; \
         got {lines:?}; pane:\n{pane}"
    );
    assert_eq!(
        lines.iter().filter(|l| **l == "target-init").count(),
        1,
        "the reread must not fire `initialize` a second time; got {lines:?}; pane:\n{pane}"
    );
}
