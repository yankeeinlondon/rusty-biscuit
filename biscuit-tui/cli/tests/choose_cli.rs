//! Integration tests for the `choose-one` and `choose-many`
//! subcommands that exercise Phase 3 CLI-level behaviours: STDIN
//! sourcing, positional arguments, and the `--delimiter` split.
//!
//! Tests here verify that the CLI's parsing and source-resolution
//! layer reaches the event loop without tripping an arg-validation
//! error. The event loop itself fails with exit code 1 because the
//! assert_cmd harness does not attach a real TTY — we lean on that
//! "reached the event loop" signal as a proxy for "parsing worked".
//!
//! The interactive-keystroke flows (Esc, Ctrl+C, Ctrl+A + submit)
//! spawn the binary under a real PTY via `expectrl` and are gated
//! behind `QUESTION_INTERACTIVE_PTY=1` so CI runs without a
//! controlling terminal skip them by default.

use assert_cmd::cargo::cargo_bin_cmd;
use predicates::prelude::*;

#[test]
fn choose_one_reads_from_stdin() {
    cargo_bin_cmd!("question")
        .args(["choose-one"])
        .write_stdin("alpha\nbeta\ngamma\n")
        .assert()
        .failure()
        .code(1)
        .stderr(predicate::str::contains("question:"));
}

#[test]
fn choose_many_reads_from_stdin() {
    cargo_bin_cmd!("question")
        .args(["choose-many"])
        .write_stdin("alpha\nbeta\ngamma\n")
        .assert()
        .failure()
        .code(1)
        .stderr(predicate::str::contains("question:"));
}

#[test]
fn choose_one_accepts_positional_args() {
    cargo_bin_cmd!("question")
        .args(["choose-one", "alpha", "beta", "gamma"])
        .assert()
        .failure()
        .code(1)
        .stderr(predicate::str::contains("question:"));
}

#[test]
fn choose_many_accepts_positional_args() {
    cargo_bin_cmd!("question")
        .args(["choose-many", "alpha", "beta", "gamma"])
        .assert()
        .failure()
        .code(1)
        .stderr(predicate::str::contains("question:"));
}

#[test]
fn delimiter_separates_label_and_value() {
    cargo_bin_cmd!("question")
        .args(["choose-one", "--delimiter", ":", "Apple:1", "Berry:2"])
        .assert()
        .failure()
        .code(1)
        .stderr(predicate::str::contains("question:"));
}

#[test]
fn choose_one_empty_stdin_errors_with_no_options_message() {
    cargo_bin_cmd!("question")
        .args(["choose-one"])
        .write_stdin("")
        .assert()
        .failure()
        .stderr(predicate::str::contains("no options provided"));
}

#[test]
fn choose_one_selected_and_initial_conflict() {
    cargo_bin_cmd!("question")
        .args([
            "choose-one",
            "--options",
            "a,b",
            "--selected",
            "a",
            "--initial",
            "b",
        ])
        .assert()
        .failure()
        .stderr(predicate::str::contains("cannot be used with"));
}

