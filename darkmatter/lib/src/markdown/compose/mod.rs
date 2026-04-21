//! Compose pipeline for markdown document preparation and transclusion.
//!
//! This module provides the `compose()` family of methods on `Markdown`
//! for running operations in three phases:
//!
//! **Inline Pre** (serial):
//! 1. **Frontmatter Interpolation** - Resolve `{{variable}}` in frontmatter values
//! 2. **Frontmatter Shell Expansion** - Execute shell commands in frontmatter values
//! 3. **Text Replacement** - Replace literal strings from frontmatter `replace` map
//! 4. **Page Blocks** - Evaluate `::block`/`::end-block` conditional regions
//! 5. **Interpolation** - Expand `{{variable}}` expressions in body content
//! 6. **Shell Expansion** - Execute `::shell` directives with security controls
//!
//! **Transclusion** (concurrent execution after serial preparation):
//! 7. **Block Transclusion** - Include `::file`/`::url` referenced documents
//! 8. **Frontmatter Transclusion** - Prepend/append `prologue`/`epilogue` documents
//! 9. **Code Transclusion** - Include `::code` file content as fenced blocks
//! 10. **TOC Linking** - Expand `::toc-linking` directives into heading link lists
//!
//! **Inline Post** (serial):
//! 11. **Cleanup** - Normalize markdown formatting
//! 12. **Normalization** - Adjust heading levels
//!
//! ## Examples
//!
//! ```
//! use darkmatter::markdown::Markdown;
//! use darkmatter::markdown::compose::{ComposeOptions, ComposeOperation};
//!
//! let content = "# Hello\nWorld";
//! let mut md: Markdown = content.into();
//!
//! // Transform with default options (all operations enabled)
//! let report = md.compose_mut().unwrap();
//!
//! // Transform with specific operations disabled
//! let options = ComposeOptions::new()
//!     .disable(ComposeOperation::Cleanup)
//!     .disable(ComposeOperation::Normalization);
//! let report = md.compose_with(options).unwrap();
//! ```

pub(crate) mod cache;
pub mod conditions;
pub mod context;
mod frontmatter_interpolation;
pub(crate) mod frontmatter_shell_expansion;
pub(crate) mod parse_utils;
pub(crate) mod perf;
mod state;
mod types;

pub mod interpolation;
pub mod page_blocks;
pub mod replacement;
pub mod shell_expansion;
pub mod toc_linking;
pub mod transclusion;

pub use biscuit_file::PathPosition;
pub use cache::{CacheAccessMode, CacheFreshnessMode, CacheStats};
pub use context::ContextMergeDiagnostic;
pub use shell_expansion::ShellCommandOrigin;
pub use shell_expansion::ShellExpansionError;
pub use shell_expansion::ShellTimeoutBehavior;
pub use state::{EffectiveState, EffectiveStateBuilder};
pub use toc_linking::TocLinkingError;
pub use transclusion::TransclusionError;
pub use types::{
    ComposeContext, ComposeOperation, ComposeOperationSet, ComposeOptions, ComposePerfMetric,
    ComposePerfReport, ComposePhase, ComposeReport, ComposeSource, ComposeStage, ComposeWarning,
    SourceRange,
};

// Internal re-exports for crate modules that still use TransclusionOptions
pub(crate) use types::TransclusionOptions;

use super::Markdown;
use super::cleanup;
use super::normalize::{self, NormalizationError};
use super::types::{MarkdownError, MarkdownResult};
use serde_json::{Map, Value};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use tracing::{debug, info, instrument, trace, warn};

use cache::operation::CacheableOperation;
use shell_expansion::{apply_replacements_in_reverse, execute_directive_detailed};

/// Shorten an absolute path for display in diagnostics.
///
/// Tries to make the path relative to the git repo root discovered by
/// walking up from the file's parent directory. Falls back to `~/…` when
/// the path is under the user's home directory, or the absolute path
/// otherwise.
fn abbreviate_path(path: &Path) -> String {
    // Try git repo root first (walk up looking for .git)
    if let Some(root) = find_git_root_from(path)
        && let Ok(rel) = path.strip_prefix(&root)
    {
        return rel.display().to_string();
    }

    // Fall back to ~/… for paths under HOME
    if let Some(home) = dirs::home_dir()
        && let Ok(rel) = path.strip_prefix(&home)
    {
        return format!("~/{}", rel.display());
    }

    path.display().to_string()
}

/// Walk up from `start` (or its parent if it's a file) looking for a `.git`
/// directory, returning the repo root if found.
fn find_git_root_from(start: &Path) -> Option<PathBuf> {
    let mut dir = if start.is_dir() {
        start.to_path_buf()
    } else {
        start.parent()?.to_path_buf()
    };
    loop {
        if dir.join(".git").exists() {
            return Some(dir);
        }
        if !dir.pop() {
            return None;
        }
    }
}

/// Applies pre-effective-state frontmatter preparation shared by runtime
/// compose and shell-command discovery.
///
/// This mutates frontmatter with external-state defaults and `--set`
/// overrides using the same rules the real compose pipeline uses. When
/// requested, it also captures the post-merge/pre-interpolation string
/// snapshot used for frontmatter shell executable provenance checks.
pub(crate) fn prepare_frontmatter_for_compose(
    markdown: &mut Markdown,
    options: &ComposeOptions,
    capture_pre_interpolation_snapshot: bool,
) -> Option<HashMap<String, String>> {
    // Apply external state as defaults using deep-merge: nested keys
    // from external state fill in missing values at every level, not
    // just top-level keys. Frontmatter values take precedence.
    if let Some(external) = options.external_state.as_ref() {
        let fm = markdown.frontmatter_mut().as_map_mut();
        let current = Value::Object(fm.iter().map(|(k, v)| (k.clone(), v.clone())).collect());
        let merged = state::deep_merge(external, &current);
        if let Value::Object(map) = merged {
            *fm = map.into_iter().collect();
        }
    }

    // Apply set overrides: unconditionally overwrite frontmatter keys.
    if let Some(overrides) = options.set_overrides.as_ref().and_then(Value::as_object) {
        let fm = markdown.frontmatter_mut().as_map_mut();
        for (key, value) in overrides {
            fm.insert(key.clone(), value.clone());
        }
    }

    capture_pre_interpolation_snapshot.then(|| {
        markdown
            .frontmatter()
            .as_map()
            .iter()
            .filter_map(|(key, value)| value.as_str().map(|s| (key.clone(), s.to_string())))
            .collect()
    })
}

#[derive(Clone)]
enum PreparedTransclusion {
    FixedReplace {
        order: usize,
        span: std::ops::Range<usize>,
        replacement: String,
        report: ComposeReport,
    },
    FixedSection {
        order: usize,
        slot: SectionSlot,
        content: Option<String>,
        report: ComposeReport,
    },
    Markdown {
        order: usize,
        target: ApplyTarget,
        path: PathBuf,
        directive_options: transclusion::BlockOptions,
        insertion_context: Option<(usize, usize)>,
    },
    Code {
        order: usize,
        span: std::ops::Range<usize>,
        path: PathBuf,
        directive_options: transclusion::BlockOptions,
        line: usize,
    },
    Toc {
        order: usize,
        span: std::ops::Range<usize>,
        directive: toc_linking::TocLinkingDirective,
    },
}

#[derive(Clone, Copy)]
enum SectionSlot {
    Prologue(usize),
    Epilogue(usize),
}

#[derive(Clone)]
enum ApplyTarget {
    Replace(std::ops::Range<usize>),
    Section(SectionSlot),
}

struct ResolvedTransclusion {
    order: usize,
    target: ApplyTarget,
    content: Option<String>,
    report: ComposeReport,
    /// Source file for file-based transclusions (used for source map).
    source_file: Option<PathBuf>,
}

// Re-export HeadingLevel for tests
#[cfg(test)]
pub use super::normalize::HeadingLevel;

impl Markdown {
    /// Transforms the document using default options.
    ///
    /// This is equivalent to `compose_with(ComposeOptions::new())`.
    ///
    /// ## Examples
    ///
    /// ```
    /// use darkmatter::markdown::Markdown;
    ///
    /// let content = "# Test\nContent";
    /// let md: Markdown = content.into();
    /// let (composed, report) = md.compose().unwrap();
    /// ```
    pub fn compose(&self) -> MarkdownResult<(Markdown, ComposeReport)> {
        self.compose_with(ComposeOptions::new())
    }

    /// Transforms the document with custom options.
    ///
    /// Returns a new `Markdown` document and a report of changes made.
    ///
    /// ## Examples
    ///
    /// ```
    /// use darkmatter::markdown::Markdown;
    /// use darkmatter::markdown::compose::{ComposeOperation, ComposeOptions};
    ///
    /// let content = "# Test\nContent";
    /// let md: Markdown = content.into();
    ///
    /// let options = ComposeOptions::new()
    ///     .disable(ComposeOperation::Normalization);
    ///
    /// let (composed, report) = md.compose_with(options).unwrap();
    /// ```
    #[instrument(skip_all, fields(source = ?options.source))]
    pub fn compose_with(
        &self,
        options: ComposeOptions,
    ) -> MarkdownResult<(Markdown, ComposeReport)> {
        let mut result = self.clone();
        let report = result.run_compose_pipeline(options)?;
        Ok((result, report))
    }

    /// Transforms the document in place, returning only the report.
    ///
    /// This is more efficient than `compose()` when you don't need
    /// to preserve the original document.
    ///
    /// ## Examples
    ///
    /// ```
    /// use darkmatter::markdown::Markdown;
    ///
    /// let content = "# Test\nContent";
    /// let mut md: Markdown = content.into();
    /// let report = md.compose_mut().unwrap();
    ///
    /// // md is now composed
    /// ```
    pub fn compose_mut(&mut self) -> MarkdownResult<ComposeReport> {
        self.run_compose_pipeline(ComposeOptions::new())
    }

    /// Internal pipeline runner.
    fn run_compose_pipeline(&mut self, options: ComposeOptions) -> MarkdownResult<ComposeReport> {
        // Resolve persistent cache root if configured
        let persistent_root = options.cache_root.as_ref().map(|root| {
            cache::FileStore::resolve_cache_root(Some(root), options.cache_namespace.as_deref())
        });

        let mut runtime = shell_expansion::types::PipelineRuntime::new(
            options.max_transclusion_depth,
            options.cache_access_mode,
            persistent_root,
        );
        let mut report = self.run_compose_pipeline_internal(options, &mut runtime)?;
        report.cache_stats = Some(runtime.cache.stats());
        Ok(report)
    }

    /// Internal recursive pipeline runner shared by root and child documents.
    ///
    /// Executes operations in three phases:
    /// 1. **Inline Pre** (serial): TextReplacement, PageBlocks, Interpolation, ShellExpansion
    /// 2. **Transclusion** (prepared serially, resolved concurrently): BlockTransclusion,
    ///    FrontmatterTransclusion, CodeTransclusion, TocLinking
    /// 3. **Inline Post** (serial): Cleanup, Normalization
    #[instrument(skip_all, fields(source = ?options.source))]
    pub(crate) fn run_compose_pipeline_internal(
        &mut self,
        options: ComposeOptions,
        runtime: &mut shell_expansion::types::PipelineRuntime,
    ) -> MarkdownResult<ComposeReport> {
        let source_id = match &options.source {
            ComposeSource::Unknown => None,
            ComposeSource::File(path) => Some(
                std::fs::canonicalize(path)
                    .unwrap_or_else(|_| path.clone())
                    .to_string_lossy()
                    .to_string(),
            ),
            ComposeSource::Url(url) => Some(url.to_string()),
        };

        if let Some(id) = source_id.clone() {
            let path = match &options.source {
                ComposeSource::File(p) => p.clone(),
                ComposeSource::Url(u) => std::path::PathBuf::from(u.to_string()),
                ComposeSource::Unknown => std::path::PathBuf::from("<unknown>"),
            };
            runtime.transclusion.enter(id, path, 1)?;
        }

        let result = (|| {
            let mut report = ComposeReport::new();
            let mut perf = perf::PerfCollector::new(options.perf_enabled);

            let pre_interpolation_snapshot = prepare_frontmatter_for_compose(
                self,
                &options,
                options.is_enabled(ComposeOperation::FrontmatterShellExpansion),
            );

            // Frontmatter Interpolation: resolve {{ }} in frontmatter values
            // before EffectiveState is built, since it mutates frontmatter
            // inputs that drive later stages.
            if options.is_enabled(ComposeOperation::FrontmatterInterpolation) {
                let fm_start = perf.is_enabled().then(std::time::Instant::now);
                let fm_report = frontmatter_interpolation::interpolate_frontmatter(
                    self.frontmatter_mut(),
                    options.context(),
                    options.fail_fast,
                )?;
                report.frontmatter_interpolations_applied = fm_report.replacements;
                report.warnings.extend(fm_report.warnings);
                if let Some(start) = fm_start {
                    perf.record(
                        perf::PerfMetricKind::FrontmatterInterpolation,
                        start.elapsed(),
                    );
                }
            }

            // Frontmatter Shell Expansion: execute $(cmd) in frontmatter values
            // before EffectiveState is built, since the expanded values must be
            // visible to all later stages.
            if options.is_enabled(ComposeOperation::FrontmatterShellExpansion) {
                let fse_start = perf.is_enabled().then(std::time::Instant::now);
                let fse_report = frontmatter_shell_expansion::execute_frontmatter_shell_expansion(
                    self.frontmatter_mut(),
                    &options,
                    runtime,
                    pre_interpolation_snapshot.as_ref(),
                )?;
                report.frontmatter_shell_expansions_applied = fse_report.replacements;
                report.shell_approvals_used += fse_report.approvals_used;
                report.warnings.extend(fse_report.warnings);
                if let Some(start) = fse_start {
                    perf.record(
                        perf::PerfMetricKind::FrontmatterShellExpansion,
                        start.elapsed(),
                    );
                }
            }

            // Build effective state for replacement/interpolation and condition checks.
            let esb_start = perf.is_enabled().then(std::time::Instant::now);
            let effective_state = EffectiveStateBuilder::new()
                .with_frontmatter(
                    self.frontmatter()
                        .as_map()
                        .iter()
                        .map(|(k, v)| (k.clone(), v.clone()))
                        .collect(),
                )
                .with_external_state(
                    options
                        .external_state
                        .clone()
                        .unwrap_or(Value::Object(Map::new())),
                )
                .with_merge_strategy(super::MergeStrategy::PreferDocument)
                .with_replace_parent_wins(options.replace_parent_wins)
                .with_context(options.context().clone())
                .with_allow_ctx_override(options.allow_ctx_override)
                .build()?;
            if let Some(start) = esb_start {
                perf.record(perf::PerfMetricKind::EffectiveStateBuild, start.elapsed());
            }

            // Convert ctx diagnostics to compose warnings
            let source_display = match &options.source {
                ComposeSource::File(p) => abbreviate_path(p),
                ComposeSource::Url(u) => u.to_string(),
                ComposeSource::Unknown => "unknown".to_string(),
            };
            for diag in effective_state.ctx_diagnostics() {
                let warning = match diag {
                    context::ContextMergeDiagnostic::UserCtxMerged { colliding_keys }
                        if colliding_keys.is_empty() =>
                    {
                        // No warning needed when merge succeeded without collisions
                        continue;
                    }
                    context::ContextMergeDiagnostic::UserCtxMerged { colliding_keys } => {
                        let keys_list = colliding_keys.join(", ");
                        ComposeWarning::new(
                            "context",
                            format!(
                                "the <blue>{source_display}</blue> document <i>defines</i> a <inverse>ctx</inverse> property and keys [<dim>{keys_list}</dim>] in the <inverse>ctx</inverse> dictionary conflict with those provided by Darkmatter's normal context dictionary!"
                            ),
                        )
                    }
                    context::ContextMergeDiagnostic::InvalidUserCtxReplaced => ComposeWarning::new(
                        "context",
                        "Document ctx was not an object; replaced with runtime context",
                    ),
                    context::ContextMergeDiagnostic::PartialRuntimeCapture { area, detail } => {
                        ComposeWarning::new(
                            "context",
                            format!("Partial runtime capture for {area}: {detail}"),
                        )
                    }
                };
                report.warnings.push(warning);
            }

            let mut transclusion_ran = false;
            for operation in ComposeOperation::default_order() {
                trace!(operation = ?operation, enabled = options.is_enabled(*operation), "compose: checking operation");
                if !options.is_enabled(*operation) {
                    continue;
                }

                info!(operation = ?operation, phase = ?operation.phase(), "compose: running operation");
                match operation.phase() {
                    ComposePhase::InlinePre => {
                        let op_start = perf.is_enabled().then(std::time::Instant::now);
                        self.run_inline_pre_operation(
                            *operation,
                            &effective_state,
                            &options,
                            runtime,
                            &mut report,
                        )?;
                        if let Some(start) = op_start {
                            let kind = match operation {
                                ComposeOperation::FrontmatterInterpolation => {
                                    perf::PerfMetricKind::FrontmatterInterpolation
                                }
                                ComposeOperation::FrontmatterShellExpansion => {
                                    perf::PerfMetricKind::FrontmatterShellExpansion
                                }
                                ComposeOperation::TextReplacement => {
                                    perf::PerfMetricKind::TextReplacement
                                }
                                ComposeOperation::PageBlocks => perf::PerfMetricKind::PageBlocks,
                                ComposeOperation::Interpolation => {
                                    perf::PerfMetricKind::Interpolation
                                }
                                ComposeOperation::ShellExpansion => {
                                    perf::PerfMetricKind::ShellExpansion
                                }
                                _ => unreachable!(),
                            };
                            perf.record(kind, start.elapsed());
                        }
                    }
                    ComposePhase::Transclusion => {
                        if transclusion_ran {
                            continue;
                        }

                        let enabled_transclusion_ops = ComposeOperation::default_order()
                            .iter()
                            .copied()
                            .filter(|op| {
                                op.phase() == ComposePhase::Transclusion && options.is_enabled(*op)
                            })
                            .collect::<Vec<_>>();

                        self.run_transclusion_phase(
                            &enabled_transclusion_ops,
                            &effective_state,
                            &options,
                            runtime,
                            &mut report,
                            &mut perf,
                        )?;
                        transclusion_ran = true;
                    }
                    ComposePhase::InlinePost => {
                        let op_start = perf.is_enabled().then(std::time::Instant::now);
                        self.run_inline_post_operation(*operation, &options, &mut report)?;
                        if let Some(start) = op_start {
                            let kind = match operation {
                                ComposeOperation::Cleanup => perf::PerfMetricKind::Cleanup,
                                ComposeOperation::Normalization => {
                                    perf::PerfMetricKind::Normalization
                                }
                                _ => unreachable!(),
                            };
                            perf.record(kind, start.elapsed());
                        }
                    }
                }
            }

            report.max_transclusion_depth = runtime.transclusion.deepest_seen;
            report.perf = perf.finish();
            Ok(report)
        })();

        if source_id.is_some() {
            runtime.transclusion.exit();
        }

        result
    }

