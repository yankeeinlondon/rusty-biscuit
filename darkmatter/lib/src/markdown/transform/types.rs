//! Type definitions for the transform pipeline.
//!
//! This module contains the core types used by the transform pipeline:
//! - `TransformOptions` - Configuration for transform execution
//! - `TransformContext` - Runtime context captured at transform start
//! - `TransformReport` - Results and diagnostics from transform execution
//! - `Stage1Stages` - Toggle flags for Stage 1 preparation stages
//! - `Stage2Stages` - Toggle flags for Stage 2 transclusion stages

use super::super::normalize::NormalizationReport;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use url::Url;

/// Configuration options for the transform pipeline.
///
/// This struct controls which stages run and provides external state
/// for interpolation and replacement operations.
///
/// ## Construction
///
/// Always use `TransformOptions::new()` to construct, which captures
/// runtime context (current time, environment variables) at creation.
///
/// ## Examples
///
/// ```
/// use darkmatter::markdown::transform::TransformOptions;
///
/// // Default options with all stages enabled
/// let options = TransformOptions::new();
///
/// // Disable cleanup stage
/// let options = TransformOptions::new()
///     .with_stages(darkmatter::markdown::transform::Stage1Stages {
///         cleanup: false,
///         ..Default::default()
///     });
/// ```
#[derive(Clone)]
pub struct TransformOptions {
    /// Controls which Stage 1 stages are enabled.
    pub stages: Stage1Stages,

    /// Controls which Stage 2 stages are enabled.
    pub stage2: Stage2Stages,

    /// Stage 2 transclusion options.
    pub transclusion: TransclusionOptions,

    /// Shell expansion options.
    pub shell: super::shell_expansion::ShellExpansionOptions,

    /// External state to merge with frontmatter for interpolation/replacement.
    ///
    /// When present, this state is merged with document frontmatter using
    /// deep merge semantics in the effective state builder.
    pub external_state: Option<serde_json::Value>,

    /// Override values that overwrite existing frontmatter keys.
    ///
    /// Unlike `external_state` which only fills in missing/null keys,
    /// these values unconditionally overwrite frontmatter properties.
    pub set_overrides: Option<serde_json::Value>,

    /// If true, the pipeline returns an error on first failure.
    /// If false, failures are recorded as warnings and the pipeline continues.
    pub fail_fast: bool,

    /// Internal flag: when true, external `replace` keys override document
    /// `replace` keys for this transform invocation only.
    pub(crate) replace_parent_wins: bool,

    /// Internal one-off replace map applied only to this document's
    /// replacement stage (never propagated to children).
    pub(crate) one_off_replace: Option<serde_json::Map<String, serde_json::Value>>,

    /// Runtime context captured at construction time.
    context: TransformContext,
}

impl std::fmt::Debug for TransformOptions {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TransformOptions")
            .field("stages", &self.stages)
            .field("stage2", &self.stage2)
            .field("transclusion", &self.transclusion)
            .field("shell", &self.shell)
            .field("external_state", &self.external_state)
            .field("set_overrides", &self.set_overrides)
            .field("fail_fast", &self.fail_fast)
            .field("replace_parent_wins", &self.replace_parent_wins)
            .field("one_off_replace", &self.one_off_replace)
            .field("context", &self.context)
            .finish()
    }
}

impl TransformOptions {
    /// Creates new transform options with default stages and captured context.
    ///
    /// This is the only way to construct `TransformOptions` because the
    /// runtime context must be captured at a known point in time for
    /// deterministic output.
    pub fn new() -> Self {
        Self {
            stages: Stage1Stages::default(),
            stage2: Stage2Stages::default(),
            transclusion: TransclusionOptions::default(),
            shell: super::shell_expansion::ShellExpansionOptions::default(),
            external_state: None,
            set_overrides: None,
            fail_fast: false,
            replace_parent_wins: false,
            one_off_replace: None,
            context: TransformContext::capture(),
        }
    }

