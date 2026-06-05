//! Lightweight performance instrumentation for the compose pipeline.
//!
//! When `ComposeOptions::perf_enabled` is `true`, a `PerfCollector`
//! records per-stage timings. When disabled, all methods are no-ops.

use super::types::{ComposePerfMetric, ComposePerfReport, ComposeStage, ShellCommandSpan};
use std::time::{Duration, Instant};

/// Metric kinds corresponding to compose pipeline stages.
///
/// Variants are listed in pipeline execution order so the final
/// report has a deterministic, intuitive ordering.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum PerfMetricKind {
    FrontmatterInterpolation,
    SchemaValidation,
    FrontmatterShellExpansion,
    EffectiveStateBuild,
    TextReplacement,
    PageBlocks,
    Interpolation,
    ShellExpansion,
    ShellBlocks,
    LinkResolve,
    TransclusionParse,
    TransclusionPrepare,
    TransclusionResolve,
    TransclusionApply,
    Cleanup,
    Normalization,
    LinkNormalization,
}

impl PerfMetricKind {
    /// Convert to the public `ComposeStage` enum.
    fn stage(self) -> ComposeStage {
        match self {
            Self::FrontmatterInterpolation => ComposeStage::FrontmatterInterpolation,
            Self::SchemaValidation => ComposeStage::SchemaValidation,
            Self::FrontmatterShellExpansion => ComposeStage::FrontmatterShellExpansion,
            Self::EffectiveStateBuild => ComposeStage::EffectiveStateBuild,
            Self::TextReplacement => ComposeStage::TextReplacement,
            Self::PageBlocks => ComposeStage::PageBlocks,
            Self::Interpolation => ComposeStage::Interpolation,
            Self::ShellExpansion => ComposeStage::ShellExpansion,
            Self::ShellBlocks => ComposeStage::ShellBlocks,
            Self::LinkResolve => ComposeStage::LinkResolve,
            Self::TransclusionParse => ComposeStage::TransclusionParse,
            Self::TransclusionPrepare => ComposeStage::TransclusionPrepare,
            Self::TransclusionResolve => ComposeStage::TransclusionResolve,
            Self::TransclusionApply => ComposeStage::TransclusionApply,
            Self::Cleanup => ComposeStage::Cleanup,
            Self::Normalization => ComposeStage::Normalization,
            Self::LinkNormalization => ComposeStage::LinkNormalization,
        }
    }

    /// All variants in pipeline execution order.
    fn all() -> &'static [PerfMetricKind] {
        &[
            Self::FrontmatterInterpolation,
            Self::SchemaValidation,
            Self::FrontmatterShellExpansion,
            Self::EffectiveStateBuild,
            Self::TextReplacement,
            Self::PageBlocks,
            Self::Interpolation,
            Self::ShellExpansion,
            Self::ShellBlocks,
            Self::LinkResolve,
            Self::TransclusionParse,
            Self::TransclusionPrepare,
            Self::TransclusionResolve,
            Self::TransclusionApply,
            Self::Cleanup,
            Self::Normalization,
            Self::LinkNormalization,
        ]
    }
}

/// Collects per-stage timing metrics during a compose run.
///
/// When `enabled` is `false`, all methods short-circuit immediately.
pub(crate) struct PerfCollector {
    enabled: bool,
    start: Option<Instant>,
    /// Fixed-size array indexed by `PerfMetricKind` ordinal.
    durations: [(Duration, usize); 17],
    /// Per-`::shell`-directive spans (DM-3). Empty when disabled.
    shell_spans: Vec<ShellCommandSpan>,
    /// Per-group context-capture timings (DM-4). Empty when disabled.
    capture_timings: Vec<(String, Duration)>,
}

impl PerfCollector {
    /// Creates a new collector. When `enabled` is `false`, all
    /// recording methods are no-ops.
    pub(crate) fn new(enabled: bool) -> Self {
        Self {
            enabled,
            start: if enabled { Some(Instant::now()) } else { None },
            durations: [(Duration::ZERO, 0); 17],
            shell_spans: Vec::new(),
            capture_timings: Vec::new(),
        }
    }

    /// Returns `true` if perf collection is active.
    pub(crate) fn is_enabled(&self) -> bool {
        self.enabled
    }

    /// Records a per-`::shell`-directive span (DM-3). No-op when disabled.
    pub(crate) fn record_shell_span(&mut self, span: ShellCommandSpan) {
        if !self.enabled {
            return;
        }
        self.shell_spans.push(span);
    }

    /// Sets the per-group context-capture timings (DM-4). No-op when disabled.
    pub(crate) fn set_capture_timings(&mut self, timings: Vec<(String, Duration)>) {
        if !self.enabled {
            return;
        }
        self.capture_timings = timings;
    }

    /// Records a duration for the given metric kind.
    pub(crate) fn record(&mut self, kind: PerfMetricKind, elapsed: Duration) {
        if !self.enabled {
            return;
        }
        let idx = kind as usize;
        self.durations[idx].0 += elapsed;
        self.durations[idx].1 += 1;
    }

    /// Times a closure and records the elapsed duration.
    #[cfg(test)]
    pub(crate) fn measure<F, R>(&mut self, kind: PerfMetricKind, f: F) -> R
    where
        F: FnOnce() -> R,
    {
        if !self.enabled {
            return f();
        }
        let start = Instant::now();
        let result = f();
        self.record(kind, start.elapsed());
        result
    }

