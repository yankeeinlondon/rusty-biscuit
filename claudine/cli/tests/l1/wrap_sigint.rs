//! Integration tests: SIGINT-during-prep exit-code behavior.
//!
//! Split out of the `wrap_commands.rs` god file; shared fixtures live in
//! `common::wrap`.

use crate::common;
use common::{CliProcessFixture, write, write_executable};
#[cfg(unix)]
use common::signal::{
    COMPOSE_INTERRUPT_NOTICE, GRACE_ARMED_NOTICE, GRACE_EXPIRED_NOTICE, ShellBarrier,
    SignalledRun, TERMINAL_LIFECYCLE_EXIT_GRACE,
};

/// Bound on reaching a fixture's readiness point: wrapper startup, prep, and
/// the provider attempt, under full-suite contention.
#[cfg(unix)]
const READY_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(20);

/// Bound on a signal handler's stderr acknowledgement of a press.
#[cfg(unix)]
const ACK_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(5);

/// Bound on the wrapper exiting once a test expects it to.
#[cfg(unix)]
const EXIT_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(15);

/// Slack past a deadline for the fixture's 5 ms polls and for the wrapper or
/// the test being descheduled under full-suite contention. It is far below
/// what the deadline guards against: a lifecycle held by its barrier never
/// finishes on its own, so a missing or re-armed deadline blows
/// [`EXIT_TIMEOUT`].
#[cfg(unix)]
const SCHEDULING_ALLOWANCE: std::time::Duration = std::time::Duration::from_millis(2500);

// Was `slow_compose_sigint_during_prep_exits_130_with_notice`. The `slow_`
// prefix is a tier marker, and `_tier_filter L1` excludes it from `just test`,
// `just test-cli` and every CI leg — inclusion needs `l1-include-slow`, which
// only darkmatter's packages declare. So the SIGINT-during-prep contract ran in
// no recipe at all. Nothing about the test is slow: the *fake provider* sleeps
// for 10 s, which is the floor the 4 s latency assertion is measured against,
// and the test itself completes in 0.18 s because interrupting that sleep is
// the whole point.
#[cfg(unix)]
#[test]
#[serial_test::serial]
fn compose_sigint_during_prep_exits_130_with_notice() {
    let fixture = CliProcessFixture::named("wrap-sigint");
    fixture.seed_user_config();

    // Frontmatter `model` hint is required so the catalog refresh gate
    // (`refresh_for_model_validation`) actually invokes the dynamic source
    // for OpenCode. Without it, `hints.model.is_none()` short-circuits the
    // refresh and the slow `opencode models` subprocess never runs, leaving
    // this test's interrupt window non-deterministic.
    //
    // The hint must be *valid*: frontmatter models are validated against
    // the baseline catalog (`resolve_model_with_env` step 4) and an invalid
    // hint falls through to the provider default (`None`), which fails
    // OpenCode's non-TTY model requirement with exit 1 before the interrupt
    // path is reached. `llamacpp/…` matches via `offering_sources` prefix,
    // so validity is structural and immune to baseline offering churn.
    let md_file = fixture.cwd().join("slow.md");
    write(
        &md_file,
        "---\ntitle: test\nmodel: llamacpp/test-model\n---\nPrompt body\n",
    );

    // Fake `opencode models` touches a readiness marker, then sleeps for
    // 10s so prep is slow enough to interrupt, and so a regression to the
    // uncancellable blocking path would clearly exceed the 4s
    // interrupt-to-exit budget below (it would have to wait ~9s for the
    // sleep to finish). The marker is the test's synchronization barrier:
    // it is written only once the model-validation refresh has reached the
    // `opencode models` subprocess, which happens *after* the SIGINT handler
    // is installed at the top of `compose`. The `opencode` provider binary
    // itself never runs.
    let ready_marker = fixture.cwd().join("opencode-models-started");
    write_executable(
        &fixture.bin_dir().join("opencode"),
        r#"#!/bin/sh
if [ "$1" = "models" ]; then
  : > "$CLAUDINE_READY_MARKER"
  /bin/sleep 10
  printf '%s\n' '["test-model"]'
  exit 0
fi
exit 0
"#,
    );

    // `CLAUDINE_BACKGROUND_REFRESH=0` forces the caller-blocking refresh
    // path (the documented escape hatch). The default W3 refresh is
    // detached, so prep would race ahead of the `opencode models`
    // subprocess and this test's interrupt window would not exist; the
    // blocking path is the one whose SIGINT cancellability is under test.
    //
    // The child has to stay alive to receive the signal, so this is the
    // builder's raw-command surface; the call site keeps only its subject —
    // signal delivery, output draining, and reaping.
    let mut command = fixture.command_std();
    command
        .env("CLAUDINE_READY_MARKER", &ready_marker)
        .env("CLAUDINE_BACKGROUND_REFRESH", "0")
        .args(["compose", "--opencode", md_file.to_str().unwrap()]);
    let mut run = SignalledRun::spawn(command, fixture.workspace_path());

    // The marker proves the child reached the cancellable `opencode models`
    // refresh, which is strictly after the SIGINT handler is installed. A
    // fixed sleep was flaky under full-suite contention — slow wrapper
    // startup could push handler installation past the deadline, so SIGINT
    // hit the default disposition (exit 130 but no clean notice).
    run.wait_for_path(&ready_marker, READY_TIMEOUT);
    let interrupt_sent_at = run.interrupt();

    let exit = run.wait_for_exit(EXIT_TIMEOUT);
    let interrupt_to_exit = exit.exited_at - interrupt_sent_at;

    // Exit code 130 = 128 + SIGINT(2)
    assert_eq!(
        exit.status.code(),
        Some(130),
        "SIGINT during prep must yield exit code 130; stderr:\n{}",
        exit.stderr
    );

    // Bounded interrupt latency: the cancellable fetch (`kill_on_drop`
    // plus the interrupt-flag race in `fetch_shell_command_models`)
    // returns within ~50 ms of the interrupt under normal conditions. The
    // fake `opencode models` sleeps for 10s — anywhere near that means
    // the fetch stopped responding to the interrupt flag. A 4-second
    // ceiling sits comfortably below the 10s blocking floor while leaving
    // headroom for OS scheduling under contention.
    assert!(
        interrupt_to_exit < std::time::Duration::from_secs(4),
        "SIGINT-to-exit latency exceeded 4s ({:?}); blocked-prep regression",
        interrupt_to_exit,
    );

    assert!(
        exit.stderr.contains(COMPOSE_INTERRUPT_NOTICE),
        "stderr must contain the clean interrupt notice; got: {}",
        exit.stderr
    );
}

