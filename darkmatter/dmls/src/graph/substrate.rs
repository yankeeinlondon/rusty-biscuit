//! The Markdown substrate indexer.
//!
//! Turns one document's source text into a [`DocumentIndex`] — the per-document
//! parse product the graph builder assembles into a snapshot. All semantic
//! parsing routes through the `darkmatter` library (headings, slugs, links);
//! this module only offsets library spans into document coordinates and
//! computes the content hash used for invalidation.
//!
//! Frontmatter is stripped before parsing: heading and link spans are taken
//! over the body slice and shifted by the body's byte base so every span is
//! document-relative (source-map-ready) rather than body-relative.
//!
//! Frontmatter *meaning* (top-level keys, the `$schema` file reference, and
//! `file(...)`-typed values declared by the document's inline `$schema`) is read
//! through the overlay's span-aware [`FrontmatterAst`] — a pure wrapper over the
//! same `extract_frontmatter_block` primitive — and interpolation variables
//! through the overlay's pure [`expressions`](crate::overlay::expressions)
//! scanner. Both are side-effect-free, so indexing stays keystroke-safe.

use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use biscuit_hash::xx_hash_bytes;
use darkmatter::markdown::compose::directives_api::{
    DirectiveKind, DirectiveOption, scan_darkmatter_directives,
};
use darkmatter::markdown::span::{SourceSpan, Spanned};
use darkmatter::markdown::{
    ReferenceKind, extract_document_references, extract_frontmatter_block, extract_headings,
};

use super::node::{LinkTarget, classify_link_target};
use crate::overlay::{FmValueKind, FrontmatterAst, expressions};
use crate::wiki::scan_wiki_links;

/// A heading fact extracted from one document (document-relative spans).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HeadingFact {
    /// Heading level (1–6).
    pub level: u8,
    /// Rendered inline text.
    pub title: String,
    /// Document-unique GitHub anchor slug.
    pub slug: String,
    /// Byte span of the whole heading element.
    pub span: SourceSpan,
    /// Byte span of the heading's inline text.
    pub title_span: SourceSpan,
    /// 1-indexed document line the heading starts on.
    pub line: usize,
}

/// A link fact extracted from one document (document-relative span).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LinkFact {
    /// Raw target string exactly as authored.
    pub raw_target: String,
    /// Parsed target intent.
    pub target: LinkTarget,
    /// Byte span of the link in the document.
    pub span: SourceSpan,
    /// 1-indexed document line the link is on.
    pub line: usize,
}

/// A `[[wiki]]` link fact extracted from one document (document-relative
/// spans). Resolution against the workspace happens at snapshot assembly.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WikiLinkFact {
    /// Unescaped file-target text (may be empty for `[[#heading]]`).
    pub target: String,
    /// Unescaped heading text, when the link had a `#`.
    pub heading: Option<String>,
    /// Unescaped alias text, when the link had a `|`.
    pub alias: Option<String>,
    /// Byte span of the whole `[[…]]` link.
    pub span: SourceSpan,
    /// Byte span of the file-target text.
    pub target_span: SourceSpan,
    /// Byte span of the heading text, when present.
    pub heading_span: Option<SourceSpan>,
    /// 1-indexed document line the link is on.
    pub line: usize,
    /// A v1-unsupported form (embed, block reference).
    pub unsupported: bool,
}

/// A `::file`/`::code` transclusion fact extracted from one document
/// (document-relative span). Resolution against the workspace happens at
/// snapshot assembly; a target that matches an indexed document gets a
/// `transcludes` edge.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TransclusionFact {
    /// Raw directive target exactly as authored (may carry a `#fragment`).
    pub raw_target: String,
    /// The workspace-local path portion (any `#fragment` stripped).
    pub path: String,
    /// Byte span of the directive target in the document.
    pub span: SourceSpan,
    /// 1-indexed document line the directive is on.
    pub line: usize,
}

