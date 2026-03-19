//! Stage 1 compose pipeline for markdown document preparation.
//!
//! This module provides the `compose()` family of methods on `Markdown`
//! for running preparation compositions in a fixed order:
//!
//! 1. **Text Replacement** - Replace literal strings from frontmatter `replace` map
//! 2. **Interpolation** - Expand `{{variable}}` expressions
//! 3. **TOC Linking** - Expand `::toc-linking` directives into heading link lists
//! 4. **Shell Expansion** - Execute `::shell` directives with security controls
//! 5. **Cleanup** - Normalize markdown formatting
//! 6. **Normalization** - Adjust heading levels
//!
//! ## Examples
//!
//! ```
//! use darkmatter::markdown::Markdown;
//! use darkmatter::markdown::compose::ComposeOptions;
//!
//! let content = "# Hello\nWorld";
//! let mut md: Markdown = content.into();
//!
//! // Transform with default options
//! let report = md.compose_mut().unwrap();
//!
//! // Transform with custom options
//! let options = ComposeOptions::new()
//!     .with_fail_fast(true);
//! let report = md.compose_with(options).unwrap();
//! ```

mod conditions;
pub(crate) mod parse_utils;
mod state;
mod types;

pub mod interpolation;
pub mod page_blocks;
pub mod replacement;
pub mod shell_expansion;
pub mod toc_linking;
pub mod transclusion;

pub use shell_expansion::ShellExpansionError;
pub use state::{EffectiveState, EffectiveStateBuilder};
pub use toc_linking::TocLinkingError;
pub use transclusion::TransclusionError;
pub use types::{
    Stage1Stages, Stage2Stages, TransclusionOptions, ComposeContext, ComposeOptions,
    ComposeReport, ComposeSource, ComposeWarning,
};

use super::Markdown;
use super::cleanup;
use super::normalize::{self, NormalizationError};
use super::types::{MarkdownError, MarkdownResult};
use serde_json::{Map, Value};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

