//! Ctrl+C during the exit-time delivery drain, for every command that can
//! reach it with a press.
//!
//! Each [`DrainCommand`] ends with one outbound `message` posted to a
//! [`WebhookListener`] that never replies, so the process sits in the drain
//! (10 s budget) after its own work is done. A first press prints the
//! command's notice and the drain carries on; a second takes the compose
//! force-exit rung and exits `130` at once. `compose` and `inline-compose`
//! hold their own interrupt guard through the drain; `sequence` and the
//! provider wrapper return without one, so the shutdown path installs the
//! same ladder with its own notice.
//!
//! The L1 `lifecycle_message_drain_interrupt.rs` tests deliver each press as a
//! control event sent to the process.
//! [`pane::assert_second_press_during_the_drain_exits_130`] runs the same
//! fixture in a terminal pane and takes the press as a closure, so the pane's
//! line discipline delivers `SIGINT` to the foreground process group, as it
//! does for a key pressed in that terminal.
//!
//! `handle` is not here: it runs as a hook subprocess with no terminal a user
//! could press in.

use std::path::Path;

use super::webhook_listener::write_config_with_webhook_route;
use super::write;
#[cfg(unix)]
use super::write_executable;

/// The notice `compose`'s own guard prints on a first press.
pub const COMPOSE_NOTICE: &str = "User interrupted compose operation";

/// The notice the shutdown path's drain ladder prints on a first press.
pub const DRAIN_NOTICE: &str = "User interrupted while waiting for outbound messages";

/// The drain's report on deliveries still running at an ordinary exit. A
/// forced exit skips it.
pub const PENDING_REPORT: &str = "still sending at exit";

/// The body of every message the fixture sends.
const MESSAGE: &str = "drain-interrupt-marker";

/// A `claude` stub that prints one successful result line and exits `0`.
pub fn write_one_line_claude(bin_dir: &Path) {
    #[cfg(unix)]
    write_executable(
        &bin_dir.join("claude"),
        r#"#!/bin/sh
cat > /dev/null 2>/dev/null
printf '%s\n' '{"type":"result","subtype":"success","result":"done","session_id":"session-1","is_error":false}'
exit 0
"#,
    );

    #[cfg(windows)]
    write(
        &bin_dir.join("claude.cmd"),
        "@echo off\r\n\
echo {\"type\":\"result\",\"subtype\":\"success\",\"result\":\"done\",\"session_id\":\"session-1\",\"is_error\":false}\r\n\
exit /b 0\r\n",
    );
}

/// A command whose exit waits on one outbound message.
#[derive(Clone, Copy, Debug)]
pub enum DrainCommand {
    Compose,
    InlineCompose,
    Sequence,
    /// `claudine claude`, whose message comes from a user-config hook action
    /// the wrapper dispatches in-process from the provider's stream. A direct
    /// wrapper has no document, so this is its only message source.
    Wrapper,
}

impl DrainCommand {
    /// Write the stub provider into `bin_dir`, the user config into `home`,
    /// and any document into `dir`; return Claudine's arguments.
    pub fn prepare(self, home: &Path, bin_dir: &Path, dir: &Path) -> Vec<String> {
        write_one_line_claude(bin_dir);
        let actions = match self {
            Self::Wrapper => serde_json::json!({
                "actions": { "turn_complete": [{ "type": "message", "message": MESSAGE }] },
            }),
            _ => serde_json::json!({}),
        };
        write_config_with_webhook_route(home, actions);

        let (subcommand, document) = match self {
            Self::Compose => (
                "compose",
                format!("---\nsuccess:\n  message: \"{MESSAGE}\"\n---\nSay hello.\n"),
            ),
            // The stub leaves the body unchanged, so the completion verdict is
            // `failure`; `finalize` fires either way.
            Self::InlineCompose => (
                "inline-compose",
                format!(
                    "---\nprompt: Write a greeting.\nfinalize:\n  message: \"{MESSAGE}\"\n---\nBody.\n"
                ),
            ),
            Self::Sequence => (
                "sequence",
                format!(
                    "---\nsequence:\n  - only\nsuccess:\n  message: \"{MESSAGE}\"\n---\nRun step {{{{ state.name }}}}\n"
                ),
            ),
            Self::Wrapper => return vec!["claude".to_string(), "Say hello.".to_string()],
        };
        let path = dir.join("drain-interrupt.md");
        write(&path, &document);
        vec![
            subcommand.to_string(),
            "--claude".to_string(),
            path.to_str().expect("UTF-8 path").to_string(),
        ]
    }