/// Ctrl+C while the wrapper tears down an agent's orphaned descendants still
/// runs the document's `failure` and `finalize` events, promptly.
///
/// The shape is an OpenCode run that exits but leaves a process holding its
/// stdout/stderr pipes. The fake's orphan survives `SIGTERM` (its trap touches
/// the marker this test waits on), so the wrapper sits in the post-exit
/// process-group teardown when the press lands. The kill grace is stretched to
/// 60 s so a teardown that ignores the interrupt blows the latency budget.
///
/// One press only: it ends the teardown within one poll, so a second press
/// cannot reliably land inside it from here. That the teardown defers a repeat
/// press is `kill_process_group_defers_repeat_interrupts_until_ctrl_c_ends_it`
/// in `commands::wrap::exec`.
#[cfg(unix)]
#[test]
#[serial_test::serial]
fn compose_sigint_during_orphan_teardown_runs_failure_lifecycle() {
    let fixture = CliProcessFixture::named("wrap-sigint-orphan");
    let orphan = OrphanTeardownFixture::new(&fixture, OrphanOnTerm::Survive);
    let mut run = orphan.spawn();

    // The orphan's `SIGTERM` trap writes this, so the press below lands inside
    // the teardown.
    run.wait_for_path(&orphan.term_marker, READY_TIMEOUT);
    let interrupt_sent_at = run.interrupt();

    let exit = run.wait_for_exit(EXIT_TIMEOUT);
    let interrupt_to_exit = exit.exited_at - interrupt_sent_at;
    let plain = &exit.stderr;

    assert!(
        interrupt_to_exit < std::time::Duration::from_secs(10),
        "Ctrl+C must cut the 60s kill grace short; took {interrupt_to_exit:?}; stderr:\n{plain}"
    );
    assert!(
        !exit.status.success(),
        "a failed, interrupted run must not exit 0; stderr:\n{plain}"
    );
    assert!(
        !plain.contains("force-exiting"),
        "the teardown must not force-exit the wrapper; stderr:\n{plain}"
    );
    assert_eq!(
        orphan.events(),
        vec!["failure", "finalize"],
        "the interrupted run must still fire its terminal lifecycle; stderr:\n{plain}"
    );
}

