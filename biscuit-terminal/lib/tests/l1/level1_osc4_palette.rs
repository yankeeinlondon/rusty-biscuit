//! Level-1 PTY proof of the OSC 4 ANSI palette query's discipline: it rides
//! the startup exchange, a silent terminal costs one bounded wait and is
//! never asked again, and no query is written in CI, under a multiplexer, or
//! when neither standard stream is a terminal.
//!
//! The `palette` probe mode reads `osc_queries::ansi_palette()` twice. Every
//! test counts the probe's request bytes on the master side; the replies, when
//! there are any, are manufactured here, so this proves the library's query
//! discipline, not any emulator's behavior.

#[cfg(unix)]
use crate::common;

#[cfg(unix)]
mod unix {
    use std::time::Duration;

    use test_toolkit::{Level, expect_level};

    use super::common::pty::{
        DA1_QUERY, DA1_REPLY, OSC10_QUERY, ProbeAnswer, REDIRECTED_PROBE_EXITED,
        count_occurrences, drive_probe, spawn_redirected_with_env, spawn_unidentified_with_env,
    };

    /// The last of the sixteen palette queries; the manufactured reply is sent
    /// once it appears, so it precedes the DA1 reply as a terminal's would.
    const LAST_PALETTE_QUERY: &[u8] = b"\x1b]4;15;?\x07";

    /// The start of any OSC 4 sequence, query or set.
    const OSC4_INTRO: &[u8] = b"\x1b]4;";

    /// Liveness bound for the whole spawn-to-marker cycle, not a timing gate.
    const PROBE_DEADLINE: Duration = Duration::from_secs(15);

    /// A silence budget well under the 1 s exchange timeout, so the silent
    /// case proves the bounded branch without waiting out a production one.
    const SHORT_SILENCE_MS: u64 = 200;

    /// The startup exchange's timeout (`osc_queries::DEFAULT_TIMEOUT`).
    const EXCHANGE_TIMEOUT_MS: u64 = 1_000;

    fn pty_available() -> bool {
        std::fs::OpenOptions::new()
            .read(true)
            .write(true)
            .open("/dev/ptmx")
            .is_ok()
    }

