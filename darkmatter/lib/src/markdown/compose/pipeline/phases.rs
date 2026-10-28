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
    ComposeOperation, ComposeReport, ComposeWarning, EffectiveState, SourceRange,
};
use super::super::{
    file_links, inline, link_normalization, link_resolve, perf, shell_blocks, shell_expansion,
    toc_linking, transclusion,
};
use tracing::{debug, info};

use transclusion::{ApplyTarget, ResolvedTransclusion, SectionSlot, TransclusionEngine};
use super::super::body_origin::{BodyProvenance, EditOrigin};
use super::super::site_scope::{SiteHook, SiteLookup};
use super::super::value_origin::DataPaths;

impl Markdown {
    /// Runs one inline-pre `operation`. `body` records where each byte of the
    /// body came from: every stage that rewrites the body carries it forward,
    /// every stage that scans for instructions skips its data, and body
    /// interpolation projects failure spans through its origin map.
    /// `frontmatter_data` names the frontmatter leaves that are data.
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn run_inline_pre_operation(
        &mut self,
        operation: ComposeOperation,
        state: &EffectiveState,
        options: &crate::markdown::compose::ComposeRequest,
        runtime: &mut shell_expansion::types::PipelineRuntime,
        report: &mut ComposeReport,
        perf: &mut perf::PerfCollector,
        body: &mut BodyProvenance,
        frontmatter_data: &DataPaths,
        schemas: &super::super::schema_validation::PreparedSchemas,
    ) -> MarkdownResult<()> {
        match operation {
            // FrontmatterInterpolation is handled before EffectiveState build,
            // not in the generic operation loop.
            ComposeOperation::FrontmatterInterpolation => Ok(()),
            // FrontmatterShellExpansion is handled before EffectiveState build,
            // not in the generic operation loop.
            ComposeOperation::FrontmatterShellExpansion => Ok(()),
            ComposeOperation::TextReplacement => {
                report.replacements_applied = inline::replacement::run_stage(
                    self,
                    state,
                    options,
                    body,
                    frontmatter_data,
                )?;
                Ok(())
            }
            ComposeOperation::PageBlocks => {
                inline::page_blocks::run_stage(self, state, options, runtime, report, body, schemas)
            }
            ComposeOperation::CollectionBlocks => {
                inline::collection_blocks::run_stage(self, state, options, runtime, report, body, schemas)
            }
            ComposeOperation::Interpolation => {
                report.interpolations_applied =
                    inline::interpolation::run_stage(self, state, options, runtime, report, body, schemas)
                        .map_err(|e| e.with_on_disk_source(&self.full_source_context_for_errors()))?;
                Ok(())
            }
            ComposeOperation::ShellExpansion => {
                inline::shell_expansion::run_stage(self, state, schemas, options, runtime, report, perf, body)
            }
            ComposeOperation::ShellBlocks => self.run_shell_blocks_operation(state, options, runtime, report, body, schemas),
            ComposeOperation::LinkResolve => {
                let before = self.content.clone();
                let edits = link_resolve::link_resolve_with_edits(self, options, report)?;
                body.advance(&before, &edits, &self.content)
            }
            _ => Ok(()),
        }
    }

    /// The [`ComposeOperation::ShellBlocks`] arm of
    /// [`run_inline_pre_operation`](Self::run_inline_pre_operation).
    ///
    /// Out of line so its evaluator and templates stay out of the dispatch
    /// frame, which the interpolation stage's nested compositions recur
    /// through.
    #[inline(never)]
    fn run_shell_blocks_operation(
        &mut self,
        state: &EffectiveState,
        options: &crate::markdown::compose::ComposeRequest,
        runtime: &mut shell_expansion::types::PipelineRuntime,
        report: &mut ComposeReport,
        body: &mut BodyProvenance,
        schemas: &super::super::schema_validation::PreparedSchemas,
    ) -> MarkdownResult<()> {
        let sb_ctx = self.full_source_context_for_errors();
        let line_offset = self.frontmatter_line_count();
        body.data.ensure_describes(&self.content)?;
        let document = super::super::context::effective_state::ResolvingLookup::new(
            state,
            options.expression_resolution_context(&runtime.remote_fetch),
        );
        let lookup = SiteLookup::new(&document, &body.scopes, Some(&runtime.iteration));
        let evaluator = shell_expansion::template::LookupEvaluator::new(&lookup);
        let templates = body
            .authored
            .as_ref()
            .filter(|authored| authored.text().contains("::shell-block"))
            .map(|authored| shell_blocks::BlockTemplates {
                authored,
                scopes: crate::markdown::compose::collection_blocks::BodyScopes::scan_body(
                    authored.text(),
                    line_offset,
                    &options.enabled_operations,
                ),
                scope: shell_expansion::site::SiteScope::new(
                    options,
                    schemas.effective_for(self),
                ),
                evaluator: &evaluator,
                site: Some(&lookup),
                ctx: shell_expansion::site::authored_context(self, Some(authored.text())),
            });
        let (new_content, stage_report, edits) = shell_blocks::run_shell_blocks_stage_in(
            &self.content,
            Some(&body.data),
            templates.as_ref(),
            options,
            &mut runtime.shell,
            &sb_ctx,
            line_offset,
        )?;
        for edit in edits.iter().filter(|edit| edit.origin == EditOrigin::Data) {
            lookup.charge(edit.range.start, edit.replacement_len).map_err(inline::shell_expansion::collection_budget)?;
        }
        body.advance(&self.content, &edits, &new_content)?;
        self.content = new_content;
        report.shell_blocks_applied += stage_report.shell_blocks_applied;
        report.shell_approvals_used += stage_report.shell_approvals_used;
        report.warnings.extend(stage_report.warnings);
        Ok(())
    }