/// Once `SIGTERM` has emptied the provider's process group, the post-exit
/// teardown returns at its next poll instead of sleeping out the kill grace —
/// no Ctrl+C involved.
///
/// The orphan's trap touches its marker and exits. With the grace stretched to
/// 60 s, a teardown that waited it out would blow [`EXIT_TIMEOUT`] and the
/// bound below by a wide margin, yet the bound leaves seconds of scheduling
/// slack over the 50 ms poll.
#[cfg(unix)]
#[test]
#[serial_test::serial]
fn compose_orphan_teardown_ends_once_the_group_exits_without_an_interrupt() {
    let fixture = CliProcessFixture::named("wrap-sigint-orphan-exits");
    let orphan = OrphanTeardownFixture::new(&fixture, OrphanOnTerm::Exit);
    let mut run = orphan.spawn();

    run.wait_for_path(&orphan.term_marker, READY_TIMEOUT);
    let term_observed_at = std::time::Instant::now();

    let exit = run.wait_for_exit(EXIT_TIMEOUT);
    let term_to_exit = exit.exited_at.saturating_duration_since(term_observed_at);
    let plain = &exit.stderr;

    assert!(
        term_to_exit < std::time::Duration::from_secs(5),
        "the teardown must end once the group is empty, not after the 60s kill grace; \
         took {term_to_exit:?}; stderr:\n{plain}"
    );
    assert!(
        !exit.status.success(),
        "a failed run must not exit 0; stderr:\n{plain}"
    );
    assert!(
        !plain.contains(COMPOSE_INTERRUPT_NOTICE),
        "nothing interrupted this run; stderr:\n{plain}"
    );
    assert_eq!(
        orphan.events(),
        vec!["failure", "finalize"],
        "stderr:\n{plain}"
    );
}

/// How the fake OpenCode's orphan answers the teardown's `SIGTERM`; either way
/// its trap touches the fixture's `term_marker` first.
#[cfg(unix)]
#[derive(Clone, Copy)]
enum OrphanOnTerm {
    /// Keeps running, so only Ctrl+C or the kill grace ends the teardown.
    Survive,
    /// Exits, emptying the group.
    Exit,
}

/// A compose whose fake OpenCode fails and leaves a descendant holding its
/// output pipes, and whose `failure` and `finalize` stacks append to
/// `events.log`.
///
/// The provider exits only once the descendant has installed its trap (the
/// `orphan` barrier), so the wrapper's post-exit teardown always meets a live
/// descendant. The descendant sits in the provider's process group, which no
/// [`SignalledRun`] owns: dropping the barrier is what ends it if the wrapper
/// never does. The borrow of `fixture` keeps the workspace holding the
/// barrier's files alive until then.
#[cfg(unix)]
struct OrphanTeardownFixture<'a> {
    fixture: &'a CliProcessFixture,
    md_file: std::path::PathBuf,
    events_log: std::path::PathBuf,
    /// Written by the orphan's `SIGTERM` trap.
    term_marker: std::path::PathBuf,
    orphan: ShellBarrier,
}

