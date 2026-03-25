//! HTML tag extraction with provenance for the reference subsystem.
//!
//! Extracts reference records from HTML tags within markdown documents:
//! - `<a>` and `<img>` (Phase 1)
//! - `<style>`, `<script>`, `<link>`, `<meta>` (Phase 2)

use crate::markdown::compose::ComposeSource;
use crate::markdown::output;
use super::types::{
    ReferenceKind, ReferenceOrigin, ReferenceRecord, ReferenceSyntax,
    ReferenceTarget, classify_target, make_reference_id,
};
use markdown::mdast::Node;
use tracing::trace;

/// Extract HTML `<a>` tags as [`ReferenceRecord`]s with provenance.
pub(crate) fn extract_html_links(
    content: &str,
    source: &ComposeSource,
) -> Vec<ReferenceRecord> {
    collect_from_html_nodes(content, source, classify_a_tag)
}

/// Extract HTML `<img>` tags as [`ReferenceRecord`]s with provenance.
pub(crate) fn extract_html_images(
    content: &str,
    source: &ComposeSource,
) -> Vec<ReferenceRecord> {
    collect_from_html_nodes(content, source, classify_img_tag)
}

/// Extract HTML `<style>` blocks as [`ReferenceRecord`]s with provenance.
pub(crate) fn extract_html_style_blocks(
    content: &str,
    source: &ComposeSource,
) -> Vec<ReferenceRecord> {
    collect_from_html_nodes(content, source, classify_style_tag)
}

/// Extract HTML `<script>` blocks as [`ReferenceRecord`]s with provenance.
pub(crate) fn extract_html_script_blocks(
    content: &str,
    source: &ComposeSource,
) -> Vec<ReferenceRecord> {
    collect_from_html_nodes(content, source, classify_script_tag)
}

/// Extract HTML `<link>` tags as [`ReferenceRecord`]s with provenance.
pub(crate) fn extract_html_link_tags(
    content: &str,
    source: &ComposeSource,
) -> Vec<ReferenceRecord> {
    collect_from_html_nodes(content, source, classify_link_tag)
}

/// Extract HTML `<meta>` tags as [`ReferenceRecord`]s with provenance.
pub(crate) fn extract_html_meta_tags(
    content: &str,
    source: &ComposeSource,
) -> Vec<ReferenceRecord> {
    collect_from_html_nodes(content, source, classify_meta_tag)
}

// ── Unified collector ───────────────────────────────────────────────

/// Walks the MDAST, finds `Html` nodes, and applies a classifier function.
fn collect_from_html_nodes(
    content: &str,
    source: &ComposeSource,
    classifier: fn(&str, &ComposeSource, usize, usize, usize) -> Option<ReferenceRecord>,
) -> Vec<ReferenceRecord> {
    let root = match output::parse_mdast(content) {
        Ok(root) => root,
        Err(error) => {
            trace!("html reference extraction skipped: {error}");
            return Vec::new();
        }
    };

    let line_index = super::local::LineIndex::new(content);
    let mut records = Vec::new();
    walk_html_nodes(&root, source, &line_index, classifier, &mut records);
    records
}

fn walk_html_nodes(
    node: &Node,
    source: &ComposeSource,
    line_index: &super::local::LineIndex,
    classifier: fn(&str, &ComposeSource, usize, usize, usize) -> Option<ReferenceRecord>,
    records: &mut Vec<ReferenceRecord>,
) {
    if let Node::Html(html_node) = node {
        let position = html_node.position.as_ref();
        let (span_start, span_end) = position
            .map(|p| (p.start.offset, p.end.offset))
            .unwrap_or((0, 0));
        let line = position
            .map(|p| p.start.line)
            .unwrap_or_else(|| line_index.line_at(span_start));

        if let Some(record) = classifier(&html_node.value, source, line, span_start, span_end) {
            records.push(record);
        }
    }

    if let Some(children) = node.children() {
        for child in children {
            walk_html_nodes(child, source, line_index, classifier, records);
        }
    }
}

// ── Tag classifiers ─────────────────────────────────────────────────

