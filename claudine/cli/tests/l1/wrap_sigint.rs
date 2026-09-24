//! Integration tests: SIGINT-during-prep exit-code behavior.
//!
//! Split out of the `wrap_commands.rs` god file; shared fixtures live in
//! `common::wrap`.

use crate::common;
use common::{CliProcessFixture, strip_ansi, write, write_executable};

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
    let child = fixture
        .command_std()
        .env("CLAUDINE_READY_MARKER", &ready_marker)
        .env("CLAUDINE_BACKGROUND_REFRESH", "0")
        .args([
            "compose",
            "--opencode",
            md_file.to_str().unwrap(),
        ])
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .unwrap();

    let pid = child.id() as i32;

    // Poll for the readiness marker rather than sleeping a fixed interval:
    // the marker proves the child reached the cancellable `opencode models`
    // refresh, which is strictly after the SIGINT handler is installed. A
    // fixed sleep was flaky under full-suite contention — slow wrapper
    // startup could push handler installation past the deadline, so SIGINT
    // hit the default disposition (exit 130 but no clean notice).
    let marker_deadline = std::time::Instant::now() + std::time::Duration::from_secs(30);
    while !ready_marker.exists() {
        assert!(
            std::time::Instant::now() < marker_deadline,
            "child never reached the opencode models refresh within 30s"
        );
        std::thread::sleep(std::time::Duration::from_millis(20));
    }
    let interrupt_sent_at = std::time::Instant::now();
    unsafe {
        libc::kill(pid, libc::SIGINT);
    }

    let output = child.wait_with_output().unwrap();
    let interrupt_to_exit = interrupt_sent_at.elapsed();

    // Exit code 130 = 128 + SIGINT(2)
    assert_eq!(
        output.status.code(),
        Some(130),
        "SIGINT during prep must yield exit code 130"
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

    let stderr = String::from_utf8_lossy(&output.stderr);
    let plain = strip_ansi(&stderr);
    assert!(
        plain.contains("User interrupted compose operation"),
        "stderr must contain the clean interrupt notice; got: {plain}"
    );
}

/// Ctrl+C while the wrapper tears down an agent's orphaned descendants still
/// runs the document's `failure` and `finalize` events, promptly.
///
/// The shape is an OpenCode run that exits but leaves a process holding its
/// stdout/stderr pipes. The fake's orphan survives `SIGTERM` (its trap touches
/// the marker this test waits on), so the wrapper sits in the post-exit
/// process-group teardown when the press lands. The kill grace is stretched to
/// 60 s: a teardown that ignores the interrupt blows the latency budget, and
/// one that treats the press as "no wait loop active" lets a repeat press
/// force-exit past `failure`/`finalize`.
#[cfg(unix)]
#[test]
#[serial_test::serial]
fn compose_sigint_during_orphan_teardown_runs_failure_lifecycle() {
    let fixture = CliProcessFixture::named("wrap-sigint-orphan");
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

    let term_marker = fixture.cwd().join("orphan-got-sigterm");
    write_executable(
        &fixture.bin_dir().join("opencode"),
        r#"#!/bin/sh
if [ "$1" = "models" ]; then
  printf '%s\n' '["test-model"]'
  exit 0
fi
(
  trap ': > "$CLAUDINE_TERM_MARKER"' TERM
  while :; do /bin/sleep 0.1; done
) &
printf '%s\n' '{"type":"init","session_id":"orphan","model":"test-model"}'
echo "ProviderModelNotFoundError: Model not found: test/missing." >&2
exit 1
"#,
    );

    // The child must stay alive to receive the signal, so this is the
    // builder's raw-command surface.
    let child = fixture
        .command_std()
        .env("CLAUDINE_TERM_MARKER", &term_marker)
        .env("CLAUDINE_KILL_GRACE", "60s")
        .args(["compose", "--opencode", "-y", md_file.to_str().unwrap()])
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .unwrap();
    let pid = child.id() as i32;

    // The marker is written only when the teardown's group `SIGTERM` reaches
    // the orphan, so the press below lands inside the teardown.
    let marker_deadline = std::time::Instant::now() + std::time::Duration::from_secs(30);
    while !term_marker.exists() {
        assert!(
            std::time::Instant::now() < marker_deadline,
            "the wrapper never began tearing down the orphaned descendant within 30s"
        );
        std::thread::sleep(std::time::Duration::from_millis(20));
    }
    let interrupt_sent_at = std::time::Instant::now();
    unsafe {
        libc::kill(pid, libc::SIGINT);
    }

    let output = child.wait_with_output().unwrap();
    let interrupt_to_exit = interrupt_sent_at.elapsed();
    let plain = strip_ansi(&String::from_utf8_lossy(&output.stderr));

    assert!(
        interrupt_to_exit < std::time::Duration::from_secs(10),
        "Ctrl+C must cut the 60s kill grace short; took {interrupt_to_exit:?}; stderr:\n{plain}"
    );
    assert!(
        !output.status.success(),
        "a failed, interrupted run must not exit 0; stderr:\n{plain}"
    );
    assert!(
        !plain.contains("force-exiting"),
        "the teardown must not force-exit the wrapper; stderr:\n{plain}"
    );
    let events = std::fs::read_to_string(fixture.cwd().join("events.log")).unwrap_or_default();
    assert_eq!(
        events.lines().collect::<Vec<_>>(),
        vec!["failure", "finalize"],
        "the interrupted run must still fire its terminal lifecycle; stderr:\n{plain}"
    );
}

