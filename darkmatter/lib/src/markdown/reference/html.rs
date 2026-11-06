//! HTML tag extraction with provenance for the reference subsystem.
//!
//! Extracts reference records from HTML tags within markdown documents:
//! - `<a>` and `<img>` (Phase 1)
//! - `<video>`, `<audio>`, `<source>`, `<iframe>` (their `src`)
//! - `<style>`, `<script>`, `<link>`, `<meta>` (Phase 2)
//!
//! A Markdown HTML node can hold any number of tags, and a tag or a `<style>`
//! body can span lines. [`scan_elements`] finds each element in a node, skipping
//! comments and raw-text bodies; a tag's record is located at its `<`.
//!
//! A raw-text element can also span nodes: CommonMark splits one opened after
//! prose, or one it has no block type for, at every tag, and its body can run
//! across paragraphs. [`visit_html_elements`] therefore walks all of a
//! document's HTML nodes in order, and [`inert_ranges`] lets the other readers
//! drop what such a body holds.

use super::types::{
    ReferenceKind, ReferenceOrigin, ReferenceRecord, ReferenceSyntax, ReferenceTarget,
    classify_target, make_reference_id,
};
use crate::markdown::compose::ComposeSource;
use crate::markdown::output;
use markdown::mdast::Node;
use std::borrow::Cow;
use std::ops::Range;
use tracing::trace;

/// Extract HTML `<a>` tags as [`ReferenceRecord`]s with provenance.
pub(crate) fn extract_html_links(content: &str, source: &ComposeSource) -> Vec<ReferenceRecord> {
    collect_from_html_nodes(content, source, classify_a_tag)
}

/// Extract HTML `<img>` tags as [`ReferenceRecord`]s with provenance.
pub(crate) fn extract_html_images(content: &str, source: &ComposeSource) -> Vec<ReferenceRecord> {
    collect_from_html_nodes(content, source, classify_img_tag)
}

/// Extract HTML `<video>` tags as [`ReferenceRecord`]s with provenance.
pub(crate) fn extract_html_videos(content: &str, source: &ComposeSource) -> Vec<ReferenceRecord> {
    collect_from_html_nodes(content, source, classify_video_tag)
}

/// Extract HTML `<audio>` tags as [`ReferenceRecord`]s with provenance.
pub(crate) fn extract_html_audio(content: &str, source: &ComposeSource) -> Vec<ReferenceRecord> {
    collect_from_html_nodes(content, source, classify_audio_tag)
}

/// Extract HTML `<source>` tags as [`ReferenceRecord`]s with provenance.
pub(crate) fn extract_html_sources(content: &str, source: &ComposeSource) -> Vec<ReferenceRecord> {
    collect_from_html_nodes(content, source, classify_source_tag)
}

