//! Lightweight performance instrumentation for the compose pipeline.
//!
//! When `ComposeOptions::perf_enabled` is `true`, a `PerfCollector`
//! records per-stage timings. When disabled, all methods are no-ops.

use super::types::{ComposePerfMetric, ComposePerfReport, ComposeStage};
use std::time::{Duration, Instant};

/// Metric kinds corresponding to compose pipeline stages.
///
/// Variants are listed in pipeline execution order so the final
/// report has a deterministic, intuitive ordering.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum PerfMetricKind {
    FrontmatterInterpolation,
    EffectiveStateBuild,
    TextReplacement,
    PageBlocks,
    Interpolation,
    ShellExpansion,
    TransclusionParse,
    TransclusionPrepare,
    TransclusionResolve,
    TransclusionApply,
    Cleanup,
    Normalization,
}

impl PerfMetricKind {
    /// Convert to the public `ComposeStage` enum.
    fn stage(self) -> ComposeStage {
        match self {
            Self::FrontmatterInterpolation => ComposeStage::FrontmatterInterpolation,
            Self::EffectiveStateBuild => ComposeStage::EffectiveStateBuild,
            Self::TextReplacement => ComposeStage::TextReplacement,
            Self::PageBlocks => ComposeStage::PageBlocks,
            Self::Interpolation => ComposeStage::Interpolation,
            Self::ShellExpansion => ComposeStage::ShellExpansion,
            Self::TransclusionParse => ComposeStage::TransclusionParse,
            Self::TransclusionPrepare => ComposeStage::TransclusionPrepare,
            Self::TransclusionResolve => ComposeStage::TransclusionResolve,
            Self::TransclusionApply => ComposeStage::TransclusionApply,
            Self::Cleanup => ComposeStage::Cleanup,
            Self::Normalization => ComposeStage::Normalization,
        }
    }

    /// All variants in pipeline execution order.
    fn all() -> &'static [PerfMetricKind] {
        &[
            Self::FrontmatterInterpolation,
            Self::EffectiveStateBuild,
            Self::TextReplacement,
            Self::PageBlocks,
            Self::Interpolation,
            Self::ShellExpansion,
            Self::TransclusionParse,
            Self::TransclusionPrepare,
            Self::TransclusionResolve,
            Self::TransclusionApply,
            Self::Cleanup,
            Self::Normalization,
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
    durations: [(Duration, usize); 12],
}

impl PerfCollector {
    /// Creates a new collector. When `enabled` is `false`, all
    /// recording methods are no-ops.
    pub(crate) fn new(enabled: bool) -> Self {
        Self {
            enabled,
            start: if enabled { Some(Instant::now()) } else { None },
            durations: [(Duration::ZERO, 0); 12],
        }
    }

    /// Returns `true` if perf collection is active.
    pub(crate) fn is_enabled(&self) -> bool {
        self.enabled
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

        Some(ComposePerfReport { total, metrics })
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
        let norm_idx = stages.iter().position(|s| *s == ComposeStage::Normalization);
        assert!(esb_idx < norm_idx);
    }
}
