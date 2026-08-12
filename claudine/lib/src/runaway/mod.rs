//! Pure, stateful content-detector for the claudine runaway-output guards.
//!
//! The detector is intentionally independent of any I/O or CLI type: callers
//! feed arbitrary streamed chunks of assistant text in (`feed`/`flush`) and
//! receive a [`Trip`] out the moment one of three guards fires — an
//! exit-expression match, a group-cycle repetition, or a per-turn volume
//! breach. The detector owns a bounded ring buffer of recent normalized
//! lines so memory is constant regardless of how long the runaway runs.
//!
//! Provider/model scope selection, kill-switch flags, and channel plumbing
//! belong to the caller (CLI wiring layer); this module only knows text and
//! thresholds.
//!
//! See `claudine/features/2026-06-19-repetitive/spec.md` Clusters A–F for
//! the design rationale.

pub mod config;
pub mod detector;
pub mod patterns;

pub use config::{extract_frontmatter_exit_expressions, parse_scope, ScopeSelector};
pub use config::{resolve_exit_expressions, resolve_guard_settings, validate_exit_expressions};
pub use config::{
    ExitExpressionEntry, ExitExpressionsLayer, ExitExpressionsValue, GuardSettings, LayerMode,
    RepetitionGuardSettings, VolumeGuardSettings,
};
pub use detector::{CaptureVolumeCap, ContentDetector, DetectorConfig};
pub use patterns::{CompiledExitExpressions, ExitExpressionInput, PatternKind};

/// A terminal content-guard trip emitted by [`ContentDetector::feed`] /
/// [`ContentDetector::flush`].
///
/// A trip is **terminal**: the caller must not feed further text after one
/// is observed, and the detector stops further work on the chunk that
/// produced it. Each variant carries the structured detail its
/// `error_kind`/message needs and mirrors one [`EarlyTermination`]
/// variant; the CLI's `trip_to_early_termination` converts between the two
/// so this module never imports the termination-channel types.
///
/// [`EarlyTermination`]: crate::stream::logs::opencode::bridge::EarlyTermination
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Trip {
    /// A user-authored exit-expression matched a completed line. Carries
    /// the matched `pattern` (literal substring or regex source) and the
    /// optional `scope` string the entry declared (e.g.
    /// `"opencode/kimi-for-coding/k2p7"`).
    ExitExpression {
        pattern: String,
        scope: Option<String>,
    },
    /// The group-cycle detector observed `repeats` consecutive identical
    /// cycles of length `cycle_len` (`L`), crossing the threshold. Single
    /// line spam is the `L = 1` case.
    RunawayRepetition { cycle_len: usize, repeats: usize },
    /// The per-turn (streaming) or per-run (capture) volume exceeded a
    /// threshold. At least one of `lines` / `bytes` is at or above its
    /// threshold at trip time.
    RunawayVolume { lines: u64, bytes: u64 },
}

/// Maximum cycle length `K` the detector attempts to recognize.
///
/// Held to `K ≈ 16` so a single-line ring of `2 * MAX_CYCLE_LENGTH = 32`
/// entries covers any plausible block cycle, including the incident's
/// 6-line block. Cycles longer than this fall through to the volume cap.
// Cluster B3 / F2 — false-positive posture: K must be ≥ 6 to catch the
// reproduced 6-line incident; 16 is cheap headroom.
pub const MAX_CYCLE_LENGTH: usize = 16;

/// Number of identical full cycles (of any length `L ≤ K`) that must
/// repeat before the repetition guard trips.
///
/// Conservative by design — a flooding model still crosses 30 cycles in
/// well under a second, while a wrongful kill is a very negative outcome
/// (legitimate repetitive output below this threshold must not trip).
// Cluster B3 / F2 — false-positive posture.
pub const MAX_REPETITION_ALLOWED: usize = 30;

/// Per-turn line-count threshold for the volume cap on the streaming path.
// Cluster F2 — content-agnostic catch-all for acyclic / over-K floods; high
// enough no honest single turn reaches it.
pub const VOLUME_LINES: u64 = 50_000;

/// Per-turn byte-count threshold for the volume cap on the streaming path
/// and per-run cap on the capture path. 32 MiB.
// Cluster F2 — bounds the unbounded capture `String` memory exposure.
pub const VOLUME_BYTES: u64 = 32 * 1024 * 1024;

#[cfg(test)]
mod tests {
    use super::*;

    /// Sanity-check the constants against the spec's false-positive
    /// posture. If these drift the unit tests' thresholds must be
    /// revisited.
    #[test]
    fn constants_match_spec_posture() {
        const { assert!(MAX_CYCLE_LENGTH >= 6, "must cover the 6-line incident") };
        assert_eq!(MAX_CYCLE_LENGTH, 16);
        assert_eq!(MAX_REPETITION_ALLOWED, 30);
        assert_eq!(VOLUME_LINES, 50_000);
        assert_eq!(VOLUME_BYTES, 32 * 1024 * 1024);
    }

    /// `Trip` variants compare cleanly so callers can assert on them
    /// without parsing strings.
    #[test]
    fn trip_variants_are_eq_comparable() {
        assert_eq!(
            Trip::ExitExpression {
                pattern: "STOP.".to_string(),
                scope: None,
            },
            Trip::ExitExpression {
                pattern: "STOP.".to_string(),
                scope: None,
            },
        );
        assert_eq!(
            Trip::RunawayRepetition {
                cycle_len: 6,
                repeats: 30,
            },
            Trip::RunawayRepetition {
                cycle_len: 6,
                repeats: 30,
            },
        );
        assert_eq!(
            Trip::RunawayVolume {
                lines: 1,
                bytes: 2,
            },
            Trip::RunawayVolume {
                lines: 1,
                bytes: 2,
            },
        );
        // Different variants are unequal even with overlapping field
        // names.
        assert_ne!(
            Trip::RunawayRepetition {
                cycle_len: 1,
                repeats: 30,
            },
            Trip::ExitExpression {
                pattern: String::new(),
                scope: None,
            },
        );
    }
}