/// Extract HTML `<iframe>` tags as [`ReferenceRecord`]s with provenance.
pub(crate) fn extract_html_iframes(content: &str, source: &ComposeSource) -> Vec<ReferenceRecord> {
    collect_from_html_nodes(content, source, classify_iframe_tag)
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

/// Applies a classifier function to every active HTML element of `content`.
fn collect_from_html_nodes(
    content: &str,
    source: &ComposeSource,
    classifier: fn(&str, &ComposeSource, usize, usize, usize) -> Option<ReferenceRecord>,
) -> Vec<ReferenceRecord> {
    let mut records = Vec::new();
    visit_html_elements(content, |text, line, span| {
        records.extend(classifier(text, source, line, span.start, span.end));
    });
    records
}

/// The ranges of `content` holding the body of a raw-text element that spans
/// Markdown nodes, each running from the end of the opener's node through the
/// closing tag. Whatever starts inside one is text, not a reference.
pub(crate) fn inert_ranges(content: &str) -> Vec<Range<usize>> {
    if !content.contains('<') {
        return Vec::new();
    }
    visit_html_elements(content, |_, _, _| {})
}

/// `content` with the body of every raw-text element masked, offsets and
/// whitespace unchanged, for a reader that looks for tags in an HTML node's text.
pub(crate) fn blank_raw_text_bodies(content: &str) -> Cow<'_, str> {
    if !content.contains('<') {
        return Cow::Borrowed(content);
    }
    let mut bodies = Vec::new();
    visit_html_elements(content, |text, _, span| {
        if let Some(name) = tag_name(text).map(str::to_ascii_lowercase)
            && RAW_TEXT_ELEMENTS.contains(&name.as_str())
            && let Some(element) = content.get(span.clone())
            && let Some(open_end) = open_tag_end(element)
        {
            // The closing tag stays: without one, reparsing the blanked text
            // could run the element to the end of the document.
            let body_len = last_closing_tag(&element[open_end..], &name)
                .map_or(element.len() - open_end, |closer| closer.start);
            bodies.push(span.start + open_end..span.start + open_end + body_len);
        }
    });
    if bodies.is_empty() {
        return Cow::Borrowed(content);
    }
    let mut blanked = content.as_bytes().to_vec();
    for body in bodies {
        for byte in &mut blanked[body] {
            // Not a space: that would indent the closing tag on a body's last
            // line into a code block, hiding it and what follows it on reparse.
            if !byte.is_ascii_whitespace() {
                *byte = b'x';
            }
        }
    }
    // A body starts and ends on character boundaries and every byte in it but
    // ASCII whitespace was replaced, so the bytes stay valid UTF-8.
    Cow::Owned(String::from_utf8_lossy(&blanked).into_owned())
}

/// One Markdown HTML node, located in the text it was parsed from.
struct HtmlNode<'a> {
    value: &'a str,
    start: usize,
    end: usize,
    line: usize,
}

fn collect_html_nodes<'a>(node: &'a Node, nodes: &mut Vec<HtmlNode<'a>>) {
    if let Node::Html(html) = node
        && let Some(position) = &html.position
    {
        nodes.push(HtmlNode {
            value: &html.value,
            start: position.start.offset,
            end: position.end.offset,
            line: position.start.line,
        });
    }
    if let Some(children) = node.children() {
        for child in children {
            collect_html_nodes(child, nodes);
        }
    }
}

/// Calls `visit` with the text, line, and span in `content` of every active
/// HTML element, in document order, and returns [`inert_ranges`].
///
/// A raw-text element's closing tag is looked for only in the HTML nodes after
/// its own, never in the source: `</textarea>` in a code span or fence reaches
/// a browser escaped, so it closes nothing. The search does not stop at a
/// paragraph, because the browser reads one body across the rendered `<p>`s.
/// With no closing tag in any later node, nothing past the opener's node is
/// inert: a prose mention of `<title>` must not hide the references after it.
fn visit_html_elements(
    content: &str,
    mut visit: impl FnMut(&str, usize, Range<usize>),
) -> Vec<Range<usize>> {
    let root = match output::parse_mdast(content) {
        Ok(root) => root,
        Err(error) => {
            trace!("html reference extraction skipped: {error}");
            return Vec::new();
        }
    };
    let mut nodes = Vec::new();
    collect_html_nodes(&root, &mut nodes);

    let mut inert = Vec::new();
    let mut index = 0;
    let mut resume_at = 0;
    while let Some(node) = nodes.get(index) {
        let lines = LineMap::new(content, node.start, node.value);
        let mut next = (index + 1, 0);
        for element in scan_elements(node.value, resume_at) {
            let line = node.line + node.value[..element.span.start].matches('\n').count();
            let start = lines.to_content(element.span.start);
            let text = &node.value[element.text];
            let spanning = closer_in_later_node(text, &nodes[index + 1..], content).and_then(|closer| {
                Some((content.get(start..closer.content_end)?, closer))
            });
            if let Some((whole, closer)) = spanning {
                visit(whole, line, start..closer.content_end);
                inert.push(node.end..closer.content_end);
                next = (index + 1 + closer.nodes_ahead, closer.value_end);
            } else {
                visit(text, line, start..lines.to_content(element.span.end));
            }
        }
        (index, resume_at) = next;
    }
    inert
}