fn classify_a_tag(
    html: &str,
    source: &ComposeSource,
    line: usize,
    span_start: usize,
    span_end: usize,
) -> Option<ReferenceRecord> {
    // Require <a> tag specifically (rec #7a) — prevents <link href="..."> from
    // being misclassified as a hyperlink.
    if !is_tag(html, "a") {
        return None;
    }
    let href = extract_attribute(html, "href")?;
    let display = extract_inner_text(html).unwrap_or_default();

    let mut attributes = serde_json::Map::new();
    attributes.insert("display".into(), serde_json::Value::String(display));
    if let Some(title) = extract_attribute(html, "title") {
        attributes.insert("title".into(), serde_json::Value::String(title));
    }

    Some(ReferenceRecord {
        id: make_reference_id(source, line, span_start),
        kind: ReferenceKind::Hyperlink,
        target: classify_target(&href),
        origin: ReferenceOrigin {
            source: source.clone(),
            line,
            span: span_start..span_end,
            syntax: ReferenceSyntax::HtmlAnchor,
        },
        attributes,
    })
}

fn classify_img_tag(
    html: &str,
    source: &ComposeSource,
    line: usize,
    span_start: usize,
    span_end: usize,
) -> Option<ReferenceRecord> {
    if !is_tag(html, "img") {
        return None;
    }
    let src = extract_attribute(html, "src")?;

    let mut attributes = serde_json::Map::new();
    if let Some(alt) = extract_attribute(html, "alt") {
        attributes.insert("alt".into(), serde_json::Value::String(alt));
    }
    if let Some(title) = extract_attribute(html, "title") {
        attributes.insert("title".into(), serde_json::Value::String(title));
    }
    if let Some(width) = extract_attribute(html, "width")
        && let Ok(w) = width.parse::<u64>()
    {
        attributes.insert("width".into(), serde_json::Value::Number(w.into()));
    }
    if let Some(height) = extract_attribute(html, "height")
        && let Ok(h) = height.parse::<u64>()
    {
        attributes.insert("height".into(), serde_json::Value::Number(h.into()));
    }

    Some(ReferenceRecord {
        id: make_reference_id(source, line, span_start),
        kind: ReferenceKind::Image,
        target: classify_target(&src),
        origin: ReferenceOrigin {
            source: source.clone(),
            line,
            span: span_start..span_end,
            syntax: ReferenceSyntax::HtmlImage,
        },
        attributes,
    })
}

fn classify_style_tag(
    html: &str,
    source: &ComposeSource,
    line: usize,
    span_start: usize,
    span_end: usize,
) -> Option<ReferenceRecord> {
    if !is_tag(html, "style") {
        return None;
    }

    let css_content = extract_inner_text(html).unwrap_or_default();

    let mut attributes = serde_json::Map::new();
    attributes.insert(
        "css_content".into(),
        serde_json::Value::String(css_content),
    );

    Some(ReferenceRecord {
        id: make_reference_id(source, line, span_start),
        kind: ReferenceKind::InlineCss,
        target: ReferenceTarget::Inline,
        origin: ReferenceOrigin {
            source: source.clone(),
            line,
            span: span_start..span_end,
            syntax: ReferenceSyntax::HtmlStyleTag,
        },
        attributes,
    })
}

fn classify_script_tag(
    html: &str,
    source: &ComposeSource,
    line: usize,
    span_start: usize,
    span_end: usize,
) -> Option<ReferenceRecord> {
    if !is_tag(html, "script") {
        return None;
    }

    // Check if it has a src attribute (import) or inline content
    if let Some(src) = extract_attribute(html, "src") {
        Some(ReferenceRecord {
            id: make_reference_id(source, line, span_start),
            kind: ReferenceKind::ScriptImport,
            target: classify_target(&src),
            origin: ReferenceOrigin {
                source: source.clone(),
                line,
                span: span_start..span_end,
                syntax: ReferenceSyntax::HtmlScriptTag,
            },
            attributes: serde_json::Map::new(),
        })
    } else {
        let script_content = extract_inner_text(html).unwrap_or_default();
        let mut attributes = serde_json::Map::new();
        attributes.insert(
            "script_content".into(),
            serde_json::Value::String(script_content),
        );

        Some(ReferenceRecord {
            id: make_reference_id(source, line, span_start),
            kind: ReferenceKind::InlineScript,
            target: ReferenceTarget::Inline,
            origin: ReferenceOrigin {
                source: source.clone(),
                line,
                span: span_start..span_end,
                syntax: ReferenceSyntax::HtmlScriptTag,
            },
            attributes,
        })
    }
}

