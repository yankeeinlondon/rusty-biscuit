//! Reference analysis subsystem for graph-aware reference discovery and validation.
//!
//! Provides local transclusion queries, unified reference types with provenance,
//! reference graph traversal through composed document trees, and validation.

mod css;
pub mod errors;
pub mod file_tree;
mod graph;
pub(crate) use graph::AuthoredLocator;
pub(crate) mod html;
pub(crate) mod local;
pub mod meta;
pub(crate) mod provenance;
mod report;
pub(crate) mod snapshot;
pub mod types;
pub mod validate;

pub use errors::{
    DependencyMismatchKind, ReferenceError, ReferenceGraphMismatchError, ReferenceGraphMismatchKind,
};
pub use types::*;

use crate::markdown::Markdown;
use crate::markdown::compose::ComposeSource;
use crate::markdown::compose::transclusion::{
    BlockOptions, DirectiveKind, InclusionSite, TransclusionError, frontmatter_value_is_inline, parse_directives,
    parse_frontmatter_refs,
};
use crate::markdown::types::MarkdownResult;
use std::path::PathBuf;

/// Resolve a local reference through the shared detailed resolver.
fn resolve_transclusion_target(
    raw_target: &str,
    source: &ComposeSource,
    options: &crate::markdown::compose::ComposeRequest,
) -> Result<Option<PathBuf>, ReferenceError> {
    lookup_local_target(raw_target, source, options).map(|lookup| match lookup {
        TargetLookup::Found(path) => Some(path),
        TargetLookup::Missing(_) => None,
    })
}

/// A local target's lookup: its file, or no file.
enum TargetLookup {
    Found(PathBuf),
    /// No file: the resolver's evidence for a confirmed no-match, `None` for
    /// a document with no file or a remote target, which have no local
    /// candidate.
    Missing(Option<Box<biscuit_file::DetailedResolution>>),
}

/// [`resolve_transclusion_target`], keeping the resolver's evidence when the
/// target is missing.
fn lookup_local_target(
    raw_target: &str,
    source: &ComposeSource,
    options: &crate::markdown::compose::ComposeRequest,
) -> Result<TargetLookup, ReferenceError> {
    let ComposeSource::File(source_path) = source else {
        return Ok(TargetLookup::Missing(None));
    };
    let context = crate::markdown::compose::context::options::source_file_context(
        options.resolution_context(),
        source_path,
        &options.source_derivation,
        options.source_opening.as_ref(),
    );
    let reference = biscuit_file::FileReference::new(raw_target)?;
    let detailed = reference.resolve_detailed(&context);
    if matches!(detailed.outcome(), biscuit_file::DetailedOutcome::Failed(biscuit_file::ResolutionFailure::NoMatch)) {
        return Ok(TargetLookup::Missing(Some(Box::new(detailed))));
    }
    match detailed.into_convenience() {
        Ok(Some(path)) => Ok(TargetLookup::Found(path)),
        Ok(None) => Ok(TargetLookup::Missing(None)),
        // A remote target has no local file to follow; the validator checks it
        // as a URL.
        Err(error) if error.resolution_failure() == biscuit_file::ResolutionFailure::UnsupportedRemote => {
            Ok(TargetLookup::Missing(None))
        }
        Err(error) => Err(error.into()),
    }
}

/// `error`, the resolution failure of an inclusion's target, at the
/// inclusion `site`: composition's own typed failure
/// ([`TransclusionError::Located`]), so the graph, validation, the local
/// query, and composition report the same code and place. Any other error, or
/// one with no site (a document with no file), is unchanged.
pub(crate) fn at_inclusion(error: ReferenceError, site: Option<InclusionSite>) -> ReferenceError {
    if site.is_none() {
        return error;
    }
    let failure = match error {
        ReferenceError::FileReference(source) => TransclusionError::FileReference(source),
        ReferenceError::Io(source) => TransclusionError::Io(source),
        other => return other,
    };
    ReferenceError::Compose(Box::new(failure.at_inclusion(site).into()))
}

