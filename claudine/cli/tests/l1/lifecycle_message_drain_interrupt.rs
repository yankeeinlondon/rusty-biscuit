//! R4: Ctrl+C during the exit-time delivery drain.
//!
//! A first press prints a notice and the drain carries on; a second press
//! takes the compose force-exit rung and exits `130` at once, well inside the
//! 10 s drain budget. `compose` keeps its own interrupt guard through the
//! drain; `sequence` has none after it returns, so the shutdown path installs
//! the same ladder with its own notice.
//!
//! Unix sends `SIGINT` to the process through `common::signal::SignalledRun`.
//! Windows spawns `claudine` in `CREATE_NEW_PROCESS_GROUP` and sends
//! `CTRL_BREAK_EVENT` to that group, the pattern `sequence_ctrl_c_windows`
//! documents. Neither opens a window, so nothing takes focus.

use crate::common;
use crate::lifecycle_message_drain::write_one_line_claude;

use common::CliProcessFixture;
use common::webhook_listener::{ListenerMode, WebhookListener, write_webhook_route};
use common::write;
use std::process::Command;
use std::time::Duration;

/// Ceiling on every wait for a notice, a request, or an exit.
const WAIT: Duration = Duration::from_secs(60);

/// Pause between seeing that the command has finished its work and the first
/// press, so the press lands in the drain rather than in the command's last
/// steps. The drain itself lasts 10 s.
const SETTLE: Duration = Duration::from_millis(500);

/// How long the drain must keep running after the first press.
const STILL_DRAINING: Duration = Duration::from_secs(1);

/// The second press must end the process within this; the drain would
/// otherwise run for most of its 10 s budget.
const FORCE_EXIT_WITHIN: Duration = Duration::from_secs(3);

/// The notice `compose`'s own guard prints on a first press.
const COMPOSE_NOTICE: &str = "User interrupted compose operation";

/// The notice the shutdown path's drain ladder prints on a first press.
const DRAIN_NOTICE: &str = "User interrupted while waiting for outbound messages";

/// Which command the scenario runs.
enum Scenario {
    Compose,
    Sequence,
}

impl Scenario {
    /// Write the document, and return the command to run it plus the stderr
    /// text that shows the command's own work is done.
    fn prepare(&self, fixture: &CliProcessFixture, listener: &WebhookListener) -> (Command, &'static str) {
        write_webhook_route(fixture.home());
        write_one_line_claude(fixture.bin_dir());
        let (subcommand, document, finished) = match self {
            Self::Compose => (
                "compose",
                "---\nsuccess:\n  message: \"drain-interrupt-marker\"\n---\nSay hello.\n",
                "no tool calls",
            ),
            Self::Sequence => (
                "sequence",
                "---\nsequence:\n  - only\nsuccess:\n  message: \"drain-interrupt-marker\"\n---\nRun step {{ state.name }}\n",
                "Sequence finished",
            ),
        };
        let path = fixture.cwd().join("interrupt.md");
        write(&path, document);
        let mut command = fixture.command_std();
        listener.apply_route_env(&mut command);
        command.args([subcommand, "--claude", path.to_str().expect("UTF-8 path")]);
        (command, finished)
    }

    fn first_press_notice(&self) -> &'static str {
        match self {
            Self::Compose => COMPOSE_NOTICE,
            Self::Sequence => DRAIN_NOTICE,
        }
    }
}

/// Run `scenario` against a route that never replies, press Ctrl+C twice
/// during the drain, and check each press's effect.
fn second_press_during_the_drain_exits_130(scenario: Scenario, name: &str) {
    let fixture = CliProcessFixture::named(name);
    let listener = WebhookListener::start(ListenerMode::NeverReply);
    let (command, finished_marker) = scenario.prepare(&fixture, &listener);
    let mut run = platform::Run::spawn(command, fixture.workspace_path());

    listener
        .wait_for_request(WAIT)
        .unwrap_or_else(|| panic!("the message was never sent; stderr:\n{}", run.stderr()));
    run.wait_for_stderr(finished_marker, WAIT);
    std::thread::sleep(SETTLE);

    run.press();
    let noticed = run.wait_for_stderr(scenario.first_press_notice(), WAIT);
    run.assert_running_until(noticed + STILL_DRAINING);

    let second = run.press();
    let (code, exited_at, stderr) = run.wait_for_exit(WAIT);
    assert_eq!(code, Some(130), "the second press force-exits; stderr:\n{stderr}");
    assert!(
        exited_at.duration_since(second) < FORCE_EXIT_WITHIN,
        "the second press exits at once, not at the end of the drain ({:?}); stderr:\n{stderr}",
        exited_at.duration_since(second)
    );
    assert!(
        !stderr.contains("still sending at exit"),
        "a forced exit skips the drain's report; stderr:\n{stderr}"
    );
}

#[test]
fn a_second_ctrl_c_during_the_compose_drain_exits_130() {
    second_press_during_the_drain_exits_130(Scenario::Compose, "drain-interrupt-compose");
}

#[test]
fn a_second_ctrl_c_during_the_sequence_drain_exits_130() {
    second_press_during_the_drain_exits_130(Scenario::Sequence, "drain-interrupt-sequence");
}

