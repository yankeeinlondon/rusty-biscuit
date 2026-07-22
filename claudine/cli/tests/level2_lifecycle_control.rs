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

/// A fake `codex` that mirrors [`write_resumable_claude`] on Codex's own wire:
/// `thread.started` carries the session id, the first invocation fails, and only
/// the `codex exec resume <id>` argv reaches the resume branch.
///
/// Codex is used for the MCP row because Claude declines runtime MCP injection
/// outright (`--mcp` errors before the first launch), while Codex accepts it and
/// still supports first-class resume.
fn write_resumable_codex(
    bin_dir: &Path,
    events_log: &Path,
    session_id: &str,
    follow_up: &str,
) {
    write_executable(
        &bin_dir.join("codex"),
        &format!(
            r#"#!/bin/sh
prompt=$(cat)
printf 'provider-ran\n' >> {log}
case " $* " in
  *" resume {session_id} "*)
    printf 'resume-session-ok\n' >> {log}
    case "$prompt" in
      *"{follow_up}"*) printf 'follow-up-ok\n' >> {log} ;;
      *) printf 'follow-up-missing:%s\n' "$prompt" >> {log} ;;
    esac
    printf '%s\n' '{{"type":"thread.started","thread_id":"{session_id}"}}'
    printf '%s\n' '{{"type":"item.completed","item":{{"type":"agent_message","text":"resumed ok"}}}}'
    exit 0
    ;;
  *)
    printf 'initial-prompt-ok\n' >> {log}
    printf '%s\n' '{{"type":"thread.started","thread_id":"{session_id}"}}'
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

/// Like [`run_provider_in_tmux_for`] but inserts extra claudine flags between
/// the provider flag and the document (e.g. `--append-system-prompt '…'`). Used
/// to prove a resume stays compatible even when a non-whitelisted launch flag is
/// present on the opening attempt but dropped from the resume argv.
fn run_provider_with_flags(
    staged: &Staged,
    provider_flag: &str,
    extra_flags: &str,
    done_marker: &str,
) -> String {
    run_provider_with_ambient_env(staged, provider_flag, extra_flags, done_marker, &[])
}