/// A file-use fact extracted from one document (document-relative span).
///
/// Covers the `$schema` file reference (`uses_schema`) and every `uses_file`
/// source: images, frontmatter `file(...)` values, `style.page.stylesheet`
/// assets, and `::file-links`/`::toc-linking` directive paths. Resolution
/// against the workspace happens at snapshot assembly; a target that matches an
/// indexed Markdown document gets a resolved edge, else the raw path is kept.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileRefFact {
    /// Raw reference exactly as authored (may carry a `#fragment`).
    pub raw_target: String,
    /// The workspace-local path portion (any `#fragment` stripped).
    pub path: String,
    /// Byte span of the reference in the document.
    pub span: SourceSpan,
    /// 1-indexed document line the reference is on.
    pub line: usize,
}

/// A `{{ … }}` interpolation-variable use extracted from one document
/// (document-relative span). Resolution to a same-document frontmatter key
/// happens at snapshot assembly.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VariableUseFact {
    /// The leading identifier of the expression (`title`, `ctx.today`).
    pub name: String,
    /// Byte span of the inner expression text.
    pub span: SourceSpan,
    /// 1-indexed document line the interpolation is on.
    pub line: usize,
}

/// A top-level frontmatter key extracted from one document (document-relative
/// span) — the target a `uses_variable` edge resolves to.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FrontmatterKeyFact {
    /// The key name.
    pub key: String,
    /// Byte span of the key token.
    pub span: SourceSpan,
    /// 1-indexed document line the key is on.
    pub line: usize,
}

/// The per-document parse product.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DocumentIndex {
    /// The document's logical path (workspace key).
    pub path: PathBuf,
    /// xxHash of the exact source bytes — the invalidation identity.
    pub content_hash: u64,
    /// Headings in document order.
    pub headings: Vec<HeadingFact>,
    /// Local + external links in document order (external links carry no
    /// graph edge but still become link nodes).
    pub links: Vec<LinkFact>,
    /// `[[wiki]]` links in document order (R-8).
    pub wiki_links: Vec<WikiLinkFact>,
    /// `::file`/`::code` transclusion targets in document order (Layer 3).
    pub transclusions: Vec<TransclusionFact>,
    /// The `$schema` file reference (`uses_schema`), if any.
    pub schema_uses: Vec<FileRefFact>,
    /// File uses (`uses_file`): images, frontmatter `file(...)` values, style
    /// assets, and `::file-links`/`::toc-linking` paths, in document order.
    pub file_uses: Vec<FileRefFact>,
    /// Body interpolation-variable uses (`uses_variable`), in document order.
    pub variable_uses: Vec<VariableUseFact>,
    /// Top-level frontmatter keys, in document order.
    pub frontmatter_keys: Vec<FrontmatterKeyFact>,
}

/// Per-document R-6 stage timings captured by [`index_document_timed`].
///
/// Each field is the wall-clock time this document spent in that stage. The
/// bench harness sums them across the worker pool; because indexing is
/// parallel the sums are aggregate CPU time (stages overlap in real time),
/// which is exactly what a "where is the bottleneck" answer needs.
#[derive(Debug, Default, Clone, Copy)]
pub struct IndexTimings {
    /// Content hashing (xxHash identity).
    pub hash: Duration,
    /// Frontmatter block location + span-aware AST read.
    pub frontmatter: Duration,
    /// Markdown parsing (headings, links, images, wiki links).
    pub parse_markdown: Duration,
    /// Darkmatter DSL scanning (transclusion/file directives, `{{ }}`
    /// interpolation).
    pub directives: Duration,
}

/// Indexes one document's `source` under logical `path`.
///
/// Pure and side-effect-free: no filesystem or network access, safe to run on
/// every keystroke over an open buffer.
pub fn index_document(path: &Path, source: &str) -> DocumentIndex {
    index_document_timed(path, source).0
}