    /// Returns a reference to the captured runtime context.
    pub fn context(&self) -> &TransformContext {
        &self.context
    }

    /// Sets the stages configuration.
    #[must_use]
    pub fn with_stages(mut self, stages: Stage1Stages) -> Self {
        self.stages = stages;
        self
    }

    /// Sets the Stage 2 stages configuration.
    #[must_use]
    pub fn with_stage2(mut self, stage2: Stage2Stages) -> Self {
        self.stage2 = stage2;
        self
    }

    /// Sets transclusion options.
    #[must_use]
    pub fn with_transclusion(mut self, transclusion: TransclusionOptions) -> Self {
        self.transclusion = transclusion;
        self
    }

    /// Sets shell expansion options.
    #[must_use]
    pub fn with_shell(mut self, shell: super::shell_expansion::ShellExpansionOptions) -> Self {
        self.shell = shell;
        self
    }

    /// Sets the transform source as a file path.
    #[must_use]
    pub fn with_source_file(mut self, path: impl Into<PathBuf>) -> Self {
        self.transclusion.source = TransformSource::File(path.into());
        self
    }

    /// Sets the transform source as a URL.
    #[must_use]
    pub fn with_source_url(mut self, url: Url) -> Self {
        self.transclusion.source = TransformSource::Url(url);
        self
    }

    /// Sets the external state for interpolation/replacement.
    #[must_use]
    pub fn with_external_state(mut self, state: serde_json::Value) -> Self {
        self.external_state = Some(state);
        self
    }

    /// Sets override values that overwrite existing frontmatter keys.
    #[must_use]
    pub fn with_set_overrides(mut self, overrides: serde_json::Value) -> Self {
        self.set_overrides = Some(overrides);
        self
    }

    /// Sets fail-fast mode.
    #[must_use]
    pub fn with_fail_fast(mut self, fail_fast: bool) -> Self {
        self.fail_fast = fail_fast;
        self
    }

    /// Internal builder: toggles parent-wins behavior for the `replace` map.
    #[must_use]
    pub(crate) fn with_replace_parent_wins(mut self, enabled: bool) -> Self {
        self.replace_parent_wins = enabled;
        self
    }

    /// Internal builder: sets a one-off replace map for this document only.
    #[must_use]
    pub(crate) fn with_one_off_replace(
        mut self,
        replace: Option<serde_json::Map<String, serde_json::Value>>,
    ) -> Self {
        self.one_off_replace = replace;
        self
    }

    /// Creates options with a specific context (for testing).
    #[cfg(test)]
    pub(crate) fn with_context(mut self, context: TransformContext) -> Self {
        self.context = context;
        self
    }
}

impl Default for TransformOptions {
    fn default() -> Self {
        Self::new()
    }
}

/// Controls which Stage 1 stages are enabled.
///
/// By default, all stages are enabled. Disable individual stages
/// for partial processing or debugging.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Stage1Stages {
    /// Text replacement stage (frontmatter `replace` map).
    pub replacement: bool,

    /// Frontmatter interpolation stage (`{{variable}}` expansion).
    pub interpolation: bool,

    /// TOC linking stage (`::toc-linking` directive expansion).
    pub toc_linking: bool,

    /// Shell expansion stage (`::shell` directive execution).
    pub shell_expansion: bool,

    /// Markdown cleanup stage (formatting normalization).
    pub cleanup: bool,

    /// Heading normalization stage (level adjustment).
    pub normalization: bool,
}

impl Default for Stage1Stages {
    fn default() -> Self {
        Self {
            replacement: true,
            interpolation: true,
            toc_linking: true,
            shell_expansion: true,
            cleanup: true,
            normalization: true,
        }
    }
}

impl Stage1Stages {
    /// Creates stages with all disabled.
    pub fn none() -> Self {
        Self {
            replacement: false,
            interpolation: false,
            toc_linking: false,
            shell_expansion: false,
            cleanup: false,
            normalization: false,
        }
    }

