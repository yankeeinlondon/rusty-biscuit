//! Engine helpers for transclusion insertion and heading handling.
//!
//! [`TransclusionEngine`] owns the prepare/resolve/render transclusion logic
//! lifted off `impl Markdown`. It borrows the document being composed and
//! drives directive preparation, concurrent resolution, and content rendering;
//! the pipeline driver applies the resolved spans back onto the document.

use crate::markdown::Markdown;
use crate::markdown::compose::cache;
use crate::markdown::compose::cache::operation::CacheableOperation;
use crate::markdown::compose::context::effective_state::{self as state, EffectiveStateBuilder};
use crate::markdown::compose::{
    ComposeOperation, ComposeOptions, ComposeReport, ComposeSource, ComposeWarning, EffectiveState,
};
use crate::markdown::compose::{
    file_links, indent, remote, remote_fetch, replacement, shell_expansion, toc_linking,
    transclusion,
};
use crate::markdown::normalize::HeadingLevel;
use crate::markdown::span::{line_at_offset, newline_offset_table};
use crate::markdown::types::MarkdownResult;
use pulldown_cmark::{Event, HeadingLevel as PulldownHeadingLevel, Parser, Tag, TagEnd};
use serde_json::{Map, Value};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// Builds a target→resolved-target cache from a preflight graph node's
/// outgoing edges, keyed by the authored transclusion target.
///
/// [`TransclusionEngine::prepare_block_transclusions`] consults this to skip a
/// second [`resolve_target`](transclusion::resolve_target) pass for a directive
/// the preflight walk already resolved. Only the resolved path/URL is reused —
/// never the preflight span — so the replacement range is always re-anchored
/// against the current transclusion-phase content.
pub(crate) fn build_resolution_cache(
    graph: &crate::markdown::compose::preflight::PreflightGraphNode,
) -> HashMap<String, crate::markdown::compose::preflight::PreflightResolvedTarget> {
    graph
        .edges
        .iter()
        .map(|edge| {
            (
                edge.directive.raw_target.clone(),
                edge.resolved_target.clone(),
            )
        })
        .collect()
}

#[derive(Debug, Clone)]
struct HeadingInfo {
    level: HeadingLevel,
    title: String,
    line: usize,
    start: usize,
    end: usize,
}

/// Finds the nearest preceding heading level before a byte offset.
pub fn find_preceding_heading_level(content: &str, offset: usize) -> Option<HeadingLevel> {
    let mut current = None;

    for (event, range) in Parser::new(content).into_offset_iter() {
        if range.start >= offset {
            break;
        }

        if let Event::Start(Tag::Heading { level, .. }) = event {
            current = Some(pulldown_to_heading_level(level));
        }
    }

    current
}

/// Re-levels markdown content and gracefully degrades H6 overflow to bold text.
pub fn relevel_with_overflow(content: &str, target: HeadingLevel) -> (String, Vec<ComposeWarning>) {
    let headings = extract_headings(content);
    if headings.is_empty() {
        return (content.to_string(), Vec::new());
    }

    let root = headings[0].level;
    let adjustment = target.as_u8() as i8 - root.as_u8() as i8;
    if adjustment == 0 {
        return (content.to_string(), Vec::new());
    }

    enum Replacement {
        Prefix {
            start: usize,
            old_level: HeadingLevel,
            new_level: HeadingLevel,
        },
        Overflow {
            start: usize,
            end: usize,
            title: String,
            line: usize,
            new_level_raw: u8,
        },
    }

    let mut replacements = Vec::new();
    let mut warnings = Vec::new();

    for heading in &headings {
        let new_level_raw = heading.level.as_u8() as i8 + adjustment;
        if (1..=6).contains(&new_level_raw) {
            if let Some(level) = HeadingLevel::new(new_level_raw as u8) {
                replacements.push(Replacement::Prefix {
                    start: heading.start,
                    old_level: heading.level,
                    new_level: level,
                });
            }
        } else {
            replacements.push(Replacement::Overflow {
                start: heading.start,
                end: heading.end,
                title: heading.title.clone(),
                line: heading.line,
                new_level_raw: new_level_raw.max(7) as u8,
            });
        }
    }

    // `extract_headings` already yields headings in ascending document order and
    // heading spans never overlap, so the output is assembled in one forward
    // pass: copy the gap before each replacement, then the replacement itself.
    // The previous code rebuilt the entire document once per heading (descending
    // order kept the offsets valid), which made re-leveling quadratic.
    let mut result = String::with_capacity(content.len());
    let mut cursor = 0usize;

    for replacement in &replacements {
        match replacement {
            Replacement::Prefix {
                start,
                old_level,
                new_level,
            } => {
                result.push_str(&content[cursor..*start]);
                for _ in 0..new_level.hash_count() {
                    result.push('#');
                }
                cursor = start + old_level.hash_count();
            }
            Replacement::Overflow {
                start,
                end,
                title,
                line,
                new_level_raw,
            } => {
                result.push_str(&content[cursor..*start]);
                result.push_str("\n\n**");
                result.push_str(title.trim());
                result.push_str("**\n\n");
                cursor = *end;
                warnings.push(
                    ComposeWarning::new(
                        "transclusion",
                        format!(
                            "Heading overflow at line {line}: converted to bold text (would become H{new_level_raw})"
                        ),
                    )
                    .at_line(*line),
                );
            }
        }
    }
    result.push_str(&content[cursor..]);

    // The pre-existing contract emits overflow warnings in reverse document
    // order: the old replacement loop ran descending to keep byte offsets valid
    // and pushed warnings as it went. The forward pass above collects them
    // ascending, so restore the observed order rather than silently changing it.
    warnings.reverse();

    (result, warnings)
}

