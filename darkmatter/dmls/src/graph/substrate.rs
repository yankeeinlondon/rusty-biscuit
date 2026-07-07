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

use std::path::{Path, PathBuf};

use biscuit_hash::xx_hash_bytes;
use darkmatter::markdown::span::SourceSpan;
use darkmatter::markdown::{
    ReferenceKind, extract_document_references, extract_frontmatter_block, extract_headings,
};

use super::node::{LinkTarget, classify_link_target};
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
}

/// Indexes one document's `source` under logical `path`.
///
/// Pure and side-effect-free: no filesystem or network access, safe to run on
/// every keystroke over an open buffer.
pub fn index_document(path: &Path, source: &str) -> DocumentIndex {
    let content_hash = xx_hash_bytes(source.as_bytes());

    // Body base: everything after the frontmatter block. A fence mismatch or
    // absent frontmatter means the whole document is the body.
    let body_base = match extract_frontmatter_block(source) {
        Ok(Some(extraction)) => extraction.body_span.start,
        Ok(None) | Err(_) => 0,
    };
    let body = &source[body_base..];
    // Document line of the first body line (0 frontmatter lines → base 0).
    let line_base = source[..body_base].bytes().filter(|&b| b == b'\n').count();

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

    let links = extract_document_references(body)
        .records
        .into_iter()
        .filter(|record| record.kind == ReferenceKind::Hyperlink)
        .filter_map(|record| {
            let raw = record.target.raw()?.to_string();
            Some(LinkFact {
                target: classify_link_target(&raw),
                raw_target: raw,
                span: shift(record.origin.span, body_base),
                line: record.origin.line + line_base,
            })
        })
        .collect();

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

    DocumentIndex {
        path: path.to_path_buf(),
        content_hash,
        headings,
        links,
        wiki_links,
    }
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
    fn test_external_links_excluded_from_local_targets() {
        let source = "[ext](https://example.com) [local](./x.md)\n";
        let index = index_document(Path::new("doc.md"), source);
        assert_eq!(index.links.len(), 2);
        assert_eq!(index.links[0].target, LinkTarget::External);
    }
}