#[cfg(unix)]
impl<'a> OrphanTeardownFixture<'a> {
    fn new(fixture: &'a CliProcessFixture, on_term: OrphanOnTerm) -> Self {
        fixture.seed_user_config();
        // A valid frontmatter model hint keeps OpenCode's non-TTY model
        // requirement satisfied (see `compose_sigint_during_prep_exits_130_with_notice`).
        let md_file = fixture.cwd().join("orphan.md");
        write(
            &md_file,
            r#"---
title: orphan teardown
model: llamacpp/test-model
failure:
  stack:
    - action: {append_line: ["events.log", "failure"]}
finalize:
  stack:
    - action: {append_line: ["events.log", "finalize"]}
---
Prompt body
"#,
        );

        let term_marker = fixture.workspace_path().join("orphan-got-sigterm");
        let orphan = ShellBarrier::new(fixture.workspace_path(), "orphan");
        let orphan_script = fixture.bin_dir().join("opencode-orphan");
        let term_trap = match on_term {
            OrphanOnTerm::Survive => r#"trap ': > "$CLAUDINE_TERM_MARKER"' TERM"#,
            OrphanOnTerm::Exit => r#"trap ': > "$CLAUDINE_TERM_MARKER"; exit 0' TERM"#,
        };
        write_executable(
            &orphan_script,
            &format!("#!/bin/sh\n{term_trap}\n{}\n", orphan.block_snippet()),
        );
        write_executable(
            &fixture.bin_dir().join("opencode"),
            &format!(
                r#"#!/bin/sh
if [ "$1" = "models" ]; then
  printf '%s\n' '["test-model"]'
  exit 0
fi
'{orphan_script}' &
{await_orphan}
printf '%s\n' '{{"type":"init","session_id":"orphan","model":"test-model"}}'
echo "ProviderModelNotFoundError: Model not found: test/missing." >&2
exit 1
"#,
                orphan_script = orphan_script.display(),
                await_orphan = orphan.await_reached_snippet(),
            ),
        );

        Self {
            fixture,
            md_file,
            events_log: fixture.cwd().join("events.log"),
            term_marker,
            orphan,
        }
    }

    /// Spawn the compose with a 60 s kill grace, so only an interrupt (or the
    /// group emptying) can end the teardown promptly.
    fn spawn(&self) -> SignalledRun {
        let mut command = self.fixture.command_std();
        command
            .env("CLAUDINE_TERM_MARKER", &self.term_marker)
            .env("CLAUDINE_KILL_GRACE", "60s")
            .args(["compose", "--opencode", "-y", self.md_file.to_str().unwrap()]);
        SignalledRun::spawn(command, self.fixture.workspace_path())
    }

    fn events(&self) -> Vec<String> {
        read_lines(&self.events_log)
    }
}

/// A compose whose provider fails and whose `failure` stack blocks in a
/// `shell` action on the `failure-action` barrier between appending
/// `failure-started` and `failure-done` to `events.log`; `finalize` appends
/// `finalize`.
///
/// The barrier is reached only once the terminal lifecycle scope is entered, so
/// a press sent after [`ShellBarrier::wait_reached`] lands inside it. The
/// borrow of `fixture` keeps the barrier's files alive until the barrier has
/// ended its shell.
#[cfg(unix)]
struct TerminalLifecycleFixture<'a> {
    fixture: &'a CliProcessFixture,
    md_file: std::path::PathBuf,
    events_log: std::path::PathBuf,
    failure_action: ShellBarrier,
}

#[cfg(unix)]
impl<'a> TerminalLifecycleFixture<'a> {
    fn new(fixture: &'a CliProcessFixture) -> Self {
        fixture.seed_user_config();
        write_executable(&fixture.bin_dir().join("claude"), "#!/bin/sh\nexit 1\n");
        let failure_action = ShellBarrier::new(fixture.workspace_path(), "failure-action");
        let md_file = fixture.cwd().join("grace.md");
        write(
            &md_file,
            &format!(
                r#"---
title: terminal grace
failure:
  stack:
    - action:
        - append_line: ["events.log", "failure-started"]
        - shell: |
            {}
        - append_line: ["events.log", "failure-done"]
finalize:
  stack:
    - action: {{append_line: ["events.log", "finalize"]}}
---
Prompt body
"#,
                failure_action.block_snippet()
            ),
        );
        Self {
            fixture,
            md_file,
            events_log: fixture.cwd().join("events.log"),
            failure_action,
        }
    }

    fn spawn(&self) -> SignalledRun {
        let mut command = self.fixture.command_std();
        command.args(["compose", "--claude", "-y", self.md_file.to_str().unwrap()]);
        SignalledRun::spawn(command, self.fixture.workspace_path())
    }

