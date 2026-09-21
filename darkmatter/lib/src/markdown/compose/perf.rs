//! Lightweight performance instrumentation for the compose pipeline.
//!
//! When `ComposeOptions::perf_enabled` is `true`, a `PerfCollector`
//! records per-stage timings. When disabled, all methods are no-ops.

use super::pipeline::operations::{ComposeOperationPerfMetric, ComposePhase};
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

impl ComposeOperationPerfMetric {
    /// Convert the operation-level metric to the runner's `PerfMetricKind`.
    pub(crate) fn to_perf_metric_kind(self) -> PerfMetricKind {
        match self {
            Self::FrontmatterInterpolation => PerfMetricKind::FrontmatterInterpolation,
            Self::FrontmatterShellExpansion => PerfMetricKind::FrontmatterShellExpansion,
            Self::TextReplacement => PerfMetricKind::TextReplacement,
            Self::PageBlocks => PerfMetricKind::PageBlocks,
            Self::Interpolation => PerfMetricKind::Interpolation,
            Self::ShellExpansion => PerfMetricKind::ShellExpansion,
            Self::ShellBlocks => PerfMetricKind::ShellBlocks,
            Self::LinkResolve => PerfMetricKind::LinkResolve,
            Self::Cleanup => PerfMetricKind::Cleanup,
            Self::Normalization => PerfMetricKind::Normalization,
            Self::LinkNormalization => PerfMetricKind::LinkNormalization,
        }
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

// ── Compose perf report types (moved from types.rs) ──────────────────
/// A timing span for a single executed `::shell` directive (DM-3).
///
/// `command_display` is redacted, whitespace-normalized, and length-capped
/// (OQ-2 Option B); `command_hash` is a stable non-crypto xxHash of the raw
/// command for local correlation without exposing the full text.
///
/// ## Notes
///
/// Only the timing fields are carried. A span is recorded only on the
/// success path (the executor returns `Err` for a non-zero exit, which
/// propagates before the span is taken), and shell results are not cached,
/// so neither an exit status nor a cache flag would carry signal here.
/// Surfacing them accurately would require widening the directive executor's
/// return type — deliberately out of scope (NG-1).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShellCommandSpan {
    /// Redacted, whitespace-normalized, length-capped command text.
    pub command_display: String,
    /// Lowercase hex xxHash of the raw (un-redacted) command.
    pub command_hash: String,
    /// Wall-clock time spent executing this directive.
    pub elapsed: Duration,
}

/// Redact, whitespace-normalize, and length-cap a raw shell command for
/// display (OQ-2 Option B).
///
/// Collapses whitespace, masks common secret/credential patterns with `***`,
/// and truncates the final string to 80 display characters (appending `…`).
///
/// ## Examples
///
/// ```
/// use darkmatter::markdown::compose::redact_shell_command;
///
/// let out = redact_shell_command("curl -H 'Authorization: Bearer abc123'");
/// assert!(out.contains("Bearer ***"));
/// ```
pub fn redact_shell_command(raw: &str) -> String {
    use std::sync::LazyLock;

    // Authorization headers / bearer tokens.
    static BEARER_RE: LazyLock<regex::Regex> = LazyLock::new(|| {
        regex::Regex::new(r"(?i)bearer\s+[^\s'\x22]+").expect("valid bearer regex")
    });
    // Secret-carrying flags in `--flag=VALUE` form.
    static FLAG_EQ_RE: LazyLock<regex::Regex> = LazyLock::new(|| {
        regex::Regex::new(r"(?i)(--(?:token|password|api-?key|secret))=\S+")
            .expect("valid flag-eq regex")
    });
    // Secret-carrying flags in `--flag VALUE` (space-separated) form.
    static FLAG_SP_RE: LazyLock<regex::Regex> = LazyLock::new(|| {
        regex::Regex::new(r"(?i)(--(?:token|password|api-?key|secret))(\s+)\S+")
            .expect("valid flag-space regex")
    });
    // URL credentials: scheme://user:pass@host.
    static URL_CRED_RE: LazyLock<regex::Regex> = LazyLock::new(|| {
        regex::Regex::new(r"([a-zA-Z][a-zA-Z0-9+.\-]*://)[^/\s:@]+:[^/\s@]+@")
            .expect("valid url-cred regex")
    });
    // Query-string secrets: ?token=… / &access_token=… / &password=… / &key=….
    static QUERY_RE: LazyLock<regex::Regex> = LazyLock::new(|| {
        regex::Regex::new(r"(?i)([?&](?:access_token|token|password|api-?key|secret|key)=)[^&\s'\x22]+")
            .expect("valid query regex")
    });
    // Long opaque token-like blobs (JWT / base64): mixes letters and digits,
    // no slash, length >= 40.
    static BLOB_RE: LazyLock<regex::Regex> = LazyLock::new(|| {
        regex::Regex::new(r"[A-Za-z0-9_\-.+=]{40,}").expect("valid blob regex")
    });

    // Collapse all whitespace runs to single spaces and trim.
    let normalized = raw.split_whitespace().collect::<Vec<_>>().join(" ");

    let mut out = normalized;
    out = BEARER_RE.replace_all(&out, "Bearer ***").into_owned();
    out = FLAG_EQ_RE.replace_all(&out, "$1=***").into_owned();
    out = FLAG_SP_RE.replace_all(&out, "$1$2***").into_owned();
    out = URL_CRED_RE.replace_all(&out, "$1***@").into_owned();
    out = QUERY_RE.replace_all(&out, "$1***").into_owned();
    // Only redact blobs that look token-like: contain both a letter and a
    // digit. This avoids masking ordinary long words while catching JWTs and
    // base64 secrets. (Blobs containing `/` are already excluded by the class.)
    out = BLOB_RE
        .replace_all(&out, |caps: &regex::Captures<'_>| {
            let m = &caps[0];
            let has_alpha = m.bytes().any(|b| b.is_ascii_alphabetic());
            let has_digit = m.bytes().any(|b| b.is_ascii_digit());
            if has_alpha && has_digit {
                "***".to_string()
            } else {
                m.to_string()
            }
        })
        .into_owned();

