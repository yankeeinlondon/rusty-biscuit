//! Per-phase dispatch for the compose pipeline driver.
//!
//! Holds the Inline-Pre / Transclusion / Inline-Post / Finalization dispatchers,
//! defined as `impl Markdown` methods lifted off `compose/mod.rs`. The
//! individual inline stage runners they dispatch to live as free functions in
//! [`super::super::inline`].

use super::super::super::Markdown;
use super::super::super::cleanup;
use super::super::super::normalize::NormalizationError;
use super::super::super::types::{MarkdownError, MarkdownResult};
use super::super::{
    ComposeOperation, ComposeOptions, ComposeReport, ComposeWarning, EffectiveState, SourceRange,
};
use super::super::{
    file_links, inline, link_normalization, link_resolve, perf, shell_blocks, shell_expansion,
    toc_linking, transclusion,
};
use tracing::{debug, info};

use transclusion::{ApplyTarget, SectionSlot, TransclusionEngine};

impl Markdown {
    pub(crate) fn run_inline_pre_operation(
        &mut self,
        operation: ComposeOperation,
        state: &EffectiveState,
        options: &ComposeOptions,
        runtime: &mut shell_expansion::types::PipelineRuntime,
        report: &mut ComposeReport,
        perf: &mut perf::PerfCollector,
    ) -> MarkdownResult<()> {
        match operation {
            // FrontmatterInterpolation is handled before EffectiveState build,
            // not in the generic operation loop.
            ComposeOperation::FrontmatterInterpolation => Ok(()),
            // FrontmatterShellExpansion is handled before EffectiveState build,
            // not in the generic operation loop.
            ComposeOperation::FrontmatterShellExpansion => Ok(()),
            ComposeOperation::TextReplacement => {
                report.replacements_applied = inline::replacement::run_stage(self, state, options);
                Ok(())
            }
            ComposeOperation::PageBlocks => {
                inline::page_blocks::run_stage(self, state, options, runtime, report)
            }
            ComposeOperation::Interpolation => {
                report.interpolations_applied =
                    inline::interpolation::run_stage(self, state, options, runtime, report)?;
                Ok(())
            }
            ComposeOperation::ShellExpansion => {
                inline::shell_expansion::run_stage(self, options, runtime, report, perf)
            }
            ComposeOperation::ShellBlocks => {
                let sb_ctx = self.full_source_context_for_errors();
                let line_offset = self.frontmatter_line_count();
                shell_blocks::run_shell_blocks_stage_for_markdown(
                    &mut self.content,
                    options,
                    &mut runtime.shell,
                    report,
                    &sb_ctx,
                    line_offset,
                )
            }
            ComposeOperation::LinkResolve => link_resolve::link_resolve(self, options, report),
            _ => Ok(()),
        }
    }