/// [`run_provider_with_flags`] with extra variables exported into the wrapper's
/// **own** process environment, ahead of any provider allow-list.
///
/// Credential rows need this: what a rebuilt provider is entitled to is decided
/// against the ambient environment Claudine was launched in, not against
/// anything the document can express.
fn run_provider_with_ambient_env(
    staged: &Staged,
    provider_flag: &str,
    extra_flags: &str,
    done_marker: &str,
    ambient: &[(&str, &str)],
) -> String {
    static SEQ: AtomicU32 = AtomicU32::new(0);
    let seq = SEQ.fetch_add(1, Ordering::Relaxed);

    let session = format!("biscuit_l2_lcctl_flags_{}_{seq}", std::process::id());
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
    let sentinel = format!("L2_CTL_DONE_{seq}");
    let mut env_prefix = format!(
        "NO_COLOR='1' HOME='{home}' PATH='{path}' ",
        home = staged.workspace.path().display(),
        path = augmented_path(&staged.bin_dir).to_string_lossy(),
    );
    for (key, value) in ambient {
        env_prefix.push_str(&format!("{key}='{value}' "));
    }
    let cmd = format!(
        "cd {ws} && {env_prefix}{claudine} compose {provider_flag} {extra_flags} {md} ; echo {sentinel}",
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
    let invocation_path = staged
        .md_file
        .strip_prefix(staged.workspace.path())
        .unwrap_or(&staged.md_file);
    let cmd = format!(
        "cd {ws} && {env_prefix}{claudine} compose --goose {md} {setters} ; echo {sentinel}",
        ws = staged.workspace.path().display(),
        md = invocation_path.display(),
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
    run_provider_await_exit(staged, "--goose")
}

/// Like [`run_compose_await_exit`] but for an arbitrary provider flag, and with
/// `MODEL` explicitly emptied.
///
/// Emptying `MODEL` matters for the resume-compatibility rows: model resolution
/// consults the generic `MODEL` environment variable *before* frontmatter
/// (`select.rs` precedence step 3), so an ambient `MODEL` in the developer's
/// shell would outrank the document's `model:` and silently neutralize a test
/// that refreshes it. The resolver skips empty values, so `MODEL=''` restores
/// frontmatter precedence without needing `env -u`.
fn run_provider_await_exit(staged: &Staged, provider_flag: &str) -> String {
    run_compose_await_exit_with_args(staged, provider_flag)
}

/// Like [`run_provider_await_exit`] but taking the whole pre-document argument
/// string, so a row can pass **no** provider flag (letting frontmatter `agent:`
/// select the provider, which is what makes the provider facet movable) or add
/// flags such as `--yolo` / `--append-system-prompt`.
fn run_compose_await_exit_with_args(staged: &Staged, extra_args: &str) -> String {
    run_compose_await_exit_on_path(
        staged,
        extra_args,
        &augmented_path(&staged.bin_dir).to_string_lossy(),
    )
}

/// A `PATH` carrying the row's fakes and the POSIX tools they need — and
/// nothing else.
///
/// [`augmented_path`] appends the developer's own `PATH`, which is right for a
/// row whose fakes only need to *win*. It is wrong for a row whose contract is
/// that a provider is **absent**: a real `gemini` in the developer's
/// `~/.local/bin` would make such a row silently vacuous, and green or red would
/// depend on the machine. `/usr/bin:/bin` holds `sh`, `env`, `sed`, `basename`
/// and no agentic CLI.
fn isolated_path(bin_dir: &Path) -> String {
    format!("{}:/usr/bin:/bin", bin_dir.display())
}

/// [`run_compose_await_exit_with_args`] with the child `PATH` spelled out.
fn run_compose_await_exit_on_path(staged: &Staged, extra_args: &str, path: &str) -> String {
    run_compose_await_exit_redirected(staged, extra_args, path, "")
}

/// `2>&1 | cat`: run compose with **stderr on a pipe** rather than the pane's
/// TTY, while still showing everything it writes.
///
/// Provider selection is TTY-gated — with a terminal on stderr an unresolvable
/// `agent:` opens the interactive picker instead of aborting — so a row
/// comparing the *no-operator* diagnostic has to take the operator away. A
/// retry has none by construction; this is how the direct arm is put in the
/// same position.
const NON_TTY_STDERR: &str = " 2>&1 | cat";

/// [`run_compose_await_exit_on_path`] with a shell redirection appended to the
/// compose command.
fn run_compose_await_exit_redirected(
    staged: &Staged,
    extra_args: &str,
    path: &str,
    redirect: &str,
) -> String {
    static SEQ: AtomicU32 = AtomicU32::new(0);
    let seq = SEQ.fetch_add(1, Ordering::Relaxed);
    let provider_flag = extra_args;

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
        "NO_COLOR='1' MODEL='' HOME='{home}' PATH='{path}' ",
        home = staged.workspace.path().display(),
    );
    let cmd = format!(
        "cd {ws} && {env_prefix}{claudine} compose {provider_flag} {md}{redirect} ; echo {sentinel}",
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

// ── review-8 finding 2: the retried attempt's actual launch bundle ───────────
//
// R8 requires a retry's refreshed body, lifecycle, context, and launch plan to
// come from one coherent prepared document, and a retry opens a *fresh* session,
// so unlike `resume` there is nothing to refuse — it must simply launch under
// the refreshed plan. The rows below drive that end to end: a document's
// `failure` stack mutates the very input a launch facet derives from and then
// `retry`s, and each row asserts what the *retried provider process itself*
// recorded — which binary ran, which flags reached its argv, which MCP
// configuration was injected, and what its environment carried.
//
// Every row authors its provider in frontmatter rather than passing a
// `--provider` flag: explicit CLI intent stays authoritative under R6, so a
// flag would pin the facet and make the assertion vacuous.
//
// Before the re-entrant launch-plan builder landed, all of these launched the
// retried attempt through the invocation's frozen argv/profile/MCP bundle.

/// A recorder that reports the launch bundle it actually received.
///
/// Records the binary name, the *flag-shaped* argv entries, and the launch
/// environment. Only arguments beginning with `-` are recorded: providers like
/// Goose take the prompt body on argv, and a multi-line body would otherwise
/// corrupt the line-oriented log this fixture is read back with.
///
/// `exit_code` lets a row make the first attempt fail (driving `failure` →
/// `retry`) and the retried attempt succeed, or fail both to observe two
/// launches under one budget.
///
/// Stdin is drained only when it is not a terminal. A row that flips
/// `interactive:` gets the pane's tty on the retried attempt, where an
/// unconditional `cat` would block until the operator sent EOF — which, in a
/// non-interactive test session, is never.
fn write_launch_recorder(bin_dir: &Path, slug: &str, events_log: &Path, exit_code: i32) {
    write_executable(
        &bin_dir.join(slug),
        &format!(
            "#!/bin/sh\nif [ ! -t 0 ]; then cat > /dev/null 2>&1; fi\n\
             printf 'launched-binary=%s\\n' \"$(basename \"$0\")\" >> {log}\n\
             flags=\nallowed=none\nprev=\n\
             for a in \"$@\"; do\n  \
             case \"$a\" in -*) flags=\"$flags $a\";; esac\n  \
             if [ \"$prev\" = '--allowed-mcp-server-names' ]; then allowed=\"$a\"; fi\n  \
             prev=\"$a\"\ndone\n\
             printf 'flags=%s\\n' \"$flags\" >> {log}\n\
             printf 'mcp-allowed=%s\\n' \"$allowed\" >> {log}\n\
             printf 'env-yolo=%s\\n' \"${{YOLO:-unset}}\" >> {log}\n\
             printf 'provider-ran\\n' >> {log}\nexit {code}\n",
            log = events_log.display(),
            code = exit_code,
        ),
    );
}

/// Stage a document plus a git repo and minimal config, with no providers
/// installed yet — each row writes the recorders it needs.
fn stage_launch_recording(doc: &str) -> Staged {
    let workspace = tempdir().unwrap();
    let bin_dir = workspace.path().join("bin");
    fs::create_dir_all(&bin_dir).unwrap();
    seed_minimal_config(workspace.path());
    assert!(init_git_repo(workspace.path()), "git init failed");
    let events_log = workspace.path().join("events.log");
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

/// A fake provider that records its **complete** argv and environment, one
/// attempt per `begin-attempt`/`end-attempt` pair.
///
/// [`write_launch_recorder`] records only a flag-shaped projection, which cannot
/// distinguish "the retry dropped the requested `--output`" from "the recorder
/// did not look for it", and says nothing at all about the child environment.
/// Review-9 finding 2 is about both, so these rows assert on everything the
/// process actually received.
fn write_full_launch_recorder(bin_dir: &Path, slug: &str, events_log: &Path, exit_code: i32) {
    write_full_launch_recorder_with_exit(bin_dir, slug, events_log, &format!("exit {exit_code}"));
}

/// [`write_full_launch_recorder`] with the exit decision spelled out, so a row
/// can make the *same* binary fail once and then succeed — which is how a facet
/// other than the provider is moved across a retry.
///
/// `exit_script` runs after the recording and must terminate the script.
fn write_full_launch_recorder_with_exit(
    bin_dir: &Path,
    slug: &str,
    events_log: &Path,
    exit_script: &str,
) {
    write_executable(
        &bin_dir.join(slug),
        &format!(
            // Probes are not attempts. Claudine runs the binary to read its
            // version during preflight and, for providers with dynamic model
            // listing, `<binary> models` during model validation; recording
            // either as a launch would make the attempt sequence depend on
            // which provider the row happens to open with.
            "#!/bin/sh\ncase \"$1\" in --version|-V|-v|version|models) exit 0;; esac\n\
             if [ ! -t 0 ]; then cat > /dev/null 2>&1; fi\n\
             printf 'begin-attempt=%s\\n' \"$(basename \"$0\")\" >> {log}\n\
             for a in \"$@\"; do printf 'argv=%s\\n' \"$a\" >> {log}; done\n\
             env | sed 's/^/env=/' >> {log}\n\
             printf 'end-attempt\\n' >> {log}\n\
             printf 'provider-ran\\n' >> {log}\n{exit_script}\n",
            log = events_log.display(),
        ),
    );
}

/// One provider process as [`write_full_launch_recorder`] observed it.
struct RecordedAttempt {
    binary: String,
    argv: Vec<String>,
    env: Vec<(String, String)>,
}

impl RecordedAttempt {
    fn has_flag(&self, flag: &str) -> bool {
        self.argv.iter().any(|arg| arg == flag)
    }

    /// True when `flag` appears immediately followed by `value` — the shape a
    /// `FlagValue` selector renders, where the flag alone proves nothing.
    fn has_pair(&self, flag: &str, value: &str) -> bool {
        self.argv
            .windows(2)
            .any(|pair| pair[0] == flag && pair[1] == value)
    }

    fn env_value(&self, key: &str) -> Option<&str> {
        self.env
            .iter()
            .find(|(k, _)| k == key)
            .map(|(_, v)| v.as_str())
    }
}

/// Parse the recorder's log into one entry per provider process, in launch order.
fn recorded_attempts(staged: &Staged) -> Vec<RecordedAttempt> {
    let mut attempts: Vec<RecordedAttempt> = Vec::new();
    for line in event_lines(staged) {
        if let Some(binary) = line.strip_prefix("begin-attempt=") {
            attempts.push(RecordedAttempt {
                binary: binary.to_string(),
                argv: Vec::new(),
                env: Vec::new(),
            });
            continue;
        }
        let Some(current) = attempts.last_mut() else {
            continue;
        };
        if let Some(arg) = line.strip_prefix("argv=") {
            current.argv.push(arg.to_string());
        } else if let Some(entry) = line.strip_prefix("env=")
            && let Some((key, value)) = entry.split_once('=')
        {
            current.env.push((key.to_string(), value.to_string()));
        }
    }
    attempts
}

/// The `launched-binary=` values in recording order.
fn launched_binaries(staged: &Staged) -> Vec<String> {
    event_lines(staged)
        .into_iter()
        .filter_map(|line| line.strip_prefix("launched-binary=").map(str::to_string))
        .collect()
}

/// **Review-8 finding 2 — provider, profile, and binary.** A document whose
/// `failure` stack rewrites its own `agent:` and then retries launches the
/// retried attempt through the *new* provider's binary and profile.
///
/// The first attempt runs `goose` (which exits non-zero, driving `failure`); the
/// stack rewrites `agent: gemini` and retries; the second attempt must be a real
/// `gemini` process. The old behavior re-ran `goose` — the rebuilt identity said
/// "gemini" while the invocation's frozen bundle spawned Goose.
#[test]
#[serial(level2_lifecycle_control)]
fn level2_lifecycle_retry_launches_the_refreshed_provider_binary() {
    require_level!(Level::L2, TmuxHarness::available(), "tmux");

    let doc = r#"---
title: retry switches provider
agent: goose
failure:
  stack:
    - action:
        - {append_line: ["events.log", "failure"]}
        - {set_frontmatter: ["doc.md", "agent", "gemini"]}
    - action: {retry: 1}
finalize:
  stack:
    - action: {append_line: ["events.log", "finalize"]}
---
Body
"#;
    let staged = stage_launch_recording(doc);
    write_launch_recorder(&staged.bin_dir, "goose", &staged.events_log, 1);
    write_launch_recorder(&staged.bin_dir, "gemini", &staged.events_log, 0);

    let pane = run_compose_await_exit_with_args(&staged, "");

    let refreshed = fs::read_to_string(&staged.md_file).unwrap();
    assert!(
        refreshed.contains("agent: gemini"),
        "fixture check: the failure stack must have rewritten `agent:` before the \
         retry; document:\n{refreshed}"
    );
    assert_eq!(
        launched_binaries(&staged),
        vec!["goose".to_string(), "gemini".to_string()],
        "the retried attempt must spawn the refreshed document's provider binary, \
         not the invocation's; pane:\n{pane}"
    );
}

/// A document that runs `agent: {from}`, rewrites its own `agent:` to `{to}` in
/// the `failure` stack, and retries once. `extra` is spliced into the
/// frontmatter above the stack.
fn provider_switch_retry_doc(from: &str, to: &str, extra: &str) -> String {
    format!(
        "---\ntitle: provider switch retry\nagent: {from}\n{extra}\
         failure:\n  stack:\n    - action:\n        \
         - {{set_frontmatter: [\"doc.md\", \"agent\", \"{to}\"]}}\n    \
         - action: {{retry: 1}}\nfinalize:\n  stack:\n    \
         - action: {{append_line: [\"events.log\", \"finalize\"]}}\n---\nBody\n"
    )
}

/// Stage the two full recorders a provider-switch row needs: the opening
/// provider fails (driving `failure`), the refreshed one succeeds.
fn stage_provider_switch(doc: &str, from: &str, to: &str) -> Staged {
    let staged = stage_launch_recording(doc);
    write_full_launch_recorder(&staged.bin_dir, from, &staged.events_log, 1);
    write_full_launch_recorder(&staged.bin_dir, to, &staged.events_log, 0);
    staged
}

/// Both attempts, with the binary identities the row expects already checked.
fn switched_attempts(staged: &Staged, pane: &str, from: &str, to: &str) -> Vec<RecordedAttempt> {
    let attempts = recorded_attempts(staged);
    assert_eq!(
        attempts.iter().map(|a| a.binary.as_str()).collect::<Vec<_>>(),
        vec![from, to],
        "the row needs exactly one attempt per provider; recorded argv: {:#?}\npane:\n{pane}",
        attempts.iter().map(|a| &a.argv).collect::<Vec<_>>(),
    );
    attempts
}

/// A fake Gemini that echoes the *content* of the system-prompt file it was
/// handed.
///
/// Gemini's delivery puts a path in `GEMINI_SYSTEM_MD` and nothing else, so
/// reading that path back out is the only way to observe from outside the
/// process that the file was still on disk when the child started. A recorder
/// that logged the variable alone would pass against a dangling path.
fn write_gemini_system_prompt_reader(bin_dir: &Path, events_log: &Path) {
    write_executable(
        &bin_dir.join("gemini"),
        &format!(
            "#!/bin/sh\ncase \"$1\" in --version|-V|-v|version|models) exit 0;; esac\n\
             if [ ! -t 0 ]; then cat > /dev/null 2>&1; fi\n\
             printf 'launched-binary=%s\\n' \"$(basename \"$0\")\" >> {log}\n\
             printf 'sysprompt-path=%s\\n' \"${{GEMINI_SYSTEM_MD:-unset}}\" >> {log}\n\
             if [ -f \"$GEMINI_SYSTEM_MD\" ]; then\n  \
             printf 'sysprompt-read=%s\\n' \"$(tr '\\n' ' ' < \"$GEMINI_SYSTEM_MD\")\" >> {log}\n\
             fi\n\
             printf 'provider-ran\\n' >> {log}\nexit 0\n",
            log = events_log.display(),
        ),
    );
}

/// **Review-10 finding 4 — a provider-switch retry's system-prompt file is
/// still on disk when the child starts.**
///
/// Delivery is provider-shaped, so a retry that lands on a different provider
/// re-applies it — and Gemini's mechanism hands the child a *path* in
/// `GEMINI_SYSTEM_MD`, never the content. The `LaunchPlan` that wrote that temp
/// file is consumed at the rebuild seam, so until the rebuilt bundle took
/// ownership of its artifacts the `NamedTempFile` dropped there and unlinked the
/// file before anything was spawned. The switched attempt then pointed Gemini at
/// a path that no longer existed, silently losing the system prompt.
///
/// The fake Gemini echoes what it can read at that path, so the sentinel
/// reaches the log only if the bytes survived to spawn.
#[test]
#[serial(level2_lifecycle_control)]
fn level2_lifecycle_retry_delivers_a_readable_system_prompt_file_after_a_switch() {
    require_level!(Level::L2, TmuxHarness::available(), "tmux");

    const SENTINEL: &str = "SYSPROMPT-SURVIVED-THE-SWITCH";

    let staged = stage_launch_recording(&provider_switch_retry_doc("goose", "gemini", ""));
    write_full_launch_recorder(&staged.bin_dir, "goose", &staged.events_log, 1);
    write_gemini_system_prompt_reader(&staged.bin_dir, &staged.events_log);

    let sysprompt = staged.workspace.path().join("sysprompt.txt");
    fs::write(&sysprompt, format!("{SENTINEL}\n")).unwrap();
    let extra = format!("--append-system-prompt '{}'", sysprompt.display());

    let pane = run_provider_with_flags(&staged, "", &extra, "finalize");
    let lines = event_lines(&staged);

    assert!(
        lines.iter().any(|l| l == "launched-binary=gemini"),
        "fixture check: the retry must reach the refreshed provider; log:\n\
         {lines:#?}\npane:\n{pane}"
    );
    assert!(
        lines
            .iter()
            .any(|l| l.starts_with("sysprompt-read=") && l.contains(SENTINEL)),
        "the switched attempt must be able to read the file GEMINI_SYSTEM_MD \
         names; log:\n{lines:#?}\npane:\n{pane}"
    );
}

/// A fake `codex` that echoes the content of the file its replacement config key
/// names.
///
/// Codex's `replace` delivery is `ConfigKeyFile`: one argv token shaped
/// `model_instructions_file=<path>`, so the path rides the argv rather than the
/// environment. Same observation problem as Gemini — the content never crosses
/// the process boundary, only the path does.
fn write_codex_system_prompt_reader(bin_dir: &Path, events_log: &Path) {
    write_executable(
        &bin_dir.join("codex"),
        &format!(
            "#!/bin/sh\ncase \"$1\" in --version|-V|-v|version|models) exit 0;; esac\n\
             if [ ! -t 0 ]; then cat > /dev/null 2>&1; fi\n\
             printf 'launched-binary=%s\\n' \"$(basename \"$0\")\" >> {log}\n\
             for a in \"$@\"; do\n  case \"$a\" in\n    \
             model_instructions_file=*)\n      \
             f=\"${{a#model_instructions_file=}}\"\n      \
             printf 'sysprompt-path=%s\\n' \"$f\" >> {log}\n      \
             if [ -f \"$f\" ]; then\n        \
             printf 'sysprompt-read=%s\\n' \"$(tr '\\n' ' ' < \"$f\")\" >> {log}\n      \
             fi\n      ;;\n  esac\ndone\n\
             printf 'provider-ran\\n' >> {log}\nexit 0\n",
            log = events_log.display(),
        ),
    );
}

/// A document that names its own provider and stamps a terminal marker, with no
/// failure/retry surface at all — the direct, first-attempt launch.
fn direct_system_prompt_doc(agent: &str) -> String {
    format!(
        "---\ntitle: direct system prompt\nagent: {agent}\nfinalize:\n  stack:\n    \
         - action: {{append_line: [\"events.log\", \"finalize\"]}}\n---\nBody\n"
    )
}

/// **Review-11 finding 1 — the *initial* launch's file-backed system prompt is
/// still on disk when the first child starts.**
///
/// The sibling row above covers a *replay*-created artifact. This one covers the
/// artifact the command phase itself writes: `CommandPhase` had no owner for it,
/// so the `NamedTempFile` dropped when command construction returned — before
/// `provider_run_handoff` reached the spawn — while the recorded argv and
/// environment still named only its path. The first attempt cannot repair the
/// loss, because its facets equal the recorded invocation and the launch-plan
/// builder therefore takes the verbatim shortcut.
///
/// Both file-backed mechanisms a direct `claudine compose` can select are
/// exercised: Gemini's `GEMINI_SYSTEM_MD` (environment) and Codex's
/// `model_instructions_file` (argv). Each fake child echoes the *bytes* it finds
/// at the path it was handed, so the sentinel reaches the log only if the file
/// survived to spawn.
#[test]
#[serial(level2_lifecycle_control)]
fn level2_lifecycle_direct_compose_delivers_a_readable_system_prompt_file() {
    require_level!(Level::L2, TmuxHarness::available(), "tmux");

    const SENTINEL: &str = "SYSPROMPT-SURVIVED-TO-FIRST-SPAWN";

    let run_arm = |agent: &str, delivery_flag: &str, write_reader: fn(&Path, &Path)| {
        let staged = stage_launch_recording(&direct_system_prompt_doc(agent));
        write_reader(&staged.bin_dir, &staged.events_log);

        let sysprompt = staged.workspace.path().join("sysprompt.txt");
        fs::write(&sysprompt, format!("{SENTINEL}\n")).unwrap();
        let extra = format!("{delivery_flag} '{}'", sysprompt.display());

        let pane = run_provider_with_flags(&staged, "", &extra, "finalize");
        let lines = event_lines(&staged);

        assert!(
            lines
                .iter()
                .any(|l| l.as_str() == format!("launched-binary={agent}")),
            "[{agent}] fixture check: the document's own provider must have \
             launched; log:\n{lines:#?}\npane:\n{pane}"
        );
        // Without this the row would pass against a provider that silently chose
        // an inline mechanism, where nothing about lifetimes is being tested.
        assert!(
            lines.iter().any(|l| l.starts_with("sysprompt-path=")),
            "[{agent}] fixture check: delivery must have been file-backed; \
             log:\n{lines:#?}\npane:\n{pane}"
        );
        assert!(
            lines
                .iter()
                .any(|l| l.starts_with("sysprompt-read=") && l.contains(SENTINEL)),
            "[{agent}] the first attempt must be able to read the file its launch \
             names; log:\n{lines:#?}\npane:\n{pane}"
        );
    };

    run_arm(
        "gemini",
        "--append-system-prompt",
        write_gemini_system_prompt_reader,
    );
    run_arm(
        "codex",
        "--replace-system-prompt",
        write_codex_system_prompt_reader,
    );
}

/// **Review-9 finding 2 — `--output` and `--sandbox` are intent, not bytes.**
///
/// Goose encodes `--output json` as no argv at all and implements no sandbox;
/// Codex encodes the same request as `--json` and does implement `--sandbox`. A
/// retry that switches Goose to Codex therefore has to *re-render* both requests
/// through the refreshed provider's profile. Replaying the opening provider's
/// (empty) slices silently dropped both.
#[test]
#[serial(level2_lifecycle_control)]
fn level2_lifecycle_retry_re_renders_output_and_sandbox_for_the_refreshed_provider() {
    require_level!(Level::L2, TmuxHarness::available(), "tmux");

    let staged = stage_provider_switch(
        &provider_switch_retry_doc("goose", "codex", ""),
        "goose",
        "codex",
    );

    let pane = run_provider_with_flags(&staged, "", "--output json --sandbox", "finalize");
    let attempts = switched_attempts(&staged, &pane, "goose", "codex");

    assert!(
        !attempts[0].has_flag("--json")
            && !attempts[0].has_flag("--output-format")
            && !attempts[0].has_flag("--sandbox"),
        "fixture check: Goose renders neither request as argv; got {:?}",
        attempts[0].argv,
    );
    assert!(
        attempts[1].has_flag("--json"),
        "the retried Codex attempt must carry the requested output format in \
         Codex's own encoding; argv: {:?}\npane:\n{pane}",
        attempts[1].argv,
    );
    assert!(
        attempts[1].has_flag("--sandbox"),
        "the retried Codex attempt must honor the requested sandbox; argv: {:?}\n\
         pane:\n{pane}",
        attempts[1].argv,
    );
}

/// The rendered selection diagnostic, workspace paths normalized so two runs in
/// two temporary directories can be compared byte for byte.
///
/// Starts at the typed identity header, which is what scopes the comparison to
/// the diagnostic: everything a route renders before it (the execution header,
/// the opening attempt's own output, the `failure` markers) is route-specific by
/// design and is asserted separately.
fn selection_diagnostic(pane: &str, staged: &Staged) -> Vec<String> {
    let workspace = staged.workspace.path().display().to_string();
    pane.lines()
        .skip_while(|line| !line.contains("agent resolution failed"))
        .take_while(|line| !line.trim().starts_with("L2_CTL_EXIT"))
        .map(|line| line.trim().replace(&workspace, "<WS>"))
        .filter(|line| !line.is_empty())
        .collect()
}

/// **Review-11 finding 2 — a retry into an unavailable provider refuses exactly
/// as invoking that document directly does, and spawns nothing.**
///
/// The rebuild used to accept any refreshed `agent:` scalar without consulting
/// the invocation's installed-provider snapshot, then paper over the missing
/// binary with the profile's bare executable name. A retry into a provider that
/// is not on `PATH` therefore passed canonical preparation, fired `start`, and
/// died at process spawn — while invoking the same document directly refused
/// during selection with the typed `agent resolution failed` diagnostic.
///
/// Both arms run the **same document bytes**: the direct arm is staged from the
/// file the retry arm's `failure` stack actually wrote, so the frontmatter
/// excerpt each diagnostic carries is identical by construction and the two
/// renderings are comparable in full.
///
/// `PATH` is deliberately [`isolated_path`], not [`augmented_path`]: this is the
/// one row whose contract is that `gemini` is *absent*, and the developer's own
/// `PATH` would decide that.
#[test]
#[serial(level2_lifecycle_control)]
fn level2_lifecycle_retry_to_an_unavailable_provider_matches_direct_selection() {
    require_level!(Level::L2, TmuxHarness::available(), "tmux");

    // Retry arm. `goose` is the only provider on PATH; it fails, the `failure`
    // stack rewrites `agent: gemini`, and the retry must refuse.
    let retry = stage_launch_recording(&provider_switch_retry_doc("goose", "gemini", ""));
    write_full_launch_recorder(&retry.bin_dir, "goose", &retry.events_log, 1);
    let retry_pane = run_compose_await_exit_redirected(
        &retry,
        "",
        &isolated_path(&retry.bin_dir),
        NON_TTY_STDERR,
    );

    let refreshed = fs::read_to_string(&retry.md_file).unwrap();
    assert!(
        refreshed.contains("agent: gemini"),
        "fixture check: the failure stack must have rewritten `agent:` before the \
         retry; document:\n{refreshed}"
    );

    // Direct arm: the very document the retry refreshed into, invoked directly.
    let direct = stage_launch_recording(&refreshed);
    write_full_launch_recorder(&direct.bin_dir, "goose", &direct.events_log, 1);
    let direct_pane = run_compose_await_exit_redirected(
        &direct,
        "",
        &isolated_path(&direct.bin_dir),
        NON_TTY_STDERR,
    );

    let direct_diagnostic = selection_diagnostic(&direct_pane, &direct);
    assert!(
        direct_diagnostic.len() >= 2,
        "fixture check: the direct arm must render a header and a body — without \
         one there is nothing to compare against; pane:\n{direct_pane}"
    );
    assert_eq!(
        selection_diagnostic(&retry_pane, &retry),
        direct_diagnostic,
        "a retry into a provider the invocation snapshot cannot run must render \
         the same typed diagnostic as invoking that document directly; \
         retry pane:\n{retry_pane}\n\ndirect pane:\n{direct_pane}"
    );

    // No child starts for the refused provider. Exactly one process ran — the
    // opening `goose` attempt whose failure drove the retry — and it is not a
    // `gemini`, bare-named or otherwise.
    let binaries: Vec<String> = recorded_attempts(&retry)
        .into_iter()
        .map(|a| a.binary)
        .collect();
    assert_eq!(
        binaries,
        vec!["goose".to_string()],
        "the refusal must happen before any spawn: only the opening attempt may \
         have run; pane:\n{retry_pane}"
    );
    assert_eq!(
        event_lines(&direct).iter().filter(|l| **l == "provider-ran").count(),
        0,
        "fixture check: the direct arm fails during selection, so no provider may \
         launch on it either; pane:\n{direct_pane}"
    );
}

/// **Review-9 finding 2, the reverse direction.** Gemini renders `--output json`
/// as `--output-format json`, which Goose does not accept. A Gemini-to-Goose
/// retry must not carry those bytes across.
#[test]
#[serial(level2_lifecycle_control)]
fn level2_lifecycle_retry_drops_the_opening_providers_flag_encoding() {
    require_level!(Level::L2, TmuxHarness::available(), "tmux");

    let staged = stage_provider_switch(
        &provider_switch_retry_doc("gemini", "goose", ""),
        "gemini",
        "goose",
    );

    let pane = run_provider_with_flags(&staged, "", "--output json --sandbox", "finalize");
    let attempts = switched_attempts(&staged, &pane, "gemini", "goose");

    assert!(
        attempts[0].has_pair("--output-format", "json"),
        "fixture check: Gemini renders the requested format as a flag/value pair; \
         got {:?}",
        attempts[0].argv,
    );
    assert!(
        !attempts[1].has_flag("--output-format"),
        "Gemini's output encoding must not reach a provider that rejects it; \
         argv: {:?}\npane:\n{pane}",
        attempts[1].argv,
    );
    assert!(
        !attempts[1].has_flag("--sandbox"),
        "a provider with no sandbox must not inherit another's flag; argv: {:?}\n\
         pane:\n{pane}",
        attempts[1].argv,
    );
}

/// **Review-9 finding 2 — a dropped `model:` clears `MODEL` in the child.**
///
/// The plan's environment overlay is applied over the invocation's base child
/// environment, which already carries the opening document's `MODEL`. Omitting
/// the key from the overlay therefore left the stale value in the process; only
/// a removal actually reaches the provider without it.
#[test]
#[serial(level2_lifecycle_control)]
fn level2_lifecycle_retry_clears_model_when_the_refresh_drops_it() {
    require_level!(Level::L2, TmuxHarness::available(), "tmux");

    // A namespaced local-runner id is catalog-valid by construction, so the row
    // does not depend on the host's live model listings.
    const OPENING_MODEL: &str = "llamacpp/opening-model";
    // A null action parameter has to arrive as a whole-value interpolation;
    // a bare YAML `null` is a typed parse refusal.
    let doc = format!(
        "---\ntitle: retry drops the model\nagent: goose\nmodel: {OPENING_MODEL}\n\
         failure:\n  stack:\n    - action:\n        \
         - {{set_frontmatter: [\"doc.md\", \"model\", \"{{{{ null }}}}\"]}}\n    \
         - action: {{retry: 1}}\nfinalize:\n  stack:\n    \
         - action: {{append_line: [\"events.log\", \"finalize\"]}}\n---\nBody\n"
    );
    let staged = stage_launch_recording(&doc);
    // Same binary both attempts: the model facet moves on its own, with the
    // provider held still.
    write_full_launch_recorder_with_exit(
        &staged.bin_dir,
        "goose",
        &staged.events_log,
        &format!(
            "if [ -f {log}.second ]; then exit 0; fi\n: > {log}.second\nexit 1",
            log = staged.events_log.display(),
        ),
    );

    let pane = run_provider_with_flags(&staged, "", "", "finalize");
    let attempts = recorded_attempts(&staged);

    assert_eq!(
        attempts.len(),
        2,
        "the row needs an opening attempt and one retry; pane:\n{pane}"
    );
    assert_eq!(
        attempts[0].env_value("MODEL"),
        Some(OPENING_MODEL),
        "fixture check: the opening attempt runs with the document's model; pane:\n{pane}"
    );
    assert_ne!(
        attempts[1].env_value("MODEL"),
        Some(OPENING_MODEL),
        "a refresh that drops `model:` must not leave the opening model in the \
         child environment; env: {:?}\npane:\n{pane}",
        attempts[1].env_value("MODEL"),
    );
}

/// **Review-9 finding 2 — provider-specific base environment does not survive a
/// provider switch.**
///
/// OpenCode is configured through the inline `OPENCODE_CONFIG_CONTENT` document
/// that Claudine writes for it (here via `--yolo`, whose permission block lands
/// in that same value). It is meaningless to any other provider, and an additive
/// overlay had no way to take it back out.
#[test]
#[serial(level2_lifecycle_control)]
fn level2_lifecycle_retry_clears_the_opening_providers_environment() {
    require_level!(Level::L2, TmuxHarness::available(), "tmux");

    let staged = stage_provider_switch(
        // OpenCode refuses to launch without a resolved model, so the document
        // pins one; only the provider facet moves.
        &provider_switch_retry_doc("opencode", "goose", "model: llamacpp/probe-model-x\n"),
        "opencode",
        "goose",
    );

    let pane = run_provider_with_flags(&staged, "", "--yolo", "finalize");
    let attempts = switched_attempts(&staged, &pane, "opencode", "goose");

    assert!(
        attempts[0].env_value("OPENCODE_CONFIG_CONTENT").is_some(),
        "fixture check: the OpenCode attempt runs with an inline config; env keys: {:?}\n\
         pane:\n{pane}",
        attempts[0].env.iter().map(|(k, _)| k).collect::<Vec<_>>(),
    );
    assert_eq!(
        attempts[1].env_value("OPENCODE_CONFIG_CONTENT"),
        None,
        "OpenCode's inline config must not reach the provider a retry switched \
         to; pane:\n{pane}"
    );
}

/// A non-secret stand-in. Only the *key* reaches an allow-list, so the value
/// never needs to be a real credential — and must not be.
const FIXTURE_OPENAI_KEY: &str = "fixture-openai-not-a-real-key";

/// **Review-10 finding 1 — credential admission follows the provider that runs.**
///
/// Goose's `allowed_env_keys` is empty and Codex's names `OPENAI_API_KEY`, so
/// the opening Goose sanitizer strips the ambient key and the Codex retry must
/// get it back. Before the fix the retry inherited Goose's sanitized base and
/// launched Codex without a credential it is entitled to.
#[test]
#[serial(level2_lifecycle_control)]
fn level2_lifecycle_retry_readmits_credentials_the_refreshed_provider_admits() {
    require_level!(Level::L2, TmuxHarness::available(), "tmux");

    let staged = stage_provider_switch(
        &provider_switch_retry_doc("goose", "codex", ""),
        "goose",
        "codex",
    );

    let pane = run_provider_with_ambient_env(
        &staged,
        "",
        "",
        "finalize",
        &[("OPENAI_API_KEY", FIXTURE_OPENAI_KEY)],
    );
    let attempts = switched_attempts(&staged, &pane, "goose", "codex");

    assert_eq!(
        attempts[0].env_value("OPENAI_API_KEY"),
        None,
        "fixture check: Goose admits no credential keys, so the opening attempt \
         must not see the ambient one; pane:\n{pane}"
    );
    assert_eq!(
        attempts[1].env_value("OPENAI_API_KEY"),
        Some(FIXTURE_OPENAI_KEY),
        "Codex admits OPENAI_API_KEY, so the switched attempt must receive the \
         ambient value the opening sanitizer removed; pane:\n{pane}"
    );
}

/// **Review-10 finding 1, leak direction.** The reverse switch must *remove* an
/// ambient credential the opening provider admitted, because the target's
/// allow-list does not name it. Before the fix the secret rode the invocation
/// base straight into a provider that should never receive it.
#[test]
#[serial(level2_lifecycle_control)]
fn level2_lifecycle_retry_strips_credentials_the_refreshed_provider_rejects() {
    require_level!(Level::L2, TmuxHarness::available(), "tmux");

    let staged = stage_provider_switch(
        &provider_switch_retry_doc("codex", "goose", ""),
        "codex",
        "goose",
    );

    let pane = run_provider_with_ambient_env(
        &staged,
        "",
        "",
        "finalize",
        &[("OPENAI_API_KEY", FIXTURE_OPENAI_KEY)],
    );
    let attempts = switched_attempts(&staged, &pane, "codex", "goose");

    assert_eq!(
        attempts[0].env_value("OPENAI_API_KEY"),
        Some(FIXTURE_OPENAI_KEY),
        "fixture check: Codex admits OPENAI_API_KEY, so the opening attempt \
         carries it; pane:\n{pane}"
    );
    assert_eq!(
        attempts[1].env_value("OPENAI_API_KEY"),
        None,
        "Goose admits no credential keys, so the switched attempt must not \
         inherit Codex's admission; pane:\n{pane}"
    );
}

/// Every `warning:` line the pane shows, trimmed and in order.
///
/// `crate::log::warn` writes `warning: <message>` to stderr, and the L2 runner
/// exports `NO_COLOR=1`, so the prefix is a literal on the captured pane.
fn pane_warnings(pane: &str) -> Vec<String> {
    pane.lines()
        .map(str::trim)
        .filter(|line| line.starts_with("warning:"))
        .map(str::to_string)
        .collect()
}

/// Run `doc` twice with the same `extra_flags`: once switching `from` → `to`
/// across a retry, and once pinned directly to `to`. Returns
/// `(switch_warnings, direct_warnings)`.
///
/// The direct run is what the comparison is *for*: the expected text is read
/// off the route the equivalence contract defines as correct rather than
/// duplicated as a literal here, so a reworded provider warning cannot leave the
/// two routes agreeing with a stale assertion.
fn warnings_from_switch_and_direct(
    from: &str,
    to: &str,
    extra_flags: &dyn Fn(&Staged) -> String,
) -> (Vec<String>, Vec<String>) {
    let doc = provider_switch_retry_doc(from, to, "");

    let switched = stage_provider_switch(&doc, from, to);
    let flags = extra_flags(&switched);
    let switch_pane = run_provider_with_flags(&switched, "", &flags, "finalize");
    switched_attempts(&switched, &switch_pane, from, to);

    // A fresh stage: the two runs must not share an events log, and the direct
    // run pins `to` with an explicit flag, which no frontmatter can move — so it
    // succeeds on its first attempt and never enters the `failure` stack.
    let direct = stage_provider_switch(&doc, from, to);
    let flags = extra_flags(&direct);
    let direct_pane = run_provider_with_flags(&direct, &format!("--{to}"), &flags, "finalize");

    (
        pane_warnings(&switch_pane),
        pane_warnings(&direct_pane),
    )
}

/// **Review-11 finding 4 — an unsupported system-prompt delivery warns on the
/// replay path too.**
///
/// Codex delivers `--replace-system-prompt` through a config-key file; Goose
/// declares `replace` unsupported in both modes. A retry that switches Codex →
/// Goose therefore re-applies the prompt for Goose and gets the "not supported"
/// notice — which the replay used to consume alongside the args and artifacts
/// and then throw away, on the theory that a retry has no operator. Invoking the
/// refreshed document directly shows it, so the switched attempt must too.
#[test]
#[serial(level2_lifecycle_control)]
fn level2_lifecycle_switch_surfaces_unsupported_system_prompt_warning() {
    require_level!(Level::L2, TmuxHarness::available(), "tmux");

    let sysprompt_flags = |staged: &Staged| {
        let sysprompt = staged.workspace.path().join("sysprompt.txt");
        fs::write(&sysprompt, "REPLACEMENT SYSTEM PROMPT\n").unwrap();
        format!("--replace-system-prompt '{}'", sysprompt.display())
    };
    let (switch_warnings, direct_warnings) =
        warnings_from_switch_and_direct("codex", "goose", &sysprompt_flags);

    let expected = direct_warnings
        .iter()
        .find(|line| line.contains("system prompt"))
        .unwrap_or_else(|| {
            panic!(
                "fixture check: invoking the document directly on Goose must warn \
                 that `replace` is unsupported; direct warnings: {direct_warnings:#?}"
            )
        });
    assert!(
        switch_warnings.contains(expected),
        "the switched attempt must render the same system-prompt warning the \
         direct route renders.\nexpected: {expected}\nswitch warnings: \
         {switch_warnings:#?}\ndirect warnings: {direct_warnings:#?}"
    );
}

/// **Review-11 finding 4 — an unsupported `--sandbox` warns on the replay path
/// too.**
///
/// The mirror of the system-prompt row for a *capability flag*: Codex
/// implements `--sandbox`, Goose does not, and the replay discarded
/// `apply_sandbox`'s return outright. The request is invocation-fixed, so the
/// refusal belongs to whichever provider the retry lands on.
#[test]
#[serial(level2_lifecycle_control)]
fn level2_lifecycle_switch_surfaces_unsupported_sandbox_warning() {
    require_level!(Level::L2, TmuxHarness::available(), "tmux");

    let (switch_warnings, direct_warnings) =
        warnings_from_switch_and_direct("codex", "goose", &|_| "--sandbox".to_string());

    let expected = direct_warnings
        .iter()
        .find(|line| line.contains("sandbox"))
        .unwrap_or_else(|| {
            panic!(
                "fixture check: invoking the document directly on Goose must warn \
                 that `--sandbox` was skipped; direct warnings: {direct_warnings:#?}"
            )
        });
    assert!(
        switch_warnings.contains(expected),
        "the switched attempt must render the same sandbox warning the direct \
         route renders.\nexpected: {expected}\nswitch warnings: \
         {switch_warnings:#?}\ndirect warnings: {direct_warnings:#?}"
    );
}

/// **Review-8 finding 2 — MCP runtime injection.** A document whose `failure`
/// stack adds an MCP `#tag` to its own body and then retries has the retried
/// attempt's MCP configuration rebuilt from the refreshed tag set.
///
/// `--allowed-mcp-server-names` is contributed by the Gemini injector alone, so
/// its value is direct evidence of the server set injected for *this* launch.
/// The seeded catalog has empty user- and repo-scope defaults, so the server can
/// only enter a session set through the body tag; the tag is lexed out of the
/// prompt before delivery, so the id in argv cannot be the prompt echoing back.
#[test]
#[serial(level2_lifecycle_control)]
fn level2_lifecycle_retry_rebuilds_mcp_injection_from_the_refreshed_body() {
    require_level!(Level::L2, TmuxHarness::available(), "tmux");

    let doc = r#"---
title: retry gains an mcp tag
agent: gemini
failure:
  stack:
    - action:
        - {append_line: ["events.log", "failure"]}
        - {append_line: ["doc.md", "also check #proxyprobeserver"]}
    - action: {retry: 1}
finalize:
  stack:
    - action: {append_line: ["events.log", "finalize"]}
---
Body with no tag
"#;
    let staged = stage_launch_recording(doc);
    seed_mcp_catalog(staged.workspace.path());
    write_launch_recorder(&staged.bin_dir, "gemini", &staged.events_log, 1);

    let pane = run_compose_await_exit_with_args(&staged, "--mcp");

    let refreshed = fs::read_to_string(&staged.md_file).unwrap();
    assert!(
        refreshed.contains("#proxyprobeserver"),
        "fixture check: the failure stack must have added the tag before the retry; \
         document:\n{refreshed}"
    );

    let allowed: Vec<String> = event_lines(&staged)
        .into_iter()
        .filter_map(|line| line.strip_prefix("mcp-allowed=").map(str::to_string))
        .collect();
    assert_eq!(
        allowed,
        vec!["none".to_string(), MCP_PROBE_SERVER.to_string()],
        "the first attempt's body selects no server and the retried attempt's \
         refreshed body selects one, so the injected MCP configuration must differ \
         between the two launches; pane:\n{pane}"
    );
}

/// **Review-11 finding 3 — the MCP tag set is the *composed* document's.**
///
/// The row above appends a literal `#proxyprobeserver` to the file on disk, so a
/// rebuild that re-read the raw source found the same tag a rebuild reading the
/// prepared document finds; it cannot discriminate where the set came from. This
/// row's tag exists **only after composition**: the body carries
/// `#{{ probe }}` and the frontmatter carries the value, so the bytes on disk
/// never contain the server id at all.
///
/// Both launches must therefore inject it — the first attempt's, whose plan the
/// invocation recorded, and the retried attempt's, rebuilt at the fresh-read
/// boundary. Re-lexing the source at either point yields no tag (`#{{` fails the
/// alphabetic-first-character rule), which is exactly the `mcp-allowed=none` the
/// pre-fix code produced while the operator's requested server silently reached
/// neither the model as text nor the launch as configuration.
#[test]
#[serial(level2_lifecycle_control)]
fn level2_lifecycle_retry_keeps_an_interpolated_mcp_tag_at_child_launch() {
    require_level!(Level::L2, TmuxHarness::available(), "tmux");

    let doc = format!(
        r#"---
title: retry keeps an interpolated mcp tag
agent: gemini
probe: {MCP_PROBE_SERVER}
failure:
  stack:
    - action: {{append_line: ["events.log", "failure"]}}
    - action: {{retry: 1}}
finalize:
  stack:
    - action: {{append_line: ["events.log", "finalize"]}}
---
Body whose server is named only after composition #{{{{ probe }}}} here
"#
    );
    let staged = stage_launch_recording(&doc);
    seed_mcp_catalog(staged.workspace.path());
    write_launch_recorder(&staged.bin_dir, "gemini", &staged.events_log, 1);

    let pane = run_compose_await_exit_with_args(&staged, "--mcp");

    let on_disk = fs::read_to_string(&staged.md_file).unwrap();
    let (_, body) = on_disk.rsplit_once("---\n").unwrap();
    assert!(
        body.contains("#{{ probe }}") && !body.contains(MCP_PROBE_SERVER),
        "fixture check: the authored body must carry only the template, so a \
         second raw read of this file can find no server id; body:\n{body}"
    );

    let allowed: Vec<String> = event_lines(&staged)
        .into_iter()
        .filter_map(|line| line.strip_prefix("mcp-allowed=").map(str::to_string))
        .collect();
    assert_eq!(
        allowed,
        vec![MCP_PROBE_SERVER.to_string(), MCP_PROBE_SERVER.to_string()],
        "an interpolated `#tag` selects a server on the first attempt and must \
         still select it on the retried attempt, whose launch is rebuilt at the \
         fresh-read boundary; pane:\n{pane}"
    );
}

/// **Review-8 finding 2 — session mode, structured output, and permission
/// mode.** A document whose `failure` stack flips its own `interactive:` and
/// then retries launches the retried attempt in the refreshed mode: the
/// structured-stream flags drop from argv, and OpenCode's `--yolo` bypass — which
/// is non-interactive only — stops applying, which the child's `YOLO`
/// environment must report.
///
/// Three facets move together here because they are read off one rebuilt facet
/// set rather than derived independently; asserting them in one row is what
/// proves they cannot disagree.
#[test]
#[serial(level2_lifecycle_control)]
fn level2_lifecycle_retry_launches_the_refreshed_mode_and_permission() {
    require_level!(Level::L2, TmuxHarness::available(), "tmux");

    let doc = r#"---
title: retry flips session mode
agent: opencode
failure:
  stack:
    - action:
        - {append_line: ["events.log", "failure"]}
        - {set_frontmatter: ["doc.md", "interactive", true]}
    - action: {retry: 1}
finalize:
  stack:
    - action: {append_line: ["events.log", "finalize"]}
---
Body
"#;
    let staged = stage_launch_recording(doc);
    write_launch_recorder(&staged.bin_dir, "opencode", &staged.events_log, 1);

    let pane = run_compose_await_exit_with_args(&staged, "--yolo");

    let flags: Vec<String> = event_lines(&staged)
        .into_iter()
        .filter_map(|line| line.strip_prefix("flags=").map(str::to_string))
        .collect();
    let yolo: Vec<String> = event_lines(&staged)
        .into_iter()
        .filter_map(|line| line.strip_prefix("env-yolo=").map(str::to_string))
        .collect();

    assert_eq!(
        flags.len(),
        2,
        "fixture check: both the original and the retried attempt must have \
         launched; got {flags:?}; pane:\n{pane}"
    );
    assert!(
        flags[0].contains("--print-logs") || flags[0].contains("--log-level"),
        "fixture check: the non-interactive opening attempt carries OpenCode's \
         structured-stream flags; got {flags:?}; pane:\n{pane}"
    );
    assert_ne!(
        flags[0], flags[1],
        "the retried attempt's argv must be rebuilt for the refreshed session \
         mode, not replayed from the invocation; got {flags:?}; pane:\n{pane}"
    );
    assert_eq!(
        yolo,
        vec!["true".to_string(), "false".to_string()],
        "OpenCode's bypass is non-interactive only, so the refreshed interactive \
         attempt must launch with the permission mode it actually achieved; \
         got {yolo:?}; pane:\n{pane}"
    );
}

/// A document that flips its own `interactive:` in the `failure` stack and
/// retries once, with the provider held still so the session mode is the only
/// facet that moves. `opening` is spliced into the frontmatter when the row
/// needs an interactive opening attempt.
fn interactivity_retry_doc(opening: &str, refreshed: bool) -> String {
    format!(
        "---\ntitle: retry flips session mode\nagent: goose\n{opening}\
         failure:\n  stack:\n    - action:\n        \
         - {{set_frontmatter: [\"doc.md\", \"interactive\", {refreshed}]}}\n    \
         - action: {{retry: 1}}\nfinalize:\n  stack:\n    \
         - action: {{append_line: [\"events.log\", \"finalize\"]}}\n---\nBody\n"
    )
}

/// Both interactivity markers as the two recorded child processes actually
/// received them.
///
/// Asserted together because they are two projections of one session mode: a
/// row that read only `INTERACTIVE` would pass with a stale hook gate, which is
/// the half of the defect nothing else in the suite observes.
fn assert_interactivity_markers(
    attempts: &[RecordedAttempt],
    opening: (&str, &str),
    refreshed: (&str, &str),
    pane: &str,
) {
    assert_eq!(
        attempts.len(),
        2,
        "the row needs an opening attempt and one retry; pane:\n{pane}"
    );
    let markers = |attempt: &RecordedAttempt| {
        (
            attempt.env_value("INTERACTIVE").map(str::to_string),
            attempt.env_value("CLAUDINE_INTERACTIVE").map(str::to_string),
        )
    };
    assert_eq!(
        markers(&attempts[0]),
        (Some(opening.0.to_string()), Some(opening.1.to_string())),
        "fixture check: the opening attempt must carry its own mode's markers; \
         pane:\n{pane}"
    );
    assert_eq!(
        markers(&attempts[1]),
        (Some(refreshed.0.to_string()), Some(refreshed.1.to_string())),
        "the retried attempt must carry the refreshed mode's markers, not the \
         opening invocation's; pane:\n{pane}"
    );
}

/// Stage a recorder for `slug` that fails its first launch and succeeds on the
/// second, so a facet other than the provider can be moved across the retry.
fn stage_fail_once_recorder(staged: &Staged, slug: &str) {
    write_full_launch_recorder_with_exit(
        &staged.bin_dir,
        slug,
        &staged.events_log,
        &format!(
            "if [ -f {log}.second ]; then exit 0; fi\n: > {log}.second\nexit 1",
            log = staged.events_log.display(),
        ),
    );
}

/// **Review-10 finding 2 — the interactivity markers follow the refreshed mode
/// into the child process.**
///
/// Wrapped providers and downstream processes read `INTERACTIVE`, and
/// `CLAUDINE_INTERACTIVE` gates hook behavior. Both were stamped once from the
/// opening session mode and no other part of the per-attempt patch covers them,
/// so a retry that flipped `interactive:` shipped refreshed argv and streaming
/// behavior beside the opening mode's markers.
#[test]
#[serial(level2_lifecycle_control)]
fn level2_lifecycle_retry_refreshes_interactivity_markers_into_interactive() {
    require_level!(Level::L2, TmuxHarness::available(), "tmux");

    let staged = stage_launch_recording(&interactivity_retry_doc("", true));
    stage_fail_once_recorder(&staged, "goose");

    let pane = run_provider_with_flags(&staged, "", "", "finalize");

    assert_interactivity_markers(
        &recorded_attempts(&staged),
        ("false", "0"),
        ("true", "1"),
        &pane,
    );
}

/// **Review-10 finding 2, the reverse direction.** An invocation that opened
/// interactive and refreshed to non-interactive must not hand the retried child
/// an `INTERACTIVE=true` it can act on, nor leave the hook gate open.
#[test]
#[serial(level2_lifecycle_control)]
fn level2_lifecycle_retry_refreshes_interactivity_markers_into_non_interactive() {
    require_level!(Level::L2, TmuxHarness::available(), "tmux");

    let staged = stage_launch_recording(&interactivity_retry_doc("interactive: true\n", false));
    stage_fail_once_recorder(&staged, "goose");

    let pane = run_provider_with_flags(&staged, "", "", "finalize");

    assert_interactivity_markers(
        &recorded_attempts(&staged),
        ("true", "1"),
        ("false", "0"),
        &pane,
    );
}

// ── review-9 finding 1: the retried attempt's execution adapters ─────────────
//
// Review 8 made the rebuilt bundle own the spawn inputs (binary, profile, argv,
// session mode, permission, MCP). These rows prove the bundle also owns the
// provider-dependent *execution adapters*: the Codex `--output-last-message`
// artifact, the stdout/stderr noise-prefix policy, and the composition dispatch
// context. Before the fix, `execute_attempt_phase` copied all of these from the
// invocation-fixed run state, so a provider-switch retry ran the new provider's
// process through the opening provider's adapters.

/// The final unique token the fake Codex writes to its `--output-last-message`
/// sink. It appears in the pane only if the retried attempt's rebuilt bundle
/// carries a working Codex artifact for the rebuilt argv's sink path.
const CODEX_FILE_TOKEN: &str = "FINAL-FROM-CODEX-FILE-9c41";

/// Codex's own stderr noise line (`stderr_noise_prefixes` for Codex). A row
/// switching INTO Codex must filter it; a row switching OUT to a provider with
/// no noise prefixes must let it through.
const CODEX_NOISE_LINE: &str = "Reading prompt from stdin...";

/// A fake `codex` that records its launch, honors `--output-last-message` by
/// writing [`CODEX_FILE_TOKEN`] to the given path, emits Codex's stderr noise
/// line, and speaks enough of the Codex wire for a clean structured attempt.
fn write_codex_with_output_file(bin_dir: &Path, events_log: &Path) {
    write_executable(
        &bin_dir.join("codex"),
        &format!(
            r#"#!/bin/sh
if [ ! -t 0 ]; then cat > /dev/null 2>&1; fi
printf 'launched-binary=%s\n' "$(basename "$0")" >> {log}
sink=none
prev=
for a in "$@"; do
  if [ "$prev" = '--output-last-message' ]; then sink="$a"; fi
  prev="$a"
done
printf 'codex-sink=%s\n' "$sink" >> {log}
if [ "$sink" != none ]; then printf '%s' '{token}' > "$sink"; fi
printf '{noise}\n' >&2
printf '%s\n' '{{"type":"thread.started","thread_id":"codex-retry-thread"}}'
printf '%s\n' '{{"type":"item.completed","item":{{"type":"agent_message","text":"stream narration"}}}}'
printf 'provider-ran\n' >> {log}
exit 0
"#,
            log = events_log.display(),
            token = CODEX_FILE_TOKEN,
            noise = CODEX_NOISE_LINE,
        ),
    );
}

/// A launch recorder that additionally prints Codex's noise line on stderr, so
/// a row can prove which provider's noise policy filtered the pane.
fn write_noisy_launch_recorder(bin_dir: &Path, slug: &str, events_log: &Path, exit_code: i32) {
    write_executable(
        &bin_dir.join(slug),
        &format!(
            "#!/bin/sh\nif [ ! -t 0 ]; then cat > /dev/null 2>&1; fi\n\
             printf 'launched-binary=%s\\n' \"$(basename \"$0\")\" >> {log}\n\
             flags=\nfor a in \"$@\"; do\n  \
             case \"$a\" in -*) flags=\"$flags $a\";; esac\ndone\n\
             printf 'flags=%s\\n' \"$flags\" >> {log}\n\
             printf '{noise}\\n' >&2\n\
             printf 'provider-ran\\n' >> {log}\nexit {code}\n",
            log = events_log.display(),
            noise = CODEX_NOISE_LINE,
            code = exit_code,
        ),
    );
}

/// Install a claudine hook config whose `session_start` action records the
/// dispatched `extra.resolved_model` into `events.log`, making the composition
/// dispatch context an externally observable per-attempt fact.
fn seed_dispatch_recorder_config(staged: &Staged) {
    let recorder = staged.bin_dir.join("record-dispatch");
    write_executable(
        &recorder,
        &format!(
            "#!/bin/sh\nprintf 'dispatched-model=%s\\n' \"$1\" >> {log}\n",
            log = staged.events_log.display(),
        ),
    );
    let config = format!(
        r#"{{
  "actions": {{
    "session_start": [
      {{ "type": "bash", "command": "{recorder}", "params": "'{{{{extra.resolved_model}}}}'" }}
    ]
  }}
}}
"#,
        recorder = recorder.display(),
    );
    fs::write(
        staged.workspace.path().join(".claudine/config.json"),
        config,
    )
    .unwrap();
}

/// The `dispatched-model=` values in recording order.
fn dispatched_models(staged: &Staged) -> Vec<String> {
    event_lines(staged)
        .into_iter()
        .filter_map(|line| line.strip_prefix("dispatched-model=").map(str::to_string))
        .collect()
}

/// **Review-9 finding 1 — switching INTO Codex.** A document whose `failure`
/// stack rewrites `agent:` to Codex (and pins a Codex `model:`) and then
/// retries must run the retried attempt through Codex's own execution
/// adapters:
///
/// - **Final output recovery** — the rebuilt argv carries
///   `--output-last-message <sink>`, and the rebuilt bundle's Codex artifact
///   wraps that same sink, so the final response is loaded from the file the
///   provider wrote. The old behavior kept the opening attempt's `None`
///   artifact and could never render the file's contents.
/// - **Pane filtering** — Codex's stderr noise line is filtered by Codex's
///   noise prefixes. The old behavior filtered with Goose's (empty) prefixes
///   and leaked the line into the pane.
/// - **Dispatch metadata** — the `session_start` hook observes the refreshed
///   document's `resolved_model`, not the opening document's (empty) one.
#[test]
#[serial(level2_lifecycle_control)]
fn level2_lifecycle_retry_into_codex_rebuilds_the_execution_adapters() {
    require_level!(Level::L2, TmuxHarness::available(), "tmux");

    let doc = r#"---
title: retry switches into codex
agent: goose
failure:
  stack:
    - action:
        - {append_line: ["events.log", "failure"]}
        - {set_frontmatter: ["doc.md", "agent", "codex"]}
        - {set_frontmatter: ["doc.md", "model", "gpt-5.4"]}
    - action: {retry: 1}
finalize:
  stack:
    - action: {append_line: ["events.log", "finalize"]}
---
Body
"#;
    let staged = stage_launch_recording(doc);
    seed_dispatch_recorder_config(&staged);
    write_launch_recorder(&staged.bin_dir, "goose", &staged.events_log, 1);
    write_codex_with_output_file(&staged.bin_dir, &staged.events_log);

    let pane = run_compose_await_exit_with_args(&staged, "");

    let refreshed = fs::read_to_string(&staged.md_file).unwrap();
    assert!(
        refreshed.contains("agent: codex") && refreshed.contains("model: gpt-5.4"),
        "fixture check: the failure stack must have rewritten `agent:`/`model:` \
         before the retry; document:\n{refreshed}"
    );
    assert_eq!(
        launched_binaries(&staged),
        vec!["goose".to_string(), "codex".to_string()],
        "the retried attempt must spawn Codex; pane:\n{pane}"
    );

    // Final output recovery through the rebuilt Codex artifact.
    let sinks: Vec<String> = event_lines(&staged)
        .into_iter()
        .filter_map(|line| line.strip_prefix("codex-sink=").map(str::to_string))
        .collect();
    assert!(
        sinks.len() == 1 && sinks[0] != "none",
        "the rebuilt Codex argv must carry --output-last-message; got {sinks:?}; \
         pane:\n{pane}"
    );
    assert!(
        pane.contains(CODEX_FILE_TOKEN),
        "the final response must be recovered from the Codex output file the \
         rebuilt artifact wraps; pane:\n{pane}"
    );

    // Pane filtering under the rebuilt provider's noise policy.
    assert!(
        !pane.contains(CODEX_NOISE_LINE),
        "Codex's stderr noise line must be filtered by the REBUILT profile's \
         prefixes, not the opening Goose profile's empty set; pane:\n{pane}"
    );

    // Dispatched provider/model metadata from the refreshed document.
    assert_eq!(
        dispatched_models(&staged),
        vec!["gpt-5.4".to_string()],
        "the session_start dispatch must carry the refreshed document's \
         resolved model, not the opening document's; pane:\n{pane}"
    );
}

/// **Review-9 finding 1 — switching OUT OF Codex.** A document that opens on
/// Codex, fails, rewrites `agent:` to Goose, and retries must drop Codex's
/// execution adapters with the provider:
///
/// - the retried argv carries no `--output-last-message` (the artifact leaves
///   the bundle together with the flag); and
/// - Goose's (empty) noise prefixes govern the pane, so a stderr line that
///   Codex's policy would filter is now visible. The old behavior kept
///   Codex's prefixes and silently swallowed the new provider's stderr.
#[test]
#[serial(level2_lifecycle_control)]
fn level2_lifecycle_retry_out_of_codex_drops_the_codex_adapters() {
    require_level!(Level::L2, TmuxHarness::available(), "tmux");

    let doc = r#"---
title: retry leaves codex
agent: codex
failure:
  stack:
    - action:
        - {append_line: ["events.log", "failure"]}
        - {set_frontmatter: ["doc.md", "agent", "goose"]}
    - action: {retry: 1}
finalize:
  stack:
    - action: {append_line: ["events.log", "finalize"]}
---
Body
"#;
    let staged = stage_launch_recording(doc);
    write_launch_recorder(&staged.bin_dir, "codex", &staged.events_log, 1);
    write_noisy_launch_recorder(&staged.bin_dir, "goose", &staged.events_log, 0);

    let pane = run_compose_await_exit_with_args(&staged, "");

    let refreshed = fs::read_to_string(&staged.md_file).unwrap();
    assert!(
        refreshed.contains("agent: goose"),
        "fixture check: the failure stack must have rewritten `agent:` before \
         the retry; document:\n{refreshed}"
    );
    assert_eq!(
        launched_binaries(&staged),
        vec!["codex".to_string(), "goose".to_string()],
        "the retried attempt must spawn Goose; pane:\n{pane}"
    );

    let flags: Vec<String> = event_lines(&staged)
        .into_iter()
        .filter_map(|line| line.strip_prefix("flags=").map(str::to_string))
        .collect();
    assert_eq!(
        flags.len(),
        2,
        "fixture check: both attempts must have launched; got {flags:?}; pane:\n{pane}"
    );
    assert!(
        flags[0].contains("--output-last-message"),
        "fixture check: the opening Codex attempt captures through the output \
         file; got {flags:?}; pane:\n{pane}"
    );
    assert!(
        !flags[1].contains("--output-last-message"),
        "a retry that leaves Codex must drop the output-file capture from the \
         rebuilt argv; got {flags:?}; pane:\n{pane}"
    );
    assert!(
        pane.contains(CODEX_NOISE_LINE),
        "the rebuilt Goose attempt has no stderr noise prefixes, so the line \
         Codex's policy would filter must reach the pane; pane:\n{pane}"
    );
}

/// A document that opens on `{from}`, switches to `{to}` in its `failure` stack
/// and retries, then chains a `resume` out of the *switched* attempt's
/// `success` stack.
///
/// The resume budget is 1, so the resumed attempt's own `success` falls through
/// to `finalize` and the run terminates in exactly three provider invocations.
fn switch_then_resume_doc(from: &str, to: &str, follow_up: &str) -> String {
    format!(
        "---\ntitle: switch then resume\nagent: {from}\n\
         failure:\n  stack:\n    - action: \
         {{set_frontmatter: [\"doc.md\", \"agent\", \"{to}\"]}}\n    \
         - action: {{retry: 1}}\n\
         success:\n  stack:\n    - action: {{resume: \"{follow_up}\"}}\n\
         finalize:\n  stack:\n    \
         - action: {{append_line: [\"events.log\", \"finalize\"]}}\n---\nBody\n"
    )
}

/// **Review-10 finding 3 — a control action chained out of a switched attempt.**
///
/// Goose opens and fails; the `failure` stack rewrites `agent: codex` and
/// retries; Codex succeeds and reports a session id; the switched attempt's own
/// `success` stack then resumes it. The resumed invocation must carry *Codex's*
/// resume encoding (`codex exec resume <id>`), never Goose's
/// (`goose run --resume --session-id <id>`).
///
/// This closes the chain the review found untested: every prior provider-switch
/// row ended at the switched attempt. It does not by itself discriminate the
/// stale-identity defect, because the resume argv is rebuilt from the refreshed
/// document rather than from the value classification forwards, and because all
/// ten shipped providers report `supports_resume() == true` — see
/// `composition_seams::classification_reads_no_invocation_fixed_launch_identity`
/// for the guard that does.
#[test]
#[serial(level2_lifecycle_control)]
fn level2_lifecycle_switch_into_codex_resumes_under_codexs_encoding() {
    require_level!(Level::L2, TmuxHarness::available(), "tmux");

    let follow_up = "finish the switched work";
    let session_id = "switch-into-codex-session";
    let staged = stage_launch_recording(&switch_then_resume_doc("goose", "codex", follow_up));
    write_launch_recorder(&staged.bin_dir, "goose", &staged.events_log, 1);
    write_session_reporting_codex(&staged.bin_dir, &staged.events_log, session_id);

    let pane = run_compose_await_exit_with_args(&staged, "");
    let lines = event_lines(&staged);

    assert_eq!(
        launched_binaries(&staged),
        vec![
            "goose".to_string(),
            "codex".to_string(),
            "codex".to_string()
        ],
        "the switched attempt must succeed and its `success` stack must resume it; \
         events {lines:?}; pane:\n{pane}"
    );
    assert!(
        lines.iter().any(|l| l == "resume-session-ok"),
        "the resumed invocation must carry Codex's own `resume <id>` argv, not the \
         opening Goose profile's `--resume --session-id`; got {lines:?}; pane:\n{pane}"
    );
    assert!(
        lines.iter().any(|l| l == "follow-up-ok"),
        "the resumed invocation must receive the `success`-stack follow-up prompt; \
         got {lines:?}; pane:\n{pane}"
    );
}

/// **Review-10 finding 3, the reverse direction.** Codex opens, reports a
/// session id on its own wire, and fails; the switch lands on Goose, which
/// succeeds. Goose runs no structured stream and so reports no session at all.
///
/// The chained `resume` must therefore be refused as session-less. The opening
/// Codex session must not stand in for it: a resume admitted on the strength of
/// a session the switched-to provider never opened would hand Goose a session id
/// from a different provider's store.
#[test]
#[serial(level2_lifecycle_control)]
fn level2_lifecycle_switch_out_of_codex_refuses_a_session_less_resume() {
    require_level!(Level::L2, TmuxHarness::available(), "tmux");

    let follow_up = "finish the switched work";
    let session_id = "switch-out-of-codex-session";
    let staged = stage_launch_recording(&switch_then_resume_doc("codex", "goose", follow_up));
    write_failing_session_reporting_codex(&staged.bin_dir, &staged.events_log, session_id);
    write_launch_recorder(&staged.bin_dir, "goose", &staged.events_log, 0);

    let pane = run_compose_await_exit_with_args(&staged, "");
    let lines = event_lines(&staged);

    assert_eq!(
        launched_binaries(&staged),
        vec!["codex".to_string(), "goose".to_string()],
        "the run must stop at the switched Goose attempt — no third invocation; \
         events {lines:?}; pane:\n{pane}"
    );
    assert!(
        pane.contains("resume") && pane.contains("session"),
        "the chained resume must be refused as session-less rather than reusing the \
         opening Codex session; events {lines:?}; pane:\n{pane}"
    );
    assert!(
        !lines.iter().any(|l| l.contains(session_id)),
        "the opening provider's session id must not reach the switched attempt's \
         recovery; got {lines:?}; pane:\n{pane}"
    );
}

/// A `codex` that reports a session id on Codex's wire and then fails, so the
/// `failure` stack runs with a live session on the record.
fn write_failing_session_reporting_codex(
    bin_dir: &Path,
    events_log: &Path,
    session_id: &str,
) {
    write_executable(
        &bin_dir.join("codex"),
        &format!(
            r#"#!/bin/sh
case "$1" in --version|-V|-v|version|models) exit 0;; esac
cat > /dev/null
printf 'launched-binary=codex\n' >> {log}
printf '%s\n' '{{"type":"thread.started","thread_id":"{session_id}"}}'
exit 1
"#,
            log = events_log.display(),
            session_id = session_id,
        ),
    );
}

/// A `codex` that always succeeds, reports `session_id` on Codex's wire, and
/// records whether it was reached through Codex's resume argv.
///
/// Unlike [`write_resumable_codex`], the first invocation succeeds: these rows
/// chain the resume out of `success`, not out of `failure`.
fn write_session_reporting_codex(bin_dir: &Path, events_log: &Path, session_id: &str) {
    write_executable(
        &bin_dir.join("codex"),
        &format!(
            r#"#!/bin/sh
case "$1" in --version|-V|-v|version|models) exit 0;; esac
prompt=$(cat)
printf 'launched-binary=codex\n' >> {log}
case " $* " in
  *" resume {session_id} "*) printf 'resume-session-ok\n' >> {log} ;;
esac
case "$prompt" in
  *"finish the switched work"*) printf 'follow-up-ok\n' >> {log} ;;
esac
printf '%s\n' '{{"type":"thread.started","thread_id":"{session_id}"}}'
printf '%s\n' '{{"type":"item.completed","item":{{"type":"agent_message","text":"ok"}}}}'
exit 0
"#,
            log = events_log.display(),
            session_id = session_id,
        ),
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