#[allow(dead_code)]
fn source_file_for_error(source: &ComposeSource) -> PathBuf {
    match source {
        ComposeSource::File(path) => path.clone(),
        ComposeSource::Url(url) => PathBuf::from(url.to_string()),
        ComposeSource::Unknown => PathBuf::from("<unknown>"),
    }
}

/// The trimmed text of one-based `line`; empty for the unknown line `0`.
fn directive_text_at_line(content: &str, line: usize) -> String {
    line.checked_sub(1)
        .and_then(|index| content.lines().nth(index))
        .unwrap_or_default()
        .trim()
        .to_string()
}

fn map_reference_parse_error(err: crate::markdown::compose::TransclusionError) -> ReferenceError {
    match err {
        crate::markdown::compose::TransclusionError::ParseDirective {
            ctx,
            line,
            message,
            caret_col,
            authored,
        } => ReferenceError::ParseDirective {
            ctx: ctx.clone(),
            line,
            message,
            directive_text: directive_text_at_line(&ctx.content, line),
            caret_col,
            authored,
        },
        other => {
            // This is a fallback for errors that aren't ParseDirective but
            // occurred during parsing.
            ReferenceError::Validation(other.to_string())
        }
    }
}

/// Extracts every reference in a single document's `content` with byte-span
/// provenance, without composing.
///
/// Pure syntactic extraction: no transclusion following, no shell execution,
/// no network — so it is safe to run over an open buffer on every keystroke.
/// This is the span-carrying, side-effect-free entry point DMLS's substrate
/// indexer builds `references` edges from; the composing
/// [`Markdown::composed_references`] is the wrong shape for a passive analyzer.
///
/// ## Returns
///
/// A [`ReferenceSet`] whose records carry byte-offset spans
/// ([`ReferenceOrigin::span`]) into `content` (not the full document — pass the
/// frontmatter-stripped body and offset the spans if document coordinates are
/// needed).
pub fn extract_document_references(content: &str) -> ReferenceSet {
    graph::extract_all_references(content, &ComposeSource::Unknown)
}

impl Markdown {
    /// Returns `true` if this document contains any transclusion directives
    /// (`::file`, `::code`, `::url`, `::toc-linking`, `::file-links`, `prologue`, or `epilogue`).
    ///
    /// Counts only inclusions whose parser succeeds: one malformed inclusion
    /// hides the valid ones its parser would have returned beside it (every
    /// body `::file`/`::code`/`::url`, every `::toc-linking`, every
    /// `::file-links`, or both frontmatter fields). Use
    /// [`transclusions_with_options`](Self::transclusions_with_options) or a
    /// reference graph to see the parse error.
    pub fn has_transclusions(&self) -> bool {
        // Check block directives
        if let Ok(directives) = parse_directives(self.content(), self.source_context_for_errors())
            && !directives.is_empty()
        {
            return true;
        }

        // Check toc-linking directives
        if let Ok(toc_directives) =
            crate::markdown::compose::toc_linking::parse_directives(self.content())
            && !toc_directives.is_empty()
        {
            return true;
        }

        // Check file-links directives
        if let Ok(file_links_directives) =
            crate::markdown::compose::file_links::parse_file_links_directives(self.content())
            && !file_links_directives.is_empty()
        {
            return true;
        }

        // Check frontmatter prologue/epilogue
        if let Ok(refs) = parse_frontmatter_refs(
            self.frontmatter().as_map(),
            self.source_context_for_errors(),
        ) && (!refs.prologue.is_empty() || !refs.epilogue.is_empty())
        {
            return true;
        }

        false
    }