/// Indexes one document, returning the parse product plus per-stage timings.
///
/// The timing instrumentation is a handful of `Instant::now()` reads around
/// the existing `darkmatter` parse calls — nanosecond-scale overhead against
/// per-document parse work — so [`index_document`] delegates here and simply
/// drops the timings rather than maintaining a second indexing path.
pub fn index_document_timed(path: &Path, source: &str) -> (DocumentIndex, IndexTimings) {
    let mut timings = IndexTimings::default();

    let stage = Instant::now();
    let content_hash = xx_hash_bytes(source.as_bytes());
    timings.hash += stage.elapsed();

    // Body base: everything after the frontmatter block. A fence mismatch or
    // absent frontmatter means the whole document is the body.
    let stage = Instant::now();
    let body_base = match extract_frontmatter_block(source) {
        Ok(Some(extraction)) => extraction.body_span.start,
        Ok(None) | Err(_) => 0,
    };
    timings.frontmatter += stage.elapsed();
    let body = &source[body_base..];
    // Document line of the first body line (0 frontmatter lines → base 0).
    let line_base = source[..body_base].bytes().filter(|&b| b == b'\n').count();

    let stage = Instant::now();
    let headings = extract_headings(body)
        .into_iter()
        .map(|record| HeadingFact {
            level: record.level.as_u8(),
            title: record.title,
            slug: record.slug,
            span: shift(record.heading_span, body_base),
            title_span: shift(record.title_span, body_base),
            line: record.line + line_base,
        })
        .collect();

    // Local images become `uses_file` sources; hyperlinks stay `references`
    // sources. One extraction pass feeds both.
    let references = extract_document_references(body).records;
    let links = references
        .iter()
        .filter(|record| record.kind == ReferenceKind::Hyperlink)
        .filter_map(|record| {
            let raw = record.target.raw()?.to_string();
            Some(LinkFact {
                target: classify_link_target(&raw),
                raw_target: raw,
                span: shift(record.origin.span.clone(), body_base),
                line: record.origin.line + line_base,
            })
        })
        .collect();

    let mut file_uses: Vec<FileRefFact> = Vec::new();
    for record in &references {
        if record.kind != ReferenceKind::Image {
            continue;
        }
        let Some(raw) = record.target.raw() else {
            continue;
        };
        if !is_local_path(raw) {
            continue;
        }
        file_uses.push(FileRefFact {
            path: path_before_fragment(raw),
            raw_target: raw.to_string(),
            span: shift(record.origin.span.clone(), body_base),
            line: record.origin.line + line_base,
        });
    }

    let wiki_links = scan_wiki_links(body)
        .into_iter()
        .map(|link| WikiLinkFact {
            target: link.target,
            heading: link.heading,
            alias: link.alias,
            span: shift(link.span, body_base),
            target_span: shift(link.target_span, body_base),
            heading_span: link.heading_span.map(|span| shift(span, body_base)),
            line: link.line + line_base,
            unsupported: link.unsupported,
        })
        .collect();
    timings.parse_markdown += stage.elapsed();

    // `::file`/`::code` targets that name a workspace-local path transclude
    // (`transcludes`); `::file-links`/`::toc-linking` paths are non-transclusion
    // file uses (`uses_file`). `::file-links` also has an option form
    // (`::file-links --dir <path> [--depth N]`) whose directory is carried in the
    // options rather than a positional target — that directory is captured as a
    // `uses_file` source too. Remote (`::url` and `http(s)://`) targets never
    // reference a graph document.
    let stage = Instant::now();
    let mut transclusions: Vec<TransclusionFact> = Vec::new();
    for directive in scan_darkmatter_directives(body) {
        let line = directive.line + line_base;
        match directive.kind {
            DirectiveKind::File | DirectiveKind::Code => {
                let Some(target) = local_directive_target(directive.target) else {
                    continue;
                };
                transclusions.push(TransclusionFact {
                    path: path_before_fragment(&target.value),
                    raw_target: target.value,
                    span: shift(target.span, body_base),
                    line,
                });
            }
            DirectiveKind::TocLinking => {
                let Some(target) = local_directive_target(directive.target) else {
                    continue;
                };
                file_uses.push(FileRefFact {
                    path: path_before_fragment(&target.value),
                    raw_target: target.value,
                    span: shift(target.span, body_base),
                    line,
                });
            }
            DirectiveKind::FileLinks => {
                // Positional glob form (`::file-links docs/*.md`) or the option
                // form (`::file-links --dir docs`); either supplies the directory
                // path. `--dir` values are directories, so the local guard is
                // remote-only (no dot/slash heuristic) — a bare `docs` is valid.
                let dir = directive
                    .target
                    .or_else(|| file_links_dir_value(directive.options));
                let Some(dir) = local_directive_target(dir) else {
                    continue;
                };
                file_uses.push(FileRefFact {
                    path: path_before_fragment(&dir.value),
                    raw_target: dir.value,
                    span: shift(dir.span, body_base),
                    line,
                });
            }
            _ => {}
        }
    }
    timings.directives += stage.elapsed();

    // Frontmatter meaning: top-level keys (uses_variable targets), the `$schema`
    // file reference (uses_schema), `file(...)`-typed values and the
    // `style.page.stylesheet` asset (uses_file). Read through the pure,
    // span-aware `FrontmatterAst`; a hard YAML error yields no tree and so no
    // frontmatter facts (the substrate degrades to body-only).
    let stage = Instant::now();
    let mut schema_uses: Vec<FileRefFact> = Vec::new();
    let mut frontmatter_keys: Vec<FrontmatterKeyFact> = Vec::new();
    if let Some(ast) = FrontmatterAst::parse(source).and_then(|parse| parse.ast) {
        let file_typed = inline_schema_file_keys(&ast);
        for entry in ast.top_level() {
            frontmatter_keys.push(FrontmatterKeyFact {
                key: entry.key.clone(),
                span: entry.key_span.clone(),
                line: line_of(source, entry.key_span.start),
            });
            if entry.kind != FmValueKind::Scalar {
                continue;
            }
            let Some(value) = &entry.scalar else { continue };
            if entry.key == "$schema" {
                // The `$schema` source reference has no schema type to trust, so
                // it keeps the dot/slash heuristic (a `.yaml`/`.md`/`.json` file).
                if looks_like_path(value) && !is_remote_target(value) {
                    schema_uses.push(file_ref_fact(source, value, &entry.value_span));
                }
            } else if file_typed.contains(&entry.key) && is_schema_file_value(value) {
                // The inline `$schema` already types this key as `file(...)`, so a
                // bare extensionless name (e.g. `LICENSE`) is a valid file use.
                file_uses.push(file_ref_fact(source, value, &entry.value_span));
            }
        }
        // `style.page.stylesheet` is the one style asset that is always a file
        // path (sub-spec #7); other style leaves are colors/lengths.
        if let Some(entry) = ast.entry_by_dotted("style.page.stylesheet")
            && entry.kind == FmValueKind::Scalar
            && let Some(value) = &entry.scalar
            && looks_like_path(value)
            && !is_remote_target(value)
        {
            file_uses.push(file_ref_fact(source, value, &entry.value_span));
        }
    }
    timings.frontmatter += stage.elapsed();

    // Body `{{ … }}` interpolation variables (uses_variable). Frontmatter-scope
    // interpolation is the frontmatter provider's concern and is excluded by the
    // body-base filter.
    let stage = Instant::now();
    let variable_uses = expressions::interpolations(source, body_base)
        .into_iter()
        .filter_map(|interpolation| {
            let expr = expressions::parse(&interpolation.text).ok()?;
            let name = expressions::root_identifier(&expr)?;
            Some(VariableUseFact {
                span: interpolation.inner.clone(),
                line: line_of(source, interpolation.inner.start),
                name,
            })
        })
        .collect();
    timings.directives += stage.elapsed();

    let index = DocumentIndex {
        path: path.to_path_buf(),
        content_hash,
        headings,
        links,
        wiki_links,
        transclusions,
        schema_uses,
        file_uses,
        variable_uses,
        frontmatter_keys,
    };
    (index, timings)
}

