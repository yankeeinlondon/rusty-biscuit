//! Level-1 PTY tests for the live graphics probe: the once-per-process
//! startup exchange that joins the OSC 10/11 color queries and the Kitty
//! graphics query under one DA1 sentinel, and the tmux-wrapped exchange sent
//! when `TERM` hints at a tmux this process cannot see.
//!
//! Each test spawns the `discovery_probe` example (`PROBE=graphics`) in a
//! pseudoterminal, reads what the library wrote on the master side, and
//! manufactures the replies, so these prove the request bytes, the caching,
//! and how replies resolve to a route, never how a real terminal answers.
//! `level2_graphics_probe_wezterm.rs` is the real-terminal counterpart. The
//! pure resolution matrices are `graphics_probe_matrix.rs` and
//! `graphics_route_matrix.rs`.
//!
//! The child is spawned with every terminal-identity variable removed
//! (`spawn_unidentified_with_env`), so the host terminal the suite runs in
//! cannot decide the route before the probe does.
//!
//! The Unix-only PTY code is target-gated so Windows still compiles.
//!
//! Run `cargo build -p biscuit-terminal --example discovery_probe` first.

#[cfg(unix)]
use crate::common;

#[cfg(unix)]
mod unix {
    use std::io::Write;
    use std::time::Duration;

    use test_toolkit::{Level, expect_level};

    use super::common::pty::{
        DA1_QUERY, DA1_REPLY, OSC10_QUERY, OSC11_QUERY, ProbeAnswer, count_occurrences,
        drive_probe, kitty_query_ids, spawn_unidentified_with_env,
    };

    const OSC10_REPLY: &[u8] = b"\x1b]10;rgb:1234/5678/9abc\x07";
    const OSC11_REPLY: &[u8] = b"\x1b]11;rgb:1e1e/1e1e/1e1e\x07";
    const TEXT_COLOR: &str = "Some(RgbValue { r: 18, g: 86, b: 154 })";
    const BG_COLOR: &str = "Some(RgbValue { r: 30, g: 30, b: 30 })";

    /// A DA1 reply reporting Sixel (attribute 4).
    const DA1_SIXEL_REPLY: &[u8] = b"\x1b[?62;4;22c";

    /// The tmux-wrapped DA1 sentinel: `ESC P tmux; ESC ESC [ c ESC \`.
    const WRAPPED_DA1: &[u8] = b"\x1bPtmux;\x1b\x1b[c\x1b\\";

    /// Alacritty answers OSC 10/11, and the route cannot identify it, so its
    /// colors and its graphics query share one exchange.
    const ALACRITTY: (&str, &str) = ("PROBE_TERM", "alacritty");
    /// An unidentified terminal the library does not ask for colors.
    const UNIDENTIFIED: (&str, &str) = ("PROBE_TERM", "xterm-256color");

    const DEADLINE: Duration = Duration::from_secs(4);

    fn pty_available() -> bool {
        std::fs::OpenOptions::new()
            .read(true)
            .write(true)
            .open("/dev/ptmx")
            .is_ok()
    }

    fn run(envs: &[(&str, &str)], answers: &mut [ProbeAnswer]) -> Vec<u8> {
        let mut all = vec![("PROBE", "graphics"), ("DARK_MODE", "1")];
        all.extend_from_slice(envs);
        let mut session = spawn_unidentified_with_env(&all);
        drive_probe(&mut session, answers, "graphics_done", DEADLINE)
    }

    /// The value of the probe's `key=` line. Matched anywhere in a line: the
    /// library's query bytes share the PTY with stdout.
    fn field(collected: &[u8], key: &str) -> String {
        let text = String::from_utf8_lossy(collected);
        let needle = format!("{key}=");
        text.lines()
            .find_map(|line| {
                let at = line.find(&needle)?;
                Some(line[at + needle.len()..].trim_end().to_string())
            })
            .unwrap_or_else(|| panic!("probe printed no {needle:?} line; raw: {text:?}"))
    }

    fn raw(collected: &[u8]) -> String {
        format!("{:?}", String::from_utf8_lossy(collected))
    }