    /// Returns local transclusion references resolved through `request`.
    ///
    /// This is a local query that does not follow transclusions recursively.
    /// For recursive traversal, use [`transclusion_graph()`](Self::transclusion_graph).
    ///
    /// Every target, body or `prologue`/`epilogue`, resolves from the
    /// document's file as the request admits it, and every origin names that
    /// file: its own source, or the request's file when it has none.
    ///
    /// The request's context is reused for every target, so later CWD, HOME,
    /// or environment changes cannot alter the result. Invalid references and
    /// I/O failures are returned; only a typed no-match produces
    /// `resolved_target: None` for a file target. `resolved_target` is a
    /// lookup, not an admission: `::file ./notes.txt` resolves to the existing
    /// file even though composition, the graphs, and validation refuse a
    /// non-Markdown inclusion. A `prologue`/`epilogue`
    /// value composition emits as inline Markdown (prose, a Markdown link,
    /// multiline or empty text) is listed with its text as `raw_target` and
    /// `resolved_target: None`; no file resolution is attempted for it. A
    /// `::toc-linking` entry lists its authored chain and is not resolved.
    ///
    /// ## Errors
    ///
    /// A malformed inclusion of any kind is the error composition raises for
    /// it, never an empty result: a body directive or `prologue`/`epilogue`
    /// that is not a string or list of strings is
    /// [`MarkdownError::Reference`](crate::markdown::MarkdownError::Reference)
    /// holding [`ReferenceError::ParseDirective`], an invalid `::toc-linking` or
    /// `::file-links` is [`MarkdownError::TocLinking`](crate::markdown::MarkdownError::TocLinking)
    /// or [`MarkdownError::FileLinks`](crate::markdown::MarkdownError::FileLinks).
    /// An omitted field and `[]` list nothing.
    pub fn transclusions_with_options(
        &self,
        request: &crate::markdown::compose::ComposeRequest,
    ) -> MarkdownResult<Vec<TransclusionRef>> {
        self.transclusions_in(request)
    }