fn extract_headings(content: &str) -> Vec<HeadingInfo> {
    let mut headings = Vec::new();
    let mut current: Option<(HeadingLevel, String, usize)> = None;

    // Built once on the first heading so each line number is a binary search
    // rather than a fresh `content[..start]` rescan, which made extraction
    // quadratic in document size. `line_at_offset` reproduces
    // `lines().count() + 1` exactly. Deferred rather than eager so a
    // heading-free document — which returns without ever asking for a line —
    // does not pay for the table.
    let mut newline_offsets: Option<Vec<usize>> = None;

    for (event, range) in Parser::new(content).into_offset_iter() {
        match event {
            Event::Start(Tag::Heading { level, .. }) => {
                current = Some((pulldown_to_heading_level(level), String::new(), range.start));
            }
            Event::Text(text) | Event::Code(text) => {
                if let Some((_, title, _)) = current.as_mut() {
                    title.push_str(&text);
                }
            }
            Event::End(TagEnd::Heading(_)) => {
                if let Some((level, title, start)) = current.take() {
                    let offsets =
                        newline_offsets.get_or_insert_with(|| newline_offset_table(content));
                    let line = line_at_offset(offsets, content, start);
                    headings.push(HeadingInfo {
                        level,
                        title,
                        line,
                        start,
                        end: range.end,
                    });
                }
            }
            _ => {}
        }
    }

    headings
}

fn pulldown_to_heading_level(level: PulldownHeadingLevel) -> HeadingLevel {
    match level {
        PulldownHeadingLevel::H1 => HeadingLevel::H1,
        PulldownHeadingLevel::H2 => HeadingLevel::H2,
        PulldownHeadingLevel::H3 => HeadingLevel::H3,
        PulldownHeadingLevel::H4 => HeadingLevel::H4,
        PulldownHeadingLevel::H5 => HeadingLevel::H5,
        PulldownHeadingLevel::H6 => HeadingLevel::H6,
    }
}

#[derive(Clone)]
pub(crate) enum PreparedTransclusion {
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
    RemoteFile {
        order: usize,
        target: ApplyTarget,
        url: url::Url,
        directive_options: transclusion::BlockOptions,
        insertion_context: Option<(usize, usize)>,
    },
    RemoteCode {
        order: usize,
        span: std::ops::Range<usize>,
        url: url::Url,
        directive_options: transclusion::BlockOptions,
        language: String,
    },
    Toc {
        order: usize,
        span: std::ops::Range<usize>,
        directive: toc_linking::TocLinkingDirective,
    },
    FileLinks {
        order: usize,
        span: std::ops::Range<usize>,
        /// The parsed directive. Discovery is deferred to the concurrent
        /// resolve stage so multiple directives' filesystem walks run in
        /// parallel, and each directive's tree is built from its discovered
        /// entries rather than walked a second time for rendering.
        directive: file_links::FileLinksDirective,
    },
}

/// Where a failed resolution's notice belongs, and what it should say.
///
/// Captured before the prepared value is consumed, because a resolution error
/// carries no span of its own. Without it the apply loop can only drop the
/// result, and dropping a result is not the same as emitting nothing for its
/// span: the authored `::file …` line survives into the composed document and
/// renders as a literal paragraph of directive syntax.
pub(crate) struct FailureAnchor {
    pub(crate) order: usize,
    pub(crate) target: ApplyTarget,
    pub(crate) notice: String,
}

impl PreparedTransclusion {
    /// The anchor and notice to substitute if resolving this item fails.
    ///
    /// The name goes in a code span deliberately. CommonMark processes
    /// backslash escapes in ordinary text, and a Windows path reaches this
    /// point often enough that an unquoted name would be silently mangled.
    pub(crate) fn failure_anchor(&self) -> FailureAnchor {
        let named = |path: &PathBuf| {
            let name = path
                .file_name()
                .map(|name| name.to_string_lossy().into_owned())
                .unwrap_or_else(|| path.to_string_lossy().into_owned());
            format!("_Could not transclude `{name}`_")
        };

        let (order, target, notice) = match self {
            Self::FixedReplace { order, span, .. } => (
                *order,
                ApplyTarget::Replace(span.clone()),
                "_Could not transclude_".to_string(),
            ),
            Self::FixedSection { order, slot, .. } => (
                *order,
                ApplyTarget::Section(*slot),
                "_Could not transclude_".to_string(),
            ),
            Self::Markdown {
                order, target, path, ..
            } => (*order, target.clone(), named(path)),
            Self::Code {
                order, span, path, ..
            } => (*order, ApplyTarget::Replace(span.clone()), named(path)),
            Self::RemoteFile {
                order, target, url, ..
            } => (
                *order,
                target.clone(),
                format!("_Could not transclude `{url}`_"),
            ),
            Self::RemoteCode {
                order, span, url, ..
            } => (
                *order,
                ApplyTarget::Replace(span.clone()),
                format!("_Could not transclude `{url}`_"),
            ),
            Self::Toc {
                order,
                span,
                directive,
            } => {
                // `targets` is a fallback chain; the first entry is what the
                // author wrote and the one the reader can act on.
                let notice = match directive.targets.first() {
                    Some(target) => format!("_Could not link headings from `{target}`_"),
                    None => "_Could not link headings_".to_string(),
                };
                (*order, ApplyTarget::Replace(span.clone()), notice)
            }
            Self::FileLinks { order, span, .. } => (
                *order,
                ApplyTarget::Replace(span.clone()),
                "_Could not build the file listing_".to_string(),
            ),
        };

        FailureAnchor {
            order,
            target,
            notice,
        }
    }
}

#[derive(Clone, Copy)]
pub(crate) enum SectionSlot {
    Prologue(usize),
    Epilogue(usize),
}

#[derive(Clone)]
pub(crate) enum ApplyTarget {
    Replace(std::ops::Range<usize>),
    Section(SectionSlot),
}

pub(crate) struct ResolvedTransclusion {
    pub(crate) order: usize,
    pub(crate) target: ApplyTarget,
    pub(crate) content: Option<String>,
    pub(crate) report: ComposeReport,
    /// Source file for file-based transclusions (used for source map).
    pub(crate) source_file: Option<PathBuf>,
}

