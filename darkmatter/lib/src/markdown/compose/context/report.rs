//! Compose execution report (`ComposeReport`), source mapping (`SourceRange`),
//! and non-fatal `ComposeWarning`.

use super::super::cache::CacheStats;
use super::super::perf::ComposePerfReport;
use crate::markdown::normalize::NormalizationReport;
use crate::markdown::schemas::SchemaAdvisory;
use std::path::PathBuf;

/// Report of changes made during compose execution.
///
/// Contains counts of changes made by each stage and any warnings
/// generated during processing.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct ComposeReport {
    /// Number of frontmatter interpolation expressions resolved.
    pub frontmatter_interpolations_applied: usize,

    /// Number of frontmatter shell expansions applied.
    pub frontmatter_shell_expansions_applied: usize,

    /// Number of text replacements applied.
    pub replacements_applied: usize,

    /// Number of interpolations resolved.
    pub interpolations_applied: usize,

    /// Number of toc-linking directives expanded.
    pub toc_links_generated: usize,

    /// Number of shell expansions applied.
    pub shell_expansions_applied: usize,

    /// Number of shell blocks applied.
    pub shell_blocks_applied: usize,

    /// Number of shell approvals used.
    pub shell_approvals_used: usize,

    /// Whether the cleanup stage modified the content.
    pub cleanup_changed: bool,

    /// Normalization report if normalization was performed.
    pub normalization_report: Option<NormalizationReport>,

    /// Number of page blocks that evaluated to true and were rendered.
    pub page_blocks_rendered: usize,

    /// Number of page blocks that evaluated to false and were skipped.
    pub page_blocks_skipped: usize,

    /// Number of transclusions applied.
    pub transclusions_applied: usize,

    /// Number of transclusions skipped (conditions/invalid ignored).
    pub transclusions_skipped: usize,

    /// Number of local links resolved to absolute paths.
    pub link_resolves_applied: usize,

    /// Number of absolute paths normalized to portable forms.
    pub link_normalizations_applied: usize,

    /// Maximum recursive transclusion depth observed.
    pub max_transclusion_depth: usize,

    /// Warnings generated during compose (non-fatal issues).
    pub warnings: Vec<ComposeWarning>,

    /// Cache statistics from this compose run.
    pub cache_stats: Option<CacheStats>,

    /// Performance timings when `ComposeOptions::perf_enabled` is `true`.
    pub perf: Option<ComposePerfReport>,

    /// Source map tracking which byte ranges came from transcluded files.
    pub source_map: Vec<SourceRange>,

    /// Remote-fetch statistics from this compose run.
    pub remote_fetch_stats: Option<super::super::remote_fetch::RemoteFetchStats>,

    /// Top-level frontmatter keys intentionally deferred from compose-time
    /// resolution (via `ComposeOptions::with_exclude_keys`). Their values
    /// survive raw in `effective_frontmatter` for caller-owned event-time
    /// interpolation. Lets callers distinguish "raw because deferred" from
    /// "raw because composition failed". Empty when no keys were deferred.
    pub deferred_frontmatter_keys: std::collections::HashSet<String>,
}

/// Maps a byte range in composed output to its originating source file.
///
/// Populated by `BlockTransclusion` when file content replaces a
/// `::file` directive. Byte positions refer to the final composed content.
#[derive(Debug, Clone, PartialEq)]
pub struct SourceRange {
    /// Start byte offset in the composed output (inclusive).
    pub byte_start: usize,
    /// End byte offset in the composed output (exclusive).
    pub byte_end: usize,
    /// The source file whose content occupies this range.
    pub source_file: PathBuf,
    /// The starting line number in the source file (1-based).
    pub source_start_line: usize,
}

impl ComposeReport {
    /// Creates a new empty report.
    pub fn new() -> Self {
        Self::default()
    }

    /// Returns true if any changes were made by any stage.
    pub fn has_changes(&self) -> bool {
        self.frontmatter_interpolations_applied > 0
            || self.frontmatter_shell_expansions_applied > 0
            || self.replacements_applied > 0
            || self.interpolations_applied > 0
            || self.toc_links_generated > 0
            || self.shell_expansions_applied > 0
            || self.shell_blocks_applied > 0
            || self.link_resolves_applied > 0
            || self.link_normalizations_applied > 0
            || self.cleanup_changed
            || self.page_blocks_rendered > 0
            || self.transclusions_applied > 0
            || self
                .normalization_report
                .as_ref()
                .is_some_and(|r| r.has_changes())
    }