    fn events(&self) -> Vec<String> {
        read_lines(&self.events_log)
    }
}

/// Whether the blocked `failure` action is let go once the grace is armed.
#[cfg(unix)]
#[derive(Clone, Copy)]
enum FailureAction {
    /// Released as soon as the grace notice is observed: the rest of the
    /// lifecycle is milliseconds of work that must fit in the grace.
    ReleasedOnGrace,
    /// Never released: the lifecycle outlasts any grace.
    Held,
}

/// Outcome of pressing Ctrl+C twice while a `failure` stack is running.
#[cfg(unix)]
struct TerminalGraceOutcome {
    exit_code: Option<i32>,
    /// From just before the second press was sent — a lower bound on when the
    /// grace began.
    press_to_exit: std::time::Duration,
    /// From observing the grace notice — an upper bound on when it began.
    grace_ack_to_exit: std::time::Duration,
    events: Vec<String>,
    stderr: String,
}

/// Press Ctrl+C twice inside a blocked `failure` action, each press only after
/// the previous one is acknowledged on stderr.
#[cfg(unix)]
fn press_twice_during_failure_stack(action: FailureAction) -> TerminalGraceOutcome {
    let fixture = CliProcessFixture::named("wrap-sigint-grace");
    let lifecycle = TerminalLifecycleFixture::new(&fixture);
    let mut run = lifecycle.spawn();

    lifecycle.failure_action.wait_reached(&mut run, READY_TIMEOUT);
    run.interrupt();
    run.wait_for_stderr(COMPOSE_INTERRUPT_NOTICE, ACK_TIMEOUT);
    let second_press_sent_at = run.interrupt();
    let grace_acknowledged_at = run.wait_for_stderr(GRACE_ARMED_NOTICE, ACK_TIMEOUT);
    if let FailureAction::ReleasedOnGrace = action {
        lifecycle.failure_action.release();
    }

    let exit = run.wait_for_exit(EXIT_TIMEOUT);
    TerminalGraceOutcome {
        exit_code: exit.status.code(),
        press_to_exit: exit.exited_at - second_press_sent_at,
        grace_ack_to_exit: exit.exited_at.saturating_duration_since(grace_acknowledged_at),
        events: lifecycle.events(),
        stderr: exit.stderr,
    }
}

/// A repeat Ctrl+C during `failure` lets the event, and the `finalize` after
/// it, finish when they fit inside the 500 ms grace.
#[cfg(unix)]
#[test]
#[serial_test::serial]
fn compose_second_sigint_lets_a_short_failure_stack_finish() {
    let outcome = press_twice_during_failure_stack(FailureAction::ReleasedOnGrace);

    assert_eq!(
        outcome.events,
        vec!["failure-started", "failure-done", "finalize"],
        "the grace must let the in-flight failure stack and finalize complete; stderr:\n{}",
        outcome.stderr
    );
    assert!(
        !outcome.stderr.contains(GRACE_EXPIRED_NOTICE),
        "a lifecycle that fits the grace must not be force-exited; stderr:\n{}",
        outcome.stderr
    );
    assert!(
        outcome.grace_ack_to_exit < TERMINAL_LIFECYCLE_EXIT_GRACE + SCHEDULING_ALLOWANCE,
        "the run must end promptly after the grace; took {:?}",
        outcome.grace_ack_to_exit
    );
}

/// A repeat Ctrl+C during a `failure` stack that outlasts the grace
/// force-exits once the 500 ms are up rather than waiting it out.
#[cfg(unix)]
#[test]
#[serial_test::serial]
fn compose_second_sigint_force_exits_a_failure_stack_that_outlasts_the_grace() {
    let outcome = press_twice_during_failure_stack(FailureAction::Held);

    assert_eq!(outcome.exit_code, Some(130), "stderr:\n{}", outcome.stderr);
    // The second press's handler cannot run before the send, and the watcher
    // sleeps at least the full grace, so this bound needs no allowance.
    assert!(
        outcome.press_to_exit >= TERMINAL_LIFECYCLE_EXIT_GRACE,
        "the force-exit must not land before the 500ms grace; took {:?}",
        outcome.press_to_exit
    );
    assert!(
        outcome.grace_ack_to_exit < TERMINAL_LIFECYCLE_EXIT_GRACE + SCHEDULING_ALLOWANCE,
        "the force-exit must land at the end of the 500ms grace; took {:?}",
        outcome.grace_ack_to_exit
    );
    assert!(
        outcome.stderr.contains(GRACE_EXPIRED_NOTICE),
        "stderr:\n{}",
        outcome.stderr
    );
    assert_eq!(outcome.events, vec!["failure-started"], "stderr:\n{}", outcome.stderr);
}