use shell_expansion::{apply_replacements_in_reverse, execute_directive};

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
    /// use darkmatter::markdown::compose::{ComposeOptions, Stage1Stages};
    ///
    /// let content = "# Test\nContent";
    /// let md: Markdown = content.into();
    ///
    /// let options = ComposeOptions::new()
    ///     .with_stages(Stage1Stages {
    ///         normalization: false,
    ///         ..Default::default()
    ///     });
    ///
    /// let (composed, report) = md.compose_with(options).unwrap();
    /// ```
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
    fn run_compose_pipeline(
        &mut self,
        options: ComposeOptions,
    ) -> MarkdownResult<ComposeReport> {
        let mut runtime =
            shell_expansion::types::PipelineRuntime::new(options.transclusion.max_depth);
        self.run_compose_pipeline_internal(options, &mut runtime)
    }

    /// Internal recursive pipeline runner shared by root and child documents.
    pub(crate) fn run_compose_pipeline_internal(
        &mut self,
        options: ComposeOptions,
        runtime: &mut shell_expansion::types::PipelineRuntime,
    ) -> MarkdownResult<ComposeReport> {
        let source_id = match &options.transclusion.source {
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
            runtime.transclusion.enter(id)?;
        }

        let result = (|| {
            let mut report = ComposeReport::new();

            // Apply external state as defaults: fill in null/missing frontmatter keys
            // so the document's frontmatter reflects the merged values.
            if let Some(external) = options.external_state.as_ref().and_then(Value::as_object) {
                let fm = self.frontmatter_mut().as_map_mut();
                for (key, value) in external {
                    match fm.get(key) {
                        None | Some(Value::Null) => {
                            fm.insert(key.clone(), value.clone());
                        }
                        _ => {} // frontmatter already has a non-null value
                    }
                }
            }

            // Apply set overrides: unconditionally overwrite frontmatter keys.
            if let Some(overrides) = options.set_overrides.as_ref().and_then(Value::as_object) {
                let fm = self.frontmatter_mut().as_map_mut();
                for (key, value) in overrides {
                    fm.insert(key.clone(), value.clone());
                }
            }

            // Build effective state for replacement/interpolation and condition checks.
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
                .build();

            // Stage 1: Text Replacement
            if options.stages.replacement {
                let replacements = self.run_replacement_stage(&effective_state, &options);
                report.replacements_applied = replacements;
            }

            // Stage 1: Interpolation
            if options.stages.interpolation {
                let interpolations = self.run_interpolation_stage(&effective_state, &options)?;
                report.interpolations_applied = interpolations;
            }

            // Stage 1: TOC Linking
            if options.stages.toc_linking {
                match toc_linking::process_toc_linking(
                    &self.content,
                    &options.transclusion.source,
                    &options.transclusion,
                    options.fail_fast,
                ) {
                    Ok((new_content, count)) => {
                        if count > 0 {
                            self.content = new_content;
                        }
                        report.toc_links_generated = count;
                    }
                    Err(e) if !options.fail_fast => {
                        report.add_warning(ComposeWarning::new("toc_linking", e.to_string()));
                    }
                    Err(e) => return Err(e.into()),
                }
            }

            // Stage 1: Shell Expansion
            if options.stages.shell_expansion {
                self.run_shell_expansion_stage(&options, runtime, &mut report)?;
            }

            // Stage 1: Cleanup
            if options.stages.cleanup {
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
            }

            // Stage 1: Normalization
            if options.stages.normalization {
                match self.run_normalization_stage() {
                    Ok(norm_report) => {
                        if norm_report.has_changes() {
                            report.normalization_report = Some(norm_report);
                        }
                    }
                    Err(NormalizationError::LevelOverflow { .. }) if !options.fail_fast => {
                        report.add_warning(ComposeWarning::new(
                            "normalization",
                            "Skipped normalization: would overflow H6",
                        ));
                    }
                    Err(e) => {
                        return Err(MarkdownError::Transform(format!(
                            "Normalization failed: {}",
                            e
                        )));
                    }
                }
            }

            // Stage 2: page blocks (conditional content regions).
            if options.stage2.page_blocks {
                self.run_page_blocks_stage(&effective_state, &mut report)?;
            }

            // Stage 2: block transclusion directives.
            if options.stage2.block_transclusion {
                self.run_block_transclusion_stage(
                    &effective_state,
                    &options,
                    runtime,
                    &mut report,
                )?;
            }

            // Stage 2: frontmatter prologue/epilogue transclusion.
            if options.stage2.fm_transclusion {
                self.run_frontmatter_transclusion_stage(
                    &effective_state,
                    &options,
                    runtime,
                    &mut report,
                )?;
            }

            report.max_transclusion_depth = runtime.transclusion.deepest_seen;
            Ok(report)
        })();

        if source_id.is_some() {
            runtime.transclusion.exit();
        }

        result
    }

    /// Runs the text replacement stage.
    ///
    /// Applies text replacements from the `replace` map in effective state.
    /// See [`replacement::apply_replacements`] for algorithm details.
    fn run_replacement_stage(
        &mut self,
        state: &EffectiveState,
        options: &ComposeOptions,
    ) -> usize {
        let (new_content, count) = if let Some(one_off) = &options.one_off_replace {
            let merged_replace = state::merge_replace_maps(state.get_replace_map(), Some(one_off));
            let mut frontmatter = HashMap::new();
            frontmatter.insert("replace".to_string(), Value::Object(merged_replace));
            let scoped_state = EffectiveStateBuilder::new()
                .with_frontmatter(frontmatter)
                .with_context(options.context().clone())
                .build();
            replacement::apply_replacements(&self.content, &scoped_state)
        } else {
            replacement::apply_replacements(&self.content, state)
        };
        if count > 0 {
            self.content = new_content;
        }
        count
    }

    /// Runs the interpolation stage.
    ///
    /// Finds `{{ expression }}` patterns in content and evaluates them
    /// against the effective state. Expressions in code blocks are skipped.
    fn run_interpolation_stage(
        &mut self,
        state: &EffectiveState,
        options: &ComposeOptions,
    ) -> MarkdownResult<usize> {
        use interpolation::{EvalResult, Evaluator, ExpressionFinder, parse};

        let finder = ExpressionFinder::new(&self.content);
        let locations = finder.find_all();

        if locations.is_empty() {
            return Ok(0);
        }

        let evaluator = Evaluator::new(state);
        let mut count = 0;
        let mut new_content = self.content.clone();

        // Apply replacements from end to start to preserve offsets
        for loc in locations.into_iter().rev() {
            match parse(&loc.expression) {
                Ok(expr) => match evaluator.eval(&expr) {
                    EvalResult::Value(replacement) => {
                        new_content.replace_range(loc.start..loc.end, &replacement);
                        count += 1;
                    }
                    EvalResult::Error { message, .. } if options.fail_fast => {
                        return Err(MarkdownError::Transform(format!(
                            "Interpolation evaluation failed for '{}': {}",
                            loc.expression, message
                        )));
                    }
                    EvalResult::Error { .. } => {
                        // Leave original expression in place
                    }
                },
                Err(e) if options.fail_fast => {
                    return Err(MarkdownError::Transform(format!(
                        "Interpolation parse failed for '{}': {}",
                        loc.expression, e
                    )));
                }
                Err(_) => {
                    // Parse error - leave original
                }
            }
        }

        self.content = new_content;
        Ok(count)
    }

    /// Runs the normalization stage.
    ///
    /// Uses `None` as target level, which means headings are not re-leveled
    /// but the document structure is validated.
    fn run_normalization_stage(
        &mut self,
    ) -> Result<normalize::NormalizationReport, NormalizationError> {
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
        if directives.is_empty() {
            return Ok(());
        }

        let policy_paths =
            shell_expansion::resolve_policy_paths(&options.shell, &options.transclusion.source)?;
        runtime.shell.ensure_loaded(&policy_paths)?;

        let mut replacements = Vec::new();

        for directive in directives {
            let replacement =
                execute_directive(&directive, options, &policy_paths, &mut runtime.shell)?;
            replacements.push((directive.span.clone(), replacement));
            report.shell_expansions_applied += 1;
        }

        apply_replacements_in_reverse(&mut self.content, replacements);
        report.shell_approvals_used += runtime.shell.take_recent_approval_count();
        Ok(())
    }

    /// Runs Stage 2 page blocks (conditional content regions).
    fn run_page_blocks_stage(
        &mut self,
        state: &EffectiveState,
        report: &mut ComposeReport,
    ) -> MarkdownResult<()> {
        let regions = page_blocks::parser::parse_page_blocks(&self.content)?;
        if regions.is_empty() {
            return Ok(());
        }

        // Warn for unknown options
        fn warn_unknown_options(
            region: &page_blocks::PageBlockRegion,
            report: &mut ComposeReport,
        ) {
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

    /// Runs Stage 2 block transclusion directives.
    fn run_block_transclusion_stage(
        &mut self,
        state: &EffectiveState,
        options: &ComposeOptions,
        runtime: &mut shell_expansion::types::PipelineRuntime,
        report: &mut ComposeReport,
    ) -> MarkdownResult<()> {
        let directives = transclusion::parse_directives(&self.content)?;
        if directives.is_empty() {
            return Ok(());
        }

        let ignore_invalid = self.resolve_ignore_invalid(options);
        let mut replacements: Vec<(std::ops::Range<usize>, String)> = Vec::new();

        for directive in directives {
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

            if let Some(expr) = &directive.options.when_expr {
                let should_include = transclusion::evaluate_condition(expr, state, directive.line)?;
                if !should_include {
                    report.transclusions_skipped += 1;
                    replacements.push((directive.span.clone(), String::new()));
                    continue;
                }
            }

            let target = transclusion::normalize_reference_token(&directive.raw_target);
            let resolved = match transclusion::resolve_target(
                directive.kind,
                &target,
                &options.transclusion,
                &options.transclusion.source,
                directive.line,
            ) {
                Ok(resolved) => resolved,
                Err(err) if ignore_invalid => {
                    report.transclusions_skipped += 1;
                    report.add_warning(
                        ComposeWarning::new("transclusion", err.to_string())
                            .at_line(directive.line),
                    );
                    replacements.push((directive.span.clone(), String::new()));
                    continue;
                }
                Err(err) => return Err(err.into()),
            };

            let replacement = match (directive.kind, resolved) {
                (
                    transclusion::DirectiveKind::File,
                    transclusion::ResolvedTarget::File { path, .. },
                ) => self.render_markdown_transclusion(
                    &path,
                    Some((directive.span.start, directive.line)),
                    &directive.options,
                    state,
                    options,
                    runtime,
                    report,
                )?,
                (
                    transclusion::DirectiveKind::Code,
                    transclusion::ResolvedTarget::File { path, .. },
                ) => self.render_code_transclusion(&path, &directive.options, state, options)?,
                (
                    transclusion::DirectiveKind::Url,
                    transclusion::ResolvedTarget::Url { url, .. },
                ) => {
                    if ignore_invalid {
                        report.transclusions_skipped += 1;
                        report.add_warning(
                            ComposeWarning::new(
                                "transclusion",
                                format!(
                                    "Skipping URL transclusion '{}': remote execution disabled",
                                    url
                                ),
                            )
                            .at_line(directive.line),
                        );
                        String::new()
                    } else {
                        return Err(transclusion::TransclusionError::UrlExecutionDisabled {
                            url: url.to_string(),
                        }
                        .into());
                    }
                }
                (_, target) => {
                    return Err(transclusion::TransclusionError::UnsupportedReferenceType {
                        reference: target.id().to_string(),
                    }
                    .into());
                }
            };

            report.transclusions_applied += 1;
            replacements.push((directive.span.clone(), replacement));
        }

        if replacements.is_empty() {
            return Ok(());
        }

        replacements.sort_by(|left, right| right.0.start.cmp(&left.0.start));
        let mut next = self.content.clone();
        for (span, replacement) in replacements {
            next.replace_range(span, &replacement);
        }
        self.content = next;
        Ok(())
    }

    /// Runs Stage 2 frontmatter transclusion (`prologue`, `epilogue`).
    fn run_frontmatter_transclusion_stage(
        &mut self,
        state: &EffectiveState,
        options: &ComposeOptions,
        runtime: &mut shell_expansion::types::PipelineRuntime,
        report: &mut ComposeReport,
    ) -> MarkdownResult<()> {
        let refs = transclusion::parse_frontmatter_refs(self.frontmatter().as_map())?;
        if refs.prologue.is_empty() && refs.epilogue.is_empty() {
            return Ok(());
        }

        let ignore_invalid = self.resolve_ignore_invalid(options);
        let mut prologue_blocks = Vec::new();
        let mut epilogue_blocks = Vec::new();

        for reference in refs.prologue {
            match self.render_frontmatter_reference(
                &reference,
                state,
                options,
                runtime,
                report,
                ignore_invalid,
            )? {
                Some(content) => {
                    if transclusion::is_file_like_reference(&reference)
                        || transclusion::is_url_like(&reference)
                    {
                        report.transclusions_applied += 1;
                    }
                    prologue_blocks.push(content);
                }
                None => {
                    report.transclusions_skipped += 1;
                }
            }
        }

        for reference in refs.epilogue {
            match self.render_frontmatter_reference(
                &reference,
                state,
                options,
                runtime,
                report,
                ignore_invalid,
            )? {
                Some(content) => {
                    if transclusion::is_file_like_reference(&reference)
                        || transclusion::is_url_like(&reference)
                    {
                        report.transclusions_applied += 1;
                    }
                    epilogue_blocks.push(content);
                }
                None => {
                    report.transclusions_skipped += 1;
                }
            }
        }

        let mut sections = Vec::new();
        sections.extend(prologue_blocks);
        sections.push(self.content.clone());
        sections.extend(epilogue_blocks);
        self.content = sections
            .into_iter()
            .filter(|part| !part.trim().is_empty())
            .collect::<Vec<_>>()
            .join("\n\n");

        Ok(())
    }

    fn render_frontmatter_reference(
        &self,
        reference: &str,
        state: &EffectiveState,
        options: &ComposeOptions,
        runtime: &mut shell_expansion::types::PipelineRuntime,
        report: &mut ComposeReport,
        ignore_invalid: bool,
    ) -> MarkdownResult<Option<String>> {
        // Inline string content: not a URL, not a file path → use as-is.
        if !transclusion::is_url_like(reference) && !transclusion::is_file_like_reference(reference)
        {
            return Ok(Some(reference.to_string()));
        }

        let kind = if transclusion::is_url_like(reference) {
            transclusion::DirectiveKind::Url
        } else {
            transclusion::DirectiveKind::File
        };

        let resolved = match transclusion::resolve_target(
            kind,
            reference,
            &options.transclusion,
            &options.transclusion.source,
            0,
        ) {
            Ok(resolved) => resolved,
            Err(err) if ignore_invalid => {
                report.add_warning(ComposeWarning::new("transclusion", err.to_string()));
                return Ok(None);
            }
            Err(err) => return Err(err.into()),
        };

        match resolved {
            transclusion::ResolvedTarget::File { path, .. } => self
                .render_markdown_transclusion(
                    &path,
                    None,
                    &transclusion::BlockOptions::default(),
                    state,
                    options,
                    runtime,
                    report,
                )
                .map(Some),
            transclusion::ResolvedTarget::Url { url, .. } => {
                if ignore_invalid {
                    report.add_warning(ComposeWarning::new(
                        "transclusion",
                        format!(
                            "Skipping URL transclusion '{}': remote execution disabled",
                            url
                        ),
                    ));
                    Ok(None)
                } else {
                    Err(transclusion::TransclusionError::UrlExecutionDisabled {
                        url: url.to_string(),
                    }
                    .into())
                }
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
        let inherited = self.build_child_external_state(state);
        let mut child_options = options
            .clone()
            .with_replace_parent_wins(matches!(
                directive_options.replace,
                transclusion::ReplaceOption::ParentWins
            ))
            .with_one_off_replace(match &directive_options.replace {
                transclusion::ReplaceOption::OneOff(one_off) => Some(one_off.clone()),
                _ => None,
            });
        child_options.external_state = Some(inherited);
        child_options.transclusion.source = ComposeSource::File(path.to_path_buf());

        let mut child = Markdown::try_from(path)?;
        let child_report = child.run_compose_pipeline_internal(child_options, runtime)?;

        report.transclusions_applied += child_report.transclusions_applied;
        report.transclusions_skipped += child_report.transclusions_skipped;
        for warning in child_report.warnings {
            report.add_warning(warning);
        }

        let mut content = child.content().to_string();

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
        path: &PathBuf,
        directive_options: &transclusion::BlockOptions,
        state: &EffectiveState,
        options: &ComposeOptions,
    ) -> MarkdownResult<String> {
        let raw = match std::fs::read_to_string(path) {
            Ok(content) => content,
            Err(err) if err.kind() == std::io::ErrorKind::InvalidData => {
                return Err(transclusion::TransclusionError::NonTextCodeSource {
                    path: path.clone(),
                }
                .into());
            }
            Err(err) => return Err(err.into()),
        };

        let base_map = state.get_replace_map().cloned().unwrap_or_default();
        let effective_map = match &directive_options.replace {
            transclusion::ReplaceOption::InheritDefault => base_map,
            transclusion::ReplaceOption::ParentWins => base_map,
            transclusion::ReplaceOption::OneOff(one_off) => {
                state::merge_replace_maps(Some(&base_map), Some(one_off))
            }
        };

        let replaced = if effective_map.is_empty() {
            raw
        } else {
            self.apply_replace_map(&raw, &effective_map, options)
        };

        let language =
            transclusion::infer_language(path, &options.transclusion.code_fallback_language);
        let fenced = transclusion::wrap_in_code_block(&replaced, &language);
        let spaced = transclusion::ensure_vertical_spacing(&fenced);
        Ok(self.apply_wrappers(spaced, directive_options))
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

    fn apply_replace_map(
        &self,
        content: &str,
        map: &Map<String, Value>,
        options: &ComposeOptions,
    ) -> String {
        let mut frontmatter = HashMap::new();
        frontmatter.insert("replace".to_string(), Value::Object(map.clone()));

        let state = EffectiveStateBuilder::new()
            .with_frontmatter(frontmatter)
            .with_context(options.context().clone())
            .build();
        let (replaced, _) = replacement::apply_replacements(content, &state);
        replaced
    }

    fn build_child_external_state(&self, state: &EffectiveState) -> Value {
        let mut inherited: Map<String, Value> = state.data().clone().into_iter().collect();

        // Prologue/epilogue are scoped to the defining document — never propagate.
        inherited.remove("prologue");
        inherited.remove("epilogue");

        Value::Object(inherited)
    }

    fn resolve_ignore_invalid(&self, options: &ComposeOptions) -> bool {
        if let Some(value) = options.transclusion.ignore_invalid {
            return value;
        }

        if let Ok(Some(value)) = self.fm_get::<bool>("ignore_invalid") {
            return value;
        }

        options
            .context()
            .env
            .get("IGNORE_INVALID")
            .and_then(|raw| parse_bool(raw))
            .unwrap_or(false)
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

        let options = ComposeOptions::new().with_stages(Stage1Stages {
            cleanup: false,
            normalization: false,
            ..Default::default()
        });

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

        let options = ComposeOptions::new().with_stages(Stage1Stages::only_cleanup());

        let (composed, report) = md.compose_with(options).unwrap();

        // Cleanup should add blank line between header and paragraph
        assert!(composed.content().contains("\n\n"));
        assert!(report.cleanup_changed);
    }

    #[test]
    fn test_compose_normalization_stage_no_change() {
        let content = "# Hello\n\n## World";
        let md: Markdown = content.into();

        let options = ComposeOptions::new().with_stages(Stage1Stages::only_normalization());

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

        let options = ComposeOptions::new().with_stages(Stage1Stages::none());

        let (composed, report) = md.compose_with(options).unwrap();

        // No changes should be made
        assert_eq!(composed.content(), md.content());
        assert!(!report.has_changes());
    }

    #[test]
    fn test_compose_stages_run_in_order() {
        // This test verifies that stages run in the expected order
        // by observing their effects

        let content = "# Header\nParagraph";
        let md: Markdown = content.into();

        // Run all stages
        let (_, report) = md.compose().unwrap();

        // Verify stages ran (via report)
        assert_eq!(report.replacements_applied, 0); // No replace map in frontmatter
        assert_eq!(report.interpolations_applied, 0); // Stub (Phase 2)
        // Cleanup should have run
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
        assert!(!ctx.today.is_empty());
        assert!(!ctx.year.is_empty());
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

        let options = ComposeOptions::new().with_stages(Stage1Stages::only_replacement());

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

        let options = ComposeOptions::new().with_stages(Stage1Stages::only_replacement());

        let (composed, report) = md.compose_with(options).unwrap();

        assert_eq!(composed.content(), "long and short");
        assert_eq!(report.replacements_applied, 2);
    }

    #[test]
    fn test_replacement_stage_non_recursive() {
        // Replacement output should NOT be re-scanned
        let content = "---\nreplace:\n  foo: foobar\n  foobar: baz\n---\nfoo";
        let md: Markdown = content.into();

        let options = ComposeOptions::new().with_stages(Stage1Stages::only_replacement());

        let (composed, report) = md.compose_with(options).unwrap();

        // "foo" -> "foobar" but NOT -> "baz"
        assert_eq!(composed.content(), "foobar");
        assert_eq!(report.replacements_applied, 1);
    }

    #[test]
    fn test_replacement_stage_null_value() {
        let content = "---\nreplace:\n  remove_me: null\n---\nHello remove_me world";
        let md: Markdown = content.into();

        let options = ComposeOptions::new().with_stages(Stage1Stages::only_replacement());

        let (composed, report) = md.compose_with(options).unwrap();

        assert_eq!(composed.content(), "Hello  world");
        assert_eq!(report.replacements_applied, 1);
    }

    #[test]
    fn test_replacement_stage_number_value() {
        let content = "---\nreplace:\n  VERSION: 42\n---\nVersion: VERSION";
        let md: Markdown = content.into();

        let options = ComposeOptions::new().with_stages(Stage1Stages::only_replacement());

        let (composed, report) = md.compose_with(options).unwrap();

        assert_eq!(composed.content(), "Version: 42");
        assert_eq!(report.replacements_applied, 1);
    }

    #[test]
    fn test_replacement_stage_no_replace_in_frontmatter() {
        let content = "---\ntitle: Test\n---\n# Hello foo";
        let md: Markdown = content.into();

        let options = ComposeOptions::new().with_stages(Stage1Stages::only_replacement());

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
            .with_stages(Stage1Stages::only_replacement())
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
            .with_stages(Stage1Stages::only_replacement())
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

        let options = ComposeOptions::new().with_stages(Stage1Stages::only_replacement());

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
        let options = ComposeOptions::new().with_stages(Stage1Stages {
            replacement: true,
            interpolation: false,
            toc_linking: false,
            shell_expansion: false,
            cleanup: true,
            normalization: false,
        });

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

        let options = ComposeOptions::new().with_stages(Stage1Stages::only_interpolation());

        let (composed, report) = md.compose_with(options).unwrap();

        assert_eq!(composed.content(), "# Hello Alice!");
        assert_eq!(report.interpolations_applied, 1);
    }

    #[test]
    fn test_interpolation_nested_variable() {
        let content = "---\nuser:\n  name: Bob\n---\nWelcome {{ user.name }}";
        let md: Markdown = content.into();

        let options = ComposeOptions::new().with_stages(Stage1Stages::only_interpolation());

        let (composed, report) = md.compose_with(options).unwrap();

        assert_eq!(composed.content(), "Welcome Bob");
        assert_eq!(report.interpolations_applied, 1);
    }

    #[test]
    fn test_interpolation_missing_variable() {
        let content = "---\ntitle: Test\n---\nHello {{ missing }}!";
        let md: Markdown = content.into();

        let options = ComposeOptions::new().with_stages(Stage1Stages::only_interpolation());

        let (composed, report) = md.compose_with(options).unwrap();

        // Missing variables become empty string
        assert_eq!(composed.content(), "Hello !");
        assert_eq!(report.interpolations_applied, 1);
    }

    #[test]
    fn test_interpolation_fallback_uses_default() {
        let content = "---\ntitle: Test\n---\nColor: {{ color | \"unknown\" }}";
        let md: Markdown = content.into();

        let options = ComposeOptions::new().with_stages(Stage1Stages::only_interpolation());

        let (composed, report) = md.compose_with(options).unwrap();

        assert_eq!(composed.content(), "Color: unknown");
        assert_eq!(report.interpolations_applied, 1);
    }

    #[test]
    fn test_interpolation_fallback_uses_primary() {
        let content = "---\ncolor: blue\n---\nColor: {{ color | \"unknown\" }}";
        let md: Markdown = content.into();

        let options = ComposeOptions::new().with_stages(Stage1Stages::only_interpolation());

        let (composed, report) = md.compose_with(options).unwrap();

        assert_eq!(composed.content(), "Color: blue");
        assert_eq!(report.interpolations_applied, 1);
    }

    #[test]
    fn test_interpolation_ternary_true() {
        let content = "---\nactive: true\n---\nStatus: {{ active ? \"on\" : \"off\" }}";
        let md: Markdown = content.into();

        let options = ComposeOptions::new().with_stages(Stage1Stages::only_interpolation());

        let (composed, report) = md.compose_with(options).unwrap();

        assert_eq!(composed.content(), "Status: on");
        assert_eq!(report.interpolations_applied, 1);
    }

    #[test]
    fn test_interpolation_ternary_false() {
        let content = "---\nactive: false\n---\nStatus: {{ active ? \"on\" : \"off\" }}";
        let md: Markdown = content.into();

        let options = ComposeOptions::new().with_stages(Stage1Stages::only_interpolation());

        let (composed, report) = md.compose_with(options).unwrap();

        assert_eq!(composed.content(), "Status: off");
        assert_eq!(report.interpolations_applied, 1);
    }

    #[test]
    fn test_interpolation_comparison_equal() {
        let content = "---\ncount: 5\n---\n{{ count == 5 ? \"five\" : \"other\" }}";
        let md: Markdown = content.into();

        let options = ComposeOptions::new().with_stages(Stage1Stages::only_interpolation());

        let (composed, report) = md.compose_with(options).unwrap();

        assert_eq!(composed.content(), "five");
        assert_eq!(report.interpolations_applied, 1);
    }

    #[test]
    fn test_interpolation_comparison_greater_than() {
        let content = "---\ncount: 10\n---\n{{ count > 5 ? \"many\" : \"few\" }}";
        let md: Markdown = content.into();

        let options = ComposeOptions::new().with_stages(Stage1Stages::only_interpolation());

        let (composed, report) = md.compose_with(options).unwrap();

        assert_eq!(composed.content(), "many");
        assert_eq!(report.interpolations_applied, 1);
    }

    #[test]
    fn test_interpolation_multiple_expressions() {
        let content = "---\nfirst: Alice\nlast: Smith\n---\n{{ first }} {{ last }}";
        let md: Markdown = content.into();

        let options = ComposeOptions::new().with_stages(Stage1Stages::only_interpolation());

        let (composed, report) = md.compose_with(options).unwrap();

        assert_eq!(composed.content(), "Alice Smith");
        assert_eq!(report.interpolations_applied, 2);
    }

    #[test]
    fn test_interpolation_skips_code_span() {
        let content = "---\nname: Alice\n---\nHello {{ name }}! Code: `{{ name }}`";
        let md: Markdown = content.into();

        let options = ComposeOptions::new().with_stages(Stage1Stages::only_interpolation());

        let (composed, report) = md.compose_with(options).unwrap();

        // Only the first expression is expanded, code span preserved
        assert_eq!(composed.content(), "Hello Alice! Code: `{{ name }}`");
        assert_eq!(report.interpolations_applied, 1);
    }

    #[test]
    fn test_interpolation_skips_fenced_code() {
        let content = "---\nname: Alice\n---\nHello {{ name }}!\n\n```\n{{ name }}\n```";
        let md: Markdown = content.into();

        let options = ComposeOptions::new().with_stages(Stage1Stages::only_interpolation());

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

        let options = ComposeOptions::new().with_stages(Stage1Stages::only_interpolation());

        let (composed, report) = md.compose_with(options).unwrap();

        assert_eq!(composed.content(), md.content());
        assert_eq!(report.interpolations_applied, 0);
    }

    #[test]
    fn test_interpolation_with_external_state() {
        let content = "# Hello {{ name }}!";
        let md: Markdown = content.into();

        let options = ComposeOptions::new()
            .with_stages(Stage1Stages::only_interpolation())
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
            .with_stages(Stage1Stages::only_interpolation())
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

        let options = ComposeOptions::new().with_stages(Stage1Stages::only_interpolation());

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
            .with_stages(Stage1Stages::only_interpolation())
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
            .with_stages(Stage1Stages::only_interpolation())
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

        let options = ComposeOptions::new().with_stages(Stage1Stages::only_interpolation());

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

        let options = ComposeOptions::new().with_stages(Stage1Stages {
            replacement: true,
            interpolation: true,
            toc_linking: false,
            shell_expansion: false,
            cleanup: false,
            normalization: false,
        });

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

        let options = ComposeOptions::new().with_stages(Stage1Stages::only_interpolation());

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
        assert!(
            composed
                .content()
                .trim_end_matches('\n')
                .ends_with("Outro")
        );
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
        ctx.env.insert("AGENT".to_string(), "claude".to_string());
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
        ctx.env.insert("AGENT".to_string(), "opencode".to_string());
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
        ctx.env.insert("AGENT".to_string(), "claude".to_string());
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
        ctx.env.insert("AGENT".to_string(), "opencode".to_string());
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

        let options = ComposeOptions::new()
            .with_stages(Stage1Stages::none())
            .with_stage2(Stage2Stages::only_page_blocks());

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

        let options = ComposeOptions::new()
            .with_stages(Stage1Stages::none())
            .with_stage2(Stage2Stages::only_page_blocks());

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
        // Stage 1 interpolation output should be visible to page block conditions
        let content =
            "---\nshow: true\n---\n\n::block when=\"show\"\n\nShown: {{show}}\n\n::end-block\n";
        let md: Markdown = content.into();

        let options = ComposeOptions::new().with_stages(Stage1Stages {
            interpolation: true,
            ..Stage1Stages::none()
        });

        let (composed, report) = md.compose_with(options).unwrap();
        assert!(
            composed.content().contains("Shown: true"),
            "Interpolation should run before page blocks, got:\n{}",
            composed.content()
        );
        assert_eq!(report.page_blocks_rendered, 1);
        assert!(report.interpolations_applied > 0);
    }

    #[test]
    fn page_block_report_and_warnings_populated() {
        let content = "---\na: true\nb: false\n---\n\n::block when=\"a\" unknown=\"x\"\n\nA\n\n::end-block\n\n::block when=\"b\"\n\nB\n\n::end-block\n";
        let md: Markdown = content.into();

        let options = ComposeOptions::new()
            .with_stages(Stage1Stages::none())
            .with_stage2(Stage2Stages::only_page_blocks());

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

        let options = ComposeOptions::new()
            .with_stages(Stage1Stages::none())
            .with_stage2(Stage2Stages::none());

        let (composed, report) = md.compose_with(options).unwrap();
        assert!(
            composed.content().contains("::block"),
            "With page_blocks disabled, directives should be left as text"
        );
        assert_eq!(report.page_blocks_rendered, 0);
        assert_eq!(report.page_blocks_skipped, 0);
    }
}
