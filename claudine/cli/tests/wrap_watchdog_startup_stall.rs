//! Integration tests: `step_timeout` bounds silence from child spawn.
//!
//! **Level 1, every platform.** These drive the real wrapper through a fake
//! provider on the fixture `PATH` and assert typed termination facts — the
//! rendered breach diagnostic and the synthesized `session_end` row's
//! `extra.exit_reason` — never a signal number. Only the fake provider itself
//! is platform-specific ([`write_provider`]); every assertion is shared. The
//! sibling `wrap_watchdog_timeout.rs` is `#![cfg(unix)]` because its `/bin/sh`
//! fixtures are, which is exactly the gap acceptance criterion 11 closes.
//!
//! ## Budget sizing
//!
//! A stall test costs its budget plus up to one watchdog tick plus
//! termination.
//!
//! - `2s` budget — the floor that still separates a stall from fixture
//!   startup. The provider stubs here idle in a sleep loop whose coarsest
//!   granularity is Windows' `ping -n 2` (~1 s), so the budget has to clear
//!   one full loop iteration with margin.
//! - `0.2s` tick (`CLAUDINE_WATCHDOG_INTERVAL`) — pure added latency between
//!   breach and kill.
//! - `0.5s` grace (`CLAUDINE_KILL_GRACE`) — a ceiling on the escalation wait,
//!   not a delay.
//!
//! ## Which deadline the stall tests assert
//!
//! Acceptance criterion 1 bounds *detection and the termination request* by
//! `step_timeout` plus one watchdog interval; the platform termination ladder
//! then owns its own `kill_grace` before SIGKILL. The wrapper's last observable
//! event is therefore the reaped child, at `step_timeout + one watchdog
//! interval + kill_grace` — [`REAP_DEADLINE_MS`].
//!
//! Nothing here is serialized: each test owns its own fixture home, `PATH`,
//! and child, and nextest runs every test in its own process, where an
//! in-process `serial_test` lock would coordinate nothing anyway.
//!
//! No wall-clock `CLAUDINE_TIMEOUT` is set anywhere in this file, and the
//! fixture builder scrubs the whole `CLAUDINE_*` namespace out of the
//! inherited environment. `step_timeout` is therefore the only rule that can
//! end these runs — which is the point.

use std::fs;
use std::time::{Duration, Instant};
mod common;
use common::wrap::*;
use common::{CliProcessFixture, strip_ansi};

/// The silence budget every test below configures (`CLAUDINE_STEP_TIMEOUT`).
const STEP_TIMEOUT_MS: u64 = 2_000;
/// The watchdog cadence every test below configures
/// (`CLAUDINE_WATCHDOG_INTERVAL`).
const WATCHDOG_INTERVAL_MS: u64 = 200;
/// The escalation grace every test below configures (`CLAUDINE_KILL_GRACE`).
const KILL_GRACE_MS: u64 = 500;

/// The ratified deadline for the reaped child. Changing any budget literal in
/// this file without changing these constants fails the stall tests loudly.
const REAP_DEADLINE_MS: u64 = STEP_TIMEOUT_MS + WATCHDOG_INTERVAL_MS + KILL_GRACE_MS;

/// Slack on the in-wrapper measurement: ticker scheduling jitter, the wait
/// loop's own wakeup, and the bounded stream-thread join that runs between the
/// reap and the moment `duration_ms` is stamped.
///
/// Sized from 9 runs on a 16-core Mac, isolated and under the full 6843-test
/// suite: 2116–2416 ms against the 2700 ms deadline. 300 ms leaves roughly
/// 600 ms of real headroom while still failing on a sub-second regression in
/// detection latency.
const IN_WRAPPER_SLACK_MS: u64 = 300;

/// Slack on the wall-clock measurement: everything outside the silence budget —
/// spawning the debug-profile binary, preflight, composition, model discovery,
/// and the summary/lifecycle rendering after the child is gone — plus host
/// contention.
///
/// Observed over the same 9 runs at 149–843 ms; 2 s is roughly 2.4× the worst
/// local sample, for a cold or heavily loaded CI runner.
const PROCESS_SLACK_MS: u64 = 2_000;