    /// Text on stderr showing the command's own work is done and the drain
    /// has begun.
    pub fn finished_marker(self) -> &'static str {
        match self {
            Self::Sequence => "Sequence finished",
            Self::Compose | Self::InlineCompose | Self::Wrapper => "no tool calls",
        }
    }

    pub fn first_press_notice(self) -> &'static str {
        match self {
            Self::Compose | Self::InlineCompose => COMPOSE_NOTICE,
            Self::Sequence | Self::Wrapper => DRAIN_NOTICE,
        }
    }
}

/// The scenario typed into a terminal pane.
#[cfg(unix)]
pub mod pane {
    use std::sync::atomic::{AtomicU32, Ordering};
    use std::time::{Duration, Instant};

    use biscuit_test_harness::TerminalHarness;

    use super::super::webhook_listener::{
        ListenerMode, NO_PROXY_LOOPBACK, PROXY_VARIABLES, WEBHOOK_URL_ENV, WebhookListener,
    };
    use super::super::{claudine_bin, sh_quote, wait_for_exit_marker};
    use super::{DrainCommand, PENDING_REPORT};

    /// Bound on startup, the provider run, and the message reaching the
    /// listener.
    const READY_TIMEOUT: Duration = Duration::from_secs(60);

    /// Bound on a press's notice being drawn, including the injector's own
    /// latency.
    const ACK_TIMEOUT: Duration = Duration::from_secs(10);

    /// Pause between seeing the command finish and the first press, so the
    /// press lands in the drain rather than in the command's last steps.
    const SETTLE: Duration = Duration::from_millis(500);

    /// How long the drain must keep running after the first press is seen.
    const STILL_DRAINING: Duration = Duration::from_secs(1);

    /// The second press must end the process within this. By then about 8 s
    /// of the 10 s drain remain, so a press that did not force the exit
    /// fails here. Covers the injector, the exit, and one pane poll.
    const FORCE_EXIT_WITHIN: Duration = Duration::from_secs(3);

    /// Bound on the shell printing the exit status once it is expected.
    const EXIT_TIMEOUT: Duration = Duration::from_secs(20);

    const PANE_POLL: Duration = Duration::from_millis(20);

    /// The pane text of the force-exit notice.
    const FORCE_EXIT_TEXT: &str = "force-exiting";