    /// Creates stages with only the specified stages enabled.
    pub fn only_replacement() -> Self {
        Self {
            replacement: true,
            ..Self::none()
        }
    }

    /// Creates stages with only interpolation enabled.
    pub fn only_interpolation() -> Self {
        Self {
            interpolation: true,
            ..Self::none()
        }
    }

    /// Creates stages with only toc_linking enabled.
    pub fn only_toc_linking() -> Self {
        Self {
            toc_linking: true,
            ..Self::none()
        }
    }

    /// Creates stages with only shell_expansion enabled.
    pub fn only_shell_expansion() -> Self {
        Self {
            shell_expansion: true,
            ..Self::none()
        }
    }

    /// Creates stages with only cleanup enabled.
    pub fn only_cleanup() -> Self {
        Self {
            cleanup: true,
            ..Self::none()
        }
    }

    /// Creates stages with only normalization enabled.
    pub fn only_normalization() -> Self {
        Self {
            normalization: true,
            ..Self::none()
        }
    }
}

/// Controls which Stage 2 transclusion stages are enabled.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Stage2Stages {
    /// Page block conditional regions (`::block` / `::end-block`).
    pub page_blocks: bool,

    /// Block directive transclusion (`::file`, `::code`, `::url`).
    pub block_transclusion: bool,

    /// Frontmatter transclusion (`prologue`, `epilogue`).
    pub fm_transclusion: bool,
}

impl Default for Stage2Stages {
    fn default() -> Self {
        Self {
            page_blocks: true,
            block_transclusion: true,
            fm_transclusion: true,
        }
    }
}

impl Stage2Stages {
    /// Creates stages with all disabled.
    pub fn none() -> Self {
        Self {
            page_blocks: false,
            block_transclusion: false,
            fm_transclusion: false,
        }
    }

    /// Creates stages with only page blocks enabled.
    pub fn only_page_blocks() -> Self {
        Self {
            page_blocks: true,
            ..Self::none()
        }
    }

    /// Creates stages with only block transclusion enabled.
    pub fn only_block() -> Self {
        Self {
            block_transclusion: true,
            ..Self::none()
        }
    }

    /// Creates stages with only frontmatter transclusion enabled.
    pub fn only_frontmatter() -> Self {
        Self {
            fm_transclusion: true,
            ..Self::none()
        }
    }
}

/// Source context for transform execution.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TransformSource {
    /// Source is unknown (e.g., in-memory string).
    Unknown,
    /// Source is a local file.
    File(PathBuf),
    /// Source is a URL.
    Url(Url),
}

impl TransformSource {
    /// Creates a file source from a path reference.
    pub fn infer_from_path(path: impl AsRef<Path>) -> Self {
        Self::File(path.as_ref().to_path_buf())
    }
}

/// Transclusion-specific options for Stage 2.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TransclusionOptions {
    /// Source context of the document being transformed.
    pub source: TransformSource,

    /// Maximum recursive include depth.
    pub max_depth: usize,

    /// Whether remote transclusion is allowed.
    pub allow_remote: bool,

    /// Whether local markdown files can be included with `::file`.
    pub allow_local_markdown: bool,

    /// Whether local text files can be included with `::code`.
    pub allow_local_code_text: bool,

    /// Fallback code language for unknown extensions.
    pub code_fallback_language: String,

    /// Explicit override for invalid-reference behavior.
    pub ignore_invalid: Option<bool>,

    /// Whether repo-root (`@`) resolution is enabled.
    pub resolve_repo_root: bool,
}

impl Default for TransclusionOptions {
    fn default() -> Self {
        Self {
            source: TransformSource::Unknown,
            max_depth: 16,
            allow_remote: false,
            allow_local_markdown: true,
            allow_local_code_text: true,
            code_fallback_language: "txt".to_string(),
            ignore_invalid: None,
            resolve_repo_root: true,
        }
    }
}