    /// The probe's environment: a terminal known to answer color queries, no
    /// inherited identity, and the light/dark fork kept off `defaults`.
    fn probe_env<'a>(extra: &[(&'a str, &'a str)]) -> Vec<(&'a str, &'a str)> {
        let mut env = vec![
            ("PROBE", "palette"),
            ("PROBE_TERM_PROGRAM", "WezTerm"),
            ("DARK_MODE", "1"),
        ];
        env.extend_from_slice(extra);
        env
    }

    fn run_probe(extra: &[(&str, &str)], answers: &mut [ProbeAnswer]) -> Vec<u8> {
        let mut session = spawn_unidentified_with_env(&probe_env(extra));
        drive_probe(&mut session, answers, "palette_done", PROBE_DEADLINE)
    }

    /// How many times each slot's `ESC ] 4 ; <i> ; ? BEL` query was written.
    fn slot_query_counts(collected: &[u8]) -> Vec<usize> {
        (0..16)
            .map(|index| count_occurrences(collected, format!("\x1b]4;{index};?\x07").as_bytes()))
            .collect()
    }

    fn value<'a>(output: &'a str, key: &str) -> &'a str {
        output
            .lines()
            .find_map(|line| {
                let at = line.find(key)?;
                Some(line[at + key.len()..].trim())
            })
            .unwrap_or_else(|| panic!("probe printed no {key:?} line; raw: {output:?}"))
    }

    fn palette_slots(output: &str) -> Vec<String> {
        (0..16)
            .map(|index| value(output, &format!("palette[{index}]=")).to_string())
            .collect()
    }

    fn assert_all_unknown(output: &str) {
        let slots = palette_slots(output);
        assert!(
            slots.iter().all(|slot| slot == "None"),
            "every slot must be unknown: {slots:?}; raw: {output:?}"
        );
        assert_eq!(value(output, "palette_stable="), "true", "raw: {output:?}");
    }

    /// AC-P2: a pty that answers nothing is sent the palette queries once.
    /// The first read waits out the silence budget (the bounded branch, not a
    /// reply) and returns well inside the exchange timeout; the second read
    /// writes nothing.
    #[test]
    fn silent_terminal_palette_returns_within_its_timeout_and_is_never_resent() {
        expect_level!(Level::L1, pty_available(), "PTY (/dev/ptmx)");

        let budget = SHORT_SILENCE_MS.to_string();
        let collected = run_probe(&[("BISCUIT_TERMINAL_QUERY_SILENCE_MS", &budget)], &mut []);
        let output = String::from_utf8_lossy(&collected);

        assert_eq!(slot_query_counts(&collected), vec![1; 16], "raw: {output:?}");
        assert_eq!(count_occurrences(&collected, DA1_QUERY), 1, "raw: {output:?}");
        assert_all_unknown(&output);
        let first_ms: u64 = value(&output, "palette_first_ms=")
            .parse()
            .expect("palette_first_ms is a number");
        assert!(
            first_ms >= SHORT_SILENCE_MS,
            "the first read returned in {first_ms} ms, before the {SHORT_SILENCE_MS} ms silence \
             budget, so it did not take the silent branch; raw: {output:?}"
        );
        assert!(
            first_ms < EXCHANGE_TIMEOUT_MS,
            "a silent terminal cost {first_ms} ms, at or past the {EXCHANGE_TIMEOUT_MS} ms \
             exchange timeout; raw: {output:?}"
        );
    }

    /// AC-P2 for a terminal that answers DA1 but not OSC 4: the exchange ends
    /// at the DA1 reply, every slot is unknown, and the per-process cache
    /// (not the silent-terminal memo, which this terminal never trips) keeps
    /// the second read from writing again.
    #[test]
    fn da1_only_terminal_palette_is_unknown_and_never_resent() {
        expect_level!(Level::L1, pty_available(), "PTY (/dev/ptmx)");

        let collected = run_probe(&[], &mut [ProbeAnswer::every(DA1_QUERY, DA1_REPLY)]);
        let output = String::from_utf8_lossy(&collected);

        assert_eq!(slot_query_counts(&collected), vec![1; 16], "raw: {output:?}");
        assert_eq!(count_occurrences(&collected, DA1_QUERY), 1, "raw: {output:?}");
        assert_all_unknown(&output);
    }

    /// AC-P1 end to end and the round-trip join: the palette asked after a
    /// `Terminal` is built comes from that construction's exchange (one DA1,
    /// one query per slot). Replies out of order, with both terminators and
    /// both `rgb:` widths, fill their slots; a malformed one stays unknown.
    #[test]
    fn answering_terminal_palette_rides_the_startup_exchange() {
        expect_level!(Level::L1, pty_available(), "PTY (/dev/ptmx)");

        let mut answers = [
            ProbeAnswer::new(
                LAST_PALETTE_QUERY,
                b"\x1b]4;9;rgb:f1/4c/4c\x1b\\\x1b]4;1;rgb:cdcd/3131/3131\x07\
                  \x1b]4;4;#2472c8\x07\x1b]4;2;rgb:zz/00/00\x07",
            ),
            ProbeAnswer::every(DA1_QUERY, DA1_REPLY),
        ];
        let collected = run_probe(&[("PROBE_PALETTE_AFTER_TERMINAL", "1")], &mut answers);
        let output = String::from_utf8_lossy(&collected);

        assert_eq!(count_occurrences(&collected, OSC10_QUERY), 1, "raw: {output:?}");
        assert_eq!(slot_query_counts(&collected), vec![1; 16], "raw: {output:?}");
        assert_eq!(
            count_occurrences(&collected, DA1_QUERY),
            1,
            "the palette must join the construction's exchange; raw: {output:?}"
        );
        let slots = palette_slots(&output);
        let expected_known = [
            (1, "Some(RgbValue { r: 205, g: 49, b: 49 })"),
            (4, "Some(RgbValue { r: 36, g: 114, b: 200 })"),
            (9, "Some(RgbValue { r: 241, g: 76, b: 76 })"),
        ];
        for (index, slot) in slots.iter().enumerate() {
            let expected = expected_known
                .iter()
                .find(|(known, _)| *known == index)
                .map_or("None", |(_, color)| color);
            assert_eq!(slot, expected, "slot {index}; raw: {output:?}");
        }
        assert_eq!(value(&output, "palette_stable="), "true", "raw: {output:?}");
    }

    /// AC-P5: in CI nothing is written to the terminal.
    #[test]
    fn ci_sends_no_palette_query() {
        expect_level!(Level::L1, pty_available(), "PTY (/dev/ptmx)");

        let collected = run_probe(&[("CI", "true")], &mut []);
        let output = String::from_utf8_lossy(&collected);

        assert_eq!(count_occurrences(&collected, OSC4_INTRO), 0, "raw: {output:?}");
        assert_eq!(count_occurrences(&collected, DA1_QUERY), 0, "raw: {output:?}");
        assert_all_unknown(&output);
    }

    /// R-P7: inside a multiplexer nothing is written to the terminal.
    #[test]
    fn multiplexer_sends_no_palette_query() {
        expect_level!(Level::L1, pty_available(), "PTY (/dev/ptmx)");

        let collected = run_probe(&[("ZELLIJ", "0")], &mut []);
        let output = String::from_utf8_lossy(&collected);

        assert_eq!(count_occurrences(&collected, OSC4_INTRO), 0, "raw: {output:?}");
        assert_all_unknown(&output);
    }

    /// AC-P5: with stdout and stderr redirected to a file, nothing is written
    /// to the controlling terminal, although `/dev/tty` is still the pty.
    #[test]
    fn redirected_output_sends_no_palette_query() {
        expect_level!(Level::L1, pty_available(), "PTY (/dev/ptmx)");

        let dir = tempfile::tempdir().expect("temp dir");
        let output_path = dir.path().join("probe.out");
        let mut session = spawn_redirected_with_env(&probe_env(&[]), &output_path);
        let collected = drive_probe(&mut session, &mut [], REDIRECTED_PROBE_EXITED, PROBE_DEADLINE);
        let output = std::fs::read_to_string(&output_path).expect("probe output file");

        assert!(output.contains("palette_done"), "probe did not finish: {output:?}");
        assert_eq!(
            count_occurrences(&collected, OSC4_INTRO),
            0,
            "raw pty: {:?}",
            String::from_utf8_lossy(&collected)
        );
        assert_eq!(count_occurrences(&collected, DA1_QUERY), 0);
        assert_all_unknown(&output);
    }
}