/// Drives transclusion preparation, resolution, and rendering for one document.
///
/// Holds a borrow of the document being composed; the prepare/resolve/render
/// methods were lifted verbatim off `impl Markdown` so behavior is unchanged.
pub(crate) struct TransclusionEngine<'a> {
    markdown: &'a Markdown,
    /// Ascending `(heading_start_offset, level)` table over the parent body,
    /// parsed once and shared across the parallel resolve stage (F15). The
    /// body is immutable for the engine's lifetime, so this replaces the
    /// per-directive full re-parse in `find_preceding_heading_level`.
    heading_starts: std::sync::OnceLock<Vec<(usize, HeadingLevel)>>,
}

impl<'a> TransclusionEngine<'a> {
    pub(crate) fn new(markdown: &'a Markdown) -> Self {
        Self {
            markdown,
            heading_starts: std::sync::OnceLock::new(),
        }
    }

    /// Nearest preceding heading level before `offset`, using the memoized
    /// heading table. Byte-identical to [`find_preceding_heading_level`]: both
    /// return the last heading whose start is strictly before `offset`.
    fn preceding_heading_level(&self, offset: usize) -> Option<HeadingLevel> {
        let table = self.heading_starts.get_or_init(|| {
            let mut table = Vec::new();
            for (event, range) in Parser::new(&self.markdown.content).into_offset_iter() {
                if let Event::Start(Tag::Heading { level, .. }) = event {
                    table.push((range.start, pulldown_to_heading_level(level)));
                }
            }
            table
        });
        let idx = table.partition_point(|(start, _)| *start < offset);
        (idx > 0).then(|| table[idx - 1].1)
    }

    /// Records a fetched remote URL body as a closure-hash dependency.
    ///
    /// The dependency's `closure_hash` is the xxHash of the response body, so a
    /// changed remote document invalidates any parent artifact transcluding it.
    /// No-op when the URL's content hash is unavailable (unregistered or failed).
    fn record_remote_dependency(
        runtime_mutex: &std::sync::Mutex<&mut shell_expansion::types::PipelineRuntime>,
        remote_fetch: &remote_fetch::RemoteFetchRuntime,
        url: &url::Url,
    ) {
        if let Some(content_hash) = remote_fetch.content_hash(url) {
            let sid = cache::hashing::source_id_hash(url.as_str());
            let dependency = cache::types::DependencyRef {
                artifact_class: cache::types::ArtifactClass::RemoteUrl,
                entry_key: sid,
                source_id_hash: sid,
                closure_hash: content_hash,
            };
            runtime_mutex.lock().unwrap().record_dependency(dependency);
        }
    }

