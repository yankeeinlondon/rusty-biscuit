//! Integration tests for the `choose-one` and `choose-many`
//! subcommands.
//!
//! The **authoritative green-path coverage** for the CLI lives in the
//! `run_with_writer` unit tests inside
//! `biscuit-tui/cli/src/commands/choose_one.rs` and
//! `biscuit-tui/cli/src/commands/choose_many.rs`. Those tests drive the
//! subcommand through its `Writer`-seam entry point with synthetic
//! state so they can assert the complete happy path (parsed args →
//! resolved options → prompt state → serialized output) without
//! spawning a process or attaching a TTY.
//!
//! This file complements the in-crate unit tests with three layers:
//!
//! 1. **Clap-level parsing regressions** — the `question ... --help`
//!    output, flag-conflict diagnostics, and value-parser rejections.
//! 2. **Source-resolution smoke tests** — the process-level stdin and
//!    positional-argv entry points. The event loop itself exits with
//!    code `1` because `assert_cmd` does not attach a real TTY; we
//!    lean on that "reached the event loop" signal as a proxy for
//!    "parsing and source resolution worked".
//! 3. **PTY flows** — the interactive keystroke tests under `mod pty`
//!    spawn the binary under a real PTY via `expectrl` and are gated
//!    behind `QUESTION_INTERACTIVE_PTY=1` so CI runs without a
//!    controlling terminal skip them by default.

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
            "--csv",
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
fn choose_one_help_lists_filter_opt_out() {
    cargo_bin_cmd!("question")
        .args(["choose-one", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("--no-filter"));
}

#[test]
fn choose_many_help_lists_filter_opt_out() {
    cargo_bin_cmd!("question")
        .args(["choose-many", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("--no-filter"));
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

// Phase 4: `--sort` exposes `inverse` as the canonical clap value.
// `reverse` remains accepted as a hidden alias for backward
// compatibility but must not appear in `--help` or completions.

#[test]
fn sort_inverse_is_accepted_and_reaches_event_loop() {
    // The CLI must accept `--sort inverse` as the canonical value and
    // pass it through to the event loop. Without a TTY the loop bails
    // out with code 1 — that signal is enough to confirm clap parsed
    // the value successfully.
    cargo_bin_cmd!("question")
        .args(["choose-one", "Alpha", "Beta", "Gamma", "--sort", "inverse"])
        .assert()
        .failure()
        .code(1)
        .stderr(predicate::str::contains("question:"));
}

#[test]
fn sort_inverse_is_accepted_for_choose_many() {
    cargo_bin_cmd!("question")
        .args(["choose-many", "a", "b", "c", "--sort", "inverse"])
        .assert()
        .failure()
        .code(1)
        .stderr(predicate::str::contains("question:"));
}

#[test]
fn sort_reverse_is_a_hidden_alias_still_accepted() {
    // `reverse` is a hidden compatibility alias for `inverse`. Clap
    // must accept it without an "invalid value" diagnostic — we assert
    // the failure mode is the no-TTY exit (code 1) rather than a clap
    // parse failure.
    cargo_bin_cmd!("question")
        .args(["choose-one", "Alpha", "Beta", "Gamma", "--sort", "reverse"])
        .assert()
        .failure()
        .code(1)
        .stderr(predicate::str::contains("question:"))
        .stderr(predicate::str::contains("invalid value").not());
}

#[test]
fn sort_rejects_unknown_value() {
    cargo_bin_cmd!("question")
        .args(["choose-one", "a", "b", "--sort", "wonky"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("invalid value"));
}

#[test]
fn choose_one_help_lists_inverse_not_reverse_for_sort() {
    // The clap-rendered `--help` for `--sort` must list `inverse` as a
    // canonical value. `reverse` is a hidden alias so it must not
    // appear anywhere in the long-form possible-values list. clap's
    // long-help renders the values as `<value>: <doc>` bullets, so we
    // anchor on `inverse:` to confirm `inverse` is the canonical
    // variant the user sees.
    cargo_bin_cmd!("question")
        .args(["choose-one", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("inverse:"))
        .stdout(predicate::str::contains("natural:"))
        .stdout(predicate::str::contains("asc:"))
        .stdout(predicate::str::contains("desc:"))
        .stdout(predicate::str::contains("reverse").not());
}

#[test]
fn choose_many_help_lists_inverse_not_reverse_for_sort() {
    cargo_bin_cmd!("question")
        .args(["choose-many", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("inverse:"))
        .stdout(predicate::str::contains("natural:"))
        .stdout(predicate::str::contains("asc:"))
        .stdout(predicate::str::contains("desc:"))
        .stdout(predicate::str::contains("reverse").not());
}

// Phase 5: `--active-color` exposes the four `ActiveChoiceColor`
// variants. Default is `grey`; the renderer maps each to a palette
// tuned for dark / light / unknown terminals.

#[test]
fn cli_accepts_active_color_grey() {
    cargo_bin_cmd!("question")
        .args(["choose-one", "Alpha", "Beta", "--active-color", "grey"])
        .assert()
        .failure()
        .code(1)
        .stderr(predicate::str::contains("question:"));
}

#[test]
fn cli_accepts_active_color_green() {
    cargo_bin_cmd!("question")
        .args(["choose-one", "Alpha", "Beta", "--active-color", "green"])
        .assert()
        .failure()
        .code(1);
}

#[test]
fn cli_accepts_active_color_yellow() {
    cargo_bin_cmd!("question")
        .args(["choose-one", "Alpha", "Beta", "--active-color", "yellow"])
        .assert()
        .failure()
        .code(1);
}

#[test]
fn cli_accepts_active_color_red() {
    cargo_bin_cmd!("question")
        .args(["choose-many", "Alpha", "Beta", "--active-color", "red"])
        .assert()
        .failure()
        .code(1);
}

#[test]
fn cli_active_color_rejects_unknown_value() {
    cargo_bin_cmd!("question")
        .args(["choose-one", "a", "b", "--active-color", "purple"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("invalid value"));
}

#[test]
fn choose_one_help_lists_active_color_values() {
    cargo_bin_cmd!("question")
        .args(["choose-one", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("--active-color"))
        .stdout(predicate::str::contains("grey:"))
        .stdout(predicate::str::contains("green:"))
        .stdout(predicate::str::contains("yellow:"))
        .stdout(predicate::str::contains("red:"));
}

#[test]
fn choose_many_help_lists_active_color_values() {
    cargo_bin_cmd!("question")
        .args(["choose-many", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("--active-color"))
        .stdout(predicate::str::contains("grey:"))
        .stdout(predicate::str::contains("green:"));
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
        .args(["choose-one", "a", "b", "c", "--margin", "2", "--mt", "0"])
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

// --- Phase 3: object-record source smoke tests ----------------------------

fn write_fixture(name: &str, body: &str) -> std::path::PathBuf {
    let path = std::env::temp_dir().join(name);
    std::fs::write(&path, body).expect("write fixture");
    path
}

#[test]
fn choose_one_file_json_object_array_reaches_event_loop() {
    // Object-shaped JSON file with `value` and `hotkey` parses without
    // error; the event loop is reached (exit 1 because no TTY).
    let path = write_fixture(
        "choose_cli_phase3_one.json",
        r#"[{"label":"Red","value":"apple","hotkey":"CTRL+R"},{"label":"Blue","value":"sky"}]"#,
    );
    cargo_bin_cmd!("question")
        .args(["choose-one", "--file", path.to_string_lossy().as_ref()])
        .assert()
        .failure()
        .code(1)
        .stderr(predicate::str::contains("question:"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn choose_one_file_yaml_object_array_reaches_event_loop() {
    let path = write_fixture(
        "choose_cli_phase3_one.yaml",
        "- label: Red\n  value: apple\n  hotkey: CTRL+R\n- label: Blue\n  value: sky\n",
    );
    cargo_bin_cmd!("question")
        .args(["choose-one", "--file", path.to_string_lossy().as_ref()])
        .assert()
        .failure()
        .code(1)
        .stderr(predicate::str::contains("question:"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn choose_one_file_toml_options_array_reaches_event_loop() {
    let path = write_fixture(
        "choose_cli_phase2_one.toml",
        "options = [{ label = \"Red\", value = \"apple\", hotkey = \"CTRL+R\" }, { label = \"Blue\", value = \"sky\" }]\n",
    );
    cargo_bin_cmd!("question")
        .args(["choose-one", "--file", path.to_string_lossy().as_ref()])
        .assert()
        .failure()
        .code(1)
        .stderr(predicate::str::contains("question:"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn choose_one_file_csv_three_columns_reaches_event_loop() {
    let path = write_fixture(
        "choose_cli_phase3_one.csv",
        "Red,apple,CTRL+R\nBlue,sky,ALT+B\n",
    );
    cargo_bin_cmd!("question")
        .args(["choose-one", "--file", path.to_string_lossy().as_ref()])
        .assert()
        .failure()
        .code(1)
        .stderr(predicate::str::contains("question:"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn choose_one_file_jsonl_object_lines_reaches_event_loop() {
    let path = write_fixture(
        "choose_cli_phase3_one.jsonl",
        "{\"label\":\"Red\",\"value\":\"apple\",\"hotkey\":\"CTRL+R\"}\n{\"label\":\"Blue\",\"value\":\"sky\"}\n",
    );
    cargo_bin_cmd!("question")
        .args(["choose-one", "--file", path.to_string_lossy().as_ref()])
        .assert()
        .failure()
        .code(1)
        .stderr(predicate::str::contains("question:"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn choose_one_file_ndjson_object_lines_reaches_event_loop() {
    let path = write_fixture(
        "choose_cli_phase3_one.ndjson",
        "{\"label\":\"Red\",\"value\":\"apple\",\"hotkey\":\"CTRL+R\"}\n",
    );
    cargo_bin_cmd!("question")
        .args(["choose-one", "--file", path.to_string_lossy().as_ref()])
        .assert()
        .failure()
        .code(1)
        .stderr(predicate::str::contains("question:"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn choose_many_file_json_object_array_reaches_event_loop() {
    let path = write_fixture(
        "choose_cli_phase3_many.json",
        r#"[{"label":"Red","value":"apple"},{"label":"Blue","value":"sky"}]"#,
    );
    cargo_bin_cmd!("question")
        .args(["choose-many", "--file", path.to_string_lossy().as_ref()])
        .assert()
        .failure()
        .code(1)
        .stderr(predicate::str::contains("question:"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn choose_one_md_frontmatter_object_array_reaches_event_loop() {
    let path = write_fixture(
        "choose_cli_phase3_one.md",
        "---\nitems:\n  - label: Red\n    value: apple\n    hotkey: CTRL+R\n  - label: Blue\n    value: sky\n---\n",
    );
    cargo_bin_cmd!("question")
        .args([
            "choose-one",
            "--md",
            path.to_string_lossy().as_ref(),
            "items",
        ])
        .assert()
        .failure()
        .code(1)
        .stderr(predicate::str::contains("question:"));
    let _ = std::fs::remove_file(&path);
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
    use std::io::Write;
    use std::process::Command;
    use std::time::{Duration, Instant};

    use expectrl::{
        Session,
        process::unix::{Signal, WaitStatus},
        session::OsSession,
    };

    fn interactive_enabled() -> bool {
        std::env::var_os("QUESTION_INTERACTIVE_PTY").is_some()
    }

    fn spawn_question(args: &[&str]) -> OsSession {
        let binary = assert_cmd::cargo::cargo_bin("question");
        let mut command = Command::new(binary);
        command.args(args);
        let mut p = Session::spawn(command).expect("spawn question under PTY");
        p.set_expect_timeout(Some(Duration::from_secs(5)));
        p
    }

    /// Polls for child exit with a hard deadline so a wedged PTY can
    /// never deadlock the test runner.
    fn wait_exit_code(session: &mut OsSession) -> i32 {
        wait_exit_code_within(session, Duration::from_secs(5))
    }

    fn wait_exit_code_within(session: &mut OsSession, timeout: Duration) -> i32 {
        let deadline = Instant::now() + timeout;
        let mut scratch = [0u8; 4096];
        loop {
            // The child's `restore_terminal` writes a few CSI sequences
            // (pop kitty flags, leave alt screen, show cursor) just
            // before exit. Those writes block the slave's stdout if the
            // PTY master buffer is full, which prevents the process
            // from ever reaching `exit`. Drain the master FD on every
            // poll iteration so the child can flush and exit cleanly.
            loop {
                match session.try_read(&mut scratch) {
                    Ok(0) => break,
                    Ok(_) => continue,
                    Err(_) => break,
                }
            }
            match session.get_process().status() {
                Ok(WaitStatus::Exited(_, code)) => return code,
                Ok(WaitStatus::Signaled(_, signal, _)) => {
                    return 128 + (signal as i32);
                }
                Ok(WaitStatus::StillAlive) => {
                    if Instant::now() >= deadline {
                        // Send SIGKILL and poll non-blockingly for the
                        // exit status. We deliberately avoid the
                        // blocking `wait()` here because on some
                        // platforms `waitpid` can deadlock against the
                        // open PTY master FD until it is dropped.
                        let _ = session.get_process_mut().kill(Signal::SIGKILL);
                        let reap_deadline = Instant::now() + Duration::from_millis(500);
                        while Instant::now() < reap_deadline {
                            if !matches!(
                                session.get_process().status(),
                                Ok(WaitStatus::StillAlive)
                            ) {
                                break;
                            }
                            std::thread::sleep(Duration::from_millis(10));
                        }
                        panic!("child did not exit within {timeout:?}");
                    }
                    std::thread::sleep(Duration::from_millis(20));
                }
                Ok(other) => panic!("unexpected wait status: {other:?}"),
                Err(e) => panic!("wait failed: {e}"),
            }
        }
    }

    /// Drains the PTY's master side until the child exits or the
    /// deadline elapses — whichever comes first. Returns whatever bytes
    /// were read so far. Replaces the previous unbounded `loop { read }`
    /// which deadlocked when a child exited without the master FD ever
    /// reporting EOF.
    fn drain_until_exit(session: &mut OsSession, timeout: Duration) -> Vec<u8> {
        let mut buf = Vec::new();
        let mut scratch = [0u8; 1024];
        let deadline = Instant::now() + timeout;
        loop {
            match session.try_read(&mut scratch) {
                Ok(0) => break,
                Ok(n) => buf.extend_from_slice(&scratch[..n]),
                Err(e) if e.kind() == std::io::ErrorKind::Interrupted => continue,
                Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {}
                Err(e) if e.kind() == std::io::ErrorKind::TimedOut => {}
                Err(_) => break,
            }
            if !matches!(
                session.get_process().status(),
                Ok(WaitStatus::StillAlive)
            ) {
                // Child exited — one final non-blocking drain, then stop.
                while let Ok(n) = session.try_read(&mut scratch) {
                    if n == 0 {
                        break;
                    }
                    buf.extend_from_slice(&scratch[..n]);
                }
                break;
            }
            if Instant::now() >= deadline {
                break;
            }
            std::thread::sleep(Duration::from_millis(10));
        }
        buf
    }

    #[test]
    fn esc_restores_initial_and_exits_with_code_0() {
        if !interactive_enabled() {
            eprintln!("skipping: set QUESTION_INTERACTIVE_PTY=1 to enable");
            return;
        }
        // Spec (2026-04-28-choose-one-improvements): Esc on `ChooseOne`
        // restores the initial selection and submits with exit code 0.
        // Pre-select `beta` so the test can also confirm the restored
        // value reaches stdout.
        let mut p = spawn_question(&["choose-one", "--selected", "beta", "alpha", "beta", "gamma"]);
        std::thread::sleep(Duration::from_millis(200));
        // Move the hover off the initial selection, then Esc.
        p.write_all(b"\x1b[B").expect("send Down");
        p.write_all(b"\x1b").expect("send Esc");

        let buf = drain_until_exit(&mut p, Duration::from_secs(5));
        let output = String::from_utf8_lossy(&buf).into_owned();
        assert_eq!(
            wait_exit_code(&mut p),
            0,
            "Esc must exit with code 0 by restoring the initial selection",
        );
        // Strip ANSI/cursor noise — just confirm the restored value
        // appears somewhere in the post-prompt stdout drain.
        assert!(
            output.contains("beta"),
            "expected restored initial value 'beta' in stdout, got {output:?}",
        );
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
        assert_eq!(wait_exit_code(&mut p), 130, "Ctrl+C must exit with code 130");
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

        // Bounded drain — never block past the deadline even if the
        // child exits without the master FD reporting EOF.
        let buf = drain_until_exit(&mut p, Duration::from_secs(5));
        let output = String::from_utf8_lossy(&buf).into_owned();
        assert_eq!(wait_exit_code(&mut p), 0, "submit must exit with code 0");
        for value in ["alpha", "beta", "gamma"] {
            assert!(
                output.contains(value),
                "expected {value:?} in stdout, got {output:?}",
            );
        }
    }

    #[test]
    fn choose_one_height_100_percent_runs_end_to_end() {
        if !interactive_enabled() {
            eprintln!("skipping: set QUESTION_INTERACTIVE_PTY=1 to enable");
            return;
        }
        // Covers the inline `--height 100%` geometry path end-to-end.
        // The math-layer test `height_spec_percent_100_resolves_to_term_rows`
        // only exercises the resolver; this smoke test proves the full
        // CLI wiring composes a runnable prompt at the terminal's row
        // count and accepts a fallback-submit-on-active Enter.
        let mut p = spawn_question(&["choose-one", "alpha", "beta", "gamma", "--height", "100%"]);
        std::thread::sleep(Duration::from_millis(200));
        p.write_all(b"\r").expect("send Enter");
        assert_eq!(
            wait_exit_code(&mut p),
            0,
            "--height 100% must submit the active option with code 0",
        );
    }
}