/// Outcome of pressing Ctrl+C twice while a `failure` stack is running.
#[cfg(unix)]
struct TerminalGraceOutcome {
    exit_code: Option<i32>,
    press_to_exit: std::time::Duration,
    events: Vec<String>,
    stderr: String,
}

/// Run a compose whose provider fails and whose `failure` stack spends
/// `failure_sleep` in a shell action, pressing Ctrl+C twice once that stack
/// has started.
#[cfg(unix)]
fn press_twice_during_failure_stack(failure_sleep: &str) -> TerminalGraceOutcome {
    let fixture = CliProcessFixture::named("wrap-sigint-grace");
    fixture.seed_user_config();
    write_executable(&fixture.bin_dir().join("claude"), "#!/bin/sh\nexit 1\n");

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
        - shell: "/bin/sleep {failure_sleep}"
        - append_line: ["events.log", "failure-done"]
finalize:
  stack:
    - action: {{append_line: ["events.log", "finalize"]}}
---
Prompt body
"#
        ),
    );
    let events_log = fixture.cwd().join("events.log");
    // Files rather than pipes: a force-exit leaves the stack's shell child
    // running with the wrapper's stderr, so a piped read would wait on it.
    let stderr_path = fixture.cwd().join("stderr.txt");

    // The child must stay alive to receive the signals, so this is the
    // builder's raw-command surface.
    let mut child = fixture
        .command_std()
        .args(["compose", "--claude", "-y", md_file.to_str().unwrap()])
        .stdout(std::process::Stdio::null())
        .stderr(std::fs::File::create(&stderr_path).unwrap())
        .spawn()
        .unwrap();
    let pid = child.id() as i32;

    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(30);
    while !std::fs::read_to_string(&events_log)
        .unwrap_or_default()
        .contains("failure-started")
    {
        assert!(
            std::time::Instant::now() < deadline,
            "the failure stack never started within 30s"
        );
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
    unsafe {
        libc::kill(pid, libc::SIGINT);
    }
    std::thread::sleep(std::time::Duration::from_millis(50));
    let second_press_at = std::time::Instant::now();
    unsafe {
        libc::kill(pid, libc::SIGINT);
    }

    let status = child.wait().unwrap();
    let press_to_exit = second_press_at.elapsed();
    TerminalGraceOutcome {
        exit_code: status.code(),
        press_to_exit,
        events: std::fs::read_to_string(&events_log)
            .unwrap_or_default()
            .lines()
            .map(str::to_string)
            .collect(),
        stderr: strip_ansi(&std::fs::read_to_string(&stderr_path).unwrap_or_default()),
    }
}

/// A repeat Ctrl+C during `failure` lets the event, and the `finalize` after
/// it, finish when they fit inside the 500 ms grace.
#[cfg(unix)]
#[test]
#[serial_test::serial]
fn compose_second_sigint_lets_a_short_failure_stack_finish() {
    let outcome = press_twice_during_failure_stack("0.2");

    assert_eq!(
        outcome.events,
        vec!["failure-started", "failure-done", "finalize"],
        "the grace must let the in-flight failure stack and finalize complete; stderr:\n{}",
        outcome.stderr
    );
    assert!(
        outcome.stderr.contains("letting the lifecycle event finish"),
        "the second press must announce the grace; stderr:\n{}",
        outcome.stderr
    );
    assert!(
        outcome.press_to_exit < std::time::Duration::from_secs(3),
        "the run must end promptly after the grace; took {:?}",
        outcome.press_to_exit
    );
}

/// A repeat Ctrl+C during a `failure` stack that outlasts the grace
/// force-exits once the 500 ms are up rather than waiting it out.
#[cfg(unix)]
#[test]
#[serial_test::serial]
fn compose_second_sigint_force_exits_a_failure_stack_that_outlasts_the_grace() {
    let outcome = press_twice_during_failure_stack("5");

    assert_eq!(outcome.exit_code, Some(130), "stderr:\n{}", outcome.stderr);
    assert!(
        outcome.press_to_exit >= std::time::Duration::from_millis(400)
            && outcome.press_to_exit < std::time::Duration::from_secs(3),
        "the force-exit must land at the end of the 500ms grace; took {:?}",
        outcome.press_to_exit
    );
    assert!(
        outcome.stderr.contains("lifecycle event still running — force-exiting"),
        "stderr:\n{}",
        outcome.stderr
    );
    assert_eq!(outcome.events, vec!["failure-started"], "stderr:\n{}", outcome.stderr);
}