    /// Returns a summary of changes made.
    pub fn summary(&self) -> String {
        if !self.has_changes() {
            return "No changes made".to_string();
        }

        let mut parts = Vec::new();

        if self.frontmatter_interpolations_applied > 0 {
            parts.push(format!(
                "{} frontmatter interpolation(s)",
                self.frontmatter_interpolations_applied
            ));
        }

        if self.frontmatter_shell_expansions_applied > 0 {
            parts.push(format!(
                "{} frontmatter shell expansion(s)",
                self.frontmatter_shell_expansions_applied
            ));
        }

        if self.replacements_applied > 0 {
            parts.push(format!("{} replacement(s)", self.replacements_applied));
        }

        if self.interpolations_applied > 0 {
            parts.push(format!("{} interpolation(s)", self.interpolations_applied));
        }

        if self.toc_links_generated > 0 {
            parts.push(format!("{} toc-link(s)", self.toc_links_generated));
        }

        if self.shell_expansions_applied > 0 {
            parts.push(format!(
                "{} shell expansion(s)",
                self.shell_expansions_applied
            ));
        }

        if self.shell_blocks_applied > 0 {
            parts.push(format!("{} shell block(s)", self.shell_blocks_applied));
        }

        if self.shell_approvals_used > 0 {
            parts.push(format!("{} shell approval(s)", self.shell_approvals_used));
        }

        if self.link_resolves_applied > 0 {
            parts.push(format!("{} link resolve(s)", self.link_resolves_applied));
        }

        if self.link_normalizations_applied > 0 {
            parts.push(format!(
                "{} link normalization(s)",
                self.link_normalizations_applied
            ));
        }

        if self.cleanup_changed {
            parts.push("cleanup applied".to_string());
        }

        if self.page_blocks_rendered > 0 {
            parts.push(format!(
                "{} page block(s) rendered",
                self.page_blocks_rendered
            ));
        }

        if self.page_blocks_skipped > 0 {
            parts.push(format!(
                "{} page block(s) skipped",
                self.page_blocks_skipped
            ));
        }

        if self.transclusions_applied > 0 {
            parts.push(format!("{} transclusion(s)", self.transclusions_applied));
        }

        if self.transclusions_skipped > 0 {
            parts.push(format!(
                "{} transclusion(s) skipped",
                self.transclusions_skipped
            ));
        }

        if let Some(ref norm) = self.normalization_report
            && norm.has_changes()
        {
            parts.push(format!("normalization: {}", norm.summary()));
        }

        if let Some(ref stats) = self.cache_stats
            && stats.has_activity()
        {
            parts.push(format!(
                "cache: {} hit(s), {} miss(es)",
                stats.hits, stats.misses
            ));
        }

        if let Some(ref rf_stats) = self.remote_fetch_stats
            && (rf_stats.fetched > 0 || rf_stats.cache_hits > 0 || rf_stats.not_modified > 0)
        {
            parts.push(format!(
                "remote: {} fetched, {} cached, {} revalidated ({} not-modified, {} stale), \
                 {} denied, {} failed",
                rf_stats.fetched,
                rf_stats.cache_hits,
                rf_stats.revalidations,
                rf_stats.not_modified,
                rf_stats.stale_served,
                rf_stats.policy_denials,
                rf_stats.failures
            ));
        }

        parts.join(", ")
    }

    /// Adds a warning to the report.
    pub fn add_warning(&mut self, warning: ComposeWarning) {
        self.warnings.push(warning);
    }

    /// Adds a schema advisory unless this report already carries the same
    /// semantic code and referenced path.
    pub(crate) fn add_schema_advisory(
        &mut self,
        advisory: &SchemaAdvisory,
        consumer: impl Into<PathBuf>,
    ) {
        let warning = ComposeWarning::from_schema_advisory(advisory, consumer);
        if self
            .warnings
            .iter()
            .any(|existing| existing.same_schema_advisory(&warning))
        {
            return;
        }
        self.warnings.push(warning);
    }

