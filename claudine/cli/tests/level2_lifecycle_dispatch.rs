//! Level 2 lifecycle dispatch-order tests (Finding 5, coverage #1).
//!
//! Drives the real `claudine compose --goose <doc>` binary end-to-end inside a
//! real tmux pane (the L2 resource) with a fake provider on `PATH`, and asserts
//! the **externally observable** ordered side-effect log the lifecycle stacks
//! wrote. Each lifecycle event's `stack` ends in
//! `{append_line: ["events.log", "<event-name>"]}`, so the file records the exact
//! firing order without depending on stderr interleaving (which scrolls off a
//! capped pane and collapses under SGR re-emission).
//!
//! Why the file and not the scrollback: `capture()` grabs only the visible
//! pane (no scrollback), and the seven-event sequence with status lines easily
//! exceeds one screenful. The side-effect file is the deterministic, full-
//! history record the spec's "Test Strategy" asks L2 to prove. We still drive a
//! real terminal so the assertion exercises the production CLI process — not an
//! in-process harness — closing the L1-only gap the review flagged.
//!
//! ## What is proven
//!
//! - **success path** (provider exits 0): `events.log` is exactly
//!   `initialize, start, success, finalize`.
//! - **failure path** (provider exits non-zero): exactly
//!   `initialize, start, failure, finalize`.
//! - **blocked path** (pre-flight fails): exactly `initialize, blocked,
//!   finalize` — no `start`, and the provider never ran (its own marker line is
//!   absent).
//!
//! ## Side-effect path resolution
//!
//! `append_line` resolves its first argument against the effect engine's
//! mutation root (`repo_root`, else the launch CWD) — it does **not** strip an
//! `@` prefix (verified in `darkmatter/lib/src/effects/verbs.rs::resolve` /
//! `normalize_path_arg`). The workspace is a git repo, so the mutation root is
//! the workspace root and a plain relative `'events.log'` lands at
//! `<workspace>/events.log`, deterministically readable by the test.
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
use test_toolkit::{Level, require_level};

/// A staged lifecycle run: a git-repo workspace, a fake `goose` provider on
/// `PATH`, the prompt document, and the side-effect log the lifecycle stacks
/// write their ordered event markers to.
struct Staged {
    workspace: tempfile::TempDir,
    bin_dir: std::path::PathBuf,
    md_file: std::path::PathBuf,
    events_log: std::path::PathBuf,
}

/// Write a fake `goose` that exits `exit_code` and appends a `provider-ran`
/// line to `events.log` so a test can assert the provider did (or did not) run.
///
/// The provider drains stdin (`cat > /dev/null`) like the real wrappers expect.
fn write_goose(bin_dir: &Path, events_log: &Path, exit_code: i32) {
    write_executable(
        &bin_dir.join("goose"),
        &format!(
            "#!/bin/sh\ncat > /dev/null\nprintf 'provider-ran\\n' >> {log}\nexit {code}\n",
            log = events_log.display(),
            code = exit_code,
        ),
    );
}

/// Write a fake `goose` that fails on the first run and succeeds on the second,
/// using `attempt_file` as a durable attempt counter.
fn write_goose_with_retry(bin_dir: &Path, events_log: &Path, attempt_file: &Path) {
    write_executable(
        &bin_dir.join("goose"),
        &format!(
            "#!/bin/sh\ncat > /dev/null\nprintf 'provider-ran\\n' >> {log}\nif [ -f {attempt_file} ]; then exit 0; fi\nprintf '1\\n' > {attempt_file}\nexit 99\n",
            log = events_log.display(),
            attempt_file = attempt_file.display(),
        ),
    );
}

/// Stage a workspace with the given prompt frontmatter+body and a fake `goose`
/// provider that exits `provider_exit`.
fn stage(frontmatter_body: &str, provider_exit: i32) -> Staged {
    let workspace = tempdir().unwrap();
    let bin_dir = workspace.path().join("bin");
    fs::create_dir_all(&bin_dir).unwrap();
    seed_minimal_config(workspace.path());
    // A git repo so the effect engine's mutation root resolves to the
    // workspace root and `events.log` lands somewhere the test can read.
    assert!(init_git_repo(workspace.path()), "git init failed");

    let events_log = workspace.path().join("events.log");
    write_goose(&bin_dir, &events_log, provider_exit);

    let md_file = workspace.path().join("doc.md");
    fs::write(&md_file, frontmatter_body).unwrap();

    Staged {
        workspace,
        bin_dir,
        md_file,
        events_log,
    }
}