/// How far past a wrongly armed grace deadline the zero- and one-press tests
/// hold the `failure` action. A deadline armed when the hold began would have
/// force-exited by `TERMINAL_LIFECYCLE_EXIT_GRACE` plus scheduling latency; a
/// wrapper that is late by more than this makes those tests pass vacuously,
/// never fail spuriously.
#[cfg(unix)]
const HOLD_PAST_GRACE: std::time::Duration = std::time::Duration::from_secs(1);

/// Outcome of a `failure` stack held past the grace with fewer than two
/// presses.
#[cfg(unix)]
struct UnboundedLifecycleOutcome {
    exit_code: Option<i32>,
    events: Vec<String>,
    stderr: String,
}

/// Hold the `failure` action for longer than the grace after `presses` (zero or
/// one) Ctrl+C, asserting the wrapper keeps running throughout, then release it.
#[cfg(unix)]
fn hold_failure_stack_past_the_grace(presses: u8) -> UnboundedLifecycleOutcome {
    let fixture = CliProcessFixture::named("wrap-sigint-no-grace");
    let lifecycle = TerminalLifecycleFixture::new(&fixture);
    let mut run = lifecycle.spawn();

    lifecycle
        .failure_action
        .wait_reached(&mut run, READY_TIMEOUT);
    let hold_from = match presses {
        0 => std::time::Instant::now(),
        1 => {
            let sent_at = run.interrupt();
            run.wait_for_stderr(COMPOSE_INTERRUPT_NOTICE, ACK_TIMEOUT);
            sent_at
        }
        _ => unreachable!("a second press arms the grace"),
    };
    run.assert_running_until(hold_from + TERMINAL_LIFECYCLE_EXIT_GRACE + HOLD_PAST_GRACE);
    lifecycle.failure_action.release();

    let exit = run.wait_for_exit(EXIT_TIMEOUT);
    UnboundedLifecycleOutcome {
        exit_code: exit.status.code(),
        events: lifecycle.events(),
        stderr: exit.stderr,
    }
}

/// Without any Ctrl+C, terminal lifecycle work may run as long as it needs:
/// entering the scope arms no deadline.
#[cfg(unix)]
#[test]
#[serial_test::serial]
fn compose_terminal_lifecycle_outlasts_the_grace_without_an_interrupt() {
    let outcome = hold_failure_stack_past_the_grace(0);

    assert_eq!(
        outcome.events,
        vec!["failure-started", "failure-done", "finalize"],
        "stderr:\n{}",
        outcome.stderr
    );
    assert_ne!(outcome.exit_code, Some(130), "stderr:\n{}", outcome.stderr);
    assert!(
        !outcome.stderr.contains(GRACE_ARMED_NOTICE),
        "stderr:\n{}",
        outcome.stderr
    );
}

/// A single Ctrl+C during terminal lifecycle work arms no deadline either; only
/// a repeat press does.
#[cfg(unix)]
#[test]
#[serial_test::serial]
fn compose_single_sigint_lets_terminal_lifecycle_outlast_the_grace() {
    let outcome = hold_failure_stack_past_the_grace(1);

    assert_eq!(
        outcome.events,
        vec!["failure-started", "failure-done", "finalize"],
        "stderr:\n{}",
        outcome.stderr
    );
    assert!(
        !outcome.stderr.contains(GRACE_ARMED_NOTICE) && !outcome.stderr.contains("force-exiting"),
        "one press must neither arm the grace nor force-exit; stderr:\n{}",
        outcome.stderr
    );
}