/// Where a raw-text element's closing tag ends, past its opener's node.
struct LaterCloser {
    /// The closing tag's node, as an index into the nodes after the opener's.
    nodes_ahead: usize,
    /// The end's offset in that node's value.
    value_end: usize,
    content_end: usize,
}

/// The closing tag in `later` for the raw-text element `text` opens, when `text`
/// has a complete open tag and no closing tag of its own.
fn closer_in_later_node(text: &str, later: &[HtmlNode<'_>], content: &str) -> Option<LaterCloser> {
    let name = tag_name(text)?.to_ascii_lowercase();
    let open_end = open_tag_end(text)?;
    if !RAW_TEXT_ELEMENTS.contains(&name.as_str()) || closing_tag(&text[open_end..], &name).is_some() {
        return None;
    }
    later.iter().enumerate().find_map(|(nodes_ahead, node)| {
        let closer = closing_tag(node.value, &name)?;
        Some(LaterCloser {
            nodes_ahead,
            value_end: closer.end,
            content_end: LineMap::new(content, node.start, node.value).to_content(closer.end),
        })
    })
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
    let href = extract_target(html, "href")?;
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
            authored: None,
            section: None,
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
    let src = extract_target(html, "src")?;

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
            authored: None,
            section: None,
        },
        attributes,
    })
}

fn classify_video_tag(
    html: &str,
    source: &ComposeSource,
    line: usize,
    span_start: usize,
    span_end: usize,
) -> Option<ReferenceRecord> {
    if !is_tag(html, "video") {
        return None;
    }
    let src = extract_target(html, "src")?;

    Some(ReferenceRecord {
        id: make_reference_id(source, line, span_start),
        kind: ReferenceKind::HtmlVideo,
        target: classify_target(&src),
        origin: ReferenceOrigin {
            source: source.clone(),
            line,
            span: span_start..span_end,
            syntax: ReferenceSyntax::HtmlVideoTag,
            authored: None,
            section: None,
        },
        attributes: serde_json::Map::new(),
    })
}

fn classify_audio_tag(
    html: &str,
    source: &ComposeSource,
    line: usize,
    span_start: usize,
    span_end: usize,
) -> Option<ReferenceRecord> {
    if !is_tag(html, "audio") {
        return None;
    }
    let src = extract_target(html, "src")?;

    Some(ReferenceRecord {
        id: make_reference_id(source, line, span_start),
        kind: ReferenceKind::HtmlAudio,
        target: classify_target(&src),
        origin: ReferenceOrigin {
            source: source.clone(),
            line,
            span: span_start..span_end,
            syntax: ReferenceSyntax::HtmlAudioTag,
            authored: None,
            section: None,
        },
        attributes: serde_json::Map::new(),
    })
}

fn classify_source_tag(
    html: &str,
    source: &ComposeSource,
    line: usize,
    span_start: usize,
    span_end: usize,
) -> Option<ReferenceRecord> {
    if !is_tag(html, "source") {
        return None;
    }
    let src = extract_target(html, "src")?;

    Some(ReferenceRecord {
        id: make_reference_id(source, line, span_start),
        kind: ReferenceKind::HtmlSource,
        target: classify_target(&src),
        origin: ReferenceOrigin {
            source: source.clone(),
            line,
            span: span_start..span_end,
            syntax: ReferenceSyntax::HtmlSourceTag,
            authored: None,
            section: None,
        },
        attributes: serde_json::Map::new(),
    })
}