/// What the fake provider does once the wrapper starts streaming from it.
///
/// Every variant answers `models` first, because model discovery runs as a
/// separate short-lived invocation before the streaming child is spawned.
#[derive(Debug, Clone, Copy)]
enum ProviderBehavior {
    /// Spawn, write nothing at all, never exit. The 2026-08-31 startup stall.
    SilentForever,
    /// Spawn and emit blank lines forever. Bytes flow, but none of them are
    /// real output, so they must not hold the silence budget open.
    WhitespaceForever,
    /// Emit non-whitespace lines on stderr for longer than the budget, then
    /// exit cleanly. This is the OpenCode bootstrap-log shape.
    ChattyThenExit,
    /// Emit one real stream event and then stall forever without ever
    /// reaching a `step_finish` boundary.
    OneEventThenSilent,
    /// Complete a whole step and exit cleanly, with a pause shorter than the
    /// budget before the exit. The companion that keeps the fix from
    /// collapsing into "always time out".
    HealthyStepThenExit,
}

/// Install the fake `opencode` provider on the fixture `PATH`.
///
/// The two shells are separate scripts rather than one parameterized template
/// because `cmd.exe` has no sub-second sleep and no `printf`; keeping them
/// whole is what makes each one readable.
fn write_provider(fixture: &CliProcessFixture, behavior: ProviderBehavior) {
    #[cfg(unix)]
    {
        let body = match behavior {
            ProviderBehavior::SilentForever => "while :; do /bin/sleep 1; done\n",
            ProviderBehavior::WhitespaceForever => {
                "while :; do printf '\\n'; /bin/sleep 0.4; done\n"
            }
            ProviderBehavior::ChattyThenExit => {
                "i=0\nwhile [ $i -lt 10 ]; do\n  printf 'loading config\\n' >&2\n  /bin/sleep 0.4\n  i=$((i + 1))\ndone\nexit 0\n"
            }
            ProviderBehavior::OneEventThenSilent => {
                "printf '%s\\n' '{\"type\":\"init\",\"session_id\":\"startup-stall\",\"model\":\"test-model\"}'\nwhile :; do /bin/sleep 1; done\n"
            }
            ProviderBehavior::HealthyStepThenExit => {
                "printf '%s\\n' '{\"type\":\"init\",\"session_id\":\"startup-stall\",\"model\":\"test-model\"}'\nprintf '%s\\n' '{\"type\":\"step_start\",\"sessionID\":\"startup-stall\"}'\nprintf '%s\\n' '{\"type\":\"text\",\"text\":\"working\"}'\nprintf '%s\\n' '{\"type\":\"step_finish\",\"sessionID\":\"startup-stall\",\"part\":{\"reason\":\"stop\",\"tokens\":{\"input\":1,\"output\":1,\"total\":2}}}'\n/bin/sleep 1\nexit 0\n"
            }
        };
        let script = format!(
            "#!/bin/sh\nif [ \"$1\" = \"models\" ]; then\n  printf '%s\\n' '[\"test-model\"]'\n  exit 0\nfi\n{body}"
        );
        common::write_executable(&fixture.bin_dir().join("opencode"), &script);
    }
    #[cfg(windows)]
    {
        // `ping -n 2 127.0.0.1` is the console-safe ~1s sleep; `timeout /t`
        // needs an interactive console handle and fails under a piped stdin.
        let body = match behavior {
            ProviderBehavior::SilentForever => ":loop\r\nping -n 2 127.0.0.1 >nul\r\ngoto loop\r\n",
            ProviderBehavior::WhitespaceForever => {
                ":loop\r\necho.\r\nping -n 2 127.0.0.1 >nul\r\ngoto loop\r\n"
            }
            ProviderBehavior::ChattyThenExit => {
                "for /l %%i in (1,1,6) do (\r\n  echo loading config 1>&2\r\n  ping -n 2 127.0.0.1 >nul\r\n)\r\nexit /b 0\r\n"
            }
            ProviderBehavior::OneEventThenSilent => {
                "echo {\"type\":\"init\",\"session_id\":\"startup-stall\",\"model\":\"test-model\"}\r\n:loop\r\nping -n 2 127.0.0.1 >nul\r\ngoto loop\r\n"
            }
            ProviderBehavior::HealthyStepThenExit => {
                "echo {\"type\":\"init\",\"session_id\":\"startup-stall\",\"model\":\"test-model\"}\r\necho {\"type\":\"step_start\",\"sessionID\":\"startup-stall\"}\r\necho {\"type\":\"text\",\"text\":\"working\"}\r\necho {\"type\":\"step_finish\",\"sessionID\":\"startup-stall\",\"part\":{\"reason\":\"stop\",\"tokens\":{\"input\":1,\"output\":1,\"total\":2}}}\r\nping -n 2 127.0.0.1 >nul\r\nexit /b 0\r\n"
            }
        };
        let script = format!(
            "@echo off\r\nif \"%~1\"==\"models\" (\r\n  echo [\"test-model\"]\r\n  exit /b 0\r\n)\r\n{body}"
        );
        common::write(&fixture.bin_dir().join("opencode.cmd"), &script);
    }
}

