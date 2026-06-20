//! Line-assembling content detector driving all three runaway guards.
//!
//! The detector is pure and stateful: feed arbitrary chunks of assistant
//! text in, get a [`Trip`] out the moment one of three guards fires. It
//! owns a partial-line buffer for chunk reassembly, a bounded ring of
//! recent normalized lines for cycle detection, per-turn volume counters,
//! and a compiled exit-expression set.
//!
//! Surface:
//! - [`ContentDetector::feed`] — append a chunk; returns the first `Trip`
//!   observed (a trip is terminal — further work on the chunk stops).
//! - [`ContentDetector::flush`] — process any trailing partial line
//!   without a newline at end-of-stream.
//! - [`ContentDetector::reset_turn`] — zero the volume counters; the
//!   streaming caller invokes it on `TurnComplete` so a multi-turn run
//!   does not accumulate.
//!
//! See `claudine/features/2026-06-19-repetitive/spec.md` Clusters A, B, F
//! for the algorithm rationale.

use std::collections::VecDeque;

use super::patterns::CompiledExitExpressions;
use super::{
    MAX_CYCLE_LENGTH, MAX_REPETITION_ALLOWED, Trip, VOLUME_BYTES, VOLUME_LINES,
};

/// Knobs that parameterize a [`ContentDetector`]. Built by the caller
/// (CLI wiring layer) from the resolved `GuardSettings` config; the
/// detector itself stays config-agnostic.
#[derive(Debug, Clone)]
pub struct DetectorConfig {
    /// Whether the group-cycle repetition guard is enabled.
    pub repetition_enabled: bool,
    /// Full-cycle count threshold at which the repetition guard trips.
    pub max_repeats: usize,
    /// Maximum cycle length `K` the detector attempts to recognize.
    pub max_cycle_length: usize,
    /// Whether the per-turn volume cap is enabled.
    pub volume_enabled: bool,
    /// Per-turn (streaming) line threshold for the volume cap.
    pub max_lines: u64,
    /// Per-turn (streaming) byte threshold for the volume cap.
    pub max_bytes: u64,
}

impl Default for DetectorConfig {
    fn default() -> Self {
        Self {
            repetition_enabled: true,
            max_repeats: MAX_REPETITION_ALLOWED,
            max_cycle_length: MAX_CYCLE_LENGTH,
            volume_enabled: true,
            max_lines: VOLUME_LINES,
            max_bytes: VOLUME_BYTES,
        }
    }
}

/// Pure, stateful line-assembling content detector.
///
/// Holds a bounded ring of the last `2 * max_cycle_length` normalized
/// lines so memory is constant regardless of how long the runaway runs.
/// The exit-expression set is owned (compiled once before streaming);
/// scope selection happens before construction so the detector never
/// looks at a pattern that is out of scope.
///
/// Cycle detection keeps two pieces of state across `feed` calls:
/// - the currently active cycle length `L` (or `None` if no cycle is
///   currently recognized), and
/// - the count of consecutive lines at the tail that match the active
///   cycle's period.
///
/// The ring (bounded at `2K`) is the sliding window used to *recognize*
/// the cycle each line; the count is what lets a long-running cycle
/// cross `MAX_REPETITION_ALLOWED` even when `2K < MAX_REPETITION_ALLOWED *
/// L` (e.g. a 6-line cycle at threshold 30 needs 180 lines observed,
/// but the ring only holds 32).
pub struct ContentDetector {
    cfg: DetectorConfig,
    patterns: CompiledExitExpressions,
    /// Partial line accumulator: streamed chunks may split a logical
    /// line across any byte boundary.
    pending: String,
    /// Ring buffer of recent normalized lines (oldest at the front).
    ring: VecDeque<String>,
    /// Per-turn line counter (reset by `reset_turn`).
    lines: u64,
    /// Per-turn byte counter (reset by `reset_turn`).
    bytes: u64,
    /// Currently active cycle length `L`, or `None` when no cycle is
    /// recognized at the tail. Resets when the cycle breaks.
    current_cycle_len: Option<usize>,
    /// Number of consecutive lines at the tail that match the active
    /// cycle. `full_cycles = consecutive_matching_lines / L`.
    consecutive_matching_lines: usize,
}