/// Runtime context captured at transform start for deterministic output.
///
/// All date/time values are captured once when the context is created,
/// ensuring consistent values throughout the transform pipeline even
/// if the transform takes significant time.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TransformContext {
    /// ISO 8601 local datetime (e.g., "2024-01-15T14:30:00").
    pub now: String,

    /// ISO 8601 UTC datetime (e.g., "2024-01-15T22:30:00Z").
    pub utc: String,

    /// Local date in YYYY-MM-DD format.
    pub today: String,

    /// Yesterday's date in YYYY-MM-DD format.
    pub yesterday: String,

    /// Tomorrow's date in YYYY-MM-DD format.
    pub tomorrow: String,

    /// Full day of week name (e.g., "Monday").
    pub dow: String,

    /// Abbreviated day of week (e.g., "Mon").
    pub dow_abbr: String,

    /// Four-digit year as string.
    pub year: String,

    /// Two-digit month as string (01-12).
    pub month: String,

    /// Full month name (e.g., "January").
    pub month_name: String,

    /// Abbreviated month name (e.g., "Jan").
    pub month_name_abbr: String,

    /// Environment variables snapshot.
    pub env: HashMap<String, String>,
}

impl TransformContext {
    /// Captures the current runtime context.
    ///
    /// This snapshots:
    /// - Current local and UTC time
    /// - Today, yesterday, and tomorrow dates
    /// - Day of week (full and abbreviated)
    /// - Year, month (numeric and named)
    /// - All environment variables
    pub fn capture() -> Self {
        use chrono::{Local, Utc};

        let now_local = Local::now();
        let now_utc = Utc::now();

        let today = now_local.date_naive();
        let yesterday = today - chrono::Duration::days(1);
        let tomorrow = today + chrono::Duration::days(1);

        Self {
            now: now_local.format("%Y-%m-%dT%H:%M:%S").to_string(),
            utc: now_utc.format("%Y-%m-%dT%H:%M:%SZ").to_string(),
            today: today.format("%Y-%m-%d").to_string(),
            yesterday: yesterday.format("%Y-%m-%d").to_string(),
            tomorrow: tomorrow.format("%Y-%m-%d").to_string(),
            dow: now_local.format("%A").to_string(),
            dow_abbr: now_local.format("%a").to_string(),
            year: now_local.format("%Y").to_string(),
            month: now_local.format("%m").to_string(),
            month_name: now_local.format("%B").to_string(),
            month_name_abbr: now_local.format("%b").to_string(),
            env: std::env::vars().collect(),
        }
    }

    /// Creates a context with fixed values for testing.
    #[cfg(test)]
    pub fn fixed_for_testing() -> Self {
        Self {
            now: "2024-06-15T10:30:00".to_string(),
            utc: "2024-06-15T17:30:00Z".to_string(),
            today: "2024-06-15".to_string(),
            yesterday: "2024-06-14".to_string(),
            tomorrow: "2024-06-16".to_string(),
            dow: "Saturday".to_string(),
            dow_abbr: "Sat".to_string(),
            year: "2024".to_string(),
            month: "06".to_string(),
            month_name: "June".to_string(),
            month_name_abbr: "Jun".to_string(),
            env: HashMap::new(),
        }
    }
}

/// Report of changes made during transform execution.
///
/// Contains counts of changes made by each stage and any warnings
/// generated during processing.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct TransformReport {
    /// Number of text replacements applied.
    pub replacements_applied: usize,

    /// Number of interpolations resolved.
    pub interpolations_applied: usize,

    /// Number of toc-linking directives expanded.
    pub toc_links_generated: usize,

    /// Number of shell expansions applied.
    pub shell_expansions_applied: usize,

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

    /// Maximum recursive transclusion depth observed.
    pub max_transclusion_depth: usize,

    /// Warnings generated during transform (non-fatal issues).
    pub warnings: Vec<TransformWarning>,
}