    fn run_inline_pre_operation(
        &mut self,
        operation: ComposeOperation,
        state: &EffectiveState,
        options: &ComposeOptions,
        runtime: &mut shell_expansion::types::PipelineRuntime,
        report: &mut ComposeReport,
    ) -> MarkdownResult<()> {
        match operation {
            // FrontmatterInterpolation is handled before EffectiveState build,
            // not in the generic operation loop.
            ComposeOperation::FrontmatterInterpolation => Ok(()),
            // FrontmatterShellExpansion is handled before EffectiveState build,
            // not in the generic operation loop.
            ComposeOperation::FrontmatterShellExpansion => Ok(()),
            ComposeOperation::TextReplacement => {
                report.replacements_applied = self.run_replacement_stage(state, options);
                Ok(())
            }
            ComposeOperation::PageBlocks => self.run_page_blocks_stage(state, report),
            ComposeOperation::Interpolation => {
                report.interpolations_applied = self.run_interpolation_stage(state, options)?;
                Ok(())
            }
            ComposeOperation::ShellExpansion => {
                self.run_shell_expansion_stage(options, runtime, report)
            }
            _ => Ok(()),
        }
    }

    fn run_inline_post_operation(
        &mut self,
        operation: ComposeOperation,
        options: &ComposeOptions,
        report: &mut ComposeReport,
    ) -> MarkdownResult<()> {
        match operation {
            ComposeOperation::Cleanup => {
                let original_content = self.content.clone();
                self.content = match options.list_spacing {
                    cleanup::ListSpacingMode::Normal => {
                        cleanup::cleanup_content_with_indent(&self.content, options.indent_size)
                    }
                    cleanup::ListSpacingMode::Compact => {
                        cleanup::cleanup_content_with_indent_compact(
                            &self.content,
                            options.indent_size,
                        )
                    }
                    cleanup::ListSpacingMode::Loose => cleanup::cleanup_content_with_indent_loose(
                        &self.content,
                        options.indent_size,
                    ),
                };
                report.cleanup_changed = self.content != original_content;
                Ok(())
            }
            ComposeOperation::Normalization => match self.run_normalization_stage() {
                Ok(norm_report) => {
                    if norm_report.has_changes() {
                        report.normalization_report = Some(norm_report);
                    }
                    Ok(())
                }
                Err(NormalizationError::LevelOverflow { .. }) if !options.fail_fast => {
                    report.add_warning(ComposeWarning::new(
                        "normalization",
                        "Skipped normalization: would overflow H6",
                    ));
                    Ok(())
                }
                Err(e) => Err(MarkdownError::Transform(format!(
                    "Normalization failed: {}",
                    e
                ))),
            },
            _ => Ok(()),
        }
    }

    fn run_transclusion_phase(
        &mut self,
        operations: &[ComposeOperation],
        state: &EffectiveState,
        options: &ComposeOptions,
        runtime: &mut shell_expansion::types::PipelineRuntime,
        report: &mut ComposeReport,
        perf_collector: &mut perf::PerfCollector,
    ) -> MarkdownResult<()> {
        use rayon::prelude::*;

        if operations.is_empty() {
            return Ok(());
        }

        info!(operations = ?operations, "compose: starting transclusion phase");
        let parse_start = perf_collector.is_enabled().then(std::time::Instant::now);

        let parsed_directives = if operations.iter().any(|op| {
            matches!(
                op,
                ComposeOperation::BlockTransclusion | ComposeOperation::CodeTransclusion
            )
        }) {
            Some(transclusion::parse_directives(&self.content)?)
        } else {
            None
        };

        let frontmatter_refs = if operations.contains(&ComposeOperation::FrontmatterTransclusion) {
            Some(transclusion::parse_frontmatter_refs(
                self.frontmatter().as_map(),
            )?)
        } else {
            None
        };

        let toc_directives = if operations.contains(&ComposeOperation::TocLinking) {
            Some(toc_linking::parse_directives(&self.content)?)
        } else {
            None
        };

        if let Some(start) = parse_start {
            perf_collector.record(perf::PerfMetricKind::TransclusionParse, start.elapsed());
        }

        let prepare_start = perf_collector.is_enabled().then(std::time::Instant::now);

        let mut prepared = Vec::new();
        let mut next_order = 0usize;

        for operation in operations {
            match operation {
                ComposeOperation::BlockTransclusion => {
                    if let Some(directives) = parsed_directives.as_ref() {
                        self.prepare_block_transclusions(
                            directives,
                            transclusion::DirectiveKind::File,
                            state,
                            options,
                            report,
                            &mut prepared,
                            &mut next_order,
                        )?;
                    }
                }
                ComposeOperation::FrontmatterTransclusion => {
                    if let Some(refs) = frontmatter_refs.as_ref() {
                        self.prepare_frontmatter_transclusions(
                            refs,
                            state,
                            options,
                            report,
                            &mut prepared,
                            &mut next_order,
                        )?;
                    }
                }
                ComposeOperation::CodeTransclusion => {
                    if let Some(directives) = parsed_directives.as_ref() {
                        self.prepare_block_transclusions(
                            directives,
                            transclusion::DirectiveKind::Code,
                            state,
                            options,
                            report,
                            &mut prepared,
                            &mut next_order,
                        )?;
                    }
                }
                ComposeOperation::TocLinking => {
                    if let Some(directives) = toc_directives.as_ref() {
                        self.prepare_toc_transclusions(
                            directives,
                            report,
                            &mut prepared,
                            &mut next_order,
                        );
                    }
                }
                _ => {}
            }
        }

        if let Some(start) = prepare_start {
            perf_collector.record(perf::PerfMetricKind::TransclusionPrepare, start.elapsed());
        }

        if prepared.is_empty() {
            return Ok(());
        }

        let resolve_start = perf_collector.is_enabled().then(std::time::Instant::now);

        let runtime_mutex = std::sync::Mutex::new(runtime);
        let results = prepared
            .into_par_iter()
            .map(|item| self.resolve_prepared_transclusion(item, state, options, &runtime_mutex))
            .collect::<Vec<_>>();

        debug!(
            resolved = results.len(),
            "compose: transclusion resolution complete"
        );
        if let Some(start) = resolve_start {
            perf_collector.record(perf::PerfMetricKind::TransclusionResolve, start.elapsed());
        }

        let apply_start = perf_collector.is_enabled().then(std::time::Instant::now);

        let mut replacements = Vec::new();
        let prologue_count = frontmatter_refs
            .as_ref()
            .map_or(0, |refs| refs.prologue.len());
        let epilogue_count = frontmatter_refs
            .as_ref()
            .map_or(0, |refs| refs.epilogue.len());
        let mut prologue_sections = vec![None; prologue_count];
        let mut epilogue_sections = vec![None; epilogue_count];

        for result in results {
            let resolved = match result {
                Ok(resolved) => resolved,
                Err(error) => {
                    let is_structural = matches!(
                        error,
                        MarkdownError::Transclusion(
                            transclusion::TransclusionError::CycleDetected { .. }
                                | transclusion::TransclusionError::MaxDepthExceeded { .. }
                        )
                    );
                    if is_structural || options.fail_fast {
                        return Err(error);
                    }
                    report.add_warning(ComposeWarning::new("transclusion", error.to_string()));
                    continue;
                }
            };

            report.merge(resolved.report);

            match resolved.target {
                ApplyTarget::Replace(span) => {
                    replacements.push((
                        resolved.order,
                        span,
                        resolved.content.unwrap_or_default(),
                        resolved.source_file,
                    ));
                }
                ApplyTarget::Section(SectionSlot::Prologue(index)) => {
                    prologue_sections[index] = resolved.content;
                }
                ApplyTarget::Section(SectionSlot::Epilogue(index)) => {
                    epilogue_sections[index] = resolved.content;
                }
            }
        }

        if !replacements.is_empty() {
            replacements.sort_by(|left, right| {
                right
                    .1
                    .start
                    .cmp(&left.1.start)
                    .then_with(|| right.0.cmp(&left.0))
            });
            let mut next = self.content.clone();
            for (_, span, replacement, _) in &replacements {
                next.replace_range(span.clone(), replacement);
            }
            self.content = next;

            // Build source map: compute final byte positions for each file transclusion.
            // Sort forward by original span start and track cumulative offset.
            {
                let mut forward: Vec<_> = replacements
                    .iter()
                    .map(|(_, span, content, source)| (span.clone(), content.len(), source.clone()))
                    .collect();
                forward.sort_by_key(|(span, _, _)| span.start);

                let mut offset: isize = 0;
                for (span, content_len, source_file) in forward {
                    let final_start = (span.start as isize + offset) as usize;
                    let final_end = final_start + content_len;

                    if let Some(file) = source_file {
                        report.source_map.push(SourceRange {
                            byte_start: final_start,
                            byte_end: final_end,
                            source_file: file,
                            source_start_line: 1,
                        });
                    }

                    offset += content_len as isize - (span.end - span.start) as isize;
                }
            }
        }

        if prologue_count > 0 || epilogue_count > 0 {
            let mut sections = Vec::new();
            sections.extend(
                prologue_sections
                    .into_iter()
                    .flatten()
                    .filter(|part| !part.trim().is_empty()),
            );
            sections.push(self.content.clone());
            sections.extend(
                epilogue_sections
                    .into_iter()
                    .flatten()
                    .filter(|part| !part.trim().is_empty()),
            );
            self.content = sections.join("\n\n");
        }

        if let Some(start) = apply_start {
            perf_collector.record(perf::PerfMetricKind::TransclusionApply, start.elapsed());
        }

        Ok(())
    }

    /// Runs the text replacement stage.
    ///
    /// Applies text replacements from the `replace` map in effective state.
    /// See [`replacement::apply_replacements`] for algorithm details.
    fn run_replacement_stage(&mut self, state: &EffectiveState, options: &ComposeOptions) -> usize {
        let (new_content, count) = if let Some(one_off) = &options.one_off_replace {
            let merged_replace = state::merge_replace_maps(state.get_replace_map(), Some(one_off));
            let mut frontmatter = HashMap::new();
            frontmatter.insert("replace".to_string(), Value::Object(merged_replace));
            let scoped_state = EffectiveStateBuilder::new()
                .with_frontmatter(frontmatter)
                .with_context(options.context().clone())
                .build()
                .expect("replace-only state has no user ctx");
            replacement::apply_replacements(&self.content, &scoped_state)
        } else {
            replacement::apply_replacements(&self.content, state)
        };
        if count > 0 {
            self.content = new_content;
        }
        debug!(count, "compose: text replacements applied");
        count
    }

    /// Runs the interpolation stage.
    ///
    /// Finds `{{ expression }}` patterns in content and evaluates them
    /// against the effective state. By default, expressions inside code
    /// spans and fenced code blocks are skipped. When
    /// `interpolate_code_spans` is enabled (via options or frontmatter),
    /// all expressions are processed regardless of surrounding code markup.
    fn run_interpolation_stage(
        &mut self,
        state: &EffectiveState,
        options: &ComposeOptions,
    ) -> MarkdownResult<usize> {
        use interpolation::{Evaluator, ScanMode, interpolate_text};

        let scan_mode = if self.resolve_interpolate_code_spans(options) {
            ScanMode::Plain
        } else {
            ScanMode::MarkdownAware
        };

        let evaluator = Evaluator::new(state);
        let result = interpolate_text(
            &self.content,
            &evaluator,
            scan_mode,
            options.fail_fast,
            "interpolation",
        )?;

        if result.replacements > 0 {
            self.content = result.output;
        }
        debug!(
            count = result.replacements,
            "compose: interpolations applied"
        );
        Ok(result.replacements)
    }

    /// Runs the normalization stage.
    ///
    /// Uses `None` as target level, which means headings are not re-leveled
    /// but the document structure is validated.
    fn run_normalization_stage(
        &mut self,
    ) -> Result<normalize::NormalizationReport, NormalizationError> {
        debug!("compose: running normalization");
        let (new_content, report) = normalize::normalize(&self.content, None)?;
        self.content = new_content;
        Ok(report)
    }

    /// Runs Stage 1 shell expansion directives.
    fn run_shell_expansion_stage(
        &mut self,
        options: &ComposeOptions,
        runtime: &mut shell_expansion::types::PipelineRuntime,
        report: &mut ComposeReport,
    ) -> MarkdownResult<()> {
        let directives = shell_expansion::parse_directives(&self.content)?;
        debug!(
            directive_count = directives.len(),
            "compose: shell expansion directives found"
        );
        if directives.is_empty() {
            return Ok(());
        }

        let shell_opts = options.shell_options();
        let policy_paths = shell_expansion::resolve_policy_paths(&shell_opts, &options.source)?;
        runtime.shell.ensure_loaded(&policy_paths)?;

        let mut replacements = Vec::new();

        for directive in directives {
            let execution =
                execute_directive_detailed(&directive, options, &policy_paths, &mut runtime.shell)?;
            replacements.push((directive.span.clone(), execution.combined_output()));
            report.warnings.extend(execution.warnings);
            report.shell_expansions_applied += 1;
        }

        apply_replacements_in_reverse(&mut self.content, replacements);
        report.shell_approvals_used += runtime.shell.take_recent_approval_count();
        Ok(())
    }