impl ContentDetector {
    /// Construct a detector with the given config and compiled
    /// exit-expression set. Both the repetition and volume guards can be
    /// disabled via `cfg`; an empty pattern set is a no-op.
    pub fn new(cfg: DetectorConfig, patterns: CompiledExitExpressions) -> Self {
        Self {
            cfg,
            patterns,
            pending: String::new(),
            ring: VecDeque::with_capacity(2 * MAX_CYCLE_LENGTH),
            lines: 0,
            bytes: 0,
            current_cycle_len: None,
            consecutive_matching_lines: 0,
        }
    }

    /// Per-turn line counter at the moment of the last call (does not
    /// mutate state; useful in tests and for diagnostics).
    pub fn current_lines(&self) -> u64 {
        self.lines
    }

    /// Per-turn byte counter at the moment of the last call.
    pub fn current_bytes(&self) -> u64 {
        self.bytes
    }

    /// Internal ring-buffer bound (`2 * max_cycle_length`). Exposed for
    /// tests that assert the memory invariant.
    fn ring_cap(&self) -> usize {
        2 * self.cfg.max_cycle_length
    }

    /// Append a chunk. Splits on `\n`, processing each completed line:
    /// normalize (trim trailing whitespace; keep blanks as `""`),
    /// increment volume, test exit-expressions, push to the ring and run
    /// cycle detection. Returns the **first** `Trip` observed and stops
    /// further work on the chunk (a trip is terminal).
    pub fn feed(&mut self, chunk: &str) -> Option<Trip> {
        self.pending.push_str(chunk);
        loop {
            let newline_pos = self.pending.find('\n')?;
            let line: String = self.pending.drain(..=newline_pos).collect();
            // `line` includes the trailing `\n`; strip it for matching.
            let completed = &line[..line.len() - 1];
            if let Some(trip) = self.process_line(completed, true) {
                return Some(trip);
            }
        }
    }

    /// Process any trailing partial line without a newline. Useful at
    /// end-of-stream so the per-line match target fires on a final
    /// partial line.
    pub fn flush(&mut self) -> Option<Trip> {
        if self.pending.is_empty() {
            return None;
        }
        let completed = std::mem::take(&mut self.pending);
        // A flushed partial line never carried a `\n`, so do not add the
        // implicit-newline byte to the volume counter (matches the
        // streaming feed which always counts the `\n` for completed
        // lines).
        self.process_line(&completed, false)
    }

    /// Zero the per-turn volume counters. The streaming caller invokes
    /// this on `TurnComplete` so a multi-turn run does not accumulate
    /// into a trip. The ring buffer and cycle state are preserved — a
    /// runaway that crosses a turn boundary mid-cycle is still a
    /// runaway.
    pub fn reset_turn(&mut self) {
        self.lines = 0;
        self.bytes = 0;
    }