/// Run `claudine compose --goose <doc>` inside a real tmux pane and wait for the
/// run to finish, then return the captured pane text.
///
/// Completion is detected by polling the side-effect file for the terminal
/// `done_marker` (the last line every lifecycle path writes), not the pane
/// scrollback: a long compose run pushes the chained shell sentinel off the
/// visible pane, and `capture()` has no scrollback. The file is the
/// deterministic synchronization barrier — once it carries the terminal marker
/// the run has emitted every lifecycle event. The caller then asserts on the
/// full ordered file contents.
///
/// `extra_claudine_args` is interpolated between `compose --goose {md}` and
/// the trailing sentinel, so callers can pass flags like `--dry-run` without
/// restating the env/session plumbing.
fn run_compose_in_tmux(staged: &Staged, done_marker: &str) -> String {
    run_compose_in_tmux_with_args(staged, done_marker, "")
}

/// `run_compose_in_tmux` variant that injects extra CLI flags.
fn run_compose_in_tmux_with_args(
    staged: &Staged,
    done_marker: &str,
    extra_claudine_args: &str,
) -> String {
    static SEQ: AtomicU32 = AtomicU32::new(0);
    let seq = SEQ.fetch_add(1, Ordering::Relaxed);

    let session = format!("biscuit_l2_lifecycle_{}_{seq}", std::process::id());
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

    let claudine = env!("CARGO_BIN_EXE_claudine");
    let sentinel = format!("L2_LC_DONE_{seq}");
    let env_pairs: Vec<(&str, String)> = vec![
        ("NO_COLOR", "1".to_string()),
        ("HOME", staged.workspace.path().display().to_string()),
        (
            "PATH",
            augmented_path(&staged.bin_dir)
                .to_string_lossy()
                .into_owned(),
        ),
    ];
    let env_prefix: String = env_pairs
        .iter()
        .map(|(k, v)| format!("{k}='{}' ", v.replace('\'', "'\\''")))
        .collect();
    // Run from the workspace root so a non-git fallback would still land
    // `events.log` there; with the git repo present the mutation root is the
    // same directory either way.
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

    // Poll the side-effect file until the terminal marker lands (run finished)
    // or the deadline elapses (the caller's assertion then reports the partial
    // file + pane).
    let deadline = Instant::now() + Duration::from_secs(30);
    while Instant::now() < deadline {
        if event_lines(staged).iter().any(|l| l == done_marker) {
            // Give the post-terminal `finalize` a brief settle so the whole
            // tail is flushed before we read (finalize fires after the marker
            // on the terminal-event paths, but blocked/finalize order writes
            // the marker last; settle covers both).
            std::thread::sleep(Duration::from_millis(150));
            break;
        }
        std::thread::sleep(Duration::from_millis(100));
    }

    let last_plain = harness.capture().map(|f| f.plain).unwrap_or_default();
    kill_session_by_name(&session);
    last_plain
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

/// A lifecycle event block whose stack appends the event name to `events.log`.
fn marker(event: &str) -> String {
    format!("{event}:\n  stack:\n    - action: {{append_line: [\"events.log\", \"{event}\"]}}\n")
}

/// A lifecycle event block whose stack appends, for each `field` in `fields`,
/// the runtime value of `err.<field>` to `events.log`. Each line lands as
/// `err-<field>=<value>` (e.g. `err-kind=LifecycleAction`), so an assertion
/// can both locate the err-payload line — the `err-<field>=` prefix is
/// unique to this test surface — and read the runtime value the stack
/// expression engine observed.
///
/// The literal prefix and `err.<field>` are joined with `+`, which
/// Darkmatter treats as string concatenation when either operand is a
/// string. `append_line` itself is strictly 2-arg (`append_line: [file, text]`),
/// so the 3-arg form would not parse.
fn err_marker(event: &str, fields: &[&str]) -> String {
    let actions: String = fields
        .iter()
        .map(|f| {
            format!("    - action: {{append_line: [\"events.log\", \"{{{{ 'err-{f}=' + err.{f} }}}}\"]}}\n")
        })
        .collect();
    format!("{event}:\n  stack:\n{actions}")
}

/// Whether the captured pane carries the success top-level `info` line text.
///
/// `info` renders through a `Status` component, so the captured (NO_COLOR)
/// plain text contains the message body verbatim even though the surrounding
/// glyphs/styling vary. We assert on the body substring, not the full line.
fn pane_has(pane: &str, needle: &str) -> bool {
    pane.contains(needle)
}

/// Success path: a document whose `initialize`/`start`/`success`/`finalize`
/// stacks each append their event name fires them in exactly that order when
/// the provider exits 0. `blocked`/`failure` markers are present in the
/// document but must NOT fire.
#[test]
#[serial(level2_lifecycle)]
fn level2_lifecycle_success_path_event_order() {
    require_level!(Level::L2, TmuxHarness::available(), "tmux");

    let doc = format!(
        "---\ntitle: lifecycle success\n{init}{start}{success}{blocked}{failure}{finalize}---\nBody\n",
        init = marker("initialize"),
        start = marker("start"),
        success = marker("success"),
        blocked = marker("blocked"),
        failure = marker("failure"),
        finalize = marker("finalize"),
    );
    let staged = stage(&doc, 0);
    let pane = run_compose_in_tmux(&staged, "finalize");

    let lines = event_lines(&staged);
    // The provider runs and writes its own marker; drop it so the comparison
    // is over the lifecycle events only (their relative order is preserved).
    let lifecycle: Vec<&String> = lines.iter().filter(|l| *l != "provider-ran").collect();
    assert_eq!(
        lifecycle,
        vec!["initialize", "start", "success", "finalize"],
        "success path must fire exactly initialize→start→success→finalize; \
         events.log was {lines:?}; pane:\n{pane}"
    );
    assert!(
        !lifecycle.iter().any(|l| **l == "blocked" || **l == "failure"),
        "neither blocked nor failure may fire on the success path; got {lines:?}"
    );
}

/// Failure path: the same document with a provider that exits non-zero fires
/// `initialize`→`start`→`failure`→`finalize` (no `success`, no `blocked`).
#[test]
#[serial(level2_lifecycle)]
fn level2_lifecycle_failure_path_event_order() {
    require_level!(Level::L2, TmuxHarness::available(), "tmux");

    let doc = format!(
        "---\ntitle: lifecycle failure\n{init}{start}{success}{blocked}{failure}{finalize}---\nBody\n",
        init = marker("initialize"),
        start = marker("start"),
        success = marker("success"),
        blocked = marker("blocked"),
        failure = marker("failure"),
        finalize = marker("finalize"),
    );
    // Provider exits 99 → AgentFailure → routes to the `failure` event.
    let staged = stage(&doc, 99);
    let pane = run_compose_in_tmux(&staged, "finalize");

    let lines = event_lines(&staged);
    // The provider runs on this path, so drop its marker before comparing the
    // lifecycle ordering.
    let lifecycle: Vec<&String> = lines.iter().filter(|l| *l != "provider-ran").collect();
    assert_eq!(
        lifecycle,
        vec!["initialize", "start", "failure", "finalize"],
        "failure path must fire exactly initialize→start→failure→finalize; \
         events.log was {lines:?}; pane:\n{pane}"
    );
    assert!(
        !lifecycle.iter().any(|l| **l == "success" || **l == "blocked"),
        "neither success nor blocked may fire on the failure path; got {lines:?}"
    );
}

/// Composition-preflight blocked path: a `start.stack` carrying a shell
/// command that the **builtin blacklist** rejects (here `rm`) fails shell
/// audit during composition preflight — the earlier preflight phase the
/// spec calls out (Finding 1 of review-3). The fix routes this failure
/// through the stack-aware event runner, so `blocked.stack` AND
/// `finalize.stack` must both fire even though `start` never does.
///
/// Before the fix the preflight failure path called
/// `guard.emit_blocked_or_failure()`, which only emits the legacy top-level
/// communication surface and never executes the typed stacks or `finalize`.
/// `events.log` is exactly `["initialize", "blocked", "finalize"]`, proving
/// BOTH the blocked stack AND the finalize stack fired (review-3 acceptance
/// criterion).
///
/// The blacklist path is used (not the interactive-prompt path) so the test
/// is deterministic across hosts: `rm` is rejected by `check_builtin_blacklist`
/// before the interactive approval handler is consulted, so the absence or
/// presence of a TTY does not change the outcome.
#[test]
#[serial(level2_lifecycle)]
fn level2_lifecycle_blocked_preflight_shell_audit_fires_blocked_and_finalize_stacks() {
    require_level!(Level::L2, TmuxHarness::available(), "tmux");

    // `start.stack` carries a blacklisted shell command, so composition
    // preflight shell-audit denies it and routes the run to `blocked` —
    // before `start` itself fires. `blocked.stack` and `finalize.stack`
    // each append their event name to `events.log`, proving the typed
    // stacks fire (not just the legacy top-level communication surface).
    let start = "start:\n  stack:\n    - action: {shell: \"rm -rf /tmp/nonexistent\"}\n";
    let doc = format!(
        "---\ntitle: lifecycle preflight blocked (shell audit)\n\
         {init}{start}{success}{blocked}{failure}{finalize}---\nBody\n",
        init = marker("initialize"),
        start = start,
        success = marker("success"),
        blocked = marker("blocked"),
        failure = marker("failure"),
        finalize = marker("finalize"),
    );
    let staged = stage(&doc, 0);
    let pane = run_compose_in_tmux(&staged, "finalize");

    let lines = event_lines(&staged);
    assert_eq!(
        lines,
        vec!["initialize", "blocked", "finalize"],
        "composition-preflight blocked path must fire exactly \
         initialize→blocked→finalize; events.log was {lines:?}; pane:\n{pane}"
    );
    assert!(
        !lines.iter().any(|l| l == "start"),
        "start must NOT fire when composition preflight blocks the run; \
         got {lines:?}"
    );
    assert!(
        !lines.iter().any(|l| l == "provider-ran"),
        "the provider must never run on the composition-preflight blocked \
         path; got {lines:?}"
    );
}

/// Composition-preflight dry-run blocked path: a `start.stack` carrying a
/// blacklisted shell command fails shell audit during composition preflight.
/// Before Finding 1's fix this failure path called
/// `guard.emit_blocked_or_failure()`, skipping the typed `blocked.stack` and
/// `finalize.stack`. This test proves the dry-run shell-audit blocked path
/// records both markers in `events.log`.
///
/// `rm` is rejected by `check_builtin_blacklist` before the interactive
/// approval handler is consulted, so the outcome is deterministic across
/// hosts regardless of TTY availability.
#[test]
#[serial(level2_lifecycle)]
fn level2_lifecycle_blocked_preflight_dry_run_shell_audit_fires_blocked_and_finalize_stacks() {
    require_level!(Level::L2, TmuxHarness::available(), "tmux");

    let start = "start:\n  stack:\n    - action: {shell: \"rm -rf /tmp/nonexistent\"}\n";
    let doc = format!(
        "---\ntitle: lifecycle preflight blocked (dry-run shell audit)\n\
         {init}{start}{success}{blocked}{failure}{finalize}---\nBody\n",
        init = marker("initialize"),
        start = start,
        success = marker("success"),
        blocked = marker("blocked"),
        failure = marker("failure"),
        finalize = marker("finalize"),
    );
    let staged = stage(&doc, 0);
    let pane = run_compose_in_tmux_with_args(&staged, "finalize", "--dry-run");

    let lines = event_lines(&staged);
    assert_eq!(
        lines,
        vec!["initialize", "blocked", "finalize"],
        "composition-preflight dry-run blocked path must fire exactly \
         initialize→blocked→finalize; events.log was {lines:?}; pane:\n{pane}"
    );
    assert!(
        !lines.iter().any(|l| l == "start"),
        "start must NOT fire when the dry-run shell audit blocks the run; \
         got {lines:?}"
    );
    assert!(
        !lines.iter().any(|l| l == "provider-ran"),
        "the provider must never run on the dry-run blocked path; got {lines:?}"
    );
}

/// Top-level-before-stack ordering for `success`: a `success` block carrying
/// BOTH a top-level `info:` communication AND a `stack:` side effect fires the
/// top-level before the stack. The top-level `info` line is captured in the
/// pane and the stack marker is recorded in `events.log`; both must be present
/// (the comm is not dropped when a stack is added) and the file order remains
/// `initialize→start→success→finalize`.
#[test]
#[serial(level2_lifecycle)]
fn level2_lifecycle_success_top_level_fires_with_stack() {
    require_level!(Level::L2, TmuxHarness::available(), "tmux");

    // `success` carries a top-level `info:` AND a stack side effect.
    let success = "success:\n  info: \"SUCCESS_TOPLEVEL_MARK\"\n  \
        stack:\n    - action: {append_line: [\"events.log\", \"success\"]}\n";
    let doc = format!(
        "---\ntitle: lifecycle success top-level\n{init}{start}{success}{finalize}---\nBody\n",
        init = marker("initialize"),
        start = marker("start"),
        success = success,
        finalize = marker("finalize"),
    );
    let staged = stage(&doc, 0);
    let pane = run_compose_in_tmux(&staged, "finalize");

    let lines = event_lines(&staged);
    let lifecycle: Vec<&String> = lines.iter().filter(|l| *l != "provider-ran").collect();
    assert_eq!(
        lifecycle,
        vec!["initialize", "start", "success", "finalize"],
        "event order must stay initialize→start→success→finalize; \
         events.log was {lines:?}; pane:\n{pane}"
    );
    // The top-level `info` communication fired (not dropped because a stack
    // was added) — it is visible in the pane.
    assert!(
        pane_has(&pane, "SUCCESS_TOPLEVEL_MARK"),
        "success top-level `info` must fire when the event also has a stack; \
         pane:\n{pane}"
    );
}

/// Downgrade path for `success.stack` ending in `{error: "..."}`: the top-level
/// `info:` communication STILL fires (top-level fires before stack processing,
/// per the spec), then the stack's `{error: "..."}` downgrades the run to `failure` —
/// the `failure` event fires and `finalize` fires. Proven by the ordered
/// `events.log` (`success`-stack → `failure`-stack → `finalize`) plus the
/// success top-level line captured in the pane.
#[test]
#[serial(level2_lifecycle)]
fn level2_lifecycle_success_stack_error_downgrades_keeps_success_comm() {
    require_level!(Level::L2, TmuxHarness::available(), "tmux");

    // `success` has a top-level `info:` and a stack whose final action is
    // `{error: "bad"}`, downgrading the run to `failure`.
    let success = "success:\n  info: \"SUCCESS_TOPLEVEL_MARK\"\n  stack:\n    \
        - action:\n        - {append_line: [\"events.log\", \"success\"]}\n        \
        - {error: \"bad\"}\n";
    let doc = format!(
        "---\ntitle: lifecycle success downgrade\n{init}{start}{success}{failure}{finalize}---\nBody\n",
        init = marker("initialize"),
        start = marker("start"),
        success = success,
        failure = marker("failure"),
        finalize = marker("finalize"),
    );
    // Provider exits 0 so the run reaches `success`; the stack's `{error: "bad"}` then
    // downgrades it to `failure`.
    let staged = stage(&doc, 0);
    let pane = run_compose_in_tmux(&staged, "finalize");

    let lines = event_lines(&staged);
    let lifecycle: Vec<&String> = lines.iter().filter(|l| *l != "provider-ran").collect();
    // The success stack ran once, then the downgrade fired the failure stack,
    // then finalize fired.
    assert_eq!(
        lifecycle,
        vec!["initialize", "start", "success", "failure", "finalize"],
        "success.stack {{error: \"bad\"}} must downgrade to failure then finalize; \
         events.log was {lines:?}; pane:\n{pane}"
    );
    // The success top-level `info` fired BEFORE the stack's downgrade and is
    // NOT suppressed by the later `error()`.
    assert!(
        pane_has(&pane, "SUCCESS_TOPLEVEL_MARK"),
        "success top-level `info` must still fire even when the stack downgrades \
         via error(); pane:\n{pane}"
    );
}

/// `failure.stack` observes the runtime `err` payload (review-4 critical
/// finding). A provider exiting non-zero (99) routes through
/// `LifecycleErrorInfo::from_action_failure("agent_failure", msg)` at
/// `loop_control.rs:1772-1775`, so `err.kind`/`err.variant`/`err.msg` must
/// all reach the stack expression engine. Each is appended to `events.log`
/// as `err-<field>=<value>`.
///
/// The static parse-time scan only proves `err` *placement*; this test
/// proves the user-observable event-time behavior. Before the production
/// fix the runtime path executed `failure` with `err: None`, so every
/// `err.*` reference resolved to `null` and the `err-<field>=` lines would
/// have read `err-kind=` / `err-variant=` / `err-msg=` (empty values).
#[test]
#[serial(level2_lifecycle)]
fn level2_lifecycle_failure_stack_observes_err_payload() {
    require_level!(Level::L2, TmuxHarness::available(), "tmux");

    let failure = err_marker("failure", &["kind", "variant", "msg"]);
    let doc = format!(
        "---\ntitle: lifecycle failure err payload\n\
         {init}{start}{success}{blocked}{failure}{finalize}---\nBody\n",
        init = marker("initialize"),
        start = marker("start"),
        success = marker("success"),
        blocked = marker("blocked"),
        failure = failure,
        finalize = marker("finalize"),
    );
    // Provider exits 99 → AgentFailure → `from_action_failure("agent_failure", msg)`.
    let staged = stage(&doc, 99);
    let pane = run_compose_in_tmux(&staged, "finalize");

    let lines = event_lines(&staged);
    // `agent_failure` is a classifiable error_kind (→ `provider.exited`), so the
    // deprecated `err.kind`/`err.variant` aliases now read as the faceted
    // `err.category`/`err.code` values (`provider` / `provider.exited`), not the
    // internal `LifecycleAction` / `agent_failure` labels.
    assert!(
        lines.iter().any(|l| l == "err-kind=provider"),
        "err.kind must reach the failure stack as 'provider' (alias of err.category); \
         events.log was {lines:?}; pane:\n{pane}"
    );
    assert!(
        lines.iter().any(|l| l == "err-variant=provider.exited"),
        "err.variant must reach the failure stack as 'provider.exited' \
         (alias of err.code; outcome.error_kind defaults to 'agent_failure' for a \
         plain non-zero exit, which classifies to provider.exited); \
         events.log was {lines:?}; pane:\n{pane}"
    );
    assert!(
        lines
            .iter()
            .any(|l| l.starts_with("err-msg=") && l.contains("agent exited with error code 99")),
        "err.msg must reach the failure stack and include the provider exit code \
         (fallback message is `agent exited with error code 99`, no attempt \
         suffix at attempt 1); \
         events.log was {lines:?}; pane:\n{pane}"
    );
}

/// `blocked.stack` observes the runtime `err` payload (review-4 critical
/// finding). A blacklisted shell command in `start.stack` fails shell audit
/// during composition preflight and routes through the blocked path, so
/// `err.kind`/`err.variant` must reach the stack expression engine.
///
/// The provider is never invoked on the blocked path, so `provider-ran`
/// stays absent.
///
/// ## The migrated values
///
/// This site used to call
/// `LifecycleErrorInfo::from_action_failure("shell_approval", fail_msg)`, which
/// synthesized the facet-less labels `err.kind = "LifecycleAction"` and
/// `err.variant = "shell_approval"`. Error-propagation Phase 5 (§D7) pointed it
/// at the typed pre-flight error instead, and `err.kind` / `err.variant` are now
/// **deprecated aliases** of `err.category` / `err.code`.
///
/// Phase 5 landed that as `composition` / `composition.failed`, which was a loss
/// of specificity — a shell denial became indistinguishable from any other
/// uncoded composition failure. Phase 8 (§D-14) gave the approval family a typed
/// variant and the `composition.shell_approval` code, so the distinction the
/// old `shell_approval` label carried is back as a faceted value, and
/// `err.detail.reason` (`blacklisted` here) says which approval failure it was.
///
/// What this test proves is unchanged and is the reason it exists: the payload
/// **reaches** the blocked stack's expression engine at all.
#[test]
#[serial(level2_lifecycle)]
fn level2_lifecycle_blocked_stack_observes_err_payload() {
    require_level!(Level::L2, TmuxHarness::available(), "tmux");

    let start = "start:\n  stack:\n    - action: {shell: \"rm -rf /tmp/nonexistent\"}\n";
    let blocked = err_marker("blocked", &["kind", "variant"]);
    let doc = format!(
        "---\ntitle: lifecycle blocked err payload\n\
         {init}{start}{success}{blocked}{failure}{finalize}---\nBody\n",
        init = marker("initialize"),
        start = start,
        success = marker("success"),
        blocked = blocked,
        failure = marker("failure"),
        finalize = marker("finalize"),
    );
    // Exit code is irrelevant — shell audit blocks before the provider runs.
    let staged = stage(&doc, 0);
    let pane = run_compose_in_tmux(&staged, "finalize");

    let lines = event_lines(&staged);
    assert!(
        lines.iter().any(|l| l == "err-kind=composition"),
        "err.kind must reach the blocked stack as the typed error's category \
         ('composition') — it is a deprecated alias of err.category since the \
         error-propagation migration; events.log was {lines:?}; pane:\n{pane}"
    );
    assert!(
        lines.iter().any(|l| l == "err-variant=composition.shell_approval"),
        "err.variant must reach the blocked stack as the typed error's code \
         ('composition.shell_approval') — it is a deprecated alias of err.code since \
         the error-propagation migration; events.log was {lines:?}; pane:\n{pane}"
    );
    assert!(
        !lines.iter().any(|l| l == "provider-ran"),
        "the provider must never run on the blocked path; got {lines:?}"
    );
}

/// Failed-`finalize` observes the runtime `err` payload on the **blocked**
/// route (review-4 critical finding — the explicitly-named blocked-then-
/// finalize case). A blacklisted shell command in `start.stack` fails shell
/// audit during composition preflight and routes to `blocked` then `finalize`
/// with the same `err_info` attached, so the `when: "err"` guard on the
/// `finalize.stack` must be truthy and observe `err.variant` carrying the
/// typed error's code (see the alias and `composition.shell_approval` notes on
/// `level2_lifecycle_blocked_stack_observes_err_payload`).
///
/// This complements `level2_lifecycle_finalize_stack_observes_err_after_failure`
/// (which covers the provider-failure route): together they prove the failed-
/// finalize `err` payload reaches the stack on **both** the blocked and the
/// failure routes, not just one. Before the production fix the blocked-route
/// `finalize` carried `err: None`, so `when: "err"` resolved to `null` (falsy)
/// and the guarded append would never land.
#[test]
#[serial(level2_lifecycle)]
fn level2_lifecycle_finalize_stack_observes_err_after_blocked() {
    require_level!(Level::L2, TmuxHarness::available(), "tmux");

    let start = "start:\n  stack:\n    - action: {shell: \"rm -rf /tmp/nonexistent\"}\n";
    // `finalize.stack` guards an `err.variant` append with `when: "err"`, then
    // appends the trailing `finalize` marker the polling loop awaits.
    let finalize = "finalize:\n  stack:\n    \
        - when: \"err\"\n      action: {append_line: [\"events.log\", \"{{ 'finalize-err-variant=' + err.variant }}\"]}\n    \
        - action: {append_line: [\"events.log\", \"finalize\"]}\n";
    let doc = format!(
        "---\ntitle: lifecycle finalize err after blocked\n\
         {init}{start}{success}{blocked}{failure}{finalize}---\nBody\n",
        init = marker("initialize"),
        start = start,
        success = marker("success"),
        blocked = marker("blocked"),
        failure = marker("failure"),
        finalize = finalize,
    );
    // Exit code is irrelevant — shell audit blocks before the provider runs.
    let staged = stage(&doc, 0);
    let pane = run_compose_in_tmux(&staged, "finalize");

    let lines = event_lines(&staged);
    assert!(
        lines
            .iter()
            .any(|l| l == "finalize-err-variant=composition.shell_approval"),
        "err.variant must reach the failed-finalize stack on the blocked route — \
         `when: 'err'` must be truthy after a shell-audit block; \
         events.log was {lines:?}; pane:\n{pane}"
    );
    // The blocked event fired (carrying err into finalize), start did not.
    assert!(
        lines.iter().any(|l| l == "blocked"),
        "the blocked event must have fired before finalize (carrying err); \
         events.log was {lines:?}; pane:\n{pane}"
    );
    assert!(
        !lines.iter().any(|l| l == "start"),
        "start must NOT fire when pre-flight blocks the run; got {lines:?}"
    );
    // The finalize stack ran to completion (the trailing marker landed).
    assert!(
        lines.iter().any(|l| l == "finalize"),
        "the finalize event must have fired (the trailing marker); \
         events.log was {lines:?}; pane:\n{pane}"
    );
}

/// Failed-`finalize` observes the runtime `err` payload (review-4 critical
/// finding). When a provider exits non-zero, `failure` fires first then
/// `finalize` fires with the same `err_info` attached
/// (`loop_control.rs:1826-1836`). The `when: "err"` guard on a
/// `finalize.stack` action must evaluate truthy — a non-empty JSON object
/// is truthy per Darkmatter's rule — so the action fires and observes
/// `err.msg`.
///
/// Before the production fix the failed-`finalize` runtime path carried
/// `err: None`, so `when: "err"` resolved to `null` (falsy), the action
/// was silently skipped, and no `finalize-err-msg=` line would land in
/// `events.log`. The trailing `{append_line: ["events.log", "finalize"]}`
/// action is the done-marker the polling loop awaits and also proves the
/// stack continued past the `when: "err"` item.
#[test]
#[serial(level2_lifecycle)]
fn level2_lifecycle_finalize_stack_observes_err_after_failure() {
    require_level!(Level::L2, TmuxHarness::available(), "tmux");

    // `finalize.stack` guards an err.msg append with `when: "err"`, then
    // appends a trailing `finalize` marker the polling loop awaits.
    let finalize = "finalize:\n  stack:\n    \
        - when: \"err\"\n      action: {append_line: [\"events.log\", \"{{ 'finalize-err-msg=' + err.msg }}\"]}\n    \
        - action: {append_line: [\"events.log\", \"finalize\"]}\n";
    let doc = format!(
        "---\ntitle: lifecycle finalize err payload\n\
         {init}{start}{success}{blocked}{failure}{finalize}---\nBody\n",
        init = marker("initialize"),
        start = marker("start"),
        success = marker("success"),
        blocked = marker("blocked"),
        failure = marker("failure"),
        finalize = finalize,
    );
    // Provider exits 99 → AgentFailure → failure → finalize (with err_info).
    let staged = stage(&doc, 99);
    let pane = run_compose_in_tmux(&staged, "finalize");

    let lines = event_lines(&staged);
    assert!(
        lines
            .iter()
            .any(|l| l.starts_with("finalize-err-msg=") && l.contains("agent exited with error code 99")),
        "err.msg must reach the failed-finalize stack — `when: 'err'` must be \
         truthy on a failure path; events.log was {lines:?}; pane:\n{pane}"
    );
    // The failure event fired (otherwise finalize would carry no `err`).
    assert!(
        lines.iter().any(|l| l == "failure"),
        "the failure event must have fired before finalize (carrying err); \
         events.log was {lines:?}; pane:\n{pane}"
    );
    // The finalize stack ran to completion (the trailing marker landed).
    assert!(
        lines.iter().any(|l| l == "finalize"),
        "the finalize event must have fired (the trailing marker); \
         events.log was {lines:?}; pane:\n{pane}"
    );
}

/// Lifecycle `failure` recovery: a `failure.stack` ending in `{retry: 2}` re-enters
/// the harness loop after the first provider failure. The provider fails on its
/// first invocation (exit 99) and succeeds on the second (exit 0), proving the
/// lifecycle recovery action replaced the removed handler DSL retry path.
#[test]
#[serial(level2_lifecycle)]
fn level2_lifecycle_failure_retry_recovers_to_success() {
    require_level!(Level::L2, TmuxHarness::available(), "tmux");

    let failure = "failure:\n  stack:\n    \
        - action: {append_line: [\"events.log\", \"failure\"]}\n    \
        - action: {retry: 2}\n";
    let doc = format!(
        "---\ntitle: lifecycle failure retry\n\
         {init}{start}{success}{failure}{finalize}---\nBody\n",
        init = marker("initialize"),
        start = marker("start"),
        success = marker("success"),
        failure = failure,
        finalize = marker("finalize"),
    );

    let workspace = tempdir().unwrap();
    let bin_dir = workspace.path().join("bin");
    fs::create_dir_all(&bin_dir).unwrap();
    seed_minimal_config(workspace.path());
    assert!(init_git_repo(workspace.path()), "git init failed");

    let events_log = workspace.path().join("events.log");
    let attempt_file = workspace.path().join("attempt.count");
    write_goose_with_retry(
        &bin_dir,
        &events_log,
        &attempt_file,
    );

    let md_file = workspace.path().join("doc.md");
    fs::write(&md_file, &doc).unwrap();

    let staged = Staged {
        workspace,
        bin_dir,
        md_file,
        events_log,
    };
    let pane = run_compose_in_tmux(&staged, "finalize");

    let lines = event_lines(&staged);
    let lifecycle: Vec<&String> = lines.iter().filter(|l| *l != "provider-ran").collect();
    assert_eq!(
        lifecycle,
        vec!["initialize", "start", "failure", "start", "success", "finalize"],
        "failure retry must re-enter the loop and reach success; \
         events.log was {lines:?}; pane:\n{pane}"
    );
    assert_eq!(
        lines.iter().filter(|l| *l == "provider-ran").count(),
        2,
        "provider must run twice (failed attempt + retry); got {lines:?}"
    );
}