    /// Consumes the collector and produces a `ComposePerfReport`.
    ///
    /// Returns `None` when collection is disabled.
    pub(crate) fn finish(self) -> Option<ComposePerfReport> {
        if !self.enabled {
            return None;
        }

        let total = self.start.map(|s| s.elapsed()).unwrap_or_default();

        let metrics = PerfMetricKind::all()
            .iter()
            .map(|kind| {
                let idx = *kind as usize;
                let (elapsed, calls) = self.durations[idx];
                ComposePerfMetric {
                    stage: kind.stage(),
                    elapsed,
                    calls,
                }
            })
            .collect();

        Some(ComposePerfReport {
            total,
            metrics,
            shell_spans: self.shell_spans,
            capture_timings: self.capture_timings,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn disabled_collector_returns_none() {
        let collector = PerfCollector::new(false);
        assert!(!collector.is_enabled());
        assert!(collector.finish().is_none());
    }

    #[test]
    fn enabled_collector_records_metrics() {
        let mut collector = PerfCollector::new(true);
        assert!(collector.is_enabled());

        collector.record(PerfMetricKind::Cleanup, Duration::from_millis(5));
        collector.record(PerfMetricKind::Cleanup, Duration::from_millis(3));
        collector.record(PerfMetricKind::Interpolation, Duration::from_millis(10));

        let report = collector.finish().unwrap();
        assert!(report.total >= Duration::ZERO);

        let cleanup = report
            .metrics
            .iter()
            .find(|m| m.stage == ComposeStage::Cleanup)
            .unwrap();
        assert_eq!(cleanup.elapsed, Duration::from_millis(8));
        assert_eq!(cleanup.calls, 2);

        let interp = report
            .metrics
            .iter()
            .find(|m| m.stage == ComposeStage::Interpolation)
            .unwrap();
        assert_eq!(interp.elapsed, Duration::from_millis(10));
        assert_eq!(interp.calls, 1);
    }

    #[test]
    fn records_shell_spans_and_capture_timings() {
        let mut collector = PerfCollector::new(true);
        collector.record_shell_span(ShellCommandSpan {
            command_display: "echo hi".to_string(),
            command_hash: "00000000deadbeef".to_string(),
            elapsed: Duration::from_millis(12),
        });
        collector.set_capture_timings(vec![("git".to_string(), Duration::from_millis(7))]);

        let report = collector.finish().unwrap();
        assert_eq!(report.shell_spans.len(), 1);
        assert_eq!(report.shell_spans[0].command_display, "echo hi");
        assert_eq!(report.shell_spans[0].elapsed, Duration::from_millis(12));
        assert_eq!(report.capture_timings.len(), 1);
        assert_eq!(report.capture_timings[0].0, "git");
        assert_eq!(report.capture_timings[0].1, Duration::from_millis(7));
    }

    #[test]
    fn disabled_collector_drops_shell_spans_and_capture_timings() {
        let mut collector = PerfCollector::new(false);
        collector.record_shell_span(ShellCommandSpan {
            command_display: "echo hi".to_string(),
            command_hash: "0".to_string(),
            elapsed: Duration::from_millis(1),
        });
        collector.set_capture_timings(vec![("git".to_string(), Duration::from_millis(1))]);
        assert!(collector.finish().is_none());
    }

    #[test]
    fn measure_records_timing() {
        let mut collector = PerfCollector::new(true);
        let result = collector.measure(PerfMetricKind::TextReplacement, || 42);
        assert_eq!(result, 42);

        let report = collector.finish().unwrap();
        let metric = report
            .metrics
            .iter()
            .find(|m| m.stage == ComposeStage::TextReplacement)
            .unwrap();
        assert_eq!(metric.calls, 1);
    }

    #[test]
    fn disabled_measure_still_runs_closure() {
        let mut collector = PerfCollector::new(false);
        let result = collector.measure(PerfMetricKind::TextReplacement, || 99);
        assert_eq!(result, 99);
        assert!(collector.finish().is_none());
    }

    #[test]
    fn frontmatter_stages_in_pipeline_order() {
        // The pipeline runs Frontmatter Interpolation, then Schema Validation,
        // then Frontmatter Shell Expansion (see compose/mod.rs). The perf report
        // must reflect that order so callers can read it as execution order.
        let report = PerfCollector::new(true).finish().unwrap();
        let stages: Vec<_> = report.metrics.iter().map(|m| m.stage).collect();
        let fi = stages
            .iter()
            .position(|s| *s == ComposeStage::FrontmatterInterpolation)
            .unwrap();
        let sv = stages
            .iter()
            .position(|s| *s == ComposeStage::SchemaValidation)
            .unwrap();
        let fse = stages
            .iter()
            .position(|s| *s == ComposeStage::FrontmatterShellExpansion)
            .unwrap();
        assert!(fi < sv, "FrontmatterInterpolation must precede SchemaValidation");
        assert!(
            sv < fse,
            "SchemaValidation must precede FrontmatterShellExpansion"
        );
    }

    #[test]
    fn metrics_in_pipeline_order() {
        let mut collector = PerfCollector::new(true);
        // Record in reverse order to verify output order is by pipeline, not insertion
        collector.record(PerfMetricKind::Normalization, Duration::from_millis(1));
        collector.record(
            PerfMetricKind::EffectiveStateBuild,
            Duration::from_millis(2),
        );

        let report = collector.finish().unwrap();
        let stages: Vec<_> = report.metrics.iter().map(|m| m.stage).collect();
        let esb_idx = stages
            .iter()
            .position(|s| *s == ComposeStage::EffectiveStateBuild);
        let norm_idx = stages
            .iter()
            .position(|s| *s == ComposeStage::Normalization);
        assert!(esb_idx < norm_idx);
    }
}