fn classify_iframe_tag(
    html: &str,
    source: &ComposeSource,
    line: usize,
    span_start: usize,
    span_end: usize,
) -> Option<ReferenceRecord> {
    if !is_tag(html, "iframe") {
        return None;
    }
    let src = extract_target(html, "src")?;

    Some(ReferenceRecord {
        id: make_reference_id(source, line, span_start),
        kind: ReferenceKind::HtmlIframe,
        target: classify_target(&src),
        origin: ReferenceOrigin {
            source: source.clone(),
            line,
            span: span_start..span_end,
            syntax: ReferenceSyntax::HtmlIframeTag,
            authored: None,
            section: None,
        },
        attributes: serde_json::Map::new(),
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
    attributes.insert("css_content".into(), serde_json::Value::String(css_content));

    Some(ReferenceRecord {
        id: make_reference_id(source, line, span_start),
        kind: ReferenceKind::InlineCss,
        target: ReferenceTarget::Inline,
        origin: ReferenceOrigin {
            source: source.clone(),
            line,
            span: span_start..span_end,
            syntax: ReferenceSyntax::HtmlStyleTag,
            authored: None,
            section: None,
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
    if let Some(src) = extract_target(html, "src") {
        Some(ReferenceRecord {
            id: make_reference_id(source, line, span_start),
            kind: ReferenceKind::ScriptImport,
            target: classify_target(&src),
            origin: ReferenceOrigin {
                source: source.clone(),
                line,
                span: span_start..span_end,
                syntax: ReferenceSyntax::HtmlScriptTag,
                authored: None,
                section: None,
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
                authored: None,
                section: None,
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

    let href = extract_target(html, "href")?;
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
            authored: None,
            section: None,
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
            authored: None,
            section: None,
        },
        attributes,
    })
}

// ── Element scanner ─────────────────────────────────────────────────

/// Elements whose body the HTML parser reads as text, so a tag-like string in
/// one is not a tag.
const RAW_TEXT_ELEMENTS: [&str; 8] =
    ["script", "style", "textarea", "title", "xmp", "iframe", "noembed", "noframes"];

/// One element a [`scan_elements`] pass found. Both ranges index the scanned text.
#[derive(Debug, PartialEq, Eq)]
struct ScannedElement {
    /// What its classifier reads: the open tag, plus the body of a raw-text
    /// element or an `<a>`.
    text: Range<usize>,
    /// What its record spans: the open tag, plus a raw-text element's body.
    /// An `<a>` stops at its open tag because it can contain other references,
    /// and the link stages rewrite targets inside spans assuming they do not
    /// nest.
    span: Range<usize>,
}

/// Finds every element start tag in `html` from `from` on, in order, past
/// comments, closing tags, declarations, and the bodies of raw-text elements.
fn scan_elements(html: &str, from: usize) -> Vec<ScannedElement> {
    let mut elements = Vec::new();
    let mut cursor = from;
    while let Some(found) = html[cursor..].find('<') {
        let start = cursor + found;
        let rest = &html[start..];
        let Some(name) = tag_name(rest) else {
            cursor = start + markup_len(rest);
            continue;
        };
        let open_end = start + open_tag_end(rest).unwrap_or(rest.len());
        let lower_name = name.to_ascii_lowercase();
        let body_end = closing_tag(&html[open_end..], &lower_name).map(|closer| open_end + closer.end);
        if RAW_TEXT_ELEMENTS.contains(&lower_name.as_str()) {
            let end = body_end.unwrap_or(html.len());
            elements.push(ScannedElement { text: start..end, span: start..end });
            cursor = end;
        } else {
            let text_end = if lower_name == "a" { body_end.unwrap_or(open_end) } else { open_end };
            elements.push(ScannedElement { text: start..text_end, span: start..open_end });
            cursor = open_end;
        }
    }
    elements
}

/// The length of the non-element markup `rest` opens: a comment, CDATA section,
/// processing instruction, declaration, or closing tag. An unterminated one
/// runs to the end, and a bare `<` is one character of text.
fn markup_len(rest: &str) -> usize {
    let terminator = if rest.starts_with("<!--") {
        "-->"
    } else if rest.starts_with("<![CDATA[") {
        "]]>"
    } else if rest.starts_with("<?") {
        "?>"
    } else if rest.starts_with("<!") || rest.starts_with("</") {
        ">"
    } else {
        return 1;
    };
    rest.find(terminator).map_or(rest.len(), |end| end + terminator.len())
}

/// The range of the first `</name>` closing tag in `text`, ignoring case.
fn closing_tag(text: &str, lower_name: &str) -> Option<Range<usize>> {
    let needle = format!("</{lower_name}");
    let mut cursor = 0;
    while let Some(found) = text.get(cursor..)?.find("</") {
        let start = cursor + found;
        let after = start + needle.len();
        if text.get(start..after).is_some_and(|opening| opening.eq_ignore_ascii_case(&needle))
            && text[after..].starts_with(|ch: char| ch.is_ascii_whitespace() || ch == '>' || ch == '/')
        {
            let end = text[after..].find('>').map_or(text.len(), |close| after + close + 1);
            return Some(start..end);
        }
        cursor = start + 2;
    }
    None
}

/// The range of the last `</name>` closing tag in `text`, ignoring case. An
/// element's text ends at its real closing tag, so one before it sat in a code
/// span and closes nothing.
fn last_closing_tag(text: &str, lower_name: &str) -> Option<Range<usize>> {
    let mut last = None;
    let mut cursor = 0;
    while let Some(closer) = closing_tag(&text[cursor..], lower_name) {
        last = Some(cursor + closer.start..cursor + closer.end);
        cursor += closer.end;
    }
    last
}

/// Maps offsets in text copied from an Html node back to `content`.
///
/// The copy lacks what each line carries in `content` around it: a `\r` and a
/// container prefix such as `> `. Lines therefore shift independently.
pub(super) struct LineMap {
    /// `(offset in the text, offset in content)` of each line's start.
    lines: Vec<(usize, usize)>,
    /// Whether every line was found verbatim in `content`, and so whether
    /// [`LineMap::to_content`] is exact rather than the line's start plus a guess.
    pub(super) exact: bool,
}

impl LineMap {
    /// Aligns `text`, whose first byte is at `start` in `content`.
    pub(super) fn new(content: &str, start: usize, text: &str) -> Self {
        let mut lines = Vec::new();
        let mut exact = true;
        let mut text_offset = 0;
        let mut line_start = start;
        for (index, line) in text.split_inclusive('\n').enumerate() {
            let line_text = line.trim_end_matches(['\r', '\n']);
            let content_line = content.get(line_start..).unwrap_or("");
            let content_line = &content_line[..content_line.find('\n').unwrap_or(content_line.len())];
            let column = if index == 0 {
                content_line.starts_with(line_text).then_some(0)
            } else {
                column_past_prefix(content_line, line_text)
            };
            exact &= column.is_some();
            lines.push((text_offset, line_start + column.unwrap_or(0)));
            text_offset += line.len();
            line_start += content_line.len() + 1;
        }
        Self { lines, exact }
    }

    /// The `content` offset of `text_offset`.
    pub(super) fn to_content(&self, text_offset: usize) -> usize {
        let index = self.lines.partition_point(|(line_start, _)| *line_start <= text_offset);
        match index.checked_sub(1).map(|line| self.lines[line]) {
            Some((line_start, content_start)) => content_start + (text_offset - line_start),
            None => text_offset,
        }
    }
}

/// Where `line_text` starts in `content_line`, past a container prefix. The
/// rightmost candidate wins: a `>` closing a tag is also what a block-quote
/// prefix is made of.
fn column_past_prefix(content_line: &str, line_text: &str) -> Option<usize> {
    let prefix_len = content_line.len() - content_line.trim_start_matches([' ', '\t', '>']).len();
    (0..=prefix_len).rev().find(|column| content_line[*column..].starts_with(line_text))
}

// ── Helpers ─────────────────────────────────────────────────────────

/// The tag name `text` opens with, when it starts with an element's start tag.
fn tag_name(text: &str) -> Option<&str> {
    let rest = text.strip_prefix('<')?;
    let end = rest
        .find(|ch: char| ch.is_ascii_whitespace() || ch == '/' || ch == '>')
        .unwrap_or(rest.len());
    let name = &rest[..end];
    name.starts_with(|ch: char| ch.is_ascii_alphabetic()).then_some(name)
}

/// Checks if the HTML string is a specific tag.
fn is_tag(html: &str, wanted: &str) -> bool {
    tag_name(html.trim_start()).is_some_and(|name| name.eq_ignore_ascii_case(wanted))
}

/// One attribute of a start tag: its raw name and its raw value, which a bare
/// attribute and an unterminated quote do not have.
type RawAttribute<'a> = (&'a str, Option<&'a str>);

/// Reads a start tag's attributes the way an HTML tokenizer does: a quote only
/// opens a value right after `=`, and any ASCII whitespace separates tokens,
/// line breaks included.
///
/// Returns the attributes and the offset just past the closing `>`, which is
/// `None` when `tag` ends first.
fn read_start_tag(tag: &str) -> (Vec<RawAttribute<'_>>, Option<usize>) {
    let mut attributes = Vec::new();
    let mut pos = tag_name(tag).map_or(0, |name| name.len() + 1);
    let skip_space = |pos: usize| {
        tag[pos..].find(|ch: char| !ch.is_ascii_whitespace()).map_or(tag.len(), |skip| pos + skip)
    };
    loop {
        pos = skip_space(pos);
        match tag[pos..].chars().next() {
            None => return (attributes, None),
            Some('>') => return (attributes, Some(pos + 1)),
            Some('/') => {
                pos += 1;
                continue;
            }
            Some(_) => {}
        }
        // A `=` that starts an attribute is part of its name.
        let name_start = pos + tag[pos..].chars().next().map_or(0, char::len_utf8);
        let name_end = tag[name_start..]
            .find(|ch: char| ch.is_ascii_whitespace() || matches!(ch, '=' | '>' | '/'))
            .map_or(tag.len(), |end| name_start + end);
        let name = &tag[pos..name_end];
        pos = skip_space(name_end);
        if !tag[pos..].starts_with('=') {
            attributes.push((name, None));
            continue;
        }
        pos = skip_space(pos + 1);
        match tag[pos..].chars().next() {
            Some(quote @ ('"' | '\'')) => match tag[pos + 1..].find(quote) {
                Some(len) => {
                    attributes.push((name, Some(&tag[pos + 1..pos + 1 + len])));
                    pos += len + 2;
                }
                None => {
                    attributes.push((name, None));
                    return (attributes, None);
                }
            },
            _ => {
                let end = tag[pos..]
                    .find(|ch: char| ch.is_ascii_whitespace() || ch == '>')
                    .map_or(tag.len(), |len| pos + len);
                attributes.push((name, Some(&tag[pos..end])));
                pos = end;
            }
        }
    }
}

/// The offset just past the `>` closing the start tag `tag` begins with.
pub(super) fn open_tag_end(tag: &str) -> Option<usize> {
    read_start_tag(tag).1
}

/// Extracts the entity-decoded value of an HTML attribute from the start tag
/// `html` begins with. The attribute name matches whole and ignores case.
pub(super) fn extract_attribute(html: &str, attr_name: &str) -> Option<String> {
    read_start_tag(html.trim_start())
        .0
        .into_iter()
        .find(|(name, _)| name.eq_ignore_ascii_case(attr_name))
        .and_then(|(_, value)| value)
        .map(|value| html_escape::decode_html_entities(value).to_string())
}

/// The byte range of an attribute's raw value within `tag`, which must begin
/// with the start tag. The attribute name matches whole and ignores case.
pub(crate) fn attribute_value_range(tag: &str, attr_name: &str) -> Option<Range<usize>> {
    let (_, value) = read_start_tag(tag)
        .0
        .into_iter()
        .find(|(name, _)| name.eq_ignore_ascii_case(attr_name))?;
    let value = value?;
    // The tokenizer's values are subslices of `tag`.
    let start = value.as_ptr() as usize - tag.as_ptr() as usize;
    Some(start..start + value.len())
}

/// The target a URL attribute names. A browser strips the whitespace around a
/// URL attribute, and an empty one loads nothing, so it is no reference; a
/// file reference could not even parse it.
fn extract_target(html: &str, attr_name: &str) -> Option<String> {
    let value = extract_attribute(html, attr_name)?;
    let target = value.trim();
    (!target.is_empty()).then(|| target.to_string())
}

/// Extracts the entity-decoded text between the start tag `html` begins with
/// and that element's closing tag. An unclosed raw-text element runs to the end
/// of `html`, as it does in a browser.
pub(super) fn extract_inner_text(html: &str) -> Option<String> {
    let html = html.trim_start();
    let name = tag_name(html)?.to_ascii_lowercase();
    let after_tag = &html[open_tag_end(html)?..];
    let text = match last_closing_tag(after_tag, &name) {
        Some(closer) => &after_tag[..closer.start],
        None if RAW_TEXT_ELEMENTS.contains(&name.as_str()) => after_tag,
        None => return None,
    };
    (!text.is_empty()).then(|| html_escape::decode_html_entities(text).to_string())
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
    fn extract_html_video_tag() {
        let content = r#"<video src="./movie.mp4"></video>"#;
        let records = extract_html_videos(content, &ComposeSource::Unknown);
        assert_eq!(records.len(), 1);
        assert_eq!(records[0].kind, ReferenceKind::HtmlVideo);
        assert_eq!(records[0].origin.syntax, ReferenceSyntax::HtmlVideoTag);
        assert!(matches!(
            &records[0].target,
            super::super::types::ReferenceTarget::LocalPath { raw } if raw == "./movie.mp4"
        ));
    }

    #[test]
    fn extract_html_audio_tag() {
        let content = r#"<audio src="song.mp3"></audio>"#;
        let records = extract_html_audio(content, &ComposeSource::Unknown);
        assert_eq!(records.len(), 1);
        assert_eq!(records[0].kind, ReferenceKind::HtmlAudio);
        assert_eq!(records[0].origin.syntax, ReferenceSyntax::HtmlAudioTag);
        assert!(matches!(
            &records[0].target,
            super::super::types::ReferenceTarget::LocalPath { raw } if raw == "song.mp3"
        ));
    }

    #[test]
    fn extract_html_source_tag() {
        let content = r#"<source src="track.ogg" type="audio/ogg">"#;
        let records = extract_html_sources(content, &ComposeSource::Unknown);
        assert_eq!(records.len(), 1);
        assert_eq!(records[0].kind, ReferenceKind::HtmlSource);
        assert_eq!(records[0].origin.syntax, ReferenceSyntax::HtmlSourceTag);
        assert!(matches!(
            &records[0].target,
            super::super::types::ReferenceTarget::LocalPath { raw } if raw == "track.ogg"
        ));
    }

    #[test]
    fn extract_html_iframe_tag() {
        let content = r#"<iframe src="https://example.com/widget"></iframe>"#;
        let records = extract_html_iframes(content, &ComposeSource::Unknown);
        assert_eq!(records.len(), 1);
        assert_eq!(records[0].kind, ReferenceKind::HtmlIframe);
        assert_eq!(records[0].origin.syntax, ReferenceSyntax::HtmlIframeTag);
        assert!(matches!(
            &records[0].target,
            super::super::types::ReferenceTarget::RemoteUrl { raw } if raw == "https://example.com/widget"
        ));
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
        assert_eq!(
            records.len(),
            0,
            "<link> should not be emitted as a hyperlink"
        );
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
