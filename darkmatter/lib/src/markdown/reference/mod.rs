//! Reference analysis subsystem for graph-aware reference discovery and validation.
//!
//! Provides local transclusion queries, unified reference types with provenance,
//! reference graph traversal through composed document trees, and validation.

mod css;
pub mod errors;
pub mod file_tree;
mod graph;
pub(crate) mod html;
pub(crate) mod local;
pub mod meta;
pub(crate) mod provenance;
pub mod types;
pub mod validate;

pub use errors::{
    DependencyMismatchKind, ReferenceError, ReferenceGraphMismatchError, ReferenceGraphMismatchKind,
};
pub use types::*;

use crate::markdown::Markdown;
use crate::markdown::compose::ComposeSource;
use crate::markdown::compose::transclusion::{
    BlockOptions, DirectiveKind, parse_directives, parse_frontmatter_refs,
};
use crate::markdown::types::MarkdownResult;
use std::path::PathBuf;

/// Resolve a raw transclusion target to a string path using `FileReference`
/// semantics, with fallback to simple path join.
///
/// Mirrors the resolution logic in [`graph::resolve_local_target`] so that
/// `transclusions().resolved_target` agrees with graph/validation resolution.
fn resolve_transclusion_target(
    raw_target: &str,
    source: &ComposeSource,
    magic_paths: &[(std::path::PathBuf, biscuit_file::PathPosition)],
) -> Option<String> {
    match source {
        ComposeSource::File(base_path) => {
            let base_dir = base_path.parent();

            // Try biscuit_file::FileReference for full resolution (@repo-root, etc.)
            if let Ok(mut file_ref) = biscuit_file::FileReference::new(raw_target) {
                for (path, position) in magic_paths {
                    file_ref = file_ref.add_magic_path(path, *position);
                }
                if let Ok(Some(resolved)) = file_ref.resolve_relative(base_dir) {
                    return Some(resolved.to_string_lossy().to_string());
                }
            }

            // Fallback to simple path join
            base_dir.map(|dir| dir.join(raw_target).to_string_lossy().to_string())
        }
        _ => None,
    }
}

#[allow(dead_code)]
fn source_file_for_error(source: &ComposeSource) -> PathBuf {
    match source {
        ComposeSource::File(path) => path.clone(),
        ComposeSource::Url(url) => PathBuf::from(url.to_string()),
        ComposeSource::Unknown => PathBuf::from("<unknown>"),
    }
}

fn directive_text_at_line(content: &str, line: usize) -> String {
    content
        .lines()
        .nth(line.saturating_sub(1))
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
        } => ReferenceError::ParseDirective {
            ctx: ctx.clone(),
            line,
            message,
            directive_text: directive_text_at_line(&ctx.content, line),
            caret_col,
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

    /// Returns all transclusion references in this document with provenance.
    ///
    /// This is a local query that does not follow transclusions recursively.
    /// For recursive traversal, use [`transclusion_graph()`](Self::transclusion_graph).
    pub fn transclusions(&self) -> MarkdownResult<Vec<TransclusionRef>> {
        let source = self.source().clone().unwrap_or(ComposeSource::Unknown);
        let mut refs = Vec::new();

        // Block directives (::file, ::code, ::url)
        let directives = parse_directives(self.content(), self.source_context_for_errors())
            .map_err(map_reference_parse_error)?;

        for directive in &directives {
            let kind = match directive.kind {
                DirectiveKind::File => TransclusionRefKind::File,
                DirectiveKind::Code => TransclusionRefKind::Code,
                DirectiveKind::Url => TransclusionRefKind::Url,
            };

            // Fill resolved_target using FileReference semantics (rec #4)
            let resolved_target = match &kind {
                TransclusionRefKind::File | TransclusionRefKind::Code => {
                    resolve_transclusion_target(&directive.raw_target, &source, &[])
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
                },
            });
        }

        // ::toc-linking directives
        if let Ok(toc_directives) =
            crate::markdown::compose::toc_linking::parse_directives(self.content())
        {
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
                    },
                });
            }
        }

        // ::file-links directives
        if let Ok(file_links_directives) =
            crate::markdown::compose::file_links::parse_file_links_directives(self.content())
        {
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
                    },
                });
            }
        }

        // Frontmatter prologue/epilogue
        if let Ok(fm_refs) = parse_frontmatter_refs(
            self.frontmatter().as_map(),
            self.source_context_for_errors(),
        ) {
            for prologue in &fm_refs.prologue {
                let resolved_target = resolve_transclusion_target(prologue, &source, &[]);
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
                    },
                });
            }

            for epilogue in &fm_refs.epilogue {
                let resolved_target = resolve_transclusion_target(epilogue, &source, &[]);
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
                    },
                });
            }
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
            if let Some(css_content) = record
                .attributes
                .get("css_content")
                .and_then(|v| v.as_str())
            {
                let css_records =
                    css::extract_css_imports(css_content, &source, record.origin.line);
                imports.extend(css_records.into_iter().map(ImportReference::from));
            }
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
            if let Some(css_content) = record
                .attributes
                .get("css_content")
                .and_then(|v| v.as_str())
            {
                let font_records =
                    css::extract_font_face_sources(css_content, &source, record.origin.line);
                imports.extend(font_records.into_iter().map(ImportReference::from));
            }
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
