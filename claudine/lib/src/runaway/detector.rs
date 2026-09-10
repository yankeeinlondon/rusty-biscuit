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

    /// Swap the compiled exit-expression set in place, preserving every
    /// other piece of detector state (the partial-line buffer, the cycle
    /// ring, and the per-turn volume counters).
    ///
    /// The CLI wiring layer calls this when a provider reports its actual
    /// model via `SessionStart` and the in-scope exit-expression set must
    /// be re-filtered for the reported `(provider, model)`. `SessionStart`
    /// fires before any meaningful assistant output, so no repetition or
    /// volume progress is discarded by re-scoping at that point — only the
    /// pattern set the next lines are tested against changes.
    pub fn set_exit_expressions(&mut self, patterns: CompiledExitExpressions) {
        self.patterns = patterns;
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
mod tests;