    fn transclusions_in(&self, options: &crate::markdown::compose::ComposeRequest) -> MarkdownResult<Vec<TransclusionRef>> {
        // Targets resolve from, and origins name, the document's file as the
        // request admits it (`ComposeRequest::admitted_document`), as
        // composition and the reference graph resolve them; every inclusion
        // form reads the same admitted source.
        let admitted = options.admitted_document(self);
        let document_source = admitted.source().clone().unwrap_or(ComposeSource::Unknown);
        let rooted = options.clone().with_root_source(&document_source);
        let options = &rooted;
        let source = match document_source {
            ComposeSource::File(_) => options.source.clone(),
            other => other,
        };
        let mut refs = Vec::new();

        // The parsers count lines in the body as it is held; a failure names
        // the line in the file as written when the body still matches it.
        let origin = || crate::markdown::compose::body_origin::BodyOrigin::capture(&admitted);

        // Block directives (::file, ::code, ::url)
        let directives = parse_directives(admitted.content(), admitted.source_context_for_errors())
            .map_err(|error| map_reference_parse_error(error.at_authored_directive(&admitted, origin().as_ref(), 0)))?;

        for directive in &directives {
            let kind = match directive.kind {
                DirectiveKind::File => TransclusionRefKind::File,
                DirectiveKind::Code => TransclusionRefKind::Code,
                DirectiveKind::Url => TransclusionRefKind::Url,
            };

            // Fill resolved_target using FileReference semantics (rec #4)
            let resolved_target = match &kind {
                TransclusionRefKind::File | TransclusionRefKind::Code => {
                    resolve_transclusion_target(&directive.raw_target, &source, options)
                        .map_err(|error| {
                            let site = crate::markdown::compose::site_scope::DirectiveSite::of_span(
                                &admitted,
                                origin().as_ref(),
                                &directive.span,
                            );
                            at_inclusion(error, InclusionSite::of_directive(&admitted, &site))
                        })?
                        .map(|path| path.to_string_lossy().to_string())
                }
                TransclusionRefKind::Url => {
                    // URL targets are already absolute
                    Some(directive.raw_target.clone())
                }
                _ => None,
            };

            refs.push(TransclusionRef {
                kind,
                raw_target: directive.raw_target.clone(),
                resolved_target,
                options: block_options_to_ref_options(&directive.options),
                origin: ReferenceOrigin {
                    source: source.clone(),
                    line: directive.line,
                    span: directive.span.clone(),
                    syntax: match directive.kind {
                        DirectiveKind::File => ReferenceSyntax::DirectiveFile,
                        DirectiveKind::Code => ReferenceSyntax::DirectiveCode,
                        DirectiveKind::Url => ReferenceSyntax::DirectiveUrl,
                    },
                    authored: None,
                    section: None,
                },
            });
        }

        // ::toc-linking directives
        let toc_directives = crate::markdown::compose::toc_linking::parse_directives(admitted.content())
            .map_err(|error| error.with_document(admitted.error_document()).at_authored_line(&admitted, origin().as_ref()))?;
        for td in &toc_directives {
            refs.push(TransclusionRef {
                kind: TransclusionRefKind::TocLinking,
                raw_target: td.targets.join(" | "),
                resolved_target: None,
                options: TransclusionRefOptions::default(),
                origin: ReferenceOrigin {
                    source: source.clone(),
                    line: td.line,
                    span: td.span.clone(),
                    syntax: ReferenceSyntax::DirectiveTocLinking,
                    authored: None,
                    section: None,
                },
            });
        }

        // ::file-links directives
        let file_links_directives =
            crate::markdown::compose::file_links::parse_file_links_directives(admitted.content())
                .map_err(|error| error.with_document(admitted.error_document()).at_authored_line(&admitted, origin().as_ref()))?;
        for fd in &file_links_directives {
            let raw_target = match &fd.mode {
                crate::markdown::compose::file_links::FileLinksMode::Glob(g) => g.clone(),
                crate::markdown::compose::file_links::FileLinksMode::Dir { path, depth } => {
                    format!("{} --depth {}", path, depth)
                }
            };
            refs.push(TransclusionRef {
                kind: TransclusionRefKind::FileLinks,
                raw_target,
                resolved_target: None,
                options: TransclusionRefOptions::default(),
                origin: ReferenceOrigin {
                    source: source.clone(),
                    line: fd.line,
                    span: fd.span.clone(),
                    syntax: ReferenceSyntax::DirectiveFileLinks,
                    authored: None,
                    section: None,
                },
            });
        }

        // Frontmatter prologue/epilogue: an omitted field or `[]` lists
        // nothing, any other non-string shape is an error.
        let fm_refs = parse_frontmatter_refs(admitted.frontmatter().as_map(), admitted.frontmatter_error_context())
            .map_err(map_reference_parse_error)?;
        for (index, prologue) in fm_refs.prologue.iter().enumerate() {
            let resolved_target = if frontmatter_value_is_inline(prologue) {
                None
            } else {
                resolve_transclusion_target(prologue, &source, options)
                    .map_err(|error| {
                        at_inclusion(error, InclusionSite::of_frontmatter_value(&admitted, "prologue", index, prologue))
                    })?
                    .map(|path| path.to_string_lossy().to_string())
            };
            refs.push(TransclusionRef {
                kind: TransclusionRefKind::Prologue,
                raw_target: prologue.clone(),
                resolved_target,
                options: TransclusionRefOptions::default(),
                origin: ReferenceOrigin {
                    source: source.clone(),
                    line: 0, // frontmatter doesn't have meaningful line numbers
                    span: 0..0,
                    syntax: ReferenceSyntax::FrontmatterPrologue,
                    authored: None,
                    section: None,
                },
            });
        }

        for (index, epilogue) in fm_refs.epilogue.iter().enumerate() {
            let resolved_target = if frontmatter_value_is_inline(epilogue) {
                None
            } else {
                resolve_transclusion_target(epilogue, &source, options)
                    .map_err(|error| {
                        at_inclusion(error, InclusionSite::of_frontmatter_value(&admitted, "epilogue", index, epilogue))
                    })?
                    .map(|path| path.to_string_lossy().to_string())
            };
            refs.push(TransclusionRef {
                kind: TransclusionRefKind::Epilogue,
                raw_target: epilogue.clone(),
                resolved_target,
                options: TransclusionRefOptions::default(),
                origin: ReferenceOrigin {
                    source: source.clone(),
                    line: 0,
                    span: 0..0,
                    syntax: ReferenceSyntax::FrontmatterEpilogue,
                    authored: None,
                    section: None,
                },
            });
        }

        Ok(refs)
    }