/// Whether a target is a remote (non-local) reference.
fn is_remote_target(raw: &str) -> bool {
    raw.starts_with("http://") || raw.starts_with("https://")
}

/// A directive target kept only when present and workspace-local (not remote).
fn local_directive_target(target: Option<Spanned<String>>) -> Option<Spanned<String>> {
    let target = target?;
    (!is_remote_target(&target.value)).then_some(target)
}

/// The `--dir` directory value of a `::file-links` option list, when present.
///
/// The scanner records the equals form (`--dir=notes`) as a `--dir` key
/// carrying a value, and the space form (`--dir notes`) as a bare `--dir` flag
/// followed by the path as the next option's bare key. Both name a directory
/// that becomes a `uses_file` source; this returns whichever spanned value
/// applies.
fn file_links_dir_value(options: Vec<DirectiveOption>) -> Option<Spanned<String>> {
    let mut options = options.into_iter();
    while let Some(option) = options.next() {
        if option.key.value != "--dir" {
            continue;
        }
        return match option.value {
            Some(value) => Some(value),
            None => options.next().map(|next| next.key),
        };
    }
    None
}

/// Whether a reference target names a workspace-local file path (not a remote
/// URL, another scheme, a bare fragment, or a data URI).
fn is_local_path(raw: &str) -> bool {
    !raw.is_empty()
        && !raw.starts_with('#')
        && !raw.starts_with("data:")
        && !raw.contains("://")
}