    fn position(haystack: &[u8], needle: &[u8]) -> Option<usize> {
        haystack.windows(needle.len()).position(|w| w == needle)
    }

    fn direct_kitty_queries(collected: &[u8]) -> Vec<u32> {
        kitty_query_ids(collected, false)
    }

    /// Answer the colors, the Kitty query, and DA1, in the order a terminal
    /// that supports all three would.
    fn full_answers() -> [ProbeAnswer; 4] {
        [
            ProbeAnswer::new(OSC10_QUERY, OSC10_REPLY),
            ProbeAnswer::new(OSC11_QUERY, OSC11_REPLY),
            ProbeAnswer::kitty("OK"),
            ProbeAnswer::every(DA1_QUERY, DA1_REPLY),
        ]
    }

    /// For a terminal whose colors are queried and whose route is unknown,
    /// the OSC 10, OSC 11, and Kitty queries go out as one contiguous request
    /// ended by the only DA1 sentinel of the construction, and the answers
    /// give a verified Kitty route and the measured colors.
    #[test]
    fn colors_and_kitty_query_share_one_exchange() {
        expect_level!(Level::L1, pty_available(), "PTY (/dev/ptmx)");
        let mut answers = full_answers();
        let collected = run(&[ALACRITTY], &mut answers);

        assert_eq!(
            count_occurrences(&collected, DA1_QUERY),
            1,
            "{}",
            raw(&collected)
        );
        let ids = direct_kitty_queries(&collected);
        assert_eq!(ids.len(), 1, "{}", raw(&collected));

        let mut request = Vec::new();
        request.extend_from_slice(OSC10_QUERY);
        request.extend_from_slice(OSC11_QUERY);
        request.extend_from_slice(
            format!("\x1b_Gi={},s=1,v=1,a=q,t=d,f=24;AAAA\x1b\\", ids[0]).as_bytes(),
        );
        request.extend_from_slice(DA1_QUERY);
        assert!(
            position(&collected, &request).is_some(),
            "expected OSC 10 + OSC 11 + Kitty query + DA1 as one contiguous request; {}",
            raw(&collected)
        );

        assert_eq!(field(&collected, "graphics_format[0]"), "Kitty");
        assert_eq!(field(&collected, "graphics_transport[0]"), "Direct");
        assert_eq!(field(&collected, "graphics_confidence[0]"), "Verified");
        assert_eq!(field(&collected, "graphics_text_color[0]"), TEXT_COLOR);
        assert_eq!(field(&collected, "graphics_bg_color[0]"), BG_COLOR);
        assert_eq!(field(&collected, "graphics_bg_source[0]"), "Measured");
    }

    /// An unidentified terminal outside the color-query list gets the Kitty
    /// query and DA1 only; a DA1 with attribute 4 and no Kitty reply resolves
    /// to verified Sixel.
    #[test]
    fn graphics_only_exchange_reads_sixel_from_da1() {
        expect_level!(Level::L1, pty_available(), "PTY (/dev/ptmx)");
        let mut answers = [ProbeAnswer::every(DA1_QUERY, DA1_SIXEL_REPLY)];
        let collected = run(&[UNIDENTIFIED], &mut answers);

        assert_eq!(
            count_occurrences(&collected, OSC10_QUERY),
            0,
            "{}",
            raw(&collected)
        );
        assert_eq!(
            count_occurrences(&collected, OSC11_QUERY),
            0,
            "{}",
            raw(&collected)
        );
        assert_eq!(
            direct_kitty_queries(&collected).len(),
            1,
            "{}",
            raw(&collected)
        );
        assert_eq!(
            count_occurrences(&collected, DA1_QUERY),
            1,
            "{}",
            raw(&collected)
        );

        assert_eq!(field(&collected, "graphics_format[0]"), "Sixel");
        assert_eq!(field(&collected, "graphics_confidence[0]"), "Verified");
        assert_eq!(
            field(&collected, "graphics_bg_source[0]"),
            "Estimated",
            "no OSC 11 was sent, so any background is a guess"
        );
    }