    /// Runs page blocks (conditional content regions).
    fn run_page_blocks_stage(
        &mut self,
        state: &EffectiveState,
        report: &mut ComposeReport,
    ) -> MarkdownResult<()> {
        debug!("compose: running page blocks");
        let regions = page_blocks::parser::parse_page_blocks(&self.content)?;
        if regions.is_empty() {
            return Ok(());
        }

        // Warn for unknown options
        fn warn_unknown_options(region: &page_blocks::PageBlockRegion, report: &mut ComposeReport) {
            for unknown in &region.options.unknown_options {
                report.add_warning(
                    ComposeWarning::new(
                        "page_blocks",
                        format!("Unknown page block option: '{}'", unknown),
                    )
                    .at_line(region.start_line),
                );
            }
            for child in &region.children {
                warn_unknown_options(child, report);
            }
        }
        for region in &regions {
            warn_unknown_options(region, report);
        }

        self.content =
            page_blocks::engine::render_page_blocks(&self.content, &regions, state, report)?;
        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    fn prepare_block_transclusions(
        &self,
        directives: &[transclusion::BlockDirective],
        kind: transclusion::DirectiveKind,
        state: &EffectiveState,
        options: &ComposeOptions,
        report: &mut ComposeReport,
        prepared: &mut Vec<PreparedTransclusion>,
        next_order: &mut usize,
    ) -> MarkdownResult<()> {
        let ignore_invalid = self.resolve_ignore_invalid(options);
        let transclusion_opts = options.transclusion_options();

        for directive in directives.iter().filter(|directive| match kind {
            transclusion::DirectiveKind::Code => {
                directive.kind == transclusion::DirectiveKind::Code
            }
            _ => directive.kind != transclusion::DirectiveKind::Code,
        }) {
            for unknown in &directive.options.unknown_options {
                report.add_warning(
                    ComposeWarning::new(
                        "transclusion",
                        format!(
                            "Unknown option '{}' on ::{} directive; ignoring",
                            unknown,
                            directive.kind.as_str()
                        ),
                    )
                    .at_line(directive.line),
                );
            }

            for error in &directive.options.deferred_set_errors {
                match error {
                    transclusion::DeferredSetError::InvalidAssignment { raw, reason } => {
                        if options.allow_invalid_frontmatter_assignment {
                            report.add_warning(
                                ComposeWarning::new(
                                    "transclusion",
                                    format!(
                                        "Invalid frontmatter assignment on ::{} directive at line {}: {} (value: {})",
                                        directive.kind.as_str(),
                                        directive.line,
                                        reason,
                                        raw
                                    ),
                                )
                                .at_line(directive.line),
                            );
                        } else {
                            return Err(
                                transclusion::TransclusionError::InvalidFrontmatterAssignment {
                                    line: directive.line,
                                    raw: raw.clone(),
                                    reason: reason.clone(),
                                }
                                .into(),
                            );
                        }
                    }
                    transclusion::DeferredSetError::ReassignedProperty { name } => {
                        if options.allow_reassigned_frontmatter_property {
                            report.add_warning(
                                ComposeWarning::new(
                                    "transclusion",
                                    format!(
                                        "Duplicate set property '{}' on ::{} directive at line {}; rightmost assignment wins",
                                        name,
                                        directive.kind.as_str(),
                                        directive.line
                                    ),
                                )
                                .at_line(directive.line),
                            );
                        } else {
                            return Err(
                                transclusion::TransclusionError::InvalidReassignedFrontmatterProperty {
                                    line: directive.line,
                                    name: name.clone(),
                                }
                                .into(),
                            );
                        }
                    }
                }
            }

            if let Some(expr) = &directive.options.when_expr {
                let should_include = transclusion::evaluate_condition(expr, state, directive.line)?;
                if !should_include {
                    let mut fixed_report = ComposeReport::new();
                    fixed_report.transclusions_skipped = 1;
                    prepared.push(PreparedTransclusion::FixedReplace {
                        order: *next_order,
                        span: directive.span.clone(),
                        replacement: String::new(),
                        report: fixed_report,
                    });
                    *next_order += 1;
                    continue;
                }
            }

            let target = transclusion::normalize_reference_token(&directive.raw_target);
            let resolved = match transclusion::resolve_target(
                directive.kind,
                &target,
                &transclusion_opts,
                &options.source,
                directive.line,
            ) {
                Ok(resolved) => resolved,
                Err(err) if ignore_invalid => {
                    let mut fixed_report = ComposeReport::new();
                    fixed_report.transclusions_skipped = 1;
                    fixed_report.add_warning(
                        ComposeWarning::new("transclusion", err.to_string())
                            .at_line(directive.line),
                    );
                    prepared.push(PreparedTransclusion::FixedReplace {
                        order: *next_order,
                        span: directive.span.clone(),
                        replacement: String::new(),
                        report: fixed_report,
                    });
                    *next_order += 1;
                    continue;
                }
                Err(err) => return Err(err.into()),
            };

            match resolved {
                transclusion::ResolvedTarget::File { path, .. } => {
                    let item = if directive.kind == transclusion::DirectiveKind::Code {
                        PreparedTransclusion::Code {
                            order: *next_order,
                            span: directive.span.clone(),
                            path,
                            directive_options: directive.options.clone(),
                            line: directive.line,
                        }
                    } else {
                        PreparedTransclusion::Markdown {
                            order: *next_order,
                            target: ApplyTarget::Replace(directive.span.clone()),
                            path,
                            directive_options: directive.options.clone(),
                            insertion_context: Some((directive.span.start, directive.line)),
                        }
                    };
                    prepared.push(item);
                    *next_order += 1;
                }
                transclusion::ResolvedTarget::Url { url, .. } if ignore_invalid => {
                    let mut fixed_report = ComposeReport::new();
                    fixed_report.transclusions_skipped = 1;
                    fixed_report.add_warning(
                        ComposeWarning::new(
                            "transclusion",
                            format!(
                                "Skipping URL transclusion '{}': remote execution disabled",
                                url
                            ),
                        )
                        .at_line(directive.line),
                    );
                    prepared.push(PreparedTransclusion::FixedReplace {
                        order: *next_order,
                        span: directive.span.clone(),
                        replacement: String::new(),
                        report: fixed_report,
                    });
                    *next_order += 1;
                }
                transclusion::ResolvedTarget::Url { url, .. } => {
                    return Err(transclusion::TransclusionError::UrlExecutionDisabled {
                        url: url.to_string(),
                    }
                    .into());
                }
            }
        }

        Ok(())
    }

    fn prepare_frontmatter_transclusions(
        &self,
        refs: &transclusion::FrontmatterRefs,
        _state: &EffectiveState,
        options: &ComposeOptions,
        _report: &mut ComposeReport,
        prepared: &mut Vec<PreparedTransclusion>,
        next_order: &mut usize,
    ) -> MarkdownResult<()> {
        for (index, reference) in refs.prologue.iter().enumerate() {
            self.prepare_frontmatter_reference(
                reference,
                SectionSlot::Prologue(index),
                options,
                prepared,
                next_order,
            )?;
        }

        for (index, reference) in refs.epilogue.iter().enumerate() {
            self.prepare_frontmatter_reference(
                reference,
                SectionSlot::Epilogue(index),
                options,
                prepared,
                next_order,
            )?;
        }

        Ok(())
    }

    fn prepare_frontmatter_reference(
        &self,
        reference: &str,
        slot: SectionSlot,
        options: &ComposeOptions,
        prepared: &mut Vec<PreparedTransclusion>,
        next_order: &mut usize,
    ) -> MarkdownResult<()> {
        if !transclusion::is_url_like(reference) && !transclusion::is_file_like_reference(reference)
        {
            prepared.push(PreparedTransclusion::FixedSection {
                order: *next_order,
                slot,
                content: Some(reference.to_string()),
                report: ComposeReport::new(),
            });
            *next_order += 1;
            return Ok(());
        }

        let kind = if transclusion::is_url_like(reference) {
            transclusion::DirectiveKind::Url
        } else {
            transclusion::DirectiveKind::File
        };
        let ignore_invalid = self.resolve_ignore_invalid(options);
        let transclusion_opts = options.transclusion_options();

        let resolved = match transclusion::resolve_target(
            kind,
            reference,
            &transclusion_opts,
            &options.source,
            0,
        ) {
            Ok(resolved) => resolved,
            Err(err) if ignore_invalid => {
                let mut fixed_report = ComposeReport::new();
                fixed_report.transclusions_skipped = 1;
                fixed_report.add_warning(ComposeWarning::new("transclusion", err.to_string()));
                prepared.push(PreparedTransclusion::FixedSection {
                    order: *next_order,
                    slot,
                    content: None,
                    report: fixed_report,
                });
                *next_order += 1;
                return Ok(());
            }
            Err(err) => return Err(err.into()),
        };

        match resolved {
            transclusion::ResolvedTarget::File { path, .. } => {
                prepared.push(PreparedTransclusion::Markdown {
                    order: *next_order,
                    target: ApplyTarget::Section(slot),
                    path,
                    directive_options: transclusion::BlockOptions::default(),
                    insertion_context: None,
                });
                *next_order += 1;
            }
            transclusion::ResolvedTarget::Url { url, .. } if ignore_invalid => {
                let mut fixed_report = ComposeReport::new();
                fixed_report.transclusions_skipped = 1;
                fixed_report.add_warning(ComposeWarning::new(
                    "transclusion",
                    format!(
                        "Skipping URL transclusion '{}': remote execution disabled",
                        url
                    ),
                ));
                prepared.push(PreparedTransclusion::FixedSection {
                    order: *next_order,
                    slot,
                    content: None,
                    report: fixed_report,
                });
                *next_order += 1;
            }
            transclusion::ResolvedTarget::Url { url, .. } => {
                return Err(transclusion::TransclusionError::UrlExecutionDisabled {
                    url: url.to_string(),
                }
                .into());
            }
        }

        Ok(())
    }

    fn prepare_toc_transclusions(
        &self,
        directives: &[toc_linking::TocLinkingDirective],
        _report: &mut ComposeReport,
        prepared: &mut Vec<PreparedTransclusion>,
        next_order: &mut usize,
    ) {
        for directive in directives {
            prepared.push(PreparedTransclusion::Toc {
                order: *next_order,
                span: directive.span.clone(),
                directive: directive.clone(),
            });
            *next_order += 1;
        }
    }

    fn resolve_prepared_transclusion(
        &self,
        item: PreparedTransclusion,
        state: &EffectiveState,
        options: &ComposeOptions,
        runtime_mutex: &std::sync::Mutex<&mut shell_expansion::types::PipelineRuntime>,
    ) -> MarkdownResult<ResolvedTransclusion> {
        match item {
            PreparedTransclusion::FixedReplace {
                order,
                span,
                replacement,
                report,
            } => Ok(ResolvedTransclusion {
                order,
                target: ApplyTarget::Replace(span),
                content: Some(replacement),
                report,
                source_file: None,
            }),
            PreparedTransclusion::FixedSection {
                order,
                slot,
                content,
                report,
            } => Ok(ResolvedTransclusion {
                order,
                target: ApplyTarget::Section(slot),
                content,
                report,
                source_file: None,
            }),
            PreparedTransclusion::Markdown {
                order,
                target,
                path,
                directive_options,
                insertion_context,
            } => {
                let mut child_runtime = {
                    let runtime = runtime_mutex.lock().unwrap();
                    runtime.clone_for_child()
                };
                let mut child_report = ComposeReport::new();
                let content = self.render_markdown_transclusion(
                    &path,
                    insertion_context,
                    &directive_options,
                    state,
                    options,
                    &mut child_runtime,
                    &mut child_report,
                )?;
                child_report.transclusions_applied += 1;
                {
                    let mut runtime = runtime_mutex.lock().unwrap();
                    runtime.merge_child(&child_runtime);
                }
                Ok(ResolvedTransclusion {
                    order,
                    target,
                    content: Some(content),
                    report: child_report,
                    source_file: Some(path),
                })
            }
            PreparedTransclusion::Code {
                order,
                span,
                path,
                directive_options,
                line: _line,
            } => {
                let cache_handle = {
                    let runtime = runtime_mutex.lock().unwrap();
                    runtime.cache.clone()
                };
                let (content, dependency) = self.render_code_transclusion(
                    &path,
                    &directive_options,
                    state,
                    options,
                    &cache_handle,
                )?;
                if let Some(dependency) = dependency {
                    let mut runtime = runtime_mutex.lock().unwrap();
                    runtime.record_dependency(dependency);
                }
                let mut code_report = ComposeReport::new();
                code_report.transclusions_applied = 1;
                Ok(ResolvedTransclusion {
                    order,
                    target: ApplyTarget::Replace(span),
                    content: Some(content),
                    report: code_report,
                    source_file: None,
                })
            }
            PreparedTransclusion::Toc {
                order,
                span,
                directive,
            } => {
                let transclusion_opts = options.transclusion_options();
                let replacement = if let Some((display_target, path)) =
                    toc_linking::resolve_target_chain(
                        &directive,
                        &options.source,
                        &transclusion_opts,
                    )? {
                    let cache_handle = {
                        let runtime = runtime_mutex.lock().unwrap();
                        runtime.cache.clone()
                    };
                    let canonical_source = cache::compose_cache_key_for_path(&path);
                    let source_id = cache::hashing::source_id_hash(&canonical_source);
                    let source_bytes =
                        std::fs::read(&path).map_err(toc_linking::TocLinkingError::Io)?;
                    let source_content_hash = cache::hashing::raw_bytes_hash(&source_bytes);
                    let buckets = cache::TocLinkingOperation::split_params(&directive.options);
                    let entry_key =
                        cache::TocLinkingOperation::variant_cache_key(source_id, &buckets);
                    let cache_key =
                        cache::TocLinkingOperation::cache_key_string(&path, &directive.options);
                    let persistent_ctx = cache::OperationPersistentContext {
                        op_kind: "toc-linking",
                        entry_key,
                        source_id,
                        canonical_source,
                        source_content_hash,
                    };
                    let line = directive.line;
                    let options_clone = directive.options.clone();
                    let display_clone = display_target.clone();
                    let path_clone = path.clone();

                    let cached = cache_handle.get_or_compute_operation(
                        &cache_key,
                        Some(&persistent_ctx),
                        options.cache_freshness_mode,
                        || {
                            let headings = {
                                let runtime = runtime_mutex.lock().unwrap();
                                runtime
                                    .load_toc_headings(&path_clone)
                                    .map_err(toc_linking::TocLinkingError::Io)?
                            };
                            let content = toc_linking::render_resolved_directive(
                                &display_clone,
                                &headings,
                                &options_clone,
                                line,
                            )
                            .map_err(crate::markdown::types::MarkdownError::TocLinking)?;
                            Ok(cache::OperationResult { content })
                        },
                    )?;

                    if let Some(dependency) = cache_handle.operation_dependency_ref(&persistent_ctx)
                    {
                        let mut runtime = runtime_mutex.lock().unwrap();
                        runtime.record_dependency(dependency);
                    }

                    cached.content.clone()
                } else {
                    directive.options.empty_text.clone().unwrap_or_default()
                };

                let mut toc_report = ComposeReport::new();
                toc_report.toc_links_generated = 1;
                Ok(ResolvedTransclusion {
                    order,
                    target: ApplyTarget::Replace(span),
                    content: Some(replacement),
                    report: toc_report,
                    source_file: None,
                })
            }
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn render_markdown_transclusion(
        &self,
        path: &Path,
        insertion_context: Option<(usize, usize)>,
        directive_options: &transclusion::BlockOptions,
        state: &EffectiveState,
        options: &ComposeOptions,
        runtime: &mut shell_expansion::types::PipelineRuntime,
        report: &mut ComposeReport,
    ) -> MarkdownResult<String> {
        // ── Core compose (cacheable via single-flight) ─────────────
        let overlay_hash = cache::hashing::set_overlay_hash(
            directive_options.set_object.as_ref(),
            &directive_options.set_properties,
        );
        let options_hash = cache::hashing::combine_options_overlay_hash(
            cache::hashing::options_hash(options),
            overlay_hash,
        );
        let persistent_ctx = cache::PersistentContext {
            source_id: cache::hashing::source_id_hash(&cache::compose_cache_key_for_path(path)),
            state_hash: cache::hashing::effective_state_hash(state),
            context_hash: cache::hashing::context_hash(state.context()),
            options_hash,
        };
        let cache_key = format!(
            "compose:{:016x}:{:016x}:{:016x}:{:016x}:{:016x}",
            persistent_ctx.source_id,
            persistent_ctx.state_hash,
            persistent_ctx.context_hash,
            persistent_ctx.options_hash,
            overlay_hash,
        );
        let cache_handle = runtime.cache.clone();

        let inherited = self.build_child_external_state(state);
        let replace_parent_wins = matches!(
            directive_options.replace,
            transclusion::ReplaceOption::ParentWins
        );
        let one_off = match &directive_options.replace {
            transclusion::ReplaceOption::OneOff(one_off) => Some(one_off.clone()),
            _ => None,
        };
        let path_buf = path.to_path_buf();

        // Snapshot the per-directive set overlay. The overlay is applied to
        // the child's authored frontmatter before any of the child's pre-op
        // stages run; it does NOT propagate through `child_options` so
        // grandchildren do not inherit it.
        let set_object = directive_options.set_object.clone();
        let set_properties = directive_options.set_properties.clone();

        let cached = cache_handle.get_or_compute_compose(
            &cache_key,
            Some(&persistent_ctx),
            options.cache_freshness_mode,
            || {
                let mut child_options = options
                    .clone()
                    .with_replace_parent_wins(replace_parent_wins)
                    .with_one_off_replace(one_off.clone());
                child_options.external_state = Some(inherited.clone());
                child_options.source = ComposeSource::File(path_buf.clone());

                let mut compose_runtime = runtime.clone_for_child();
                let mut child = compose_runtime.load_markdown(path)?;

                // Apply the three-layer set overlay on the child's frontmatter
                // before any of its pre-op stages observe it. Keeping this
                // scoped inside the closure preserves the rule that
                // grandchildren referenced by the child's own `::file`
                // directives do NOT inherit this parent-applied overlay.
                if set_object.is_some() || !set_properties.is_empty() {
                    let base_indexmap = std::mem::take(child.frontmatter_mut().as_map_mut());
                    let base_map: serde_json::Map<String, Value> =
                        base_indexmap.into_iter().collect();
                    let overlaid =
                        state::apply_set_overrides(&base_map, set_object.as_ref(), &set_properties);
                    *child.frontmatter_mut().as_map_mut() = overlaid.into_iter().collect();
                }

                let child_report =
                    child.run_compose_pipeline_internal(child_options, &mut compose_runtime)?;
                runtime.merge_child(&compose_runtime);

                Ok(cache::ComposeResult {
                    content: child.content().to_string(),
                    report: child_report,
                    dependencies: compose_runtime.dependencies().to_vec(),
                })
            },
        )?;

        if let Some(dependency) = cache_handle.compose_dependency_ref(&persistent_ctx) {
            runtime.record_dependency(dependency);
        }

        report.merge(cached.report.clone());

        // ── Post-cache transforms (parent-specific, cheap) ─────────
        let mut content = cached.content.clone();

        // Apply exclude patterns to remove heading sections from the child.
        if !directive_options.exclude.is_empty() {
            let mut child_md = Markdown::new(content);
            child_md.remove_sections(&directive_options.exclude);
            content = child_md.into_parts().1;
        }

        if let Some((offset, line)) = insertion_context
            && let Some(parent_level) =
                transclusion::find_preceding_heading_level(&self.content, offset)
        {
            let target_level =
                super::normalize::HeadingLevel::new((parent_level.as_u8() + 1).min(6))
                    .unwrap_or(super::normalize::HeadingLevel::H6);
            let (releveled, warnings) = transclusion::relevel_with_overflow(&content, target_level);
            content = releveled;
            for warning in warnings {
                report.add_warning(warning.at_line(line));
            }
        }

        // For block directives (::file), ensure the final output ends with a
        // blank line so subsequent parent content is not absorbed into the last
        // block element of the child (e.g., a list item or blockquote).
        // This runs AFTER apply_wrappers because wrappers like wrap_quotation
        // use `.lines().join("\n")` which strips trailing newlines.
        // Frontmatter prologue/epilogue transclusion (insertion_context=None)
        // doesn't need this because sections are joined with "\n\n".
        let mut result = self.apply_wrappers(content, directive_options);
        if insertion_context.is_some() && !result.ends_with("\n\n") {
            if !result.ends_with('\n') {
                result.push('\n');
            }
            result.push('\n');
        }

        Ok(result)
    }

    fn render_code_transclusion(
        &self,
        path: &Path,
        directive_options: &transclusion::BlockOptions,
        state: &EffectiveState,
        options: &ComposeOptions,
        cache_handle: &cache::RunLocalCache,
    ) -> MarkdownResult<(String, Option<cache::types::DependencyRef>)> {
        // Compute variant params (needed for both cache key and core)
        let base_map = state.get_replace_map().cloned().unwrap_or_default();
        let effective_map = match &directive_options.replace {
            transclusion::ReplaceOption::InheritDefault => base_map,
            transclusion::ReplaceOption::ParentWins => base_map,
            transclusion::ReplaceOption::OneOff(one_off) => {
                state::merge_replace_maps(Some(&base_map), Some(one_off))
            }
        };
        let language = transclusion::infer_language(path, &options.code_fallback_language);
        let canonical_source = cache::compose_cache_key_for_path(path);
        let source_id = cache::hashing::source_id_hash(&canonical_source);
        let source_bytes = std::fs::read(path)?;
        let source_content_hash = cache::hashing::raw_bytes_hash(&source_bytes);

        let op = cache::CodeOperation;
        let mut buckets = op.split_params(directive_options);
        buckets
            .variant
            .push(("language".to_string(), language.clone()));
        let entry_key = op.variant_cache_key(source_id, &buckets);
        let cache_key = format!("code:{}:{:016x}", canonical_source, entry_key);
        let persistent_ctx = cache::OperationPersistentContext {
            op_kind: "code",
            entry_key,
            source_id,
            canonical_source,
            source_content_hash,
        };

        // Core computation (cacheable via single-flight)
        let context = options.context().clone();
        let path_buf = path.to_path_buf();
        let raw_text = match String::from_utf8(source_bytes) {
            Ok(text) => text,
            Err(_) => {
                return Err(transclusion::TransclusionError::NonTextCodeSource {
                    path: path_buf.clone(),
                }
                .into());
            }
        };
        let cached = cache_handle.get_or_compute_operation(
            &cache_key,
            Some(&persistent_ctx),
            options.cache_freshness_mode,
            || {
                let raw = raw_text.clone();

                let replaced = if effective_map.is_empty() {
                    raw
                } else {
                    let mut frontmatter = HashMap::new();
                    frontmatter.insert("replace".to_string(), Value::Object(effective_map.clone()));
                    let temp_state = EffectiveStateBuilder::new()
                        .with_frontmatter(frontmatter)
                        .with_context(context.clone())
                        .build()
                        .expect("replace-only state has no user ctx");
                    let (replaced, _) = replacement::apply_replacements(&raw, &temp_state);
                    replaced
                };

                let fenced = transclusion::wrap_in_code_block(&replaced, &language);
                let spaced = transclusion::ensure_vertical_spacing(&fenced);
                Ok(cache::OperationResult { content: spaced })
            },
        )?;

        // Post: apply wrappers (cheap, directive-specific)
        Ok((
            self.apply_wrappers(cached.content.clone(), directive_options),
            cache_handle.operation_dependency_ref(&persistent_ctx),
        ))
    }

    fn apply_wrappers(
        &self,
        mut content: String,
        directive_options: &transclusion::BlockOptions,
    ) -> String {
        if let Some(quotation) = &directive_options.quotation {
            let attribution = if quotation.is_empty() {
                None
            } else {
                Some(quotation.as_str())
            };
            content = transclusion::wrap_quotation(&content, attribution);
        }

        if let Some(summary) = &directive_options.disclosure {
            content = transclusion::wrap_disclosure(&content, summary);
        }

        content
    }

    fn build_child_external_state(&self, state: &EffectiveState) -> Value {
        let mut inherited: Map<String, Value> = state.data().clone().into_iter().collect();

        // Prologue/epilogue are scoped to the defining document — never propagate.
        // ctx is captured fresh per-document by EffectiveStateBuilder, so the
        // parent's merged runtime context must not leak into children (it would
        // appear as a document-defined ctx and trigger false collision warnings).
        inherited.remove("prologue");
        inherited.remove("epilogue");
        inherited.remove("ctx");

        Value::Object(inherited)
    }

    fn resolve_ignore_invalid(&self, options: &ComposeOptions) -> bool {
        if let Some(value) = options.ignore_invalid_references {
            return value;
        }

        if let Ok(Some(value)) = self.fm_get::<bool>("ignore_invalid") {
            return value;
        }

        options
            .context()
            .env()
            .get("IGNORE_INVALID")
            .and_then(|raw| parse_bool(raw))
            .unwrap_or(false)
    }

    /// Resolves whether interpolation should process code spans.
    ///
    /// Checks (in priority order):
    /// 1. `ComposeOptions::interpolate_code_spans`
    /// 2. Frontmatter `interpolate_code_spans` key
    fn resolve_interpolate_code_spans(&self, options: &ComposeOptions) -> bool {
        if options.interpolate_code_spans {
            return true;
        }

        if let Ok(Some(value)) = self.fm_get::<bool>("interpolate_code_spans") {
            return value;
        }

        false
    }
}

fn parse_bool(raw: &str) -> Option<bool> {
    match raw.trim().to_ascii_lowercase().as_str() {
        "1" | "true" | "yes" | "on" => Some(true),
        "0" | "false" | "no" | "off" => Some(false),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    #[allow(unused_imports)]
    use super::HeadingLevel;
    use super::*;

    #[test]
    fn test_compose_returns_unchanged_document() {
        let content = "# Hello\n\nWorld";
        let md: Markdown = content.into();

        let (composed, _report) = md.compose().unwrap();

        // Content should still match (cleanup may add spacing)
        assert!(composed.content().contains("Hello"));
        assert!(composed.content().contains("World"));
    }

    #[test]
    fn test_compose_mut_modifies_in_place() {
        let content = "# Hello\n\nWorld";
        let mut md: Markdown = content.into();

        let _report = md.compose_mut().unwrap();

        assert!(md.content().contains("Hello"));
        assert!(md.content().contains("World"));
    }

    #[test]
    fn test_compose_with_custom_options() {
        let content = "# Hello\n\nWorld";
        let md: Markdown = content.into();

        let options = ComposeOptions::new()
            .disable(ComposeOperation::Cleanup)
            .disable(ComposeOperation::Normalization);

        let (composed, report) = md.compose_with(options).unwrap();

        // With cleanup disabled, content should be unchanged
        assert_eq!(composed.content(), md.content());
        assert!(!report.cleanup_changed);
    }

    #[test]
    fn test_compose_cleanup_stage() {
        // Content without proper spacing
        let content = "# Header\nParagraph";
        let md: Markdown = content.into();

        let options = ComposeOptions::new().only(&[ComposeOperation::Cleanup]);

        let (composed, report) = md.compose_with(options).unwrap();

        // Cleanup should add blank line between header and paragraph
        assert!(composed.content().contains("\n\n"));
        assert!(report.cleanup_changed);
    }

    #[test]
    fn test_compose_normalization_stage_no_change() {
        let content = "# Hello\n\n## World";
        let md: Markdown = content.into();

        let options = ComposeOptions::new().only(&[ComposeOperation::Normalization]);

        let (_, report) = md.compose_with(options).unwrap();

        // Well-formed document, no normalization needed
        assert!(report.normalization_report.is_none());
    }

    #[test]
    fn test_compose_preserves_frontmatter() {
        let content = "---\ntitle: Test\n---\n# Hello";
        let md: Markdown = content.into();

        let (composed, _) = md.compose().unwrap();

        let title: Option<String> = composed.fm_get("title").unwrap();
        assert_eq!(title, Some("Test".to_string()));
    }

    #[test]
    fn test_compose_report_summary() {
        let content = "# Header\nParagraph";
        let md: Markdown = content.into();

        let (_, report) = md.compose().unwrap();

        // Should have a meaningful summary
        let summary = report.summary();
        assert!(!summary.is_empty());
    }

    #[test]
    fn test_compose_report_has_changes() {
        let content = "# Header\nParagraph";
        let md: Markdown = content.into();

        let (_, report) = md.compose().unwrap();

        // Cleanup should have made changes
        assert!(report.has_changes());
        assert!(report.cleanup_changed);
    }

    #[test]
    fn test_compose_stages_all_disabled() {
        let content = "# Header\nParagraph";
        let md: Markdown = content.into();

        let options = ComposeOptions::new().only(&[]);

        let (composed, report) = md.compose_with(options).unwrap();

        // No changes should be made
        assert_eq!(composed.content(), md.content());
        assert!(!report.has_changes());
    }

    #[test]
    fn test_compose_stages_run_in_order() {
        let content = "---\nshow: false\n---\nBefore\n\n::block when=\"show\"\n\n::shell echo hidden\n\n::end-block\n\n::code ./example.rs\nAfter";
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join("root.md");
        let code = dir.path().join("example.rs");
        std::fs::write(&root, content).unwrap();
        std::fs::write(&code, "fn main() {}\n").unwrap();

        let options = ComposeOptions::new()
            .with_source_file(&root)
            .with_shell_policy_root(dir.path())
            .only(&[
                ComposeOperation::PageBlocks,
                ComposeOperation::ShellExpansion,
                ComposeOperation::CodeTransclusion,
                ComposeOperation::Cleanup,
            ]);

        let (composed, report) = Markdown::try_from(root.as_path())
            .unwrap()
            .compose_with(options)
            .unwrap();

        assert!(!composed.content().contains("hidden"));
        assert!(composed.content().contains("```rs"));
        assert!(composed.content().contains("\n\n```rs"));
        assert_eq!(report.page_blocks_skipped, 1);
        assert_eq!(report.shell_expansions_applied, 0);
        assert_eq!(report.transclusions_applied, 1);
        assert!(report.cleanup_changed);
    }

    #[test]
    fn test_compose_with_external_state() {
        let content = "# Hello";
        let md: Markdown = content.into();

        let options =
            ComposeOptions::new().with_external_state(serde_json::json!({"key": "value"}));

        // Should not fail
        let result = md.compose_with(options);
        assert!(result.is_ok());
    }

    #[test]
    fn test_compose_options_context_captured() {
        let options = ComposeOptions::new();
        let ctx = options.context();

        // Context should have been captured
        assert!(!ctx.today().is_empty());
        assert!(!ctx.year().is_empty());
    }

    #[test]
    fn test_compose_fail_fast_false_continues_on_warning() {
        // Document that would cause normalization warning
        // (but for now normalization doesn't fail with None target)
        let content = "# Hello";
        let md: Markdown = content.into();

        let options = ComposeOptions::new().with_fail_fast(false);

        let result = md.compose_with(options);
        assert!(result.is_ok());
    }

    #[test]
    fn test_effective_state_available_to_stages() {
        let content = "---\nkey: value\n---\n# Hello";
        let md: Markdown = content.into();

        // External state should merge with frontmatter
        let options =
            ComposeOptions::new().with_external_state(serde_json::json!({"external": "data"}));

        let result = md.compose_with(options);
        assert!(result.is_ok());
    }

    // ============================================
    // Replacement stage integration tests
    // ============================================

    #[test]
    fn test_replacement_stage_with_frontmatter() {
        let content = "---\nreplace:\n  foo: bar\n---\n# Hello foo\n\nContent with foo here.";
        let md: Markdown = content.into();

        let options = ComposeOptions::new().only(&[ComposeOperation::TextReplacement]);

        let (composed, report) = md.compose_with(options).unwrap();

        assert!(composed.content().contains("Hello bar"));
        assert!(composed.content().contains("Content with bar here."));
        assert_eq!(report.replacements_applied, 2);
    }

    #[test]
    fn test_replacement_stage_overlap_resolution() {
        // Longest key wins: "foobar" before "foo"
        let content = "---\nreplace:\n  foo: short\n  foobar: long\n---\nfoobar and foo";
        let md: Markdown = content.into();

        let options = ComposeOptions::new().only(&[ComposeOperation::TextReplacement]);

        let (composed, report) = md.compose_with(options).unwrap();

        assert_eq!(composed.content(), "long and short");
        assert_eq!(report.replacements_applied, 2);
    }

    #[test]
    fn test_replacement_stage_non_recursive() {
        // Replacement output should NOT be re-scanned
        let content = "---\nreplace:\n  foo: foobar\n  foobar: baz\n---\nfoo";
        let md: Markdown = content.into();

        let options = ComposeOptions::new().only(&[ComposeOperation::TextReplacement]);

        let (composed, report) = md.compose_with(options).unwrap();

        // "foo" -> "foobar" but NOT -> "baz"
        assert_eq!(composed.content(), "foobar");
        assert_eq!(report.replacements_applied, 1);
    }

    #[test]
    fn test_replacement_stage_null_value() {
        let content = "---\nreplace:\n  remove_me: null\n---\nHello remove_me world";
        let md: Markdown = content.into();

        let options = ComposeOptions::new().only(&[ComposeOperation::TextReplacement]);

        let (composed, report) = md.compose_with(options).unwrap();

        assert_eq!(composed.content(), "Hello  world");
        assert_eq!(report.replacements_applied, 1);
    }

    #[test]
    fn test_replacement_stage_number_value() {
        let content = "---\nreplace:\n  VERSION: 42\n---\nVersion: VERSION";
        let md: Markdown = content.into();

        let options = ComposeOptions::new().only(&[ComposeOperation::TextReplacement]);

        let (composed, report) = md.compose_with(options).unwrap();

        assert_eq!(composed.content(), "Version: 42");
        assert_eq!(report.replacements_applied, 1);
    }

    #[test]
    fn test_replacement_stage_no_replace_in_frontmatter() {
        let content = "---\ntitle: Test\n---\n# Hello foo";
        let md: Markdown = content.into();

        let options = ComposeOptions::new().only(&[ComposeOperation::TextReplacement]);

        let (composed, report) = md.compose_with(options).unwrap();

        // No changes when no replace map
        assert_eq!(composed.content(), md.content());
        assert_eq!(report.replacements_applied, 0);
    }

    #[test]
    fn test_replacement_stage_with_external_state() {
        // External state can provide replace map
        let content = "# Hello foo";
        let md: Markdown = content.into();

        let options = ComposeOptions::new()
            .only(&[ComposeOperation::TextReplacement])
            .with_external_state(serde_json::json!({
                "replace": {"foo": "bar"}
            }));

        let (composed, report) = md.compose_with(options).unwrap();

        assert_eq!(composed.content(), "# Hello bar");
        assert_eq!(report.replacements_applied, 1);
    }

    #[test]
    fn test_replacement_stage_frontmatter_overrides_external_with_deep_merge() {
        // External state replace map is merged as defaults; frontmatter wins on conflicts.
        let content = "---\nreplace:\n  foo: from_fm\n  baz: qux\n---\nfoo baz";
        let md: Markdown = content.into();

        let options = ComposeOptions::new()
            .only(&[ComposeOperation::TextReplacement])
            .with_external_state(serde_json::json!({
                "replace": {"foo": "from_external"}
            }));

        let (composed, report) = md.compose_with(options).unwrap();

        assert_eq!(composed.content(), "from_fm qux");
        assert_eq!(report.replacements_applied, 2);
    }

    #[test]
    fn test_replacement_stage_report_summary() {
        let content = "---\nreplace:\n  a: b\n---\na a a";
        let md: Markdown = content.into();

        let options = ComposeOptions::new().only(&[ComposeOperation::TextReplacement]);

        let (_, report) = md.compose_with(options).unwrap();

        assert_eq!(report.replacements_applied, 3);
        let summary = report.summary();
        assert!(summary.contains("3 replacement(s)"));
    }

    #[test]
    fn test_replacement_then_cleanup() {
        // Test that replacement runs before cleanup
        let content = "---\nreplace:\n  foo: bar\n---\n# Header\nfoo here";
        let md: Markdown = content.into();

        // Enable both replacement and cleanup
        let options = ComposeOptions::new()
            .only(&[ComposeOperation::TextReplacement, ComposeOperation::Cleanup]);

        let (composed, report) = md.compose_with(options).unwrap();

        // Replacement happened
        assert!(composed.content().contains("bar here"));
        assert_eq!(report.replacements_applied, 1);

        // Cleanup added blank line
        assert!(composed.content().contains("\n\n"));
        assert!(report.cleanup_changed);
    }

    // ============================================
    // Interpolation stage integration tests
    // ============================================

    #[test]
    fn test_interpolation_simple_variable() {
        let content = "---\nname: Alice\n---\n# Hello {{ name }}!";
        let md: Markdown = content.into();

        let options = ComposeOptions::new().only(&[ComposeOperation::Interpolation]);

        let (composed, report) = md.compose_with(options).unwrap();

        assert_eq!(composed.content(), "# Hello Alice!");
        assert_eq!(report.interpolations_applied, 1);
    }

    #[test]
    fn test_interpolation_nested_variable() {
        let content = "---\nuser:\n  name: Bob\n---\nWelcome {{ user.name }}";
        let md: Markdown = content.into();

        let options = ComposeOptions::new().only(&[ComposeOperation::Interpolation]);

        let (composed, report) = md.compose_with(options).unwrap();

        assert_eq!(composed.content(), "Welcome Bob");
        assert_eq!(report.interpolations_applied, 1);
    }

    #[test]
    fn test_interpolation_missing_variable() {
        let content = "---\ntitle: Test\n---\nHello {{ missing }}!";
        let md: Markdown = content.into();

        let options = ComposeOptions::new().only(&[ComposeOperation::Interpolation]);

        let (composed, report) = md.compose_with(options).unwrap();

        // Missing variables become empty string
        assert_eq!(composed.content(), "Hello !");
        assert_eq!(report.interpolations_applied, 1);
    }

    #[test]
    fn test_interpolation_fallback_uses_default() {
        let content = "---\ntitle: Test\n---\nColor: {{ color | \"unknown\" }}";
        let md: Markdown = content.into();

        let options = ComposeOptions::new().only(&[ComposeOperation::Interpolation]);

        let (composed, report) = md.compose_with(options).unwrap();

        assert_eq!(composed.content(), "Color: unknown");
        assert_eq!(report.interpolations_applied, 1);
    }

    #[test]
    fn test_interpolation_fallback_uses_primary() {
        let content = "---\ncolor: blue\n---\nColor: {{ color | \"unknown\" }}";
        let md: Markdown = content.into();

        let options = ComposeOptions::new().only(&[ComposeOperation::Interpolation]);

        let (composed, report) = md.compose_with(options).unwrap();

        assert_eq!(composed.content(), "Color: blue");
        assert_eq!(report.interpolations_applied, 1);
    }

    #[test]
    fn test_interpolation_ternary_true() {
        let content = "---\nactive: true\n---\nStatus: {{ active ? \"on\" : \"off\" }}";
        let md: Markdown = content.into();

        let options = ComposeOptions::new().only(&[ComposeOperation::Interpolation]);

        let (composed, report) = md.compose_with(options).unwrap();

        assert_eq!(composed.content(), "Status: on");
        assert_eq!(report.interpolations_applied, 1);
    }

    #[test]
    fn test_interpolation_ternary_false() {
        let content = "---\nactive: false\n---\nStatus: {{ active ? \"on\" : \"off\" }}";
        let md: Markdown = content.into();

        let options = ComposeOptions::new().only(&[ComposeOperation::Interpolation]);

        let (composed, report) = md.compose_with(options).unwrap();

        assert_eq!(composed.content(), "Status: off");
        assert_eq!(report.interpolations_applied, 1);
    }

    #[test]
    fn test_interpolation_comparison_equal() {
        let content = "---\ncount: 5\n---\n{{ count == 5 ? \"five\" : \"other\" }}";
        let md: Markdown = content.into();

        let options = ComposeOptions::new().only(&[ComposeOperation::Interpolation]);

        let (composed, report) = md.compose_with(options).unwrap();

        assert_eq!(composed.content(), "five");
        assert_eq!(report.interpolations_applied, 1);
    }

    #[test]
    fn test_interpolation_comparison_greater_than() {
        let content = "---\ncount: 10\n---\n{{ count > 5 ? \"many\" : \"few\" }}";
        let md: Markdown = content.into();

        let options = ComposeOptions::new().only(&[ComposeOperation::Interpolation]);

        let (composed, report) = md.compose_with(options).unwrap();

        assert_eq!(composed.content(), "many");
        assert_eq!(report.interpolations_applied, 1);
    }

    #[test]
    fn test_interpolation_multiple_expressions() {
        let content = "---\nfirst: Alice\nlast: Smith\n---\n{{ first }} {{ last }}";
        let md: Markdown = content.into();

        let options = ComposeOptions::new().only(&[ComposeOperation::Interpolation]);

        let (composed, report) = md.compose_with(options).unwrap();

        assert_eq!(composed.content(), "Alice Smith");
        assert_eq!(report.interpolations_applied, 2);
    }

    #[test]
    fn test_interpolation_skips_code_span() {
        let content = "---\nname: Alice\n---\nHello {{ name }}! Code: `{{ name }}`";
        let md: Markdown = content.into();

        let options = ComposeOptions::new().only(&[ComposeOperation::Interpolation]);

        let (composed, report) = md.compose_with(options).unwrap();

        // Only the first expression is expanded, code span preserved
        assert_eq!(composed.content(), "Hello Alice! Code: `{{ name }}`");
        assert_eq!(report.interpolations_applied, 1);
    }

    #[test]
    fn test_interpolation_code_spans_via_option() {
        let content = "---\nname: Alice\n---\nHello {{ name }}! Code: `{{ name }}`";
        let md: Markdown = content.into();

        let options = ComposeOptions::new()
            .only(&[ComposeOperation::Interpolation])
            .with_interpolate_code_spans(true);

        let (composed, report) = md.compose_with(options).unwrap();

        // Both expressions expanded when interpolate_code_spans is enabled
        assert_eq!(composed.content(), "Hello Alice! Code: `Alice`");
        assert_eq!(report.interpolations_applied, 2);
    }

    #[test]
    fn test_interpolation_code_spans_via_frontmatter() {
        let content = "---\nname: Alice\ninterpolate_code_spans: true\n---\nHello {{ name }}! Code: `{{ name }}`";
        let md: Markdown = content.into();

        let options = ComposeOptions::new().only(&[ComposeOperation::Interpolation]);

        let (composed, report) = md.compose_with(options).unwrap();

        // Both expressions expanded when frontmatter flag is set
        assert_eq!(composed.content(), "Hello Alice! Code: `Alice`");
        assert_eq!(report.interpolations_applied, 2);
    }

    #[test]
    fn test_interpolation_skips_fenced_code() {
        let content = "---\nname: Alice\n---\nHello {{ name }}!\n\n```\n{{ name }}\n```";
        let md: Markdown = content.into();

        let options = ComposeOptions::new().only(&[ComposeOperation::Interpolation]);

        let (composed, report) = md.compose_with(options).unwrap();

        // Only the first expression is expanded, code block preserved
        assert!(composed.content().contains("Hello Alice!"));
        assert!(composed.content().contains("```\n{{ name }}\n```"));
        assert_eq!(report.interpolations_applied, 1);
    }

    #[test]
    fn test_interpolation_no_expressions() {
        let content = "---\nname: Alice\n---\n# Just plain text";
        let md: Markdown = content.into();

        let options = ComposeOptions::new().only(&[ComposeOperation::Interpolation]);

        let (composed, report) = md.compose_with(options).unwrap();

        assert_eq!(composed.content(), md.content());
        assert_eq!(report.interpolations_applied, 0);
    }

    #[test]
    fn test_interpolation_with_external_state() {
        let content = "# Hello {{ name }}!";
        let md: Markdown = content.into();

        let options = ComposeOptions::new()
            .only(&[ComposeOperation::Interpolation])
            .with_external_state(serde_json::json!({"name": "External"}));

        let (composed, report) = md.compose_with(options).unwrap();

        assert_eq!(composed.content(), "# Hello External!");
        assert_eq!(report.interpolations_applied, 1);
    }

    #[test]
    fn test_interpolation_frontmatter_overrides_external() {
        let content = "---\nname: Frontmatter\n---\n# Hello {{ name }}!";
        let md: Markdown = content.into();

        let options = ComposeOptions::new()
            .only(&[ComposeOperation::Interpolation])
            .with_external_state(serde_json::json!({"name": "External"}));

        let (composed, report) = md.compose_with(options).unwrap();

        // Frontmatter wins on conflict
        assert_eq!(composed.content(), "# Hello Frontmatter!");
        assert_eq!(report.interpolations_applied, 1);
    }

    #[test]
    fn test_interpolation_chained_fallback() {
        let content = "---\nbackup: second\n---\nValue: {{ missing | backup | \"default\" }}";
        let md: Markdown = content.into();

        let options = ComposeOptions::new().only(&[ComposeOperation::Interpolation]);

        let (composed, report) = md.compose_with(options).unwrap();

        assert_eq!(composed.content(), "Value: second");
        assert_eq!(report.interpolations_applied, 1);
    }

    #[test]
    fn test_interpolation_parse_error_preserves_original() {
        // Malformed expression should be left as-is (not fail_fast)
        let content = "---\nname: Alice\n---\nHello {{ @invalid }}!";
        let md: Markdown = content.into();

        let options = ComposeOptions::new()
            .only(&[ComposeOperation::Interpolation])
            .with_fail_fast(false);

        let (composed, report) = md.compose_with(options).unwrap();

        // Invalid expression left unchanged
        assert_eq!(composed.content(), "Hello {{ @invalid }}!");
        assert_eq!(report.interpolations_applied, 0);
    }

    #[test]
    fn test_interpolation_parse_error_fail_fast_returns_error() {
        let content = "---\nname: Alice\n---\nHello {{ @invalid }}!";
        let md: Markdown = content.into();

        let options = ComposeOptions::new()
            .only(&[ComposeOperation::Interpolation])
            .with_fail_fast(true);

        let err = md.compose_with(options).unwrap_err();
        assert!(matches!(err, MarkdownError::Transform(_)));
    }

    #[test]
    fn test_full_compose_with_interpolation() {
        // Integration test: frontmatter + interpolation + cleanup
        let content = "---\nname: Alice\ncount: 3\n---\n# Welcome {{ name }}\nYou have {{ count > 0 ? \"items\" : \"nothing\" }}";
        let md: Markdown = content.into();

        let (composed, report) = md.compose().unwrap();

        assert!(composed.content().contains("Welcome Alice"));
        assert!(composed.content().contains("You have items"));
        assert_eq!(report.interpolations_applied, 2);
        assert!(report.cleanup_changed); // Cleanup adds blank line
    }

    #[test]
    fn test_interpolation_report_summary() {
        let content = "---\na: 1\nb: 2\n---\n{{ a }} {{ b }}";
        let md: Markdown = content.into();

        let options = ComposeOptions::new().only(&[ComposeOperation::Interpolation]);

        let (_, report) = md.compose_with(options).unwrap();

        assert_eq!(report.interpolations_applied, 2);
        let summary = report.summary();
        assert!(summary.contains("2 interpolation(s)"));
    }

    // ============================================
    // E2E Integration tests
    // ============================================

    #[test]
    fn test_e2e_all_stages_with_external_state() {
        // Full pipeline: replacement -> interpolation -> cleanup -> normalization
        let content = r#"---
replace:
  PLACEHOLDER: actual
name: Alice
count: 5
---
# Welcome {{ name }}
PLACEHOLDER content here.
{{ count > 3 ? "many items" : "few items" }}"#;

        let md: Markdown = content.into();

        let options = ComposeOptions::new().with_external_state(serde_json::json!({
            "extra": "external_value"
        }));

        let (composed, report) = md.compose_with(options).unwrap();

        // Replacement happened
        assert!(composed.content().contains("actual content here"));
        assert_eq!(report.replacements_applied, 1);

        // Interpolation happened
        assert!(composed.content().contains("Welcome Alice"));
        assert!(composed.content().contains("many items"));
        assert_eq!(report.interpolations_applied, 2);

        // Cleanup happened (blank line added)
        assert!(report.cleanup_changed);

        // Full summary
        let summary = report.summary();
        assert!(summary.contains("1 replacement(s)"));
        assert!(summary.contains("2 interpolation(s)"));
    }

    #[test]
    fn test_e2e_unicode_content() {
        // Test Unicode handling in replacement and interpolation
        let content = r#"---
replace:
  ":smile:": "😊"
  ":wave:": "👋"
greeting: こんにちは
---
Hello :wave: {{ greeting }} :smile:"#;

        let md: Markdown = content.into();

        let options = ComposeOptions::new().only(&[
            ComposeOperation::TextReplacement,
            ComposeOperation::Interpolation,
        ]);

        let (composed, report) = md.compose_with(options).unwrap();

        assert_eq!(composed.content(), "Hello 👋 こんにちは 😊");
        assert_eq!(report.replacements_applied, 2);
        assert_eq!(report.interpolations_applied, 1);
    }

    #[test]
    fn test_e2e_helper_functions() {
        let content = r#"---
items:
  - a
  - b
  - c
value: "42"
pi: 3.14159
---
Items: {{ length(items) }}
Number: {{ number(value) }}
Rounded: {{ round(pi) }}"#;

        let md: Markdown = content.into();

        let options = ComposeOptions::new().only(&[ComposeOperation::Interpolation]);

        let (composed, report) = md.compose_with(options).unwrap();

        assert!(composed.content().contains("Items: 3"));
        assert!(composed.content().contains("Number: 42"));
        assert!(composed.content().contains("Rounded: 3"));
        assert_eq!(report.interpolations_applied, 3);
    }

    #[test]
    fn test_stage2_file_transclusion_relevels_to_parent_heading() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join("root.md");
        let child = dir.path().join("child.md");

        std::fs::write(&root, "## Parent\n\n::file ./child.md").unwrap();
        std::fs::write(&child, "# Child\n\nBody").unwrap();

        let md = Markdown::try_from(root.as_path()).unwrap();
        let options = ComposeOptions::new().with_source_file(root);
        let (composed, report) = md.compose_with(options).unwrap();

        assert!(composed.content().contains("### Child"));
        assert!(composed.content().contains("Body"));
        assert_eq!(report.transclusions_applied, 1);
    }

    #[test]
    fn test_stage2_nested_transclusion_counts_recursive_includes() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join("root.md");
        let a = dir.path().join("a.md");
        let b = dir.path().join("b.md");

        std::fs::write(&root, "::file ./a.md").unwrap();
        std::fs::write(&a, "::file ./b.md").unwrap();
        std::fs::write(&b, "# Leaf").unwrap();

        let md = Markdown::try_from(root.as_path()).unwrap();
        let options = ComposeOptions::new().with_source_file(root);
        let (composed, report) = md.compose_with(options).unwrap();

        assert!(composed.content().contains("# Leaf"));
        assert_eq!(report.transclusions_applied, 2);
        assert!(report.max_transclusion_depth >= 2);
    }

    #[test]
    fn test_stage2_duplicate_sibling_includes_are_not_treated_as_cycles() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join("root.md");
        let child = dir.path().join("child.md");

        std::fs::write(&root, "::file ./child.md\n\n::file ./child.md").unwrap();
        std::fs::write(&child, "# Child").unwrap();

        let md = Markdown::try_from(root.as_path()).unwrap();
        let options = ComposeOptions::new().with_source_file(&root);
        let (composed, report) = md.compose_with(options).unwrap();

        assert_eq!(composed.content().matches("# Child").count(), 2);
        assert_eq!(report.transclusions_applied, 2);
    }

    #[test]
    fn test_stage2_diamond_dependency_graph_is_allowed() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join("root.md");
        let left = dir.path().join("left.md");
        let right = dir.path().join("right.md");
        let shared = dir.path().join("shared.md");

        std::fs::write(&root, "::file ./left.md\n\n::file ./right.md").unwrap();
        std::fs::write(&left, "## Left\n\n::file ./shared.md").unwrap();
        std::fs::write(&right, "## Right\n\n::file ./shared.md").unwrap();
        std::fs::write(&shared, "### Shared").unwrap();

        let md = Markdown::try_from(root.as_path()).unwrap();
        let options = ComposeOptions::new().with_source_file(&root);
        let (composed, report) = md.compose_with(options).unwrap();

        assert_eq!(composed.content().matches("### Shared").count(), 2);
        assert_eq!(report.transclusions_applied, 4);
    }