    /// Builds a transclusion-only graph (no link/image extraction at leaf nodes).
    ///
    /// The returned [`ReferenceGraph`] records its build mode as
    /// `TransclusionOnly` in its private provenance. Such a graph is **not**
    /// accepted by [`validate_references_with_graph`](Self::validate_references_with_graph),
    /// which requires a `Full`-mode graph; use [`reference_graph`](Self::reference_graph)
    /// when the graph will feed prebuilt-graph validation.
    ///
    /// ## Errors
    ///
    /// Fails with composition's error for any malformed inclusion in a
    /// document the graph visits (a body directive, `::toc-linking`,
    /// `::file-links`, or a `prologue`/`epilogue` that is not a string or list
    /// of strings), as [`reference_graph`](Self::reference_graph) does.
    pub fn transclusion_graph(
        &self,
        options: ReferenceGraphOptions,
    ) -> MarkdownResult<ReferenceGraph> {
        graph::build_transclusion_graph(self, &options)
    }

    /// Builds a full reference graph (transclusions + all reference types at each node).
    ///
    /// The returned [`ReferenceGraph`] records its build mode as `Full` in its
    /// private provenance, so it is eligible for prebuilt-graph validation via
    /// [`validate_references_with_graph`](Self::validate_references_with_graph)
    /// when built from the same document and `options.compose`.
    ///
    /// ## Errors
    ///
    /// A malformed inclusion in any visited document fails the build with the
    /// error composition raises for it (`MarkdownError::Transclusion`,
    /// `TocLinking`, or `FileLinks`), so no graph, flattened reference list,
    /// or validation report exists for a document composition would reject.
    pub fn reference_graph(
        &self,
        options: ReferenceGraphOptions,
    ) -> MarkdownResult<ReferenceGraph> {
        graph::build_reference_graph(self, &options)
    }

    /// Flattens the reference graph into composed-order references.
    pub fn composed_references(
        &self,
        options: ReferenceGraphOptions,
    ) -> MarkdownResult<ReferenceSet> {
        let g = graph::build_reference_graph(self, &options)?;
        Ok(graph::flatten_graph(&g))
    }

    /// Returns composed-order hyperlinks.
    pub fn composed_links(
        &self,
        options: ReferenceGraphOptions,
    ) -> MarkdownResult<Vec<LinkReference>> {
        let refs = self.composed_references(options)?;
        Ok(refs
            .records
            .into_iter()
            .filter(|r| r.kind == ReferenceKind::Hyperlink)
            .map(LinkReference::from)
            .collect())
    }

    /// Returns composed-order image references.
    pub fn composed_image_references(
        &self,
        options: ReferenceGraphOptions,
    ) -> MarkdownResult<Vec<ImageReference>> {
        let refs = self.composed_references(options)?;
        Ok(refs
            .records
            .into_iter()
            .filter(|r| r.kind == ReferenceKind::Image)
            .map(ImageReference::from)
            .collect())
    }

    // ── Graph-aware Phase 2 APIs (rec #9) ──────────────────────────

    /// Returns inline CSS blocks across the composed document graph.
    pub fn inline_css_graph(
        &self,
        options: ReferenceGraphOptions,
    ) -> MarkdownResult<Vec<InlineCssBlock>> {
        Ok(self
            .composed_references(options)?
            .filter_convert(ReferenceKind::InlineCss))
    }