    /// Type `command` into the pane's shell against a listener that never
    /// replies, press twice during the drain, and check each press.
    ///
    /// `press` delivers one Ctrl+C to the pane. The pane must be wide enough
    /// that the notices do not wrap (120 columns or more).
    ///
    /// ## Panics
    ///
    /// When any step misses its bound, the first press ends the drain, the
    /// second does not exit `130` at once, or the drain's report is printed.
    pub fn assert_second_press_during_the_drain_exits_130<H: TerminalHarness>(
        harness: &mut H,
        command: DrainCommand,
        mut press: impl FnMut(&mut H),
    ) {
        static SEQ: AtomicU32 = AtomicU32::new(0);
        let workspace = tempfile::tempdir().expect("workspace");
        let root = workspace.path();
        let bin_dir = root.join("bin");
        let args = command.prepare(root, &bin_dir, root);
        let listener = WebhookListener::start(ListenerMode::NeverReply);
        let marker = format!(
            "DRAIN_CTRL_C_{}_{}",
            std::process::id(),
            SEQ.fetch_add(1, Ordering::Relaxed)
        );

        // Separate lines keep the typed command short: an inline `PATH=` with
        // the whole system path wraps across many rows.
        for line in [
            format!("cd {}", sh_quote(&root.display().to_string())),
            format!(
                "export PATH={}:\"$PATH\"",
                sh_quote(&bin_dir.display().to_string())
            ),
            format!("unset {}", PROXY_VARIABLES.join(" ")),
        ] {
            harness
                .send_command_with_env(&line, &[])
                .expect("prepare the pane shell");
        }
        let home = root.display().to_string();
        let spool = root.join("playa-spool").display().to_string();
        let url = listener.webhook_url();
        let env = [
            ("HOME", home.as_str()),
            ("NO_COLOR", "1"),
            ("CLAUDINE_RENDEZVOUS_REPORT", "false"),
            ("PLAYA_DRY_RUN", "1"),
            ("PLAYA_SPOOL_DIR", spool.as_str()),
            (WEBHOOK_URL_ENV, url.as_str()),
            ("NO_PROXY", NO_PROXY_LOOPBACK),
        ];
        let quoted: Vec<String> = args.iter().map(|arg| sh_quote(arg)).collect();
        // The status goes on a fresh line: a notice may end without one, and
        // the marker is only matched at the start of a row.
        harness
            .send_command_with_env(
                &format!(
                    "{} {} ; printf '\\n%s:%s\\n' {marker} $?",
                    sh_quote(claudine_bin()),
                    quoted.join(" ")
                ),
                &env,
            )
            .expect("type the command");

        if listener.wait_for_request(READY_TIMEOUT).is_none() {
            panic!("the message was never sent; pane:\n{}", pane_text(harness));
        }
        wait_for_pane(harness, command.finished_marker(), READY_TIMEOUT);
        std::thread::sleep(SETTLE);

        press(harness);
        let noticed = wait_for_pane(harness, command.first_press_notice(), ACK_TIMEOUT);
        let status_prefix = format!("{marker}:");
        while Instant::now() < noticed + STILL_DRAINING {
            let pane = pane_text(harness);
            assert!(
                !pane
                    .lines()
                    .any(|line| line.trim().starts_with(&status_prefix)),
                "the first press ended the drain; pane:\n{pane}"
            );
            std::thread::sleep(PANE_POLL);
        }

        let second = Instant::now();
        press(harness);
        let (frame, status) = wait_for_exit_marker(harness, &marker, EXIT_TIMEOUT);
        let waited = second.elapsed();
        let pane = frame.plain;
        assert_eq!(status, "130", "the second press force-exits; pane:\n{pane}");
        assert!(
            waited < FORCE_EXIT_WITHIN,
            "the second press exits at once, not at the end of the drain ({waited:?}); pane:\n{pane}"
        );
        assert!(
            pane.contains(FORCE_EXIT_TEXT),
            "the forced exit prints its notice; pane:\n{pane}"
        );
        assert!(
            !pane.contains(PENDING_REPORT),
            "a forced exit skips the drain's report; pane:\n{pane}"
        );
    }

    fn pane_text(harness: &mut impl TerminalHarness) -> String {
        harness
            .capture()
            .map(|frame| frame.plain)
            .unwrap_or_default()
    }

    /// Poll the pane until it shows `needle`; return when it was seen.
    fn wait_for_pane(
        harness: &mut impl TerminalHarness,
        needle: &str,
        timeout: Duration,
    ) -> Instant {
        let deadline = Instant::now() + timeout;
        loop {
            let pane = pane_text(harness);
            if pane.contains(needle) {
                return Instant::now();
            }
            assert!(
                Instant::now() < deadline,
                "the pane did not show {needle:?} within {timeout:?}; pane:\n{pane}"
            );
            std::thread::sleep(PANE_POLL);
        }
    }
}