    /// Process one completed line: normalize, count volume, test
    /// exit-expressions, push to the ring, and run cycle detection.
    /// Returns the first trip observed.
    ///
    /// `with_implicit_newline` controls whether the byte counter gets
    /// the trailing `\n` byte added (true for `feed`'s completed lines,
    /// false for `flush`'s trailing partial line).
    fn process_line(
        &mut self,
        line: &str,
        with_implicit_newline: bool,
    ) -> Option<Trip> {
        // Volume accounting (F2). Performed before normalization so the
        // counter matches the bytes the model actually emitted (including
        // trailing whitespace that normalization trims). "Exceeds" the
        // threshold means strictly greater than, so a turn that lands
        // exactly on the threshold does not trip — the next line does.
        if self.cfg.volume_enabled {
            self.lines = self.lines.saturating_add(1);
            let mut added = line.len() as u64;
            if with_implicit_newline {
                added = added.saturating_add(1);
            }
            self.bytes = self.bytes.saturating_add(added);
            if self.lines > self.cfg.max_lines || self.bytes > self.cfg.max_bytes {
                return Some(Trip::RunawayVolume {
                    lines: self.lines,
                    bytes: self.bytes,
                });
            }
        }

        // Normalize (B2): trim trailing whitespace only. Leading
        // whitespace is preserved so an indented cycle is not normalized
        // into a different cycle.
        let normalized: String = line.trim_end().to_string();
        // B3: blank lines are kept as `""`, not skipped. `trim_end` on
        // an all-whitespace line already yields `""`.

        // Exit-expression match (E3d): per completed line, first match
        // wins. Empty pattern set is a no-op (zero per-line cost).
        if let Some((pattern, scope)) = self.patterns.matches_line(&normalized) {
            return Some(Trip::ExitExpression {
                pattern: pattern.to_string(),
                scope: scope.clone(),
            });
        }

        // Push to the ring and run cycle detection. Only the repetition
        // guard needs the ring; if the guard is disabled, skip the push
        // entirely (zero overhead for runs that opt out).
        if self.cfg.repetition_enabled {
            if self.ring.len() == self.ring_cap() {
                self.ring.pop_front();
            }
            self.ring.push_back(normalized);
            if let Some((cycle_len, repeats)) = self.detect_cycle() {
                return Some(Trip::RunawayRepetition {
                    cycle_len,
                    repeats,
                });
            }
        }

        None
    }

    /// Group-cycle detection (B1/B2). Find the smallest period `L` in
    /// `1..=max_cycle_length` such that the last `2L` ring entries are
    /// two identical halves; update the consecutive-match state and
    /// trip when `consecutive_matching_lines / L` reaches
    /// `max_repeats`.
    ///
    /// State model: the ring (bounded at `2K`) is the sliding window
    /// used to recognize the cycle each line. The
    /// `consecutive_matching_lines` counter is what lets a long-running
    /// cycle cross `max_repeats` even when `2K < max_repeats * L`
    /// (e.g. a 6-line cycle at threshold 30 needs 180 lines observed,
    /// but the ring only holds 32). When the recognized `L` changes or
    /// no `L` matches, the counter resets.
    fn detect_cycle(&mut self) -> Option<(usize, usize)> {
        let n = self.ring.len();
        if n < 2 {
            // Need at least two entries to recognize even L = 1.
            return None;
        }
        // Smallest L in 1..=max_cycle_length with two identical halves
        // at the tail. Skip L values where the ring doesn't yet hold
        // 2L entries.
        let max_l = self.cfg.max_cycle_length.min(n / 2);
        let detected_l = (1..=max_l)
            .find(|&candidate| self.tail_is_two_identical_halves(candidate));

        match detected_l {
            Some(cycle_len) => {
                match self.current_cycle_len {
                    Some(prev) if prev == cycle_len => {
                        // The active cycle continues — one more matching
                        // line at the tail.
                        self.consecutive_matching_lines =
                            self.consecutive_matching_lines.saturating_add(1);
                    }
                    _ => {
                        // New cycle (or first detection). The ring just
                        // verified two matching halves at this `L`, so
                        // seed the counter at `2 * L`.
                        self.current_cycle_len = Some(cycle_len);
                        self.consecutive_matching_lines = 2 * cycle_len;
                    }
                }
                let full_cycles = self.consecutive_matching_lines / cycle_len;
                if full_cycles >= self.cfg.max_repeats {
                    return Some((cycle_len, full_cycles));
                }
                None
            }
            None => {
                // No cycle recognized at the tail — reset state so a
                // later runaway starts fresh.
                self.current_cycle_len = None;
                self.consecutive_matching_lines = 0;
                None
            }
        }
    }