    #[test]
    fn test_stage2_cycle_detection_fails() {
        let dir = tempfile::tempdir().unwrap();
        let a = dir.path().join("a.md");
        let b = dir.path().join("b.md");

        std::fs::write(&a, "::file ./b.md").unwrap();
        std::fs::write(&b, "::file ./a.md").unwrap();

        let md = Markdown::try_from(a.as_path()).unwrap();
        let options = ComposeOptions::new().with_source_file(a);
        let err = md.compose_with(options).unwrap_err();

        assert!(matches!(
            err,
            MarkdownError::Transclusion(transclusion::TransclusionError::CycleDetected { .. })
        ));
    }

    #[test]
    fn test_stage2_code_transclusion_wraps_fenced_block() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join("root.md");
        let code = dir.path().join("main.rs");

        std::fs::write(&root, "## Code\n\n::code ./main.rs").unwrap();
        std::fs::write(&code, "fn main() {\n    println!(\"hi\");\n}\n").unwrap();

        let md = Markdown::try_from(root.as_path()).unwrap();
        let options = ComposeOptions::new().with_source_file(root);
        let (composed, report) = md.compose_with(options).unwrap();

        assert!(composed.content().contains("```rs"));
        assert!(composed.content().contains("fn main()"));
        assert_eq!(report.transclusions_applied, 1);
    }

    #[test]
    fn test_stage2_code_transclusion_uses_fallback_language_for_unknown_extension() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join("root.md");
        let code = dir.path().join("sample.weird");

        std::fs::write(&root, "::code ./sample.weird").unwrap();
        std::fs::write(&code, "hello").unwrap();

        let md = Markdown::try_from(root.as_path()).unwrap();
        let options = ComposeOptions::new().with_source_file(root);
        let (composed, report) = md.compose_with(options).unwrap();

        assert!(composed.content().contains("```txt"));
        assert!(composed.content().contains("hello"));
        assert_eq!(report.transclusions_applied, 1);
    }

    #[test]
    fn test_stage2_repeated_code_includes_are_allowed() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join("root.md");
        let code = dir.path().join("main.rs");

        std::fs::write(&root, "::code ./main.rs\n\n::code ./main.rs").unwrap();
        std::fs::write(&code, "fn repeated() {}\n").unwrap();

        let md = Markdown::try_from(root.as_path()).unwrap();
        let options = ComposeOptions::new().with_source_file(&root);
        let (composed, report) = md.compose_with(options).unwrap();

        assert_eq!(composed.content().matches("fn repeated() {}").count(), 2);
        assert_eq!(report.transclusions_applied, 2);
    }

    #[test]
    fn test_stage2_when_false_skips_directive() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join("root.md");
        let child = dir.path().join("child.md");

        std::fs::write(
            &root,
            "---\nenabled: false\n---\n::file ./child.md when=\"enabled\"",
        )
        .unwrap();
        std::fs::write(&child, "# Child").unwrap();

        let md = Markdown::try_from(root.as_path()).unwrap();
        let options = ComposeOptions::new().with_source_file(root);
        let (composed, report) = md.compose_with(options).unwrap();

        assert!(!composed.content().contains("Child"));
        assert_eq!(report.transclusions_skipped, 1);
    }

    #[test]
    fn test_stage2_frontmatter_prologue_epilogue() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join("root.md");
        let intro = dir.path().join("intro.md");
        let outro = dir.path().join("outro.md");

        std::fs::write(
            &root,
            "---\nprologue: ./intro.md\nepilogue: [\"./outro.md\"]\n---\nBody",
        )
        .unwrap();
        std::fs::write(&intro, "Intro").unwrap();
        std::fs::write(&outro, "Outro").unwrap();

        let md = Markdown::try_from(root.as_path()).unwrap();
        let options = ComposeOptions::new().with_source_file(root);
        let (composed, report) = md.compose_with(options).unwrap();

        assert!(composed.content().starts_with("Intro"));
        assert!(composed.content().contains("Body"));
        assert!(composed.content().trim_end_matches('\n').ends_with("Outro"));
        assert_eq!(report.transclusions_applied, 2);
    }

    #[test]
    fn test_stage2_same_file_can_be_used_in_prologue_and_body() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join("root.md");
        let shared = dir.path().join("shared.md");

        std::fs::write(&root, "---\nprologue: ./shared.md\n---\n::file ./shared.md").unwrap();
        std::fs::write(&shared, "## Shared").unwrap();

        let md = Markdown::try_from(root.as_path()).unwrap();
        let options = ComposeOptions::new().with_source_file(&root);
        let (composed, report) = md.compose_with(options).unwrap();

        assert_eq!(composed.content().matches("## Shared").count(), 2);
        assert_eq!(report.transclusions_applied, 2);
    }

    #[test]
    fn test_stage2_missing_source_context_for_relative_path() {
        let md: Markdown = "::file ./child.md".into();
        let err = md.compose().unwrap_err();
        assert!(matches!(
            err,
            MarkdownError::Transclusion(
                transclusion::TransclusionError::MissingSourceContext { .. }
            )
        ));
    }

    #[test]
    fn test_toc_linking_fail_fast_false_becomes_warning() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join("root.md");
        std::fs::write(&root, "::toc-linking ./missing.md").unwrap();

        let md = Markdown::try_from(root.as_path()).unwrap();
        let options = ComposeOptions::new()
            .with_source_file(&root)
            .with_fail_fast(false);
        let (composed, report) = md.compose_with(options).unwrap();

        assert_eq!(composed.content().trim_end(), "::toc-linking ./missing.md");
        assert_eq!(report.toc_links_generated, 0);
        assert!(
            report
                .warnings
                .iter()
                .any(|warning| warning.message.contains("File not found"))
        );
    }

    #[test]
    fn test_toc_linking_fail_fast_true_returns_error() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join("root.md");
        std::fs::write(&root, "::toc-linking ./missing.md").unwrap();

        let md = Markdown::try_from(root.as_path()).unwrap();
        let options = ComposeOptions::new()
            .with_source_file(&root)
            .with_fail_fast(true);
        let err = md.compose_with(options).unwrap_err();

        assert!(matches!(err, MarkdownError::TocLinking(_)));
    }

    #[test]
    fn test_stage2_h6_overflow_converts_to_bold_text() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join("root.md");
        let child = dir.path().join("child.md");

        std::fs::write(&root, "###### Root\n\n::file ./child.md").unwrap();
        std::fs::write(&child, "## Child\n\n### Deep").unwrap();

        let md = Markdown::try_from(root.as_path()).unwrap();
        let options = ComposeOptions::new().with_source_file(root);
        let (composed, report) = md.compose_with(options).unwrap();

        assert!(composed.content().contains("###### Child"));
        assert!(composed.content().contains("**Deep**"));
        assert!(
            report
                .warnings
                .iter()
                .any(|warning| warning.message.contains("Heading overflow"))
        );
    }

    #[test]
    fn test_stage2_consecutive_file_directives_separated_by_blank_line() {
        // Regression test: when two ::file directives are consecutive, the second
        // file's content must not be absorbed into the last block element (e.g., a
        // list) of the first file.
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join("root.md");
        let one = dir.path().join("one.md");
        let two = dir.path().join("two.md");

        std::fs::write(&root, "::file ./one.md\n\n::file ./two.md").unwrap();
        std::fs::write(&one, "- Item A\n- Item B").unwrap();
        std::fs::write(&two, "## Section Two\n\nParagraph.").unwrap();

        let md = Markdown::try_from(root.as_path()).unwrap();
        let options = ComposeOptions::new().with_source_file(root);
        let (composed, report) = md.compose_with(options).unwrap();

        // Two transclusions should have occurred
        assert_eq!(report.transclusions_applied, 2);
        // The heading from two.md must exist as a proper heading, not inside a list
        assert!(
            composed.content().contains("\n## Section Two\n")
                || composed.content().contains("\n## Section Two"),
            "Second file's heading should not be absorbed into first file's list, got:\n{}",
            composed.content()
        );
    }

    #[test]
    fn test_stage2_frontmatter_inline_string_prologue() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join("root.md");
        std::fs::write(
            &root,
            "---\nprologue: \"**Draft** document\"\n---\nBody content.",
        )
        .unwrap();

        let md = Markdown::try_from(root.as_path()).unwrap();
        let options = ComposeOptions::new().with_source_file(root);
        let (composed, report) = md.compose_with(options).unwrap();

        assert!(composed.content().starts_with("**Draft** document"));
        assert!(composed.content().contains("Body content."));
        assert_eq!(report.transclusions_applied, 0); // inline string is not a transclusion
    }

    #[test]
    fn test_stage2_frontmatter_inline_string_epilogue() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join("root.md");
        std::fs::write(&root, "---\nepilogue: \"End of document.\"\n---\nBody.").unwrap();

        let md = Markdown::try_from(root.as_path()).unwrap();
        let options = ComposeOptions::new().with_source_file(root);
        let (composed, report) = md.compose_with(options).unwrap();

        assert!(
            composed
                .content()
                .trim_end_matches('\n')
                .ends_with("End of document.")
        );
        assert_eq!(report.transclusions_applied, 0);
    }

    #[test]
    fn test_stage2_frontmatter_mixed_file_and_inline() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join("root.md");
        let intro = dir.path().join("intro.md");

        std::fs::write(
            &root,
            "---\nprologue: [\"./intro.md\", \"Inline note.\"]\n---\nBody.",
        )
        .unwrap();
        std::fs::write(&intro, "File intro.").unwrap();

        let md = Markdown::try_from(root.as_path()).unwrap();
        let options = ComposeOptions::new().with_source_file(root);
        let (composed, report) = md.compose_with(options).unwrap();

        let content = composed.content();
        assert!(content.starts_with("File intro."));
        assert!(content.contains("Inline note."));
        assert!(content.contains("Body."));
        assert_eq!(report.transclusions_applied, 1); // only the file counts
    }

    #[test]
    fn test_stage2_frontmatter_bare_filename_is_treated_as_file_reference() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join("root.md");
        let intro = dir.path().join("intro.md");

        std::fs::write(&root, "---\nprologue: intro.md\n---\nBody.").unwrap();
        std::fs::write(&intro, "Intro text.").unwrap();

        let md = Markdown::try_from(root.as_path()).unwrap();
        let options = ComposeOptions::new().with_source_file(root);
        let (composed, report) = md.compose_with(options).unwrap();

        let content = composed.content();
        assert!(content.starts_with("Intro text."));
        assert!(content.contains("Body."));
        assert_eq!(report.transclusions_applied, 1);
    }

    #[test]
    fn test_stage2_parent_frontmatter_propagates_to_child_interpolation() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join("root.md");
        let child = dir.path().join("child.md");

        std::fs::write(&root, "---\nauthor: Alice\n---\n::file ./child.md").unwrap();
        std::fs::write(&child, "Written by {{ author }}.").unwrap();

        let md = Markdown::try_from(root.as_path()).unwrap();
        let options = ComposeOptions::new().with_source_file(root);
        let (composed, _report) = md.compose_with(options).unwrap();

        assert!(composed.content().contains("Written by Alice."));
    }

    #[test]
    fn test_stage2_parent_replace_map_propagates_to_child() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join("root.md");
        let child = dir.path().join("child.md");

        std::fs::write(
            &root,
            "---\nreplace:\n  PLACEHOLDER: actual\n---\n::file ./child.md",
        )
        .unwrap();
        std::fs::write(&child, "Content with PLACEHOLDER here.").unwrap();

        let md = Markdown::try_from(root.as_path()).unwrap();
        let options = ComposeOptions::new().with_source_file(root);
        let (composed, _report) = md.compose_with(options).unwrap();

        assert!(composed.content().contains("Content with actual here."));
    }

    #[test]
    fn test_stage2_replace_parent_wins_inverts_precedence() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join("root.md");
        let child = dir.path().join("child.md");

        std::fs::write(
            &root,
            "---\nreplace:\n  TOKEN: parent\n---\n::file ./child.md replace=true",
        )
        .unwrap();
        std::fs::write(&child, "---\nreplace:\n  TOKEN: child\n---\nTOKEN").unwrap();

        let md = Markdown::try_from(root.as_path()).unwrap();
        let options = ComposeOptions::new().with_source_file(root);
        let (composed, _report) = md.compose_with(options).unwrap();

        assert_eq!(composed.content().trim(), "parent");
    }

    #[test]
    fn test_stage2_replace_one_off_does_not_propagate_to_grandchildren() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join("root.md");
        let child = dir.path().join("child.md");
        let grand = dir.path().join("grand.md");

        std::fs::write(
            &root,
            "---\nreplace:\n  A: root\n---\n::file ./child.md replace={\"ONE\":\"oneoff\"}",
        )
        .unwrap();
        std::fs::write(&child, "Child: ONE A\n::file ./grand.md").unwrap();
        std::fs::write(&grand, "Grand: ONE A").unwrap();

        let md = Markdown::try_from(root.as_path()).unwrap();
        let options = ComposeOptions::new().with_source_file(root);
        let (composed, _report) = md.compose_with(options).unwrap();

        let content = composed.content();
        assert!(content.contains("Child: oneoff root"));
        assert!(content.contains("Grand: ONE root"));
    }

    #[test]
    fn test_stage2_prologue_epilogue_do_not_propagate_to_children() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join("root.md");
        let child = dir.path().join("child.md");

        std::fs::write(
            &root,
            "---\nepilogue: \"Root epilogue.\"\n---\n::file ./child.md",
        )
        .unwrap();
        std::fs::write(&child, "Child body.").unwrap();

        let md = Markdown::try_from(root.as_path()).unwrap();
        let options = ComposeOptions::new().with_source_file(root);
        let (composed, _report) = md.compose_with(options).unwrap();

        let content = composed.content();
        // "Root epilogue." should appear exactly once — at the end of root, not within child
        assert_eq!(content.matches("Root epilogue.").count(), 1);
        assert!(content.trim_end_matches('\n').ends_with("Root epilogue."));
    }

    #[test]
    fn test_stage2_inline_epilogue_with_markdown_links() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join("root.md");

        // Epilogue containing markdown link syntax should be treated as inline
        // content, not as a file reference. Previously, the `/` in `](./...)
        // caused `is_file_like_reference` to misidentify it as a path.
        std::fs::write(
            &root,
            "---\nepilogue: \"---\\n\\n- No [animals](./animals.md) were hurt\"\n---\nBody.",
        )
        .unwrap();

        let md = Markdown::try_from(root.as_path()).unwrap();
        let options = ComposeOptions::new().with_source_file(root);
        let (composed, _report) = md.compose_with(options).unwrap();

        let content = composed.content();
        assert!(
            content.contains("[animals](./animals.md)"),
            "Inline epilogue with markdown links should be preserved, got:\n{}",
            content
        );
    }

    #[test]
    fn test_stage2_exclude_removes_section() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join("root.md");
        let child = dir.path().join("child.md");

        std::fs::write(&root, "::file ./child.md exclude=\"## Remove Me\"").unwrap();
        std::fs::write(
            &child,
            "## Keep\n\nKept body.\n\n## Remove Me\n\nRemoved body.\n\n## Also Keep\n\nAlso kept.",
        )
        .unwrap();

        let md = Markdown::try_from(root.as_path()).unwrap();
        let options = ComposeOptions::new().with_source_file(root);
        let (composed, report) = md.compose_with(options).unwrap();

        assert!(composed.content().contains("## Keep"));
        assert!(composed.content().contains("Kept body."));
        assert!(!composed.content().contains("Remove Me"));
        assert!(!composed.content().contains("Removed body."));
        assert!(composed.content().contains("## Also Keep"));
        assert_eq!(report.transclusions_applied, 1);
    }

    #[test]
    fn test_stage2_exclude_wildcard() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join("root.md");
        let child = dir.path().join("child.md");

        std::fs::write(&root, "::file ./child.md exclude=\"## Remove*\"").unwrap();
        std::fs::write(
            &child,
            "## Keep\n\nKept.\n\n## Remove This\n\nGone.\n\n## Also Keep\n\nStays.",
        )
        .unwrap();

        let md = Markdown::try_from(root.as_path()).unwrap();
        let options = ComposeOptions::new().with_source_file(root);
        let (composed, _) = md.compose_with(options).unwrap();

        assert!(composed.content().contains("## Keep"));
        assert!(!composed.content().contains("Remove This"));
        assert!(composed.content().contains("## Also Keep"));
    }

    #[test]
    fn test_stage2_exclude_prelude() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join("root.md");
        let child = dir.path().join("child.md");

        std::fs::write(&root, "::file ./child.md exclude=\"!prelude\"").unwrap();
        std::fs::write(&child, "Prelude text here.\n\n## Heading\n\nBody.").unwrap();

        let md = Markdown::try_from(root.as_path()).unwrap();
        let options = ComposeOptions::new().with_source_file(root);
        let (composed, _) = md.compose_with(options).unwrap();

        assert!(!composed.content().contains("Prelude text"));
        assert!(composed.content().contains("## Heading"));
        assert!(composed.content().contains("Body."));
    }

    #[test]
    fn test_stage2_multiple_excludes() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join("root.md");
        let child = dir.path().join("child.md");

        std::fs::write(&root, "::file ./child.md exclude=\"## A\" exclude=\"## C\"").unwrap();
        std::fs::write(
            &child,
            "## A\n\nA body.\n\n## B\n\nB body.\n\n## C\n\nC body.",
        )
        .unwrap();

        let md = Markdown::try_from(root.as_path()).unwrap();
        let options = ComposeOptions::new().with_source_file(root);
        let (composed, _) = md.compose_with(options).unwrap();

        assert!(!composed.content().contains("## A"));
        assert!(composed.content().contains("## B"));
        assert!(!composed.content().contains("## C"));
    }

    #[test]
    fn test_stage2_quotation_wrapper_does_not_absorb_following_content() {
        // Regression: wrap_quotation consumed trailing \n\n, causing the
        // next paragraph to become a lazy continuation of the blockquote.
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join("root.md");
        let child = dir.path().join("child.md");

        std::fs::write(
            &root,
            "## Section\n\n::file ./child.md quotation=\"Source\"\n\nFollowing paragraph.",
        )
        .unwrap();
        std::fs::write(&child, "Quoted content here.").unwrap();

        let md = Markdown::try_from(root.as_path()).unwrap();
        let options = ComposeOptions::new().with_source_file(root);
        let (composed, report) = md.compose_with(options).unwrap();

        assert_eq!(report.transclusions_applied, 1);
        // The "Following paragraph" must NOT be inside the blockquote
        let content = composed.content();
        assert!(
            content.contains("\n\nFollowing paragraph."),
            "Following content should be separated from blockquote by blank line, got:\n{}",
            content
        );
        // Verify blockquote is present
        assert!(content.contains("> Quoted content here."));
        assert!(content.contains("> — Source"));
    }

    // ============================================
    // Conditional transclusion tests
    // ============================================

    #[test]
    fn test_stage2_when_env_match_includes_directive() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join("root.md");
        let child = dir.path().join("child.md");

        std::fs::write(
            &root,
            "## Section\n\n::file ./child.md when=\"env.AGENT == 'claude'\"",
        )
        .unwrap();
        std::fs::write(&child, "Claude content.").unwrap();

        let md = Markdown::try_from(root.as_path()).unwrap();
        let mut ctx = types::ComposeContext::capture();
        ctx.env_mut()
            .insert("AGENT".to_string(), "claude".to_string());
        let options = ComposeOptions::new()
            .with_source_file(root)
            .with_context(ctx);
        let (composed, report) = md.compose_with(options).unwrap();

        assert!(composed.content().contains("Claude content."));
        assert_eq!(report.transclusions_applied, 1);
        assert_eq!(report.transclusions_skipped, 0);
    }

    #[test]
    fn test_stage2_when_env_mismatch_skips_directive() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join("root.md");
        let child = dir.path().join("child.md");

        std::fs::write(
            &root,
            "## Section\n\n::file ./child.md when=\"env.AGENT == 'claude'\"",
        )
        .unwrap();
        std::fs::write(&child, "Claude content.").unwrap();

        let md = Markdown::try_from(root.as_path()).unwrap();
        let mut ctx = types::ComposeContext::capture();
        ctx.env_mut()
            .insert("AGENT".to_string(), "opencode".to_string());
        let options = ComposeOptions::new()
            .with_source_file(root)
            .with_context(ctx);
        let (composed, report) = md.compose_with(options).unwrap();

        assert!(!composed.content().contains("Claude content."));
        assert_eq!(report.transclusions_applied, 0);
        assert_eq!(report.transclusions_skipped, 1);
    }

    #[test]
    fn test_stage2_when_env_unset_skips_equality() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join("root.md");
        let child = dir.path().join("child.md");

        std::fs::write(
            &root,
            "## Section\n\n::file ./child.md when=\"env.AGENT == 'claude'\"",
        )
        .unwrap();
        std::fs::write(&child, "Claude content.").unwrap();

        let md = Markdown::try_from(root.as_path()).unwrap();
        // Use a fixed context with no AGENT env var
        let ctx = types::ComposeContext::fixed_for_testing();
        let options = ComposeOptions::new()
            .with_source_file(root)
            .with_context(ctx);
        let (composed, report) = md.compose_with(options).unwrap();

        assert!(!composed.content().contains("Claude content."));
        assert_eq!(report.transclusions_skipped, 1);
    }

    #[test]
    fn test_stage2_mutual_exclusion_conditions() {
        // Three directives with mutually exclusive conditions:
        //   AGENT == 'claude'
        //   AGENT == 'opencode'
        //   !env.AGENT (unset)
        // Only one should match at any time.
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join("root.md");
        let cc = dir.path().join("cc.md");
        let oc = dir.path().join("oc.md");
        let default = dir.path().join("default.md");

        std::fs::write(
            &root,
            "## Section\n\n\
             ::file ./cc.md when=\"env.AGENT == 'claude'\"\n\
             ::file ./oc.md when=\"env.AGENT == 'opencode'\"\n\
             ::file ./default.md when=\"!env.AGENT\"",
        )
        .unwrap();
        std::fs::write(&cc, "CC only.").unwrap();
        std::fs::write(&oc, "OC only.").unwrap();
        std::fs::write(&default, "Default only.").unwrap();

        // Test 1: AGENT=claude → only cc.md included
        let md = Markdown::try_from(root.as_path()).unwrap();
        let mut ctx = types::ComposeContext::capture();
        ctx.env_mut()
            .insert("AGENT".to_string(), "claude".to_string());
        let opts = ComposeOptions::new()
            .with_source_file(&root)
            .with_context(ctx);
        let (out, report) = md.compose_with(opts).unwrap();
        assert!(out.content().contains("CC only."), "Expected CC content");
        assert!(!out.content().contains("OC only."), "Should not contain OC");
        assert!(
            !out.content().contains("Default only."),
            "Should not contain default"
        );
        assert_eq!(report.transclusions_applied, 1);
        assert_eq!(report.transclusions_skipped, 2);

        // Test 2: AGENT=opencode → only oc.md included
        let md = Markdown::try_from(root.as_path()).unwrap();
        let mut ctx = types::ComposeContext::capture();
        ctx.env_mut()
            .insert("AGENT".to_string(), "opencode".to_string());
        let opts = ComposeOptions::new()
            .with_source_file(&root)
            .with_context(ctx);
        let (out, report) = md.compose_with(opts).unwrap();
        assert!(!out.content().contains("CC only."));
        assert!(out.content().contains("OC only."), "Expected OC content");
        assert!(!out.content().contains("Default only."));
        assert_eq!(report.transclusions_applied, 1);
        assert_eq!(report.transclusions_skipped, 2);

        // Test 3: AGENT not set → only default.md included
        let md = Markdown::try_from(root.as_path()).unwrap();
        let ctx = types::ComposeContext::fixed_for_testing();
        let opts = ComposeOptions::new()
            .with_source_file(&root)
            .with_context(ctx);
        let (out, report) = md.compose_with(opts).unwrap();
        assert!(!out.content().contains("CC only."));
        assert!(!out.content().contains("OC only."));
        assert!(
            out.content().contains("Default only."),
            "Expected default content"
        );
        assert_eq!(report.transclusions_applied, 1);
        assert_eq!(report.transclusions_skipped, 2);
    }

    // ============================================
    // Re-leveling tests
    // ============================================

    #[test]
    fn test_stage2_relevel_h1_child_under_h3_parent() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join("root.md");
        let child = dir.path().join("child.md");

        std::fs::write(
            &root,
            "# Title\n\n## Overview\n\n### Details\n\n::file ./child.md",
        )
        .unwrap();
        std::fs::write(&child, "# Child Title\n\n## Child Sub\n\nBody.").unwrap();

        let md = Markdown::try_from(root.as_path()).unwrap();
        let options = ComposeOptions::new().with_source_file(root);
        let (composed, _) = md.compose_with(options).unwrap();

        // Parent heading before directive is H3, so child should be re-leveled:
        // H1 → H4, H2 → H5
        assert!(
            composed.content().contains("#### Child Title"),
            "H1 should become H4, got:\n{}",
            composed.content()
        );
        assert!(
            composed.content().contains("##### Child Sub"),
            "H2 should become H5, got:\n{}",
            composed.content()
        );
    }

    // ── Page block integration tests ────────────────────────────────────

    #[test]
    fn page_block_true_preserves_content_through_pipeline() {
        let content = "---\nflag: true\n---\n\nbefore\n\n::block when=\"flag\"\n\nkept content\n\n::end-block\n\nafter\n";
        let md: Markdown = content.into();

        let options = ComposeOptions::new().only(&[ComposeOperation::PageBlocks]);

        let (composed, report) = md.compose_with(options).unwrap();
        assert!(
            composed.content().contains("kept content"),
            "True block body should be preserved, got:\n{}",
            composed.content()
        );
        assert!(
            composed.content().contains("before"),
            "Content before block should be preserved"
        );
        assert!(
            composed.content().contains("after"),
            "Content after block should be preserved"
        );
        assert_eq!(report.page_blocks_rendered, 1);
    }

    #[test]
    fn page_block_false_removes_content_through_pipeline() {
        let content = "---\nflag: false\n---\n\nbefore\n\n::block when=\"flag\"\n\nremoved\n\n::end-block\n\nafter\n";
        let md: Markdown = content.into();

        let options = ComposeOptions::new().only(&[ComposeOperation::PageBlocks]);

        let (composed, report) = md.compose_with(options).unwrap();
        assert!(
            !composed.content().contains("removed"),
            "False block body should be removed, got:\n{}",
            composed.content()
        );
        assert!(composed.content().contains("before"));
        assert!(composed.content().contains("after"));
        assert_eq!(report.page_blocks_skipped, 1);
    }

    #[test]
    fn page_block_coexists_with_interpolation() {
        let content =
            "---\nshow: true\n---\n\n::block when=\"show\"\n\nShown: {{show}}\n\n::end-block\n";
        let md: Markdown = content.into();

        let options = ComposeOptions::new().only(&[
            ComposeOperation::Interpolation,
            ComposeOperation::PageBlocks,
        ]);

        let (composed, report) = md.compose_with(options).unwrap();
        assert!(
            composed.content().contains("Shown: true"),
            "Page blocks and interpolation should both apply, got:\n{}",
            composed.content()
        );
        assert_eq!(report.page_blocks_rendered, 1);
        assert!(report.interpolations_applied > 0);
    }

    #[test]
    fn page_block_report_and_warnings_populated() {
        let content = "---\na: true\nb: false\n---\n\n::block when=\"a\" unknown=\"x\"\n\nA\n\n::end-block\n\n::block when=\"b\"\n\nB\n\n::end-block\n";
        let md: Markdown = content.into();

        let options = ComposeOptions::new().only(&[ComposeOperation::PageBlocks]);

        let (_, report) = md.compose_with(options).unwrap();
        assert_eq!(report.page_blocks_rendered, 1);
        assert_eq!(report.page_blocks_skipped, 1);
        assert!(
            report
                .warnings
                .iter()
                .any(|w| w.message.contains("unknown")),
            "Should warn about unknown option"
        );
    }

    #[test]
    fn page_block_toggle_disabled_leaves_directives_as_text() {
        let content = "::block when=\"x\"\nbody\n::end-block\n";
        let md: Markdown = content.into();

        let options = ComposeOptions::new().only(&[]);

        let (composed, report) = md.compose_with(options).unwrap();
        assert!(
            composed.content().contains("::block"),
            "With page_blocks disabled, directives should be left as text"
        );
        assert_eq!(report.page_blocks_rendered, 0);
        assert_eq!(report.page_blocks_skipped, 0);
    }

    #[test]
    fn perf_disabled_produces_no_report() {
        let content = "# Test\nSome content";
        let md: Markdown = content.into();

        let options = ComposeOptions::new(); // perf_enabled defaults to false
        let (_, report) = md.compose_with(options).unwrap();
        assert!(report.perf.is_none(), "Perf should be None when disabled");
    }

    #[test]
    fn perf_enabled_produces_report() {
        let content = "# Test\nSome content";
        let md: Markdown = content.into();

        let options = ComposeOptions::new().with_perf(true);
        let (_, report) = md.compose_with(options).unwrap();
        assert!(
            report.perf.is_some(),
            "Perf should be populated when enabled"
        );

        let perf = report.perf.unwrap();
        assert!(perf.total > std::time::Duration::ZERO);
        assert!(!perf.metrics.is_empty(), "Should have at least one metric");

        // Verify expected stages are present
        let stages: Vec<_> = perf.metrics.iter().map(|m| m.stage).collect();
        assert!(stages.contains(&ComposeStage::EffectiveStateBuild));
        assert!(stages.contains(&ComposeStage::Cleanup));
    }

    #[test]
    fn perf_enabled_with_interpolation() {
        let content = "---\nname: World\n---\nHello {{ name }}!";
        let md: Markdown = content.into();

        let options = ComposeOptions::new().with_perf(true);
        let (composed, report) = md.compose_with(options).unwrap();

        assert!(composed.content().contains("Hello World!"));
        let perf = report.perf.unwrap();
        let interp = perf
            .metrics
            .iter()
            .find(|m| m.stage == ComposeStage::Interpolation)
            .unwrap();
        assert_eq!(interp.calls, 1);
    }

    // ── Frontmatter Interpolation Integration Tests ─────────────────

    #[test]
    fn test_frontmatter_interpolation_spec_example() {
        let content = "---\nbase: /path/to/something\nspec: \"{{base}}/spec.md\"\nplan: \"{{base}}/plan.md\"\n---\nThe spec is located at: {{spec}}\nThe plan is located at: {{plan}}";
        let md: Markdown = content.into();
        let (composed, report) = md
            .compose_with(ComposeOptions::new().only(&[
                ComposeOperation::FrontmatterInterpolation,
                ComposeOperation::Interpolation,
            ]))
            .unwrap();

        assert_eq!(report.frontmatter_interpolations_applied, 2);
        assert!(
            composed
                .content()
                .contains("The spec is located at: /path/to/something/spec.md")
        );
        assert!(
            composed
                .content()
                .contains("The plan is located at: /path/to/something/plan.md")
        );
    }

    #[test]
    fn test_frontmatter_interpolation_with_set_overrides() {
        let content = "---\nbase: /original\nspec: \"{{base}}/spec.md\"\n---\nSpec: {{spec}}";
        let md: Markdown = content.into();
        let (composed, report) = md
            .compose_with(
                ComposeOptions::new()
                    .only(&[
                        ComposeOperation::FrontmatterInterpolation,
                        ComposeOperation::Interpolation,
                    ])
                    .with_set_overrides(serde_json::json!({"base": "/override"})),
            )
            .unwrap();

        assert_eq!(report.frontmatter_interpolations_applied, 1);
        assert!(composed.content().contains("Spec: /override/spec.md"));
    }

    #[test]
    fn test_frontmatter_interpolation_arrays_and_objects() {
        let content = "---\nbase: /root\npaths:\n  - \"{{base}}/a\"\n  - \"{{base}}/b\"\nmeta:\n  home: \"{{base}}/home\"\n---\n";
        let md: Markdown = content.into();
        let (_, report) = md
            .compose_with(ComposeOptions::new().only(&[ComposeOperation::FrontmatterInterpolation]))
            .unwrap();

        assert!(report.frontmatter_interpolations_applied >= 3);
    }

    #[test]
    fn test_frontmatter_interpolation_disabled() {
        let content = "---\nbase: /path\nspec: \"{{base}}/spec.md\"\n---\n{{spec}}";
        let md: Markdown = content.into();
        let (composed, report) = md
            .compose_with(
                ComposeOptions::new()
                    .disable(ComposeOperation::FrontmatterInterpolation)
                    .only(&[ComposeOperation::Interpolation]),
            )
            .unwrap();

        assert_eq!(report.frontmatter_interpolations_applied, 0);
        // body interpolation sees the raw templated value
        assert!(composed.content().contains("{{base}}/spec.md"));
    }

    #[test]
    fn test_frontmatter_interpolation_body_still_skips_code() {
        let content = "---\nname: World\n---\nHello {{ name }}! Code: `{{ name }}`";
        let md: Markdown = content.into();
        let (composed, _) = md
            .compose_with(ComposeOptions::new().only(&[
                ComposeOperation::FrontmatterInterpolation,
                ComposeOperation::Interpolation,
            ]))
            .unwrap();

        assert!(composed.content().contains("Hello World!"));
        assert!(composed.content().contains("`{{ name }}`"));
    }

    #[test]
    fn test_frontmatter_interpolation_report_counted_separately() {
        let content = "---\nbase: /path\nspec: \"{{base}}/spec.md\"\n---\nHello {{ spec }}!";
        let md: Markdown = content.into();
        let (_, report) = md
            .compose_with(ComposeOptions::new().only(&[
                ComposeOperation::FrontmatterInterpolation,
                ComposeOperation::Interpolation,
            ]))
            .unwrap();

        assert_eq!(report.frontmatter_interpolations_applied, 1);
        assert_eq!(report.interpolations_applied, 1);
    }

    #[test]
    fn test_frontmatter_interpolation_summary() {
        let mut report = ComposeReport::new();
        report.frontmatter_interpolations_applied = 2;
        let summary = report.summary();
        assert!(summary.contains("2 frontmatter interpolation(s)"));
    }

    #[test]
    fn test_frontmatter_interpolation_report_merge() {
        let mut r1 = ComposeReport::new();
        r1.frontmatter_interpolations_applied = 3;
        let mut r2 = ComposeReport::new();
        r2.frontmatter_interpolations_applied = 5;
        r1.merge(r2);
        assert_eq!(r1.frontmatter_interpolations_applied, 8);
    }

    // ── Nested external state regression tests ────────────────────────

    #[test]
    fn test_frontmatter_interpolation_nested_external_state() {
        // External state has nested keys; frontmatter references them.
        let content = "---\nmeta:\n  author: Local\nspec: \"{{meta.base}}/spec.md\"\n---\n{{spec}}";
        let md: Markdown = content.into();
        let (composed, report) = md
            .compose_with(
                ComposeOptions::new()
                    .with_external_state(serde_json::json!({
                        "meta": {"base": "/root", "author": "Parent"}
                    }))
                    .only(&[
                        ComposeOperation::FrontmatterInterpolation,
                        ComposeOperation::Interpolation,
                    ]),
            )
            .unwrap();

        // meta.base from external state should be deep-merged in
        assert!(
            composed.content().contains("/root/spec.md"),
            "Expected /root/spec.md but got: {}",
            composed.content()
        );
        // frontmatter author should win over external
        assert_eq!(
            composed
                .frontmatter()
                .as_map()
                .get("meta")
                .and_then(|v| v.get("author")),
            Some(&serde_json::json!("Local"))
        );
        assert!(report.frontmatter_interpolations_applied >= 1);
    }

    #[test]
    fn test_external_state_deep_merge_preserves_frontmatter_values() {
        // Both frontmatter and external have nested objects; frontmatter wins on conflict.
        let content =
            "---\nconfig:\n  theme: dark\n---\ntheme={{config.theme}} lang={{config.lang}}";
        let md: Markdown = content.into();
        let (composed, _) = md
            .compose_with(
                ComposeOptions::new()
                    .with_external_state(serde_json::json!({
                        "config": {"theme": "light", "lang": "en"}
                    }))
                    .only(&[ComposeOperation::Interpolation]),
            )
            .unwrap();

        assert!(
            composed.content().contains("theme=dark"),
            "Frontmatter should win: {}",
            composed.content()
        );
        assert!(
            composed.content().contains("lang=en"),
            "External nested key should fill in: {}",
            composed.content()
        );
    }

    // ── Child document frontmatter from parent state ──────────────────

    #[test]
    fn test_child_frontmatter_interpolation_from_parent_state() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join("root.md");
        let child = dir.path().join("child.md");

        std::fs::write(&root, "---\nbase: /docs\n---\n::file ./child.md").unwrap();
        std::fs::write(
            &child,
            "---\nspec: \"{{base}}/spec.md\"\n---\nSpec: {{spec}}",
        )
        .unwrap();

        let md = Markdown::try_from(root.as_path()).unwrap();
        let options = ComposeOptions::new().with_source_file(root);
        let (composed, _) = md.compose_with(options).unwrap();

        assert!(
            composed.content().contains("Spec: /docs/spec.md"),
            "Child should derive frontmatter from parent state: {}",
            composed.content()
        );
    }

    // ── Interpolated prologue/epilogue paths ──────────────────────────

    #[test]
    fn test_interpolated_prologue_path() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join("root.md");
        let intro = dir.path().join("intro.md");

        std::fs::write(
            &root,
            "---\nparts: .\nprologue: \"{{parts}}/intro.md\"\n---\nBody",
        )
        .unwrap();
        std::fs::write(&intro, "Prologue content").unwrap();

        let md = Markdown::try_from(root.as_path()).unwrap();
        let options = ComposeOptions::new().with_source_file(root);
        let (composed, report) = md.compose_with(options).unwrap();

        assert!(
            composed.content().contains("Prologue content"),
            "Interpolated prologue path should resolve: {}",
            composed.content()
        );
        assert!(report.frontmatter_interpolations_applied >= 1);
    }

    #[test]
    fn test_interpolated_epilogue_path() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join("root.md");
        let outro = dir.path().join("outro.md");

        std::fs::write(
            &root,
            "---\nparts: .\nepilogue: \"{{parts}}/outro.md\"\n---\nBody",
        )
        .unwrap();
        std::fs::write(&outro, "Epilogue content").unwrap();

        let md = Markdown::try_from(root.as_path()).unwrap();
        let options = ComposeOptions::new().with_source_file(root);
        let (composed, report) = md.compose_with(options).unwrap();

        assert!(
            composed.content().contains("Epilogue content"),
            "Interpolated epilogue path should resolve: {}",
            composed.content()
        );
        assert!(report.frontmatter_interpolations_applied >= 1);
    }

    // ── Page blocks consuming interpolated frontmatter values ─────────

    #[test]
    fn test_page_block_uses_interpolated_frontmatter() {
        // Frontmatter interpolation produces a value that page blocks consume.
        let content = "---\nbase: show\nflag: \"{{base}}\"\n---\n\n::block when=\"flag\"\n\nVisible\n\n::end-block\n";
        let md: Markdown = content.into();

        let options = ComposeOptions::new().only(&[
            ComposeOperation::FrontmatterInterpolation,
            ComposeOperation::PageBlocks,
        ]);
        let (composed, report) = md.compose_with(options).unwrap();

        assert!(
            composed.content().contains("Visible"),
            "Page block should see interpolated frontmatter value: {}",
            composed.content()
        );
        assert!(report.frontmatter_interpolations_applied >= 1);
        assert!(report.page_blocks_rendered >= 1);
    }

    #[test]
    fn test_page_block_false_from_interpolated_frontmatter() {
        let content = "---\nbase: \"\"\nflag: \"{{base}}\"\n---\n\n::block when=\"flag\"\n\nHidden\n\n::end-block\n\nAfter\n";
        let md: Markdown = content.into();

        let options = ComposeOptions::new().only(&[
            ComposeOperation::FrontmatterInterpolation,
            ComposeOperation::PageBlocks,
        ]);
        let (composed, _) = md.compose_with(options).unwrap();

        assert!(
            !composed.content().contains("Hidden"),
            "Page block with falsy interpolated value should be removed: {}",
            composed.content()
        );
        assert!(composed.content().contains("After"));
    }

    mod frontmatter_shell_expansion_integration {
        use super::*;
        use crate::markdown::compose::shell_expansion::types::{
            ShellApprovalDecision, ShellApprovalHandler, ShellApprovalRequest, ShellExpansionError,
            ShellExpansionOptions,
        };
        use std::sync::Arc;
        use std::time::Duration;
        use tempfile::TempDir;

        struct MockApproval;
        impl ShellApprovalHandler for MockApproval {
            fn approve(
                &self,
                _req: ShellApprovalRequest,
            ) -> Result<ShellApprovalDecision, ShellExpansionError> {
                Ok(ShellApprovalDecision::AllowOnce)
            }
        }

        #[test]
        fn frontmatter_shell_output_visible_to_body_interpolation() {
            let temp_dir = TempDir::new().unwrap();
            let content = "---\ngreeting: \"$(echo hello)\"\n---\nMessage: {{greeting}}\n";
            let md: Markdown = content.into();

            let options = ComposeOptions::new()
                .only(&[
                    ComposeOperation::FrontmatterInterpolation,
                    ComposeOperation::FrontmatterShellExpansion,
                    ComposeOperation::Interpolation,
                ])
                .with_shell(ShellExpansionOptions {
                    policy_root: Some(temp_dir.path().to_path_buf()),
                    approval_handler: Some(Arc::new(MockApproval)),
                    ..Default::default()
                });

            let (composed, report) = md.compose_with(options).unwrap();
            assert!(
                composed.content().contains("Message: hello"),
                "Expected 'Message: hello' in:\n{}",
                composed.content()
            );
            assert_eq!(report.frontmatter_shell_expansions_applied, 1);
        }

        #[test]
        fn frontmatter_interpolation_feeds_into_shell_expansion() {
            let temp_dir = TempDir::new().unwrap();
            let content = "---\nfile: README.md\ndir: \"$(dirname {{file}})\"\n---\nDir: {{dir}}\n";
            let md: Markdown = content.into();

            let options = ComposeOptions::new()
                .only(&[
                    ComposeOperation::FrontmatterInterpolation,
                    ComposeOperation::FrontmatterShellExpansion,
                    ComposeOperation::Interpolation,
                ])
                .with_shell(ShellExpansionOptions {
                    policy_root: Some(temp_dir.path().to_path_buf()),
                    approval_handler: Some(Arc::new(MockApproval)),
                    ..Default::default()
                });

            let (composed, report) = md.compose_with(options).unwrap();
            // dirname README.md returns "."
            assert!(
                composed.content().contains("Dir: ."),
                "Expected 'Dir: .' in:\n{}",
                composed.content()
            );
            assert_eq!(report.frontmatter_shell_expansions_applied, 1);
        }

        #[test]
        fn body_and_frontmatter_shell_coexist() {
            let temp_dir = TempDir::new().unwrap();
            let content =
                "---\nfm_val: \"$(echo from-frontmatter)\"\n---\n::shell echo from-body\n";
            let md: Markdown = content.into();

            let options = ComposeOptions::new()
                .only(&[
                    ComposeOperation::FrontmatterShellExpansion,
                    ComposeOperation::ShellExpansion,
                ])
                .with_shell(ShellExpansionOptions {
                    policy_root: Some(temp_dir.path().to_path_buf()),
                    approval_handler: Some(Arc::new(MockApproval)),
                    ..Default::default()
                });

            let (composed, report) = md.compose_with(options).unwrap();
            assert_eq!(report.frontmatter_shell_expansions_applied, 1);
            assert_eq!(report.shell_expansions_applied, 1);
            assert!(composed.content().contains("from-body"));
        }

        #[test]
        fn frontmatter_shell_with_no_candidates_is_noop() {
            let content = "---\ntitle: Hello\n---\nBody text\n";
            let md: Markdown = content.into();

            let options =
                ComposeOptions::new().only(&[ComposeOperation::FrontmatterShellExpansion]);

            let (composed, report) = md.compose_with(options).unwrap();
            assert_eq!(report.frontmatter_shell_expansions_applied, 0);
            assert!(composed.content().contains("Body text"));
        }

        #[test]
        fn frontmatter_shell_timeout_empty_emits_warning() {
            let temp_dir = TempDir::new().unwrap();
            let content = "---\nval: \"$(sleep 1)\"\n---\nValue: {{val}}\n";
            let md: Markdown = content.into();

            let options = ComposeOptions::new()
                .only(&[
                    ComposeOperation::FrontmatterShellExpansion,
                    ComposeOperation::Interpolation,
                ])
                .with_shell(ShellExpansionOptions {
                    timeout: Duration::from_millis(100),
                    timeout_behavior: super::ShellTimeoutBehavior::EmptyString,
                    policy_root: Some(temp_dir.path().to_path_buf()),
                    approval_handler: Some(Arc::new(MockApproval)),
                    ..Default::default()
                });

            let (composed, report) = md.compose_with(options).unwrap();
            assert!(composed.content().contains("Value: "));
            assert_eq!(report.frontmatter_shell_expansions_applied, 1);
            assert_eq!(report.warnings.len(), 1);
            assert!(report.warnings[0].message.contains("timed out"));
        }

        #[test]
        fn frontmatter_shell_rejects_interpolated_executable() {
            let content = "---\ncmd_name: echo\nval: \"$({{cmd_name}} hello)\"\n---\n";
            let md: Markdown = content.into();

            let options = ComposeOptions::new().only(&[
                ComposeOperation::FrontmatterInterpolation,
                ComposeOperation::FrontmatterShellExpansion,
            ]);

            let err = md.compose_with(options).unwrap_err();
            assert!(
                err.to_string()
                    .contains("Frontmatter shell executable may not come from interpolation")
            );
        }
    }

    mod infix_logic_conditions {
        use super::*;

        fn compose_with_page_blocks(content: &str) -> (String, ComposeReport) {
            let md: Markdown = content.into();
            let options = ComposeOptions::new().only(&[ComposeOperation::PageBlocks]);
            let (composed, report) = md.compose_with(options).unwrap();
            (composed.content().to_string(), report)
        }

        #[test]
        fn page_block_with_infix_and_true() {
            let content =
                "---\na: true\nb: true\n---\n::block when=\"a && b\"\ninside\n::end-block\n";
            let (output, report) = compose_with_page_blocks(content);
            assert!(output.contains("inside"));
            assert_eq!(report.page_blocks_rendered, 1);
            assert_eq!(report.page_blocks_skipped, 0);
        }

        #[test]
        fn page_block_with_infix_and_false() {
            let content =
                "---\na: true\nb: false\n---\n::block when=\"a && b\"\ninside\n::end-block\n";
            let (output, report) = compose_with_page_blocks(content);
            assert!(!output.contains("inside"));
            assert_eq!(report.page_blocks_rendered, 0);
            assert_eq!(report.page_blocks_skipped, 1);
        }

        #[test]
        fn page_block_with_infix_or_one_true() {
            let content =
                "---\na: false\nb: true\n---\n::block when=\"a || b\"\ninside\n::end-block\n";
            let (output, report) = compose_with_page_blocks(content);
            assert!(output.contains("inside"));
            assert_eq!(report.page_blocks_rendered, 1);
        }

        #[test]
        fn page_block_with_infix_or_both_false() {
            let content =
                "---\na: false\nb: false\n---\n::block when=\"a || b\"\ninside\n::end-block\n";
            let (output, _report) = compose_with_page_blocks(content);
            assert!(!output.contains("inside"));
        }

        #[test]
        fn page_block_with_grouped_precedence() {
            // (a || b) && c — grouping overrides default precedence
            let content = "---\na: false\nb: true\nc: true\n---\n::block when=\"(a || b) && c\"\ninside\n::end-block\n";
            let (output, _report) = compose_with_page_blocks(content);
            assert!(output.contains("inside"));

            let content_false = "---\na: false\nb: true\nc: false\n---\n::block when=\"(a || b) && c\"\ninside\n::end-block\n";
            let (output, _report) = compose_with_page_blocks(content_false);
            assert!(!output.contains("inside"));
        }

        #[test]
        fn page_block_fallback_mixed_with_infix() {
            // Fallback `|` binds tighter than `||`. When `missing_var` is unset,
            // the fallback yields "go" which matches the literal, and `|| b`
            // short-circuits to true.
            let content = "---\nb: false\n---\n::block when=\"(missing_var | \\\"go\\\") == \\\"go\\\" || b\"\ninside\n::end-block\n";
            let (output, _report) = compose_with_page_blocks(content);
            assert!(output.contains("inside"));
        }

        #[test]
        fn transclusion_directive_with_mixed_infix_logic() {
            let dir = tempfile::tempdir().unwrap();
            let root = dir.path().join("root.md");
            let child = dir.path().join("child.md");

            // Child loaded only if (enabled || fallback) && !skip
            std::fs::write(&child, "child body").unwrap();
            std::fs::write(
                &root,
                "---\nenabled: true\nskip: false\n---\nbefore\n\n::file child.md when=\"enabled && !skip\"\n\nafter\n",
            )
            .unwrap();

            let options = ComposeOptions::new()
                .with_source_file(&root)
                .only(&[ComposeOperation::BlockTransclusion]);

            let (composed, _) = Markdown::try_from(root.as_path())
                .unwrap()
                .compose_with(options)
                .unwrap();
            assert!(composed.content().contains("child body"));
        }

        #[test]
        fn transclusion_skipped_when_infix_condition_false() {
            let dir = tempfile::tempdir().unwrap();
            let root = dir.path().join("root.md");
            let child = dir.path().join("child.md");

            std::fs::write(&child, "child body").unwrap();
            std::fs::write(
                &root,
                "---\nenabled: true\nskip: true\n---\nbefore\n\n::file child.md when=\"enabled && !skip\"\n\nafter\n",
            )
            .unwrap();

            let options = ComposeOptions::new()
                .with_source_file(&root)
                .only(&[ComposeOperation::BlockTransclusion]);

            let (composed, _) = Markdown::try_from(root.as_path())
                .unwrap()
                .compose_with(options)
                .unwrap();
            assert!(!composed.content().contains("child body"));
        }
    }
}