    /// A queried terminal that answers DA1 but not OSC 11 keeps its per-app
    /// default background, labelled as an estimate, never as measured.
    #[test]
    fn unanswered_background_query_is_an_estimate() {
        expect_level!(Level::L1, pty_available(), "PTY (/dev/ptmx)");
        let mut answers = [ProbeAnswer::every(DA1_QUERY, DA1_REPLY)];
        let collected = run(&[ALACRITTY], &mut answers);

        assert_eq!(count_occurrences(&collected, OSC11_QUERY), 1, "{}", raw(&collected));
        assert_eq!(field(&collected, "graphics_bg_color[0]"), BG_COLOR, "{}", raw(&collected));
        assert_eq!(field(&collected, "graphics_bg_source[0]"), "Estimated", "{}", raw(&collected));
    }

    /// DA1 without attribute 4 and no Kitty reply is a verified negative.
    #[test]
    fn da1_without_kitty_or_sixel_is_verified_no_support() {
        expect_level!(Level::L1, pty_available(), "PTY (/dev/ptmx)");
        let mut answers = [ProbeAnswer::every(DA1_QUERY, DA1_REPLY)];
        let collected = run(&[UNIDENTIFIED], &mut answers);

        assert_eq!(
            direct_kitty_queries(&collected).len(),
            1,
            "{}",
            raw(&collected)
        );
        assert_eq!(field(&collected, "graphics_format[0]"), "None");
        assert_eq!(field(&collected, "graphics_confidence[0]"), "Verified");
        assert_eq!(field(&collected, "graphics_reason_kind[0]"), "NoSupport");
    }

    /// A Kitty error reply for the query's ID is a verified rejection, and
    /// the reason says so.
    #[test]
    fn kitty_error_reply_is_a_reasoned_rejection() {
        expect_level!(Level::L1, pty_available(), "PTY (/dev/ptmx)");
        let mut answers = [
            ProbeAnswer::kitty("ENOTSUPPORTED:graphics disabled"),
            ProbeAnswer::every(DA1_QUERY, DA1_REPLY),
        ];
        let collected = run(&[UNIDENTIFIED], &mut answers);

        assert_eq!(field(&collected, "graphics_format[0]"), "None");
        assert_eq!(field(&collected, "graphics_confidence[0]"), "Verified");
        assert_eq!(field(&collected, "graphics_reason_kind[0]"), "NoSupport");
        let reason = field(&collected, "graphics_reason[0]");
        assert!(
            reason.contains("rejected the Kitty graphics query"),
            "{reason}"
        );
        assert!(reason.contains("ENOTSUPPORTED"), "{reason}");
    }

    /// Spawns the probe with a line typed ahead, and has the probe wait
    /// (without reading) until that line is pending before it constructs.
    ///
    /// The line is written right after spawn; the PTY queues it whether or
    /// not the child has started. `PROBE_AWAIT_TYPEAHEAD_MS` closes the race
    /// the other way: the construction cannot start before the keys arrive,
    /// and the probe reports `typeahead_pending=true` when they did. A whole
    /// line is typed because stdin is canonical until the exchange begins,
    /// and only a complete line makes it readable.
    fn run_with_typeahead(envs: &[(&str, &str)], answers: &mut [ProbeAnswer]) -> Vec<u8> {
        let mut all = vec![
            ("PROBE", "graphics"),
            ("DARK_MODE", "1"),
            ("PROBE_AWAIT_TYPEAHEAD_MS", "2000"),
        ];
        all.extend_from_slice(envs);
        let mut session = spawn_unidentified_with_env(&all);
        session.write_all(b"typed\n").expect("type ahead");
        session.flush().expect("flush typeahead");
        let collected = drive_probe(&mut session, answers, "graphics_done", DEADLINE);
        assert_eq!(
            field(&collected, "typeahead_pending"),
            "true",
            "the typed line never became pending; {}",
            raw(&collected)
        );
        collected
    }