    /// Whether the last `2 * cycle_len` ring entries form two
    /// identical halves (exact equality on normalized lines — B2).
    fn tail_is_two_identical_halves(&self, cycle_len: usize) -> bool {
        let n = self.ring.len();
        debug_assert!(n >= 2 * cycle_len, "caller must ensure 2L entries exist");
        let half_start = n - 2 * cycle_len;
        let mid = n - cycle_len;
        for i in 0..cycle_len {
            if self.ring[half_start + i] != self.ring[mid + i] {
                return false;
            }
        }
        true
    }
}

/// Per-run volume cap for the capture path. Unlike [`ContentDetector`]
/// (per-turn, scans text), this is a coarse per-run byte/line gate
/// applied to the growing capture buffer to bound its memory exposure
/// (Cluster F3). It deliberately does not perform exit-expression or
/// repetition detection — the capture path gets Ctrl+C + volume cap
/// only.
#[derive(Debug, Clone)]
pub struct CaptureVolumeCap {
    pub enabled: bool,
    pub max_lines: u64,
    pub max_bytes: u64,
}

impl Default for CaptureVolumeCap {
    fn default() -> Self {
        Self {
            enabled: true,
            max_lines: VOLUME_LINES,
            max_bytes: VOLUME_BYTES,
        }
    }
}

impl CaptureVolumeCap {
    /// Construct a cap with the configured thresholds and enabled flag.
    pub fn new(enabled: bool, max_lines: u64, max_bytes: u64) -> Self {
        Self {
            enabled,
            max_lines,
            max_bytes,
        }
    }