/// When, after the press that armed the grace, a third press is sent. Half the
/// grace splits the slack evenly: the test may oversleep by up to this much and
/// the press still lands before the deadline, and the wrapper may overrun the
/// deadline by up to this much and still exit within the grace of the third
/// press.
#[cfg(unix)]
const LATE_PRESS_OFFSET: std::time::Duration = std::time::Duration::from_millis(250);

/// A third Ctrl+C inside the grace neither force-exits at once nor restarts the
/// 500 ms: the run still ends on the deadline the second press armed.
#[cfg(unix)]
#[test]
#[serial_test::serial]
fn compose_later_sigints_do_not_postpone_the_grace_deadline() {
    let fixture = CliProcessFixture::named("wrap-sigint-late-press");
    let lifecycle = TerminalLifecycleFixture::new(&fixture);
    let mut run = lifecycle.spawn();

    lifecycle
        .failure_action
        .wait_reached(&mut run, READY_TIMEOUT);
    run.interrupt();
    run.wait_for_stderr(COMPOSE_INTERRUPT_NOTICE, ACK_TIMEOUT);
    let grace_press_sent_at = run.interrupt();
    run.wait_for_stderr(GRACE_ARMED_NOTICE, ACK_TIMEOUT);
    run.assert_running_until(grace_press_sent_at + LATE_PRESS_OFFSET);
    let late_press_sent_at = run.interrupt();
    // Every press in the window writes the grace notice, so the second copy
    // proves the late press was handled before the deadline.
    run.wait_for_stderr_count(GRACE_ARMED_NOTICE, 2, ACK_TIMEOUT);

    let exit = run.wait_for_exit(EXIT_TIMEOUT);
    let plain = &exit.stderr;

    assert_eq!(exit.status.code(), Some(130), "stderr:\n{plain}");
    assert!(
        exit.exited_at - grace_press_sent_at >= TERMINAL_LIFECYCLE_EXIT_GRACE,
        "the force-exit must not land before the 500ms grace; stderr:\n{plain}"
    );
    // A deadline restarted by the late press could not fire before this.
    assert!(
        exit.exited_at - late_press_sent_at < TERMINAL_LIFECYCLE_EXIT_GRACE,
        "the late press must not postpone the deadline; exited {:?} after it; stderr:\n{plain}",
        exit.exited_at - late_press_sent_at
    );
    assert_eq!(
        plain.matches(GRACE_EXPIRED_NOTICE).count(),
        1,
        "stderr:\n{plain}"
    );
    assert_eq!(
        lifecycle.events(),
        vec!["failure-started"],
        "stderr:\n{plain}"
    );
}

/// Dropping the orphan fixture ends an orphan that a test left running — the
/// shape of a panic before Claudine tears down the fake OpenCode group — before
/// the workspace holding its release file is removed.
#[cfg(unix)]
#[test]
#[serial_test::serial]
fn orphan_fixture_drop_ends_an_orphan_outside_the_wrapper_group() {
    let fixture = CliProcessFixture::named("wrap-sigint-orphan-drop");
    let orphan = OrphanTeardownFixture::new(&fixture, OrphanOnTerm::Survive);
    let mut run = orphan.spawn();
    run.wait_for_path(&orphan.term_marker, READY_TIMEOUT);
    let orphan_pid = recorded_pid(orphan.orphan.reached_path());

    // The wrapper is still inside its 60 s teardown, so killing its group
    // leaves the orphan running in the provider's group.
    drop(run);
    assert!(
        process_exists(orphan_pid),
        "the orphan must outlive the wrapper's group for this test to mean anything"
    );

    // Checked once, not polled: the drop must not return while the orphan
    // could still be waiting on a release file.
    drop(orphan);
    assert!(
        !process_exists(orphan_pid),
        "dropping the fixture must end the orphan before it returns"
    );
}