    /// Input waiting at exchange time holds the graphics query back: nothing
    /// is written, the route is unknown with the reason, and the typed line
    /// is still there for the program to read.
    #[test]
    fn waiting_input_skips_the_graphics_query() {
        expect_level!(Level::L1, pty_available(), "PTY (/dev/ptmx)");
        let mut answers = [ProbeAnswer::every(DA1_QUERY, DA1_REPLY)];
        let collected = run_with_typeahead(
            &[UNIDENTIFIED, ("PROBE_READ_TYPEAHEAD_MS", "1000")],
            &mut answers,
        );

        assert!(
            direct_kitty_queries(&collected).is_empty(),
            "{}",
            raw(&collected)
        );
        assert_eq!(
            count_occurrences(&collected, DA1_QUERY),
            0,
            "{}",
            raw(&collected)
        );
        assert_eq!(field(&collected, "graphics_format[0]"), "None");
        assert_eq!(field(&collected, "graphics_confidence[0]"), "Unknown");
        let reason = field(&collected, "graphics_reason[0]");
        assert!(reason.contains("input was waiting"), "{reason}");
        assert_eq!(field(&collected, "typeahead_line"), "Some(\"typed\")");
    }

    /// With colors wanted, waiting input holds back only the graphics query:
    /// the colors are still queried and measured, and no Kitty query is
    /// written in the construction.
    #[test]
    fn waiting_input_still_queries_colors() {
        expect_level!(Level::L1, pty_available(), "PTY (/dev/ptmx)");
        let mut answers = full_answers();
        let collected = run_with_typeahead(&[ALACRITTY], &mut answers);

        assert_eq!(
            count_occurrences(&collected, OSC10_QUERY),
            1,
            "{}",
            raw(&collected)
        );
        assert_eq!(
            count_occurrences(&collected, OSC11_QUERY),
            1,
            "{}",
            raw(&collected)
        );
        assert_eq!(field(&collected, "graphics_text_color[0]"), TEXT_COLOR);
        assert!(
            direct_kitty_queries(&collected).is_empty(),
            "{}",
            raw(&collected)
        );
        assert_eq!(field(&collected, "graphics_confidence[0]"), "Unknown");
        let reason = field(&collected, "graphics_reason[0]");
        assert!(reason.contains("input was waiting"), "{reason}");
    }

    /// `ProbeMode::Off` writes nothing to the terminal, not even for a
    /// terminal whose colors would be queried, and the reason names the
    /// caller.
    #[test]
    fn caller_probe_off_writes_nothing() {
        expect_level!(Level::L1, pty_available(), "PTY (/dev/ptmx)");
        let mut answers = full_answers();
        let collected = run(&[ALACRITTY, ("PROBE_GRAPHICS_PROBE", "off")], &mut answers);

        assert!(
            !collected.contains(&0x1b),
            "expected no escape bytes; {}",
            raw(&collected)
        );
        assert_eq!(field(&collected, "graphics_confidence[0]"), "Unknown");
        let reason = field(&collected, "graphics_reason[0]");
        assert!(reason.contains("the caller turned probing off"), "{reason}");
    }

    /// `BISCUIT_TERMINAL_PROBE=off` writes nothing either, and the reason
    /// names the variable.
    #[test]
    fn env_probe_off_writes_nothing() {
        expect_level!(Level::L1, pty_available(), "PTY (/dev/ptmx)");
        let mut answers = full_answers();
        let collected = run(
            &[ALACRITTY, ("BISCUIT_TERMINAL_PROBE", "off")],
            &mut answers,
        );

        assert!(
            !collected.contains(&0x1b),
            "expected no escape bytes; {}",
            raw(&collected)
        );
        assert_eq!(field(&collected, "graphics_confidence[0]"), "Unknown");
        let reason = field(&collected, "graphics_reason[0]");
        assert!(reason.contains("BISCUIT_TERMINAL_PROBE=off"), "{reason}");
    }

    /// A terminal whose identity decides the route (WezTerm) gets its colors
    /// queried but no Kitty query.
    #[test]
    fn known_kitty_terminal_is_not_probed() {
        expect_level!(Level::L1, pty_available(), "PTY (/dev/ptmx)");
        let mut answers = full_answers();
        let collected = run(&[("PROBE_TERM_PROGRAM", "WezTerm")], &mut answers);

        assert_eq!(
            count_occurrences(&collected, OSC10_QUERY),
            1,
            "{}",
            raw(&collected)
        );
        assert!(
            position(&collected, b"\x1b_G").is_none(),
            "{}",
            raw(&collected)
        );
        assert_eq!(field(&collected, "graphics_format[0]"), "Kitty");
        assert_eq!(field(&collected, "graphics_confidence[0]"), "Known");
    }