#[test]
fn choose_one_initial_flag_is_hidden_from_help() {
    cargo_bin_cmd!("question")
        .args(["choose-one", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("--selected"))
        .stdout(predicate::str::contains("--initial").not());
}

#[test]
fn choose_many_initial_flag_is_hidden_from_help() {
    cargo_bin_cmd!("question")
        .args(["choose-many", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("--selected"))
        .stdout(predicate::str::contains("--initial").not());
}

#[test]
fn choose_one_help_lists_border_flags() {
    cargo_bin_cmd!("question")
        .args(["choose-one", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("--border"))
        .stdout(predicate::str::contains("--border-label"))
        .stdout(predicate::str::contains("--border-style"));
}

#[test]
fn choose_many_help_lists_border_flags() {
    cargo_bin_cmd!("question")
        .args(["choose-many", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("--border"))
        .stdout(predicate::str::contains("--border-label"))
        .stdout(predicate::str::contains("--border-style"));
}

#[test]
fn choose_one_border_style_rejects_unknown_value() {
    cargo_bin_cmd!("question")
        .args(["choose-one", "a", "b", "--border-style", "wonky"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("invalid value"));
}

#[test]
fn choose_one_border_flag_reaches_event_loop() {
    // Without a TTY the event loop bails out — we only assert the
    // CLI parsed and accepted `--border`. The exit code is 1 (Esc /
    // ABORTED) because `event::read` fails immediately.
    cargo_bin_cmd!("question")
        .args(["choose-one", "a", "b", "c", "--border"])
        .assert()
        .failure()
        .code(1)
        .stderr(predicate::str::contains("question:"));
}

#[test]
fn choose_one_border_label_reaches_event_loop() {
    cargo_bin_cmd!("question")
        .args(["choose-one", "a", "b", "c", "--border-label", "Pick"])
        .assert()
        .failure()
        .code(1)
        .stderr(predicate::str::contains("question:"));
}

#[test]
fn choose_one_border_style_double_reaches_event_loop() {
    cargo_bin_cmd!("question")
        .args(["choose-one", "a", "b", "c", "--border-style", "double"])
        .assert()
        .failure()
        .code(1)
        .stderr(predicate::str::contains("question:"));
}

#[test]
fn choose_many_border_flag_reaches_event_loop() {
    cargo_bin_cmd!("question")
        .args(["choose-many", "a", "b", "c", "--border"])
        .assert()
        .failure()
        .code(1)
        .stderr(predicate::str::contains("question:"));
}

#[test]
fn choose_one_help_lists_margin_flags() {
    cargo_bin_cmd!("question")
        .args(["choose-one", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("--margin"))
        .stdout(predicate::str::contains("--mt"))
        .stdout(predicate::str::contains("--mb"))
        .stdout(predicate::str::contains("--ml"))
        .stdout(predicate::str::contains("--mr"));
}

#[test]
fn choose_many_help_lists_margin_flags() {
    cargo_bin_cmd!("question")
        .args(["choose-many", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("--margin"))
        .stdout(predicate::str::contains("--mt"))
        .stdout(predicate::str::contains("--mb"))
        .stdout(predicate::str::contains("--ml"))
        .stdout(predicate::str::contains("--mr"));
}

#[test]
fn choose_one_margin_rejects_non_integer_value() {
    cargo_bin_cmd!("question")
        .args(["choose-one", "a", "b", "--margin", "wobbly"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("invalid value"));
}

#[test]
fn choose_one_margin_flag_reaches_event_loop() {
    cargo_bin_cmd!("question")
        .args(["choose-one", "a", "b", "c", "--margin", "2"])
        .assert()
        .failure()
        .code(1)
        .stderr(predicate::str::contains("question:"));
}

#[test]
fn choose_one_margin_with_per_side_overrides_reaches_event_loop() {
    cargo_bin_cmd!("question")
        .args([
            "choose-one",
            "a",
            "b",
            "c",
            "--margin",
            "2",
            "--mt",
            "0",
        ])
        .assert()
        .failure()
        .code(1)
        .stderr(predicate::str::contains("question:"));
}

#[test]
fn choose_many_margin_flag_reaches_event_loop() {
    cargo_bin_cmd!("question")
        .args(["choose-many", "a", "b", "c", "--margin", "1"])
        .assert()
        .failure()
        .code(1)
        .stderr(predicate::str::contains("question:"));
}

#[test]
fn choose_many_per_side_margin_flags_reach_event_loop() {
    cargo_bin_cmd!("question")
        .args([
            "choose-many",
            "a",
            "b",
            "--mt",
            "1",
            "--mb",
            "2",
            "--ml",
            "3",
            "--mr",
            "4",
        ])
        .assert()
        .failure()
        .code(1)
        .stderr(predicate::str::contains("question:"));
}

#[test]
fn choose_one_help_lists_height_flag() {
    cargo_bin_cmd!("question")
        .args(["choose-one", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("--height"));
}

#[test]
fn choose_many_help_lists_height_flag() {
    cargo_bin_cmd!("question")
        .args(["choose-many", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("--height"));
}

#[test]
fn choose_one_height_rejects_non_numeric_value() {
    cargo_bin_cmd!("question")
        .args(["choose-one", "a", "b", "--height", "tall"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("invalid"));
}

#[test]
fn choose_one_height_rejects_zero_cells() {
    cargo_bin_cmd!("question")
        .args(["choose-one", "a", "b", "--height", "0"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("greater than 0"));
}

#[test]
fn choose_one_height_rejects_zero_percent() {
    cargo_bin_cmd!("question")
        .args(["choose-one", "a", "b", "--height", "0%"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("between 1 and 100"));
}

#[test]
fn choose_one_height_rejects_percent_above_100() {
    cargo_bin_cmd!("question")
        .args(["choose-one", "a", "b", "--height", "101%"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("between 1 and 100"));
}

#[test]
fn choose_one_height_cells_reaches_event_loop() {
    cargo_bin_cmd!("question")
        .args(["choose-one", "a", "b", "c", "--height", "5"])
        .assert()
        .failure()
        .code(1)
        .stderr(predicate::str::contains("question:"));
}

#[test]
fn choose_one_height_percent_reaches_event_loop() {
    cargo_bin_cmd!("question")
        .args(["choose-one", "a", "b", "c", "--height", "50%"])
        .assert()
        .failure()
        .code(1)
        .stderr(predicate::str::contains("question:"));
}

#[test]
fn choose_many_height_cells_reaches_event_loop() {
    cargo_bin_cmd!("question")
        .args(["choose-many", "a", "b", "c", "--height", "8"])
        .assert()
        .failure()
        .code(1)
        .stderr(predicate::str::contains("question:"));
}

#[test]
fn choose_many_height_percent_reaches_event_loop() {
    cargo_bin_cmd!("question")
        .args(["choose-many", "a", "b", "c", "--height", "25%"])
        .assert()
        .failure()
        .code(1)
        .stderr(predicate::str::contains("question:"));
}

// --- Phase 12: named regressions covering the finished CLI surface. --------

/// Smoke-checks the positional-args path end-to-end.
///
/// Mirrors the `choose-one a b c` invocation from the manual QA
/// checklist: clap should accept the trio, the stdin-fallback branch
/// should be skipped, and the command should reach the event loop
/// (where it then fails with exit code 1 because assert_cmd does not
/// attach a TTY).
#[test]
fn choose_one_positional_args() {
    cargo_bin_cmd!("question")
        .args(["choose-one", "alpha", "beta", "gamma"])
        .assert()
        .failure()
        .code(1)
        .stderr(predicate::str::contains("question:"));
}

// --- PTY-backed keystroke flows --------------------------------------------
//
// These tests spawn `question` under a real PTY so that crossterm can
// open /dev/tty and the event loop actually runs. They are gated
// behind `QUESTION_INTERACTIVE_PTY=1` to keep headless CI green by
// default; run them locally with:
//
//     QUESTION_INTERACTIVE_PTY=1 cargo test -p tui-chrome-cli --test choose_cli
//
// The test harness asserts exit codes, not rendered output, because
// we only care that Esc / Ctrl+C / Ctrl+A are routed through the new
// event-loop plumbing from Phase 2 + Phase 6.

#[cfg(unix)]
mod pty {
    use std::io::{Read, Write};
    use std::time::Duration;

    use expectrl::{process::unix::WaitStatus, session::OsSession, spawn};

    fn interactive_enabled() -> bool {
        std::env::var_os("QUESTION_INTERACTIVE_PTY").is_some()
    }

    fn spawn_question(args: &[&str]) -> OsSession {
        let binary = assert_cmd::cargo::cargo_bin("question");
        let mut cmd = binary.display().to_string();
        for arg in args {
            cmd.push(' ');
            cmd.push('"');
            cmd.push_str(arg);
            cmd.push('"');
        }
        let mut p = spawn(&cmd).expect("spawn question under PTY");
        p.set_expect_timeout(Some(Duration::from_secs(5)));
        p
    }

    fn wait_exit_code(session: &OsSession) -> i32 {
        match session.get_process().wait().expect("wait for child") {
            WaitStatus::Exited(_, code) => code,
            other => panic!("unexpected wait status: {other:?}"),
        }
    }

    #[test]
    fn esc_exits_with_code_1() {
        if !interactive_enabled() {
            eprintln!("skipping: set QUESTION_INTERACTIVE_PTY=1 to enable");
            return;
        }
        let mut p = spawn_question(&["choose-one", "alpha", "beta", "gamma"]);
        std::thread::sleep(Duration::from_millis(200));
        p.write_all(b"\x1b").expect("send Esc");
        assert_eq!(wait_exit_code(&p), 1, "Esc must exit with code 1");
    }

    #[test]
    fn ctrl_c_exits_with_code_130() {
        if !interactive_enabled() {
            eprintln!("skipping: set QUESTION_INTERACTIVE_PTY=1 to enable");
            return;
        }
        let mut p = spawn_question(&["choose-one", "alpha", "beta", "gamma"]);
        std::thread::sleep(Duration::from_millis(200));
        p.write_all(b"\x03").expect("send Ctrl+C");
        assert_eq!(wait_exit_code(&p), 130, "Ctrl+C must exit with code 130");
    }

    #[test]
    fn choose_many_ctrl_a_then_submit_writes_all_values() {
        if !interactive_enabled() {
            eprintln!("skipping: set QUESTION_INTERACTIVE_PTY=1 to enable");
            return;
        }
        let mut p = spawn_question(&["choose-many", "alpha", "beta", "gamma"]);
        std::thread::sleep(Duration::from_millis(200));
        p.write_all(b"\x01").expect("send Ctrl+A");
        p.write_all(b"\r").expect("send Enter");

        let mut buf = Vec::new();
        // Drain the PTY's stdout until the child exits — read returns
        // zero once the master side reports EOF.
        let mut scratch = [0u8; 1024];
        loop {
            match p.read(&mut scratch) {
                Ok(0) => break,
                Ok(n) => buf.extend_from_slice(&scratch[..n]),
                Err(e) if e.kind() == std::io::ErrorKind::Interrupted => continue,
                Err(_) => break,
            }
        }
        let output = String::from_utf8_lossy(&buf).into_owned();
        assert_eq!(wait_exit_code(&p), 0, "submit must exit with code 0");
        for value in ["alpha", "beta", "gamma"] {
            assert!(
                output.contains(value),
                "expected {value:?} in stdout, got {output:?}",
            );
        }
    }
}