#[cfg(unix)]
mod platform {
    use super::common::signal::SignalledRun;
    use std::path::Path;
    use std::process::Command;
    use std::time::{Duration, Instant};

    pub(super) struct Run(SignalledRun);

    impl Run {
        pub(super) fn spawn(command: Command, output_dir: &Path) -> Self {
            Self(SignalledRun::spawn(command, output_dir))
        }

        pub(super) fn stderr(&self) -> String {
            self.0.stderr()
        }

        pub(super) fn press(&self) -> Instant {
            self.0.interrupt()
        }

        pub(super) fn wait_for_stderr(&mut self, needle: &str, timeout: Duration) -> Instant {
            self.0.wait_for_stderr(needle, timeout)
        }

        pub(super) fn assert_running_until(&mut self, until: Instant) {
            self.0.assert_running_until(until);
        }

        pub(super) fn wait_for_exit(&mut self, timeout: Duration) -> (Option<i32>, Instant, String) {
            let exit = self.0.wait_for_exit(timeout);
            (exit.status.code(), exit.exited_at, exit.stderr)
        }
    }
}

#[cfg(windows)]
mod platform {
    use super::common::strip_ansi;
    use std::fs;
    use std::os::windows::process::CommandExt;
    use std::path::{Path, PathBuf};
    use std::process::{Child, Command, Stdio};
    use std::time::{Duration, Instant};

    /// Makes Claudine's pid a console process-group id that
    /// `GenerateConsoleCtrlEvent` can target without reaching this test.
    const CREATE_NEW_PROCESS_GROUP: u32 = 0x0000_0200;

    /// `CTRL_C_EVENT` cannot target one group, and a new group starts with
    /// Ctrl+C disabled, so Ctrl+Break is the deliverable press.
    const CTRL_BREAK_EVENT: u32 = 1;

    const POLL: Duration = Duration::from_millis(10);

    #[link(name = "kernel32")]
    unsafe extern "system" {
        fn GenerateConsoleCtrlEvent(dwCtrlEvent: u32, dwProcessGroupId: u32) -> i32;
    }

    /// A `claudine` in its own console process group, with output captured
    /// to files so stderr can be polled while it runs. Dropping it kills a
    /// child that is still running.
    pub(super) struct Run {
        child: Child,
        stderr_path: PathBuf,
    }

    impl Run {
        pub(super) fn spawn(mut command: Command, output_dir: &Path) -> Self {
            let stdout_path = output_dir.join("drain-interrupt.stdout");
            let stderr_path = output_dir.join("drain-interrupt.stderr");
            let child = command
                .creation_flags(CREATE_NEW_PROCESS_GROUP)
                .stdin(Stdio::null())
                .stdout(fs::File::create(&stdout_path).expect("create stdout capture"))
                .stderr(fs::File::create(&stderr_path).expect("create stderr capture"))
                .spawn()
                .expect("spawn claudine");
            Self { child, stderr_path }
        }

        pub(super) fn stderr(&self) -> String {
            strip_ansi(&fs::read_to_string(&self.stderr_path).unwrap_or_default())
        }

        pub(super) fn press(&self) -> Instant {
            let sent_at = Instant::now();
            // SAFETY: a documented Win32 call with plain integer arguments;
            // the group id is the pid of a child this run spawned.
            let sent = unsafe { GenerateConsoleCtrlEvent(CTRL_BREAK_EVENT, self.child.id()) };
            assert_ne!(
                sent,
                0,
                "GenerateConsoleCtrlEvent failed: {}",
                std::io::Error::last_os_error()
            );
            sent_at
        }

        pub(super) fn wait_for_stderr(&mut self, needle: &str, timeout: Duration) -> Instant {
            let deadline = Instant::now() + timeout;
            loop {
                if self.stderr().contains(needle) {
                    return Instant::now();
                }
                if let Some(status) = self.child.try_wait().expect("poll child")
                    && !self.stderr().contains(needle)
                {
                    panic!("claudine exited ({status}) without writing {needle:?}; stderr:\n{}", self.stderr());
                }
                assert!(
                    Instant::now() < deadline,
                    "claudine did not write {needle:?} within {timeout:?}; stderr:\n{}",
                    self.stderr()
                );
                std::thread::sleep(POLL);
            }
        }

        pub(super) fn assert_running_until(&mut self, until: Instant) {
            while Instant::now() < until {
                if let Some(status) = self.child.try_wait().expect("poll child") {
                    panic!("claudine exited ({status}) too early; stderr:\n{}", self.stderr());
                }
                std::thread::sleep(POLL);
            }
        }

        pub(super) fn wait_for_exit(&mut self, timeout: Duration) -> (Option<i32>, Instant, String) {
            let deadline = Instant::now() + timeout;
            loop {
                if let Some(status) = self.child.try_wait().expect("poll child") {
                    return (status.code(), Instant::now(), self.stderr());
                }
                assert!(
                    Instant::now() < deadline,
                    "claudine did not exit within {timeout:?}; stderr:\n{}",
                    self.stderr()
                );
                std::thread::sleep(POLL);
            }
        }
    }

    impl Drop for Run {
        fn drop(&mut self) {
            if matches!(self.child.try_wait(), Ok(None)) {
                let _ = self.child.kill();
                let _ = self.child.wait();
            }
        }
    }
}