/// The workspace-local path portion of a reference target — everything before
/// the first `#` (a heading/anchor or `::code` line-range fragment).
fn path_before_fragment(raw: &str) -> String {
    match raw.split_once('#') {
        Some((path, _)) => path.to_string(),
        None => raw.to_string(),
    }
}

/// Builds a [`FileRefFact`] over a frontmatter value (spans are already
/// document-relative, so no shift is applied).
fn file_ref_fact(source: &str, value: &str, value_span: &SourceSpan) -> FileRefFact {
    FileRefFact {
        path: path_before_fragment(value),
        raw_target: value.to_string(),
        span: value_span.clone(),
        line: line_of(source, value_span.start),
    }
}

/// Whether a frontmatter scalar is plausibly a local file path (not a URL, an
/// inline object, or an obvious non-path token).
///
/// A dot/slash heuristic for values with no declared schema type — the `$schema`
/// source reference and the `style.page.stylesheet` asset. A value the inline
/// `$schema` already types as `file(...)` uses [`is_schema_file_value`] instead,
/// which does not require a dot or slash.
fn looks_like_path(value: &str) -> bool {
    let value = value.trim();
    !value.is_empty()
        && !value.contains("://")
        && !value.starts_with('{')
        && (value.contains('/') || value.contains('.'))
}

/// Whether a value the inline `$schema` already types as `file(...)` is a
/// navigable local file use.
///
/// The schema having confirmed the `file` type makes a bare extensionless
/// filename (e.g. `LICENSE`, `Makefile`) a valid relative reference — the same
/// implicit-relative form `FileReference` accepts — so only a URL or an
/// inline-object literal disqualifies it. The dot/slash heuristic in
/// [`looks_like_path`] must not gate this path.
fn is_schema_file_value(value: &str) -> bool {
    let value = value.trim();
    !value.is_empty() && !value.contains("://") && !value.starts_with('{')
}

/// The direct-child keys of a document's inline `$schema` mapping whose declared
/// type is `file(...)`.
///
/// This is the pure, document-local slice of `file(...)`-typed detection: an
/// inline `$schema` needs no config or filesystem to read. Extension-baseline
/// and referenced-file `file(...)` properties still resolve request-time in the
/// frontmatter provider (they need the effective schema).
fn inline_schema_file_keys(ast: &FrontmatterAst) -> HashSet<String> {
    ast.entries()
        .iter()
        .enumerate()
        // Structural parentage, not a dotted prefix: a top-level key literally
        // spelled `$schema.foo` is not a child of `$schema`.
        .filter(|(index, entry)| {
            entry.kind == FmValueKind::Scalar
                && ast.key_path_at(*index).as_slice() == ["$schema", entry.key.as_str()]
        })
        .map(|(_, entry)| entry)
        .filter(|entry| entry.scalar.as_deref().is_some_and(is_file_type_string))
        .map(|entry| entry.key.clone())
        .collect()
}

/// Whether a SimplifiedSchema type string declares a `file(...)` type (in any
/// union arm), ignoring the trailing `-> description`.
fn is_file_type_string(type_string: &str) -> bool {
    let type_part = type_string.split("->").next().unwrap_or("").trim();
    type_part.split('|').any(|arm| {
        let arm = arm.trim();
        arm == "file" || arm.starts_with("file(") || arm.starts_with("file[") || arm.starts_with("file ")
    })
}

/// The 1-indexed document line containing byte `offset`.
fn line_of(source: &str, offset: usize) -> usize {
    source[..offset.min(source.len())].bytes().filter(|&b| b == b'\n').count() + 1
}

