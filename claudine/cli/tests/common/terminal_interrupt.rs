//! Lifecycle Ctrl+C scenarios driven through a real terminal pane.
//!
//! `level2_lifecycle_ctrl_c_tmux.rs` and `level3_lifecycle_ctrl_c.rs` run the
//! same scenarios and differ only in how Ctrl+C reaches the pane: a tmux
//! key or an OS keyboard chord. Either way the pane's line discipline turns
//! it into `SIGINT` for the foreground process group, which the L1
//! `wrap_sigint.rs` tests (a `kill(2)` to the wrapper alone) never exercise.
//! The press is a closure the caller supplies, so the focus-stealing APIs an
//! OS chord needs stay in the `level3_` file.
//!
//! The fixtures mirror `wrap_sigint.rs`'s `OrphanTeardownFixture` and
//! `TerminalLifecycleFixture` with one difference. `SystemShellRunner` runs a
//! lifecycle `shell` action in Claudine's own process group, so a keyboard
//! press also signals it; the blocking action here ignores `SIGINT`, standing
//! in for lifecycle work a press does not kill. Without that the first press
//! would end the action, and the grace would have nothing to wait for.
//!
//! The repeat-press teardown scenario needs the teardown held open, because
//! the first press ends it within one 50 ms poll and an orphan that exits on
//! `SIGTERM` ends it with no press at all. It sets
//! `CLAUDINE_TEST_TEARDOWN_HOLD`, a seam the binary honors only when built with
//! the `terminal-tests` feature these tiers already require.
//!
//! Precise timing (a force-exit no earlier than the grace, a deadline measured
//! from the press itself) stays in L1: a pane is observed by polling and a
//! keypress by its echo, so the bounds here are coarser and each assertion
//! says which latencies it absorbs.

use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU32, Ordering};
use std::time::{Duration, Instant};

use biscuit_test_harness::TerminalHarness;
use tempfile::TempDir;

use super::signal::{
    COMPOSE_INTERRUPT_NOTICE, GRACE_ARMED_NOTICE, GRACE_EXPIRED_NOTICE, ShellBarrier,
    TERMINAL_LIFECYCLE_EXIT_GRACE,
};
use super::{claudine_bin, sh_quote, wait_for_exit_marker, write, write_executable};

/// Bound on wrapper startup, prep, and the provider attempt reaching the
/// window under test.
const READY_TIMEOUT: Duration = Duration::from_secs(30);

/// Bound on a press's notice being drawn. Includes the injector's own latency,
/// which for an OS chord is a click and an `osascript` round trip.
const ACK_TIMEOUT: Duration = Duration::from_secs(10);

/// Bound on the shell regaining control once the test expects the wrapper to
/// exit.
const EXIT_TIMEOUT: Duration = Duration::from_secs(20);

/// Slack past the grace for the injector, the pane poll, and scheduling. Far
/// below what it guards against: a held lifecycle never finishes on its own.
const PANE_ALLOWANCE: Duration = Duration::from_secs(3);

/// Poll interval for pane captures. Short, because the short-lifecycle
/// scenario must acknowledge the grace notice and release the action well
/// inside the 500 ms grace.
const PANE_POLL: Duration = Duration::from_millis(10);

/// How long past the grace a single press must leave the wrapper running.
const SINGLE_PRESS_HOLD: Duration = Duration::from_secs(1);

/// Minimum spacing between sending the second and the third press. Wide
/// enough that a deadline restarted by the third press would exit visibly
/// later than the original one; narrow enough that an OS chord, whose
/// injection takes a click and an `osascript` round trip, still lands inside
/// the 500 ms grace.
const THIRD_PRESS_SPACING: Duration = Duration::from_millis(250);

/// Latest the wrapper may exit after the third press is seen, when that press
/// did not extend the grace. A restarted deadline exits 500 ms after the
/// press was delivered, which is at most one notice observation (a pane poll
/// and a capture, well under 100 ms) before it was seen.
const THIRD_PRESS_EXIT_BOUND: Duration = Duration::from_millis(400);

/// How long a deferred repeat press must leave the held teardown running. A
/// force-exit writes its notice and exits within milliseconds of the press
/// being echoed, so this is far longer than the failure it guards against.
const DEFER_SETTLE: Duration = Duration::from_millis(500);