/// Fixture plus a prompt document, ready to compose.
fn stall_fixture(name: &str, behavior: ProviderBehavior) -> (CliProcessFixture, std::path::PathBuf) {
    let fixture = CliProcessFixture::named(name);
    fixture.seed_user_config();
    let md_file = fixture.cwd().join("test.md");
    fs::write(&md_file, "---\ntitle: startup stall\n---\nHello\n").unwrap();
    write_provider(&fixture, behavior);
    (fixture, md_file)
}

/// `extra.exit_reason` from the last synthesized `session_end` row, if the
/// run produced a log at all.
fn last_exit_reason(fixture: &CliProcessFixture) -> Option<String> {
    let log_path = today_log_path(fixture.home());
    let log = fs::read_to_string(&log_path).ok()?;
    let last = log.lines().last()?.to_string();
    let entry: serde_json::Value = serde_json::from_str(&last).ok()?;
    entry
        .get("extra")
        .and_then(|e| e.get("exit_reason"))
        .and_then(|v| v.as_str())
        .map(str::to_string)
}

/// The wrapper's own spawn-to-reap measurement for the last run.
///
/// These fixtures never reach a provider summary, so `extra.duration_ms` is
/// always the wrapper's `started_at.elapsed()` fallback — stamped once the
/// child has been reaped and the stream threads joined, and anchored on the
/// exact instant the silence rule measures from. That makes it the tightest
/// quantity these tests can observe: it excludes this test's own process
/// startup, composition, and post-run rendering.
fn last_child_lifetime_ms(fixture: &CliProcessFixture) -> Option<u64> {
    let log_path = today_log_path(fixture.home());
    let log = fs::read_to_string(&log_path).ok()?;
    let last = log.lines().last()?.to_string();
    let entry: serde_json::Value = serde_json::from_str(&last).ok()?;
    entry
        .get("extra")
        .and_then(|e| e.get("duration_ms"))
        .and_then(serde_json::Value::as_u64)
}

/// Assert the reap deadline on both quantities a stall test can observe.
///
/// The in-wrapper measurement is the real gate; the wall-clock one exists so a
/// regression that somehow bypasses the summary still cannot hide.
fn assert_reaped_within_deadline(fixture: &CliProcessFixture, elapsed: Duration, plain: &str) {
    let child_ms = last_child_lifetime_ms(fixture)
        .expect("session_end must carry duration_ms, or the deadline goes unasserted");
    assert!(
        child_ms <= REAP_DEADLINE_MS + IN_WRAPPER_SLACK_MS,
        "the child lived {child_ms}ms; the ratified reap deadline is \
         {REAP_DEADLINE_MS}ms (step_timeout + one watchdog interval + kill_grace) \
         plus {IN_WRAPPER_SLACK_MS}ms of scheduling slack; stderr: {plain}"
    );

    let elapsed_ms = u64::try_from(elapsed.as_millis()).unwrap_or(u64::MAX);
    assert!(
        elapsed_ms <= REAP_DEADLINE_MS + PROCESS_SLACK_MS,
        "the whole wrapped run took {elapsed_ms}ms; the ratified reap deadline is \
         {REAP_DEADLINE_MS}ms plus {PROCESS_SLACK_MS}ms for wrapper startup and \
         teardown; in-wrapper measurement was {child_ms}ms; stderr: {plain}"
    );
}

