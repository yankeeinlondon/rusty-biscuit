//! Level-2 PTY evidence for the per-process OSC 10 foreground-color cache
//! (Findings 2 & 21 of the 2026-07-12 performance review).
//!
//! These tests spawn the `discovery_probe` example inside a real
//! pseudoterminal (via [`expectrl`], through the shared `tests/common/pty.rs`
//! helper — no second PTY abstraction), so `is_tty()` is true and the OSC
//! foreground query is actually emitted. The proof runs in a **dedicated
//! child process**, so the `OnceLock`-backed cache starts empty and no test
//! ordering can contaminate it.
//!
//! They are `level2_*` because they require a real PTY; run them only through
//! `just test-l2`. The Unix-only `expectrl`/PTY code is target-gated so Windows
//! still compiles and records a clean unsupported/skip disposition.
//!
//! Run `cargo build -p biscuit-terminal --example discovery_probe` first.

#[cfg(unix)]
mod common;

#[cfg(unix)]
mod unix {
    use std::time::Duration;

    use super::common::pty::{
        OSC10_QUERY, OSC11_QUERY, OscAnswer, count_occurrences, drive_probe, spawn_with_env,
    };

    /// A distinctive manufactured foreground reply, `rgb:1234/5678/9abc`.
    ///
    /// It decodes to `RgbValue { r: 18, g: 86, b: 154 }` — a value that equals
    /// no terminal default (dark defaults are 229/229/229, Apple 0/0/0), so
    /// observing it across every construction proves the cached first response
    /// was *reused*, not that two constructions coincidentally agreed.
    const OSC10_REPLY: &[u8] = b"\x1b]10;rgb:1234/5678/9abc\x07";
    const OSC10_EXPECTED: &str = "Some(RgbValue { r: 18, g: 86, b: 154 })";

    /// A dark background reply so `color_mode()` resolves from luminance and
    /// never forks the macOS `defaults` fallback during the proof.
    const OSC11_REPLY: &[u8] = b"\x1b]11;rgb:1e1e/1e1e/1e1e\x07";

    /// Collect the probe's `terminal_text_color[i]=...` values in index order.
    ///
    /// The marker is located anywhere in the line, not at the start: the first
    /// value line is preceded on the wire by the raw `ESC ] 10 ; ? BEL` query
    /// bytes that the library wrote to the same PTY.
    fn text_color_lines(collected: &[u8]) -> Vec<String> {
        String::from_utf8_lossy(collected)
            .lines()
            .filter_map(|line| {
                let idx = line.find("terminal_text_color[")?;
                let rest = &line[idx + "terminal_text_color[".len()..];
                // rest looks like `0]=Some(RgbValue { .. })`
                Some(match rest.split_once("]=") {
                    Some((_, value)) => value.to_string(),
                    None => rest.to_string(),
                })
            })
            .collect()
    }

    /// Three `Terminal` constructions in one process emit exactly one OSC 10
    /// request, and every construction reports the same cached foreground
    /// response. Proves the `TEXT_COLOR_CACHE` reuse (Findings 2 & 21).
    #[test]
    fn level2_terminal_construction_emits_single_osc10_request() {
        let mut session = spawn_with_env(&[
            ("PROBE", "terminal_cache"),
            ("PROBE_TERM_PROGRAM", "WezTerm"),
            ("PROBE_TERM_CONSTRUCTIONS", "3"),
            // Keep color_mode() off the macOS `defaults` fork; the OSC 11 answer
            // below also satisfies it, this is belt-and-suspenders.
            ("DARK_MODE", "1"),
        ]);

        // Manufacture a reply only for the *first* OSC 10 (and OSC 11) request.
        // The cache means later constructions never re-query, so a single
        // answer suffices for all three.
        let mut answers = [
            OscAnswer::new(OSC11_QUERY, OSC11_REPLY),
            OscAnswer::new(OSC10_QUERY, OSC10_REPLY),
        ];

        let collected = drive_probe(
            &mut session,
            &mut answers,
            "terminal_cache_done",
            Duration::from_secs(10),
        );

        // (1) exactly one OSC 10 request across three constructions.
        let osc10_requests = count_occurrences(&collected, OSC10_QUERY);
        assert_eq!(
            osc10_requests,
            1,
            "expected exactly one OSC 10 request across 3 Terminal constructions, saw {osc10_requests}; \
             raw: {:?}",
            String::from_utf8_lossy(&collected)
        );

        // (2) every construction reports the same cached, manufactured value.
        let values = text_color_lines(&collected);
        assert_eq!(
            values.len(),
            3,
            "expected 3 text_color lines, got {values:?}; raw: {:?}",
            String::from_utf8_lossy(&collected)
        );
        for (i, value) in values.iter().enumerate() {
            assert_eq!(
                value, OSC10_EXPECTED,
                "construction {i} reported {value:?}, expected the reused cached response \
                 {OSC10_EXPECTED:?}",
            );
        }
    }

    /// Record repeated-construction latency (warm-up + samples + dispersion)
    /// for the feature-local evidence index. The probe absorbs the one-time
    /// cold OSC round-trip in its warm-up loop, then times cached repeated
    /// constructions. This test emits the statistics on stdout for evidence
    /// harvesting and sanity-asserts that repeated construction is cheap.
    #[test]
    fn level2_terminal_repeated_construction_latency() {
        let mut session = spawn_with_env(&[
            ("PROBE", "terminal_latency"),
            ("PROBE_TERM_PROGRAM", "WezTerm"),
            ("PROBE_TERM_WARMUP", "3"),
            ("PROBE_TERM_SAMPLES", "50"),
            ("DARK_MODE", "1"),
        ]);

        let mut answers = [
            OscAnswer::new(OSC11_QUERY, OSC11_REPLY),
            OscAnswer::new(OSC10_QUERY, OSC10_REPLY),
        ];

        let collected = drive_probe(
            &mut session,
            &mut answers,
            "terminal_latency_done",
            Duration::from_secs(20),
        );

        let text = String::from_utf8_lossy(&collected);
        let field = |key: &str| -> Option<String> {
            text.lines()
                .find_map(|line| line.trim().strip_prefix(key).map(|v| v.to_string()))
        };

        let samples: usize = field("terminal_latency_samples=")
            .and_then(|v| v.parse().ok())
            .unwrap_or(0);
        let median_ns: f64 = field("terminal_latency_median_ns=")
            .and_then(|v| v.parse().ok())
            .unwrap_or(f64::INFINITY);

        assert_eq!(samples, 50, "expected 50 timed samples; raw: {text}");

        // Emit the whole stats block for evidence harvesting (visible with
        // `--no-capture`).
        for line in text.lines() {
            if line.trim().starts_with("terminal_latency_") {
                println!("{}", line.trim());
            }
        }

        // Sanity: repeated (cached) construction must be far cheaper than a
        // single un-cached OSC round-trip (two 100 ms timeouts). A regression
        // that dropped the cache would re-pay the tty round-trip on every
        // construction and blow past this loose bound.
        assert!(
            median_ns < 50_000_000.0,
            "repeated-construction median {median_ns} ns is implausibly high — cache may be broken; \
             raw: {text}"
        );
    }
}

/// On non-Unix targets the PTY/`expectrl` path does not compile; record a clean
/// unsupported/skip disposition so Windows still builds this binary.
#[cfg(not(unix))]
#[test]
fn level2_terminal_osc_cache_unsupported_on_this_platform() {
    eprintln!(
        "level2_terminal_osc_cache: skipped — PTY (expectrl) OSC cache evidence is Unix-only"
    );
}