fn classify_link_tag(
    html: &str,
    source: &ComposeSource,
    line: usize,
    span_start: usize,
    span_end: usize,
) -> Option<ReferenceRecord> {
    if !is_tag(html, "link") {
        return None;
    }

    let href = extract_attribute(html, "href")?;
    let rel = extract_attribute(html, "rel").unwrap_or_default();
    let as_attr = extract_attribute(html, "as").unwrap_or_default();

    // Determine kind based on rel and as attributes (rec #7b).
    // Only actual stylesheet-like rel values become CssImport.
    // Other rel values (canonical, alternate, preconnect, etc.) are not CSS imports.
    let kind = if as_attr == "font"
        || href.ends_with(".woff")
        || href.ends_with(".woff2")
        || href.ends_with(".ttf")
        || href.ends_with(".otf")
    {
        ReferenceKind::FontImport
    } else if rel.contains("stylesheet") {
        ReferenceKind::CssImport
    } else {
        // Non-stylesheet <link> tags (canonical, alternate, preconnect, icon, etc.)
        // are recorded as hyperlinks for completeness, not as CSS imports.
        ReferenceKind::Hyperlink
    };

    let mut attributes = serde_json::Map::new();
    if !rel.is_empty() {
        attributes.insert("rel".into(), serde_json::Value::String(rel));
    }
    if !as_attr.is_empty() {
        attributes.insert("as".into(), serde_json::Value::String(as_attr));
    }

    Some(ReferenceRecord {
        id: make_reference_id(source, line, span_start),
        kind,
        target: classify_target(&href),
        origin: ReferenceOrigin {
            source: source.clone(),
            line,
            span: span_start..span_end,
            syntax: ReferenceSyntax::HtmlLinkTag,
        },
        attributes,
    })
}

fn classify_meta_tag(
    html: &str,
    source: &ComposeSource,
    line: usize,
    span_start: usize,
    span_end: usize,
) -> Option<ReferenceRecord> {
    if !is_tag(html, "meta") {
        return None;
    }

    let mut attributes = serde_json::Map::new();

    // Capture all known meta attributes
    if let Some(name) = extract_attribute(html, "name") {
        attributes.insert("name".into(), serde_json::Value::String(name));
    }
    if let Some(property) = extract_attribute(html, "property") {
        attributes.insert("property".into(), serde_json::Value::String(property));
    }
    if let Some(http_equiv) = extract_attribute(html, "http-equiv") {
        attributes.insert("http-equiv".into(), serde_json::Value::String(http_equiv));
    }
    if let Some(charset) = extract_attribute(html, "charset") {
        attributes.insert("charset".into(), serde_json::Value::String(charset));
    }
    if let Some(content) = extract_attribute(html, "content") {
        attributes.insert("content".into(), serde_json::Value::String(content));
    }

    Some(ReferenceRecord {
        id: make_reference_id(source, line, span_start),
        kind: ReferenceKind::MetaTag,
        target: ReferenceTarget::Inline,
        origin: ReferenceOrigin {
            source: source.clone(),
            line,
            span: span_start..span_end,
            syntax: ReferenceSyntax::HtmlMetaTag,
        },
        attributes,
    })
}

// ── Helpers ─────────────────────────────────────────────────────────

/// Checks if the HTML string is a specific tag.
fn is_tag(html: &str, tag_name: &str) -> bool {
    let trimmed = html.trim_start();
    trimmed.starts_with(&format!("<{tag_name} "))
        || trimmed.starts_with(&format!("<{tag_name}>"))
        || trimmed.starts_with(&format!("<{tag_name}/"))
}

/// Extracts the value of an HTML attribute from a tag string.
pub(super) fn extract_attribute(html: &str, attr_name: &str) -> Option<String> {
    let patterns = [
        format!("{attr_name}=\""),
        format!("{attr_name}='"),
        format!("{attr_name} = \""),
        format!("{attr_name} = '"),
    ];

    for pattern in &patterns {
        if let Some(start) = html.find(pattern.as_str()) {
            let value_start = start + pattern.len();
            let quote_char = if pattern.ends_with('"') { '"' } else { '\'' };
            if let Some(end) = html[value_start..].find(quote_char) {
                return Some(
                    html_escape::decode_html_entities(&html[value_start..value_start + end])
                        .to_string(),
                );
            }
        }
    }

    None
}

