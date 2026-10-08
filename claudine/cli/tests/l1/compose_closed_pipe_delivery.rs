//! A terminal that refuses Claudine's output with a write error is lost
//! delivery, not a provider failure.
//!
//! **Level 1, Unix.** The shipped `claudine compose --no-interactive --claude`
//! runs against a scripted Claude stream (init, assistant answer, successful
//! result). The test closes the reader of Claudine's stdout pipe, and
//! separately of its stderr pipe, right after spawning, so every write to that
//! stream fails with `EPIPE` (Rust ignores `SIGPIPE`, and Claudine does not
//! restore it). The run still succeeds, and its one `session_end` record
//! carries `output_incomplete`. Gated to Unix because on Windows the
//! `session_end` row does not appear under the fixture home (see
//! `wrap_incomplete_subagents.rs`), so the record half cannot be asserted.

use std::fs;
use std::process::Stdio;

use crate::common;
use common::incomplete_subagents::write_replay_provider;
use common::wrap::today_log_path;
use common::{CliProcessFixture, write};

/// Which of Claudine's own output pipes the test closes.
#[derive(Debug, Clone, Copy)]
enum Closed {
    Neither,
    Stdout,
    Stderr,
}

/// What a run left behind.
struct Run {
    success: bool,
    stdout: String,
    session_ends: Vec<serde_json::Value>,
}

fn run_with(closed: Closed) -> Run {
    let fixture = CliProcessFixture::named("compose-closed-pipe");
    write_replay_provider(
        fixture.bin_dir(),
        &[
            r#"{"type":"system","subtype":"init","session_id":"closed-pipe","model":"claude-opus"}"#
                .to_string(),
            r#"{"type":"assistant","message":{"content":[{"type":"text","text":"done"}]}}"#
                .to_string(),
            r#"{"type":"result","subtype":"success","stop_reason":"end_turn","num_turns":1,"duration_ms":1000}"#
                .to_string(),
        ],
    );
    let md_file = fixture.cwd().join("ask.md");
    write(&md_file, "Say done.\n");

    let mut child = fixture
        .command_std()
        .args(["compose", "--no-interactive", "--claude", md_file.to_str().unwrap()])
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("claudine spawns");
    match closed {
        Closed::Neither => {}
        Closed::Stdout => drop(child.stdout.take()),
        Closed::Stderr => drop(child.stderr.take()),
    }
    let output = child.wait_with_output().expect("claudine exits");

    let log = fs::read_to_string(today_log_path(fixture.home())).unwrap_or_default();
    let session_ends = log
        .lines()
        .filter(|line| line.contains("stream_wrapper_summary"))
        .map(|line| serde_json::from_str::<serde_json::Value>(line).unwrap())
        .collect();
    Run {
        success: output.status.success(),
        stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
        session_ends,
    }
}

/// Control: a terminal that reads everything gets the answer, and the one
/// record carries no `output_incomplete`.
#[test]
fn a_reading_terminal_gets_the_answer_and_a_complete_record() {
    let run = run_with(Closed::Neither);

    assert!(run.success);
    assert!(run.stdout.contains("done"), "{:?}", run.stdout);
    assert_eq!(run.session_ends.len(), 1, "{:#?}", run.session_ends);
    let extra = &run.session_ends[0]["extra"];
    assert_eq!(extra["exit_code"], 0, "{extra}");
    assert!(extra.get("output_incomplete").is_none(), "{extra}");
}

/// A closed stdout or stderr pipe loses the output written to it; the run
/// still succeeds and its one record says delivery was incomplete.
#[test]
fn a_closed_stdout_or_stderr_pipe_is_recorded_as_incomplete_delivery() {
    for closed in [Closed::Stdout, Closed::Stderr] {
        let run = run_with(closed);

        assert!(run.success, "{closed:?}: the provider's success is kept");
        assert_eq!(run.session_ends.len(), 1, "{closed:?}: {:#?}", run.session_ends);
        let extra = &run.session_ends[0]["extra"];
        assert_eq!(extra["exit_code"], 0, "{closed:?}: {extra}");
        let incomplete = &extra["output_incomplete"];
        assert!(
            incomplete["failed_writes"].as_u64() >= Some(1),
            "{closed:?}: {extra}"
        );
    }
}