    /// Returns CSS `@import` references across the composed document graph.
    pub fn css_import_graph(
        &self,
        options: ReferenceGraphOptions,
    ) -> MarkdownResult<Vec<ImportReference>> {
        Ok(self
            .composed_references(options)?
            .filter_convert(ReferenceKind::CssImport))
    }

    /// Returns inline script blocks across the composed document graph.
    pub fn inline_script_graph(
        &self,
        options: ReferenceGraphOptions,
    ) -> MarkdownResult<Vec<InlineScriptBlock>> {
        Ok(self
            .composed_references(options)?
            .filter_convert(ReferenceKind::InlineScript))
    }

    /// Returns `<script src="...">` import references across the composed document graph.
    pub fn script_import_graph(
        &self,
        options: ReferenceGraphOptions,
    ) -> MarkdownResult<Vec<ImportReference>> {
        Ok(self
            .composed_references(options)?
            .filter_convert(ReferenceKind::ScriptImport))
    }

    /// Returns font import references across the composed document graph.
    pub fn font_import_graph(
        &self,
        options: ReferenceGraphOptions,
    ) -> MarkdownResult<Vec<ImportReference>> {
        Ok(self
            .composed_references(options)?
            .filter_convert(ReferenceKind::FontImport))
    }

    // ── Phase 2: Extended extraction (local, single-document) ──────

    /// Returns inline `<style>` blocks from this document.
    pub fn inline_css(&self) -> MarkdownResult<Vec<InlineCssBlock>> {
        let source = self.source().clone().unwrap_or(ComposeSource::Unknown);
        let records = html::extract_html_style_blocks(self.content(), &source);
        Ok(records
            .into_iter()
            .filter(|r| r.kind == ReferenceKind::InlineCss)
            .map(InlineCssBlock::from)
            .collect())
    }

    /// Returns CSS `@import` references from inline `<style>` blocks.
    pub fn css_imports(&self) -> MarkdownResult<Vec<ImportReference>> {
        let source = self.source().clone().unwrap_or(ComposeSource::Unknown);
        let style_records = html::extract_html_style_blocks(self.content(), &source);
        let mut imports = Vec::new();
        for record in &style_records {
            let css_records = css::style_references(self.content(), record, css::extract_css_imports);
            imports.extend(css_records.into_iter().map(ImportReference::from));
        }
        Ok(imports)
    }

    /// Returns inline `<script>` blocks (no `src`).
    pub fn inline_scripts(&self) -> MarkdownResult<Vec<InlineScriptBlock>> {
        let source = self.source().clone().unwrap_or(ComposeSource::Unknown);
        let records = html::extract_html_script_blocks(self.content(), &source);
        Ok(records
            .into_iter()
            .filter(|r| r.kind == ReferenceKind::InlineScript)
            .map(InlineScriptBlock::from)
            .collect())
    }

    /// Returns `<script src="...">` import references.
    pub fn script_imports(&self) -> MarkdownResult<Vec<ImportReference>> {
        let source = self.source().clone().unwrap_or(ComposeSource::Unknown);
        let records = html::extract_html_script_blocks(self.content(), &source);
        Ok(records
            .into_iter()
            .filter(|r| r.kind == ReferenceKind::ScriptImport)
            .map(ImportReference::from)
            .collect())
    }

    /// Returns font import references (`<link ... as="font">`, `@font-face src`).
    pub fn font_imports(&self) -> MarkdownResult<Vec<ImportReference>> {
        let source = self.source().clone().unwrap_or(ComposeSource::Unknown);
        let mut imports = Vec::new();

        // From <link> tags
        let link_records = html::extract_html_link_tags(self.content(), &source);
        imports.extend(
            link_records
                .into_iter()
                .filter(|r| r.kind == ReferenceKind::FontImport)
                .map(ImportReference::from),
        );

        // From @font-face in <style> blocks
        let style_records = html::extract_html_style_blocks(self.content(), &source);
        for record in &style_records {
            let font_records = css::style_references(self.content(), record, css::extract_font_face_sources);
            imports.extend(font_records.into_iter().map(ImportReference::from));
        }

        Ok(imports)
    }

