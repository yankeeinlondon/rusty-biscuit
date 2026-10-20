//! Recorded lifecycle traces of `claudine compose` and `claudine sequence`,
//! captured through the real binary with a fake provider: the Stage 1 baseline
//! of `2026-10-27-darkmatter-lifecycle-ownership`.
//!
//! Each scenario runs the fixture documents in
//! `tests/fixtures/lifecycle_traces/` and compares one ordered channel,
//! `trace.log` in the fixture directory, with `<id>.trace` line for line. These
//! traces record what Claudine does at this revision; a replay of the moved
//! engine must reproduce them unchanged, so a mismatch is never fixed by editing
//! a `.trace` file to match new behavior.
//!
//! ## The trace channel
//!
//! Every writer appends to the same file, so the lines are totally ordered:
//!
//! - `event <name>` / `event <subject>:<name>` is the first item of every
//!   authored stack (`append_line`). A sequence root is `seq`, a group or leg
//!   its `name`, a prompt step `step`, and a proxy target `target`.
//! - `err <code>` is written where `err` is bound, `err.code` when it has one
//!   and `uncoded: <err.msg>` otherwise; `finalize` writes `err none` when
//!   `err` is null. The fixtures keep `err.msg` free of paths.
//! - `state <key>=<value>` snapshots a `set` mutation or a loop value.
//! - `shell <event>` is appended by a lifecycle `shell:` action, so its absence
//!   after a stack's earlier lines is the evidence that a shell was refused.
//! - `task <name> …` is appended by a sequence `shell:` task.
//! - `work <fresh|resume> <status>` is appended by the fake provider before it
//!   exits with its scripted status; `resume` means it was launched with `-r`.
//! - `exit <code>` is appended by the test after the process exits.
//!
//! The module is Unix-only: the provider is `launch_recorder`'s `/bin/sh`
//! stub, and the fixture shells are POSIX.

use std::collections::BTreeSet;
use std::fs;
use std::path::Path;

use crate::common::{CliProcessFixture, launch_recorder, sh_quote};

/// The one file every trace writer appends to, relative to the fixture cwd.
const TRACE_FILE: &str = "trace.log";

/// The fake provider's `system`/`result` stream for session `session-1`.
const INIT_EVENT: &str =
    r#"{"type":"system","subtype":"init","session_id":"session-1","model":"claude-test"}"#;
const RESULT_EVENT: &str = r#"{"type":"result","subtype":"success","result":"done","session_id":"session-1","is_error":false}"#;

struct Scenario {
    id: &'static str,
    /// Exit status of launch `n` (0-based, across every provider launch of the
    /// run); launches past the end repeat the last status.
    provider_exits: &'static [i32],
    args: &'static [&'static str],
    /// `(file name, content)`; the first is the entry document.
    documents: &'static [(&'static str, &'static str)],
    trace: &'static str,
}

/// `behavior` for [`launch_recorder::install`]: append a `work` line, emit the
/// session stream (with a result only on success), exit with the scripted
/// status. Runs after the recorder consumed stdin; `$n` is the launch index.
fn provider_behavior(trace: &Path, exits: &[i32]) -> String {
    let exits = if exits.is_empty() { vec![0] } else { exits.to_vec() };
    let exits = exits.iter().map(i32::to_string).collect::<Vec<_>>().join(" ");
    let trace = sh_quote(&trace.to_string_lossy());
    format!(
        "mode=fresh\n\
         for arg in \"$@\"; do [ \"$arg\" = -r ] && mode=resume; done\n\
         set -- {exits}\n\
         i=0; code=0\n\
         for e in \"$@\"; do code=$e; [ \"$i\" -eq \"$n\" ] && break; i=$((i + 1)); done\n\
         printf 'work %s %s\\n' \"$mode\" \"$code\" >> {trace}\n\
         printf '%s\\n' {init}\n\
         if [ \"$code\" -eq 0 ]; then printf '%s\\n' {result}; fi\n\
         exit \"$code\"",
        init = sh_quote(INIT_EVENT),
        result = sh_quote(RESULT_EVENT),
    )
}