    // Length-cap the final redacted string to 80 display chars.
    const MAX_CHARS: usize = 80;
    if out.chars().count() > MAX_CHARS {
        let truncated: String = out.chars().take(MAX_CHARS).collect();
        format!("{truncated}…")
    } else {
        out
    }
}

/// A single timing metric from the compose pipeline.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ComposePerfMetric {
    /// Pipeline stage this metric represents.
    pub stage: ComposeStage,
    /// Accumulated elapsed time for this metric.
    pub elapsed: Duration,
    /// Number of times this metric was recorded.
    pub calls: usize,
}

/// Aggregated performance timings from the compose pipeline.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ComposePerfReport {
    /// Total compose pipeline time.
    pub total: Duration,
    /// Per-stage metrics in deterministic order.
    pub metrics: Vec<ComposePerfMetric>,
    /// Per-`::shell`-directive timing spans (DM-3). Populated only when
    /// perf collection is enabled.
    pub shell_spans: Vec<ShellCommandSpan>,
    /// Per-group context-capture timings (DM-4), as `(group_name, elapsed)`.
    /// Populated only when perf collection is enabled.
    pub capture_timings: Vec<(String, Duration)>,
}

/// Named compose pipeline stages for type-safe metric identification.
///
/// Variants are listed in pipeline execution order so reports have
/// a deterministic, intuitive ordering.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ComposeStage {
    FrontmatterInterpolation,
    SchemaValidation,
    FrontmatterShellExpansion,
    EffectiveStateBuild,
    TextReplacement,
    PageBlocks,
    Interpolation,
    ShellExpansion,
    ShellBlocks,
    TransclusionParse,
    TransclusionPrepare,
    TransclusionResolve,
    TransclusionApply,
    LinkResolve,
    LinkNormalization,
    Cleanup,
    Normalization,
}

impl std::fmt::Display for ComposeStage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::FrontmatterInterpolation => "frontmatter interpolation",
            Self::SchemaValidation => "schema validation",
            Self::FrontmatterShellExpansion => "frontmatter shell expansion",
            Self::EffectiveStateBuild => "effective state build",
            Self::TextReplacement => "text replacement",
            Self::PageBlocks => "page blocks",
            Self::Interpolation => "interpolation",
            Self::ShellExpansion => "shell expansion",
            Self::ShellBlocks => "shell blocks",
            Self::TransclusionParse => "transclusion parse",
            Self::TransclusionPrepare => "transclusion prepare",
            Self::TransclusionResolve => "transclusion resolve",
            Self::TransclusionApply => "transclusion apply",
            Self::LinkResolve => "link resolve",
            Self::LinkNormalization => "link normalization",
            Self::Cleanup => "cleanup",
            Self::Normalization => "normalization",
        })
    }
}

impl ComposeStage {
    /// Returns the `ComposePhase` this stage belongs to (DM-2).
    ///
    /// Feeds the claudine perf tree, where the 17 flat stages nest under
    /// their four phases.
    pub fn phase(&self) -> ComposePhase {
        match self {
            Self::FrontmatterInterpolation
            | Self::SchemaValidation
            | Self::FrontmatterShellExpansion
            | Self::EffectiveStateBuild
            | Self::TextReplacement
            | Self::PageBlocks
            | Self::Interpolation
            | Self::ShellExpansion
            | Self::ShellBlocks
            | Self::LinkResolve => ComposePhase::InlinePre,

            Self::TransclusionParse
            | Self::TransclusionPrepare
            | Self::TransclusionResolve
            | Self::TransclusionApply => ComposePhase::Transclusion,

            Self::Cleanup | Self::Normalization => ComposePhase::InlinePost,

            Self::LinkNormalization => ComposePhase::Finalization,
        }
    }
}

impl ComposePerfReport {
    /// Creates an empty perf report.
    pub fn new() -> Self {
        Self {
            total: Duration::ZERO,
            metrics: Vec::new(),
            shell_spans: Vec::new(),
            capture_timings: Vec::new(),
        }
    }

    /// Merges another perf report into this one by summing matching
    /// metric durations and call counts.
    pub fn merge(&mut self, other: &ComposePerfReport) {
        self.total += other.total;

        for other_metric in &other.metrics {
            if let Some(existing) = self
                .metrics
                .iter_mut()
                .find(|m| m.stage == other_metric.stage)
            {
                existing.elapsed += other_metric.elapsed;
                existing.calls += other_metric.calls;
            } else {
                self.metrics.push(*other_metric);
            }
        }

        self.shell_spans
            .extend(other.shell_spans.iter().cloned());
        self.capture_timings
            .extend(other.capture_timings.iter().cloned());
    }
}

impl Default for ComposePerfReport {
    fn default() -> Self {
        Self::new()
    }
}