    /// Terminal.app is known to lack a pixel format, so no Kitty query (and,
    /// since it is not asked for colors, no DA1) is written.
    #[test]
    fn terminal_app_is_not_probed() {
        expect_level!(Level::L1, pty_available(), "PTY (/dev/ptmx)");
        let mut answers = [ProbeAnswer::every(DA1_QUERY, DA1_REPLY)];
        let collected = run(&[("PROBE_TERM_PROGRAM", "Apple_Terminal")], &mut answers);

        assert!(
            position(&collected, b"\x1b_G").is_none(),
            "{}",
            raw(&collected)
        );
        assert_eq!(
            count_occurrences(&collected, DA1_QUERY),
            0,
            "{}",
            raw(&collected)
        );
        assert_eq!(field(&collected, "graphics_format[0]"), "None");
        assert_eq!(field(&collected, "graphics_confidence[0]"), "Known");
        assert_eq!(field(&collected, "graphics_reason_kind[0]"), "NoSupport");
    }

    /// Three constructions in one process send one Kitty query and resolve
    /// the same route from its cached answer.
    #[test]
    fn graphics_probe_is_sent_once_per_process() {
        expect_level!(Level::L1, pty_available(), "PTY (/dev/ptmx)");
        let mut answers = [
            ProbeAnswer::kitty("OK"),
            ProbeAnswer::every(DA1_QUERY, DA1_REPLY),
        ];
        let collected = run(
            &[UNIDENTIFIED, ("PROBE_GRAPHICS_CONSTRUCTIONS", "3")],
            &mut answers,
        );

        assert_eq!(
            direct_kitty_queries(&collected).len(),
            1,
            "{}",
            raw(&collected)
        );
        assert_eq!(
            count_occurrences(&collected, DA1_QUERY),
            1,
            "{}",
            raw(&collected)
        );
        for i in 0..3 {
            assert_eq!(field(&collected, &format!("graphics_format[{i}]")), "Kitty");
            assert_eq!(
                field(&collected, &format!("graphics_confidence[{i}]")),
                "Verified"
            );
            assert_eq!(
                field(&collected, &format!("graphics_reason[{i}]")),
                field(&collected, "graphics_reason[0]")
            );
        }
    }

    const HIDDEN_TMUX: (&str, &str) = ("PROBE_TERM", "tmux-256color");

    /// `TERM=tmux-256color` without `TMUX`: a direct exchange carrying DA1
    /// alone (an unwrapped Kitty query would retitle a real tmux pane), then
    /// a wrapped one. The wrapped Kitty query comes back as a passthrough
    /// tmux delivers it (unwrapped), giving a verified Kitty route through
    /// tmux.
    #[test]
    fn hidden_tmux_passthrough_verifies_a_tmux_route() {
        expect_level!(Level::L1, pty_available(), "PTY (/dev/ptmx)");
        let mut answers = [
            ProbeAnswer::kitty_wrapped("OK"),
            ProbeAnswer::every(DA1_QUERY, DA1_REPLY),
        ];
        let collected = run(&[HIDDEN_TMUX], &mut answers);

        assert_eq!(
            direct_kitty_queries(&collected).len(),
            0,
            "the direct exchange must not carry the Kitty query; {}",
            raw(&collected)
        );
        assert_eq!(
            kitty_query_ids(&collected, true).len(),
            1,
            "{}",
            raw(&collected)
        );
        let direct_at = position(&collected, DA1_QUERY).expect("direct DA1 request");
        let wrapped_at = position(&collected, b"\x1bPtmux;").expect("wrapped query");
        assert!(
            direct_at < wrapped_at,
            "direct exchange must come first; {}",
            raw(&collected)
        );

        assert_eq!(field(&collected, "graphics_format[0]"), "Kitty");
        assert_eq!(field(&collected, "graphics_transport[0]"), "Tmux");
        assert_eq!(field(&collected, "graphics_confidence[0]"), "Verified");
    }