    pub(crate) fn run_inline_post_operation(
        &mut self,
        operation: ComposeOperation,
        options: &ComposeOptions,
        report: &mut ComposeReport,
    ) -> MarkdownResult<()> {
        match operation {
            ComposeOperation::Cleanup => {
                // Detect whether cleanup changed the body via an xxHash of the
                // before/after content instead of cloning the whole body and
                // comparing (F34). A cache-key-strength collision would only
                // mis-set the advisory `cleanup_changed` report flag.
                let original_hash = biscuit_hash::xx_hash(&self.content);
                // Fixed-width reflow must run over canonical unwrapped prose, so a
                // requested `fixed_width` forces incidental-newline stripping even
                // under `Preserve`. Otherwise reflow would re-wrap the source's own
                // incidental wrapping rather than the document's canonical form.
                let strip_incidental = options.incidental_newline_mode
                    == cleanup::IncidentalNewlineMode::Strip
                    || options.fixed_width.is_some();
                if strip_incidental {
                    self.content = cleanup::strip_incidental_newlines(&self.content);
                }
                self.content = match options.list_spacing {
                    cleanup::ListSpacingMode::Normal => {
                        cleanup::cleanup_content_with_indent_preserving_incidental(
                            &self.content,
                            options.indent_size,
                        )
                    }
                    cleanup::ListSpacingMode::Compact => {
                        cleanup::cleanup_content_with_indent_compact_preserving_incidental(
                            &self.content,
                            options.indent_size,
                        )
                    }
                    cleanup::ListSpacingMode::Loose => {
                        cleanup::cleanup_content_with_indent_loose_preserving_incidental(
                            &self.content,
                            options.indent_size,
                        )
                    }
                };
                if let Some(width) = options.fixed_width {
                    self.content = cleanup::reflow_to_width(&self.content, width);
                }
                report.cleanup_changed = biscuit_hash::xx_hash(&self.content) != original_hash;
                Ok(())
            }
            ComposeOperation::Normalization => match inline::normalize::run_stage(self) {
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

    pub(crate) fn run_finalization_operation(
        &mut self,
        operation: ComposeOperation,
        options: &ComposeOptions,
        report: &mut ComposeReport,
    ) -> MarkdownResult<()> {
        match operation {
            ComposeOperation::LinkNormalization => {
                link_normalization::normalize_links(self, options, report)
            }
            _ => Ok(()),
        }
    }

    pub(crate) fn run_transclusion_phase(
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
            Some(transclusion::parse_directives(
                &self.content,
                self.source_context_for_errors(),
            )?)
        } else {
            None
        };

        let frontmatter_refs = if operations.contains(&ComposeOperation::FrontmatterTransclusion) {
            Some(transclusion::parse_frontmatter_refs(
                self.frontmatter().as_map(),
                self.source_context_for_errors(),
            )?)
        } else {
            None
        };

        let toc_directives = if operations.contains(&ComposeOperation::TocLinking) {
            Some(toc_linking::parse_directives(&self.content)?)
        } else {
            None
        };

        let file_links_directives = if operations.contains(&ComposeOperation::FileLinks) {
            Some(file_links::parse_file_links_directives(&self.content)?)
        } else {
            None
        };

        if let Some(start) = parse_start {
            perf_collector.record(perf::PerfMetricKind::TransclusionParse, start.elapsed());
        }

        let prepare_start = perf_collector.is_enabled().then(std::time::Instant::now);

        let mut prepared = Vec::new();
        let mut next_order = 0usize;

        // Borrow the document into a transclusion engine for the prepare and
        // resolve stages. The borrow ends before the apply stage mutates
        // `self.content` below.
        let engine = TransclusionEngine::new(self);

        for operation in operations {
            match operation {
                ComposeOperation::BlockTransclusion => {
                    if let Some(directives) = parsed_directives.as_ref() {
                        // When a preflight graph is attached, reuse its
                        // resolved targets as a resolution cache so the engine
                        // skips a second `resolve_target` pass. Spans still
                        // come from `directives` (parsed from the current
                        // content), so the cache cannot reintroduce stale
                        // preflight offsets.
                        let resolved_cache = options
                            .preflight_graph()
                            .map(transclusion::build_resolution_cache);
                        engine.prepare_block_transclusions(
                            directives,
                            transclusion::DirectiveKind::File,
                            state,
                            options,
                            &runtime.remote_fetch,
                            report,
                            &mut prepared,
                            &mut next_order,
                            resolved_cache.as_ref(),
                        )?;
                    }
                }
                ComposeOperation::FrontmatterTransclusion => {
                    if let Some(refs) = frontmatter_refs.as_ref() {
                        engine.prepare_frontmatter_transclusions(
                            refs,
                            state,
                            options,
                            &runtime.remote_fetch,
                            report,
                            &mut prepared,
                            &mut next_order,
                        )?;
                    }
                }
                ComposeOperation::CodeTransclusion => {
                    if let Some(directives) = parsed_directives.as_ref() {
                        // `::code` directives are never in the preflight graph
                        // (they contribute no shell entries), so there is no
                        // resolution cache to reuse here.
                        engine.prepare_block_transclusions(
                            directives,
                            transclusion::DirectiveKind::Code,
                            state,
                            options,
                            &runtime.remote_fetch,
                            report,
                            &mut prepared,
                            &mut next_order,
                            None,
                        )?;
                    }
                }
                ComposeOperation::TocLinking => {
                    if let Some(directives) = toc_directives.as_ref() {
                        engine.prepare_toc_transclusions(
                            directives,
                            report,
                            &mut prepared,
                            &mut next_order,
                        );
                    }
                }
                ComposeOperation::FileLinks => {
                    if let Some(directives) = file_links_directives.as_ref() {
                        engine.prepare_file_links_transclusions(
                            directives,
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
            .map(|item| engine.resolve_prepared_transclusion(item, state, options, &runtime_mutex))
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
                        MarkdownError::Transclusion(ref inner)
                            if matches!(
                                inner.as_ref(),
                                transclusion::TransclusionError::CycleDetected { .. }
                                    | transclusion::TransclusionError::MaxDepthExceeded { .. }
                                    | transclusion::TransclusionError::RemoteFetchFailed { .. }
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
}