/// The non-comment lines of a `.trace` fixture. Only CRLF is normalized, so a
/// Windows checkout with `autocrlf` compares equal; nothing else is rewritten.
fn expected_lines(trace: &str) -> Vec<&str> {
    trace
        .lines()
        .map(|line| line.strip_suffix('\r').unwrap_or(line))
        .filter(|line| !line.starts_with('#'))
        .collect()
}

fn header<'a>(trace: &'a str, key: &str) -> Option<&'a str> {
    trace
        .lines()
        .find_map(|line| line.strip_suffix('\r').unwrap_or(line).strip_prefix(&format!("# {key}: ")))
}

fn check(scenario: &Scenario) {
    assert_eq!(header(scenario.trace, "scenario"), Some(scenario.id), "trace header names another scenario");
    let entry = scenario.documents[0].0;
    assert_eq!(header(scenario.trace, "doc"), Some(entry), "trace header names another entry document");

    let fixture = CliProcessFixture::named("lifecycle-traces");
    fixture.seed_user_config();
    for (name, content) in scenario.documents {
        fs::write(fixture.cwd().join(name), content).unwrap();
    }
    let trace_path = fixture.cwd().join(TRACE_FILE);
    launch_recorder::install(&fixture, "claude", &provider_behavior(&trace_path, scenario.provider_exits));

    let output = fixture.command().args(scenario.args).arg(entry).output().unwrap();

    let mut observed: Vec<String> =
        fs::read_to_string(&trace_path).unwrap_or_default().lines().map(str::to_owned).collect();
    let code = output.status.code().map_or_else(|| format!("{:?}", output.status), |code| code.to_string());
    observed.push(format!("exit {code}"));
    assert_eq!(
        observed,
        expected_lines(scenario.trace),
        "scenario {}; stderr:\n{}",
        scenario.id,
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(!fixture.audio_spool().exists(), "scenario {} published audio", scenario.id);
}

/// One `#[test]` per scenario. Each fixture path is a literal inside its own
/// `#[test] fn`, which is what lets CI map a fixture edit to that one test.
/// Also emits [`SCENARIOS`] for the fixture-directory guard.
macro_rules! scenarios {
    ($(
        #[test]
        fn $name:ident() {
            id: $id:literal,
            exits: [$($exit:expr),* $(,)?],
            args: [$($arg:literal),* $(,)?],
            documents: [$(($file:literal, $content:expr)),+ $(,)?],
            trace: $trace:expr $(,)?
        }
    )*) => {
        $(
            #[test]
            fn $name() {
                check(&Scenario {
                    id: $id,
                    provider_exits: &[$($exit),*],
                    args: &[$($arg),*],
                    documents: &[$(($file, $content)),+],
                    trace: $trace,
                });
            }
        )*

        const SCENARIOS: &[&str] = &[$($id),*];
    };
}

scenarios! {
    #[test]
    fn success() {
        id: "success",
        exits: [0],
        args: ["compose", "--claude", "--yolo"],
        documents: [("success.md", include_str!("../fixtures/lifecycle_traces/success.md"))],
        trace: include_str!("../fixtures/lifecycle_traces/success.trace"),
    }

    #[test]
    fn blocked_preflight() {
        id: "blocked-preflight",
        exits: [0],
        args: ["compose", "--claude"],
        documents: [("blocked-preflight.md", include_str!("../fixtures/lifecycle_traces/blocked-preflight.md"))],
        trace: include_str!("../fixtures/lifecycle_traces/blocked-preflight.trace"),
    }

    #[test]
    fn blocked_refused_handoff() {
        id: "blocked-refused-handoff",
        exits: [0],
        args: ["compose", "--claude"],
        documents: [("blocked-refused-handoff.md", include_str!("../fixtures/lifecycle_traces/blocked-refused-handoff.md"))],
        trace: include_str!("../fixtures/lifecycle_traces/blocked-refused-handoff.trace"),
    }

    #[test]
    fn catch_initialize() {
        id: "catch-initialize",
        exits: [0],
        args: ["compose", "--claude"],
        documents: [("catch-initialize.md", include_str!("../fixtures/lifecycle_traces/catch-initialize.md"))],
        trace: include_str!("../fixtures/lifecycle_traces/catch-initialize.trace"),
    }

    #[test]
    fn catch_start() {
        id: "catch-start",
        exits: [0],
        args: ["compose", "--claude"],
        documents: [("catch-start.md", include_str!("../fixtures/lifecycle_traces/catch-start.md"))],
        trace: include_str!("../fixtures/lifecycle_traces/catch-start.trace"),
    }

    #[test]
    fn catch_blocked() {
        id: "catch-blocked",
        exits: [0],
        args: ["compose", "--claude"],
        documents: [("catch-blocked.md", include_str!("../fixtures/lifecycle_traces/catch-blocked.md"))],
        trace: include_str!("../fixtures/lifecycle_traces/catch-blocked.trace"),
    }

    #[test]
    fn catch_success() {
        id: "catch-success",
        exits: [0],
        args: ["compose", "--claude"],
        documents: [("catch-success.md", include_str!("../fixtures/lifecycle_traces/catch-success.md"))],
        trace: include_str!("../fixtures/lifecycle_traces/catch-success.trace"),
    }

    #[test]
    fn catch_failure() {
        id: "catch-failure",
        exits: [1],
        args: ["compose", "--claude"],
        documents: [("catch-failure.md", include_str!("../fixtures/lifecycle_traces/catch-failure.md"))],
        trace: include_str!("../fixtures/lifecycle_traces/catch-failure.trace"),
    }

    #[test]
    fn catch_finalize() {
        id: "catch-finalize",
        exits: [0],
        args: ["compose", "--claude"],
        documents: [("catch-finalize.md", include_str!("../fixtures/lifecycle_traces/catch-finalize.md"))],
        trace: include_str!("../fixtures/lifecycle_traces/catch-finalize.trace"),
    }

    #[test]
    fn catch_loop() {
        id: "catch-loop",
        exits: [0],
        args: ["compose", "--claude"],
        documents: [("catch-loop.md", include_str!("../fixtures/lifecycle_traces/catch-loop.md"))],
        trace: include_str!("../fixtures/lifecycle_traces/catch-loop.trace"),
    }

    #[test]
    fn downgrade_success() {
        id: "downgrade-success",
        exits: [0],
        args: ["compose", "--claude"],
        documents: [("downgrade-success.md", include_str!("../fixtures/lifecycle_traces/downgrade-success.md"))],
        trace: include_str!("../fixtures/lifecycle_traces/downgrade-success.trace"),
    }

    #[test]
    fn downgrade_blocked() {
        id: "downgrade-blocked",
        exits: [0],
        args: ["compose", "--claude"],
        documents: [("downgrade-blocked.md", include_str!("../fixtures/lifecycle_traces/downgrade-blocked.md"))],
        trace: include_str!("../fixtures/lifecycle_traces/downgrade-blocked.trace"),
    }

    #[test]
    fn finalize_error() {
        id: "finalize-error",
        exits: [0],
        args: ["compose", "--claude"],
        documents: [("finalize-error.md", include_str!("../fixtures/lifecycle_traces/finalize-error.md"))],
        trace: include_str!("../fixtures/lifecycle_traces/finalize-error.trace"),
    }

    #[test]
    fn retry_admitted() {
        id: "retry-admitted",
        exits: [1, 0],
        args: ["compose", "--claude"],
        documents: [("retry-admitted.md", include_str!("../fixtures/lifecycle_traces/retry-admitted.md"))],
        trace: include_str!("../fixtures/lifecycle_traces/retry-admitted.trace"),
    }

    #[test]
    fn retry_exhausted() {
        id: "retry-exhausted",
        exits: [1],
        args: ["compose", "--claude"],
        documents: [("retry-exhausted.md", include_str!("../fixtures/lifecycle_traces/retry-exhausted.md"))],
        trace: include_str!("../fixtures/lifecycle_traces/retry-exhausted.trace"),
    }

    #[test]
    fn retry_in_finalize() {
        id: "retry-in-finalize",
        exits: [1, 0],
        args: ["compose", "--claude"],
        documents: [("retry-in-finalize.md", include_str!("../fixtures/lifecycle_traces/retry-in-finalize.md"))],
        trace: include_str!("../fixtures/lifecycle_traces/retry-in-finalize.trace"),
    }

    #[test]
    fn resume_admitted() {
        id: "resume-admitted",
        exits: [1, 0],
        args: ["compose", "--claude"],
        documents: [("resume-admitted.md", include_str!("../fixtures/lifecycle_traces/resume-admitted.md"))],
        trace: include_str!("../fixtures/lifecycle_traces/resume-admitted.trace"),
    }

    #[test]
    fn resume_no_session() {
        id: "resume-no-session",
        exits: [0],
        args: ["compose", "--claude"],
        documents: [("resume-no-session.md", include_str!("../fixtures/lifecycle_traces/resume-no-session.md"))],
        trace: include_str!("../fixtures/lifecycle_traces/resume-no-session.trace"),
    }

    #[test]
    fn resume_incompatible() {
        id: "resume-incompatible",
        exits: [1, 0],
        args: ["compose", "--claude"],
        documents: [("resume-incompatible.md", include_str!("../fixtures/lifecycle_traces/resume-incompatible.md"))],
        trace: include_str!("../fixtures/lifecycle_traces/resume-incompatible.trace"),
    }

    #[test]
    fn proxy_adopted() {
        id: "proxy-adopted",
        exits: [0],
        args: ["compose", "--claude"],
        documents: [
            ("proxy-adopted.md", include_str!("../fixtures/lifecycle_traces/proxy-adopted.md")),
            ("proxy-adopted.target.md", include_str!("../fixtures/lifecycle_traces/proxy-adopted.target.md")),
        ],
        trace: include_str!("../fixtures/lifecycle_traces/proxy-adopted.trace"),
    }

    #[test]
    fn proxy_target_bootstrap_failed() {
        id: "proxy-target-bootstrap-failed",
        exits: [0],
        args: ["compose", "--claude"],
        documents: [
            ("proxy-target-bootstrap-failed.md", include_str!("../fixtures/lifecycle_traces/proxy-target-bootstrap-failed.md")),
            ("proxy-target-bootstrap-failed.target.md", include_str!("../fixtures/lifecycle_traces/proxy-target-bootstrap-failed.target.md")),
        ],
        trace: include_str!("../fixtures/lifecycle_traces/proxy-target-bootstrap-failed.trace"),
    }

    #[test]
    fn loop_break() {
        id: "loop-break",
        exits: [0],
        args: ["compose", "--claude"],
        documents: [("loop-break.md", include_str!("../fixtures/lifecycle_traces/loop-break.md"))],
        trace: include_str!("../fixtures/lifecycle_traces/loop-break.trace"),
    }

    #[test]
    fn cancelled_with_handler() {
        id: "cancelled-with-handler",
        exits: [],
        args: ["sequence", "--claude", "--yolo"],
        documents: [("cancelled-with-handler.md", include_str!("../fixtures/lifecycle_traces/cancelled-with-handler.md"))],
        trace: include_str!("../fixtures/lifecycle_traces/cancelled-with-handler.trace"),
    }

    #[test]
    fn cancelled_without_handler() {
        id: "cancelled-without-handler",
        exits: [],
        args: ["sequence", "--claude", "--yolo"],
        documents: [("cancelled-without-handler.md", include_str!("../fixtures/lifecycle_traces/cancelled-without-handler.md"))],
        trace: include_str!("../fixtures/lifecycle_traces/cancelled-without-handler.trace"),
    }

    #[test]
    fn cancelled_not_opted_in() {
        id: "cancelled-not-opted-in",
        exits: [],
        args: ["sequence", "--claude", "--yolo"],
        documents: [("cancelled-not-opted-in.md", include_str!("../fixtures/lifecycle_traces/cancelled-not-opted-in.md"))],
        trace: include_str!("../fixtures/lifecycle_traces/cancelled-not-opted-in.trace"),
    }

    #[test]
    fn skip() {
        id: "skip",
        exits: [0],
        args: ["compose", "--claude"],
        documents: [("skip.md", include_str!("../fixtures/lifecycle_traces/skip.md"))],
        trace: include_str!("../fixtures/lifecycle_traces/skip.trace"),
    }

    #[test]
    fn sequence_nested_group() {
        id: "sequence-nested-group",
        exits: [0],
        args: ["sequence", "--claude", "--yolo"],
        documents: [
            ("sequence-nested-group.md", include_str!("../fixtures/lifecycle_traces/sequence-nested-group.md")),
            ("sequence-nested-group.step.md", include_str!("../fixtures/lifecycle_traces/sequence-nested-group.step.md")),
        ],
        trace: include_str!("../fixtures/lifecycle_traces/sequence-nested-group.trace"),
    }

    #[test]
    fn shell_locked_blocked() {
        id: "shell-locked-blocked",
        exits: [0],
        args: ["compose", "--claude", "--yolo"],
        documents: [("shell-locked-blocked.md", include_str!("../fixtures/lifecycle_traces/shell-locked-blocked.md"))],
        trace: include_str!("../fixtures/lifecycle_traces/shell-locked-blocked.trace"),
    }

    #[test]
    fn shell_unlocked_blocked_after_start() {
        id: "shell-unlocked-blocked-after-start",
        exits: [0],
        args: ["compose", "--claude", "--yolo"],
        documents: [("shell-unlocked-blocked-after-start.md", include_str!("../fixtures/lifecycle_traces/shell-unlocked-blocked-after-start.md"))],
        trace: include_str!("../fixtures/lifecycle_traces/shell-unlocked-blocked-after-start.trace"),
    }

    #[test]
    fn shell_relocked_after_proxy() {
        id: "shell-relocked-after-proxy",
        exits: [0],
        args: ["compose", "--claude", "--yolo"],
        documents: [
            ("shell-relocked-after-proxy.md", include_str!("../fixtures/lifecycle_traces/shell-relocked-after-proxy.md")),
            ("shell-relocked-after-proxy.target.md", include_str!("../fixtures/lifecycle_traces/shell-relocked-after-proxy.target.md")),
        ],
        trace: include_str!("../fixtures/lifecycle_traces/shell-relocked-after-proxy.trace"),
    }
}

/// Every `<id>.trace` in the fixture directory has a test and every test has
/// its `.trace`; every other fixture file belongs to a scenario (`<id>.md` or
/// `<id>.<role>.md`).
#[test]
fn every_trace_fixture_has_exactly_one_test() {
    let directory = biscuit_test_harness::manifest_dir!().join("tests/fixtures/lifecycle_traces");
    let files: Vec<String> = fs::read_dir(&directory)
        .unwrap()
        .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    let traces: BTreeSet<&str> = files.iter().filter_map(|name| name.strip_suffix(".trace")).collect();
    let tested: BTreeSet<&str> = SCENARIOS.iter().copied().collect();
    assert_eq!(SCENARIOS.len(), tested.len(), "a scenario id is used by two tests");
    assert_eq!(traces, tested, "trace fixtures and scenario tests differ");
    let orphans: Vec<&String> = files
        .iter()
        .filter(|name| {
            !name.ends_with(".trace")
                && !tested.iter().any(|id| {
                    name.strip_prefix(id)
                        .is_some_and(|rest| rest == ".md" || (rest.starts_with('.') && rest.ends_with(".md")))
                })
        })
        .collect();
    assert!(orphans.is_empty(), "fixture files no scenario runs: {orphans:?}");
}