/// Acceptance criterion 1. A child that spawns successfully and then writes
/// nothing has no activity clock at all, so before this fix `step_timeout`
/// had no reference to measure from and only the opt-in wall-clock `timeout`
/// could end the run. None is set here.
///
/// The outer `.timeout(...)` is only a hang backstop, so a genuinely wedged
/// wrapper fails as a timeout instead of blocking the suite; the deadline is
/// asserted by [`assert_reaped_within_deadline`].
#[test]
fn watchdog_startup_stall_terminates_without_a_wall_clock_timeout() {
    let (fixture, md_file) = stall_fixture("startup-silent", ProviderBehavior::SilentForever);

    let started = Instant::now();
    let assert = fixture
        .command()
        .env("OPENCODE_MODEL", "test-model")
        .env("CLAUDINE_STEP_TIMEOUT", "2s")
        .env("CLAUDINE_WATCHDOG_INTERVAL", "0.2s")
        .env("CLAUDINE_KILL_GRACE", "0.5s")
        .args(["compose", "--opencode", md_file.to_str().unwrap()])
        .timeout(Duration::from_secs(60))
        .assert()
        .failure();
    let elapsed = started.elapsed();

    let plain = strip_ansi(&String::from_utf8_lossy(&assert.get_output().stderr));
    assert_reaped_within_deadline(&fixture, elapsed, &plain);
    assert!(
        plain.contains("no output since the wrapped process launched"),
        "the breach must be reported as a startup stall, not a mid-run one; got: {plain}"
    );
    assert!(
        plain.contains("step_timeout"),
        "the breach must name the rule that fired; got: {plain}"
    );
    assert_eq!(
        last_exit_reason(&fixture).as_deref(),
        Some("step_timeout"),
        "session_end must carry error_kind=step_timeout; stderr: {plain}"
    );
}

/// Acceptance criterion 2, positive half. Non-whitespace bytes arriving
/// before any semantic event are real progress and must push the deadline
/// out: the fixture chatters for ~4s against a 2s budget and is not killed.
#[test]
fn watchdog_startup_bytes_move_the_deadline() {
    let (fixture, md_file) = stall_fixture("startup-chatty", ProviderBehavior::ChattyThenExit);

    let assert = fixture
        .command()
        .env("OPENCODE_MODEL", "test-model")
        .env("CLAUDINE_STEP_TIMEOUT", "2s")
        .env("CLAUDINE_WATCHDOG_INTERVAL", "0.2s")
        .env("CLAUDINE_KILL_GRACE", "0.5s")
        .args(["compose", "--opencode", md_file.to_str().unwrap()])
        .timeout(Duration::from_secs(60))
        .assert();

    let plain = strip_ansi(&String::from_utf8_lossy(&assert.get_output().stderr));
    assert!(
        !plain.contains("terminating due to step_timeout"),
        "periodic non-whitespace output must keep the silence rule quiet; got: {plain}"
    );
    assert_ne!(
        last_exit_reason(&fixture).as_deref(),
        Some("step_timeout"),
        "a chattering child must not be recorded as a silence kill; stderr: {plain}"
    );
}