/// A shell that never polls for its release is killed once the bound expires,
/// so the directory holding its marker files can always be removed.
#[cfg(unix)]
#[test]
fn shell_barrier_kills_a_shell_that_ignores_its_release() {
    let workspace = tempfile::tempdir().expect("workspace");
    let barrier = ShellBarrier::new(workspace.path(), "stubborn");
    let reached = barrier.reached_path();
    let stubborn = workspace.path().join("stubborn.sh");
    write_executable(
        &stubborn,
        &format!(
            "#!/bin/sh\necho $$ > '{staging}' && mv '{staging}' '{reached}'\nexec /bin/sleep 600\n",
            staging = reached.with_extension("staging").display(),
            reached = reached.display(),
        ),
    );
    // Started in the background of a shell that exits at once, so the stubborn
    // shell is not this test's child: `kill(pid, 0)` then tracks its life
    // rather than a zombie this test would have to reap.
    let status = std::process::Command::new("/bin/sh")
        .arg("-c")
        .arg(format!(
            "'{}' </dev/null >/dev/null 2>&1 &",
            stubborn.display()
        ))
        .status()
        .expect("start the stubborn shell");
    assert!(status.success());

    let deadline = std::time::Instant::now() + READY_TIMEOUT;
    while !reached.exists() {
        assert!(
            std::time::Instant::now() < deadline,
            "the stubborn shell never reached the barrier"
        );
        std::thread::sleep(std::time::Duration::from_millis(5));
    }
    let pid = recorded_pid(reached);

    barrier.release_or_kill(std::time::Duration::from_millis(200));
    assert_gone(pid);
}

/// An exited wrapper stays unreaped until the run is dropped, so its pid — the
/// id of the group `Drop` kills — cannot be reused in between.
#[cfg(unix)]
#[test]
fn signalled_run_keeps_an_exited_wrapper_unreaped_until_drop() {
    let workspace = tempfile::tempdir().expect("workspace");
    let mut command = std::process::Command::new("/bin/sh");
    command.args(["-c", "exit 3"]);
    let mut run = SignalledRun::spawn(command, workspace.path());

    let exit = run.wait_for_exit(EXIT_TIMEOUT);
    let pid = run.pid();
    assert_eq!(exit.status.code(), Some(3));
    assert!(
        process_exists(pid),
        "the exited wrapper must still hold its pid until drop"
    );
    // A second observation must agree with the first: nothing reaped it.
    assert_eq!(run.wait_for_exit(EXIT_TIMEOUT).status.code(), Some(3));

    drop(run);
    assert!(!process_exists(pid), "drop must reap the wrapper");
}

/// The unreaped exit is decoded like `wait(2)` would: a signal death reports
/// the signal and no exit code.
#[cfg(unix)]
#[test]
fn signalled_run_reports_a_wrapper_killed_by_a_signal() {
    use std::os::unix::process::ExitStatusExt as _;

    let workspace = tempfile::tempdir().expect("workspace");
    let mut command = std::process::Command::new("/bin/sh");
    command.args(["-c", "kill -KILL $$"]);
    let mut run = SignalledRun::spawn(command, workspace.path());

    let exit = run.wait_for_exit(EXIT_TIMEOUT);
    assert_eq!(exit.status.signal(), Some(libc::SIGKILL));
    assert_eq!(exit.status.code(), None);
}

#[cfg(unix)]
fn recorded_pid(reached: &std::path::Path) -> i32 {
    let pid = std::fs::read_to_string(reached).expect("read barrier pid");
    pid.trim()
        .parse()
        .unwrap_or_else(|_| panic!("barrier recorded a non-pid {pid:?}"))
}

#[cfg(unix)]
fn process_exists(pid: i32) -> bool {
    // SAFETY: signal 0 only probes whether `pid` exists.
    unsafe { libc::kill(pid, 0) == 0 }
}

/// Assert `pid` disappears promptly. A killed non-child lingers until its new
/// parent reaps it, so this polls briefly rather than checking once.
#[cfg(unix)]
fn assert_gone(pid: i32) {
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(2);
    while process_exists(pid) {
        assert!(
            std::time::Instant::now() < deadline,
            "pid {pid} is still running"
        );
        std::thread::sleep(std::time::Duration::from_millis(5));
    }
}

#[cfg(unix)]
fn read_lines(path: &std::path::Path) -> Vec<String> {
    std::fs::read_to_string(path)
        .unwrap_or_default()
        .lines()
        .map(str::to_string)
        .collect()
}