    /// Merges another report into this one.
    pub fn merge(&mut self, mut other: ComposeReport) {
        self.frontmatter_interpolations_applied += other.frontmatter_interpolations_applied;
        self.frontmatter_shell_expansions_applied += other.frontmatter_shell_expansions_applied;
        self.replacements_applied += other.replacements_applied;
        self.interpolations_applied += other.interpolations_applied;
        self.toc_links_generated += other.toc_links_generated;
        self.shell_expansions_applied += other.shell_expansions_applied;
        self.shell_blocks_applied += other.shell_blocks_applied;
        self.shell_approvals_used += other.shell_approvals_used;
        self.link_resolves_applied += other.link_resolves_applied;
        self.link_normalizations_applied += other.link_normalizations_applied;
        self.cleanup_changed |= other.cleanup_changed;
        self.page_blocks_rendered += other.page_blocks_rendered;
        self.page_blocks_skipped += other.page_blocks_skipped;
        self.transclusions_applied += other.transclusions_applied;
        self.transclusions_skipped += other.transclusions_skipped;
        self.max_transclusion_depth = self
            .max_transclusion_depth
            .max(other.max_transclusion_depth);

        if self.normalization_report.is_none() {
            self.normalization_report = other.normalization_report.take();
        }

        for warning in other.warnings.drain(..) {
            if warning.is_schema_advisory()
                && self
                    .warnings
                    .iter()
                    .any(|existing| existing.same_schema_advisory(&warning))
            {
                continue;
            }
            self.warnings.push(warning);
        }

        // Merge cache stats
        match (&mut self.cache_stats, other.cache_stats) {
            (Some(self_stats), Some(ref other_stats)) => self_stats.merge(other_stats),
            (None, Some(other_stats)) => self.cache_stats = Some(other_stats),
            _ => {}
        }

        // Merge perf reports
        match (&mut self.perf, other.perf) {
            (Some(self_perf), Some(ref other_perf)) => self_perf.merge(other_perf),
            (None, Some(other_perf)) => self.perf = Some(other_perf),
            _ => {}
        }

        // Deferred keys accumulate across recursive child pipelines.
        self.deferred_frontmatter_keys
            .extend(other.deferred_frontmatter_keys);
    }
}

/// A warning generated during compose processing.
///
/// Warnings indicate non-fatal issues that did not prevent the
/// transform from completing (when `fail_fast = false`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ComposeWarning {
    /// The stage that generated this warning.
    pub stage: String,

    /// Human-readable description of the issue.
    pub message: String,

    /// Line number where the issue occurred (1-indexed), if applicable.
    pub line_number: Option<usize>,

    /// Stable producer identity when the warning originates from a typed
    /// diagnostic source.
    pub source: Option<String>,

    /// Stable machine-readable code when available.
    pub code: Option<String>,

    /// Referenced file associated with the warning, when applicable.
    pub path: Option<PathBuf>,

    /// Root document that consumed the advisory.
    pub consumer: Option<PathBuf>,
}

impl ComposeWarning {
    /// Creates a new warning.
    pub fn new(stage: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            stage: stage.into(),
            message: message.into(),
            line_number: None,
            source: None,
            code: None,
            path: None,
            consumer: None,
        }
    }

    /// Projects a typed schema advisory into the compose warning model.
    pub(crate) fn from_schema_advisory(
        advisory: &SchemaAdvisory,
        consumer: impl Into<PathBuf>,
    ) -> Self {
        let path = std::fs::canonicalize(advisory.path())
            .unwrap_or_else(|_| advisory.path().to_path_buf());
        Self {
            stage: "schema_validation".to_string(),
            message: advisory.message(),
            line_number: None,
            source: Some(advisory.source().to_string()),
            code: Some(advisory.code().to_string()),
            path: Some(path),
            consumer: Some(consumer.into()),
        }
    }

    /// Adds a line number to this warning.
    #[must_use]
    pub fn at_line(mut self, line: usize) -> Self {
        self.line_number = Some(line);
        self
    }

    fn is_schema_advisory(&self) -> bool {
        self.source.as_deref() == Some(SchemaAdvisory::SOURCE) && self.code.is_some()
    }

    fn same_schema_advisory(&self, other: &Self) -> bool {
        self.source == other.source && self.code == other.code && self.path == other.path
    }
}