    /// Prepares `::file` / `::url` / `::code` transclusions from directives
    /// **parsed against the current content**.
    ///
    /// `resolved_cache` is an optional preflight resolution cache keyed by
    /// normalized target. On a hit the engine skips
    /// [`transclusion::resolve_target`] and reuses the preflight-resolved
    /// path/URL — but never the preflight span. Spans always come from
    /// `directives`, which are parsed from the live transclusion-phase
    /// content, because earlier inline-pre stages (frontmatter shell
    /// expansion, interpolation, …) can shift byte offsets after the
    /// condition-blind preflight walk. A miss (target changed since
    /// preflight, or no cache) resolves the target normally.
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn prepare_block_transclusions(
        &self,
        directives: &[transclusion::BlockDirective],
        kind: transclusion::DirectiveKind,
        state: &EffectiveState,
        options: &ComposeOptions,
        remote_fetch: &remote_fetch::RemoteFetchRuntime,
        report: &mut ComposeReport,
        prepared: &mut Vec<PreparedTransclusion>,
        next_order: &mut usize,
        resolved_cache: Option<
            &HashMap<String, crate::markdown::compose::preflight::PreflightResolvedTarget>,
        >,
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
                    transclusion::DeferredSetError::InvalidAssignment { raw, reason, line } => {
                        if options.allow_invalid_frontmatter_assignment {
                            report.add_warning(
                                ComposeWarning::new(
                                    "transclusion",
                                    format!(
                                        "Invalid frontmatter assignment on ::{} directive at line {}: {} (value: {})",
                                        directive.kind.as_str(),
                                        line,
                                        reason,
                                        raw
                                    ),
                                )
                                .at_line(*line),
                            );
                        } else {
                            return Err(
                                transclusion::TransclusionError::InvalidFrontmatterAssignment {
                                    ctx: Box::new(self.markdown.source_context_for_errors()),
                                    line: *line,
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
                                    ctx: Box::new(self.markdown.source_context_for_errors()),
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
                let lookup = state::ResolvingLookup::new(
                    state,
                    options.expression_resolution_context(remote_fetch),
                );
                for warning in
                    crate::markdown::compose::conditions::collect_condition_context_warnings(
                        expr, &lookup, "condition",
                    )
                {
                    report.add_warning(warning.at_line(directive.line));
                }
                let should_include = transclusion::evaluate_condition(
                    expr,
                    &lookup,
                    directive.line,
                    self.markdown.source_context_for_errors(),
                )?;
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

            let target = &directive.raw_target;
            let cached = resolved_cache.and_then(|cache| cache.get(target));
            let resolved = if let Some(cached) = cached {
                // Reuse the preflight-resolved target, skipping a second
                // `resolve_target` pass. The span still rides on `directive`
                // (parsed from current content), so the preflight span drift
                // that motivated this cache cannot corrupt the replacement
                // range. The `id` is unused at prepare time (it is discarded
                // by every match arm below), so an empty id is sound here.
                match cached {
                    crate::markdown::compose::preflight::PreflightResolvedTarget::File(path) => {
                        transclusion::ResolvedTarget::File {
                            path: path.clone(),
                            id: String::new(),
                        }
                    }
                    crate::markdown::compose::preflight::PreflightResolvedTarget::Url(url) => {
                        transclusion::ResolvedTarget::Url {
                            url: url.clone(),
                            id: String::new(),
                        }
                    }
                }
            } else {
                match transclusion::resolve_target(
                    directive.kind,
                    target,
                    &transclusion_opts,
                    &options.source,
                    directive.line,
                    self.markdown.source_context_for_errors(),
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
                }
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
                transclusion::ResolvedTarget::Url { url, .. }
                    if options.allow_remote_transclusion =>
                {
                    // The eager pre-scan only sees URLs present in the original
                    // content. A directive whose URL was produced by an earlier
                    // compose phase (interpolation, replacement) reaches here
                    // unregistered, so register it now to start its fetch and
                    // keep point-of-use from failing with "not registered".
                    remote_fetch.register_nested(url.clone());
                    if directive.kind == transclusion::DirectiveKind::Code {
                        let language = transclusion::infer_language(
                            std::path::Path::new(url.path()),
                            &options.code_fallback_language,
                        );
                        prepared.push(PreparedTransclusion::RemoteCode {
                            order: *next_order,
                            span: directive.span.clone(),
                            url,
                            directive_options: directive.options.clone(),
                            language,
                        });
                    } else {
                        prepared.push(PreparedTransclusion::RemoteFile {
                            order: *next_order,
                            target: ApplyTarget::Replace(directive.span.clone()),
                            url,
                            directive_options: directive.options.clone(),
                            insertion_context: Some((directive.span.start, directive.line)),
                        });
                    }
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

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn prepare_frontmatter_transclusions(
        &self,
        refs: &transclusion::FrontmatterRefs,
        _state: &EffectiveState,
        options: &ComposeOptions,
        remote_fetch: &remote_fetch::RemoteFetchRuntime,
        _report: &mut ComposeReport,
        prepared: &mut Vec<PreparedTransclusion>,
        next_order: &mut usize,
    ) -> MarkdownResult<()> {
        for (index, reference) in refs.prologue.iter().enumerate() {
            self.prepare_frontmatter_reference(
                reference,
                SectionSlot::Prologue(index),
                options,
                remote_fetch,
                prepared,
                next_order,
            )?;
        }

        for (index, reference) in refs.epilogue.iter().enumerate() {
            self.prepare_frontmatter_reference(
                reference,
                SectionSlot::Epilogue(index),
                options,
                remote_fetch,
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
        remote_fetch: &remote_fetch::RemoteFetchRuntime,
        prepared: &mut Vec<PreparedTransclusion>,
        next_order: &mut usize,
    ) -> MarkdownResult<()> {
        let file_ref = match transclusion::classify_frontmatter_reference(reference) {
            transclusion::FrontmatterReference::Inline => {
                prepared.push(PreparedTransclusion::FixedSection {
                    order: *next_order,
                    slot,
                    content: Some(reference.to_string()),
                    report: ComposeReport::new(),
                });
                *next_order += 1;
                return Ok(());
            }
            transclusion::FrontmatterReference::Parsed(file_ref) => file_ref,
            transclusion::FrontmatterReference::ParseError(error) => {
                return Err(transclusion::TransclusionError::from(error).into());
            }
        };

        let ignore_invalid = self.resolve_ignore_invalid(options);
        let transclusion_opts = options.transclusion_options();

        let resolved = match transclusion::resolve_parsed_target(
            transclusion::DirectiveKind::File,
            &file_ref,
            &transclusion_opts,
            &options.source,
            0,
            self.markdown.source_context_for_errors(),
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
            transclusion::ResolvedTarget::Url { url, .. }
                if options.allow_remote_transclusion =>
            {
                // Frontmatter `prologue`/`epilogue` URLs are not seen by the
                // eager pre-scan (it only covers directives and expression
                // arguments), so register the slot here. Without this,
                // `PreparedTransclusion::RemoteFile` fails at point-of-use with
                // "URL was not registered for fetching" — matching the
                // directive path's register-on-discovery behavior.
                remote_fetch.register_nested(url.clone());
                prepared.push(PreparedTransclusion::RemoteFile {
                    order: *next_order,
                    target: ApplyTarget::Section(slot),
                    url,
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

    pub(crate) fn prepare_toc_transclusions(
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

    pub(crate) fn prepare_file_links_transclusions(
        &self,
        directives: &[file_links::FileLinksDirective],
        prepared: &mut Vec<PreparedTransclusion>,
        next_order: &mut usize,
    ) {
        // Discovery is intentionally NOT performed here: it runs in the
        // concurrent resolve stage (see `resolve_file_links_transclusion`) so
        // multiple directives' expensive filesystem walks parallelize. This
        // loop only enqueues the parsed directive.
        for directive in directives {
            prepared.push(PreparedTransclusion::FileLinks {
                order: *next_order,
                span: directive.span.clone(),
                directive: directive.clone(),
            });
            *next_order += 1;
        }
    }

    pub(crate) fn resolve_prepared_transclusion(
        &self,
        item: PreparedTransclusion,
        state: &EffectiveState,
        state_identity: cache::hashing::PhaseStateIdentity,
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
                    state_identity,
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
            PreparedTransclusion::RemoteFile {
                order,
                target,
                url,
                directive_options,
                insertion_context,
            } => {
                let remote_fetch = {
                    let runtime = runtime_mutex.lock().unwrap();
                    runtime.remote_fetch.clone()
                };
                let body_text = remote_fetch
                    .get_content(&url)
                    .map_err(|e| {
                        transclusion::TransclusionError::RemoteFetchFailed {
                            url: url.to_string(),
                            reason: e,
                        }
                    })?
                    .ok_or_else(|| transclusion::TransclusionError::RemoteFetchFailed {
                        url: url.to_string(),
                        reason: "URL was not registered for fetching".to_string(),
                    })?;

                Self::record_remote_dependency(runtime_mutex, &remote_fetch, &url);

                // Parse the fetched body as Markdown and recursively compose it.
                let mut child =
                    crate::markdown::Markdown::try_from_content(body_text).map_err(|e| {
                        crate::markdown::types::MarkdownError::Transform(format!(
                            "failed to parse fetched Markdown from '{}': {e}",
                            url
                        ))
                    })?;

                let child_source = ComposeSource::Url(url.clone());

                // Eagerly register any remote URLs the fetched document itself
                // references, so the child pipeline's point-of-use waits land on
                // an already-in-flight slot rather than an unregistered one.
                // Mirror the root pipeline's op-scoping: directive URLs follow
                // block transclusion, expression URLs follow interpolation.
                if options.allow_remote_transclusion {
                    let mut child_catalog = remote::RemoteUrlCatalog::new();

                    if options.is_enabled(ComposeOperation::BlockTransclusion) {
                        let child_directives = transclusion::parse_directives(
                            child.content(),
                            child.source_context_for_errors(),
                        )
                        .unwrap_or_default();
                        for entry in remote::discover_remote_urls_from_directives(
                            &child_directives,
                            &child_source,
                        ) {
                            child_catalog.add(entry);
                        }
                    }

                    if options.is_enabled(ComposeOperation::Interpolation) {
                        for entry in remote::discover_remote_urls_from_expressions(
                            child.content(),
                            &child_source,
                        ) {
                            child_catalog.add(entry);
                        }
                    }

                    for nested in child_catalog.urls() {
                        remote_fetch.register_nested(nested);
                    }
                }

                let mut child_runtime = {
                    let runtime = runtime_mutex.lock().unwrap();
                    runtime.clone_for_child()
                };
                let mut child_options = options.clone();
                child_options.source = child_source;
                // Recursive graph reuse for remote children: hand the child its
                // OWN preflight sub-node (whose edges point at grandchildren) so
                // its transclusion stage reuses grandchild URL/path resolution
                // too. Mirrors the local-file path; a miss falls back to None.
                child_options.preflight_graph = options
                    .preflight_graph()
                    .and_then(|graph| graph.child_for_url(&url).cloned());

                let child_report = child
                    .run_compose_pipeline_internal(child_options, &mut child_runtime)?;
                {
                    let mut runtime = runtime_mutex.lock().unwrap();
                    runtime.merge_child(&child_runtime);
                }

                let mut content = child.content().to_string();
                let mut merged_report = child_report;
                merged_report.transclusions_applied += 1;

                if let Some((offset, line)) = insertion_context
                    && let Some(parent_level) = self.preceding_heading_level(offset)
                {
                    let target_level =
                        HeadingLevel::new((parent_level.as_u8() + 1).min(6))
                            .unwrap_or(HeadingLevel::H6);
                    let (releveled, warnings) =
                        transclusion::relevel_with_overflow(&content, target_level);
                    content = releveled;
                    for warning in warnings {
                        merged_report.add_warning(warning.at_line(line));
                    }
                }

                let result = self.apply_wrappers(content, &directive_options);
                Ok(ResolvedTransclusion {
                    order,
                    target,
                    content: Some(result),
                    report: merged_report,
                    source_file: None,
                })
            }
            PreparedTransclusion::RemoteCode {
                order,
                span,
                url,
                directive_options,
                language,
            } => {
                let remote_fetch = {
                    let runtime = runtime_mutex.lock().unwrap();
                    runtime.remote_fetch.clone()
                };
                let body_text = remote_fetch
                    .get_content(&url)
                    .map_err(|e| {
                        transclusion::TransclusionError::RemoteFetchFailed {
                            url: url.to_string(),
                            reason: e,
                        }
                    })?
                    .ok_or_else(|| transclusion::TransclusionError::RemoteFetchFailed {
                        url: url.to_string(),
                        reason: "URL was not registered for fetching".to_string(),
                    })?;

                Self::record_remote_dependency(runtime_mutex, &remote_fetch, &url);

                let fenced = transclusion::wrap_in_code_block(&body_text, &language);
                let spaced = transclusion::ensure_vertical_spacing(&fenced);
                let result = self.apply_wrappers(spaced, &directive_options);
                let mut code_report = ComposeReport::new();
                code_report.transclusions_applied = 1;
                Ok(ResolvedTransclusion {
                    order,
                    target: ApplyTarget::Replace(span),
                    content: Some(result),
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
                        self.markdown.source_context_for_errors(),
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
                            // Render with no indentation so the cache entry is
                            // indent-independent. Indentation is caller-local
                            // and is applied below after cache lookup.
                            let content = toc_linking::render_resolved_directive(
                                &display_clone,
                                &headings,
                                &options_clone,
                                line,
                                "",
                                None,
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

                    toc_linking::indent_text(
                        &cached.content,
                        &directive.indent,
                        directive.inferred_indent.as_deref(),
                    )
                } else {
                    let empty_text = directive.options.empty_text.clone().unwrap_or_default();
                    toc_linking::indent_text(
                        &empty_text,
                        &directive.indent,
                        directive.inferred_indent.as_deref(),
                    )
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
            PreparedTransclusion::FileLinks {
                order,
                span,
                directive,
            } => self.resolve_file_links_transclusion(order, span, directive, options),
        }
    }

    /// Resolves a single `::file-links` directive in the concurrent stage.
    ///
    /// Discovery runs here (not during preparation) so directives parallelize.
    /// On a match the [`FileSystem`](biscuit_terminal::components::filesystem::FileSystem)
    /// tree is built directly from the discovered entries — no second
    /// filesystem walk — and its fully-styled render subtree is embedded
    /// losslessly via [`renderable::tree::embed`]. Empty and invalid results
    /// reproduce the strict/permissive behavior the preparation stage used to
    /// apply.
    fn resolve_file_links_transclusion(
        &self,
        order: usize,
        span: std::ops::Range<usize>,
        directive: file_links::FileLinksDirective,
        options: &ComposeOptions,
    ) -> MarkdownResult<ResolvedTransclusion> {
        let skipped_replace = |replacement: String, report: ComposeReport| {
            Ok(ResolvedTransclusion {
                order,
                target: ApplyTarget::Replace(span.clone()),
                content: Some(replacement),
                report,
                source_file: None,
            })
        };

        let render = match file_links::discover(&directive, &options.source) {
            Ok(result) => match result.render {
                Some(render) => render,
                None => {
                    // Empty result: strict mode inserts a subtle notice,
                    // permissive mode removes the directive with a warning.
                    let mut report = ComposeReport::new();
                    report.transclusions_skipped = 1;
                    if options.fail_fast {
                        return skipped_replace("_No matching files_".to_string(), report);
                    }
                    report.add_warning(
                        ComposeWarning::new(
                            "file_links",
                            format!(
                                "No matching files for ::file-links directive at line {}",
                                directive.line
                            ),
                        )
                        .at_line(directive.line),
                    );
                    return skipped_replace(String::new(), report);
                }
            },
            Err(err) => {
                if self.resolve_ignore_invalid(options) {
                    let mut report = ComposeReport::new();
                    report.transclusions_skipped = 1;
                    report.add_warning(
                        ComposeWarning::new("file_links", err.to_string()).at_line(directive.line),
                    );
                    return skipped_replace(String::new(), report);
                }
                return Err(err.into());
            }
        };

        // Build the FileSystem tree directly from the discovered entries and
        // inject it, so the component renders without re-walking the directory.
        let tree = file_links::build_included_tree(&render);
        let mut fs =
            biscuit_terminal::components::filesystem::FileSystem::new(&render.component_root)
                .map_err(|e| {
                    crate::markdown::types::MarkdownError::Transform(format!(
                        "failed to create FileSystem for ::file-links at line {}: {e}",
                        directive.line
                    ))
                })?;
        fs = fs
            .with_prebuilt_tree(tree)
            .with_file_links()
            .italicize_dot_files(true)
            .dim_gitignore(true)
            .show_root(true);
        if !render.dimmed_prefix.is_empty() {
            fs = fs.with_dimmed_root_prefix(&render.dimmed_prefix);
        }
        if !render.target_name.is_empty() {
            fs = fs.with_root_display_name(&render.target_name);
        }
        if render.uses_repo_icon {
            fs = fs.with_root_icon(
                biscuit_terminal::components::filesystem::RootIconKind::Repository,
            );
        }

        // Carry the fully-styled render subtree through the composed document
        // losslessly: the fold splices it back so terminal and browser
        // rendering reproduce the live component (color, dim, icons), while
        // plain-Markdown consumers see the embedded portable fallback.
        use renderable::tree::{TreeRenderable, encode_embedded_subtree};
        let node = fs.render_tree();
        let embedded = encode_embedded_subtree(&node).map_err(|e| {
            crate::markdown::types::MarkdownError::Transform(format!(
                "failed to embed ::file-links render tree at line {}: {e}",
                directive.line
            ))
        })?;
        let replacement = indent::indent_text(
            &embedded,
            &directive.indent,
            directive.inferred_indent.as_deref(),
        );

        let mut report = ComposeReport::new();
        report.transclusions_applied = 1;
        Ok(ResolvedTransclusion {
            order,
            target: ApplyTarget::Replace(span),
            content: Some(replacement),
            report,
            source_file: None,
        })
    }

    // NOTE: `::file` and `::code` directives share the same indentation
    // preservation bug as `::toc-linking` (see spec.md for 2026-05-07).
    // Unlike `::toc-linking`, the fix is not trivially co-located here:
    // `PreparedTransclusion::Markdown` and `PreparedTransclusion::Code`
    // do not capture directive indentation, and the underlying
    // `transclusion::Directive` struct lacks indent fields. Fixing this
    // would require structural changes across the transclusion pipeline.
    // Tracked as part of the same feature but deferred to a follow-up.
    #[allow(clippy::too_many_arguments)]
    fn render_markdown_transclusion(
        &self,
        path: &Path,
        insertion_context: Option<(usize, usize)>,
        directive_options: &transclusion::BlockOptions,
        state: &EffectiveState,
        state_identity: cache::hashing::PhaseStateIdentity,
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
        // 35.1: the state and context hashes are phase-wide (identical for every
        // directive), captured once by the caller and threaded in here.
        let persistent_ctx = cache::PersistentContext {
            source_id: cache::hashing::source_id_hash(&cache::compose_cache_key_for_path(path)),
            state_hash: state_identity.state_hash,
            context_hash: state_identity.context_hash,
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
            options.persistent_cache_eligible(),
            || {
                let mut child_options = options
                    .clone()
                    .with_replace_parent_wins(replace_parent_wins)
                    .with_one_off_replace(one_off.clone());
                child_options.external_state = Some(inherited.clone());
                child_options = child_options.with_accepted_source_file(path_buf.clone());
                // Recursive graph reuse: hand the child its OWN preflight
                // sub-node (whose edges point at grandchildren), so the child's
                // transclusion stage reuses grandchild target resolution too.
                // Never the parent graph — its edges point back at this child,
                // so its resolution cache would be keyed by the wrong
                // (parent-level) targets. A miss (no graph attached, or this
                // child was not in the preflight walk) falls back to None.
                child_options.preflight_graph = options
                    .preflight_graph()
                    .and_then(|graph| graph.child_for_source(&path_buf).cloned());

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
            && let Some(parent_level) = self.preceding_heading_level(offset)
        {
            let target_level =
                HeadingLevel::new((parent_level.as_u8() + 1).min(6))
                    .unwrap_or(HeadingLevel::H6);
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

    pub(crate) fn apply_wrappers(
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
            let summary = if summary.is_empty() || summary.eq_ignore_ascii_case("true") {
                "Details"
            } else {
                summary.as_str()
            };
            let body = content.trim_end_matches('\n');
            content = format!("::disclosure\n{summary}\n::details\n{body}\n::end-disclosure");
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

        if let Ok(Some(value)) = self.markdown.fm_get::<bool>("ignore_invalid") {
            return value;
        }

        options
            .context()
            .env()
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
    use super::*;

    #[test]
    fn finds_preceding_heading_level() {
        let content = "# Root\n\nText\n\n## Child\n\n::file ./x.md\n";
        let offset = content.find("::file").unwrap();
        assert_eq!(
            find_preceding_heading_level(content, offset),
            Some(HeadingLevel::H2)
        );
    }

    #[test]
    fn overflow_headings_become_bold() {
        let content = "## Section\n\n### Deep\n";
        let (new_content, warnings) = relevel_with_overflow(content, HeadingLevel::H6);

        assert!(new_content.contains("###### Section"));
        assert!(new_content.contains("**Deep**"));
        assert_eq!(warnings.len(), 1);
    }

    /// Finding 35.2 regression coverage.
    ///
    /// The optimization replaced a per-heading whole-document rebuild and a
    /// per-heading `content[..start]` rescan with one forward output pass and one
    /// newline-offset table. Both are pure performance changes, so the contract
    /// is exact equality with the pre-change algorithm — proven differentially
    /// against an oracle rather than by re-asserting hand-picked substrings.
    mod finding_35_2 {
        use super::*;

        /// Byte-for-byte reimplementation of the pre-optimization
        /// `relevel_with_overflow`: per-heading line numbers via
        /// `content[..start].lines().count() + 1`, replacements applied
        /// descending with a full-document rebuild per heading, and warnings
        /// pushed in that descending order.
        fn naive_relevel(content: &str, target: HeadingLevel) -> (String, Vec<ComposeWarning>) {
            #[derive(Clone)]
            enum Replacement {
                Prefix {
                    start: usize,
                    old_level: HeadingLevel,
                    new_level: HeadingLevel,
                },
                Overflow {
                    start: usize,
                    end: usize,
                    title: String,
                    line: usize,
                    new_level_raw: u8,
                },
            }

            let headings = naive_extract_headings(content);
            if headings.is_empty() {
                return (content.to_string(), Vec::new());
            }

            let root = headings[0].level;
            let adjustment = target.as_u8() as i8 - root.as_u8() as i8;
            if adjustment == 0 {
                return (content.to_string(), Vec::new());
            }

            let mut replacements = Vec::new();
            let mut warnings = Vec::new();

            for heading in &headings {
                let new_level_raw = heading.level.as_u8() as i8 + adjustment;
                if (1..=6).contains(&new_level_raw) {
                    if let Some(level) = HeadingLevel::new(new_level_raw as u8) {
                        replacements.push(Replacement::Prefix {
                            start: heading.start,
                            old_level: heading.level,
                            new_level: level,
                        });
                    }
                } else {
                    replacements.push(Replacement::Overflow {
                        start: heading.start,
                        end: heading.end,
                        title: heading.title.clone(),
                        line: heading.line,
                        new_level_raw: new_level_raw.max(7) as u8,
                    });
                }
            }

            replacements.sort_by(|left, right| {
                let left_start = match left {
                    Replacement::Prefix { start, .. } | Replacement::Overflow { start, .. } => {
                        *start
                    }
                };
                let right_start = match right {
                    Replacement::Prefix { start, .. } | Replacement::Overflow { start, .. } => {
                        *start
                    }
                };
                right_start.cmp(&left_start)
            });

            let mut result = content.to_string();

            for replacement in replacements {
                match replacement {
                    Replacement::Prefix {
                        start,
                        old_level,
                        new_level,
                    } => {
                        let prefix_end = start + old_level.hash_count();
                        let replacement = "#".repeat(new_level.hash_count());
                        result =
                            format!("{}{}{}", &result[..start], replacement, &result[prefix_end..]);
                    }
                    Replacement::Overflow {
                        start,
                        end,
                        title,
                        line,
                        new_level_raw,
                    } => {
                        let bold_block = format!("\n\n**{}**\n\n", title.trim());
                        result = format!("{}{}{}", &result[..start], bold_block, &result[end..]);
                        warnings.push(
                            ComposeWarning::new(
                                "transclusion",
                                format!(
                                    "Heading overflow at line {line}: converted to bold text (would become H{new_level_raw})"
                                ),
                            )
                            .at_line(line),
                        );
                    }
                }
            }

            (result, warnings)
        }

        /// Pre-optimization heading extraction: the line number is a fresh
        /// `lines().count()` over the growing prefix.
        fn naive_extract_headings(content: &str) -> Vec<HeadingInfo> {
            let mut headings = Vec::new();
            let mut current: Option<(HeadingLevel, String, usize)> = None;

            for (event, range) in Parser::new(content).into_offset_iter() {
                match event {
                    Event::Start(Tag::Heading { level, .. }) => {
                        current =
                            Some((pulldown_to_heading_level(level), String::new(), range.start));
                    }
                    Event::Text(text) | Event::Code(text) => {
                        if let Some((_, title, _)) = current.as_mut() {
                            title.push_str(&text);
                        }
                    }
                    Event::End(TagEnd::Heading(_)) => {
                        if let Some((level, title, start)) = current.take() {
                            let line = content[..start].lines().count() + 1;
                            headings.push(HeadingInfo {
                                level,
                                title,
                                line,
                                start,
                                end: range.end,
                            });
                        }
                    }
                    _ => {}
                }
            }

            headings
        }

        fn all_levels() -> [HeadingLevel; 6] {
            [
                HeadingLevel::H1,
                HeadingLevel::H2,
                HeadingLevel::H3,
                HeadingLevel::H4,
                HeadingLevel::H5,
                HeadingLevel::H6,
            ]
        }

        fn assert_matches_oracle(label: &str, content: &str) {
            for target in all_levels() {
                let (actual_text, actual_warnings) = relevel_with_overflow(content, target);
                let (expected_text, expected_warnings) = naive_relevel(content, target);

                assert_eq!(
                    actual_text, expected_text,
                    "{label}: releveled text differs from the pre-optimization output at target {target:?}"
                );
                assert_eq!(
                    actual_warnings, expected_warnings,
                    "{label}: overflow warnings (content, line, and order) differ at target {target:?}"
                );
            }
        }

        /// Content shapes that exercise line counting, offsets, and the
        /// replacement branches. Each is checked at all six target levels, so
        /// every case covers the prefix branch, the overflow branch, and the
        /// zero-adjustment fast path.
        fn cases() -> Vec<(&'static str, String)> {
            vec![
                ("single h1", "# Only\n".to_string()),
                ("no trailing newline", "# Only".to_string()),
                ("no headings", "Just prose.\n\nMore prose.\n".to_string()),
                ("empty document", String::new()),
                (
                    "descending levels",
                    "# Root\n\nText\n\n## Two\n\n### Three\n\n#### Four\n\n##### Five\n\n###### Six\n"
                        .to_string(),
                ),
                (
                    "deep root forces overflow",
                    "##### Deep\n\n###### Deeper\n\ntail\n".to_string(),
                ),
                (
                    "consecutive headings",
                    "# A\n## B\n### C\n".to_string(),
                ),
                (
                    "blank line runs",
                    "# A\n\n\n\n## B\n\n\n\n### C\n".to_string(),
                ),
                (
                    "crlf newlines",
                    "# A\r\n\r\n## B\r\n\r\ntext\r\n\r\n### C\r\n".to_string(),
                ),
                (
                    "multibyte prose before headings",
                    format!("{}\n\n# Ünïcödé Rüt\n\n## Naïve — Sección\n\ntail\n", "é".repeat(200)),
                ),
                (
                    "heading with inline code",
                    "# Root\n\n## Uses `code` here\n".to_string(),
                ),
                (
                    "heading inside blockquote",
                    "# Root\n\n> ## Quoted\n\ntail\n".to_string(),
                ),
                (
                    "fenced block containing hashes",
                    "# Root\n\n```md\n# Not a heading\n```\n\n## Real\n".to_string(),
                ),
                (
                    "setext headings",
                    "Root\n====\n\nSub\n---\n\ntail\n".to_string(),
                ),
                (
                    "whitespace-padded titles",
                    "#   Padded Root   \n\n##\tTabbed\n".to_string(),
                ),
                (
                    "many headings",
                    (0..60)
                        .map(|i| format!("## Section {i}\n\nProse for section {i}.\n\n"))
                        .collect::<String>(),
                ),
            ]
        }

        #[test]
        fn relevel_output_matches_the_pre_optimization_algorithm() {
            for (label, content) in cases() {
                assert_matches_oracle(label, &content);
            }
        }

        /// The committed benchmark fixtures are the exact bytes the Phase-10
        /// measurement runs against, so they are also the passive corpus for the
        /// equivalence claim.
        #[test]
        fn relevel_output_matches_the_oracle_across_shipped_fixtures() {
            let dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("../features/2026-07-15-performance-followup/benchmarks/fixtures");

            let mut checked = 0;
            for entry in std::fs::read_dir(&dir).expect("fixture directory readable") {
                let path = entry.expect("readable dir entry").path();
                if path.extension().and_then(|e| e.to_str()) != Some("md") {
                    continue;
                }
                let content = std::fs::read_to_string(&path).expect("fixture readable");
                assert_matches_oracle(&path.display().to_string(), &content);
                checked += 1;
            }

            assert!(
                checked >= 13,
                "expected the shipped fixture corpus, found only {checked} markdown fixtures"
            );
        }

        /// Pins the observed warning order explicitly. The forward output pass
        /// collects overflow warnings ascending, so the reverse-document-order
        /// contract of the descending rebuild has to be restored deliberately —
        /// an assertion on `len()` alone would not catch losing it.
        #[test]
        fn overflow_warnings_stay_in_reverse_document_order() {
            let content = "### First\n\n#### Second\n\n##### Third\n";
            let (_, warnings) = relevel_with_overflow(content, HeadingLevel::H6);

            let lines: Vec<Option<usize>> = warnings.iter().map(|w| w.line_number).collect();
            assert_eq!(
                lines,
                vec![Some(5), Some(3)],
                "overflow warnings must stay in reverse document order"
            );
            assert!(
                warnings[0].message.contains("would become H8"),
                "unexpected first warning: {}",
                warnings[0].message
            );
            assert!(
                warnings[1].message.contains("would become H7"),
                "unexpected second warning: {}",
                warnings[1].message
            );
        }

        /// The overflow warning's line must be the heading's own source line, not
        /// a position derived from the rewritten output.
        #[test]
        fn overflow_warning_lines_track_source_lines_after_multibyte_prose() {
            let prose = "é".repeat(300);
            let content = format!("### Root\n\n{prose}\n\n#### Overflowing\n");
            let (_, warnings) = relevel_with_overflow(&content, HeadingLevel::H6);

            assert_eq!(warnings.len(), 1);
            assert_eq!(
                warnings[0].line_number,
                Some(5),
                "line must count newlines, not bytes or chars"
            );
        }

        #[test]
        fn zero_adjustment_returns_content_verbatim() {
            let content = "## Root\n\n### Child\n\ntail\n";
            let (text, warnings) = relevel_with_overflow(content, HeadingLevel::H2);

            assert_eq!(text, content);
            assert!(warnings.is_empty());
        }

        #[test]
        fn heading_free_content_returns_verbatim() {
            let content = "Just prose.\n\nNo headings at all.\n";
            let (text, warnings) = relevel_with_overflow(content, HeadingLevel::H3);

            assert_eq!(text, content);
            assert!(warnings.is_empty());
        }
    }
}