    pub(crate) fn run_inline_post_operation(
        &mut self,
        operation: ComposeOperation,
        options: &crate::markdown::compose::ComposeRequest,
        report: &mut ComposeReport,
    ) -> MarkdownResult<()> {
        match operation {
            ComposeOperation::Cleanup => {
                // Detect whether cleanup changed the body via an xxHash of the
                // before/after content instead of cloning the whole body and
                // comparing (F34). A cache-key-strength collision would only
                // mis-set the advisory `cleanup_changed` report flag.
                let original_hash = biscuit_hash::xx_hash(&self.content);
                // Compose's cleanup options are explicit rule values for this
                // run; a document's own `style.markdown` is not consulted.
                // A width implies unwrapping, so `fixed_width` re-wraps
                // canonical unwrapped prose even under `Preserve`.
                let overrides = cleanup::rules::MarkdownRuleOverrides {
                    unwrap: cleanup::rules::Leaf::Value(
                        options.incidental_newline_mode == cleanup::IncidentalNewlineMode::Strip,
                    ),
                    fixed_width: cleanup::rules::Leaf::Value(match options.fixed_width {
                        Some(width) => cleanup::rules::FixedWidth::Columns(
                            u16::try_from(width).unwrap_or(u16::MAX),
                        ),
                        None => cleanup::rules::FixedWidth::Off,
                    }),
                    indentation: cleanup::rules::Leaf::Value(options.indentation),
                    list_item_padding: cleanup::rules::Leaf::Value(options.list_spacing),
                    ..cleanup::rules::MarkdownRuleOverrides::default()
                };
                self.content = cleanup::cleanup_content_with(&self.content, &overrides);
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
        options: &crate::markdown::compose::ComposeRequest,
        report: &mut ComposeReport,
    ) -> MarkdownResult<()> {
        match operation {
            ComposeOperation::LinkNormalization => {
                link_normalization::normalize_links(self, options, report)
            }
            _ => Ok(()),
        }
    }

    /// Runs the transclusion phase.
    ///
    /// `body` is consumed: the directive parse is the last scan of this
    /// document's body for instructions, so provenance ends here. A directive
    /// inside data is not a directive.
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn run_transclusion_phase(
        &mut self,
        operations: &[ComposeOperation],
        state: &EffectiveState,
        schemas: &super::super::schema_validation::PreparedSchemas,
        options: &crate::markdown::compose::ComposeRequest,
        runtime: &mut shell_expansion::types::PipelineRuntime,
        report: &mut ComposeReport,
        perf_collector: &mut perf::PerfCollector,
        body: BodyProvenance,
    ) -> MarkdownResult<()> {
        use rayon::prelude::*;

        if operations.is_empty() {
            return Ok(());
        }

        info!(operations = ?operations, "compose: starting transclusion phase");
        let parse_start = perf_collector.is_enabled().then(std::time::Instant::now);
        body.data.ensure_describes(&self.content)?;
        let data = Some(&body.data);
        let iteration = runtime.iteration.clone();

        let parsed_directives = if operations.iter().any(|op| {
            matches!(
                op,
                ComposeOperation::BlockTransclusion | ComposeOperation::CodeTransclusion
            )
        }) {
            Some(transclusion::parse_directives_in(
                &self.content,
                data,
                self.source_context_for_errors(),
                0,
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
            Some(toc_linking::parse_directives_in(&self.content, data)?)
        } else {
            None
        };

        let file_links_directives = if operations.contains(&ComposeOperation::FileLinks) {
            Some(file_links::parse_file_links_directives_in(&self.content, data)?)
        } else {
            None
        };

        if let Some(start) = parse_start {
            perf_collector.record(perf::PerfMetricKind::TransclusionParse, start.elapsed());
        }

        let prepare_start = perf_collector.is_enabled().then(std::time::Instant::now);

        let mut prepared = Vec::new();
        let mut next_order = 0usize;
        // The authored site and original items of each directive a
        // collection body repeated.
        let content_now = self.content.clone();
        let line_offset = self.frontmatter_line_count();
        let sites: Vec<Option<super::super::preflight::ScopedSite>> = parsed_directives
            .iter()
            .flatten()
            .map(|directive| body.site_at(&content_now, &directive.span, line_offset))
            .collect();

        // Borrow the document into a transclusion engine for the prepare and
        // resolve stages. The borrow ends before the apply stage mutates
        // `self.content` below.
        let markdown: &Markdown = self;
        let engine = TransclusionEngine::new(markdown);
        let mut watch = super::ReportWatch::new(report, options, super::FinalVerdict { markdown, state, schemas });

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
                            watch.as_mut(),
                            &body.scopes,
                            &sites,
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
                        // Pre-flight records `::code` targets as passive
                        // targets, not graph edges, so there is no
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
                            watch.as_mut(),
                            &body.scopes,
                            &sites,
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

        // Preparing evaluated every directive's `when=`; a blocking warning
        // it reported stops the document before any child composes.
        super::stop_if_blocked(
            report,
            options,
            Some(super::FinalVerdict { markdown: self, state, schemas }),
        )?;

        if prepared.is_empty() {
            return Ok(());
        }

        let resolve_start = perf_collector.is_enabled().then(std::time::Instant::now);

        // 35.1: the effective-state and context hashes are phase-wide — the same
        // `EffectiveState` drives every directive resolved below. Capture the
        // pair once here instead of re-canonicalizing the full state/context
        // maps inside each markdown transclusion's cache-key construction.
        let state_identity = crate::markdown::compose::cache::hashing::PhaseStateIdentity::capture(state);
        let runtime_mutex = std::sync::Mutex::new(runtime);
        let results = prepared
            .into_par_iter()
            .map(|item| {
                // The anchor is taken before `item` is consumed: a resolution
                // error carries no span, and without one a tolerated failure
                // can only be skipped — which leaves the authored directive
                // text in the document.
                let anchor = item.failure_anchor();
                engine
                    .resolve_prepared_transclusion(
                        item,
                        state,
                        state_identity,
                        options,
                        &runtime_mutex,
                    )
                    // Boxed because this pair is the `Err` of a hot `Result`
                    // returned for every prepared item, and the anchor makes it
                    // large enough to widen the success path's stack footprint.
                    .map_err(|error| Box::new((anchor, error)))
            })
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
                Err(failure) => {
                    let (anchor, error) = *failure;
                    if transclusion_failure_is_fatal(&error) || options.fail_fast {
                        return Err(error);
                    }
                    // Tolerating the failure is a promise that the output stays
                    // usable, and a document that silently omits a transcluded
                    // section does not keep it — the gap has to be visible in
                    // the artifact, not only in a warning on stderr. This
                    // matches the removal-and-warn the engine already applies
                    // to resolution failures under `ignore_invalid`, except
                    // that the span is filled rather than emptied.
                    let mut skipped = ComposeReport::new();
                    skipped.transclusions_skipped = 1;
                    skipped.add_warning(tolerated_failure_warning(&error));
                    ResolvedTransclusion {
                        order: anchor.order,
                        content: Some(self.fit_notice_to_span(&anchor.target, anchor.notice)),
                        target: anchor.target,
                        report: skipped,
                        source_file: None,
                    }
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

        for (_, span, replacement, _) in &replacements {
            body.scopes
                .charge(&iteration, span.start, replacement.len())
                .map_err(inline::shell_expansion::collection_budget)?;
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

    /// Shapes a failure notice to sit where the directive it replaces did.
    ///
    /// Every directive parser starts a span at its line's first byte and ends
    /// it past the newline, so the replaced range owns both the leading
    /// indentation and the line break. A bare notice would therefore unnest a
    /// directive written inside a list item and glue the following line onto
    /// itself. Successful replacements carry their own indentation from the
    /// renderer that built them; this is the failure path's equivalent.
    fn fit_notice_to_span(&self, target: &ApplyTarget, notice: String) -> String {
        let ApplyTarget::Replace(span) = target else {
            return notice;
        };
        let indent: String = self.content[span.start..]
            .chars()
            .take_while(|character| *character == ' ' || *character == '\t')
            .collect();
        let newline = if self.content[..span.end].ends_with('\n') {
            "\n"
        } else {
            ""
        };
        format!("{indent}{notice}{newline}")
    }
}

/// Whether a transcluded child's failure must fail the compose instead of
/// being replaced by a notice.
///
/// A child that cannot read its runtime context would leave the partially
/// composed document the missing-capture contract forbids, and a child that
/// reads a host global declared unavailable broke the host's scope rule. A
/// child the design-time call check rejected makes a call no runtime value can
/// bind, which is the document's rejection, not a resolution failure. A child command
/// missing from the approval set is a broken pre-flight invariant, not a
/// failure an author can tolerate.
/// The warning of a tolerated transclusion failure. A target that does not
/// exist is the missing transclusion its recovery governs. Out of line: the
/// caller's frame recurs once per nested composition.
#[inline(never)]
fn tolerated_failure_warning(error: &MarkdownError) -> ComposeWarning {
    let failure = error.resolution_failure();
    let code = (failure == Some(biscuit_file::ResolutionFailure::NoMatch))
        .then(|| super::super::policy::MISSING_TRANSCLUSION_CODE.to_string());
    ComposeWarning { code, ..ComposeWarning::new("transclusion", error.to_string()) }.with_resolution_failure(failure)
}

fn transclusion_failure_is_fatal(error: &MarkdownError) -> bool {
    matches!(
        error,
        MarkdownError::Transclusion(inner)
            if matches!(
                inner.as_ref(),
                transclusion::TransclusionError::CycleDetected { .. }
                    | transclusion::TransclusionError::MaxDepthExceeded { .. }
                    | transclusion::TransclusionError::RemoteFetchFailed { .. }
            )
    ) || error.missing_runtime_context().is_some()
        || error.unavailable_binding().is_some()
        || error.rejected_call().is_some()
        // A policy that blocks the child's warning blocks the document.
        || error.blocked_diagnostics().is_some()
        || error.pre_approval_violation().is_some()
        || error.shell_input_violation().is_some()
        || error.shell_span_failure()
        // A collection failure never becomes a successful notice.
        || matches!(error, MarkdownError::Collection(_))
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::markdown::compose::ShellBlockError;
    use crate::markdown::compose::shell_blocks::types::SourceExcerpt;
    use crate::markdown::compose::shell_expansion::types::{ShellCommandOrigin, ShellExpansionError};

    /// A `NotPreApproved` for `echo main`, as execution raises it.
    pub(crate) fn not_pre_approved() -> ShellExpansionError {
        ShellExpansionError::NotPreApproved {
            ctx: Box::new(biscuit_terminal::errors::SourceContext::new(
                "part.md".into(),
                "part.md".into(),
                "echo main\n",
            )),
            command: "echo main".to_string(),
            origin: ShellCommandOrigin::Body { line: 1 },
            source_desc: " (in part.md)".to_string(),
        }
    }

    /// The same failure as a `::shell-block` command, where a `when_error`
    /// fallback would apply to an ordinary command failure.
    pub(crate) fn not_pre_approved_in_shell_block() -> MarkdownError {
        MarkdownError::from(ShellBlockError::Command {
            block_start_line: 1,
            command_line: 2,
            partial_output: Box::default(),
            excerpt: SourceExcerpt::from_text("echo main\n", 2, 2, 1),
            source: Box::new(not_pre_approved()),
            source_file: None,
        })
    }

    #[test]
    fn a_pre_approval_violation_in_a_child_is_fatal() {
        for error in [
            MarkdownError::from(not_pre_approved()),
            not_pre_approved_in_shell_block(),
        ] {
            assert!(error.pre_approval_violation().is_some(), "{error:?}");
            assert!(transclusion_failure_is_fatal(&error), "{error:?}");
        }
    }

    #[test]
    fn a_child_shell_failure_is_fatal() {
        let not_found = || ShellExpansionError::CommandNotFound {
            ctx: Box::new(biscuit_terminal::errors::SourceContext::new(
                "part.md".into(),
                "part.md".into(),
                "nope\n",
            )),
            command: "nope".to_string(),
            origin: ShellCommandOrigin::Body { line: 1 },
        };
        let in_block = MarkdownError::from(ShellBlockError::Command {
            block_start_line: 1,
            command_line: 2,
            partial_output: Box::default(),
            excerpt: SourceExcerpt::from_text("nope\n", 2, 2, 1),
            source: Box::new(not_found()),
            source_file: None,
        });
        for error in [MarkdownError::from(not_found()), in_block] {
            assert!(error.pre_approval_violation().is_none(), "{error:?}");
            assert!(error.shell_span_failure(), "{error:?}");
            assert!(transclusion_failure_is_fatal(&error), "{error:?}");
        }
    }

    #[test]
    fn an_ordinary_child_failure_stays_tolerable() {
        let error = MarkdownError::Transform("ordinary".to_string());
        assert!(!error.shell_span_failure());
        assert!(!transclusion_failure_is_fatal(&error));
    }
}