    /// A wrapped exchange nobody answers costs only the silence budget and
    /// leaves the route unknown, with the transport override as the fix.
    #[test]
    fn hidden_tmux_silence_suggests_the_transport_override() {
        expect_level!(Level::L1, pty_available(), "PTY (/dev/ptmx)");
        let mut answers = [ProbeAnswer::new(DA1_QUERY, DA1_REPLY)];
        let collected = run(
            &[HIDDEN_TMUX, ("BISCUIT_TERMINAL_QUERY_SILENCE_MS", "300")],
            &mut answers,
        );

        assert_eq!(
            kitty_query_ids(&collected, true).len(),
            1,
            "{}",
            raw(&collected)
        );
        assert_eq!(
            count_occurrences(&collected, WRAPPED_DA1),
            1,
            "{}",
            raw(&collected)
        );
        assert_eq!(field(&collected, "graphics_format[0]"), "None");
        assert_eq!(field(&collected, "graphics_confidence[0]"), "Unknown");
        let reason = field(&collected, "graphics_reason[0]");
        assert!(
            reason.contains("the wrapped Kitty graphics query was not answered"),
            "{reason}"
        );
        let fix = field(&collected, "graphics_fix[0]");
        assert!(fix.contains("BISCUIT_GRAPHICS_TRANSPORT=tmux"), "{fix}");
        assert!(fix.contains("BISCUIT_GRAPHICS_TRANSPORT=direct"), "{fix}");
        assert!(direct_kitty_queries(&collected).is_empty(), "{}", raw(&collected));
    }

    /// Inside GNU screen (`STY`), which takes an unwrapped APC as a
    /// hardstatus change, the direct exchange carries DA1 alone, so no Kitty
    /// query reaches screen and no Kitty route comes from it.
    #[test]
    fn gnu_screen_direct_exchange_carries_no_kitty_query() {
        expect_level!(Level::L1, pty_available(), "PTY (/dev/ptmx)");
        let mut answers = [
            ProbeAnswer::kitty("OK"),
            ProbeAnswer::every(DA1_QUERY, DA1_REPLY),
        ];
        let collected = run(&[UNIDENTIFIED, ("STY", "123.pts-0.host")], &mut answers);

        assert!(direct_kitty_queries(&collected).is_empty(), "{}", raw(&collected));
        assert!(kitty_query_ids(&collected, true).is_empty(), "{}", raw(&collected));
        assert!(count_occurrences(&collected, DA1_QUERY) >= 1, "{}", raw(&collected));
        assert_eq!(field(&collected, "graphics_format[0]"), "None");
    }

    /// The wrapped exchange doubles every ESC inside the envelope exactly
    /// once, and is the only Kitty query sent.
    #[test]
    fn wrapped_exchange_doubles_escapes_once() {
        expect_level!(Level::L1, pty_available(), "PTY (/dev/ptmx)");
        let mut answers = [
            ProbeAnswer::kitty_wrapped("OK"),
            ProbeAnswer::every(DA1_QUERY, DA1_REPLY),
        ];
        let collected = run(&[HIDDEN_TMUX], &mut answers);

        let direct = direct_kitty_queries(&collected);
        let wrapped = kitty_query_ids(&collected, true);
        assert_eq!((direct.len(), wrapped.len()), (0, 1), "{}", raw(&collected));

        let mut expected = format!(
            "\x1bPtmux;\x1b\x1b_Gi={},s=1,v=1,a=q,t=d,f=24;AAAA\x1b\x1b\\\x1b\\",
            wrapped[0]
        )
        .into_bytes();
        expected.extend_from_slice(WRAPPED_DA1);
        assert!(
            position(&collected, &expected).is_some(),
            "expected the exact wrapped query and sentinel; {}",
            raw(&collected)
        );
        assert!(
            position(&collected, b"\x1b\x1b\x1b").is_none(),
            "an ESC was doubled more than once; {}",
            raw(&collected)
        );
    }
}