    /// Returns parsed `<meta>` tags.
    pub fn meta_tags(&self) -> MarkdownResult<meta::MetaTagMap> {
        Ok(meta::parse_meta_tags(self.content()))
    }

    /// Merges `<meta>` tag values into frontmatter.
    ///
    /// Returns the number of keys inserted or updated.
    pub fn merge_meta_into_frontmatter(&mut self, overwrite: bool) -> MarkdownResult<usize> {
        let meta_map = meta::parse_meta_tags(self.content());
        meta::merge_meta_into_frontmatter(&meta_map, self, overwrite)
    }

    /// Sets a `<meta>` tag in the document content.
    ///
    /// If a tag with the same key already exists, its value is updated
    /// in place. Otherwise, a new `<meta>` tag is appended.
    ///
    /// ## Examples
    ///
    /// ```
    /// use darkmatter::markdown::Markdown;
    ///
    /// let mut md = Markdown::new("# Hello");
    /// md.set_meta_tag("author", "Ken");
    /// assert_eq!(md.meta_tags().unwrap().get("author").unwrap().as_str(), "Ken");
    /// ```
    ///
    /// ## Returns
    ///
    /// The number of existing tags that were updated (0 means a new tag
    /// was inserted, 1 means an existing tag was replaced).
    pub fn set_meta_tag(&mut self, key: &str, value: &str) -> usize {
        meta::set_meta_tag(self, key, value)
    }

    // ── Validation ──────────────────────────────────────────────────

    /// Validates all references in the document (optionally following transclusions).
    pub fn validate_references(
        &self,
        options: validate::ReferenceValidationOptions,
    ) -> MarkdownResult<validate::ReferenceValidationReport> {
        validate::validate(self, &options).map_err(Into::into)
    }

    /// Validates references against an already-built reference graph.
    ///
    /// Callers that have already built a `Full`-mode [`ReferenceGraph`] for this
    /// document (via [`reference_graph`](Self::reference_graph) with the same
    /// `options.graph`) pass it in to skip a redundant `build_reference_graph`
    /// (Finding 18).
    ///
    /// Before flattening, the prebuilt graph's private provenance is checked
    /// against this call's inputs across four dimensions — document identity,
    /// source, graph mode (`Full` required), and options
    /// (`options.graph`; the other [`ReferenceValidationOptions`](validate::ReferenceValidationOptions)
    /// fields do not participate) — and then every visited local descendant is
    /// re-read from the authoritative filesystem and compared against its
    /// recorded content identity. Identity is clone-stable, so a graph built
    /// from a clone of the options still passes.
    ///
    /// ## Errors
    ///
    /// A mismatch on **any** dimension, or a changed / missing / unreadable
    /// visited descendant, is a **hard error**
    /// ([`ReferenceError::ReferenceGraphMismatch`]) — this method never silently
    /// rebuilds the graph. Build-and-validate callers that do not already hold a
    /// matching graph should call
    /// [`validate_references`](Self::validate_references) instead, which builds
    /// and validates in one step and is compatibility-guaranteed by
    /// construction.
    pub fn validate_references_with_graph(
        &self,
        graph: &ReferenceGraph,
        options: validate::ReferenceValidationOptions,
    ) -> MarkdownResult<validate::ReferenceValidationReport> {
        validate::validate_with_graph(self, &options, graph).map_err(Into::into)
    }
}

/// Converts `BlockOptions` to `TransclusionRefOptions`.
fn block_options_to_ref_options(opts: &BlockOptions) -> TransclusionRefOptions {
    TransclusionRefOptions {
        when_expr: opts.when_expr.clone(),
        replace: opts.replace.clone(),
        quotation: opts.quotation.clone(),
        disclosure: opts.disclosure.clone(),
        exclude: opts.exclude.clone(),
    }
}