/// What the pane's line discipline echoes for each Ctrl+C it turns into
/// `SIGINT` (`ECHOCTL`), so the count shows how many presses were delivered.
const CTRL_C_ECHO: &str = "^C";

/// The pane text of a force-exit notice, shared by the immediate and the
/// grace-expired rungs.
const FORCE_EXIT_TEXT: &str = "force-exiting";

/// One `claudine compose` typed into a pane, with the files it reads and
/// writes in a private workspace that is also the pane's `HOME` and CWD.
pub struct PaneCompose {
    /// The orphan in the teardown scenario, the `failure` action otherwise.
    /// Ended in `Drop`; see there.
    barrier: ShellBarrier,
    workspace: TempDir,
    events_log: PathBuf,
    /// Written once the wrapper sits in the window a press must land in.
    ready: PathBuf,
    /// `CLAUDINE_TEST_TEARDOWN_HOLD`'s path, when the teardown is held.
    teardown_hold: Option<PathBuf>,
    args: String,
    env: Vec<(&'static str, String)>,
    marker: String,
}

/// What the pane showed once the wrapper exited.
pub struct PaneExit {
    /// The wrapper's `$?`, echoed by the shell after it.
    pub status: String,
    /// When the exit was observed; at most one pane poll after the shell
    /// printed it.
    pub observed_at: Instant,
    /// The final pane text.
    pub pane: String,
}

impl PaneCompose {
    /// A compose whose fake OpenCode fails and leaves a `SIGTERM`-surviving
    /// descendant in its group, with a 60 s kill grace; ready once the
    /// wrapper's teardown has signalled that descendant.
    pub fn orphan_teardown() -> Self {
        Self::orphan_teardown_with(false)
    }

    /// [`PaneCompose::orphan_teardown`] whose teardown, however its signalling
    /// ends, then stays in its interrupt-deferring window until
    /// [`PaneCompose::release_teardown`].
    pub fn orphan_teardown_held() -> Self {
        Self::orphan_teardown_with(true)
    }

    fn orphan_teardown_with(hold: bool) -> Self {
        let workspace = tempfile::tempdir().expect("workspace");
        let root = workspace.path();
        let bin_dir = root.join("bin");
        super::wrap::seed_minimal_config(root);
        let md_file = root.join("orphan.md");
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

        let term_marker = root.join("orphan-got-sigterm");
        let orphan = ShellBarrier::new(root, "orphan");
        let orphan_script = bin_dir.join("opencode-orphan");
        write_executable(
            &orphan_script,
            &format!(
                "#!/bin/sh\ntrap ': > \"$CLAUDINE_TERM_MARKER\"' TERM\n{}\n",
                orphan.block_snippet()
            ),
        );
        write_executable(
            &bin_dir.join("opencode"),
            &format!(
                r#"#!/bin/sh
if [ "$1" = "models" ]; then
  printf '%s\n' '["test-model"]'
  exit 0
fi
{orphan_script} &
{await_orphan}
printf '%s\n' '{{"type":"init","session_id":"orphan","model":"test-model"}}'
echo "ProviderModelNotFoundError: Model not found: test/missing." >&2
exit 1
"#,
                orphan_script = sh_quote(&orphan_script.display().to_string()),
                await_orphan = orphan.await_reached_snippet(),
            ),
        );

        let mut env = vec![
            ("CLAUDINE_KILL_GRACE", "60s".to_string()),
            ("CLAUDINE_TERM_MARKER", term_marker.display().to_string()),
        ];
        let teardown_hold = hold.then(|| root.join("teardown-hold"));
        if let Some(hold) = &teardown_hold {
            env.push(("CLAUDINE_TEST_TEARDOWN_HOLD", hold.display().to_string()));
        }
        let mut compose = Self::new(
            workspace,
            orphan,
            term_marker,
            format!("--opencode -y {}", sh_quote(&md_file.display().to_string())),
            env,
        );
        compose.teardown_hold = teardown_hold;
        compose
    }