impl TransformReport {
    /// Creates a new empty report.
    pub fn new() -> Self {
        Self::default()
    }

    /// Returns true if any changes were made by any stage.
    pub fn has_changes(&self) -> bool {
        self.replacements_applied > 0
            || self.interpolations_applied > 0
            || self.toc_links_generated > 0
            || self.shell_expansions_applied > 0
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

        if self.shell_approvals_used > 0 {
            parts.push(format!("{} shell approval(s)", self.shell_approvals_used));
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

        parts.join(", ")
    }

    /// Adds a warning to the report.
    pub fn add_warning(&mut self, warning: TransformWarning) {
        self.warnings.push(warning);
    }
}

/// A warning generated during transform processing.
///
/// Warnings indicate non-fatal issues that did not prevent the
/// transform from completing (when `fail_fast = false`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TransformWarning {
    /// The stage that generated this warning.
    pub stage: String,

    /// Human-readable description of the issue.
    pub message: String,

    /// Line number where the issue occurred (1-indexed), if applicable.
    pub line_number: Option<usize>,
}

impl TransformWarning {
    /// Creates a new warning.
    pub fn new(stage: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            stage: stage.into(),
            message: message.into(),
            line_number: None,
        }
    }

    /// Adds a line number to this warning.
    #[must_use]
    pub fn at_line(mut self, line: usize) -> Self {
        self.line_number = Some(line);
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_transform_options_new_captures_context() {
        let options = TransformOptions::new();
        let ctx = options.context();

        // Context should have captured current date
        assert!(!ctx.today.is_empty());
        assert!(!ctx.year.is_empty());
        assert!(!ctx.dow.is_empty());
    }

    #[test]
    fn test_transform_options_default_stages() {
        let options = TransformOptions::new();

        assert!(options.stages.replacement);
        assert!(options.stages.interpolation);
        assert!(options.stages.cleanup);
        assert!(options.stages.normalization);
        assert!(options.stage2.block_transclusion);
        assert!(options.stage2.fm_transclusion);
    }

    #[test]
    fn test_transclusion_options_defaults() {
        let options = TransformOptions::new();
        assert_eq!(options.transclusion.max_depth, 16);
        assert!(matches!(
            options.transclusion.source,
            TransformSource::Unknown
        ));
        assert_eq!(options.transclusion.code_fallback_language, "txt");
    }

    #[test]
    fn test_transform_options_builder_pattern() {
        let options = TransformOptions::new()
            .with_stages(Stage1Stages {
                cleanup: false,
                ..Default::default()
            })
            .with_stage2(Stage2Stages::only_block())
            .with_transclusion(TransclusionOptions {
                max_depth: 8,
                ..Default::default()
            })
            .with_fail_fast(true)
            .with_external_state(serde_json::json!({"key": "value"}));

        assert!(!options.stages.cleanup);
        assert!(options.stage2.block_transclusion);
        assert!(!options.stage2.fm_transclusion);
        assert_eq!(options.transclusion.max_depth, 8);
        assert!(options.fail_fast);
        assert!(options.external_state.is_some());
    }

    #[test]
    fn test_transform_options_with_context() {
        let fixed_ctx = TransformContext::fixed_for_testing();
        let options = TransformOptions::new().with_context(fixed_ctx.clone());

        assert_eq!(options.context().today, "2024-06-15");
        assert_eq!(options.context().year, "2024");
    }

    #[test]
    fn test_stage1_stages_default_all_enabled() {
        let stages = Stage1Stages::default();

        assert!(stages.replacement);
        assert!(stages.interpolation);
        assert!(stages.cleanup);
        assert!(stages.normalization);
    }

    #[test]
    fn test_stage1_stages_none() {
        let stages = Stage1Stages::none();

        assert!(!stages.replacement);
        assert!(!stages.interpolation);
        assert!(!stages.cleanup);
        assert!(!stages.normalization);
    }

    #[test]
    fn test_stage1_stages_only_methods() {
        let r = Stage1Stages::only_replacement();
        assert!(r.replacement && !r.interpolation && !r.cleanup && !r.normalization);

        let i = Stage1Stages::only_interpolation();
        assert!(!i.replacement && i.interpolation && !i.cleanup && !i.normalization);

        let c = Stage1Stages::only_cleanup();
        assert!(!c.replacement && !c.interpolation && c.cleanup && !c.normalization);

        let n = Stage1Stages::only_normalization();
        assert!(!n.replacement && !n.interpolation && !n.cleanup && n.normalization);
    }

    #[test]
    fn test_stage2_stages_only_methods() {
        let none = Stage2Stages::none();
        assert!(!none.block_transclusion);
        assert!(!none.fm_transclusion);

        let block = Stage2Stages::only_block();
        assert!(block.block_transclusion);
        assert!(!block.fm_transclusion);

        let fm = Stage2Stages::only_frontmatter();
        assert!(!fm.block_transclusion);
        assert!(fm.fm_transclusion);
    }

    #[test]
    fn test_transform_context_capture() {
        let ctx = TransformContext::capture();

        // Should have reasonable values
        assert!(ctx.year.parse::<i32>().is_ok());
        assert!(ctx.month.len() == 2);
        assert!(!ctx.today.is_empty());
        assert!(!ctx.yesterday.is_empty());
        assert!(!ctx.tomorrow.is_empty());
    }

    #[test]
    fn test_transform_context_fixed_for_testing() {
        let ctx = TransformContext::fixed_for_testing();

        assert_eq!(ctx.today, "2024-06-15");
        assert_eq!(ctx.yesterday, "2024-06-14");
        assert_eq!(ctx.tomorrow, "2024-06-16");
        assert_eq!(ctx.dow, "Saturday");
        assert_eq!(ctx.year, "2024");
        assert_eq!(ctx.month, "06");
    }

    #[test]
    fn test_transform_report_new() {
        let report = TransformReport::new();

        assert_eq!(report.replacements_applied, 0);
        assert_eq!(report.interpolations_applied, 0);
        assert!(!report.cleanup_changed);
        assert!(report.normalization_report.is_none());
        assert_eq!(report.transclusions_applied, 0);
        assert_eq!(report.transclusions_skipped, 0);
        assert_eq!(report.max_transclusion_depth, 0);
        assert!(report.warnings.is_empty());
    }

    #[test]
    fn test_transform_report_has_changes() {
        let mut report = TransformReport::new();
        assert!(!report.has_changes());

        report.replacements_applied = 1;
        assert!(report.has_changes());
    }

    #[test]
    fn test_transform_report_summary() {
        let mut report = TransformReport::new();
        assert_eq!(report.summary(), "No changes made");

        report.replacements_applied = 2;
        report.interpolations_applied = 3;
        report.cleanup_changed = true;
        report.transclusions_applied = 1;
        report.transclusions_skipped = 1;

        let summary = report.summary();
        assert!(summary.contains("2 replacement(s)"));
        assert!(summary.contains("3 interpolation(s)"));
        assert!(summary.contains("cleanup applied"));
        assert!(summary.contains("1 transclusion(s)"));
        assert!(summary.contains("1 transclusion(s) skipped"));
    }

    #[test]
    fn test_transform_warning() {
        let warning = TransformWarning::new("interpolation", "Missing variable: foo");
        assert_eq!(warning.stage, "interpolation");
        assert_eq!(warning.message, "Missing variable: foo");
        assert!(warning.line_number.is_none());

        let warning_with_line = warning.at_line(42);
        assert_eq!(warning_with_line.line_number, Some(42));
    }

    #[test]
    fn test_transform_report_add_warning() {
        let mut report = TransformReport::new();
        report.add_warning(TransformWarning::new("test", "test warning"));

        assert_eq!(report.warnings.len(), 1);
        assert_eq!(report.warnings[0].message, "test warning");
    }
}