    /// Check whether the supplied running counters trip the cap. Returns
    /// a [`Trip::RunawayVolume`] capturing the counters when breached, or
    /// `None` when the cap is disabled or under both thresholds.
    ///
    /// The capture path tracks `total_bytes` (and optionally
    /// `total_lines`) as it appends to its capture `String`; this helper
    /// is the per-append trip check.
    pub fn check(&self, total_lines: u64, total_bytes: u64) -> Option<Trip> {
        if !self.enabled {
            return None;
        }
        if total_lines > self.max_lines || total_bytes > self.max_bytes {
            Some(Trip::RunawayVolume {
                lines: total_lines,
                bytes: total_bytes,
            })
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runaway::patterns::{ExitExpressionInput, PatternKind};
    use crate::runaway::{MAX_CYCLE_LENGTH, MAX_REPETITION_ALLOWED, Trip};

    /// Build a detector with the defaults and an empty pattern set.
    fn detector() -> ContentDetector {
        ContentDetector::new(DetectorConfig::default(), CompiledExitExpressions::empty())
    }

    /// Push `count` repetitions of the supplied lines as a single
    /// joined feed (simulates one streaming chunk per cycle). Each
    /// cycle entry must already carry its own trailing `\n` so the
    /// helper does not double-up newlines (which would interleave
    /// blank lines and confuse the cycle detector).
    fn feed_cycle(det: &mut ContentDetector, cycle: &[&str], count: usize) -> Option<Trip> {
        let mut joined = String::new();
        for _ in 0..count {
            for line in cycle {
                joined.push_str(line);
            }
        }
        det.feed(&joined)
    }

    // ------------------------------------------------------------------
    // VC-2.1 — Captured-runaway trip (6-line cycle, repeats >= 30).
    // ------------------------------------------------------------------

    #[test]
    fn vc_2_1_captured_six_line_cycle_trips_at_threshold() {
        let mut det = detector();
        // One-time preamble: must not contribute to the cycle.
        det.feed("This is the final listening.\n");
        let cycle = ["Done.\n", "No more.\n", "End.\n", "STOP.\n", "OK.\n", "Bye.\n"];
        // 29 cycles (well under threshold) must NOT trip.
        assert!(feed_cycle(&mut det, &cycle, 29).is_none());
        // The 30th cycle pushes the trip.
        let trip = feed_cycle(&mut det, &cycle, 1).expect("30 cycles should trip");
        match trip {
            Trip::RunawayRepetition {
                cycle_len,
                repeats,
            } => {
                assert_eq!(cycle_len, 6);
                assert!(
                    repeats >= MAX_REPETITION_ALLOWED,
                    "repeats should reach the threshold, got {repeats}"
                );
            }
            other => panic!("expected RunawayRepetition, got {other:?}"),
        }
    }

    // ------------------------------------------------------------------
    // VC-2.2 — Single-line spam trips at L = 1.
    // ------------------------------------------------------------------

    #[test]
    fn vc_2_2_single_line_spam_trips_at_l1() {
        let mut det = detector();
        // 29 cycles: not yet.
        let trip29 = feed_cycle(&mut det, &["STOP.\n"], 29);
        assert!(trip29.is_none(), "29 single-line cycles must not trip");
        // 30 cycles: trips at L=1, repeats=30.
        let trip = feed_cycle(&mut det, &["STOP.\n"], 1).expect("30 cycles should trip");
        match trip {
            Trip::RunawayRepetition {
                cycle_len,
                repeats,
            } => {
                assert_eq!(cycle_len, 1);
                assert_eq!(repeats, 30);
            }
            other => panic!("expected RunawayRepetition, got {other:?}"),
        }
    }

    // ------------------------------------------------------------------
    // VC-2.3 — Blank-line flood trips.
    // ------------------------------------------------------------------

    #[test]
    fn vc_2_3_blank_line_flood_trips() {
        let mut det = detector();
        // Blanks are kept as `""` (B3), so a blank flood is an L=1 cycle.
        let trip = feed_cycle(&mut det, &["\n"], MAX_REPETITION_ALLOWED);
        match trip.expect("blank flood should trip") {
            Trip::RunawayRepetition {
                cycle_len,
                repeats,
            } => {
                assert_eq!(cycle_len, 1);
                assert!(repeats >= MAX_REPETITION_ALLOWED);
            }
            other => panic!("expected RunawayRepetition, got {other:?}"),
        }
    }

    // ------------------------------------------------------------------
    // VC-2.4 — Realistic repetitive-but-legitimate output does NOT trip.
    // ------------------------------------------------------------------

    #[test]
    fn vc_2_4_realistic_repetitive_output_does_not_trip() {
        let mut det = detector();

        // A 6-line cycle repeated only 10× (well under the threshold).
        let cycle = ["Done.\n", "No more.\n", "End.\n", "STOP.\n", "OK.\n", "Bye.\n"];
        assert!(feed_cycle(&mut det, &cycle, 10).is_none());

        // A numbered list 1..100: monotonically increasing, never
        // matches itself.
        let mut numbered = String::new();
        for i in 1..=100 {
            numbered.push_str(&format!("{i}.\n"));
        }
        assert!(det.feed(&numbered).is_none());

        // A markdown table — header + separator + body. Each line
        // distinct.
        let table = "| a | b |\n|---|---|\n| 1 | 2 |\n| 3 | 4 |\n";
        assert!(det.feed(table).is_none());

        // Final flush with no pending data: still no trip.
        assert!(det.flush().is_none());
    }

    // ------------------------------------------------------------------
    // VC-2.5 — Exit-expression match across chunk boundaries.
    // ------------------------------------------------------------------

    #[test]
    fn vc_2_5_exit_expression_match_across_chunk_boundaries() {
        let patterns = CompiledExitExpressions::compile(&[ExitExpressionInput {
            patterns: vec!["STOP.".to_string()],
            kind: PatternKind::Literal,
            ignore_case: false,
            scope: None,
        }])
        .unwrap();
        let mut det = ContentDetector::new(DetectorConfig::default(), patterns);
        // Split the pattern across two chunks; the line assembler must
        // reassemble before matching.
        assert!(det.feed("STO").is_none());
        let trip = det.feed("P.\n").expect("reassembled line should match");
        match trip {
            Trip::ExitExpression { pattern, scope } => {
                assert_eq!(pattern, "STOP.");
                assert!(scope.is_none());
            }
            other => panic!("expected ExitExpression, got {other:?}"),
        }
    }

    // ------------------------------------------------------------------
    // VC-2.6 — Literal `ignore_case` + regex inline flags.
    // ------------------------------------------------------------------

    #[test]
    fn vc_2_6_literal_ignore_case_and_regex_inline_flags() {
        // Literal with ignore_case matches "stop."
        let lit_ci = CompiledExitExpressions::compile(&[ExitExpressionInput {
            patterns: vec!["STOP.".to_string()],
            kind: PatternKind::Literal,
            ignore_case: true,
            scope: None,
        }])
        .unwrap();
        let mut det = ContentDetector::new(DetectorConfig::default(), lit_ci);
        let trip = det.feed("ok stop.\n").expect("ignore_case should match");
        assert!(matches!(trip, Trip::ExitExpression { .. }));

        // Regex with inline `(?i)` matches without ignore_case.
        let re_inline = CompiledExitExpressions::compile(&[ExitExpressionInput {
            patterns: vec![r"(?i)stop\.".to_string()],
            kind: PatternKind::Regex,
            ignore_case: false,
            scope: None,
        }])
        .unwrap();
        let mut det = ContentDetector::new(DetectorConfig::default(), re_inline);
        let trip = det.feed("ok STOP.\n").expect("(?i) inline flag should match");
        assert!(matches!(trip, Trip::ExitExpression { .. }));

        // Literal default (case-sensitive) does NOT match "StopS" against
        // "STOP." — the metacharacter-surprise guardrail from E3a.
        let lit_default = CompiledExitExpressions::compile(&[ExitExpressionInput {
            patterns: vec!["STOP.".to_string()],
            kind: PatternKind::Literal,
            ignore_case: false,
            scope: None,
        }])
        .unwrap();
        let mut det = ContentDetector::new(DetectorConfig::default(), lit_default);
        assert!(
            det.feed("ok StopS\n").is_none(),
            "case-sensitive literal must not match StopS"
        );
    }

    // ------------------------------------------------------------------
    // VC-2.7 — Volume cap trips on acyclic flood; reset zeroes counters.
    // ------------------------------------------------------------------

    #[test]
    fn vc_2_7_volume_cap_trips_on_acyclic_flood_and_resets_per_turn() {
        let mut det = detector();
        // 50_000 distinct lines: just under threshold (VOLUME_LINES).
        for i in 0..50_000u64 {
            let trip = det.feed(&format!("line {i}\n"));
            assert!(
                trip.is_none(),
                "should not trip until threshold, got trip at {i}"
            );
        }
        // The 50_001st distinct line trips.
        let trip = det
            .feed("the final distinct line\n")
            .expect("50_001st line should trip the volume cap");
        assert!(matches!(trip, Trip::RunawayVolume { .. }));

        // reset_turn zeroes counters so a multi-turn run does not
        // accumulate into a trip.
        det.reset_turn();
        assert_eq!(det.current_lines(), 0);
        assert_eq!(det.current_bytes(), 0);
        // 50_000 more lines after reset do NOT trip.
        for i in 0..50_000u64 {
            let trip = det.feed(&format!("post-reset line {i}\n"));
            assert!(trip.is_none(), "post-reset should not trip");
        }
    }

    // ------------------------------------------------------------------
    // VC-2.8 — Ring buffer stays bounded.
    // ------------------------------------------------------------------

    #[test]
    fn vc_2_8_ring_buffer_stays_bounded() {
        // Disable the volume cap so 100k lines can be fed without
        // tripping — this checkpoint is about the memory invariant
        // only.
        let cfg = DetectorConfig {
            volume_enabled: false,
            ..DetectorConfig::default()
        };
        let mut det = ContentDetector::new(cfg, CompiledExitExpressions::empty());
        // Feed 100k distinct lines — the ring must cap at
        // `2 * MAX_CYCLE_LENGTH`.
        let mut joined = String::new();
        for i in 0..100_000u64 {
            joined.push_str(&format!("line {i}\n"));
        }
        assert!(det.feed(&joined).is_none(), "distinct lines should not trip");
        assert!(
            det.ring.len() <= 2 * MAX_CYCLE_LENGTH,
            "ring should be bounded at 2 * MAX_CYCLE_LENGTH ({}), got {}",
            2 * MAX_CYCLE_LENGTH,
            det.ring.len(),
        );
    }

    // ------------------------------------------------------------------
    // Additional internal invariants — exit-expression precedence,
    // disable flags, and cycle-reset behavior.
    // ------------------------------------------------------------------

    #[test]
    fn exit_expression_beats_repetition_when_pattern_matches_first_line() {
        let patterns = CompiledExitExpressions::compile(&[ExitExpressionInput {
            patterns: vec!["STOP.".to_string()],
            kind: PatternKind::Literal,
            ignore_case: false,
            scope: None,
        }])
        .unwrap();
        let mut det = ContentDetector::new(DetectorConfig::default(), patterns);
        // The cycle below would trip repetition at 30 cycles; the
        // pattern matches the first completed line and should fire
        // immediately.
        let trip = feed_cycle(&mut det, &["STOP.\n"], 1).expect("first line should match");
        assert!(matches!(trip, Trip::ExitExpression { .. }));
    }

    #[test]
    fn disabled_repetition_does_not_trip_even_on_obvious_cycle() {
        let cfg = DetectorConfig {
            repetition_enabled: false,
            ..DetectorConfig::default()
        };
        let mut det = ContentDetector::new(cfg, CompiledExitExpressions::empty());
        // 100 cycles of single-line spam — would trip with the guard on.
        let trip = feed_cycle(&mut det, &["STOP.\n"], 100);
        assert!(trip.is_none(), "disabled repetition must not trip");
    }

    #[test]
    fn disabled_volume_does_not_trip_even_on_huge_flood() {
        let cfg = DetectorConfig {
            volume_enabled: false,
            ..DetectorConfig::default()
        };
        let mut det = ContentDetector::new(cfg, CompiledExitExpressions::empty());
        // Feed a flood well above the line threshold. Distinct lines so
        // repetition does not trip either.
        let mut joined = String::new();
        for i in 0..100_000u64 {
            joined.push_str(&format!("line {i}\n"));
        }
        let trip = det.feed(&joined);
        assert!(trip.is_none(), "disabled volume must not trip");
    }

    #[test]
    fn flush_processes_trailing_partial_line() {
        let patterns = CompiledExitExpressions::compile(&[ExitExpressionInput {
            patterns: vec!["STOP.".to_string()],
            kind: PatternKind::Literal,
            ignore_case: false,
            scope: None,
        }])
        .unwrap();
        let mut det = ContentDetector::new(DetectorConfig::default(), patterns);
        // Feed a partial line with no trailing newline.
        assert!(det.feed("ok STOP.").is_none());
        let trip = det.flush().expect("flush should match the partial line");
        assert!(matches!(trip, Trip::ExitExpression { .. }));
    }

    #[test]
    fn flush_on_empty_pending_is_none() {
        let mut det = detector();
        assert!(det.flush().is_none());
    }

    #[test]
    fn flush_partial_line_does_not_count_implicit_newline_byte() {
        let mut det = detector();
        // 5 bytes partial, no newline.
        det.feed("hello");
        // flush: should count exactly 5 bytes (no implicit newline).
        assert!(det.flush().is_none());
        assert_eq!(det.current_bytes(), 5);
        assert_eq!(det.current_lines(), 1);
    }

    #[test]
    fn feed_complete_line_counts_implicit_newline_byte() {
        let mut det = detector();
        det.feed("hello\n");
        assert_eq!(det.current_bytes(), 6, "hello + newline = 6 bytes");
        assert_eq!(det.current_lines(), 1);
    }

    #[test]
    fn volume_byte_threshold_trips_before_line_threshold_when_bytes_dominate() {
        // Cap lines at a very high number and bytes at 10 — a single
        // long line should trip the byte cap without crossing the line
        // threshold.
        let cfg = DetectorConfig {
            max_lines: u64::MAX,
            max_bytes: 10,
            ..DetectorConfig::default()
        };
        let mut det = ContentDetector::new(cfg, CompiledExitExpressions::empty());
        let trip = det
            .feed("0123456789\n")
            .expect("byte threshold should trip on the 11-byte line");
        match trip {
            Trip::RunawayVolume { bytes, .. } => {
                assert!(bytes > 10, "byte counter should exceed 10, got {bytes}");
            }
            other => panic!("expected RunawayVolume, got {other:?}"),
        }
    }

    #[test]
    fn trailing_whitespace_is_normalized_for_cycle_match() {
        // Trailing whitespace should be trimmed so two lines that differ
        // only by trailing whitespace are treated as identical for cycle
        // detection.
        let mut det = detector();
        // 60 lines that alternate "STOP." and "STOP.   " — normalization
        // makes them all equal so this is a single-line cycle.
        let mut joined = String::new();
        for i in 0..60 {
            if i % 2 == 0 {
                joined.push_str("STOP.\n");
            } else {
                joined.push_str("STOP.   \n");
            }
        }
        let trip = det.feed(&joined).expect("normalized cycle should trip");
        match trip {
            Trip::RunawayRepetition {
                cycle_len,
                repeats,
            } => {
                assert_eq!(cycle_len, 1, "trailing whitespace must be normalized");
                assert!(repeats >= MAX_REPETITION_ALLOWED);
            }
            other => panic!("expected RunawayRepetition, got {other:?}"),
        }
    }

    #[test]
    fn cycle_break_resets_consecutive_count() {
        // If a cycle is broken by a distinct line, the count resets and
        // a fresh cycle must re-accumulate before tripping.
        let mut det = detector();
        // 25 cycles — under threshold, no trip.
        assert!(feed_cycle(&mut det, &["spam\n"], 25).is_none());
        // A distinct line breaks the cycle.
        assert!(det.feed("unique\n").is_none());
        // 25 more cycles — under threshold (counter was reset).
        assert!(feed_cycle(&mut det, &["spam\n"], 25).is_none());
        // 5 more cycles push it over 30 (from reset → 30 total).
        let trip = feed_cycle(&mut det, &["spam\n"], 5).expect("should trip after 30 fresh cycles");
        assert!(matches!(trip, Trip::RunawayRepetition { .. }));
    }

    #[test]
    fn capture_volume_cap_default_thresholds_match_streaming_volume() {
        let cap = CaptureVolumeCap::default();
        assert!(cap.enabled);
        assert_eq!(cap.max_bytes, super::super::VOLUME_BYTES);
        assert_eq!(cap.max_lines, super::super::VOLUME_LINES);
    }

    #[test]
    fn capture_volume_cap_check_trips_on_bytes_or_lines() {
        let cap = CaptureVolumeCap::new(true, 100, 1_000);
        // Below both — no trip.
        assert!(cap.check(50, 500).is_none());
        // Lines exceeded.
        assert!(cap.check(101, 500).is_some());
        // Bytes exceeded.
        assert!(cap.check(50, 1_001).is_some());
        // Disabled — never trips.
        let disabled = CaptureVolumeCap::new(false, 0, 0);
        assert!(disabled.check(u64::MAX, u64::MAX).is_none());
    }
}