/// Resume compatibility (AC15), end to end: a resume opened with a launch flag
/// the resume path intentionally drops must still be treated as *compatible*, so
/// the provider is genuinely resumed rather than falsely refused.
///
/// `--append-system-prompt` is on the opening attempt's argv but is not one of
/// the flags [`append_resume_passthrough_args`] carries into the resume argv, so
/// the resumed attempt's *spawned* argv no longer contains it. The session-
/// compatibility key reads the invocation's canonical (pre-resume-normalization)
/// argv for the system-prompt facet precisely so this intentional drop does not
/// register as a refresh-time change. This proves that key correctness through a
/// real run: two provider invocations, the resume branch reached, and no
/// `resume incompatible after refresh` diagnostic.
///
/// The refusal counterpart is
/// [`level2_lifecycle_resume_refuses_when_refresh_changes_model`]; together they
/// pin both directions of the AC15 comparison, so neither an over-eager nor an
/// absent guard can pass.
#[test]
#[serial(level2_lifecycle_control)]
fn level2_lifecycle_resume_with_dropped_launch_flag_stays_compatible() {
    require_level!(Level::L2, TmuxHarness::available(), "tmux");

    let follow_up = "please finish the resumed work";
    let doc = format!(
        r#"---
title: resume compat with system prompt
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
    let session_id = "compat-session-abc";
    write_resumable_claude(&bin_dir, &events_log, session_id, follow_up);

    // `--append-system-prompt` reads a FILE; its content is delivered to the
    // provider on the opening attempt's argv and dropped from the resume argv.
    let sysprompt = workspace.path().join("sysprompt.txt");
    fs::write(&sysprompt, "stay terse\n").unwrap();

    let md_file = workspace.path().join("doc.md");
    fs::write(&md_file, doc).unwrap();
    let staged = Staged {
        workspace,
        bin_dir,
        md_file,
        events_log,
        rendezvous_endpoint: None,
    };

    let extra_flags = format!("--append-system-prompt {}", sysprompt.display());
    let pane = run_provider_with_flags(&staged, "--claude", &extra_flags, "finalize");

    let lines = event_lines(&staged);
    assert_eq!(
        lines.iter().filter(|l| **l == "provider-ran").count(),
        2,
        "the session opened with a system prompt must still resume (two provider runs); \
         got {lines:?}; pane:\n{pane}"
    );
    assert!(
        lines.iter().any(|l| l == "resume-session-ok"),
        "the resume must reach the provider's resume branch, not be refused; \
         got {lines:?}; pane:\n{pane}"
    );
    assert!(
        !pane.contains("resume incompatible"),
        "a resume that only dropped a non-whitelisted launch flag must not be refused \
         as incompatible; pane:\n{pane}"
    );
    assert_eq!(
        lines,
        [
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
        ],
        "the resumed attempt must run start→success→finalize with no incompatibility refusal; \
         pane:\n{pane}"
    );
}

/// Collapse a captured pane into one whitespace-normalized line.
///
/// Rendered `StatusBlock` prose is hard-wrapped to the pane width, so a phrase
/// the test cares about ("resume incompatible after refresh") is routinely split
/// across two lines. Normalizing first lets the assertions name the phrase the
/// operator reads instead of whichever fragment happened to fit.
///
/// The block's vertical border glyphs are dropped before normalizing: a wrap
/// that lands mid-phrase puts a `┃` *inside* a backticked facet name, so a
/// naive normalization would still miss `` `model` `` for a purely cosmetic
/// reason.
fn flattened(pane: &str) -> String {
    pane.replace(['┃', '│', '┆', '┊'], " ")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

/// Resume incompatibility (AC15), end to end: when the canonical refresh that
/// precedes a `resume` changes a launch facet the provider fixed when it opened
/// the session, the resume must be **refused** — no second provider launch, a
/// diagnostic naming the changed facet, and a recommendation to retry.
///
/// This is the first row of the refusal matrix, driven through `model:`. The
/// document opens pinned to one `model:`; its `failure` stack rewrites that
/// frontmatter with `set_frontmatter` and *then* asks to `resume`. The resume's
/// fresh-read boundary re-reads the mutated document and rebuilds the launch
/// bundle from that read (`target_launch::rebuild_launch_identity`), so the
/// refreshed `MODEL` reaches the child environment the session-compatibility key
/// is computed from — and no longer matches the key the opening attempt stored.
/// Before the per-attempt rebuild landed, the launch identity was frozen at
/// invocation and this refusal had no reachable trigger at all.
///
/// Both models are `llamacpp/`-namespaced, which `matches_offering_source`
/// accepts for Claude by construction, so the row does not depend on the host's
/// live model listings.
///
/// This is the first row of the AC15 refusal matrix; the rest, and the account
/// of which facets have no reachable row, follow immediately below.
#[test]
#[serial(level2_lifecycle_control)]
fn level2_lifecycle_resume_refuses_when_refresh_changes_model() {
    require_level!(Level::L2, TmuxHarness::available(), "tmux");

    let follow_up = "please finish the resumed work";
    let doc = format!(
        r#"---
title: resume refused after model refresh
model: llamacpp/opener-model
start:
  stack:
    - action: {{append_line: ["events.log", "start"]}}
failure:
  stack:
    - action: {{append_line: ["events.log", "failure"]}}
    - action: {{set_frontmatter: ["doc.md", "model", "llamacpp/refreshed-model"]}}
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
    let session_id = "refused-session-abc";
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

    let pane = run_provider_await_exit(&staged, "--claude");
    let flat = flattened(&pane);

    // The mutation must actually have landed; otherwise a green refusal
    // assertion below would be proving nothing about a *changed* facet.
    let refreshed = fs::read_to_string(&staged.md_file).unwrap();
    assert!(
        refreshed.contains("llamacpp/refreshed-model"),
        "the failure stack must have rewritten the document's model before the resume; \
         document:\n{refreshed}"
    );

    let lines = event_lines(&staged);
    assert_eq!(
        lines.iter().filter(|l| **l == "provider-ran").count(),
        1,
        "a refused resume must not launch the provider a second time; \
         got {lines:?}; pane:\n{pane}"
    );
    assert!(
        !lines.iter().any(|l| l == "resume-session-ok"),
        "the live session must never be handed the newly prepared launch plan; \
         got {lines:?}; pane:\n{pane}"
    );
    assert_eq!(
        lines,
        REFUSED_RESUME_EVENTS,
        "the refusal must land after the resumed attempt's `start`, before any second \
         provider spawn, and close the run through `failure` then `finalize`; pane:\n{pane}"
    );
    assert!(
        flat.contains("resume incompatible after refresh"),
        "the refusal must surface the typed diagnostic header; pane:\n{pane}"
    );
    assert!(
        flat.contains("`model`"),
        "the diagnostic must name the changed facet; pane:\n{pane}"
    );
    assert!(
        flat.contains("Use `retry` to start a fresh session with the new plan"),
        "the diagnostic must recommend retry as the way forward; pane:\n{pane}"
    );
}

// -- AC15 resume-compatibility refusal matrix ------------------------------
//
// Each row below drives a *real* refusal through the shipped binary: the
// document opens a live provider session, its `failure` stack mutates the very
// input a launch facet is derived from, and then asks to `resume`. The resume's
// fresh-read boundary rebuilds the launch identity from that mutated read
// (`target_launch::rebuild_launch_identity`), the compatibility key moves, and
// the resume is refused before any second spawn.
//
// ## Facets with no row of their own, and why
//
// Two facets have no reachable end-to-end refusal, and are proven at the
// projection layer (L1 `session_key::tests`) instead:
//
// - **`workspace CWD`** has no document surface at all — it comes from the launch
//   workspace and `--repo`, both invocation-fixed.
// - **`system prompt`** has no document surface for its *content*: a document
//   cannot author one, and rewriting the `--append-system-prompt` file
//   afterwards changes nothing because the composed text was already captured.
//   Its *delivery* is rebuilt — a provider move re-applies it in the new
//   provider's shape (`wrap::launch_plan`) — so the facet moves only as a
//   consequence of the provider moving, never on its own. Making the content
//   itself movable is review-8 finding 3's scope.
//
// A further four have no *isolating* row, because none of them has its own
// document surface: each is a function of the provider and/or the session mode,
// so the row that moves its input names it alongside that input's own facet.
// `profile/binary` and `resume protocol` move with the provider row;
// `structured-output mode` moves with both the provider and the interactivity
// row; `permission mode` moves with the provider row under `--yolo`. Each is
// asserted by name where it moves — a row that moved one in isolation would be a
// fiction.

/// Stage a document plus the resumable fake `claude` and a fake `goose`, so a
/// frontmatter `agent:` switch has real binaries on both sides of the change.
fn stage_resumable(doc: &str, session_id: &str, follow_up: &str) -> Staged {
    let workspace = tempdir().unwrap();
    let bin_dir = workspace.path().join("bin");
    fs::create_dir_all(&bin_dir).unwrap();
    seed_minimal_config(workspace.path());
    assert!(init_git_repo(workspace.path()), "git init failed");

    let events_log = workspace.path().join("events.log");
    write_resumable_claude(&bin_dir, &events_log, session_id, follow_up);
    write_succeeding_goose(&bin_dir, &events_log);

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

/// The full event trace every AC15 refusal row must produce.
///
/// The resumed attempt re-enters at `start`, so that marker is expected; the
/// refusal lands between it and the spawn, which is why `provider-ran` appears
/// only once. The tail is the ratified lifecycle contract: an error after
/// `start` routes through `failure` and then exactly one `finalize`, so a
/// document's cleanup and `err`-aware recovery still run. The refusal used to
/// propagate as a bare error and stop at the second `start`, which made it the
/// only post-`start`, pre-spawn failure that skipped its own closure.
///
/// The second `failure` re-runs the stack that asked to resume. Its `resume`
/// control action is *not* dispatched from this path, so the refusal cannot
/// loop back into another attempt — hence one `provider-ran`, not two.
const REFUSED_RESUME_EVENTS: [&str; 7] = [
    "start",
    "provider-ran",
    "initial-prompt-ok",
    "failure",
    "start",
    "failure",
    "finalize",
];

/// Assert the shared shape of an AC15 refusal: the provider ran exactly once,
/// the live session was never handed the new plan, the run still closed through
/// `failure` then `finalize`, the diagnostic names every expected facet, and it
/// recommends `retry`.
fn assert_resume_refused(staged: &Staged, pane: &str, expected_facets: &[&str]) {
    let flat = flattened(pane);
    let lines = event_lines(staged);
    assert_eq!(
        lines.iter().filter(|l| **l == "provider-ran").count(),
        1,
        "a refused resume must not launch the provider a second time; \
         got {lines:?}; pane:\n{pane}"
    );
    assert!(
        !lines.iter().any(|l| l == "resume-session-ok"),
        "the live session must never be handed the newly prepared launch plan; \
         got {lines:?}; pane:\n{pane}"
    );
    assert_eq!(
        lines,
        REFUSED_RESUME_EVENTS,
        "a refusal is a post-`start` failure and owes the lifecycle tail: \
         `failure` then exactly one `finalize`; pane:\n{pane}"
    );
    assert!(
        flat.contains("resume incompatible after refresh"),
        "the refusal must surface the typed diagnostic header; pane:\n{pane}"
    );
    for facet in expected_facets {
        assert!(
            flat.contains(&format!("`{facet}`")),
            "the diagnostic must name the changed facet `{facet}`; pane:\n{pane}"
        );
    }
    assert!(
        flat.contains("Use `retry` to start a fresh session with the new plan"),
        "the diagnostic must recommend retry as the way forward; pane:\n{pane}"
    );
}

/// AC15 — `provider` (and the `profile/binary`, `resume protocol`, and
/// `structured-output mode` that follow from it).
///
/// The run passes **no** provider flag, so the document's own `agent:` selects
/// the provider — which is precisely what makes the facet movable. The `failure`
/// stack rewrites `agent: claude` to `agent: goose` and then asks to resume, so
/// the refreshed read resolves a different provider, a different binary, a
/// different resume protocol, and (Goose declares no stream protocol where Claude
/// declares `stream-json`) a different structured-output mode.
#[test]
#[serial(level2_lifecycle_control)]
fn level2_lifecycle_resume_refuses_when_refresh_changes_provider() {
    require_level!(Level::L2, TmuxHarness::available(), "tmux");

    let follow_up = "please finish the resumed work";
    let doc = format!(
        r#"---
title: resume refused after provider refresh
agent: claude
start:
  stack:
    - action: {{append_line: ["events.log", "start"]}}
failure:
  stack:
    - action: {{append_line: ["events.log", "failure"]}}
    - action: {{set_frontmatter: ["doc.md", "agent", "goose"]}}
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
    let staged = stage_resumable(&doc, "provider-refused-abc", follow_up);

    let pane = run_compose_await_exit_with_args(&staged, "");

    let refreshed = fs::read_to_string(&staged.md_file).unwrap();
    assert!(
        refreshed.contains("agent: goose"),
        "the failure stack must have rewritten the document's agent before the resume; \
         document:\n{refreshed}"
    );

    assert_resume_refused(
        &staged,
        &pane,
        &["provider", "profile/binary", "resume protocol"],
    );
}

/// AC15 — `interactivity` (and the `structured-output mode` it implies).
///
/// `interactive:` is the one frontmatter surface over session shape. The
/// `failure` stack writes `interactive: true` and then asks to resume, so the
/// refreshed read resolves an interactive session — and structured streaming,
/// which is non-interactive only, drops with it. Provider stays pinned by
/// `--claude`, which isolates this pair from the provider row above.
#[test]
#[serial(level2_lifecycle_control)]
fn level2_lifecycle_resume_refuses_when_refresh_changes_interactivity() {
    require_level!(Level::L2, TmuxHarness::available(), "tmux");

    let follow_up = "please finish the resumed work";
    let doc = format!(
        r#"---
title: resume refused after interactivity refresh
start:
  stack:
    - action: {{append_line: ["events.log", "start"]}}
failure:
  stack:
    - action: {{append_line: ["events.log", "failure"]}}
    - action: {{set_frontmatter: ["doc.md", "interactive", true]}}
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
    let staged = stage_resumable(&doc, "interactivity-refused-abc", follow_up);

    let pane = run_compose_await_exit_with_args(&staged, "--claude");

    let refreshed = fs::read_to_string(&staged.md_file).unwrap();
    assert!(
        refreshed.contains("interactive: true"),
        "the failure stack must have rewritten the document's interactivity before the \
         resume; document:\n{refreshed}"
    );

    assert_resume_refused(
        &staged,
        &pane,
        &["interactivity", "structured-output mode"],
    );
}

/// AC15 — `permission mode`.
///
/// `--yolo` is invocation intent, but whether the provider *achieves* the bypass
/// depends on which provider runs: Claude takes a direct flag, Pi declares no
/// bypass mechanism at all. The `failure` stack switches `agent:` to `pi`, so the
/// refreshed read resolves `prompt` where the live session was opened under
/// `bypass`.
#[test]
#[serial(level2_lifecycle_control)]
fn level2_lifecycle_resume_refuses_when_refresh_changes_permission_mode() {
    require_level!(Level::L2, TmuxHarness::available(), "tmux");

    let follow_up = "please finish the resumed work";
    let doc = format!(
        r#"---
title: resume refused after permission-mode refresh
agent: claude
start:
  stack:
    - action: {{append_line: ["events.log", "start"]}}
failure:
  stack:
    - action: {{append_line: ["events.log", "failure"]}}
    - action: {{set_frontmatter: ["doc.md", "agent", "pi"]}}
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
    let staged = stage_resumable(&doc, "permission-refused-abc", follow_up);

    let pane = run_compose_await_exit_with_args(&staged, "--yolo");

    assert_resume_refused(&staged, &pane, &["permission mode"]);
}

/// AC15 — `MCP server set`.
///
/// With MCP in play (`--mcp`), the `#tag`s in a document's body are what select
/// its servers. The `failure` stack appends a tagged line to the document body
/// and then asks to resume, so a fresh preparation of that document would select
/// a different server set than the live session was opened with.
#[test]
#[serial(level2_lifecycle_control)]
fn level2_lifecycle_resume_refuses_when_refresh_changes_mcp_server_set() {
    require_level!(Level::L2, TmuxHarness::available(), "tmux");

    let follow_up = "please finish the resumed work";
    let doc = format!(
        r#"---
title: resume refused after mcp tag refresh
start:
  stack:
    - action: {{append_line: ["events.log", "start"]}}
failure:
  stack:
    - action: {{append_line: ["events.log", "failure"]}}
    - action: {{append_line: ["doc.md", "also check #calendar"]}}
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
    let staged = stage_resumable(&doc, "mcp-refused-abc", follow_up);
    write_resumable_codex(
        &staged.bin_dir,
        &staged.events_log,
        "mcp-refused-abc",
        follow_up,
    );

    let pane = run_compose_await_exit_with_args(&staged, "--codex --mcp");

    let refreshed = fs::read_to_string(&staged.md_file).unwrap();
    assert!(
        refreshed.contains("#calendar"),
        "the failure stack must have added an MCP tag to the body before the resume; \
         document:\n{refreshed}"
    );

    assert_resume_refused(&staged, &pane, &["MCP server set"]);
}

/// **Review-11 finding 3 — AC15 over a *composed* MCP tag set.**
///
/// The row above moves the server set by appending a literal `#calendar` to the
/// body, which a rebuild sees whether it reads the prepared document or the raw
/// file. This row moves it by `set_frontmatter`, leaving the body's
/// `#{{ probe }}` template untouched: the two reads of the file on disk are
/// byte-identical in the body, and only composition resolves the tag.
///
/// The refusal is therefore reachable only for an implementation that takes the
/// tag set from the composed document. Re-lexing the raw source sees no tag
/// before *or* after the mutation, judges the MCP facet unmoved, and admits the
/// resume — handing the live session a launch plan built for a different server
/// set, which is precisely what AC15 forbids.
#[test]
#[serial(level2_lifecycle_control)]
fn level2_lifecycle_resume_refuses_when_refresh_changes_an_interpolated_mcp_tag() {
    require_level!(Level::L2, TmuxHarness::available(), "tmux");

    let follow_up = "please finish the resumed work";
    let doc = format!(
        r#"---
title: resume refused after interpolated mcp tag refresh
probe: calendar
start:
  stack:
    - action: {{append_line: ["events.log", "start"]}}
failure:
  stack:
    - action: {{append_line: ["events.log", "failure"]}}
    - action: {{set_frontmatter: ["doc.md", "probe", "slack"]}}
    - action: {{resume: "{follow_up}"}}
success:
  stack:
    - action: {{append_line: ["events.log", "success"]}}
finalize:
  stack:
    - action: {{append_line: ["events.log", "finalize"]}}
---
Original body naming its server only after composition #{{{{ probe }}}} here
"#
    );
    let staged = stage_resumable(&doc, "mcp-interp-refused-abc", follow_up);
    write_resumable_codex(
        &staged.bin_dir,
        &staged.events_log,
        "mcp-interp-refused-abc",
        follow_up,
    );

    let pane = run_compose_await_exit_with_args(&staged, "--codex --mcp");

    let refreshed = fs::read_to_string(&staged.md_file).unwrap();
    let (_, body) = refreshed.rsplit_once("---\n").unwrap();
    assert!(
        refreshed.contains("probe: slack"),
        "the failure stack must have moved the frontmatter value before the resume; \
         document:\n{refreshed}"
    );
    assert!(
        body.contains("#{{ probe }}") && !body.contains("slack") && !body.contains("calendar"),
        "fixture check: the body must be unchanged and carry no literal server id, \
         so only the composed tag set moved; body:\n{body}"
    );

    assert_resume_refused(&staged, &pane, &["MCP server set"]);
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

/// Stage a source/target pair (both under the sentinel-gated proxy goose) for a
/// terminal-event proxy into a *looping* target. `write_proxy_goose` exits 0
/// only when an argument carries `target_sentinel`, so the source (no sentinel)
/// fails and the looping target (sentinel in its body) succeeds every iteration.
fn stage_proxy_pair_gated(source_doc: &str, target_doc: &str, target_sentinel: &str) -> Staged {
    let workspace = tempdir().unwrap();
    let bin_dir = workspace.path().join("bin");
    fs::create_dir_all(&bin_dir).unwrap();
    seed_minimal_config(workspace.path());
    assert!(init_git_repo(workspace.path()), "git init failed");

    let events_log = workspace.path().join("events.log");
    write_proxy_goose(&bin_dir, &events_log, target_sentinel);

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

/// Run `claudine compose --goose <doc>` in a real tmux pane and block until the
/// `marker` line has been recorded at least `expected` times (a looping run's
/// terminal marker is not unique, so we settle on a count) or the total line
/// count goes stable (a run that finished short of `expected` — e.g. a
/// regression that dropped the target's loop). Bounded under the nextest
/// slow-timeout so a non-producing run never hangs the suite.
fn run_compose_settle(staged: &Staged, marker: &str, expected: usize) -> String {
    static SEQ: AtomicU32 = AtomicU32::new(0);
    let seq = SEQ.fetch_add(1, Ordering::Relaxed);

    let session = format!("biscuit_l2_lcctl_settle_{}_{seq}", std::process::id());
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
    let sentinel = format!("L2_CTL_SETTLE_{seq}");
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

    let deadline = Instant::now() + Duration::from_secs(30);
    let mut last_total = 0usize;
    let mut stable_since: Option<Instant> = None;
    while Instant::now() < deadline {
        let lines = event_lines(staged);
        if lines.iter().filter(|l| l.as_str() == marker).count() >= expected {
            std::thread::sleep(Duration::from_millis(200));
            break;
        }
        let total = lines.len();
        if total == last_total {
            match stable_since {
                Some(since) if since.elapsed() >= Duration::from_millis(1200) => break,
                Some(_) => {}
                None => stable_since = Some(Instant::now()),
            }
        } else {
            stable_since = None;
            last_total = total;
        }
        std::thread::sleep(Duration::from_millis(100));
    }

    let pane = harness.capture().map(|f| f.plain).unwrap_or_default();
    kill_session_by_name(&session);
    pane
}

/// Finding 2 (review-5): a terminal-event (`failure`) proxy to a *looping*
/// target must run that target through the full canonical launch pipeline, so
/// the target acquires its `loop:` and iterates exactly as many times as a
/// direct invocation of the same target. This proves the terminal proxy is
/// surfaced to the command coordinator (R6/R7) rather than adopted in-harness
/// (which rebuilds only AGENT/MODEL/YOLO and never re-runs loop recognition, so
/// a proxied looping target would run a single iteration). The existing
/// terminal-proxy fixture uses a non-looping target and cannot catch this
/// regression, because adopt-in-harness and surface produce identical output
/// for a one-shot target.
#[test]
#[serial(level2_lifecycle_control)]
fn level2_lifecycle_failure_proxy_to_looping_target_matches_direct_iterations() {
    require_level!(Level::L2, TmuxHarness::available(), "tmux");

    // The target's body carries this sentinel so the gated goose exits 0 on
    // every iteration; the source body omits it, so the source fails and
    // proxies.
    let target_sentinel = "LOOP_TARGET_BODY_MARKER";

    // Looping target: phase 1 → `until: phase > 2` runs iterations at phase
    // 1, 2, 3, so the loop-gate stack records `target-iter` exactly three
    // times (the gate observes pre-increment phase each pass).
    let target_doc = format!(
        "---\ntitle: proxy loop target\nphase: 1\nloop:\n  until: \"phase > 2\"\n  \
         action: \"increment(phase)\"\n  max: 10\n  stack:\n    \
         - action: {{append_line: ['events.log', 'target-iter']}}\n---\n\
         {target_sentinel} phase {{{{phase}}}}\n"
    );

    // Baseline: invoke the looping target directly (it IS `doc.md` here).
    let direct = stage_proxy_pair_gated(&target_doc, &target_doc, target_sentinel);
    let direct_pane = run_compose_settle(&direct, "target-iter", 3);
    let direct_iters = event_lines(&direct)
        .iter()
        .filter(|l| l.as_str() == "target-iter")
        .count();
    assert_eq!(
        direct_iters, 3,
        "direct invocation of the looping target must iterate 3 times; \
         got {:?}; pane:\n{direct_pane}",
        event_lines(&direct)
    );

    // Proxy: a source whose provider fails hands off at `failure` to the same
    // looping target.
    let source_doc = "---\ntitle: proxy loop source\nfailure:\n  stack:\n    \
         - action: {append_line: ['events.log', 'source-failure']}\n    \
         - action: {proxy: '@target.md'}\n---\nsource body\n";
    let proxy = stage_proxy_pair_gated(source_doc, &target_doc, target_sentinel);
    let proxy_pane = run_compose_settle(&proxy, "target-iter", 3);
    let proxy_lines = event_lines(&proxy);
    let proxy_iters = proxy_lines
        .iter()
        .filter(|l| l.as_str() == "target-iter")
        .count();

    // The source proxied via its `failure` terminal event exactly once.
    assert_eq!(
        proxy_lines.iter().filter(|l| l.as_str() == "source-failure").count(),
        1,
        "the source must fail and proxy once (terminal-event route); \
         got {proxy_lines:?}; pane:\n{proxy_pane}"
    );
    // Loop-ownership equivalence: the proxied looping target iterates the same
    // number of times as the direct invocation. Adopt-in-harness would run a
    // single iteration here (loop recognition happens only in the coordinator's
    // re-prepare), so any count other than the direct baseline is a regression.
    assert_eq!(
        proxy_iters, direct_iters,
        "a terminal-event proxy to a looping target must iterate as many times \
         as a direct invocation (got proxy={proxy_iters}, direct={direct_iters}); \
         a lower proxy count means the terminal proxy was adopted in-harness \
         instead of surfaced to the coordinator; pane:\n{proxy_pane}"
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
///
/// Waits for the compose command to *exit* rather than for `target-init` in the
/// log. The log marker is written by the target's `initialize`, which runs
/// **before** the back-proxy is refused — so waiting on it and capturing after a
/// fixed grace period raced the error block onto the pane, and the race widened
/// when Phase 12 gave `LifecycleProxyCycle` a real `StatusBlock` (a taller block
/// takes longer to render than the bare `Display` line it replaced). The
/// sentinel is echoed only once claudine has exited and flushed, so it is the
/// honest wait for an assertion about final output.
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
    let pane = run_compose_await_exit(&staged);

    let lines = event_lines(&staged);

    assert!(
        lines.iter().any(|l| l == "target-init"),
        "target initialize must run before the back-proxy; got {lines:?}; pane:\n{pane}"
    );
    assert!(
        pane.contains("CompositionError: proxy chain forms a cycle")
            && pane.contains("hands off to")
            && pane.contains("Active chain")
            && pane.contains("hop limit of 16"),
        "the typed cycle diagnostic must surface its reason, active chain, and bound; pane:\n{pane}"
    );
}

// ── AC29: initialize handoff-refusal routing ────────────────────────────────
//
// A live `initialize` proxy is committed against the invocation-wide ledger
// *while the source's `initialize` guard is still live* (the commit-while-live
// contract). A refused hop — a missing target, a cycle, or a hop-limit overrun —
// must therefore route through the source's still-legal `blocked` then
// `finalize` with the typed `err` available, synthesize no duplicate terminal or
// `finalize`, and never activate the target. These rows assert that ordered
// behavior for each refusal reason; the pre-existing `..._cycle_guarded` row
// above only checks the diagnostic text, not the source's catch stacks.

/// The three catch-stack blocks every AC29 refusal source shares: `blocked`
/// records the typed `err.kind`/`err.variant`, `finalize` records `err.msg`
/// under a `when: err` guard and then a bare `source-finalize` marker (so a
/// duplicate `finalize` would show two of them).
const AC29_CATCH_STACKS: &str = "blocked:\n  stack:\n    \
     - action: {append_line: ['events.log', \"{{ 'blocked-kind=' + err.kind }}\"]}\n    \
     - action: {append_line: ['events.log', \"{{ 'blocked-variant=' + err.variant }}\"]}\nfinalize:\n  stack:\n    \
     - when: \"err\"\n      action: {append_line: ['events.log', \"{{ 'finalize-msg=' + err.msg }}\"]}\n    \
     - action: {append_line: ['events.log', 'source-finalize']}\n";

/// Assert the shared AC29 outcome: the source `initialize` ran exactly once, the
/// provider never launched, and `blocked` → `finalize` fired once each with the
/// typed `err` payload observable.
fn assert_ac29_source_catch(lines: &[String], pane: &str) {
    assert_eq!(
        lines.iter().filter(|l| **l == "source-init").count(),
        1,
        "the source initialize must run exactly once (no target activation re-runs it); \
         got {lines:?}; pane:\n{pane}"
    );
    assert_eq!(
        lines.iter().filter(|l| **l == "provider-ran").count(),
        0,
        "a refused initialize handoff must not launch any provider; got {lines:?}; pane:\n{pane}"
    );
    assert!(
        lines
            .iter()
            .any(|l| l.starts_with("blocked-kind=") && l.len() > "blocked-kind=".len()),
        "the source blocked.stack must fire and observe a non-empty err.kind; \
         got {lines:?}; pane:\n{pane}"
    );
    assert!(
        lines
            .iter()
            .any(|l| l.starts_with("blocked-variant=") && l.len() > "blocked-variant=".len()),
        "the source blocked.stack must observe a non-empty err.variant; got {lines:?}; pane:\n{pane}"
    );
    assert!(
        lines
            .iter()
            .any(|l| l.starts_with("finalize-msg=") && l.len() > "finalize-msg=".len()),
        "the source finalize.stack `when: err` must be truthy and observe a non-empty err.msg; \
         got {lines:?}; pane:\n{pane}"
    );
    assert_eq!(
        lines.iter().filter(|l| **l == "source-finalize").count(),
        1,
        "the source finalize must fire exactly once — no duplicate/synthetic finalize; \
         got {lines:?}; pane:\n{pane}"
    );
}

/// AC29 — a **missing target**: the source's `initialize` proxies to a document
/// that does not resolve. The commit-while-live resolution fails, and the still-
/// live source routes it through `blocked` → `finalize` with `err`.
#[test]
#[serial(level2_lifecycle_control)]
fn level2_lifecycle_initialize_proxy_missing_target_routes_source_blocked_finalize() {
    require_level!(Level::L2, TmuxHarness::available(), "tmux");

    let source_doc = format!(
        "---\ntitle: refuse source\ninitialize:\n  stack:\n    \
         - action: {{append_line: ['events.log', 'source-init']}}\n    \
         - action: {{proxy: '@no-such-target.md'}}\n{catch}---\nsource body\n",
        catch = AC29_CATCH_STACKS,
    );
    let staged = stage(&source_doc);
    let pane = run_in_tmux_for(&staged, "source-finalize");
    let lines = event_lines(&staged);

    assert_ac29_source_catch(&lines, &pane);
    assert!(
        pane.contains("no-such-target.md") || pane.to_lowercase().contains("could not"),
        "the missing-target diagnostic must name the unresolved reference; pane:\n{pane}"
    );
}

/// AC29 — a **cycle**: the source's `initialize` proxies back to itself. The
/// ledger (seeded with the source) refuses the first hop as a cycle, and the
/// still-live source routes it through `blocked` → `finalize` with `err`. The
/// source is the cycle target, so "no target activation" means its `initialize`
/// must not run a second time.
#[test]
#[serial(level2_lifecycle_control)]
fn level2_lifecycle_initialize_proxy_cycle_routes_source_blocked_finalize() {
    require_level!(Level::L2, TmuxHarness::available(), "tmux");

    let source_doc = format!(
        "---\ntitle: cycle source\ninitialize:\n  stack:\n    \
         - action: {{append_line: ['events.log', 'source-init']}}\n    \
         - action: {{proxy: '@doc.md'}}\n{catch}---\nsource body\n",
        catch = AC29_CATCH_STACKS,
    );
    let staged = stage(&source_doc);
    let pane = run_in_tmux_for(&staged, "source-finalize");
    let lines = event_lines(&staged);

    assert_ac29_source_catch(&lines, &pane);
    assert!(
        pane.contains("cycle") || pane.contains("hop limit"),
        "the LifecycleProxyCycle diagnostic must surface; pane:\n{pane}"
    );
}

/// AC29 — a **hop-limit** overrun: a chain of `initialize` proxies reaches
/// [`MAX_PROXY_HOPS`] documents, and the final hop is refused. The last still-
/// live document in the chain routes the refusal through its own `blocked` →
/// `finalize` with `err`; the over-limit target never activates.
///
/// `MAX_PROXY_HOPS` is 16 and the ledger seeds the chain with the caller
/// (`doc.md`, length 1), so `doc.md` → `h1` → … → `h15` fills the chain to 16
/// and `h15`'s hop to the (existing) `h16` is the overrun. Only `h15` carries
/// catch stacks; the earlier links hand off cleanly (no synthetic terminal).
#[test]
#[serial(level2_lifecycle_control)]
fn level2_lifecycle_initialize_proxy_hop_limit_routes_source_blocked_finalize() {
    require_level!(Level::L2, TmuxHarness::available(), "tmux");

    // `doc.md` proxies into the chain; its `source-init` marker doubles as the
    // shared assertion's "exactly one source-init" anchor.
    let doc = "---\ntitle: hop chain head\ninitialize:\n  stack:\n    \
         - action: {append_line: ['events.log', 'source-init']}\n    \
         - action: {proxy: '@h1.md'}\n---\nhead body\n";
    let staged = stage(doc);
    let ws = staged.workspace.path();

    // h1..h14 hand off cleanly to the next link.
    for i in 1..=14 {
        let body = format!(
            "---\ntitle: hop {i}\ninitialize:\n  stack:\n    \
             - action: {{proxy: '@h{next}.md'}}\n---\nhop {i} body\n",
            next = i + 1,
        );
        fs::write(ws.join(format!("h{i}.md")), body).unwrap();
    }
    // h15 is the last still-live document: its hop to h16 overruns the limit, so
    // its own blocked/finalize catch stacks must fire. It reuses the source
    // marker names so the shared assertion applies unchanged.
    let h15 = format!(
        "---\ntitle: hop 15\ninitialize:\n  stack:\n    \
         - action: {{proxy: '@h16.md'}}\n{catch}---\nhop 15 body\n",
        catch = AC29_CATCH_STACKS,
    );
    fs::write(ws.join("h15.md"), h15).unwrap();
    // h16 exists so the refusal is a hop-limit overrun, not a missing target;
    // if it ever launched a provider the `provider-ran` guard would catch it.
    let h16 = "---\ntitle: hop 16 target\nsuccess:\n  stack:\n    \
         - action: {append_line: ['events.log', 'h16-success']}\n---\nhop 16 body\n";
    fs::write(ws.join("h16.md"), h16).unwrap();

    let pane = run_in_tmux_for(&staged, "source-finalize");
    let lines = event_lines(&staged);

    assert_ac29_source_catch(&lines, &pane);
    assert!(
        !lines.iter().any(|l| l == "h16-success"),
        "the over-limit target must never activate; got {lines:?}; pane:\n{pane}"
    );
    assert!(
        pane.contains("cycle") || pane.contains("hop limit"),
        "the hop-limit diagnostic must surface; pane:\n{pane}"
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
/// pre-flight already passed).
///
/// ## The migrated values
///
/// This site used to wrap the underlying error via
/// `from_action_failure("harness_attempt", ...)`, so the stacks observed the
/// synthesized labels `err.kind = "LifecycleAction"` / `err.variant =
/// "harness_attempt"` — which described *the machinery that noticed* rather than
/// the failure. Error-propagation Phase 5 (§D7) passes the typed diagnostic
/// through, and `err.kind`/`err.variant` are now deprecated aliases of
/// `err.category`/`err.code`, so the stacks observe `config` /`config.invalid`.
///
/// That is strictly more useful here, and worth stating plainly: a malformed
/// `exit_expressions` regex genuinely **is** a config-validation failure. An
/// author writing `when: err.category == "config"` now matches it; under the
/// old labels no faceted clause could.
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
    // `attempt_result?` site passes the typed error through, so the stacks see
    // its real facets rather than a synthesized label (see the doc comment).
    assert!(
        lines.iter().any(|l| l == "failure-kind=config"),
        "failure.stack must fire and observe the typed error's category \
         (err.kind='config'); got {lines:?}; pane:\n{pane}"
    );
    assert!(
        lines.iter().any(|l| l == "failure-variant=config.invalid"),
        "failure.stack must observe the typed error's code \
         (err.variant='config.invalid'); got {lines:?}; pane:\n{pane}"
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
    run_until_settled_with_flags(staged, "--goose", expected_lines)
}

/// [`run_until_settled`] with the claudine flags spelled out.
///
/// Rows whose purpose is to prove *target-owned* selection must pass an empty
/// `flags` string: a pinned `--goose` is explicit invocation intent that
/// outranks the target's authored `agent:`, which would make the assertion
/// vacuous.
fn run_until_settled_with_flags(staged: &Staged, flags: &str, expected_lines: usize) -> String {
    run_until_settled_with_params(staged, flags, "", expected_lines, SettlePacing::default())
}

/// How long [`run_until_settled_with_params`] waits overall, and how long a
/// stalled marker count must hold before it concludes the run finished short.
///
/// The defaults suit the synthetic fixtures, whose iterations are a few hundred
/// milliseconds apart. A document that does substantially more per-iteration
/// work — schema validation, file-reference resolution, and composing a large
/// body, as the shipped `implement-plan.md` does — needs a wider stability
/// window, or the gap *between* two healthy iterations reads as "settled".
struct SettlePacing {
    deadline: Duration,
    stable_for: Duration,
}

impl Default for SettlePacing {
    fn default() -> Self {
        Self {
            deadline: Duration::from_secs(40),
            stable_for: Duration::from_millis(1200),
        }
    }
}

/// [`run_until_settled_with_flags`] plus trailing `key=value` composition
/// setters, appended after the document path exactly as a user would type them.
///
/// Used by the shipped-implement-route row, whose documents take their
/// `spec:`/`plan:` parameters from the command line.
fn run_until_settled_with_params(
    staged: &Staged,
    flags: &str,
    params: &str,
    expected_lines: usize,
    pacing: SettlePacing,
) -> String {
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
    let invocation_path = staged
        .md_file
        .strip_prefix(staged.workspace.path())
        .unwrap_or(&staged.md_file);
    let cmd = format!(
        "cd {ws} && {env_prefix}{claudine} compose {flags} {md} {params} ; echo {sentinel}",
        ws = staged.workspace.path().display(),
        md = invocation_path.display(),
    );
    harness
        .send_command_with_env(&cmd, &[])
        .expect("send compose command");

    let deadline = Instant::now() + pacing.deadline;
    let mut last_count = 0usize;
    let mut stable_since: Option<Instant> = None;
    while Instant::now() < deadline {
        let count = event_lines(staged).len();
        if count >= expected_lines {
            std::thread::sleep(Duration::from_millis(150));
            break;
        }
        // The stability break detects a run that settled *below* the expected
        // marker count (e.g. errored after a few events). Guard it on `count > 0`
        // so it never fires during startup: a proxied/adopted target does more
        // pre-launch work (re-prep + staged bootstrap) before its first event
        // than a direct run, and an unguarded window would false-break with an
        // empty log while that setup is still in flight.
        if count == last_count && count > 0 {
            match stable_since {
                Some(since) if since.elapsed() >= pacing.stable_for => break,
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

/// A router that *owns* a `loop:` yet hands off at `initialize` before its own
/// loop begins. Before Phase 10 the loop engine surfaced the handoff and it was
/// refused with `LifecycleInitializeFailed` ("a document with `loop:` frontmatter
/// cannot hand off yet"). It must now be honored: the source loop never begins,
/// and the target owns execution.
const EQUIV_LOOP_ROUTER: &str = r#"---
title: proxy loop router
phase: 1
loop:
  until: "phase > 2"
  action: "increment(phase)"
  max: 10
initialize:
  stack:
    - action: {proxy: "@target.md"}
finalize:
  stack:
    - action: {append_line: ["events.log", "router-finalize:{{phase}}"]}
---
router body phase {{phase}}
"#;

/// **The motivating bug** (`features/2026-07-13-proxy-with/spec.md`): a proxied
/// target must execute exactly as it does when invoked directly.
///
/// A router with no `loop:` proxies at `initialize` to a looping target. Before
/// Phase 10, loop-vs-single was decided at `cli/src/commands/compose/prep.rs`
/// from the **router's** frontmatter, before the router's `initialize` proxy
/// fired — so the routed run treated the target as a single-run document and
/// executed one provider attempt while the direct run executed all three
/// iterations.
///
/// Phase 10 moved loop recognition to after `initialize` routing stabilizes: an
/// `initialize` proxy to a looping target is hoisted to the composition
/// command's active-document coordinator (`compose/prep.rs`), which commits the
/// handoff, re-prepares the target as a fresh document, and gives it the same
/// document loop it would receive when invoked directly. This test is the
/// headline acceptance signal for that move.
///
/// Deterministic by construction: a fake provider, a self-contained temporary
/// fixture, and no live Claude/Codex/Gemini service.
///
/// This row owns the *synthetic* fixture, which is what exercises the loop
/// coordinator directly. The shipped `prompts/implement.md` route has its own
/// regression at
/// [`level2_lifecycle_shipped_implement_route_matches_direct_run`]; it is no
/// longer a manual-only smoke case.
#[test]
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

/// A router that owns a `loop:` but proxies at `initialize` must have its
/// hand-off **honored**, not refused — the source loop never begins and the
/// target owns execution (R7, acceptance criterion 7).
///
/// Before Phase 10 a loop-owning router's `initialize` proxy was refused with
/// `LifecycleInitializeFailed` ("a document with `loop:` frontmatter cannot hand
/// off yet"); the run exited non-zero without ever reaching the target. This
/// asserts the target now executes its own three iterations, exactly as a direct
/// invocation of the target does, and the router's own loop never runs.
#[test]
#[serial(level2_lifecycle_control)]
fn level2_lifecycle_loop_router_initialize_proxy_is_honored() {
    require_level!(Level::L2, TmuxHarness::available(), "tmux");

    // 1 target-init + 3 provider-ran + 3 target-finalize = 7 markers.
    const EXPECTED_MARKERS: usize = 7;

    let direct = stage_proxy_pair(EQUIV_LOOP_TARGET, EQUIV_LOOP_TARGET, true);
    let direct_pane = run_until_settled(&direct, EXPECTED_MARKERS);
    let direct_lines = event_lines(&direct);

    let routed = stage_proxy_pair(EQUIV_LOOP_ROUTER, EQUIV_LOOP_TARGET, true);
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

    assert_eq!(
        phases(&direct_lines),
        vec!["target-finalize:1", "target-finalize:2", "target-finalize:3"],
        "fixture check: the target loops three times when invoked directly; \
         got {direct_lines:?}; pane:\n{direct_pane}"
    );
    assert_eq!(
        phases(&routed_lines),
        phases(&direct_lines),
        "the loop-owning router's hand-off is honored: the target owns execution \
         and loops three times; routed {routed_lines:?}; pane:\n{routed_pane}"
    );
    assert_eq!(
        count(&routed_lines, "provider-ran"),
        count(&direct_lines, "provider-ran"),
        "iteration count must not depend on the route; routed {routed_lines:?}; \
         direct {direct_lines:?}"
    );
    assert_eq!(
        count(&routed_lines, "target-init"),
        1,
        "the target's initialize fires exactly once on the routed run; \
         got {routed_lines:?}; pane:\n{routed_pane}"
    );
    // The router owns a `loop:`, but its loop never begins: a clean hand-off
    // ends the source before any iteration, so its `finalize` never fires.
    assert_eq!(
        count(&routed_lines, "router-finalize:1"),
        0,
        "a clean hand-off ends the source loop before it starts; the router's \
         finalize must not fire; got {routed_lines:?}; pane:\n{routed_pane}"
    );
}

// ── The shipped `implement` route ───────────────────────────────────────────
//
// The rows above prove generic loop-ownership on a synthetic fixture. This one
// protects the *shipped artifacts* the feature was written for — their routing
// conditions and their multi-phase loop schema — from drifting independently
// (`features/2026-07-13-proxy-with/spec.md:1068-1072`).
//
// `prompts/implement.md` is executed verbatim; the target is a side-effect-free
// copy of `prompts/_implement/implement-plan.md` because the shipped file's
// `say:`, `effect:`, and `shell:` properties are respectively a real TTS call,
// real audio playback, and a pair of commands that are denied without an
// interactive approver (which would divert the run to `blocked` before the
// loop's second iteration). `tests/shipped_prompt_route_drift.rs` mechanically
// holds that copy in sync and fails when either shipped file changes.

/// A fake `goose` that records the phase heading of the body it was handed.
///
/// Goose takes the prompt on argv, not stdin, so both channels are scanned. The
/// phase is read out of the *rendered* body (`# Implement Phase N of M`), which
/// makes the loop's per-iteration mutation observable without adding any
/// marker property to the document under test.
fn write_phase_recording_goose(bin_dir: &Path, events_log: &Path) {
    write_executable(
        &bin_dir.join("goose"),
        &format!(
            "#!/bin/sh\nstdin=$(cat)\nprintf 'provider-ran\\n' >> {log}\n\
             phase=$(printf '%s %s' \"$stdin\" \"$*\" | \
             sed -n 's/.*Implement Phase \\([0-9][0-9]*\\) of \\([0-9][0-9]*\\).*/\\1-of-\\2/p' | \
             head -1)\n\
             if [ -z \"$phase\" ]; then phase=UNRESOLVED; fi\n\
             printf 'phase:%s\\n' \"$phase\" >> {log}\nexit 0\n",
            log = events_log.display(),
        ),
    );
}

/// Stage the shipped `implement` route in a self-contained workspace.
///
/// `entry` is the workspace-relative document `claudine compose` is pointed at:
/// `implement.md` for the routed run, `_implement/implement-plan.md` for the
/// direct run. Both receive the same `spec=` parameter, so the only difference
/// between the two runs is the hand-off itself.
fn stage_shipped_implement_route(entry: &str, total_phases: usize) -> Staged {
    let workspace = tempdir().unwrap();
    let root = workspace.path().to_path_buf();
    let bin_dir = root.join("bin");
    fs::create_dir_all(&bin_dir).unwrap();
    fs::create_dir_all(root.join("_implement")).unwrap();
    fs::create_dir_all(root.join("feature")).unwrap();
    seed_minimal_config(&root);
    // `implement-plan.md` derives `total_phases` from the plan's frontmatter, so
    // it is schema-`required` but never caller-supplied. On a TTY that is the
    // one input `resolve_interactive_options` would open a biscuit-tui prompt
    // for, and nothing in a tmux pane types an answer. Declining the prompt is
    // what a non-TTY invocation already does implicitly — the derived value is
    // then used — so this keeps the derivation under test instead of masking it
    // with a caller override.
    fs::write(
        root.join(".claudine/config.json"),
        "{\"prompt_for_missing\": false}",
    )
    .unwrap();
    assert!(init_git_repo(&root), "git init failed");

    let events_log = root.join("events.log");
    write_phase_recording_goose(&bin_dir, &events_log);

    let manifest = Path::new(env!("CARGO_MANIFEST_DIR"));
    let repo_root = manifest
        .ancestors()
        .nth(2)
        .expect("repository root is two levels above claudine/cli");
    fs::copy(
        repo_root.join("prompts/implement.md"),
        root.join("implement.md"),
    )
    .expect("copy the shipped router");
    fs::copy(
        manifest.join("tests/fixtures/shipped_implement_route/_implement/implement-plan.md"),
        root.join("_implement/implement-plan.md"),
    )
    .expect("copy the drift-guarded implement-plan fixture");

    // The router branches on `frontmatter(spec, 'implemented')`; an unimplemented
    // spec is the branch that reaches `implement-plan.md`.
    fs::write(
        root.join("feature/spec.md"),
        "---\nimplemented: false\n---\n\n# Spec\n",
    )
    .unwrap();
    fs::write(
        root.join("feature/plan.md"),
        format!("---\ntotal_phases: {total_phases}\nstart_phase: 1\n---\n\n# Plan\n"),
    )
    .unwrap();

    Staged {
        workspace,
        bin_dir,
        md_file: root.join(entry),
        events_log,
        rendezvous_endpoint: None,
    }
}

/// **The shipped motivating route** (`spec.md:1057-1060,1068-1072`): routing
/// `prompts/implement.md` to `prompts/_implement/implement-plan.md` must execute
/// every phase exactly as invoking `implement-plan.md` directly does.
///
/// This is the shipped-artifact counterpart to
/// [`level2_lifecycle_initialize_proxy_to_looping_target_matches_direct_run`].
/// That row owns a synthetic fixture and proves the loop coordinator honors an
/// `initialize` hand-off in general; it cannot notice the shipped router's
/// `when:` conditions being rewritten, the `$schema` gaining a required
/// parameter, or `loop.until` changing shape. This row can, because it runs the
/// real router and a mechanically-synced copy of the real target.
///
/// The phase sequence is read out of the *rendered body* the fake provider
/// receives, so the assertion is over what the agent would actually have been
/// asked to do on each iteration — not over a marker the test injected.
#[test]
#[serial(level2_lifecycle_control)]
fn level2_lifecycle_shipped_implement_route_matches_direct_run() {
    require_level!(Level::L2, TmuxHarness::available(), "tmux");

    const TOTAL_PHASES: usize = 3;
    // Each provider invocation records `provider-ran` + `phase:N-of-3`.
    const EXPECTED_MARKERS: usize = TOTAL_PHASES * 2;

    // `spec=` is what the router branches on; `plan=` is what
    // `implement-plan.md` declares `required` in its `$schema`. Both runs get
    // the identical pair, so the hand-off is the only difference between them.
    //
    // The paths are workspace-relative — the shape a user actually types, and
    // the shape the document's `'@{{area}}/{{plan}}'` body reference is written
    // for. The runner `cd`s into the workspace before invoking claudine.
    const PARAMS: &str = "spec=feature/spec.md plan=feature/plan.md";

    // The shipped document re-validates its schema, re-resolves `@`-references,
    // and re-composes a large body on every iteration, so under parallel L2 load
    // healthy iterations sit seconds apart — wide enough that the default
    // 1200ms stability window would read the gap between two good iterations as
    // "the run settled early". The deadline stays at the default so two arms
    // still fit inside the tier's 90s termination ceiling.
    let pacing = || SettlePacing {
        stable_for: Duration::from_secs(4),
        ..SettlePacing::default()
    };

    // Direct: `implement-plan.md` IS the invoked document.
    let direct = stage_shipped_implement_route("_implement/implement-plan.md", TOTAL_PHASES);
    let direct_pane = run_until_settled_with_params(
        &direct,
        "--goose",
        PARAMS,
        EXPECTED_MARKERS,
        pacing(),
    );
    let direct_lines = event_lines(&direct);

    // Routed: the shipped router is invoked and proxies to the same target.
    let routed = stage_shipped_implement_route("implement.md", TOTAL_PHASES);
    let routed_pane = run_until_settled_with_params(
        &routed,
        "--goose",
        PARAMS,
        EXPECTED_MARKERS,
        pacing(),
    );
    let routed_lines = event_lines(&routed);

    let phases = |lines: &[String]| -> Vec<String> {
        lines
            .iter()
            .filter(|l| l.starts_with("phase:"))
            .cloned()
            .collect()
    };
    let count = |lines: &[String], needle: &str| lines.iter().filter(|l| *l == needle).count();

    // Assert the direct run first: it is the contract the routed run must match,
    // so a broken fixture is not misread as a routing regression.
    assert_eq!(
        phases(&direct_lines),
        vec!["phase:1-of-3", "phase:2-of-3", "phase:3-of-3"],
        "fixture check: invoked directly, the shipped implement-plan document \
         resolves `total_phases` from the plan and runs every phase in order; \
         got {direct_lines:?}; pane:\n{direct_pane}"
    );

    assert_eq!(
        phases(&routed_lines),
        phases(&direct_lines),
        "the shipped route must execute the same phases in the same order as a \
         direct invocation; routed {routed_lines:?}; direct {direct_lines:?}; \
         pane:\n{routed_pane}"
    );
    assert_eq!(
        count(&routed_lines, "provider-ran"),
        count(&direct_lines, "provider-ran"),
        "the shipped router must not change how many provider attempts the plan \
         document makes; routed {routed_lines:?}; direct {direct_lines:?}; \
         pane:\n{routed_pane}"
    );
    assert_eq!(
        count(&routed_lines, "provider-ran"),
        TOTAL_PHASES,
        "every phase of the plan must run; got {routed_lines:?}; pane:\n{routed_pane}"
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

// ── Phase 13: the direct/proxy equivalence matrix ───────────────────────────
//
// Every row here answers one question: does this target behave identically when
// invoked directly and when reached through an `initialize` proxy? The rows are
// deliberately uniform — one probe document, two arms, one comparison — because
// the feature's whole claim is that the route is not observable.

/// Both arms of one equivalence row.
struct EquivalenceArms {
    direct_lines: Vec<String>,
    routed_lines: Vec<String>,
    direct_pane: String,
    routed_pane: String,
}

/// Rewrite the values that legitimately differ between arms into stable
/// placeholders.
///
/// Each arm owns its own temporary workspace, so every path-derived facet —
/// `ctx.area` among them — carries a random component. Comparing raw lines
/// would compare that randomness and never agree. What survives normalization
/// is exactly the routing-sensitive content the row is asserting on.
fn normalize_arm(lines: &[String], staged: &Staged) -> Vec<String> {
    let workspace = staged.workspace.path();
    let path = workspace.display().to_string();
    let area = workspace
        .file_name()
        .expect("a tempdir always has a final component")
        .to_string_lossy()
        .to_string();
    lines
        .iter()
        .map(|line| line.replace(&path, "<WS>").replace(&area, "<AREA>"))
        .collect()
}

/// Run `target_doc` directly and through the router, returning both arms'
/// normalized event logs and panes.
///
/// Both arms execute the **same file** — `target.md` — and differ only in how
/// execution reaches it: the direct arm invokes it, the routed arm invokes
/// `doc.md` (the router) and is handed off at `initialize`. Pointing the direct
/// arm at the target file itself, rather than at a copy under a second name, is
/// what keeps document-path-derived facets comparable — a copy would differ for
/// reasons that are not routing.
fn equivalence_arms(target_doc: &str, expected_markers: usize) -> EquivalenceArms {
    equivalence_arms_configured(EQUIV_ROUTER, target_doc, "--goose", expected_markers, |_| {})
}

/// [`equivalence_arms`] with the router, the claudine flags, and a per-arm
/// staging hook spelled out.
///
/// `prepare` runs against each freshly staged arm before it is executed — the
/// seam rows use to install a recording provider stub, seed an MCP catalog, or
/// write any other fixture the arm needs.
fn equivalence_arms_configured(
    router_doc: &str,
    target_doc: &str,
    flags: &str,
    expected_markers: usize,
    prepare: impl Fn(&Staged),
) -> EquivalenceArms {
    let stage = |routed: bool| {
        let mut staged = stage_proxy_pair(router_doc, target_doc, true);
        if !routed {
            staged.md_file = staged.workspace.path().join("target.md");
        }
        prepare(&staged);
        staged
    };

    let direct = stage(false);
    let direct_pane = run_until_settled_with_flags(&direct, flags, expected_markers);
    let direct_lines = normalize_arm(&event_lines(&direct), &direct);

    let routed = stage(true);
    let routed_pane = run_until_settled_with_flags(&routed, flags, expected_markers);
    let routed_lines = normalize_arm(&event_lines(&routed), &routed);

    EquivalenceArms {
        direct_lines,
        routed_lines,
        direct_pane,
        routed_pane,
    }
}

/// The equivalence probe: a target that stamps the facets the matrix compares
/// into `events.log` from its own lifecycle surfaces.
///
/// Covered here are the facets a target can observe about itself **without**
/// R6's per-document launch rebuild: authored frontmatter, a computed
/// property, the `ctx.*` snapshot, lifecycle signal order, and the target
/// `initialize` count. The launch facets (provider, model, argv, MCP, child
/// environment, child CWD) are a separate row — see
/// [`level2_lifecycle_equivalence_target_pinned_model_matches_direct_run`].
const EQUIV_PROBE_TARGET: &str = r#"---
title: equivalence probe
note: authored-note
derived: "{{ note }}-derived"
initialize:
  stack:
    - action: {append_line: ["events.log", "sig=initialize"]}
success:
  stack:
    - action:
        - {append_line: ["events.log", "sig=success"]}
        - {append_line: ["events.log", "fm.note={{ note }}"]}
        - {append_line: ["events.log", "fm.derived={{ derived }}"]}
        - {append_line: ["events.log", "ctx.area={{ ctx.area }}"]}
        - {append_line: ["events.log", "ctx.os={{ ctx.os }}"]}
finalize:
  stack:
    - action: {append_line: ["events.log", "sig=finalize"]}
---
probe body note={{ note }}
"#;

/// **Acceptance criteria 8 and 9, non-launch facets.** The probe's frontmatter,
/// computed property, `ctx.*` snapshot, lifecycle signal order, and target
/// `initialize` count are identical whether it is invoked directly or reached
/// through an `initialize` proxy.
///
/// This row covers the non-launch facets. The two facets it cannot cover —
/// launch state (R6) and loop ownership — have their own enabled rows
/// ([`level2_lifecycle_equivalence_target_pinned_model_matches_direct_run`] and
/// [`level2_lifecycle_initialize_proxy_to_looping_target_matches_direct_run`]).
#[test]
#[serial(level2_lifecycle_control)]
fn level2_lifecycle_equivalence_probe_matches_direct_run() {
    require_level!(Level::L2, TmuxHarness::available(), "tmux");

    // sig=initialize, provider-ran, sig=success, 4 stamped facets, sig=finalize.
    const EXPECTED_MARKERS: usize = 8;

    let arms = equivalence_arms(EQUIV_PROBE_TARGET, EXPECTED_MARKERS);

    // The direct arm is the contract the routed arm must match. Assert its
    // shape first so a broken fixture is not misread as a routing bug.
    //
    // `ctx.os` renders a platform-dependent value, so pinning its text here
    // would make the matrix macOS-only. The fixture check asserts only that it
    // resolved to something; the equality assertion below is what compares it
    // across the two arms, which is the property this row exists to test.
    let stable: Vec<String> = arms
        .direct_lines
        .iter()
        .filter(|line| !line.starts_with("ctx.os="))
        .cloned()
        .collect();
    assert_eq!(
        stable,
        vec![
            "sig=initialize",
            "provider-ran",
            "sig=success",
            "fm.note=authored-note",
            "fm.derived=authored-note-derived",
            // A bare temporary git repo sits in no package area, so `ctx.area`
            // resolves empty. It is stamped anyway: an empty value that stays
            // empty on both arms is still the facet agreeing.
            "ctx.area=",
            "sig=finalize",
        ],
        "fixture check: the probe stamps its facets in this order when invoked \
         directly; pane:\n{}",
        arms.direct_pane
    );
    assert!(
        arms.direct_lines
            .iter()
            .any(|line| line.starts_with("ctx.os=") && line.len() > "ctx.os=".len()),
        "fixture check: `ctx.os` must resolve to a non-empty value; got {:?}; pane:\n{}",
        arms.direct_lines,
        arms.direct_pane
    );

    assert_eq!(
        arms.routed_lines, arms.direct_lines,
        "the route must not be observable: a target reached through an \
         `initialize` proxy must stamp the same frontmatter, computed property, \
         `ctx.*` snapshot, lifecycle signal order, and initialize count as the \
         same target invoked directly; pane:\n{}",
        arms.routed_pane
    );
}

/// A probe that pins its own `model:` and stamps the launch facets derived from
/// it. The provider stays fixed (`--goose` on both arms), so `model:` is the
/// one launch input free to move — which makes this the cheapest honest probe
/// of R6 that needs no second provider stub and cannot trip interactive
/// provider selection.
///
/// The id rides Goose's declared `llamacpp` local-runner namespace. Frontmatter
/// models are catalog-validated (`ModelCatalog::is_valid`) and an invalid one is
/// dropped silently, which would empty this probe's facet for a reason that has
/// nothing to do with routing. A namespaced id is accepted by construction, so
/// it cannot age out the way a real model id would.
const EQUIV_PINNED_MODEL_TARGET: &str = r#"---
title: equivalence probe with a pinned model
model: llamacpp/probe-model-x
initialize:
  stack:
    - action: {append_line: ["events.log", "sig=initialize"]}
success:
  stack:
    - action:
        - {append_line: ["events.log", "sig=success"]}
        - {append_line: ["events.log", "env.MODEL={{ env.MODEL }}"]}
finalize:
  stack:
    - action: {append_line: ["events.log", "sig=finalize"]}
---
pinned probe body
"#;

/// **Acceptance criteria 9 and 10, launch facets.** A target that pins its own
/// `model:` resolves the same launch state on both routes — invoked directly and
/// reached through an `initialize` proxy.
///
/// This row is the R6 launch-rebuild contract (now enabled; before the rebuild
/// landed it was a reproduction of the R6 gap that failed by design). When a
/// handoff surfaces to the command-owned coordinator, the target is re-prepared
/// as a fresh document and its `model:` is recomputed into the launch
/// environment — via `compose/prep.rs::prepare_and_run_active_document`, with the
/// in-harness fallback `harness_orch::loop_control::target_launch` overlaying the
/// `AGENT`/`MODEL`/`YOLO` env — so the routed arm resolves the same `env.MODEL`
/// as the direct arm rather than the router's frozen value.
///
/// It probes `model:` rather than `agent:` because the provider is pinned by
/// `--goose` on both arms — explicit CLI intent that stays authoritative — so
/// `model:` is the one launch input free to move without a second provider stub
/// or an interactive selection prompt.
#[test]
#[serial(level2_lifecycle_control)]
fn level2_lifecycle_equivalence_target_pinned_model_matches_direct_run() {
    require_level!(Level::L2, TmuxHarness::available(), "tmux");

    // sig=initialize, provider-ran, sig=success, env.MODEL, sig=finalize.
    const EXPECTED_MARKERS: usize = 5;

    let arms = equivalence_arms(EQUIV_PINNED_MODEL_TARGET, EXPECTED_MARKERS);

    assert!(
        arms.direct_lines
            .contains(&"env.MODEL=llamacpp/probe-model-x".to_string()),
        "fixture check: invoked directly, the target's own `model:` reaches the \
         launch environment; got {:?}; pane:\n{}",
        arms.direct_lines,
        arms.direct_pane
    );

    assert_eq!(
        arms.routed_lines, arms.direct_lines,
        "a proxied target must resolve its own launch state; pane:\n{}",
        arms.routed_pane
    );
}

// ── review-6 finding 3: the complete AC9 context matrix ──────────────────────
//
// AC9 names five facets — `ctx.area`, `ctx.agent`, `ctx.model`, `env.AGENT`,
// `env.MODEL` — and three surfaces they must agree on: the prompt body, the
// effective frontmatter, and the lifecycle events. The row below is the
// principal probe for all fifteen cells; the older
// `level2_lifecycle_equivalence_probe_matches_direct_run` remains as the
// signal-order/computed-property row.

/// A router with no launch identity of its own beyond the provider it needs to
/// resolve before proxying. It authors **no** `model:`, so every model-derived
/// value the target observes can only have come from the target.
const AC9_ROUTER: &str = r#"---
title: ac9 router
agent: goose
initialize:
  stack:
    - action: {proxy: "@target.md"}
---
router body
"#;

/// The AC9 probe: it stamps all five named facets from all three required
/// surfaces.
///
/// - **Body** — `body.<facet>=[…]` spans, recovered from the prompt the fake
///   provider records, so the assertion is over what actually reached the agent.
/// - **Effective frontmatter** — `fm_*` whole-value properties resolved during
///   composition, read back by a lifecycle action so their *effective* (not
///   authored) values are what land in the log.
/// - **Lifecycle** — `lc.*` stamps evaluated inside the `success` stack.
///
/// The target authors its own `agent:` and `model:`, and both arms run with **no
/// CLI provider flag**, so every stamped value is target-owned. `llamacpp/…`
/// rides Goose's declared local-runner namespace: frontmatter models are
/// catalog-validated and an invalid one is dropped silently, which would empty
/// the model facets for a reason that has nothing to do with routing.
const AC9_PROBE_TARGET: &str = r#"---
title: ac9 probe target
agent: goose
model: llamacpp/ac9-probe-model
fm_area: "{{ ctx.area }}"
fm_agent: "{{ ctx.agent }}"
fm_model: "{{ ctx.model }}"
fm_env_agent: "{{ env.AGENT }}"
fm_env_model: "{{ env.MODEL }}"
initialize:
  stack:
    - action: {append_line: ["events.log", "sig=initialize"]}
success:
  stack:
    - action:
        - {append_line: ["events.log", "sig=success"]}
        - {append_line: ["events.log", "lc.ctx.area={{ ctx.area }}"]}
        - {append_line: ["events.log", "lc.ctx.agent={{ ctx.agent }}"]}
        - {append_line: ["events.log", "lc.ctx.model={{ ctx.model }}"]}
        - {append_line: ["events.log", "lc.env.AGENT={{ env.AGENT }}"]}
        - {append_line: ["events.log", "lc.env.MODEL={{ env.MODEL }}"]}
        - {append_line: ["events.log", "fm.ctx.area={{ fm_area }}"]}
        - {append_line: ["events.log", "fm.ctx.agent={{ fm_agent }}"]}
        - {append_line: ["events.log", "fm.ctx.model={{ fm_model }}"]}
        - {append_line: ["events.log", "fm.env.AGENT={{ fm_env_agent }}"]}
        - {append_line: ["events.log", "fm.env.MODEL={{ fm_env_model }}"]}
finalize:
  stack:
    - action: {append_line: ["events.log", "sig=finalize"]}
---
probe body.ctx.area=[{{ ctx.area }}] body.ctx.agent=[{{ ctx.agent }}] body.ctx.model=[{{ ctx.model }}] body.env.AGENT=[{{ env.AGENT }}] body.env.MODEL=[{{ env.MODEL }}]
"#;

/// Recover the `body.<facet>=[<value>]` spans from the recorded prompt line.
///
/// The recorded line is the whole stdin+argv the provider was handed, which
/// carries absolute temporary paths that legitimately differ between arms.
/// Extracting the spans — rather than comparing the raw line — compares exactly
/// the body-surface facets the row asserts on and nothing else.
fn body_facets(lines: &[String]) -> Vec<String> {
    let prompt = lines
        .iter()
        .find(|line| line.starts_with("prompt:"))
        .cloned()
        .unwrap_or_default();
    let mut facets = Vec::new();
    let mut rest = prompt.as_str();
    while let Some(start) = rest.find("body.") {
        rest = &rest[start + "body.".len()..];
        let Some(open) = rest.find("=[") else { break };
        let key = rest[..open].to_string();
        rest = &rest[open + 2..];
        let Some(close) = rest.find(']') else { break };
        facets.push(format!("{key}={}", &rest[..close]));
        rest = &rest[close + 1..];
    }
    facets
}

/// **Acceptance criterion 9, complete.** `ctx.area`, `ctx.agent`, `ctx.model`,
/// `env.AGENT`, and `env.MODEL` resolve identically whether the target is
/// invoked directly or reached through an `initialize` proxy — in the prompt
/// body, in the effective frontmatter, and in the lifecycle events.
///
/// Neither arm pins a provider on the CLI, so the agent and model facets are
/// resolved from the *target's* authored frontmatter on both routes. That is
/// what makes the row non-vacuous: the router authors no `model:` at all, so a
/// routed arm that reused the router's frozen launch state would stamp the
/// empty/default model rather than `llamacpp/ac9-probe-model`, and the exact
/// fixture assertions below would fail before the arms were ever compared.
#[test]
#[serial(level2_lifecycle_control)]
fn level2_lifecycle_equivalence_ac9_context_facets_match_direct_run() {
    require_level!(Level::L2, TmuxHarness::available(), "tmux");

    // sig=initialize, prompt, provider-ran, sig=success, 10 stamps, sig=finalize.
    const EXPECTED_MARKERS: usize = 15;
    const MODEL: &str = "llamacpp/ac9-probe-model";

    let arms = equivalence_arms_configured(
        AC9_ROUTER,
        AC9_PROBE_TARGET,
        "",
        EXPECTED_MARKERS,
        |staged| write_prompt_recording_goose(&staged.bin_dir, &staged.events_log),
    );

    let direct_body = body_facets(&arms.direct_lines);
    let routed_body = body_facets(&arms.routed_lines);

    // Fixture check: the direct arm is the contract. Asserting its exact values
    // first means a broken probe is not misread as a routing bug — and pins the
    // body surface to the five named facets rather than "some spans were found".
    assert_eq!(
        direct_body,
        vec![
            // A bare temporary git repo sits in no package area, so `ctx.area`
            // resolves empty. It is stamped anyway: an empty value that stays
            // empty on both arms is still the facet agreeing.
            "ctx.area=".to_string(),
            "ctx.agent=goose".to_string(),
            format!("ctx.model={MODEL}"),
            "env.AGENT=goose".to_string(),
            format!("env.MODEL={MODEL}"),
        ],
        "fixture check: invoked directly, the prompt body carries all five AC9 \
         facets resolved from the target's own frontmatter; pane:\n{}",
        arms.direct_pane
    );

    // The frontmatter and lifecycle surfaces, in the order the probe stamps them.
    let stamped: Vec<String> = arms
        .direct_lines
        .iter()
        .filter(|line| line.starts_with("lc.") || line.starts_with("fm."))
        .cloned()
        .collect();
    assert_eq!(
        stamped,
        vec![
            "lc.ctx.area=".to_string(),
            "lc.ctx.agent=goose".to_string(),
            format!("lc.ctx.model={MODEL}"),
            "lc.env.AGENT=goose".to_string(),
            format!("lc.env.MODEL={MODEL}"),
            "fm.ctx.area=".to_string(),
            "fm.ctx.agent=goose".to_string(),
            format!("fm.ctx.model={MODEL}"),
            "fm.env.AGENT=goose".to_string(),
            format!("fm.env.MODEL={MODEL}"),
        ],
        "fixture check: invoked directly, the lifecycle and effective-frontmatter \
         surfaces carry all five AC9 facets; got {:?}; pane:\n{}",
        arms.direct_lines,
        arms.direct_pane
    );

    assert_eq!(
        routed_body, direct_body,
        "the prompt body a proxied target delivers must resolve every AC9 facet \
         exactly as the same target invoked directly does; pane:\n{}",
        arms.routed_pane
    );
    // The recorded `prompt:` line is compared through `body_facets` above; the
    // raw line also carries the provider's argv, whose temporary paths differ
    // per arm for reasons that are not routing.
    let without_prompt = |lines: &[String]| -> Vec<String> {
        lines
            .iter()
            .filter(|line| !line.starts_with("prompt:"))
            .cloned()
            .collect()
    };
    assert_eq!(
        without_prompt(&arms.routed_lines),
        without_prompt(&arms.direct_lines),
        "the effective frontmatter and lifecycle surfaces of a proxied target \
         must resolve every AC9 facet exactly as the same target invoked \
         directly does; pane:\n{}",
        arms.routed_pane
    );
}

// ── review-3 finding 5: overlay redaction + additional matrix rows ───────────
//
// These rows close the gaps the review named in the required Level 2 evidence:
// a real pane-text assertion for AC 30 (overlay values never reach rendered
// status/diagnostics), the three-document forwarding/omission chain, a
// cross-repository proxy context/file-resolution row, a stdout/stderr routing
// comparison, and proxy containment inside a sequence step. Every row uses the
// fake `goose` provider and self-contained temporary paths.

/// A distinctive overlay value that must never appear in any rendered status or
/// diagnostic on the pane, yet must reach the target lifecycle that consumes it.
const AC30_OVERLAY_SECRET: &str = "SEKRIToverlayVALUExyz";

/// **Acceptance criterion 30, real-terminal evidence.** A proxy hand-off whose
/// `with:` carries a secret-shaped value renders its user-facing status through
/// `TerminalRenderable` components (the `report_proxy_handoff` INFO line), and
/// that rendered output never discloses the overlay's *values* — while the
/// target's own lifecycle still receives the value (redaction is a display
/// concern, not a data one).
///
/// The overlay value is deliberately absent from the target *body* (which is
/// previewed as the agent prompt and would legitimately echo it), so its only
/// route to the pane would be a status/diagnostic that leaked it. The target's
/// `success` stack stamps the value into `events.log` — off the pane — proving
/// the value reached the code that needs it (spec "Security and Side Effects":
/// status may report that a hand-off includes an overlay, but must not print
/// overlay values).
#[test]
#[serial(level2_lifecycle_control)]
fn level2_lifecycle_proxy_overlay_value_is_not_disclosed_in_rendered_status() {
    require_level!(Level::L2, TmuxHarness::available(), "tmux");

    let source_doc = format!(
        "---\ntitle: overlay redaction router\ninitialize:\n  stack:\n    \
         - action: {{action: proxy, target: '@target.md', with: {{secret: \"{secret}\"}}}}\n\
         ---\nrouter body\n",
        secret = AC30_OVERLAY_SECRET,
    );
    // The target authors its own `secret` default and reads the effective value
    // through its `success` stack into `events.log`. Its body never references
    // `secret`, so the value cannot reach the agent-prompt preview.
    let target_doc = "---\ntitle: overlay redaction target\nsecret: authored-default\nsuccess:\n  stack:\n    \
         - action: {append_line: ['events.log', 'consumed={{ secret }}']}\nfinalize:\n  stack:\n    \
         - action: {append_line: ['events.log', 'redaction-finalize']}\n---\ntarget body carries no secret\n";
    let staged = stage_proxy_pair(&source_doc, target_doc, true);

    let pane = run_in_tmux_for(&staged, "redaction-finalize");
    let lines = event_lines(&staged);

    // The rendered hand-off status must be present — this is the
    // `TerminalRenderable` surface the value could have leaked through.
    assert!(
        pane.contains("flow control redirected"),
        "the proxy hand-off status must render through the terminal component; pane:\n{pane}"
    );
    // The overlay value must have reached the target's own lifecycle.
    assert!(
        lines
            .iter()
            .any(|l| l == &format!("consumed={AC30_OVERLAY_SECRET}")),
        "the overlay value must reach the target lifecycle that consumes it \
         (redaction hides it from display, not from the code that needs it); got {lines:?}"
    );
    // …but must never appear on the rendered pane.
    assert!(
        !pane.contains(AC30_OVERLAY_SECRET),
        "no rendered status or diagnostic may disclose the overlay value; pane:\n{pane}"
    );
}

/// **Acceptance criterion 26, three-document forwarding/omission chain.** A
/// downstream proxy replaces the overlay unless forwarding is explicit: an
/// overlay key the middle document forwards with an explicit `with:` reaches the
/// final target; a key it merely received but did not forward does not.
///
/// `doc.md` proxies to `@mid.md` carrying two overlay keys. `mid.md` proxies to
/// `@target.md` forwarding only `token` (via `{{ token }}`, resolved from its own
/// overlay-installed frontmatter) and omitting `extra`. The final target stamps
/// both effective values: `token` is the source's value carried across two hops,
/// while `extra` falls back to the target's own authored default because the
/// middle hop did not forward it.
#[test]
#[serial(level2_lifecycle_control)]
fn level2_lifecycle_proxy_three_document_chain_forwards_only_explicit_keys() {
    require_level!(Level::L2, TmuxHarness::available(), "tmux");

    let source_doc = "---\ntitle: chain source\ninitialize:\n  stack:\n    \
         - action: {action: proxy, target: '@mid.md', with: {token: from-source, extra: source-extra}}\n\
         ---\nsource body\n";
    // The middle hop forwards `token` explicitly and omits `extra`.
    let mid_doc = "---\ntitle: chain middle\ntoken: mid-default\ninitialize:\n  stack:\n    \
         - action: {action: proxy, target: '@target.md', with: {token: \"{{ token }}\"}}\n\
         ---\nmiddle body\n";
    let target_doc = "---\ntitle: chain target\ntoken: target-default\nextra: authored-extra\nsuccess:\n  stack:\n    \
         - action: {append_line: ['events.log', 'token={{ token }} extra={{ extra }}']}\nfinalize:\n  stack:\n    \
         - action: {append_line: ['events.log', 'chain-finalize']}\n---\nchain target body\n";
    let staged = stage_proxy_pair(source_doc, target_doc, true);
    fs::write(staged.workspace.path().join("mid.md"), mid_doc).unwrap();

    let pane = run_in_tmux_for(&staged, "chain-finalize");
    let lines = event_lines(&staged);

    assert!(
        lines
            .iter()
            .any(|l| l == "token=from-source extra=authored-extra"),
        "the explicitly forwarded `token` must cross both hops while the \
         unforwarded `extra` must fall back to the target's authored default \
         (a downstream proxy replaces the overlay unless forwarding is explicit); \
         got {lines:?}; pane:\n{pane}"
    );
    assert!(
        !lines.iter().any(|l| l.contains("extra=source-extra")),
        "the source overlay's `extra` must not silently forward across the \
         middle hop that omitted it; got {lines:?}; pane:\n{pane}"
    );
}

/// The cross-repository file-resolution probe target. Its `$schema` requires a
/// `file(required;eager)` reference that resolves against the launch area — the
/// workspace the wrapper `cd`s into — not the target document's own repository.
///
/// The success signal is deliberately the provider launch (`provider-ran`, which
/// the fake `goose` records to an absolute path), not a lifecycle `append_line`:
/// a relative `append_line` anchors on the *target document's* directory, which
/// for this row is the nested `otherrepo/`, so it would not reach the launch-root
/// `events.log` the arm reads. If the `spec` reference failed to resolve, the run
/// would abort before launching the provider, so `provider-ran` present is
/// exactly "the schema reference resolved."
const XREPO_TARGET: &str = r#"---
title: cross repo target
$schema:
    spec: file(required;eager)
---
cross repo target body
"#;

/// A router at the launch root that proxies to a target living in a *different*
/// git repository nested under the launch workspace. It carries the same schema
/// so the caller's `spec=` param is accepted and forwarded across the hand-off.
const XREPO_ROUTER: &str = r#"---
title: cross repo router
$schema:
    spec: file(required;eager)
initialize:
  stack:
    - action: {proxy: "@otherrepo/target.md"}
---
router body
"#;

/// Stage one arm of the cross-repository row. Both arms launch from the same
/// workspace (the launch repository); the target document lives in a nested,
/// independent git repository (`otherrepo/`), while the `spec` payload the
/// target's schema resolves lives only at the launch root. The direct arm
/// invokes the target by its nested path; the routed arm invokes the launch-root
/// router that proxies to it.
fn stage_cross_repo_arm(routed: bool) -> Staged {
    let workspace = tempdir().unwrap();
    let bin_dir = workspace.path().join("bin");
    fs::create_dir_all(&bin_dir).unwrap();
    seed_minimal_config(workspace.path());
    assert!(init_git_repo(workspace.path()), "git init failed");

    let events_log = workspace.path().join("events.log");
    write_succeeding_goose(&bin_dir, &events_log);

    let otherrepo = workspace.path().join("otherrepo");
    fs::create_dir_all(&otherrepo).unwrap();
    assert!(init_git_repo(&otherrepo), "git init otherrepo failed");
    fs::write(otherrepo.join("target.md"), XREPO_TARGET).unwrap();

    let source = workspace.path().join("doc.md");
    fs::write(&source, XREPO_ROUTER).unwrap();

    // The schema payload exists only at the launch root — never inside
    // `otherrepo/`. A target that resolved `spec` against its own repository
    // would fail to find it; resolving against the launch area is what makes
    // both arms succeed.
    fs::write(
        workspace.path().join("payload.md"),
        "---\nimplemented: true\n---\npayload\n",
    )
    .unwrap();

    let md_file = if routed {
        source
    } else {
        otherrepo.join("target.md")
    };

    Staged {
        workspace,
        bin_dir,
        md_file,
        events_log,
        rendezvous_endpoint: None,
    }
}

/// **Acceptance criteria 8-10, cross-repository proxy context and file
/// resolution.** A target reached through a proxy resolves a `file(...)` schema
/// reference against the same launch-area anchor as the same target invoked
/// directly, even when the target document lives in a different repository than
/// the proxying router and the payload lives only at the launch root. The route
/// is not observable in the resolution anchor.
#[test]
#[serial(level2_lifecycle_control)]
fn level2_lifecycle_equivalence_cross_repo_file_resolution_matches_direct_run() {
    require_level!(Level::L2, TmuxHarness::available(), "tmux");

    let assert_resolved = |lines: &[String], pane: &str, arm: &str| {
        assert!(
            lines.iter().any(|l| l == "provider-ran"),
            "[{arm}] the target's `spec` schema reference must resolve against \
             the launch area (the payload lives only at the launch root, not in \
             the target's own repository) so the provider launches; \
             got {lines:?}; pane:\n{pane}"
        );
        assert!(
            !pane.contains("did not satisfy the schema"),
            "[{arm}] no schema-validation failure must surface; pane:\n{pane}"
        );
    };

    let direct = stage_cross_repo_arm(false);
    let direct_pane = run_proxy_in_tmux_with_set(&direct, "spec=payload.md", "provider-ran");
    assert_resolved(&event_lines(&direct), &direct_pane, "direct");

    let routed = stage_cross_repo_arm(true);
    let routed_pane = run_proxy_in_tmux_with_set(&routed, "spec=payload.md", "provider-ran");
    assert_resolved(&event_lines(&routed), &routed_pane, "routed");
}

/// Run `claudine compose --goose <doc>` in tmux with the process stdout
/// redirected to `stdout.txt`, so stdout- and stderr-channel routing can be
/// distinguished: the returned pane holds only what reached stderr, and the
/// second string is what reached stdout.
fn run_capturing_stdout(staged: &Staged, done_marker: &str) -> (String, String) {
    static SEQ: AtomicU32 = AtomicU32::new(0);
    let seq = SEQ.fetch_add(1, Ordering::Relaxed);

    let session = format!("biscuit_l2_lcout_{}_{seq}", std::process::id());
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
    let sentinel = format!("L2_OUT_DONE_{seq}");
    let stdout_path = staged.workspace.path().join("stdout.txt");
    let cmd = format!(
        "cd {ws} && NO_COLOR='1' HOME='{home}' PATH='{path}' {claudine} compose --goose {md} > {out} ; echo {sentinel}",
        ws = staged.workspace.path().display(),
        home = staged.workspace.path().display(),
        path = augmented_path(&staged.bin_dir).to_string_lossy(),
        md = staged.md_file.display(),
        out = stdout_path.display(),
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
    let stdout = fs::read_to_string(&stdout_path).unwrap_or_default();
    (pane, stdout)
}

/// A target whose `success` event routes one marker to stdout and one to stderr.
const ROUTING_TARGET: &str = r#"---
title: routing target
success:
  stdout: "STDOUTrouteMARKER"
  info: "STDERRrouteMARKER"
  stack:
    - action: {append_line: ["events.log", "routing-done"]}
---
routing target body
"#;

/// Stage one arm of the stdout/stderr routing row: the direct arm invokes the
/// routing target, the routed arm invokes [`EQUIV_ROUTER`] and is handed off.
fn stage_routing_arm(routed: bool) -> Staged {
    let mut staged = stage_proxy_pair(EQUIV_ROUTER, ROUTING_TARGET, true);
    if !routed {
        staged.md_file = staged.workspace.path().join("target.md");
    }
    staged
}

/// **Acceptance criterion 10, stdout/stderr routing.** A target's stdout-channel
/// and stderr-channel lifecycle output route to the same process streams whether
/// it is invoked directly or reached through a proxy: the stdout-channel marker
/// lands on stdout (and never on the stderr pane), and the stderr-channel marker
/// lands on the pane (and never on stdout), identically on both routes.
#[test]
#[serial(level2_lifecycle_control)]
fn level2_lifecycle_equivalence_stdout_stderr_routing_matches_direct_run() {
    require_level!(Level::L2, TmuxHarness::available(), "tmux");

    let assert_routing = |pane: &str, stdout: &str, arm: &str| {
        assert!(
            stdout.contains("STDOUTrouteMARKER"),
            "[{arm}] the stdout-channel marker must reach process stdout; stdout:\n{stdout}"
        );
        assert!(
            !pane.contains("STDOUTrouteMARKER"),
            "[{arm}] the stdout-channel marker must not reach the stderr pane; pane:\n{pane}"
        );
        assert!(
            pane.contains("STDERRrouteMARKER"),
            "[{arm}] the stderr-channel marker must reach the stderr pane; pane:\n{pane}"
        );
        assert!(
            !stdout.contains("STDERRrouteMARKER"),
            "[{arm}] the stderr-channel marker must not reach process stdout; stdout:\n{stdout}"
        );
    };

    let direct = stage_routing_arm(false);
    let (direct_pane, direct_stdout) = run_capturing_stdout(&direct, "routing-done");
    assert_routing(&direct_pane, &direct_stdout, "direct");

    let routed = stage_routing_arm(true);
    let (routed_pane, routed_stdout) = run_capturing_stdout(&routed, "routing-done");
    assert_routing(&routed_pane, &routed_stdout, "routed");
}

/// **Acceptance criterion 6, proxy inside a sequence step.** A sequence step
/// whose document hands off via `proxy` runs its target within that one step and
/// cannot advance to the next step or restart the current one: the two-step
/// sequence runs the target exactly once per step (twice total) and completes
/// both steps.
#[test]
#[serial(level2_lifecycle_control)]
fn level2_lifecycle_proxy_inside_sequence_step_is_contained() {
    require_level!(Level::L2, TmuxHarness::available(), "tmux");

    let sequence_doc = "---\ntitle: sequence with a proxy step\nsequence:\n  - alpha\n  - beta\n\
         initialize:\n  stack:\n    - action: {proxy: '@target.md'}\n---\nsequence body {{ state }}\n";
    let target_doc = "---\ntitle: sequence proxy target\nsuccess:\n  stack:\n    \
         - action: {append_line: ['events.log', 'target-ran']}\n---\nsequence proxy target body\n";
    let staged = stage_proxy_pair(sequence_doc, target_doc, true);

    // Two steps, each proxying to the target once: 2 `target-ran` markers.
    let pane = run_sequence_until_target_runs(&staged, 2);
    let lines = event_lines(&staged);

    assert_eq!(
        lines.iter().filter(|l| **l == "target-ran").count(),
        2,
        "each of the two sequence steps must run the proxied target exactly \
         once — a proxy inside a step neither advances nor restarts the \
         sequence; got {lines:?}; pane:\n{pane}"
    );
}

/// Run `claudine sequence --goose <doc>` in tmux and block until the target has
/// run `expected_runs` times (or the deadline elapses). Returns the pane.
fn run_sequence_until_target_runs(staged: &Staged, expected_runs: usize) -> String {
    static SEQ: AtomicU32 = AtomicU32::new(0);
    let seq = SEQ.fetch_add(1, Ordering::Relaxed);

    let session = format!("biscuit_l2_lcseq_{}_{seq}", std::process::id());
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
    let sentinel = format!("L2_SEQ_DONE_{seq}");
    let cmd = format!(
        "cd {ws} && NO_COLOR='1' HOME='{home}' PATH='{path}' {claudine} sequence --goose {md} ; echo {sentinel}",
        ws = staged.workspace.path().display(),
        home = staged.workspace.path().display(),
        path = augmented_path(&staged.bin_dir).to_string_lossy(),
        md = staged.md_file.display(),
    );
    harness
        .send_command_with_env(&cmd, &[])
        .expect("send sequence command");

    let deadline = Instant::now() + Duration::from_secs(40);
    while Instant::now() < deadline {
        let runs = event_lines(staged)
            .iter()
            .filter(|l| **l == "target-ran")
            .count();
        if runs >= expected_runs {
            std::thread::sleep(Duration::from_millis(150));
            break;
        }
        std::thread::sleep(Duration::from_millis(100));
    }

    let pane = harness.capture().map(|f| f.plain).unwrap_or_default();
    kill_session_by_name(&session);
    pane
}

// ── review-5 finding 3: sequence-step proxy equivalence (loop + launch) ──────
//
// The prior sequence proxy row (`level2_lifecycle_proxy_inside_sequence_step_is_
// contained`) uses a non-looping target under a pinned `--goose` provider and
// asserts only once-per-step execution. It cannot see loop ownership (R7) or a
// target-owned launch facet (R6), which is exactly what the reduced in-harness
// path dropped. These two rows close that gap: a looping sequence target and a
// target-authored `model:`, each compared against a direct invocation.

/// A two-step sequence source whose every step hands its step off to the looping
/// `target.md` at `initialize`.
const SEQ_LOOP_SOURCE: &str = "---\ntitle: sequence with a looping proxy step\n\
     sequence:\n  - alpha\n  - beta\ninitialize:\n  stack:\n    \
     - action: {proxy: '@target.md'}\n---\nsequence body\n";

/// A two-step sequence source whose every step proxies to the pinned-model
/// target. Mirrors [`SEQ_LOOP_SOURCE`] but reaches [`EQUIV_PINNED_MODEL_TARGET`].
const SEQ_MODEL_SOURCE: &str = "---\ntitle: sequence pinned-model launch facet\n\
     sequence:\n  - alpha\n  - beta\ninitialize:\n  stack:\n    \
     - action: {proxy: '@target.md'}\n---\nsequence body\n";

/// Run `claudine sequence --goose <doc>` in tmux and block until `events.log`
/// holds `expected_lines` markers, or the marker count settles (so a run that
/// produces *fewer* markers than expected surfaces the mismatch instead of
/// burning the whole deadline). Returns the captured pane.
fn run_sequence_until_settled(staged: &Staged, expected_lines: usize) -> String {
    static SEQ: AtomicU32 = AtomicU32::new(0);
    let seq = SEQ.fetch_add(1, Ordering::Relaxed);

    let session = format!("biscuit_l2_lcseqset_{}_{seq}", std::process::id());
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
    let sentinel = format!("L2_SEQSET_DONE_{seq}");
    let cmd = format!(
        "cd {ws} && NO_COLOR='1' HOME='{home}' PATH='{path}' {claudine} sequence --goose {md} ; echo {sentinel}",
        ws = staged.workspace.path().display(),
        home = staged.workspace.path().display(),
        path = augmented_path(&staged.bin_dir).to_string_lossy(),
        md = staged.md_file.display(),
    );
    harness
        .send_command_with_env(&cmd, &[])
        .expect("send sequence command");

    let deadline = Instant::now() + Duration::from_secs(40);
    let mut last_count = 0usize;
    let mut stable_since: Option<Instant> = None;
    while Instant::now() < deadline {
        let count = event_lines(staged).len();
        if count >= expected_lines {
            std::thread::sleep(Duration::from_millis(150));
            break;
        }
        // Guarded on `count > 0` so the settle detection never false-breaks
        // during startup: a sequence step whose proxied target is re-prepared
        // and staged does more pre-launch work before its first event than a
        // direct step. See the note in `run_until_settled`.
        if count == last_count && count > 0 {
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

/// **Finding 3, R7 loop ownership + R1 containment inside a sequence step.** A
/// sequence step whose document proxies to a *looping* target gives that target
/// its own document loop — the target runs its full iteration count, identical to
/// a direct invocation — while the proxy stays contained in the step: each of the
/// two steps activates the target exactly once (`target-init` twice), never
/// advancing early or restarting the current step.
///
/// The reduced in-harness path this replaces adopted the target as a single
/// provider attempt and never re-ran loop recognition, so a routed step ran one
/// iteration where a direct run loops three times.
#[test]
#[serial(level2_lifecycle_control)]
fn level2_lifecycle_sequence_step_proxy_to_looping_target_owns_the_loop() {
    require_level!(Level::L2, TmuxHarness::available(), "tmux");

    let count = |lines: &[String], needle: &str| lines.iter().filter(|l| *l == needle).count();

    // Direct contract: the looping target invoked directly runs three iterations.
    // 1 target-init + 3 provider-ran + 3 target-finalize = 7 markers.
    let direct = stage_proxy_pair(EQUIV_LOOP_TARGET, EQUIV_LOOP_TARGET, true);
    let direct_pane = run_until_settled(&direct, 7);
    let direct_lines = event_lines(&direct);
    assert_eq!(
        count(&direct_lines, "provider-ran"),
        3,
        "fixture check: the looping target runs three iterations when invoked \
         directly; got {direct_lines:?}; pane:\n{direct_pane}"
    );

    // Routed: a two-step sequence, each step proxying to the same looping target.
    // Per step: 1 target-init + 3 provider-ran + 3 target-finalize; two steps → 14.
    let routed = stage_proxy_pair(SEQ_LOOP_SOURCE, EQUIV_LOOP_TARGET, true);
    let routed_pane = run_sequence_until_settled(&routed, 14);
    let routed_lines = event_lines(&routed);

    assert_eq!(
        count(&routed_lines, "provider-ran"),
        6,
        "each of the two sequence steps must give the looping target its own \
         three-iteration loop (R7): expected 6 provider attempts (3 per step); the \
         reduced single-run adoption would produce 2; got {routed_lines:?}; \
         pane:\n{routed_pane}"
    );
    assert_eq!(
        count(&routed_lines, "target-init"),
        2,
        "the proxy stays contained in its step: the target activates exactly once \
         per step and the sequence neither advances early nor restarts a step \
         (either would change the initialize count); got {routed_lines:?}; \
         pane:\n{routed_pane}"
    );
    assert_eq!(
        count(&routed_lines, "provider-ran"),
        2 * count(&direct_lines, "provider-ran"),
        "the per-step loop iteration count must match a direct invocation of the \
         target; routed {routed_lines:?}; direct {direct_lines:?}; pane:\n{routed_pane}"
    );
}

/// **Finding 3, R6 target launch rebuild inside a sequence step.** A sequence
/// step's proxy target rebuilds its full launch bundle: the target's authored
/// `model:` reaches its launch environment, exactly as when the target is
/// composed directly. The reduced in-harness path rebuilt only AGENT/MODEL/YOLO
/// from the step and left a proxied target's own `model:` unresolved
/// (`env.MODEL=` empty).
///
/// `--goose` pins the provider on both arms — explicit CLI intent still wins
/// (R6) — so `model:` is the one launch facet free to move; its presence on the
/// routed arm proves the resolved launch state is the target's, not the step's.
#[test]
#[serial(level2_lifecycle_control)]
fn level2_lifecycle_sequence_step_proxy_rebuilds_target_launch_bundle() {
    require_level!(Level::L2, TmuxHarness::available(), "tmux");

    // Direct contract: the target composed directly (goose pinned) resolves its
    // own `model:` into env.MODEL. 5 markers.
    let direct = stage_proxy_pair(EQUIV_PINNED_MODEL_TARGET, EQUIV_PINNED_MODEL_TARGET, true);
    let direct_pane = run_until_settled(&direct, 5);
    let direct_lines = event_lines(&direct);
    assert!(
        direct_lines.contains(&"env.MODEL=llamacpp/probe-model-x".to_string()),
        "fixture check: invoked directly (goose pinned), the target's own `model:` \
         reaches the launch environment; got {direct_lines:?}; pane:\n{direct_pane}"
    );

    // Routed: a two-step sequence, each step proxying to the pinned-model target.
    // Per step: sig=initialize, provider-ran, sig=success, env.MODEL, sig=finalize
    // = 5 markers; two steps → 10.
    let routed = stage_proxy_pair(SEQ_MODEL_SOURCE, EQUIV_PINNED_MODEL_TARGET, true);
    let routed_pane = run_sequence_until_settled(&routed, 10);
    let routed_lines = event_lines(&routed);

    assert!(
        routed_lines.contains(&"env.MODEL=llamacpp/probe-model-x".to_string()),
        "a sequence step's proxy target must rebuild its full launch bundle: the \
         target's authored `model:` must reach env.MODEL, proving the launch state \
         is the target's and not the step's reduced AGENT/MODEL/YOLO; got \
         {routed_lines:?}; pane:\n{routed_pane}"
    );
    assert!(
        !routed_lines.iter().any(|l| l == "env.MODEL="),
        "the proxied target's MODEL must never resolve empty — an empty value is \
         the signature of the reduced in-harness path that inherits the step's \
         MODEL instead of rebuilding the target's; got {routed_lines:?}; \
         pane:\n{routed_pane}"
    );
}

// ── review-5 finding 6: remaining wrong-level gaps ───────────────────────────
//
// These rows lift required behaviors that had only Level 1 evidence up to Level
// 2 (a real `claudine compose`/`inline-compose` run in a real tmux pane):
//
// - AC6  inline-compose proxy closure file-output ownership (real file mutation)
// - AC17 approved bytes == executed bytes, end-to-end through a real shell
// - AC26 overlay retention across retry, resume, AND loop refresh
// - AC10 additional target launch facets: effective child CWD and the survival
//        of a CLI-supplied system prompt through a proxy hand-off
//
// Every row uses the fake `#!/bin/sh` providers and self-contained temporary
// paths already established above.

// ── AC6: inline-compose proxy closure rewrites only the final target ─────────

/// A fake `goose` for inline-compose: it drains the prompt and prints a fixed
/// replacement body to stdout, which inline-compose captures and writes back to
/// the *active* document's body. The body is deliberately distinct from every
/// fixture's authored body so the "unchanged body" closure guard cannot trip.
fn write_inline_body_goose(bin_dir: &Path, events_log: &Path, new_body: &str) {
    write_executable(
        &bin_dir.join("goose"),
        &format!(
            "#!/bin/sh\ncat > /dev/null 2>&1\nprintf 'provider-ran\\n' >> {log}\n\
             printf '%s\\n' '{body}'\nexit 0\n",
            log = events_log.display(),
            body = new_body,
        ),
    );
}

/// Run `claudine inline-compose --goose <doc>` in a real tmux pane and block
/// until the shell sentinel lands (the command exited). inline-compose mutates
/// files rather than emitting an `events.log` terminal marker, so the honest
/// wait is the compose command's own exit.
fn run_inline_compose_await_exit(staged: &Staged) -> String {
    static SEQ: AtomicU32 = AtomicU32::new(0);
    let seq = SEQ.fetch_add(1, Ordering::Relaxed);

    let session = format!("biscuit_l2_lcinline_{}_{seq}", std::process::id());
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
    let sentinel = format!("L2_INLINE_DONE_{seq}");
    let env_prefix = format!(
        "NO_COLOR='1' HOME='{home}' PATH='{path}' ",
        home = staged.workspace.path().display(),
        path = augmented_path(&staged.bin_dir).to_string_lossy(),
    );
    let cmd = format!(
        "cd {ws} && {env_prefix}{claudine} inline-compose --goose {md} ; echo {sentinel}",
        ws = staged.workspace.path().display(),
        md = staged.md_file.display(),
    );
    harness
        .send_command_with_env(&cmd, &[])
        .expect("send inline-compose command");

    let deadline = Instant::now() + Duration::from_secs(40);
    let mut pane = String::new();
    while Instant::now() < deadline {
        pane = harness.capture().map(|f| f.plain).unwrap_or_default();
        if pane.lines().any(|l| l.trim() == sentinel) {
            break;
        }
        std::thread::sleep(Duration::from_millis(100));
    }
    kill_session_by_name(&session);
    pane
}

/// **Acceptance criterion 6, real file-mutation evidence.** An `inline-compose`
/// run whose invoked router proxies through a middle document to a final target
/// rewrites the body of **only** the final target — the router and the middle
/// document are left byte-identical.
///
/// Closure ownership follows the active document, so the closure belongs to the
/// document that actually launched the provider (the final target), not the
/// router that pointed at it. The in-process
/// `inline_closure_ownership_follows_the_adopted_target` test proves the closure
/// *plan* moves with adoption; this proves the on-disk write does too, and that
/// no non-final document in the chain is touched.
#[test]
#[serial(level2_lifecycle_control)]
fn level2_lifecycle_inline_compose_proxy_closure_rewrites_only_final_target() {
    require_level!(Level::L2, TmuxHarness::available(), "tmux");

    // Every document is an inline-compose document (has `prompt:`); the first two
    // hand off at `initialize` before their own prompt is ever used.
    let router = "---\nprompt: gen-router\ninitialize:\n  stack:\n    \
         - action: {proxy: '@mid.md'}\n---\nrouter original body\n";
    let mid = "---\nprompt: gen-mid\ninitialize:\n  stack:\n    \
         - action: {proxy: '@final.md'}\n---\nmid original body\n";
    let final_doc = "---\nprompt: gen-final\n---\nfinal original body\n";

    let staged = stage_success(router);
    fs::write(staged.workspace.path().join("mid.md"), mid).unwrap();
    let final_path = staged.workspace.path().join("final.md");
    fs::write(&final_path, final_doc).unwrap();
    write_inline_body_goose(
        &staged.bin_dir,
        &staged.events_log,
        "REWRITTEN-BY-INLINE-CLOSURE-XYZ",
    );

    let pane = run_inline_compose_await_exit(&staged);

    // The router (the invoked document) is never rewritten — it handed off.
    assert_eq!(
        fs::read_to_string(&staged.md_file).unwrap(),
        router,
        "the router's bytes must be unchanged — closure belongs to the final \
         target, not the invoked document; pane:\n{pane}"
    );
    // The middle document in the chain is likewise untouched.
    assert_eq!(
        fs::read_to_string(staged.workspace.path().join("mid.md")).unwrap(),
        mid,
        "a non-final document in the proxy chain must not be rewritten; pane:\n{pane}"
    );
    // The final target's body IS rewritten (closure) and a `hash:` is stamped.
    let final_after = fs::read_to_string(&final_path).unwrap();
    assert!(
        final_after.contains("REWRITTEN-BY-INLINE-CLOSURE-XYZ"),
        "the final target's body must be rewritten with the provider output; \
         final:\n{final_after}\npane:\n{pane}"
    );
    assert!(
        !final_after.contains("final original body"),
        "the final target's original body must be replaced; final:\n{final_after}"
    );
    assert!(
        final_after.contains("hash:"),
        "the inline closure must stamp a Darkmatter `hash:` into the rewritten \
         final target; final:\n{final_after}"
    );
    assert!(
        final_after.contains("prompt: gen-final"),
        "the final target's authored frontmatter must be preserved through the \
         rewrite; final:\n{final_after}"
    );
}

// ── AC17: approved bytes equal executed bytes, end-to-end ────────────────────

/// A recorder on `PATH` that writes its first argument verbatim to
/// `executed_log`. A lifecycle `shell` action invokes it, so the bytes it
/// records are exactly the bytes the audited-and-approved command executed.
fn write_bytes_recorder(bin_dir: &Path, executed_log: &Path) {
    write_executable(
        &bin_dir.join("logbytes"),
        &format!(
            "#!/bin/sh\nprintf '%s' \"$1\" > {log}\nexit 0\n",
            log = executed_log.display(),
        ),
    );
}

/// **Acceptance criterion 17, end-to-end byte equality.** A target lifecycle
/// `shell` command whose argument interpolates an overlay-supplied value is
/// audited/approved with that value resolved, and the exact resolved bytes are
/// what the shell executes — no template leak, no re-evaluation to the authored
/// default, and no re-quoting drift across an embedded space.
///
/// The overlay merges into the target's authored frontmatter before the audit
/// reads the command, so the approved string is the resolved one
/// (`loop_control::tests::shell_approval::approved_bytes_equal_the_bytes_a_with_value_resolves_to`
/// pins this in-process). This row proves the same resolved bytes reach a real
/// `SystemShellRunner`: the recorder captures precisely what ran.
#[test]
#[serial(level2_lifecycle_control)]
fn level2_lifecycle_proxy_shell_approved_bytes_equal_executed_bytes() {
    require_level!(Level::L2, TmuxHarness::available(), "tmux");

    // The router installs the overlay value the command interpolates; the target
    // authors a different default so a template re-evaluation would be visible.
    let source_doc = "---\ntitle: approved-bytes router\ninitialize:\n  stack:\n    \
         - action: {action: proxy, target: '@target.md', with: {marker: 'forty two'}}\n\
         ---\nrouter body\n";
    let target_doc = "---\ntitle: approved-bytes target\nmarker: authored\nsuccess:\n  stack:\n    \
         - action: {shell: \"logbytes 'exact {{ marker }} bytes'\"}\n    \
         - action: {append_line: ['events.log', 'ac17-done']}\n---\ntarget body\n";
    let staged = stage_proxy_pair(source_doc, target_doc, true);

    // The recorder is whitelisted so the non-interactive run approves it without
    // a TTY handler (mirrors the overlay-layering L1 fixture's `prefix echo`).
    let executed_log = staged.workspace.path().join("executed.log");
    write_bytes_recorder(&staged.bin_dir, &executed_log);
    fs::write(
        staged.workspace.path().join(".darkmatter-shell-whitelist"),
        "prefix logbytes\n",
    )
    .unwrap();

    let pane = run_in_tmux_for(&staged, "ac17-done");

    let executed = fs::read_to_string(&executed_log).unwrap_or_default();
    assert_eq!(
        executed, "exact forty two bytes",
        "the bytes the shell executed must equal the audit-resolved command's \
         argument: the overlay value (`forty two`) resolved once, the embedded \
         space survived intact, and neither the authored default nor the raw \
         `{{{{ marker }}}}` template reached execution; pane:\n{pane}"
    );
}

// ── AC25: overlay-installed control-plane shell configuration ────────────────
//
// The AC17 row above proves byte equality for a shell action the *target*
// authored, with the overlay supplying only an interpolated value. AC25 is the
// stronger claim: `with:` may install the lifecycle stack itself — the overlay
// is the *origin* of the shell configuration — and that reach still buys no
// exemption from the target's own policy. Both rows below therefore stage a
// target that authors no `initialize` at all, so the audited command can only
// have come from the router's overlay.

/// **Acceptance criterion 25, denial.** A router that installs the target's
/// entire `initialize` shell stack through `proxy.with:` does not get to run an
/// un-approvable command: the target's narrow initialize gate refuses it, the
/// operator sees the denial, and no provider process starts.
///
/// `rm` is builtin-blacklisted, so a gate that ran at all refuses it. The
/// sentinel is the physical evidence — if the overlay could bypass target-side
/// policy the command would delete it before anything else could object.
#[test]
#[serial(level2_lifecycle_control)]
fn level2_lifecycle_overlay_installed_initialize_shell_is_denied_by_target_policy() {
    require_level!(Level::L2, TmuxHarness::available(), "tmux");

    let source_doc = "---\ntitle: overlay-installed denial router\ninitialize:\n  stack:\n    \
         - action: {action: proxy, target: '@target.md', with: \
         {initialize: {stack: [{action: {shell: 'rm sentinel.txt'}}]}}}\n\
         ---\nrouter body\n";
    // The target authors no `initialize`: the audited stack is the overlay's.
    let target_doc = "---\ntitle: overlay-installed denial target\nsuccess:\n  stack:\n    \
         - action: {append_line: ['events.log', 'target-success']}\n---\ntarget body\n";
    let staged = stage_proxy_pair(source_doc, target_doc, true);
    let sentinel = staged.workspace.path().join("sentinel.txt");
    fs::write(&sentinel, "intact").unwrap();

    let pane = run_in_tmux_until_exit(&staged);
    let lines = event_lines(&staged);
    let prose = flattened(&pane);

    assert!(
        sentinel.exists(),
        "a blacklisted command installed by `proxy.with:` must be refused by the \
         target's own shell policy — it deleted the sentinel instead; pane:\n{pane}"
    );
    assert!(
        prose.contains("blacklisted"),
        "the operator must see the target-side denial diagnostic; flattened \
         pane:\n{prose}"
    );
    assert!(
        prose.contains("rm sentinel.txt"),
        "the denial must name the overlay-installed command; flattened \
         pane:\n{prose}"
    );
    assert!(
        !lines.iter().any(|l| l == "provider-ran"),
        "a refused overlay-installed command must stop the run before the \
         provider launches; got {lines:?}; pane:\n{pane}"
    );
    assert!(
        !lines.iter().any(|l| l == "target-success"),
        "no target terminal event may fire when the boot never completed; \
         got {lines:?}; pane:\n{pane}"
    );
}

/// **Acceptance criterion 25, approval.** The same overlay-installed
/// `initialize` stack, this time carrying a command the target's policy does
/// approve, runs — and the bytes the shell executed are exactly the bytes the
/// audit approved.
///
/// The pairing with the denial row above is the point: approval is a policy
/// *decision* about overlay-installed configuration, not a path that skips the
/// gate. The whitelist lives in the target's own policy root, so what changed
/// between the two rows is the target's verdict, not the overlay's reach.
#[test]
#[serial(level2_lifecycle_control)]
fn level2_lifecycle_overlay_installed_initialize_shell_runs_the_approved_bytes() {
    require_level!(Level::L2, TmuxHarness::available(), "tmux");

    let source_doc = "---\ntitle: overlay-installed approval router\ninitialize:\n  stack:\n    \
         - action: {action: proxy, target: '@target.md', with: \
         {initialize: {stack: [{action: {shell: \"logbytes 'overlay installed bytes'\"}}]}}}\n\
         ---\nrouter body\n";
    let target_doc = "---\ntitle: overlay-installed approval target\nsuccess:\n  stack:\n    \
         - action: {append_line: ['events.log', 'ac25-approved']}\n---\ntarget body\n";
    let staged = stage_proxy_pair(source_doc, target_doc, true);

    let executed_log = staged.workspace.path().join("executed.log");
    write_bytes_recorder(&staged.bin_dir, &executed_log);
    fs::write(
        staged.workspace.path().join(".darkmatter-shell-whitelist"),
        "prefix logbytes\n",
    )
    .unwrap();

    let pane = run_in_tmux_for(&staged, "ac25-approved");
    let lines = event_lines(&staged);
    let prose = flattened(&pane);

    assert_eq!(
        fs::read_to_string(&executed_log).unwrap_or_default(),
        "overlay installed bytes",
        "the approved bytes must be the executed bytes for a shell action the \
         overlay installed, embedded spaces and all; pane:\n{pane}"
    );
    assert!(
        !prose.contains("blacklisted"),
        "an approved overlay-installed command must not render a denial; \
         flattened pane:\n{prose}"
    );
    assert!(
        lines.iter().any(|l| l == "provider-ran"),
        "the run must reach the provider once the gate approves; got {lines:?}; \
         pane:\n{pane}"
    );
}

// ── AC26: overlay retention across retry, resume, and loop refresh ───────────

/// **Acceptance criterion 26, retry re-entry.** The immediate `proxy.with:`
/// overlay survives a retry of the target: the overlay value is observable in the
/// target's lifecycle on the original attempt *and* the retried one, never
/// reverting to the target's authored default.
#[test]
#[serial(level2_lifecycle_control)]
fn level2_lifecycle_proxy_with_overlay_survives_a_retry() {
    require_level!(Level::L2, TmuxHarness::available(), "tmux");

    let source_doc = "---\ntitle: overlay retry router\ninitialize:\n  stack:\n    \
         - action: {action: proxy, target: '@target.md', with: {token: OVL26retry}}\n\
         ---\nrouter body\n";
    let target_doc = "---\ntitle: overlay retry target\ntoken: authored\nfailure:\n  stack:\n    \
         - action: {append_line: ['events.log', 'attempt-token={{ token }}']}\n    \
         - action: {retry: 1}\nfinalize:\n  stack:\n    \
         - action: {append_line: ['events.log', 'final-token={{ token }}']}\n    \
         - action: {append_line: ['events.log', 'retry-done']}\n---\ntarget body\n";
    // Failing provider so the target always routes through `failure` → `retry`.
    let staged = stage_proxy_pair(source_doc, target_doc, false);

    let pane = run_in_tmux_for(&staged, "retry-done");
    let lines = event_lines(&staged);

    assert_eq!(
        lines.iter().filter(|l| **l == "attempt-token=OVL26retry").count(),
        2,
        "the overlay must be in force on the original attempt AND the retried one \
         ({{retry: 1}} → two failures); got {lines:?}; pane:\n{pane}"
    );
    assert!(
        !lines.iter().any(|l| l == "attempt-token=authored"),
        "the overlay must never revert to the target's authored default across a \
         retry re-entry; got {lines:?}; pane:\n{pane}"
    );
    assert!(
        lines.iter().any(|l| l == "final-token=OVL26retry"),
        "the overlay must still be in force at the terminal finalize; \
         got {lines:?}; pane:\n{pane}"
    );
}

/// **Acceptance criterion 26, loop refresh.** The overlay survives every
/// iteration of a proxied looping target: each loop refresh re-materializes the
/// target with the same overlay in force, so the overlay value appears on every
/// iteration and never reverts to the authored default.
#[test]
#[serial(level2_lifecycle_control)]
fn level2_lifecycle_proxy_with_overlay_survives_a_loop_refresh() {
    require_level!(Level::L2, TmuxHarness::available(), "tmux");

    let source_doc = "---\ntitle: overlay loop router\ninitialize:\n  stack:\n    \
         - action: {action: proxy, target: '@target.md', with: {token: OVL26loop}}\n\
         ---\nrouter body\n";
    let target_doc = "---\ntitle: overlay loop target\ntoken: authored\nphase: 1\n\
         loop:\n  until: \"phase > 2\"\n  action: \"increment(phase)\"\n  max: 10\n\
         finalize:\n  stack:\n    \
         - action: {append_line: ['events.log', 'loop-token={{ token }}:{{ phase }}']}\n\
         ---\ntarget body\n";
    let staged = stage_proxy_pair(source_doc, target_doc, true);

    // 3 iterations × (provider-ran + loop-token) = 6 markers.
    let pane = run_until_settled(&staged, 6);
    let lines = event_lines(&staged);

    for phase in 1..=3 {
        assert!(
            lines.iter().any(|l| *l == format!("loop-token=OVL26loop:{phase}")),
            "the overlay must be in force on loop iteration {phase} \
             (`loop-token=OVL26loop:{phase}`); got {lines:?}; pane:\n{pane}"
        );
    }
    assert!(
        !lines.iter().any(|l| l.starts_with("loop-token=authored")),
        "the overlay must never revert to the authored default across a loop \
         refresh; got {lines:?}; pane:\n{pane}"
    );
}

/// **Acceptance criterion 26, resume re-entry.** The overlay survives a resume
/// of the target: after `failure` resumes the session and the run re-enters at
/// `start`, the overlay value is still observable in the target's lifecycle.
#[test]
#[serial(level2_lifecycle_control)]
fn level2_lifecycle_proxy_with_overlay_survives_a_resume() {
    require_level!(Level::L2, TmuxHarness::available(), "tmux");

    let follow_up = "please finish the resumed work";
    let source_doc = "---\ntitle: overlay resume router\ninitialize:\n  stack:\n    \
         - action: {action: proxy, target: '@target.md', with: {token: OVL26resume}}\n\
         ---\nrouter body\n";
    let target_doc = format!(
        "---\ntitle: overlay resume target\ntoken: authored\nstart:\n  stack:\n    \
         - action: {{append_line: ['events.log', 'start-token={{{{ token }}}}']}}\nfailure:\n  stack:\n    \
         - action: {{resume: \"{follow_up}\"}}\nsuccess:\n  stack:\n    \
         - action: {{append_line: ['events.log', 'success-token={{{{ token }}}}']}}\nfinalize:\n  stack:\n    \
         - action: {{append_line: ['events.log', 'resume-done']}}\n---\ntarget body\n"
    );
    let staged = stage_proxy_pair(source_doc, &target_doc, true);
    // A resume-capable Claude: fails the first attempt after reporting a session,
    // then succeeds when re-invoked with the resume argv + follow-up prompt.
    write_resumable_claude(
        &staged.bin_dir,
        &staged.events_log,
        "overlay-resume-session",
        follow_up,
    );

    let pane = run_provider_in_tmux_for(&staged, "--claude", "resume-done");
    let lines = event_lines(&staged);

    assert!(
        lines.iter().any(|l| l == "resume-session-ok"),
        "the resume must actually reach the provider's resume branch (precondition); \
         got {lines:?}; pane:\n{pane}"
    );
    assert_eq!(
        lines.iter().filter(|l| **l == "start-token=OVL26resume").count(),
        2,
        "the overlay must be in force on the opening `start` AND the resumed \
         `start`; got {lines:?}; pane:\n{pane}"
    );
    assert!(
        lines.iter().any(|l| l == "success-token=OVL26resume"),
        "the overlay must still be in force at the resumed success; \
         got {lines:?}; pane:\n{pane}"
    );
    assert!(
        !lines.iter().any(|l| l == "start-token=authored"),
        "the overlay must never revert to the authored default across a resume \
         re-entry; got {lines:?}; pane:\n{pane}"
    );
}

// ── AC10: target launch facets ───────────────────────────────────────────────
//
// Two kinds of facet live here. The first is invocation-level and
// route-independent by construction: the child provider's effective CWD (the
// launch area) and a CLI-supplied system prompt (immutable invocation intent the
// hand-off must not drop). The second is *target-driven* and is what the R6
// launch rebuild exists for — profile/binary and entrypoint, argv, the effective
// child environment, interactivity, structured-output mode, dispatch
// configuration, and MCP injection. Those rows run with **no CLI provider flag**
// so the selection they assert on can only have come from the target's own
// authored frontmatter; pinning a provider would make them vacuous.

/// A fake `goose` that records its effective working directory (the child
/// process CWD the provider is spawned in) to `events.log`.
fn write_cwd_recording_goose(bin_dir: &Path, events_log: &Path) {
    write_executable(
        &bin_dir.join("goose"),
        &format!(
            "#!/bin/sh\ncat > /dev/null 2>&1\nprintf 'child-cwd:%s\\n' \"$(pwd -P)\" >> {log}\n\
             printf 'provider-ran\\n' >> {log}\nexit 0\n",
            log = events_log.display(),
        ),
    );
}

/// A minimal probe target whose only job is to stamp a terminal marker once the
/// provider has launched, so a run can be settled on it.
const LAUNCH_FACET_PROBE_TARGET: &str = "---\ntitle: launch facet probe\nsuccess:\n  stack:\n    \
     - action: {append_line: ['events.log', 'probe-done']}\n---\nlaunch facet probe body\n";

/// **Acceptance criterion 10, effective child CWD.** The provider child spawns in
/// the launch area regardless of route: invoked directly or reached through an
/// `initialize` proxy, the fake provider records the same effective working
/// directory (its own arm's repository root). The launch CWD is invocation-level
/// state borrowed into every launch bundle, so the proxy hand-off must not shift
/// it.
#[test]
#[serial(level2_lifecycle_control)]
fn level2_lifecycle_equivalence_child_cwd_matches_direct_run() {
    require_level!(Level::L2, TmuxHarness::available(), "tmux");

    let child_cwd = |staged: &Staged, pane: &str, arm: &str| -> String {
        let lines = event_lines(staged);
        let line = lines
            .iter()
            .find(|l| l.starts_with("child-cwd:"))
            .unwrap_or_else(|| {
                panic!("[{arm}] the provider must record its CWD; got {lines:?}; pane:\n{pane}")
            });
        line.trim_start_matches("child-cwd:").to_string()
    };
    // Each arm's child must spawn in that arm's own launch area (repo root).
    let expect_launch_area = |staged: &Staged, recorded: &str, arm: &str| {
        let want = staged
            .workspace
            .path()
            .canonicalize()
            .expect("workspace canonicalizes");
        let got = Path::new(recorded)
            .canonicalize()
            .unwrap_or_else(|_| Path::new(recorded).to_path_buf());
        assert_eq!(
            got, want,
            "[{arm}] the provider child must spawn in the launch area (repo root); \
             recorded {recorded}"
        );
    };

    // Direct: the probe target IS the invoked document.
    let direct = {
        let mut staged = stage_proxy_pair(EQUIV_ROUTER, LAUNCH_FACET_PROBE_TARGET, true);
        write_cwd_recording_goose(&staged.bin_dir, &staged.events_log);
        staged.md_file = staged.workspace.path().join("target.md");
        staged
    };
    let direct_pane = run_provider_in_tmux_for(&direct, "--goose", "probe-done");
    let direct_cwd = child_cwd(&direct, &direct_pane, "direct");
    expect_launch_area(&direct, &direct_cwd, "direct");

    // Routed: the router is invoked and proxies to the probe target at initialize.
    let routed = {
        let staged = stage_proxy_pair(EQUIV_ROUTER, LAUNCH_FACET_PROBE_TARGET, true);
        write_cwd_recording_goose(&staged.bin_dir, &staged.events_log);
        staged
    };
    let routed_pane = run_provider_in_tmux_for(&routed, "--goose", "probe-done");
    let routed_cwd = child_cwd(&routed, &routed_pane, "routed");
    expect_launch_area(&routed, &routed_cwd, "routed");
}

/// A fake `goose` that records whether a system-prompt sentinel reached its argv.
/// Goose delivers a non-interactive appended system prompt via the `--system`
/// inline flag, so the sentinel rides an argument.
fn write_sysprompt_recording_goose(bin_dir: &Path, events_log: &Path, sentinel: &str) {
    write_executable(
        &bin_dir.join("goose"),
        &format!(
            "#!/bin/sh\ncat > /dev/null 2>&1\nfor a in \"$@\"; do\n  case \"$a\" in\n    \
             *{sentinel}*) printf 'sysprompt-seen\\n' >> {log} ;;\n  esac\ndone\n\
             printf 'provider-ran\\n' >> {log}\nexit 0\n",
            log = events_log.display(),
            sentinel = sentinel,
        ),
    );
}

/// **Acceptance criterion 10, system-prompt delivery.** A CLI-supplied
/// `--append-system-prompt` is delivered to the provider whether the target is
/// invoked directly or reached through an `initialize` proxy: the proxy hand-off
/// must not drop the borrowed launch bundle's system prompt. Explicit CLI intent
/// is immutable invocation state that survives the route.
#[test]
#[serial(level2_lifecycle_control)]
fn level2_lifecycle_equivalence_cli_system_prompt_survives_the_proxy() {
    require_level!(Level::L2, TmuxHarness::available(), "tmux");

    const SYSPROMPT_SENTINEL: &str = "SYSPROMPTsentinelXYZ";

    let run_arm = |routed: bool, arm: &str| {
        let mut staged = stage_proxy_pair(EQUIV_ROUTER, LAUNCH_FACET_PROBE_TARGET, true);
        write_sysprompt_recording_goose(&staged.bin_dir, &staged.events_log, SYSPROMPT_SENTINEL);
        let sp_file = staged.workspace.path().join("sysprompt.txt");
        fs::write(&sp_file, format!("{SYSPROMPT_SENTINEL}\n")).unwrap();
        if !routed {
            staged.md_file = staged.workspace.path().join("target.md");
        }
        let extra = format!("--append-system-prompt {}", sp_file.display());
        let pane = run_provider_with_flags(&staged, "--goose", &extra, "probe-done");
        let lines = event_lines(&staged);
        assert!(
            lines.iter().any(|l| l == "sysprompt-seen"),
            "[{arm}] the CLI-supplied system prompt must reach the provider; \
             got {lines:?}; pane:\n{pane}"
        );
    };

    run_arm(false, "direct");
    run_arm(true, "routed");
}

/// A fake provider named for its slug that records which provider actually
/// launched. Staging two of these lets a test tell the router's default apart
/// from the target's authored provider.
fn write_named_provider(bin_dir: &Path, slug: &str, events_log: &Path) {
    write_executable(
        &bin_dir.join(slug),
        &format!(
            "#!/bin/sh\ncat > /dev/null 2>&1\nprintf 'launched={slug}\\n' >> {log}\nexit 0\n",
            slug = slug,
            log = events_log.display(),
        ),
    );
}

/// **Acceptance criterion 10, target-authored provider selection.** On the
/// surfaced compose coordinator path, a proxied target's provider is rebuilt from
/// the *target's* own frontmatter (`prepare_and_run_active_document` re-runs
/// `eagerly_resolve_target` against the adopted document's raw hints), so a router
/// with no `--provider` flag that hands off to a target which authors a different
/// provider launches the *target's* provider — identical to invoking the target
/// directly. An explicit CLI `--provider` still wins (immutable invocation
/// intent).
///
/// Both fake providers are inert `#!/bin/sh` stubs that record which one ran. The
/// router authors `goose` only so its own eager resolution succeeds before it
/// proxies at `initialize`; it never launches (the hand-off precedes `start`).
#[test]
#[serial(level2_lifecycle_control)]
fn level2_lifecycle_equivalence_target_authored_provider_matches_direct_run() {
    require_level!(Level::L2, TmuxHarness::available(), "tmux");

    // Router pins `goose` (so its pre-proxy resolution is unambiguous), proxies at
    // initialize; the target authors a *different* provider, `codex`.
    let router = "---\ntitle: provider router\nagent: goose\ninitialize:\n  stack:\n    \
         - action: {proxy: '@target.md'}\n---\nrouter body\n";
    let target = "---\ntitle: provider target\nagent: codex\nsuccess:\n  stack:\n    \
         - action: {append_line: ['events.log', 'target-done']}\n---\ntarget body\n";

    let stage_pair = || {
        let staged = stage_proxy_pair(router, target, true);
        write_named_provider(&staged.bin_dir, "goose", &staged.events_log);
        write_named_provider(&staged.bin_dir, "codex", &staged.events_log);
        staged
    };

    // Direct: invoke the target itself, no CLI provider flag → its authored
    // `codex` launches. This is the contract the routed arm must match.
    let direct = {
        let mut staged = stage_pair();
        staged.md_file = staged.workspace.path().join("target.md");
        staged
    };
    let direct_pane = run_provider_in_tmux_for(&direct, "", "target-done");
    let direct_lines = event_lines(&direct);
    assert!(
        direct_lines.iter().any(|l| l == "launched=codex"),
        "fixture check: invoked directly, the target's authored `codex` launches; \
         got {direct_lines:?}; pane:\n{direct_pane}"
    );

    // Routed: invoke the router (no CLI provider flag). It proxies at initialize;
    // the surfaced coordinator rebuilds the target's provider from the target's own
    // frontmatter, so `codex` launches — and the router's `goose` never does.
    let routed = stage_pair();
    let routed_pane = run_provider_in_tmux_for(&routed, "", "target-done");
    let routed_lines = event_lines(&routed);
    assert!(
        routed_lines.iter().any(|l| l == "launched=codex"),
        "a proxied target must launch its OWN authored provider (surfaced-path R6 \
         provider rebuild), identical to a direct invocation; got {routed_lines:?}; \
         pane:\n{routed_pane}"
    );
    assert!(
        !routed_lines.iter().any(|l| l == "launched=goose"),
        "the router's default provider must NOT launch for the proxied target — the \
         hand-off precedes `start` and the target's provider is rebuilt; \
         got {routed_lines:?}; pane:\n{routed_pane}"
    );

    // Precedence: an explicit CLI `--goose` outranks the target's authored `codex`
    // even through the proxy — explicit invocation intent stays authoritative.
    let pinned = stage_pair();
    let pinned_pane = run_provider_in_tmux_for(&pinned, "--goose", "target-done");
    let pinned_lines = event_lines(&pinned);
    assert!(
        pinned_lines.iter().any(|l| l == "launched=goose"),
        "an explicit CLI `--goose` must win over the target's authored `codex`; \
         got {pinned_lines:?}; pane:\n{pinned_pane}"
    );
    assert!(
        !pinned_lines.iter().any(|l| l == "launched=codex"),
        "the target's authored provider must not override an explicit CLI flag; \
         got {pinned_lines:?}; pane:\n{pinned_pane}"
    );
}

/// A fake provider that records the whole launch bundle it was handed: which
/// binary and entrypoint were selected, the flag-shaped argv, and the effective
/// child environment Claudine built for it.
///
/// Only flag-shaped argv tokens are recorded. The positional prompt and every
/// temporary-file argument carry per-arm paths that legitimately differ, whereas
/// the flags are exactly the profile/structured-mode/dispatch decisions the row
/// asserts on. `CLAUDINE_SESSION_ID` is a fresh UUID per launch, so only its
/// presence is recorded, never its value.
fn write_launch_bundle_recorder(bin_dir: &Path, slug: &str, events_log: &Path) {
    write_executable(
        &bin_dir.join(slug),
        &format!(
            "#!/bin/sh\ncat > /dev/null 2>&1\n\
             printf 'launched-binary=%s\\n' \"$(basename \"$0\")\" >> {log}\n\
             printf 'entrypoint=%s\\n' \"$1\" >> {log}\n\
             printf 'argv-flags=%s\\n' \"$(for a in \"$@\"; do case \"$a\" in -*) printf '%s ' \"$a\" ;; esac; done)\" >> {log}\n\
             printf 'child.AGENT=%s\\n' \"$AGENT\" >> {log}\n\
             printf 'child.MODEL=%s\\n' \"$MODEL\" >> {log}\n\
             printf 'child.INTERACTIVE=%s\\n' \"$INTERACTIVE\" >> {log}\n\
             printf 'child.YOLO=%s\\n' \"$YOLO\" >> {log}\n\
             printf 'child.CLAUDINE_INTERACTIVE=%s\\n' \"$CLAUDINE_INTERACTIVE\" >> {log}\n\
             if [ -n \"$CLAUDINE_SESSION_ID\" ]; then printf 'child.session-id=present\\n' >> {log}; \
             else printf 'child.session-id=absent\\n' >> {log}; fi\n\
             if [ -n \"$AGENT_PARAMS\" ]; then printf 'child.agent-params=present\\n' >> {log}; \
             else printf 'child.agent-params=absent\\n' >> {log}; fi\n\
             printf 'provider-ran\\n' >> {log}\nexit 0\n",
            log = events_log.display(),
        ),
    );
}

/// A router that resolves `goose` for its own pre-proxy selection and hands off
/// at `initialize`. It authors no `model:` and no `interactive:`, so anything the
/// target observes for those facets is target-owned by construction.
const LAUNCH_BUNDLE_ROUTER: &str = r#"---
title: launch bundle router
agent: goose
initialize:
  stack:
    - action: {proxy: "@target.md"}
---
router body
"#;

/// The launch-bundle probe: it authors a **different** provider than its router
/// plus an explicit `interactive:` so the recorded bundle can only match a
/// bundle rebuilt for this document.
const LAUNCH_BUNDLE_TARGET: &str = r#"---
title: launch bundle target
agent: codex
interactive: false
success:
  stack:
    - action: {append_line: ["events.log", "sig=success"]}
---
launch bundle target body
"#;

/// **Acceptance criterion 10, target-driven launch bundle.** A proxied target's
/// profile/binary, entrypoint, argv, effective child environment, interactivity,
/// structured-output mode, and dispatch configuration are all rebuilt for the
/// *target* and match the same target invoked directly.
///
/// Neither arm passes a provider flag, so the whole bundle follows the target's
/// authored `agent: codex` — not the router's `goose`. That is the row's
/// anti-vacuity property, and it is asserted directly: the routed arm must record
/// `launched-binary=codex`, and the router's `goose` stub (which is on `PATH` and
/// would happily record itself) must never run.
///
/// The recorded facets stand in for the bundle as follows: `launched-binary` and
/// `entrypoint` are the profile/binary decision; `argv-flags` carries the
/// structured-output-mode and provider-profile flags; `child.INTERACTIVE` /
/// `child.CLAUDINE_INTERACTIVE` are the resolved interactivity as delivered to
/// the child and to the hook subprocess respectively; `child.session-id` and
/// `child.agent-params` are the dispatch/correlation configuration.
#[test]
#[serial(level2_lifecycle_control)]
fn level2_lifecycle_equivalence_target_launch_bundle_matches_direct_run() {
    require_level!(Level::L2, TmuxHarness::available(), "tmux");

    // 10 recorder lines + provider-ran + sig=success.
    const EXPECTED_MARKERS: usize = 12;

    let arms = equivalence_arms_configured(
        LAUNCH_BUNDLE_ROUTER,
        LAUNCH_BUNDLE_TARGET,
        "",
        EXPECTED_MARKERS,
        |staged| {
            write_launch_bundle_recorder(&staged.bin_dir, "codex", &staged.events_log);
            write_launch_bundle_recorder(&staged.bin_dir, "goose", &staged.events_log);
        },
    );

    let bundle = |lines: &[String]| -> Vec<String> {
        lines
            .iter()
            .filter(|line| {
                line.starts_with("launched-binary=")
                    || line.starts_with("entrypoint=")
                    || line.starts_with("argv-flags=")
                    || line.starts_with("child.")
            })
            .cloned()
            .collect()
    };
    let direct_bundle = bundle(&arms.direct_lines);
    let routed_bundle = bundle(&arms.routed_lines);

    // Fixture check: the direct arm resolved the target's own provider and its
    // authored non-interactive mode, and Claudine handed the child a full
    // dispatch/correlation environment.
    for expected in [
        "launched-binary=codex",
        "child.AGENT=codex",
        "child.INTERACTIVE=false",
        "child.CLAUDINE_INTERACTIVE=0",
        "child.session-id=present",
        "child.agent-params=present",
    ] {
        assert!(
            direct_bundle.iter().any(|line| line == expected),
            "fixture check: invoked directly, the target's launch bundle must \
             carry `{expected}`; got {direct_bundle:?}; pane:\n{}",
            arms.direct_pane
        );
    }
    assert!(
        direct_bundle
            .iter()
            .any(|line| line.starts_with("entrypoint=") && line.len() > "entrypoint=".len()),
        "fixture check: the selected profile must contribute an entrypoint \
         subcommand; got {direct_bundle:?}; pane:\n{}",
        arms.direct_pane
    );

    // The router's provider must never launch on the routed arm: the hand-off
    // precedes `start`, and the bundle is rebuilt for the adopted target.
    assert!(
        !arms
            .routed_lines
            .iter()
            .any(|line| line == "launched-binary=goose"),
        "the router's `goose` must not launch for the proxied target; got {:?}; \
         pane:\n{}",
        arms.routed_lines,
        arms.routed_pane
    );

    assert_eq!(
        routed_bundle, direct_bundle,
        "a proxied target's profile/binary, entrypoint, argv, effective child \
         environment, interactivity, structured-output mode, and dispatch \
         configuration must all be rebuilt for the target and match the same \
         target invoked directly; pane:\n{}",
        arms.routed_pane
    );
}

/// The catalog id the MCP row's target activates through a prompt tag.
const MCP_PROBE_SERVER: &str = "proxyprobeserver";

/// Seed a hermetic MCP catalog under the arm's `HOME` containing exactly one
/// server, with both the user-scope and repo-scope default lists empty.
///
/// Empty defaults are what make the row's tag assertion meaningful: the only way
/// `proxyprobeserver` can enter a session set is the `#proxyprobeserver` tag in
/// the *target's* body. Seeding all three state files also short-circuits
/// `bootstrap_mcp_state`, so no native provider config on the host machine can
/// leak into the fixture.
fn seed_mcp_catalog(workspace: &Path) {
    let mcp_dir = workspace.join(".claudine").join("mcp");
    fs::create_dir_all(&mcp_dir).unwrap();
    // Runtime MCP injection for Codex and Gemini runs through a shadow HOME,
    // and the shadow-home builder mirrors the *original* provider config
    // directory — it refuses to run when that directory is missing. `HOME` is
    // the arm's temporary workspace, so both must be materialized here.
    fs::create_dir_all(workspace.join(".gemini")).unwrap();
    fs::create_dir_all(workspace.join(".codex")).unwrap();
    fs::write(
        mcp_dir.join("catalog.json"),
        format!(
            r#"{{"version":1,"servers":{{"{id}":{{"id":"{id}","transport":"stdio",
               "command":"/bin/true","args":["--probe"],"metadata":{{
               "fingerprint":"probe","created_at":"2026-01-01T00:00:00Z",
               "updated_at":"2026-01-01T00:00:00Z"}}}}}}}}"#,
            id = MCP_PROBE_SERVER,
        ),
    )
    .unwrap();
    fs::write(mcp_dir.join("defaults.json"), r#"{"version":1,"defaults":[]}"#).unwrap();
    fs::write(
        mcp_dir.join("provider-state.json"),
        r#"{"version":1,"providers":{},"repos":{}}"#,
    )
    .unwrap();
    // Repo-scope defaults replace user-scope defaults; seed them empty too so
    // the repo scope cannot reintroduce a default server.
    fs::write(
        workspace.join(".claudine").join("mcp.json"),
        r#"{"version":1,"defaults":[]}"#,
    )
    .unwrap();
}

/// A fake provider that records the MCP state its launch actually received.
///
/// `--allowed-mcp-server-names` is contributed by the Gemini injector alone —
/// no other provider's injector produces argv at all — so both the flag's
/// presence and its value are evidence that MCP was composed for *this*
/// provider, with the server set the target's own prompt tag selected.
fn write_mcp_recording_gemini(bin_dir: &Path, events_log: &Path) {
    write_executable(
        &bin_dir.join("gemini"),
        &format!(
            "#!/bin/sh\ncat > /dev/null 2>&1\n\
             printf 'launched-binary=%s\\n' \"$(basename \"$0\")\" >> {log}\n\
             allowed=none\nprev=\n\
             for a in \"$@\"; do\n  \
             if [ \"$prev\" = '--allowed-mcp-server-names' ]; then allowed=\"$a\"; fi\n  \
             prev=\"$a\"\ndone\n\
             printf 'mcp-allowed=%s\\n' \"$allowed\" >> {log}\n\
             printf 'provider-ran\\n' >> {log}\nexit 0\n",
            log = events_log.display(),
        ),
    );
}

/// A router on an MCP-capable provider that is **not** the target's. Its own body
/// carries no tag, so its session set is empty; a routed arm that reused the
/// router's MCP plan would inject nothing at all.
const MCP_ROUTER: &str = r#"---
title: mcp router
agent: codex
initialize:
  stack:
    - action: {proxy: "@target.md"}
---
router body with no tag
"#;

/// The MCP probe: it authors its own provider and carries the activating tag in
/// its body. The tag is lexed out of the prompt before delivery, so the only way
/// the server id can reach the child's argv is Gemini's own MCP injection.
const MCP_PROBE_TARGET: &str = r#"---
title: mcp target
agent: gemini
success:
  stack:
    - action: {append_line: ["events.log", "sig=success"]}
---
mcp target body #proxyprobeserver
"#;

/// **Acceptance criterion 10, target-specific MCP tags and injection.** A proxied
/// target's MCP plan is rebuilt from the *target's* provider and the *target's*
/// prompt tags, and matches the same target invoked directly.
///
/// This is the provider-switch case: the router authors `codex` (whose injector
/// writes a shadow-home TOML and contributes no argv), the target authors
/// `gemini` (whose injector contributes `--allowed-mcp-server-names`). No
/// provider flag is passed, so the injector actually used can only have been
/// chosen from the target's own frontmatter.
///
/// Non-vacuity comes from three directions: the seeded catalog has empty
/// defaults, so the server can only enter a session set through the target's
/// tag; the tag is lexed out of the prompt before delivery, so the id appearing
/// in argv cannot be the prompt echoing itself; and the direct arm's exact
/// `launched-binary`/`mcp-allowed` values must hold before the arms are compared
/// at all.
#[test]
#[serial(level2_lifecycle_control)]
fn level2_lifecycle_equivalence_target_mcp_injection_matches_direct_run() {
    require_level!(Level::L2, TmuxHarness::available(), "tmux");

    // launched-binary, mcp-allowed, provider-ran, sig=success.
    const EXPECTED_MARKERS: usize = 4;

    let arms = equivalence_arms_configured(
        MCP_ROUTER,
        MCP_PROBE_TARGET,
        "--mcp",
        EXPECTED_MARKERS,
        |staged| {
            seed_mcp_catalog(staged.workspace.path());
            write_mcp_recording_gemini(&staged.bin_dir, &staged.events_log);
            write_named_provider(&staged.bin_dir, "codex", &staged.events_log);
        },
    );

    for expected in [
        "launched-binary=gemini".to_string(),
        format!("mcp-allowed={MCP_PROBE_SERVER}"),
    ] {
        assert!(
            arms.direct_lines.contains(&expected),
            "fixture check: invoked directly, the target's own provider is \
             selected and its tagged MCP server is injected (`{expected}`); got \
             {:?}; pane:\n{}",
            arms.direct_lines,
            arms.direct_pane
        );
    }
    assert!(
        !arms.routed_lines.iter().any(|line| line == "launched=codex"),
        "the router's `codex` must not launch for the proxied target; got {:?}; \
         pane:\n{}",
        arms.routed_lines,
        arms.routed_pane
    );

    assert_eq!(
        arms.routed_lines, arms.direct_lines,
        "a proxied target's MCP plan must be rebuilt from the target's provider \
         and the target's prompt tags, matching the same target invoked \
         directly; pane:\n{}",
        arms.routed_pane
    );
}

// ── Acceptance criterion 28 — three-route typed-diagnostic matrix ───────────
//
// AC28 requires that the *same target failure* keeps the same typed diagnostic
// identity and the same actionable rendering whether the target was invoked
// directly, reached by a proxy from `initialize`, or reached by a proxy from
// terminal recovery. The Level 1 preparation-service tests
// (`prepare::service::tests::a_schema_failure_has_one_typed_identity_across_every_entry`)
// compare Rust variants at one layer; they cannot show that the shipped binary
// carries that variant out through each coordinator/harness boundary and renders
// the same block on a real terminal. These rows do.
//
// Each row runs one failure fixture through all three routes and compares the
// rendered diagnostic. The only difference the contract permits between routes
// is proxy provenance — the `flow control redirected` line — which is asserted
// present on the two proxy routes and absent on the direct one.
//
// ## The divergence these rows caught
//
// `prepare_and_run_active_document` had no schema pre-validation in front of its
// pre-flight compose, while the invocation boundary (`run_composition_inner`)
// did. A proxied target therefore reached Darkmatter's built-in schema
// validation first and failed with the raw `MarkdownError: schema validation
// failed` — different variant, different remediation text, and no frontmatter
// excerpt — where the identical document invoked directly produced the typed
// `CompositionError: schema validation`. Both schema rows below fail against
// that ordering, which is what makes them non-vacuous as regression cover.

/// Which entry reason drives one cell of the matrix.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum DiagnosticRoute {
    /// The target document is named on the command line.
    Direct,
    /// A router hands off at `initialize`, before its own `start`.
    InitializeProxy,
    /// A router's provider fails and its `failure` stack hands off.
    TerminalRecoveryProxy,
}

impl DiagnosticRoute {
    const ALL: [Self; 3] = [
        Self::Direct,
        Self::InitializeProxy,
        Self::TerminalRecoveryProxy,
    ];

    fn is_proxy(self) -> bool {
        self != Self::Direct
    }

    /// The router that reaches the target, or `None` for the direct route.
    ///
    /// The terminal-recovery router stamps `source-failure` so the row can prove
    /// the provider really failed and the *terminal* stack really ran, rather
    /// than the run having taken some earlier exit that happens to render the
    /// same text.
    fn router_doc(self, overlay: &str) -> Option<String> {
        match self {
            Self::Direct => None,
            Self::InitializeProxy => Some(format!(
                "---\ntitle: initialize router\ninitialize:\n  stack:\n    \
                 - action: {{action: proxy, target: '@target.md', with: {overlay}}}\n\
                 ---\nrouter body\n"
            )),
            Self::TerminalRecoveryProxy => Some(format!(
                "---\ntitle: terminal recovery router\nfailure:\n  stack:\n    \
                 - action: {{append_line: ['events.log', 'source-failure']}}\n    \
                 - action: {{action: proxy, target: '@target.md', with: {overlay}}}\n\
                 ---\nrouter body\n"
            )),
        }
    }
}

/// One failure class, expressed so all three routes can reproduce it.
struct DiagnosticFixture {
    /// The target document. Written to `target.md` on every route, so all three
    /// arms fail on the *same file* and source attribution is comparable.
    target_doc: &'static str,
    /// Caller `key=value` positionals that supply, on the direct route, the same
    /// invalid value an overlay supplies on the proxy routes. Empty when the
    /// failure is authored into the target itself.
    direct_setters: &'static str,
    /// The `with:` mapping each router carries. `{}` when the failure needs no
    /// overlay.
    overlay: &'static str,
    /// The rendered typed identity: error-family name plus variant label, as the
    /// `ErrorHeader` renders it. This is the identity AC28 is about — not a
    /// substring of the human-readable prose underneath it.
    identity: &'static str,
    /// What the diagnostic must name so the operator lands on the right file.
    ///
    /// Which *thing* carries the attribution is a property of the error family,
    /// not of the route: a schema verdict names the document it validated, while
    /// a transclusion failure names the reference it could not resolve (anchored,
    /// as the resolved path in the body shows, on the target's own directory).
    /// Both are target-owned; neither may name the router.
    attribution: &'static str,
}

/// A schema-invalid value authored into the target's own frontmatter.
const DIAG_SCHEMA_FAILURE: DiagnosticFixture = DiagnosticFixture {
    target_doc: "---\n$schema:\n    count: 'number(required)'\ncount: not-a-number\n\
                 ---\ntarget body\n",
    direct_setters: "",
    overlay: "{}",
    identity: "CompositionError: schema validation",
    attribution: "target.md",
};

/// An invalid `proxy.with` overlay (acceptance criterion 24): the target
/// declares a required `count` it does not author, and the overlay supplies a
/// value of the wrong type.
///
/// The direct arm supplies the identical bad value through the caller's own
/// `key=value` mechanism, which is the honest comparison — "an invalid overlay
/// produces the *normal* typed target schema error" is precisely the claim.
const DIAG_INVALID_OVERLAY: DiagnosticFixture = DiagnosticFixture {
    target_doc: "---\n$schema:\n    count: 'number(required)'\n---\ntarget body\n",
    direct_setters: "count=not-a-number",
    overlay: "{count: 'not-a-number'}",
    identity: "CompositionError: schema validation",
    attribution: "target.md",
};

/// A typed target-*preparation* failure that is not a schema verdict: the
/// target's body transcludes a partial that does not exist, so preparation fails
/// inside Darkmatter's compose pipeline and the typed cause has to survive out
/// through the walker on every route.
const DIAG_PREPARATION_FAILURE: DiagnosticFixture = DiagnosticFixture {
    target_doc: "---\ntitle: preparation failure target\n---\n::file _no_such_partial.md\n",
    direct_setters: "",
    overlay: "{}",
    identity: "TransclusionError: I/O failure",
    attribution: "no_such_partial.md",
};

/// One cell's observable outcome.
struct RouteDiagnostic {
    pane: String,
    exit_code: Option<i32>,
    lines: Vec<String>,
    /// The rendered diagnostic, from its identity header to the end of the
    /// output, with workspace-specific paths normalized.
    tail: Vec<String>,
}

/// Run one (route × failure) cell and capture its pane, exit status, side-effect
/// log, and normalized diagnostic tail.
fn run_diagnostic_route(route: DiagnosticRoute, fixture: &DiagnosticFixture) -> RouteDiagnostic {
    static SEQ: AtomicU32 = AtomicU32::new(0);
    let seq = SEQ.fetch_add(1, Ordering::Relaxed);

    // The failing provider serves both purposes: it drives the terminal-recovery
    // router's `failure` event, and on the other two routes it is the stub that
    // must never be reached (the target fails before any launch).
    let router = route.router_doc(fixture.overlay);
    let mut staged = stage_proxy_pair(
        router.as_deref().unwrap_or("---\ntitle: unused\n---\nunused\n"),
        fixture.target_doc,
        false,
    );
    if router.is_none() {
        staged.md_file = staged.workspace.path().join("target.md");
    }
    let setters = if route.is_proxy() {
        ""
    } else {
        fixture.direct_setters
    };

    let session = format!("biscuit_l2_lcdiag_{}_{seq}", std::process::id());
    let shell = biscuit_test_harness::detect_shell();
    let spawned = std::process::Command::new("tmux")
        .args([
            "new-session",
            "-d",
            "-s",
            &session,
            "-x",
            "200",
            // Taller than the other runners: the terminal-recovery route renders
            // the router's whole launch surface (system prompt, agent prompt,
            // provider failure) above the diagnostic, and `capture()` has no
            // scrollback — a 60-row pane would scroll the provenance line off.
            "-y",
            "120",
            &format!("{shell} -l"),
        ])
        .status()
        .map(|s| s.success())
        .unwrap_or(false);
    assert!(spawned, "failed to spawn tmux session");

    let mut harness = TmuxHarness::attach(&session);
    let _ = biscuit_test_harness::wait_for_prompt(&mut harness);

    let claudine = env!("CARGO_BIN_EXE_claudine");
    // `%s` keeps the literal token off the echoed command line's own `=<digits>`
    // shape, so the parse below cannot mistake the command for its output.
    let exit_token = format!("L2DIAGEXIT_{seq}");
    let cmd = format!(
        "cd {ws} && NO_COLOR='1' MODEL='' HOME='{home}' PATH='{path}' \
         {claudine} compose --goose {md} {setters} ; printf '{exit_token}=%s\\n' $?",
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
    let mut exit_code = None;
    while Instant::now() < deadline {
        pane = harness.capture().map(|f| f.plain).unwrap_or_default();
        if let Some(code) = parse_exit_token(&pane, &exit_token) {
            exit_code = Some(code);
            break;
        }
        std::thread::sleep(Duration::from_millis(100));
    }
    kill_session_by_name(&session);

    let tail = diagnostic_tail(&pane, fixture.identity, &exit_token, &staged);
    RouteDiagnostic {
        lines: event_lines(&staged),
        tail,
        pane,
        exit_code,
    }
}

/// Read the shell's reported exit status back off the pane.
fn parse_exit_token(pane: &str, token: &str) -> Option<i32> {
    pane.lines().find_map(|line| {
        line.trim()
            .strip_prefix(token)
            .and_then(|rest| rest.strip_prefix('='))
            .filter(|code| !code.is_empty() && code.chars().all(|c| c.is_ascii_digit()))
            .and_then(|code| code.parse().ok())
    })
}

/// The rendered diagnostic: every line from the identity header down to the exit
/// token, with the arm's temporary workspace path replaced by a placeholder.
///
/// Starting at the identity header is what scopes the comparison to the
/// diagnostic itself. Everything a route legitimately renders *before* it — the
/// execution header, the router's own launch surface, and the proxy provenance
/// line — is route-specific by design and is asserted separately.
fn diagnostic_tail(
    pane: &str,
    identity: &str,
    exit_token: &str,
    staged: &Staged,
) -> Vec<String> {
    let workspace = staged.workspace.path().display().to_string();
    pane.lines()
        .skip_while(|line| !line.contains(identity))
        .take_while(|line| !line.trim().starts_with(exit_token))
        .map(|line| line.trim().replace(&workspace, "<WS>"))
        .filter(|line| !line.is_empty())
        .collect()
}

/// Assert everything one cell owes on its own, before any cross-route
/// comparison: exit status, typed identity, styled rendering, source
/// attribution, single rendering, proxy provenance, and the anti-vacuity
/// evidence that the route actually ran the way it claims.
fn assert_diagnostic_cell(
    route: DiagnosticRoute,
    fixture: &DiagnosticFixture,
    diag: &RouteDiagnostic,
) {
    let pane = &diag.pane;

    assert_eq!(
        diag.exit_code,
        Some(1),
        "{route:?}: a failed target must exit non-zero with the composition \
         failure status; pane:\n{pane}"
    );

    // The typed identity — error family plus variant label — rendered exactly
    // once. Counting is the duplicate-rendering assertion: a diagnostic caught
    // and re-rendered at two boundaries (the historical failure mode on the
    // proxy routes) shows up here as two.
    let renderings = pane.matches(fixture.identity).count();
    assert_eq!(
        renderings,
        1,
        "{route:?}: the typed diagnostic `{}` must be rendered exactly once, \
         got {renderings}; pane:\n{pane}",
        fixture.identity,
    );

    assert!(
        diag.tail.iter().any(|line| line.starts_with('┃')),
        "{route:?}: the diagnostic must render as a styled bordered block, not a \
         crude single-line `Error: …`; tail {:?}; pane:\n{pane}",
        diag.tail,
    );

    // Source attribution: the failure is attributed to the target's own
    // document or reference on every route. A router that put its own name here
    // would send the operator to the wrong file.
    assert!(
        diag.tail.iter().any(|line| line.contains(fixture.attribution)),
        "{route:?}: the diagnostic must attribute the failure to `{}`; \
         tail {:?}; pane:\n{pane}",
        fixture.attribution,
        diag.tail,
    );
    assert!(
        !diag.tail.iter().any(|line| line.contains("<WS>/doc.md")),
        "{route:?}: the diagnostic must not attribute the target's failure to the \
         router; tail {:?}; pane:\n{pane}",
        diag.tail,
    );

    // Proxy provenance is the one intentional difference between the routes.
    let provenance = pane.contains("flow control redirected to target.md");
    assert_eq!(
        provenance,
        route.is_proxy(),
        "{route:?}: the proxy provenance line must be present on proxy routes and \
         absent on the direct route; pane:\n{pane}"
    );

    // Anti-vacuity. Without these an early abort — a router that never resolved,
    // a provider that never launched, a run that died before the hand-off —
    // would leave the assertions above trivially satisfiable by a pane that
    // never exercised the route at all.
    assert!(
        diag.tail.len() >= 2,
        "{route:?}: the captured diagnostic must carry a header and a body; \
         tail {:?}; pane:\n{pane}",
        diag.tail,
    );
    let provider_runs = diag.lines.iter().filter(|l| **l == "provider-ran").count();
    match route {
        DiagnosticRoute::TerminalRecoveryProxy => {
            assert_eq!(
                provider_runs, 1,
                "{route:?}: the router's provider must launch exactly once — the \
                 terminal-recovery route is only reached through a real provider \
                 failure; got {:?}; pane:\n{pane}",
                diag.lines,
            );
            assert!(
                diag.lines.iter().any(|l| l == "source-failure"),
                "{route:?}: the router's terminal `failure` stack must have run, \
                 which is what makes this the recovery route rather than an \
                 earlier exit; got {:?}; pane:\n{pane}",
                diag.lines,
            );
        }
        _ => assert_eq!(
            provider_runs, 0,
            "{route:?}: no provider may launch — the target fails during \
             preparation, before any launch; got {:?}; pane:\n{pane}",
            diag.lines,
        ),
    }
}

/// Run one fixture through all three routes and assert both halves of AC28: each
/// cell is individually sound, and the three rendered diagnostics are identical.
fn assert_route_equivalent_diagnostic(fixture: &DiagnosticFixture) {
    let cells: Vec<(DiagnosticRoute, RouteDiagnostic)> = DiagnosticRoute::ALL
        .iter()
        .map(|route| (*route, run_diagnostic_route(*route, fixture)))
        .collect();

    for (route, diag) in &cells {
        assert_diagnostic_cell(*route, fixture, diag);
    }

    // The direct arm is the contract; the proxy arms must match it exactly.
    // Everything route-specific was rendered above the identity header and is
    // excluded by construction, so what is compared here is the diagnostic and
    // nothing else.
    let (_, direct) = &cells[0];
    for (route, diag) in &cells[1..] {
        assert_eq!(
            diag.tail, direct.tail,
            "{route:?}: the same target failing the same way must produce the \
             same typed identity and the same actionable rendering as the direct \
             route (acceptance criterion 28); direct pane:\n{}\n\nrouted pane:\n{}",
            direct.pane, diag.pane,
        );
    }
}

/// **Acceptance criterion 28, schema failure.** A target whose own frontmatter
/// violates its `$schema` renders one typed `CompositionError: schema
/// validation` on all three routes.
#[test]
#[serial(level2_lifecycle_control)]
fn level2_lifecycle_diagnostic_matrix_schema_failure_is_route_equivalent() {
    require_level!(Level::L2, TmuxHarness::available(), "tmux");

    assert_route_equivalent_diagnostic(&DIAG_SCHEMA_FAILURE);
}

/// **Acceptance criteria 24 and 28, invalid overlay.** An invalid `proxy.with`
/// value produces the target's *normal* typed schema error — the same one the
/// caller's own `key=value` produces on the direct route — on all three routes,
/// and no provider is launched.
#[test]
#[serial(level2_lifecycle_control)]
fn level2_lifecycle_diagnostic_matrix_invalid_overlay_is_route_equivalent() {
    require_level!(Level::L2, TmuxHarness::available(), "tmux");

    assert_route_equivalent_diagnostic(&DIAG_INVALID_OVERLAY);

    // Anti-vacuity for this fixture specifically: the target authors no `count`,
    // so a run that never applied the overlay would fail as *missing* rather
    // than *invalid* — a different typed identity. Reaching the validation
    // verdict is therefore proof the overlay landed in the target's frontmatter.
    let routed = run_diagnostic_route(DiagnosticRoute::InitializeProxy, &DIAG_INVALID_OVERLAY);
    assert!(
        !routed.pane.contains("Required propert"),
        "the overlay must have supplied `count` — a missing-property diagnostic \
         would mean the row never exercised overlay validation at all; pane:\n{}",
        routed.pane
    );
}

/// **Acceptance criterion 28, typed target-preparation failure.** A failure that
/// is not a schema verdict — an unresolvable `::file` transclusion — keeps its
/// typed `TransclusionError` identity and its actionable rendering across all
/// three routes.
#[test]
#[serial(level2_lifecycle_control)]
fn level2_lifecycle_diagnostic_matrix_preparation_failure_is_route_equivalent() {
    require_level!(Level::L2, TmuxHarness::available(), "tmux");

    assert_route_equivalent_diagnostic(&DIAG_PREPARATION_FAILURE);
}

// ── Review 7, finding 1: `initialize` precedes the schema verdict ───────────
//
// R4 orders a fresh document's stages: narrow initialize-shell gate → its own
// `initialize` → schema validation → full pre-flight. The rows below pin the
// consequence an operator can see. A target that only satisfies its `$schema`
// *because* its `initialize` supplies the value must run — on every route. And
// when `initialize` cannot repair it, the target must still have run and still
// have paid its owed `blocked`/`finalize` before the diagnostic is rendered.

/// A target that violates its own `$schema` until its `initialize` repairs it.
///
/// `count` is required and unauthored, so any verdict reached before
/// `initialize` fails the document. `initialize` writes it; the body then proves
/// which read was delivered — the pre-`initialize` bootstrap composes
/// `count-is-`, the stabilized reread composes `count-is-7`.
const INIT_REPAIRS_SCHEMA_TARGET: &str = "---\n$schema:\n    count: 'number(required)'\n\
     initialize:\n  stack:\n    \
     - action: {append_line: ['events.log', 'target-init']}\n    \
     - action: {set_frontmatter: ['target.md', 'count', 7]}\n\
     finalize:\n  stack:\n    \
     - action: {append_line: ['events.log', 'target-finalize']}\n\
     ---\ncount-is-{{ count }}\n";

/// The same target, whose `initialize` writes a value that is *still* invalid.
///
/// The verdict is therefore genuinely owed — and owed *after* `initialize` and
/// after the target's own `blocked`/`finalize`, which is what the event log
/// pins.
const INIT_CANNOT_REPAIR_SCHEMA_TARGET: &str = "---\n$schema:\n    count: 'number(required)'\n\
     initialize:\n  stack:\n    \
     - action: {append_line: ['events.log', 'target-init']}\n    \
     - action: {set_frontmatter: ['target.md', 'count', 'still-not-a-number']}\n\
     blocked:\n  stack:\n    \
     - action: {append_line: ['events.log', 'target-blocked']}\n\
     finalize:\n  stack:\n    \
     - action: {append_line: ['events.log', 'target-finalize']}\n\
     ---\ncount-is-{{ count }}\n";

// The same two documents, plus a `loop:` they own. Owning a `loop:` routes a
// document down `execute_loop_or_single`'s loop branch instead of its single
// branch, and that branch used to hard-code `defer_schema_verdict: false` and an
// unconditional schema-aware iteration-1 prepare — so a looping document reached
// its verdict *before* its own `initialize`, and the identical document without
// `loop:` did not. These rows are the regression cover for that divergence.
//
// `phase` starts past the `until` bound, so the run is exactly one iteration.
// The claim under test is the stage order a looping document is given; later
// iterations deliberately reuse the prepared source snapshot rather than
// rereading (the ratified stage matrix), so a multi-pass fixture would be
// asserting a different contract.

/// [`INIT_REPAIRS_SCHEMA_TARGET`] with a `loop:` of its own.
const INIT_REPAIRS_SCHEMA_LOOP_TARGET: &str = "---\n$schema:\n    count: 'number(required)'\n\
     phase: 3\n\
     loop:\n  until: 'phase > 2'\n  action: 'increment(phase)'\n  max: 10\n\
     initialize:\n  stack:\n    \
     - action: {append_line: ['events.log', 'target-init']}\n    \
     - action: {set_frontmatter: ['target.md', 'count', 7]}\n\
     finalize:\n  stack:\n    \
     - action: {append_line: ['events.log', 'target-finalize']}\n\
     ---\ncount-is-{{ count }}\n";

/// [`INIT_CANNOT_REPAIR_SCHEMA_TARGET`] with a `loop:` of its own.
const INIT_CANNOT_REPAIR_SCHEMA_LOOP_TARGET: &str =
    "---\n$schema:\n    count: 'number(required)'\n\
     phase: 3\n\
     loop:\n  until: 'phase > 2'\n  action: 'increment(phase)'\n  max: 10\n\
     initialize:\n  stack:\n    \
     - action: {append_line: ['events.log', 'target-init']}\n    \
     - action: {set_frontmatter: ['target.md', 'count', 'still-not-a-number']}\n\
     blocked:\n  stack:\n    \
     - action: {append_line: ['events.log', 'target-blocked']}\n\
     finalize:\n  stack:\n    \
     - action: {append_line: ['events.log', 'target-finalize']}\n\
     ---\ncount-is-{{ count }}\n";

/// A `goose` that records its prompt, then succeeds only for the *target*'s
/// stabilized body.
///
/// One stub has to serve two opposite roles on the recovery route: the router's
/// provider must fail (that failure is what drives the `failure` stack that
/// proxies), and the target's must succeed. Keying on the target's own body text
/// is what lets a single binary do both without the arms diverging.
fn write_target_succeeding_goose(bin_dir: &Path, events_log: &Path) {
    write_executable(
        &bin_dir.join("goose"),
        &format!(
            "#!/bin/sh\nstdin=$(cat)\nprintf 'prompt:%s %s\\n' \"$stdin\" \"$*\" >> {log}\n\
             printf 'provider-ran\\n' >> {log}\n\
             case \"$stdin$*\" in\n  *count-is-*) exit 0 ;;\n  *) exit 99 ;;\nesac\n",
            log = events_log.display(),
        ),
    );
}

/// One cell of the initialize-before-schema matrix.
struct OrderingRun {
    pane: String,
    lines: Vec<String>,
    exit_code: Option<i32>,
}

/// Run `target_doc` on one route and return its pane, event log, and exit status.
///
/// Shares [`DiagnosticRoute`]'s router documents so the three routes here are the
/// same three the AC28 diagnostic matrix uses.
fn run_initialize_ordering_route(route: DiagnosticRoute, target_doc: &str) -> OrderingRun {
    static SEQ: AtomicU32 = AtomicU32::new(0);
    let seq = SEQ.fetch_add(1, Ordering::Relaxed);

    let router = route.router_doc("{}");
    let mut staged = stage_proxy_pair(
        router.as_deref().unwrap_or("---\ntitle: unused\n---\nunused\n"),
        target_doc,
        false,
    );
    write_target_succeeding_goose(&staged.bin_dir, &staged.events_log);
    if router.is_none() {
        staged.md_file = staged.workspace.path().join("target.md");
    }

    let session = format!("biscuit_l2_lcinit_{}_{seq}", std::process::id());
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
            "120",
            &format!("{shell} -l"),
        ])
        .status()
        .map(|s| s.success())
        .unwrap_or(false);
    assert!(spawned, "failed to spawn tmux session");

    let mut harness = TmuxHarness::attach(&session);
    let _ = biscuit_test_harness::wait_for_prompt(&mut harness);

    let claudine = env!("CARGO_BIN_EXE_claudine");
    let exit_token = format!("L2INITEXIT_{seq}");
    let cmd = format!(
        "cd {ws} && NO_COLOR='1' MODEL='' HOME='{home}' PATH='{path}' \
         {claudine} compose --goose {md} ; printf '{exit_token}=%s\\n' $?",
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
    let mut exit_code = None;
    while Instant::now() < deadline {
        pane = harness.capture().map(|f| f.plain).unwrap_or_default();
        if let Some(code) = parse_exit_token(&pane, &exit_token) {
            exit_code = Some(code);
            break;
        }
        std::thread::sleep(Duration::from_millis(100));
    }
    kill_session_by_name(&session);

    OrderingRun {
        lines: event_lines(&staged),
        pane,
        exit_code,
    }
}

/// Assert the whole ordering claim for one route: `initialize` ran once, it ran
/// before the verdict, and the delivered prompt came from the stabilized reread.
fn assert_initialize_precedes_schema(route: DiagnosticRoute, target_doc: &str) {
    let run = run_initialize_ordering_route(route, target_doc);
    let lines = &run.lines;
    let pane = &run.pane;

    assert_eq!(
        lines.iter().filter(|l| **l == "target-init").count(),
        1,
        "{route:?}: the target's `initialize` must run exactly once — the whole \
         point of the staged boot is that the reread does not re-emit it; \
         got {lines:?}; pane:\n{pane}"
    );

    // The load-bearing assertion. Reaching the provider at all means the schema
    // verdict was reached *after* `initialize` supplied the required property:
    // a verdict taken before it would have failed on a missing `count`.
    assert!(
        lines.iter().any(|l| l.contains("count-is-7")),
        "{route:?}: the delivered prompt must carry the value `initialize` \
         supplied, which is only possible if the verdict came after it; \
         got {lines:?}; pane:\n{pane}"
    );
    assert!(
        !pane.contains("Required propert") && !pane.contains("schema validation"),
        "{route:?}: no schema diagnostic may be rendered — `initialize` \
         satisfied the requirement; pane:\n{pane}"
    );
    assert!(
        lines.iter().any(|l| l == "target-finalize"),
        "{route:?}: the target must close its own run; got {lines:?}; \
         pane:\n{pane}"
    );
    assert_eq!(
        run.exit_code,
        Some(0),
        "{route:?}: the run must succeed; pane:\n{pane}"
    );
}

/// **R4 / acceptance criteria 11 and 12, direct.** A document whose own
/// `initialize` supplies a required schema property runs: the verdict is reached
/// after `initialize`, not before it.
#[test]
#[serial(level2_lifecycle_control)]
fn level2_lifecycle_initialize_precedes_schema_verdict_direct() {
    require_level!(Level::L2, TmuxHarness::available(), "tmux");

    assert_initialize_precedes_schema(DiagnosticRoute::Direct, INIT_REPAIRS_SCHEMA_TARGET);
}

/// **R4 / acceptance criteria 11 and 12, initialize proxy.** The same document
/// reached through an `initialize` proxy keeps the same stage order.
#[test]
#[serial(level2_lifecycle_control)]
fn level2_lifecycle_initialize_precedes_schema_verdict_initialize_proxy() {
    require_level!(Level::L2, TmuxHarness::available(), "tmux");

    assert_initialize_precedes_schema(DiagnosticRoute::InitializeProxy, INIT_REPAIRS_SCHEMA_TARGET);
}

/// **R4 / acceptance criteria 11 and 12, recovery proxy.** The same document
/// reached through a terminal-recovery proxy keeps the same stage order.
#[test]
#[serial(level2_lifecycle_control)]
fn level2_lifecycle_initialize_precedes_schema_verdict_recovery_proxy() {
    require_level!(Level::L2, TmuxHarness::available(), "tmux");

    assert_initialize_precedes_schema(
        DiagnosticRoute::TerminalRecoveryProxy,
        INIT_REPAIRS_SCHEMA_TARGET,
    );
}

/// **R4 / acceptance criteria 11 and 12, looping document, direct.** Owning a
/// `loop:` must not move the verdict ahead of `initialize`.
///
/// The loop branch threaded no [`claudine::composition::SchemaStage`]: it built
/// its prepare options with `defer_schema_verdict: false` and always reached the
/// verdict on iteration 1, so this exact document failed on a missing `count`
/// while the same document without `loop:` ran.
#[test]
#[serial(level2_lifecycle_control)]
fn level2_lifecycle_initialize_precedes_schema_verdict_loop_direct() {
    require_level!(Level::L2, TmuxHarness::available(), "tmux");

    assert_initialize_precedes_schema(DiagnosticRoute::Direct, INIT_REPAIRS_SCHEMA_LOOP_TARGET);
}

/// **R4 / acceptance criteria 11 and 12, looping document, initialize proxy.**
#[test]
#[serial(level2_lifecycle_control)]
fn level2_lifecycle_initialize_precedes_schema_verdict_loop_initialize_proxy() {
    require_level!(Level::L2, TmuxHarness::available(), "tmux");

    assert_initialize_precedes_schema(
        DiagnosticRoute::InitializeProxy,
        INIT_REPAIRS_SCHEMA_LOOP_TARGET,
    );
}

/// **R4 / acceptance criteria 11 and 12, looping document, recovery proxy.**
#[test]
#[serial(level2_lifecycle_control)]
fn level2_lifecycle_initialize_precedes_schema_verdict_loop_recovery_proxy() {
    require_level!(Level::L2, TmuxHarness::available(), "tmux");

    assert_initialize_precedes_schema(
        DiagnosticRoute::TerminalRecoveryProxy,
        INIT_REPAIRS_SCHEMA_LOOP_TARGET,
    );
}

/// **R4, still-invalid target.** When `initialize` cannot repair the violation,
/// the target has nonetheless run its `initialize` and paid its owed
/// `blocked`/`finalize` before the diagnostic is rendered — and no provider
/// launched for it.
///
/// Ordering is asserted on the event log rather than on the pane because the log
/// is written by the stacks themselves, in the order they fired.
#[test]
#[serial(level2_lifecycle_control)]
fn level2_lifecycle_still_invalid_target_runs_initialize_and_closure_first() {
    require_level!(Level::L2, TmuxHarness::available(), "tmux");

    assert_still_invalid_target_pays_its_closure(INIT_CANNOT_REPAIR_SCHEMA_TARGET);
}

/// **R4, still-invalid looping target.** The converse of the looping rows above:
/// deferring the verdict must not *drop* it. A looping document whose
/// `initialize` cannot repair the violation still runs `initialize`, still pays
/// its owed `blocked`/`finalize`, and still fails with the typed diagnostic — in
/// that order, on every route.
#[test]
#[serial(level2_lifecycle_control)]
fn level2_lifecycle_still_invalid_loop_target_runs_initialize_and_closure_first() {
    require_level!(Level::L2, TmuxHarness::available(), "tmux");

    assert_still_invalid_target_pays_its_closure(INIT_CANNOT_REPAIR_SCHEMA_LOOP_TARGET);
}

fn assert_still_invalid_target_pays_its_closure(target_doc: &str) {
    for route in DiagnosticRoute::ALL {
        let run = run_initialize_ordering_route(route, target_doc);
        let lines = &run.lines;
        let pane = &run.pane;

        let position = |marker: &str| lines.iter().position(|l| l == marker);
        let init = position("target-init").unwrap_or_else(|| {
            panic!(
                "{route:?}: the target's `initialize` must run even when the \
                 document is schema-invalid — the verdict is owed *after* it; \
                 got {lines:?}; pane:\n{pane}"
            )
        });
        let blocked = position("target-blocked").unwrap_or_else(|| {
            panic!(
                "{route:?}: a post-lifecycle preparation failure must route \
                 through the target's own `blocked`; got {lines:?}; pane:\n{pane}"
            )
        });
        let finalize = position("target-finalize").unwrap_or_else(|| {
            panic!(
                "{route:?}: the target must close with `finalize`; got {lines:?}; \
                 pane:\n{pane}"
            )
        });
        assert!(
            init < blocked && blocked < finalize,
            "{route:?}: the owed order is `initialize` → `blocked` → `finalize`; \
             got {lines:?}; pane:\n{pane}"
        );
        assert_eq!(
            lines.iter().filter(|l| **l == "target-init").count(),
            1,
            "{route:?}: `initialize` must still fire exactly once; got {lines:?}; \
             pane:\n{pane}"
        );

        assert!(
            pane.contains("CompositionError: schema validation"),
            "{route:?}: the verdict must still be rendered, and as the typed \
             identity the direct route renders; pane:\n{pane}"
        );
        assert_eq!(
            run.exit_code,
            Some(1),
            "{route:?}: an unrepaired schema violation fails the run; pane:\n{pane}"
        );

        // The target itself never launches: its verdict lands before any
        // attempt. On the recovery route the *router*'s provider ran once, which
        // is what produced the failure that proxied here.
        let expected_launches = usize::from(route == DiagnosticRoute::TerminalRecoveryProxy);
        assert_eq!(
            lines.iter().filter(|l| **l == "provider-ran").count(),
            expected_launches,
            "{route:?}: the schema-invalid target must not reach a provider; \
             got {lines:?}; pane:\n{pane}"
        );
    }
}

// ---------------------------------------------------------------------------
// Direct provider wrappers — review-7 finding 3
// ---------------------------------------------------------------------------

/// A `CLAUDE.md` that activates the wrapper harness (`timeout:` is a harness
/// key, which is what `has_harness_properties` gates on) and then authors every
/// lifecycle surface a composed prompt would, including a `failure`-stack
/// `proxy` to a target that moves each facet the in-harness fallback used to
/// borrow.
const WRAPPER_MEMORY_FILE_WITH_LIFECYCLE: &str = r#"---
timeout: 5m
initialize:
  stack:
    - action: {append_line: ["events.log", "src-initialize"]}
start:
  stack:
    - action: {append_line: ["events.log", "src-start"]}
failure:
  stack:
    - action: {append_line: ["events.log", "src-failure"]}
    - action: {proxy: "@target.md"}
finalize:
  stack:
    - action: {append_line: ["events.log", "src-finalize"]}
---
memory file body
"#;

/// The hand-off target, authored so that adopting it against the *invocation's*
/// launch bundle would be observable on every previously borrowed facet:
/// `agent:` moves profile/binary, `interactive:` moves the argv entrypoint and
/// structured-output mode, and the body `#calendar` tag moves the MCP server set.
const WRAPPER_PROXY_TARGET: &str = r#"---
agent: codex
interactive: true
success:
  stack:
    - action: {append_line: ["events.log", "TARGET-SUCCESS"]}
finalize:
  stack:
    - action: {append_line: ["events.log", "TARGET-FINALIZE"]}
---
target body mentioning #calendar
"#;

/// Stage a repo whose root memory file drives the direct-wrapper harness.
///
/// Unlike [`stage_proxy_pair`] the document under test is **not** passed on the
/// command line: `claudine claude "<prompt>"` discovers `CLAUDE.md` itself via
/// `find_wrapper_harness_source`, which is precisely what makes this the
/// wrapper path rather than a composition command.
fn stage_wrapper_memory_file(memory_file: &str, target_doc: &str) -> Staged {
    let workspace = tempdir().unwrap();
    let bin_dir = workspace.path().join("bin");
    fs::create_dir_all(&bin_dir).unwrap();
    seed_minimal_config(workspace.path());
    assert!(init_git_repo(workspace.path()), "git init failed");

    let events_log = workspace.path().join("events.log");
    // Both providers record under their own name, so "which binary launched" is
    // observable rather than inferred. `claude` fails, which is the only way to
    // reach a `failure`-stack proxy at all.
    write_executable(
        &bin_dir.join("claude"),
        &format!(
            "#!/bin/sh\ncat > /dev/null 2>&1\n\
             printf 'launched-binary=claude\\n' >> {log}\nexit 99\n",
            log = events_log.display(),
        ),
    );
    write_executable(
        &bin_dir.join("codex"),
        &format!(
            "#!/bin/sh\ncat > /dev/null 2>&1\n\
             printf 'launched-binary=codex\\n' >> {log}\nexit 0\n",
            log = events_log.display(),
        ),
    );

    let md_file = workspace.path().join("CLAUDE.md");
    fs::write(&md_file, memory_file).unwrap();
    fs::write(workspace.path().join("target.md"), target_doc).unwrap();

    Staged {
        workspace,
        bin_dir,
        md_file,
        events_log,
        rendezvous_endpoint: None,
    }
}

/// Run `claudine claude --no-interactive "<prompt>"` in a real tmux pane.
///
/// No document argument: the wrapper finds its own harness source. `PATH` is
/// augmented with the fake bin dir and `HOME` is redirected into the workspace,
/// as every other row here does.
fn run_wrapper_in_tmux(staged: &Staged, done_marker: &str) -> String {
    static SEQ: AtomicU32 = AtomicU32::new(0);
    let seq = SEQ.fetch_add(1, Ordering::Relaxed);

    let session = format!("biscuit_l2_lcctl_wrap_{}_{seq}", std::process::id());
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
    let sentinel = format!("L2_CTL_DONE_{seq}");
    let cmd = format!(
        "cd {ws} && NO_COLOR='1' HOME='{home}' PATH='{path}' \
         {claudine} claude --no-interactive 'do the thing' ; echo {sentinel}",
        ws = staged.workspace.path().display(),
        home = staged.workspace.path().display(),
        path = augmented_path(&staged.bin_dir).to_string_lossy(),
    );
    harness
        .send_command_with_env(&cmd, &[])
        .expect("send wrapper command");

    let deadline = Instant::now() + Duration::from_secs(40);
    while Instant::now() < deadline {
        if event_lines(staged).iter().any(|l| l == done_marker) {
            std::thread::sleep(Duration::from_millis(400));
            break;
        }
        std::thread::sleep(Duration::from_millis(100));
    }

    let pane = harness.capture().map(|f| f.plain).unwrap_or_default();
    kill_session_by_name(&session);
    pane
}

/// **Review-7 finding 3 — the direct provider wrappers hand off nothing.**
///
/// The finding asked for wrapper equivalence rows covering the facets the
/// in-harness fallback borrowed (profile/binary, argv entrypoint, MCP runtime
/// injection). No such row can be honest, and this is the row that says why:
/// the wrapper passthrough never installs its memory file's lifecycle
/// configuration at all, so it cannot raise a `proxy` — the borrowed-bundle
/// divergence the finding predicts is **unreachable**, not merely untested.
///
/// The passthrough builds its guard from `LifecycleConfig::default()`
/// (`wrapper_stages.rs::run_execution_stage`) and only the staged proxy
/// bootstrap ever calls `set_config`, which no passthrough run reaches. So a
/// memory file authoring `initialize`/`start`/`failure`/`finalize` gets none of
/// them.
///
/// That is what this row pins, on the shipped binary: the wrapper harness is
/// genuinely engaged (the provider launches through it and the run reports the
/// memory file as its prompt), yet no authored lifecycle marker appears and the
/// target's `codex` — which a borrowed-bundle adoption would have had to
/// launch, since the target authors `agent: codex` — never runs.
///
/// Two consequences this row protects. If someone wires lifecycle into the
/// passthrough without also giving it an owning coordinator, the source markers
/// appear here and this fails, forcing the coordinator decision rather than
/// silently re-opening the reduced launch path. And if the refusal in
/// `surface_or_adopt_terminal_proxy` is ever reached, `TARGET-*` cannot appear
/// without it having been consumed.
#[test]
#[serial(level2_lifecycle_control)]
fn level2_lifecycle_wrapper_passthrough_raises_no_proxy_handoff() {
    require_level!(Level::L2, TmuxHarness::available(), "tmux");

    let staged = stage_wrapper_memory_file(
        WRAPPER_MEMORY_FILE_WITH_LIFECYCLE,
        WRAPPER_PROXY_TARGET,
    );
    let pane = run_wrapper_in_tmux(&staged, "launched-binary=claude");
    let lines = event_lines(&staged);

    // Fixture check: the wrapper harness really did run this memory file. Without
    // this the row would pass vacuously for a wrapper that never engaged at all.
    assert!(
        lines.iter().any(|l| l == "launched-binary=claude"),
        "fixture check: the wrapper must launch its own provider through the \
         harness; got {lines:?}; pane:\n{pane}"
    );
    assert!(
        pane.contains("CLAUDE.md"),
        "fixture check: the run must report the memory file as its harness \
         source; pane:\n{pane}"
    );

    // The behavior under test: no authored lifecycle surface fires, so no
    // `proxy` control can be raised.
    for marker in ["src-initialize", "src-start", "src-failure", "src-finalize"] {
        assert!(
            !lines.iter().any(|l| l == marker),
            "the direct wrapper passthrough installs no lifecycle config, so \
             `{marker}` must not fire; got {lines:?}; pane:\n{pane}"
        );
    }

    // The hand-off's consequences are absent end to end: no target lifecycle,
    // and — the anti-vacuity property — the target's own provider never
    // launched, so no launch bundle was borrowed for it.
    for marker in ["TARGET-SUCCESS", "TARGET-FINALIZE", "launched-binary=codex"] {
        assert!(
            !lines.iter().any(|l| l == marker),
            "no hand-off is raised, so the target must never run: `{marker}` \
             must be absent; got {lines:?}; pane:\n{pane}"
        );
    }
}