/// Shifts a body-relative span into document coordinates.
fn shift(span: SourceSpan, base: usize) -> SourceSpan {
    (span.start + base)..(span.end + base)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_index_headings_and_slugs() {
        let source = "# Getting Started\n\n## Setup\n\n## Setup\n";
        let index = index_document(Path::new("doc.md"), source);
        assert_eq!(index.headings.len(), 3);
        assert_eq!(index.headings[0].slug, "getting-started");
        assert_eq!(index.headings[0].level, 1);
        // Duplicate heading slugs get GitHub `-1` disambiguation.
        assert_eq!(index.headings[1].slug, "setup");
        assert_eq!(index.headings[2].slug, "setup-1");
        // The parser's heading span includes the trailing newline.
        assert_eq!(source[index.headings[0].span.clone()].trim_end(), "# Getting Started");
    }

    #[test]
    fn test_index_links_classified() {
        let source = "See [setup](guide/setup.md#install) and [top](#getting-started).\n";
        let index = index_document(Path::new("doc.md"), source);
        assert_eq!(index.links.len(), 2);
        assert_eq!(index.links[0].raw_target, "guide/setup.md#install");
        assert_eq!(
            index.links[0].target,
            LinkTarget::RelativePath {
                path: "guide/setup.md".to_string(),
                fragment: Some("install".to_string()),
            }
        );
        assert_eq!(
            index.links[1].target,
            LinkTarget::SameDocumentAnchor {
                slug: "getting-started".to_string()
            }
        );
    }

    #[test]
    fn test_frontmatter_offsets_spans_to_document_coordinates() {
        let source = "---\ntitle: X\n---\n\n# Body Heading\n";
        let index = index_document(Path::new("doc.md"), source);
        assert_eq!(index.headings.len(), 1);
        // The heading span must index the real document position, not a
        // body-relative one.
        assert_eq!(source[index.headings[0].span.clone()].trim_end(), "# Body Heading");
        // Body heading is on document line 5 (1:---, 2:title, 3:---, 4:blank).
        assert_eq!(index.headings[0].line, 5);
    }

    #[test]
    fn test_content_hash_changes_with_content() {
        let a = index_document(Path::new("d.md"), "# A\n");
        let b = index_document(Path::new("d.md"), "# B\n");
        assert_ne!(a.content_hash, b.content_hash);
        let a2 = index_document(Path::new("d.md"), "# A\n");
        assert_eq!(a.content_hash, a2.content_hash);
    }

    #[test]
    fn test_transclusion_facts_extracted() {
        let source = "# Doc\n\n::file ./intro.md\n::code ./mod.rs\n::url https://x/y.md\n";
        let index = index_document(Path::new("doc.md"), source);
        // The remote `::url` target is excluded; both local targets are kept.
        assert_eq!(index.transclusions.len(), 2);
        assert_eq!(index.transclusions[0].path, "./intro.md");
        assert_eq!(&source[index.transclusions[0].span.clone()], "./intro.md");
        assert_eq!(index.transclusions[1].path, "./mod.rs");
    }

    #[test]
    fn test_transclusion_path_strips_fragment() {
        let source = "::file ./guide.md#setup\n";
        let index = index_document(Path::new("doc.md"), source);
        assert_eq!(index.transclusions[0].path, "./guide.md");
        assert_eq!(index.transclusions[0].raw_target, "./guide.md#setup");
    }

    #[test]
    fn test_file_links_glob_form_is_a_file_use() {
        let source = "# Doc\n\n::file-links docs/*.md\n";
        let index = index_document(Path::new("doc.md"), source);
        assert_eq!(index.transclusions.len(), 0);
        assert_eq!(index.file_uses.len(), 1);
        assert_eq!(index.file_uses[0].path, "docs/*.md");
        assert_eq!(index.file_uses[0].raw_target, "docs/*.md");
        assert_eq!(&source[index.file_uses[0].span.clone()], "docs/*.md");
    }

    #[test]
    fn test_file_links_dir_option_form_is_a_file_use() {
        // The `--dir` option form carries no positional target; its directory
        // value must still become a `uses_file` source (Review 2, High #3).
        let source = "# Doc\n\n::file-links --dir docs --depth 2\n";
        let index = index_document(Path::new("doc.md"), source);
        assert_eq!(index.file_uses.len(), 1);
        assert_eq!(index.file_uses[0].path, "docs");
        assert_eq!(index.file_uses[0].raw_target, "docs");
        // The span must index the directory token in the real document.
        assert_eq!(&source[index.file_uses[0].span.clone()], "docs");
    }

    #[test]
    fn test_file_links_dir_equals_form_is_a_file_use() {
        let source = "::file-links --dir=notes\n";
        let index = index_document(Path::new("doc.md"), source);
        assert_eq!(index.file_uses.len(), 1);
        assert_eq!(index.file_uses[0].path, "notes");
        assert_eq!(&source[index.file_uses[0].span.clone()], "notes");
    }

    #[test]
    fn test_file_links_remote_dir_option_excluded() {
        let source = "::file-links --dir=https://example.com/notes\n";
        let index = index_document(Path::new("doc.md"), source);
        // A remote `--dir` value never references a workspace-local directory.
        assert!(index.file_uses.is_empty());
    }

    #[test]
    fn test_toc_linking_positional_form_is_a_file_use() {
        let source = "::toc-linking ./guide.md\n";
        let index = index_document(Path::new("doc.md"), source);
        assert_eq!(index.transclusions.len(), 0);
        assert_eq!(index.file_uses.len(), 1);
        assert_eq!(index.file_uses[0].path, "./guide.md");
        assert_eq!(&source[index.file_uses[0].span.clone()], "./guide.md");
    }

    #[test]
    fn test_inline_schema_extensionless_file_value_is_a_file_use() {
        // Once the inline `$schema` types `license` as `file`, a bare
        // extensionless value (`LICENSE`) is a valid `uses_file` source.
        let source = "---\n$schema:\n  license: \"file\"\nlicense: LICENSE\n---\n\n# Doc\n";
        let index = index_document(Path::new("doc.md"), source);
        assert_eq!(index.file_uses.len(), 1);
        assert_eq!(index.file_uses[0].path, "LICENSE");
        assert_eq!(&source[index.file_uses[0].span.clone()], "LICENSE");
    }

    #[test]
    fn test_inline_schema_file_value_still_rejects_url_and_inline_object() {
        // A schema-typed `file` value that is a URL or an inline-object literal
        // is never a local file use.
        let url = "---\n$schema:\n  home: \"file\"\nhome: https://example.com/x\n---\n\n# Doc\n";
        assert!(index_document(Path::new("doc.md"), url).file_uses.is_empty());
        let inline = "---\n$schema:\n  home: \"file\"\nhome: \"{ a: 1 }\"\n---\n\n# Doc\n";
        assert!(index_document(Path::new("doc.md"), inline).file_uses.is_empty());
    }

    #[test]
    fn test_is_schema_file_value_accepts_extensionless() {
        assert!(is_schema_file_value("LICENSE"));
        assert!(is_schema_file_value("Makefile"));
        assert!(is_schema_file_value("./guide.md"));
        assert!(!is_schema_file_value("https://example.com/s.yaml"));
        assert!(!is_schema_file_value("{ inline: true }"));
        assert!(!is_schema_file_value("   "));
    }

    #[test]
    fn test_external_links_excluded_from_local_targets() {
        let source = "[ext](https://example.com) [local](./x.md)\n";
        let index = index_document(Path::new("doc.md"), source);
        assert_eq!(index.links.len(), 2);
        assert_eq!(index.links[0].target, LinkTarget::External);
    }

    #[test]
    fn test_interpolation_literal_produces_no_variable_use_fact() {
        // `{{{ ... }}}` is inert on every scanning surface; the substrate must
        // not materialize a `VariableUseFact` (and therefore no graph edge).
        let source = "# Doc\n\nThe title literal is {{{ title }}}.\n";
        let index = index_document(Path::new("doc.md"), source);
        assert!(index.variable_uses.is_empty());
    }

    #[test]
    fn test_regular_interpolation_produces_variable_use_fact() {
        let source = "---\ntitle: Guide\n---\n\n# {{ title }}\n";
        let index = index_document(Path::new("doc.md"), source);
        assert_eq!(index.variable_uses.len(), 1);
        assert_eq!(index.variable_uses[0].name, "title");
    }
}
