//! Level-1 PTY proof that a terminal which does not answer queries costs one
//! bounded exchange per process, and that no unanswered query is repeated.
//!
//! The `query_budget` probe builds two `Terminal`s, asks for the cell size
//! three times, and asks for the cursor position once. Every live query
//! appends a DA1 (`CSI c`) sentinel, so the test reads the library's decisions
//! straight off the master side by counting request bytes. The counts are the
//! gate. The elapsed bound is only a backstop and is generous: before the
//! fix this sequence cost about 3.3 s of timeouts in a silent pty.
//!
//! Level 1: the test manufactures every reply (or none), so it proves the
//! library's query discipline, not any emulator's behavior.

#[cfg(unix)]
use crate::common;

#[cfg(unix)]
mod unix {
    use std::time::Duration;

    use test_toolkit::{Level, expect_level};

    use super::common::pty::{
        DA1_QUERY, DA1_REPLY, OSC10_QUERY, OSC11_QUERY, ProbeAnswer, SILENCE_BUDGET_FOR_TESTS,
        count_occurrences, drive_probe, spawn_with_env,
    };

    const CSI_14T_QUERY: &[u8] = b"\x1b[14t";
    const DSR_QUERY: &[u8] = b"\x1b[6n";

    /// Liveness bound for the whole spawn-to-marker cycle, not a timing gate.
    const PROBE_DEADLINE: Duration = Duration::from_secs(15);

    /// In-process backstop: 4x the 500 ms local silence budget, and still
    /// below the ~3.3 s the unbounded per-query timeouts cost.
    const ELAPSED_CEILING_MS: u64 = 2_000;

    fn pty_available() -> bool {
        std::fs::OpenOptions::new()
            .read(true)
            .write(true)
            .open("/dev/ptmx")
            .is_ok()
    }

    /// `default_budget` clears the harness's widened silence budget so the
    /// probe runs on the library default.
    fn run_probe(answers: &mut [ProbeAnswer], default_budget: bool) -> Vec<u8> {
        let budget = if default_budget {
            ""
        } else {
            SILENCE_BUDGET_FOR_TESTS.1
        };
        let mut session = spawn_with_env(&[
            (SILENCE_BUDGET_FOR_TESTS.0, budget),
            ("PROBE", "query_budget"),
            ("PROBE_TERM_PROGRAM", "WezTerm"),
            // Keep color_mode() off the macOS `defaults` fork.
            ("DARK_MODE", "1"),
        ]);
        drive_probe(&mut session, answers, "query_budget_done", PROBE_DEADLINE)
    }

    fn elapsed_ms(collected: &[u8]) -> u64 {
        let text = String::from_utf8_lossy(collected);
        let key = "query_budget_elapsed_ms=";
        text.lines()
            .find_map(|line| {
                let idx = line.find(key)?;
                line[idx + key.len()..].trim().parse().ok()
            })
            .unwrap_or_else(|| panic!("probe printed no {key:?} line; raw: {text:?}"))
    }

    fn counts(collected: &[u8]) -> [usize; 5] {
        [
            count_occurrences(collected, OSC10_QUERY),
            count_occurrences(collected, OSC11_QUERY),
            count_occurrences(collected, CSI_14T_QUERY),
            count_occurrences(collected, DSR_QUERY),
            count_occurrences(collected, DA1_QUERY),
        ]
    }

    /// A pty that answers nothing is written to once: the batched OSC 10/11
    /// request with its DA1 sentinel. The silence found there stops the
    /// CSI 14 t and DSR queries from being written at all.
    #[test]
    fn silent_terminal_is_queried_once_per_process() {
        expect_level!(Level::L1, pty_available(), "PTY (/dev/ptmx)");

        let collected = run_probe(&mut [], true);

        let raw = String::from_utf8_lossy(&collected);
        assert_eq!(
            counts(&collected),
            [1, 1, 0, 0, 1],
            "expected [OSC10, OSC11, CSI 14t, DSR, DA1] = [1, 1, 0, 0, 1]; raw: {raw:?}"
        );
        let elapsed = elapsed_ms(&collected);
        assert!(
            elapsed < ELAPSED_CEILING_MS,
            "silent terminal cost {elapsed} ms, ceiling {ELAPSED_CEILING_MS} ms; raw: {raw:?}"
        );
    }

    /// A terminal that answers DA1 but none of the queries is asked each
    /// question exactly once, and every unanswered question resolves at the
    /// DA1 reply rather than at a timeout.
    #[test]
    fn da1_only_terminal_gets_each_query_once() {
        expect_level!(Level::L1, pty_available(), "PTY (/dev/ptmx)");

        let collected = run_probe(&mut [ProbeAnswer::every(DA1_QUERY, DA1_REPLY)], false);

        let raw = String::from_utf8_lossy(&collected);
        assert_eq!(
            counts(&collected),
            [1, 1, 1, 1, 3],
            "expected [OSC10, OSC11, CSI 14t, DSR, DA1] = [1, 1, 1, 1, 3]; raw: {raw:?}"
        );
        let elapsed = elapsed_ms(&collected);
        assert!(
            elapsed < ELAPSED_CEILING_MS,
            "DA1-only terminal cost {elapsed} ms, ceiling {ELAPSED_CEILING_MS} ms; raw: {raw:?}"
        );
    }
}

/// The PTY path is Unix-only; Windows has no live terminal queries to bound.
#[cfg(not(unix))]
#[test]
fn query_budget_unsupported_on_this_platform() {
    eprintln!("level1_query_budget: skipped — PTY query evidence is Unix-only");
}