    /// A compose whose provider fails and whose `failure` stack appends
    /// `failure-started`, blocks in a `SIGINT`-ignoring `shell` action until
    /// released, then appends `failure-done`; `finalize` appends `finalize`.
    /// Ready once the action is blocked, which is inside the terminal
    /// lifecycle scope.
    pub fn terminal_lifecycle() -> Self {
        let workspace = tempfile::tempdir().expect("workspace");
        let root = workspace.path();
        super::wrap::seed_minimal_config(root);
        write_executable(&root.join("bin").join("claude"), "#!/bin/sh\nexit 1\n");
        let failure_action = ShellBarrier::new(root, "failure-action");
        let md_file = root.join("grace.md");
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
            trap '' INT; {}
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
        let ready = failure_action.reached_path().to_path_buf();
        Self::new(
            workspace,
            failure_action,
            ready,
            format!("--claude -y {}", sh_quote(&md_file.display().to_string())),
            Vec::new(),
        )
    }

    fn new(
        workspace: TempDir,
        barrier: ShellBarrier,
        ready: PathBuf,
        args: String,
        mut env: Vec<(&'static str, String)>,
    ) -> Self {
        static SEQ: AtomicU32 = AtomicU32::new(0);
        env.extend([
            ("HOME", workspace.path().display().to_string()),
            ("CLAUDINE_RENDEZVOUS_REPORT", "false".to_string()),
        ]);
        let marker = format!(
            "LIFECYCLE_CTRL_C_{}_{}",
            std::process::id(),
            SEQ.fetch_add(1, Ordering::Relaxed)
        );
        Self {
            events_log: workspace.path().join("events.log"),
            workspace,
            ready,
            teardown_hold: None,
            barrier,
            args,
            env,
            marker,
        }
    }

    /// Type the compose into the pane's shell and wait until the wrapper sits
    /// in the window under test.
    ///
    /// ## Panics
    ///
    /// When the window is not reached within the readiness bound.
    pub fn launch(&self, harness: &mut impl TerminalHarness) {
        let root = self.workspace.path().display().to_string();
        let bin_dir = self.workspace.path().join("bin").display().to_string();
        // Separate lines keep the typed command short: an inline `PATH=` with
        // the whole system path wraps across many rows.
        harness
            .send_command_with_env(&format!("cd {}", sh_quote(&root)), &[])
            .expect("cd into workspace");
        harness
            .send_command_with_env(&format!("export PATH={}:\"$PATH\"", sh_quote(&bin_dir)), &[])
            .expect("prepend fake providers to PATH");
        let env: Vec<(&str, &str)> = self.env.iter().map(|(k, v)| (*k, v.as_str())).collect();
        // The status goes on a fresh line: the interrupt notice ends without a
        // newline, and the marker is only matched at the start of a row.
        harness
            .send_command_with_env(
                &format!(
                    "{} compose {} ; printf '\\n%s:%s\\n' {} $?",
                    sh_quote(claudine_bin()),
                    self.args,
                    self.marker
                ),
                &env,
            )
            .expect("type the compose command");

        let deadline = Instant::now() + READY_TIMEOUT;
        while !self.ready.exists() {
            assert!(
                Instant::now() < deadline,
                "the wrapper did not reach the window under test within {READY_TIMEOUT:?}; pane:\n{}",
                pane_text(harness)
            );
            std::thread::sleep(PANE_POLL);
        }
    }

    /// Wait for the shell to print the wrapper's exit status.
    pub fn wait_for_exit(&self, harness: &mut impl TerminalHarness) -> PaneExit {
        let (frame, status) = wait_for_exit_marker(harness, &self.marker, EXIT_TIMEOUT);
        PaneExit {
            status,
            observed_at: Instant::now(),
            pane: frame.plain,
        }
    }

    /// Wait until the held teardown has been reached: after a press cut the
    /// kill grace short, or earlier if the orphan exited on its own.
    ///
    /// ## Panics
    ///
    /// When it is not reached within the acknowledgement bound, which is also
    /// how a binary built without the `terminal-tests` seam fails.
    pub fn wait_for_teardown_hold(&self, harness: &mut impl TerminalHarness) {
        let reached = self.teardown_marker(".reached");
        let deadline = Instant::now() + ACK_TIMEOUT;
        while !reached.exists() {
            assert!(
                Instant::now() < deadline,
                "the teardown was not held within {ACK_TIMEOUT:?} of the press; is the binary \
                 built with `terminal-tests`? pane:\n{}",
                pane_text(harness)
            );
            std::thread::sleep(PANE_POLL);
        }
    }

    /// Let a held teardown finish.
    pub fn release_teardown(&self) {
        fs::write(self.teardown_marker(".release"), b"").expect("release the held teardown");
    }

    fn teardown_marker(&self, suffix: &str) -> PathBuf {
        let hold = self
            .teardown_hold
            .as_ref()
            .expect("the compose was built with a held teardown");
        PathBuf::from(format!("{}{suffix}", hold.display()))
    }

    fn events(&self) -> Vec<String> {
        fs::read_to_string(&self.events_log)
            .unwrap_or_default()
            .lines()
            .map(str::to_string)
            .collect()
    }
}

impl Drop for PaneCompose {
    /// The blocked shell belongs to no process group the test owns: the
    /// orphan sits in the provider's group, and a force-exited wrapper leaves
    /// its `failure` action behind in the pane's former foreground group. It
    /// is ended here, before the `workspace` field that holds its release file
    /// is dropped. A held teardown is released too, so a failed assertion does
    /// not leave the wrapper parked in it.
    fn drop(&mut self) {
        if self.teardown_hold.is_some() {
            let _ = fs::write(self.teardown_marker(".release"), b"");
        }
        self.barrier.release_or_kill(Duration::from_secs(5));
    }
}

/// Ctrl+C while the wrapper tears down an OpenCode run's surviving descendant
/// cuts the 60 s kill grace short and still runs `failure` then `finalize`.
pub fn assert_press_during_orphan_teardown_runs_the_lifecycle<H: TerminalHarness>(
    harness: &mut H,
    mut press: impl FnMut(&mut H),
) {
    let compose = PaneCompose::orphan_teardown();
    compose.launch(harness);
    let pressed_at = Instant::now();
    press(harness);

    let exit = compose.wait_for_exit(harness);
    let pane = &exit.pane;
    assert!(
        exit.observed_at - pressed_at < Duration::from_secs(10),
        "Ctrl+C must cut the 60s kill grace short; took {:?}; pane:\n{pane}",
        exit.observed_at - pressed_at
    );
    assert_ne!(exit.status, "0", "a failed run must not exit 0; pane:\n{pane}");
    assert!(
        joined(pane).contains(COMPOSE_INTERRUPT_NOTICE),
        "the press must be acknowledged in the pane; pane:\n{pane}"
    );
    assert!(
        !joined(pane).contains("force-exiting"),
        "the teardown must not force-exit the wrapper; pane:\n{pane}"
    );
    assert_eq!(
        compose.events(),
        vec!["failure", "finalize"],
        "the interrupted run must still fire its terminal lifecycle; pane:\n{pane}"
    );
}

/// A repeat Ctrl+C during a `failure` action that then finishes inside the
/// grace lets the event and `finalize` complete, and the pane shows the grace
/// notice but never the force-exit.
pub fn assert_repeat_press_lets_short_lifecycle_finish<H: TerminalHarness>(
    harness: &mut H,
    mut press: impl FnMut(&mut H),
) {
    let compose = PaneCompose::terminal_lifecycle();
    compose.launch(harness);
    press(harness);
    wait_for_notice(harness, COMPOSE_INTERRUPT_NOTICE);
    press(harness);
    wait_for_notice(harness, GRACE_ARMED_NOTICE);
    compose.barrier.release();

    let exit = compose.wait_for_exit(harness);
    let pane = &exit.pane;
    assert_eq!(
        compose.events(),
        vec!["failure-started", "failure-done", "finalize"],
        "the grace must let the in-flight failure stack and finalize complete; pane:\n{pane}"
    );
    assert!(
        !joined(pane).contains(GRACE_EXPIRED_NOTICE),
        "a lifecycle that fits the grace must not be force-exited; pane:\n{pane}"
    );
}

/// A repeat Ctrl+C during a `failure` action that never finishes force-exits
/// the wrapper with 130 at the end of the grace, and the pane shows both the
/// grace and the force-exit notices.
pub fn assert_repeat_press_force_exits_long_lifecycle<H: TerminalHarness>(
    harness: &mut H,
    mut press: impl FnMut(&mut H),
) {
    let compose = PaneCompose::terminal_lifecycle();
    compose.launch(harness);
    press(harness);
    wait_for_notice(harness, COMPOSE_INTERRUPT_NOTICE);
    let second_pressed_at = Instant::now();
    press(harness);

    let exit = compose.wait_for_exit(harness);
    let pane = &exit.pane;
    assert_eq!(exit.status, "130", "pane:\n{pane}");
    let press_to_exit = exit.observed_at - second_pressed_at;
    assert!(
        press_to_exit < TERMINAL_LIFECYCLE_EXIT_GRACE + PANE_ALLOWANCE,
        "the force-exit must land at the end of the 500ms grace; took {press_to_exit:?}; pane:\n{pane}"
    );
    let text = joined(pane);
    assert!(text.contains(GRACE_ARMED_NOTICE), "pane:\n{pane}");
    assert!(text.contains(GRACE_EXPIRED_NOTICE), "pane:\n{pane}");
    assert_eq!(compose.events(), vec!["failure-started"], "pane:\n{pane}");
}

/// A repeat Ctrl+C that lands while the wrapper is still tearing down an
/// OpenCode run's surviving descendant is deferred: no force-exit, and
/// `failure` then `finalize` still run once the teardown ends.
///
/// The binary's `terminal-tests` seam holds the teardown inside its deferring
/// window until the test releases it, whether the first press cut the 60 s
/// kill grace short or the orphan had already exited and the press landed in
/// the hold itself. The second press is known to have reached the wrapper's
/// process group once the pane echoes a second `^C`.
pub fn assert_repeat_press_during_orphan_teardown_defers<H: TerminalHarness>(
    harness: &mut H,
    mut press: impl FnMut(&mut H),
) {
    let compose = PaneCompose::orphan_teardown_held();
    compose.launch(harness);
    press(harness);
    compose.wait_for_teardown_hold(harness);
    wait_for_notice(harness, COMPOSE_INTERRUPT_NOTICE);

    press(harness);
    wait_for_notice_count(harness, CTRL_C_ECHO, 2);
    assert_pane_running_until(harness, &compose, Instant::now() + DEFER_SETTLE);
    compose.release_teardown();

    let exit = compose.wait_for_exit(harness);
    let pane = &exit.pane;
    assert!(
        !joined(pane).contains(FORCE_EXIT_TEXT),
        "a repeat press during the teardown must be deferred, not force-exit; pane:\n{pane}"
    );
    assert!(
        !["0", "130"].contains(&exit.status.as_str()),
        "a failed, interrupted run exits non-zero but not with the force-exit 130; got {}; \
         pane:\n{pane}",
        exit.status
    );
    assert_eq!(
        compose.events(),
        vec!["failure", "finalize"],
        "the deferred press must leave the terminal lifecycle to run; pane:\n{pane}"
    );
}

/// One Ctrl+C during a `failure` action arms no deadline: the wrapper keeps
/// running well past the 500 ms grace, and once the action is released the
/// event and `finalize` complete with no grace or force-exit notice.
pub fn assert_single_press_lets_lifecycle_outlast_the_grace<H: TerminalHarness>(
    harness: &mut H,
    mut press: impl FnMut(&mut H),
) {
    let compose = PaneCompose::terminal_lifecycle();
    compose.launch(harness);
    press(harness);
    // The press was delivered before its notice was seen, so a deadline it
    // armed would have expired by this bound.
    let acknowledged_at = wait_for_notice(harness, COMPOSE_INTERRUPT_NOTICE);
    assert_pane_running_until(
        harness,
        &compose,
        acknowledged_at + TERMINAL_LIFECYCLE_EXIT_GRACE + SINGLE_PRESS_HOLD,
    );
    compose.barrier.release();

    let exit = compose.wait_for_exit(harness);
    let pane = &exit.pane;
    assert_eq!(
        compose.events(),
        vec!["failure-started", "failure-done", "finalize"],
        "a single press must let the failure stack and finalize complete; pane:\n{pane}"
    );
    let text = joined(pane);
    assert!(!text.contains(GRACE_ARMED_NOTICE), "pane:\n{pane}");
    assert!(!text.contains(FORCE_EXIT_TEXT), "pane:\n{pane}");
}

/// A third Ctrl+C while the grace is armed is acknowledged but does not
/// postpone the deadline: the wrapper force-exits with 130 once, when the
/// grace armed by the second press ends.
pub fn assert_later_press_does_not_postpone_the_grace<H: TerminalHarness>(
    harness: &mut H,
    mut press: impl FnMut(&mut H),
) {
    let compose = PaneCompose::terminal_lifecycle();
    compose.launch(harness);
    press(harness);
    wait_for_notice(harness, COMPOSE_INTERRUPT_NOTICE);
    let second_sent_at = Instant::now();
    press(harness);
    wait_for_notice(harness, GRACE_ARMED_NOTICE);
    // Spaced by send time, so an OS chord's injection latency, the same for
    // both presses, cancels out of the spacing.
    assert_pane_running_until(harness, &compose, second_sent_at + THIRD_PRESS_SPACING);
    press(harness);
    // A second copy of the grace notice proves the press landed while the
    // grace was armed rather than after the force-exit.
    let third_seen_at = wait_for_notice_count(harness, GRACE_ARMED_NOTICE, 2);

    let exit = compose.wait_for_exit(harness);
    let pane = &exit.pane;
    assert_eq!(exit.status, "130", "pane:\n{pane}");
    assert_eq!(
        joined(pane).matches(GRACE_EXPIRED_NOTICE).count(),
        1,
        "the grace must expire exactly once; pane:\n{pane}"
    );
    assert_eq!(compose.events(), vec!["failure-started"], "pane:\n{pane}");
    let since_second = exit.observed_at - second_sent_at;
    assert!(
        since_second < TERMINAL_LIFECYCLE_EXIT_GRACE + PANE_ALLOWANCE,
        "the force-exit must land at the end of the grace the second press armed; took \
         {since_second:?}; pane:\n{pane}"
    );
    let since_third = exit.observed_at.saturating_duration_since(third_seen_at);
    assert!(
        since_third < THIRD_PRESS_EXIT_BOUND,
        "the third press must not restart the 500ms grace; the exit came {since_third:?} after \
         it was seen; pane:\n{pane}"
    );
}

/// Wait until the pane shows `needle`.
///
/// ## Returns
///
/// When it was seen; at most one pane poll and capture after it was drawn.
///
/// ## Panics
///
/// When it is not drawn within the acknowledgement bound.
fn wait_for_notice(harness: &mut impl TerminalHarness, needle: &str) -> Instant {
    wait_for_notice_count(harness, needle, 1)
}

/// Wait until the pane shows `needle` at least `count` times.
///
/// ## Returns
///
/// As [`wait_for_notice`], for the `count`th copy.
///
/// ## Panics
///
/// As [`wait_for_notice`].
fn wait_for_notice_count(
    harness: &mut impl TerminalHarness,
    needle: &str,
    count: usize,
) -> Instant {
    let deadline = Instant::now() + ACK_TIMEOUT;
    loop {
        let pane = pane_text(harness);
        if joined(&pane).matches(needle).count() >= count {
            return Instant::now();
        }
        assert!(
            Instant::now() < deadline,
            "the pane did not show {needle:?} {count} time(s) within {ACK_TIMEOUT:?}; pane:\n{pane}"
        );
        std::thread::sleep(PANE_POLL);
    }
}

/// Return at `until`, having polled the pane throughout: the wrapper has not
/// exited and has written no force-exit notice.
///
/// ## Panics
///
/// When either is seen before `until`.
fn assert_pane_running_until(
    harness: &mut impl TerminalHarness,
    compose: &PaneCompose,
    until: Instant,
) {
    let exited = format!("{}:", compose.marker);
    loop {
        let pane = pane_text(harness);
        let early = until.saturating_duration_since(Instant::now());
        assert!(
            !pane.lines().any(|line| line.trim().starts_with(&exited)),
            "the wrapper exited {early:?} before it was allowed to; pane:\n{pane}"
        );
        assert!(
            !joined(&pane).contains(FORCE_EXIT_TEXT),
            "the wrapper force-exited {early:?} before it was allowed to; pane:\n{pane}"
        );
        let now = Instant::now();
        if now >= until {
            return;
        }
        std::thread::sleep(PANE_POLL.min(until - now));
    }
}

fn pane_text(harness: &mut impl TerminalHarness) -> String {
    harness.capture().map(|frame| frame.plain).unwrap_or_default()
}

/// The pane with its rows joined: a notice written mid-line can be
/// hard-wrapped at any column, including inside the substring asserted on.
fn joined(pane: &str) -> String {
    pane.replace('\n', "")
}