/// Extracts inner text between opening and closing tags.
pub(super) fn extract_inner_text(html: &str) -> Option<String> {
    let tag_end = html.find('>')?;
    let after_tag = &html[tag_end + 1..];

    if let Some(close_start) = after_tag.find("</") {
        let text = &after_tag[..close_start];
        if !text.is_empty() {
            return Some(html_escape::decode_html_entities(text).to_string());
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extract_html_anchor_link() {
        let content = r#"<a href="./doc.md">link text</a>"#;
        let records = extract_html_links(content, &ComposeSource::Unknown);
        assert_eq!(records.len(), 1);
        assert_eq!(records[0].kind, ReferenceKind::Hyperlink);
        assert_eq!(records[0].origin.syntax, ReferenceSyntax::HtmlAnchor);
        assert!(matches!(
            &records[0].target,
            super::super::types::ReferenceTarget::LocalPath { raw } if raw == "./doc.md"
        ));
    }

    #[test]
    fn extract_html_anchor_remote() {
        let content = r#"<a href="https://example.com">example</a>"#;
        let records = extract_html_links(content, &ComposeSource::Unknown);
        assert_eq!(records.len(), 1);
        assert!(matches!(
            &records[0].target,
            super::super::types::ReferenceTarget::RemoteUrl { .. }
        ));
    }

    #[test]
    fn extract_html_img() {
        let content = r#"<img src="photo.jpg" alt="a photo">"#;
        let records = extract_html_images(content, &ComposeSource::Unknown);
        assert_eq!(records.len(), 1);
        assert_eq!(records[0].kind, ReferenceKind::Image);
        assert_eq!(records[0].origin.syntax, ReferenceSyntax::HtmlImage);
        let alt = records[0].attributes.get("alt").unwrap().as_str().unwrap();
        assert_eq!(alt, "a photo");
    }

    #[test]
    fn extract_inline_style() {
        let content = "<style>body { color: red; }</style>";
        let records = extract_html_style_blocks(content, &ComposeSource::Unknown);
        assert_eq!(records.len(), 1);
        assert_eq!(records[0].kind, ReferenceKind::InlineCss);
        assert_eq!(records[0].origin.syntax, ReferenceSyntax::HtmlStyleTag);
        assert!(matches!(&records[0].target, ReferenceTarget::Inline));
        let css = records[0]
            .attributes
            .get("css_content")
            .unwrap()
            .as_str()
            .unwrap();
        assert_eq!(css, "body { color: red; }");
    }

    #[test]
    fn extract_script_import() {
        let content = r#"<script src="app.js"></script>"#;
        let records = extract_html_script_blocks(content, &ComposeSource::Unknown);
        assert_eq!(records.len(), 1);
        assert_eq!(records[0].kind, ReferenceKind::ScriptImport);
        assert!(matches!(
            &records[0].target,
            super::super::types::ReferenceTarget::LocalPath { raw } if raw == "app.js"
        ));
    }

    #[test]
    fn extract_inline_script() {
        let content = r#"<script>console.log("hi")</script>"#;
        let records = extract_html_script_blocks(content, &ComposeSource::Unknown);
        assert_eq!(records.len(), 1);
        assert_eq!(records[0].kind, ReferenceKind::InlineScript);
        assert!(matches!(&records[0].target, ReferenceTarget::Inline));
    }

    #[test]
    fn extract_link_stylesheet() {
        let content = r#"<link rel="stylesheet" href="style.css">"#;
        let records = extract_html_link_tags(content, &ComposeSource::Unknown);
        assert_eq!(records.len(), 1);
        assert_eq!(records[0].kind, ReferenceKind::CssImport);
        assert_eq!(records[0].origin.syntax, ReferenceSyntax::HtmlLinkTag);
    }

    #[test]
    fn extract_link_font() {
        let content = r#"<link rel="preload" href="font.woff2" as="font">"#;
        let records = extract_html_link_tags(content, &ComposeSource::Unknown);
        assert_eq!(records.len(), 1);
        assert_eq!(records[0].kind, ReferenceKind::FontImport);
    }

    #[test]
    fn extract_meta_name() {
        let content = r#"<meta name="description" content="Hello">"#;
        let records = extract_html_meta_tags(content, &ComposeSource::Unknown);
        assert_eq!(records.len(), 1);
        assert_eq!(records[0].kind, ReferenceKind::MetaTag);
        assert_eq!(records[0].origin.syntax, ReferenceSyntax::HtmlMetaTag);
        let name = records[0].attributes.get("name").unwrap().as_str().unwrap();
        assert_eq!(name, "description");
    }

    #[test]
    fn extract_meta_og() {
        let content = r#"<meta property="og:title" content="Title">"#;
        let records = extract_html_meta_tags(content, &ComposeSource::Unknown);
        assert_eq!(records.len(), 1);
        let prop = records[0]
            .attributes
            .get("property")
            .unwrap()
            .as_str()
            .unwrap();
        assert_eq!(prop, "og:title");
    }

    #[test]
    fn extract_meta_charset() {
        let content = r#"<meta charset="utf-8">"#;
        let records = extract_html_meta_tags(content, &ComposeSource::Unknown);
        assert_eq!(records.len(), 1);
        let charset = records[0]
            .attributes
            .get("charset")
            .unwrap()
            .as_str()
            .unwrap();
        assert_eq!(charset, "utf-8");
    }

    #[test]
    fn extract_attribute_double_quotes() {
        assert_eq!(
            extract_attribute(r#"<a href="foo.md">"#, "href"),
            Some("foo.md".into())
        );
    }

    #[test]
    fn extract_attribute_single_quotes() {
        assert_eq!(
            extract_attribute("<a href='bar.md'>", "href"),
            Some("bar.md".into())
        );
    }

    #[test]
    fn extract_attribute_missing() {
        assert_eq!(extract_attribute("<a>text</a>", "href"), None);
    }

    #[test]
    fn is_tag_variations() {
        assert!(is_tag("<img src=\"a.png\">", "img"));
        assert!(is_tag("<img src=\"a.png\" />", "img"));
        assert!(!is_tag("<a href=\"x\">", "img"));
        assert!(is_tag("<style>body{}</style>", "style"));
        assert!(is_tag("<meta charset=\"utf-8\">", "meta"));
        assert!(is_tag("<link rel=\"stylesheet\" href=\"x\">", "link"));
        assert!(is_tag("<script src=\"x\"></script>", "script"));
    }

    // ── Regression tests for rec #7a and #7b ────────────────────────

    #[test]
    fn link_tag_does_not_appear_in_extract_html_links() {
        // rec #7a: <link> tags must NOT be classified as hyperlinks by extract_html_links
        let content = r#"<link rel="stylesheet" href="style.css">"#;
        let records = extract_html_links(content, &ComposeSource::Unknown);
        assert_eq!(records.len(), 0, "<link> should not be emitted as a hyperlink");
    }

    #[test]
    fn canonical_link_is_not_css_import() {
        // rec #7b: rel="canonical" must not be classified as CssImport
        let content = r#"<link rel="canonical" href="https://example.com/page">"#;
        let records = extract_html_link_tags(content, &ComposeSource::Unknown);
        assert_eq!(records.len(), 1);
        assert_ne!(records[0].kind, ReferenceKind::CssImport);
    }

    #[test]
    fn preconnect_link_is_not_css_import() {
        let content = r#"<link rel="preconnect" href="https://fonts.googleapis.com">"#;
        let records = extract_html_link_tags(content, &ComposeSource::Unknown);
        assert_eq!(records.len(), 1);
        assert_ne!(records[0].kind, ReferenceKind::CssImport);
    }

    #[test]
    fn alternate_link_is_not_css_import() {
        let content = r#"<link rel="alternate" href="/feed.xml">"#;
        let records = extract_html_link_tags(content, &ComposeSource::Unknown);
        assert_eq!(records.len(), 1);
        assert_ne!(records[0].kind, ReferenceKind::CssImport);
    }

    #[test]
    fn stylesheet_link_is_css_import() {
        let content = r#"<link rel="stylesheet" href="main.css">"#;
        let records = extract_html_link_tags(content, &ComposeSource::Unknown);
        assert_eq!(records.len(), 1);
        assert_eq!(records[0].kind, ReferenceKind::CssImport);
    }

    #[test]
    fn icon_link_is_not_css_import() {
        let content = r#"<link rel="icon" href="/favicon.ico">"#;
        let records = extract_html_link_tags(content, &ComposeSource::Unknown);
        assert_eq!(records.len(), 1);
        assert_ne!(records[0].kind, ReferenceKind::CssImport);
    }
}