/// Acceptance criterion 2, negative half — and the reason the positive half
/// is not vacuous. Blank lines are bytes, but they are not output: the byte
/// clock ignores them, so the spawn-anchored budget keeps advancing and the
/// breach still reports a startup stall.
#[test]
fn watchdog_startup_whitespace_does_not_move_the_deadline() {
    let (fixture, md_file) =
        stall_fixture("startup-whitespace", ProviderBehavior::WhitespaceForever);

    let assert = fixture
        .command()
        .env("OPENCODE_MODEL", "test-model")
        .env("CLAUDINE_STEP_TIMEOUT", "2s")
        .env("CLAUDINE_WATCHDOG_INTERVAL", "0.2s")
        .env("CLAUDINE_KILL_GRACE", "0.5s")
        .args(["compose", "--opencode", md_file.to_str().unwrap()])
        .timeout(Duration::from_secs(60))
        .assert()
        .failure();

    let plain = strip_ansi(&String::from_utf8_lossy(&assert.get_output().stderr));
    assert!(
        plain.contains("no output since the wrapped process launched"),
        "whitespace-only writes must leave the reference on the spawn instant; got: {plain}"
    );
    assert_eq!(
        last_exit_reason(&fixture).as_deref(),
        Some("step_timeout"),
        "session_end must carry error_kind=step_timeout; stderr: {plain}"
    );
}

/// The survival companion for the case above: the same provider shape, but
/// it completes a step and exits. Without this a fix that simply always
/// timed out would look correct.
#[test]
fn watchdog_opencode_healthy_step_survives() {
    let (fixture, md_file) =
        stall_fixture("startup-opencode-healthy", ProviderBehavior::HealthyStepThenExit);

    let assert = fixture
        .command()
        .env("OPENCODE_MODEL", "test-model")
        .env("CLAUDINE_STEP_TIMEOUT", "2s")
        .env("CLAUDINE_WATCHDOG_INTERVAL", "0.2s")
        .env("CLAUDINE_KILL_GRACE", "0.5s")
        .args(["compose", "--opencode", md_file.to_str().unwrap()])
        .timeout(Duration::from_secs(60))
        .assert()
        .success();

    let plain = strip_ansi(&String::from_utf8_lossy(&assert.get_output().stderr));
    assert!(
        !plain.contains("Agent Error"),
        "a healthy run must not render a watchdog breach; got: {plain}"
    );
    assert_ne!(
        last_exit_reason(&fixture).as_deref(),
        Some("step_timeout"),
        "a healthy run must not be recorded as a silence kill; stderr: {plain}"
    );
}

/// Acceptance criterion 4. OpenCode with activity but no `step_finish` used
/// to be exempt from the silence rule for as long as it liked, because the
/// cold-start guard keyed on a missing `provider_status`. The stall is now
/// bounded, and the diagnostic distinguishes it from the no-output case
/// above.
///
/// "Bounded" is the whole point of the test, so the same reap deadline is
/// asserted here. The silence clock is anchored on the `init` event rather
/// than on spawn, but that event is the fixture's first line, so the deadline
/// is the same one to within the child's own startup.
#[test]
fn watchdog_opencode_stall_before_first_step_is_bounded() {
    let (fixture, md_file) =
        stall_fixture("startup-opencode-cold", ProviderBehavior::OneEventThenSilent);

    let started = Instant::now();
    let assert = fixture
        .command()
        .env("OPENCODE_MODEL", "test-model")
        .env("CLAUDINE_STEP_TIMEOUT", "2s")
        .env("CLAUDINE_WATCHDOG_INTERVAL", "0.2s")
        .env("CLAUDINE_KILL_GRACE", "0.5s")
        .args(["compose", "--opencode", md_file.to_str().unwrap()])
        .timeout(Duration::from_secs(60))
        .assert()
        .failure();
    let elapsed = started.elapsed();

    let plain = strip_ansi(&String::from_utf8_lossy(&assert.get_output().stderr));
    assert_reaped_within_deadline(&fixture, elapsed, &plain);
    assert!(
        plain.contains("no stream activity"),
        "activity did arrive, so the breach is anchored on it, not on launch; got: {plain}"
    );
    assert!(
        plain.contains("never completed a step"),
        "the diagnostic must say the stall preceded the first step boundary; got: {plain}"
    );
    assert_eq!(
        last_exit_reason(&fixture).as_deref(),
        Some("step_timeout"),
        "session_end must carry error_kind=step_timeout; stderr: {plain}"
    );
}
