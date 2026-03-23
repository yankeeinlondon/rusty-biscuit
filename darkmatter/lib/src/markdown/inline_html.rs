use std::ops::Range;

use html_escape::decode_html_entities;
use markdown::mdast::Node;
use pulldown_cmark::{Event, Parser};
use tracing::trace;

use super::{markdown_parse_options, output};
use crate::render::{ImageRef, Link};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum HtmlFragmentKind {
    AnchorOpen,
    AnchorClose,
    Image,
    Other,
}

#[derive(Debug)]
struct HtmlFragment<'a> {
    span: Range<usize>,
    value: &'a str,
    kind: HtmlFragmentKind,
}

pub(crate) fn has_inline_html(content: &str) -> bool {
    let mut fence_marker = None;

    for line in content.split_inclusive('\n') {
        if let Some(marker) = fence_marker {
            if detect_fence_marker(line) == Some(marker) {
                fence_marker = None;
            }
            continue;
        }

        if let Some(marker) = detect_fence_marker(line) {
            fence_marker = Some(marker);
            continue;
        }

        if line_has_inline_html(line) {
            return true;
        }
    }

    false
}

pub(crate) fn extract_inline_html_links(content: &str) -> Vec<Link> {
    if !has_inline_html(content) {
        return Vec::new();
    }

    match output::parse_mdast(content) {
        Ok(root) => {
            let fragments = collect_html_fragments(&root, content);
            extract_links_from_fragments(content, &fragments)
        }
        Err(error) => {
            trace!("falling back to pulldown-cmark link extraction: {error}");
            fallback_extract_links(content)
        }
    }
}

pub(crate) fn extract_inline_html_images(content: &str) -> Vec<ImageRef> {
    if !has_inline_html(content) {
        return Vec::new();
    }

    match output::parse_mdast(content) {
        Ok(root) => {
            let fragments = collect_html_fragments(&root, content);
            extract_images_from_fragments(&fragments)
        }
        Err(error) => {
            trace!("falling back to pulldown-cmark image extraction: {error}");
            fallback_extract_images(content)
        }
    }
}

fn extract_links_from_fragments(content: &str, fragments: &[HtmlFragment<'_>]) -> Vec<Link> {
    #[derive(Debug)]
    struct OpenAnchor {
        start: usize,
        nested_invalid: bool,
    }

    let mut links = Vec::new();
    let mut current_anchor: Option<OpenAnchor> = None;

    for fragment in fragments {
        let complete_anchors = extract_complete_anchor_candidates(fragment.value);
        if !complete_anchors.is_empty() {
            for candidate in complete_anchors {
                match Link::try_from(candidate) {
                    Ok(link) => {
                        let normalized = normalize_inline_html_link_display(link.display());
                        links.push(link.with_display(normalized));
                    }
                    Err(error) => trace!("skipping malformed inline html link: {error}"),
                }
            }
            continue;
        }

        match fragment.kind {
            HtmlFragmentKind::AnchorOpen => match current_anchor.as_mut() {
                Some(anchor) => anchor.nested_invalid = true,
                None => {
                    current_anchor = Some(OpenAnchor {
                        start: fragment.span.start,
                        nested_invalid: false,
                    });
                }
            },
            HtmlFragmentKind::AnchorClose => {
                let Some(anchor) = current_anchor.take() else {
                    continue;
                };

                if anchor.nested_invalid {
                    continue;
                }

                let Some(slice) = content.get(anchor.start..fragment.span.end) else {
                    continue;
                };

                match Link::try_from(slice) {
                    Ok(link) => {
                        let normalized = normalize_inline_html_link_display(link.display());
                        links.push(link.with_display(normalized));
                    }
                    Err(error) => trace!("skipping malformed inline html link: {error}"),
                }
            }
            HtmlFragmentKind::Image | HtmlFragmentKind::Other => {}
        }
    }

    links
}

fn extract_images_from_fragments(fragments: &[HtmlFragment<'_>]) -> Vec<ImageRef> {
    let mut images = Vec::new();

    for fragment in fragments {
        for candidate in extract_image_candidates(fragment.value) {
            match ImageRef::try_from(candidate) {
                Ok(image) => images.push(image),
                Err(error) => trace!("skipping malformed inline html image: {error}"),
            }
        }
    }

    images
}

fn classify_fragment(value: &str) -> HtmlFragmentKind {
    let trimmed = value.trim();
    let Some(rest) = trimmed.strip_prefix('<') else {
        return HtmlFragmentKind::Other;
    };

    if rest.starts_with('!') || rest.starts_with('?') {
        return HtmlFragmentKind::Other;
    }

    let (is_closing, rest) = if let Some(rest) = rest.strip_prefix('/') {
        (true, rest)
    } else {
        (false, rest)
    };

    let tag_end = rest
        .find(|ch: char| !ch.is_ascii_alphanumeric() && ch != '-')
        .unwrap_or(rest.len());
    let tag_name = &rest[..tag_end];

    if tag_name.is_empty() {
        return HtmlFragmentKind::Other;
    }

    if is_closing && tag_name.eq_ignore_ascii_case("a") {
        HtmlFragmentKind::AnchorClose
    } else if !is_closing && tag_name.eq_ignore_ascii_case("a") {
        HtmlFragmentKind::AnchorOpen
    } else if !is_closing && tag_name.eq_ignore_ascii_case("img") {
        HtmlFragmentKind::Image
    } else {
        HtmlFragmentKind::Other
    }
}

fn collect_html_fragments<'a>(root: &Node, content: &'a str) -> Vec<HtmlFragment<'a>> {
    let mut fragments = Vec::new();
    collect_html_fragments_inner(root, content, &mut fragments);
    fragments
}

fn collect_html_fragments_inner<'a>(
    node: &Node,
    content: &'a str,
    fragments: &mut Vec<HtmlFragment<'a>>,
) {
    if let Node::Html(_) = node
        && let Some(position) = node.position()
        && let Some(span) = position_to_byte_span(position)
        && let Some(value) = content.get(span.clone())
    {
        fragments.push(HtmlFragment {
            kind: classify_fragment(value),
            span,
            value,
        });
    }

    if let Some(children) = node.children() {
        for child in children {
            collect_html_fragments_inner(child, content, fragments);
        }
    }
}

fn normalize_inline_html_link_display(raw: &str) -> String {
    let mut normalized = String::with_capacity(raw.len());
    let mut idx = 0usize;

    while idx < raw.len() {
        let remaining = &raw[idx..];

        if remaining.starts_with('<')
            && let Some(tag_len) = find_html_tag_end(remaining)
        {
            let tag = &remaining[..tag_len];

            if tag_name_eq(tag, "br") {
                normalized.push('\n');
            } else if tag_name_eq(tag, "code") {
                normalized.push('`');
            }

            idx += tag_len;
            continue;
        }

        let next_tag = remaining.find('<').unwrap_or(remaining.len());
        let text = &remaining[..next_tag];
        normalized.push_str(&strip_markdown_inline_markers(text));
        idx += next_tag;
    }

    decode_html_entities(normalized.trim()).into_owned()
}

/// Strips Markdown inline formatting markers (`**`, `__`, `*`, `_`, `~~`)
/// from text, leaving the visible content intact.
fn strip_markdown_inline_markers(text: &str) -> String {
    let mut result = String::with_capacity(text.len());
    let chars: Vec<char> = text.chars().collect();
    let len = chars.len();
    let mut i = 0;

    while i < len {
        match chars[i] {
            '*' | '_' => {
                let marker = chars[i];
                if i + 1 < len && chars[i + 1] == marker {
                    // Skip double marker (`**` or `__`)
                    i += 2;
                } else {
                    // Skip single marker (`*` or `_`)
                    i += 1;
                }
            }
            '~' if i + 1 < len && chars[i + 1] == '~' => {
                // Skip strikethrough (`~~`)
                i += 2;
            }
            ch => {
                result.push(ch);
                i += 1;
            }
        }
    }

    result
}

fn fallback_extract_links(content: &str) -> Vec<Link> {
    let parser = Parser::new_ext(content, markdown_parse_options());
    let mut links = Vec::new();
    let mut open_tag: Option<String> = None;
    let mut display = String::new();
    let mut nested_invalid = false;

    for event in parser {
        match event {
            Event::InlineHtml(html) | Event::Html(html) => match classify_fragment(&html) {
                HtmlFragmentKind::AnchorOpen => {
                    if open_tag.is_some() {
                        nested_invalid = true;
                    } else {
                        open_tag = Some(html.to_string());
                        display.clear();
                    }
                }
                HtmlFragmentKind::AnchorClose => {
                    let Some(open) = open_tag.take() else {
                        continue;
                    };

                    if nested_invalid {
                        nested_invalid = false;
                        display.clear();
                        continue;
                    }

                    let candidate = format!("{open}{display}{html}");
                    match Link::try_from(candidate.as_str()) {
                        Ok(link) => {
                            let normalized = normalize_inline_html_link_display(link.display());
                            links.push(link.with_display(normalized));
                        }
                        Err(error) => trace!("skipping fallback inline html link: {error}"),
                    }
                    display.clear();
                }
                HtmlFragmentKind::Image | HtmlFragmentKind::Other => {
                    if open_tag.is_some() {
                        display.push_str(&html);
                    }
                }
            },
            Event::Text(text) if open_tag.is_some() => display.push_str(&text),
            Event::Code(code) if open_tag.is_some() => {
                display.push_str("<code>");
                display.push_str(&code);
                display.push_str("</code>");
            }
            Event::SoftBreak | Event::HardBreak if open_tag.is_some() => display.push('\n'),
            _ => {}
        }
    }

    links
}

fn fallback_extract_images(content: &str) -> Vec<ImageRef> {
    let parser = Parser::new_ext(content, markdown_parse_options());
    let mut images = Vec::new();

    for event in parser {
        let html = match event {
            Event::InlineHtml(html) | Event::Html(html) => html,
            _ => continue,
        };

        if classify_fragment(&html) != HtmlFragmentKind::Image {
            continue;
        }

        match ImageRef::try_from(html.as_ref()) {
            Ok(image) => images.push(image),
            Err(error) => trace!("skipping fallback inline html image: {error}"),
        }
    }

    images
}

fn position_to_byte_span(position: &markdown::unist::Position) -> Option<Range<usize>> {
    if position.start.offset <= position.end.offset {
        Some(position.start.offset..position.end.offset)
    } else {
        None
    }
}

fn detect_fence_marker(line: &str) -> Option<char> {
    let trimmed = line.trim_end_matches(['\r', '\n']);
    let indent = trimmed
        .chars()
        .take_while(|ch| *ch == ' ' || *ch == '\t')
        .count();

    if indent > 3 {
        return None;
    }

    let rest = &trimmed[indent..];
    let marker = rest.chars().next()?;
    if marker != '`' && marker != '~' {
        return None;
    }

    let count = rest.chars().take_while(|ch| *ch == marker).count();
    if count >= 3 { Some(marker) } else { None }
}

fn line_has_inline_html(line: &str) -> bool {
    let mut idx = 0usize;

    while idx < line.len() {
        let remaining = &line[idx..];

        if remaining.starts_with('`')
            && let Some(backtick_len) = matching_backtick_run_len(remaining)
        {
            idx += backtick_len;
            continue;
        }

        if remaining.starts_with('<') && is_potential_html_start(remaining) {
            return true;
        }

        idx += remaining
            .chars()
            .next()
            .map(char::len_utf8)
            .unwrap_or_default()
            .max(1);
    }

    false
}

fn matching_backtick_run_len(input: &str) -> Option<usize> {
    let run_len = input.chars().take_while(|ch| *ch == '`').count();
    let closing = "`".repeat(run_len);
    let search_start = run_len;
    let remainder = input.get(search_start..)?;
    let close_idx = remainder.find(&closing)?;
    Some(search_start + close_idx + run_len)
}

fn is_potential_html_start(input: &str) -> bool {
    let Some(rest) = input.strip_prefix('<') else {
        return false;
    };

    if is_probable_autolink(input) {
        return false;
    }

    match rest.chars().next() {
        Some(ch) if ch.is_ascii_alphabetic() => true,
        Some('/') => rest
            .chars()
            .nth(1)
            .is_some_and(|next| next.is_ascii_alphabetic()),
        Some('!') | Some('?') => true,
        _ => false,
    }
}

fn is_probable_autolink(input: &str) -> bool {
    let lower = input.to_ascii_lowercase();
    if lower.starts_with("<http://")
        || lower.starts_with("<https://")
        || lower.starts_with("<mailto:")
    {
        return true;
    }

    let Some(end) = input.find('>') else {
        return false;
    };

    let candidate = &input[1..end];
    !candidate.contains(char::is_whitespace)
        && !candidate.contains('<')
        && candidate.contains('@')
        && !candidate.contains('/')
}

fn find_html_tag_end(input: &str) -> Option<usize> {
    let mut quote = None;

    for (idx, ch) in input.char_indices() {
        match ch {
            '<' if idx == 0 => {}
            '"' | '\'' if quote == Some(ch) => quote = None,
            '"' | '\'' if quote.is_none() => quote = Some(ch),
            '>' if quote.is_none() => return Some(idx + 1),
            _ => {}
        }
    }

    None
}

fn tag_name_eq(tag: &str, expected: &str) -> bool {
    let trimmed = tag.trim();
    let Some(rest) = trimmed.strip_prefix('<') else {
        return false;
    };
    let rest = rest
        .strip_prefix('/')
        .unwrap_or(rest)
        .trim_start_matches(|ch: char| ch.is_ascii_whitespace());
    let name_end = rest
        .find(|ch: char| !ch.is_ascii_alphanumeric() && ch != '-')
        .unwrap_or(rest.len());
    let name = &rest[..name_end];
    !name.is_empty() && name.eq_ignore_ascii_case(expected)
}

fn extract_image_candidates(html: &str) -> Vec<&str> {
    let mut candidates = Vec::new();
    let mut idx = 0usize;

    while idx < html.len() {
        let Some(next_tag_rel) = html[idx..].find('<') else {
            break;
        };
        let tag_start = idx + next_tag_rel;
        let remaining = &html[tag_start..];
        let Some(tag_len) = find_html_tag_end(remaining) else {
            break;
        };
        let tag = &remaining[..tag_len];

        if classify_fragment(tag) == HtmlFragmentKind::Image {
            candidates.push(tag);
        }

        idx = tag_start + tag_len;
    }

    candidates
}

fn extract_complete_anchor_candidates(html: &str) -> Vec<&str> {
    let mut candidates = Vec::new();
    let mut idx = 0usize;

    while idx < html.len() {
        let Some(next_tag_rel) = html[idx..].find('<') else {
            break;
        };
        let tag_start = idx + next_tag_rel;
        let remaining = &html[tag_start..];
        let Some(open_tag_len) = find_html_tag_end(remaining) else {
            break;
        };
        let open_tag = &remaining[..open_tag_len];

        if classify_fragment(open_tag) != HtmlFragmentKind::AnchorOpen {
            idx = tag_start + 1;
            continue;
        }

        let mut search_idx = tag_start + open_tag_len;
        let mut nested_invalid = false;
        let mut closing_end = None;

        while search_idx < html.len() {
            let Some(close_tag_rel) = html[search_idx..].find('<') else {
                break;
            };
            let inner_tag_start = search_idx + close_tag_rel;
            let inner_remaining = &html[inner_tag_start..];
            let Some(inner_tag_len) = find_html_tag_end(inner_remaining) else {
                break;
            };
            let inner_tag = &inner_remaining[..inner_tag_len];

            match classify_fragment(inner_tag) {
                HtmlFragmentKind::AnchorOpen => nested_invalid = true,
                HtmlFragmentKind::AnchorClose => {
                    closing_end = Some(inner_tag_start + inner_tag_len);
                    break;
                }
                HtmlFragmentKind::Image | HtmlFragmentKind::Other => {}
            }

            search_idx = inner_tag_start + inner_tag_len;
        }

        let Some(closing_end) = closing_end else {
            idx = tag_start + open_tag_len;
            continue;
        };

        if !nested_invalid {
            candidates.push(&html[tag_start..closing_end]);
        }

        idx = closing_end;
    }

    candidates
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::markdown::Markdown;
    use crate::render::{FetchPriority, ImageDecoding, ImageLoading, LinkTarget};

    #[test]
    fn has_inline_html_returns_false_for_plain_markdown() {
        assert!(!has_inline_html("# Hello\n\nJust text."));
        assert!(!has_inline_html("[link](https://x.com)"));
        assert!(!has_inline_html("![img](./a.png)"));
    }

    #[test]
    fn has_inline_html_detects_html_tags() {
        assert!(has_inline_html(
            r#"Before <a href="https://x.com">X</a> after"#
        ));
        assert!(has_inline_html(r#"An <img src="./a.png" />"#));
        assert!(has_inline_html("<!-- comment -->"));
        assert!(has_inline_html(r#"<div class="note">Hello</div>"#));
    }

    #[test]
    fn has_inline_html_skips_autolinks_and_code_regions() {
        let fenced = "```html\n<a href=\"https://x.com\">X</a>\n```";
        assert!(!has_inline_html("<https://example.com>"));
        assert!(!has_inline_html(fenced));
        assert!(!has_inline_html(r#"Use `<img src="./a.png" />` inline."#));
    }

    #[test]
    fn inline_html_image_references_extracts_basic_image() {
        let md: Markdown = r#"<img src="./a.png" alt="A" />"#.into();
        let images = md.inline_html_image_references();

        assert_eq!(images.len(), 1);
        assert_eq!(images[0].src(), Some("./a.png"));
        assert_eq!(images[0].alt(), "A");
    }

    #[test]
    fn inline_html_image_references_preserve_attributes_and_order() {
        let md: Markdown = concat!(
            r#"<img src="./a.png" alt="A" title="One" />"#,
            "\n",
            r#"<IMG SRC="./b.png" ALT="B" SRCSET="./b.png 1x, ./b@2x.png 2x" loading="lazy" />"#,
        )
        .into();
        let images = md.inline_html_image_references();

        assert_eq!(images.len(), 2);
        assert_eq!(images[0].src(), Some("./a.png"));
        assert_eq!(images[0].title(), Some("One"));
        assert_eq!(images[1].src(), Some("./b.png"));
        assert_eq!(images[1].srcset(), Some("./b.png 1x, ./b@2x.png 2x"));
        assert_eq!(images[1].loading(), ImageLoading::Lazy);
    }

    #[test]
    fn inline_html_image_references_skip_malformed_and_code_block_images() {
        let malformed: Markdown = r#"<img alt="Missing source" />"#.into();
        assert!(malformed.inline_html_image_references().is_empty());

        let fenced: Markdown = "```html\n<img src=\"./a.png\" alt=\"A\" />\n```".into();
        assert!(fenced.inline_html_image_references().is_empty());
    }

    #[test]
    fn inline_html_image_references_ignore_markdown_images() {
        let md: Markdown =
            "![Markdown](./md.png)\n\n<img src=\"./html.png\" alt=\"HTML\" />".into();
        let images = md.inline_html_image_references();

        assert_eq!(images.len(), 1);
        assert_eq!(images[0].src(), Some("./html.png"));
    }

    #[test]
    fn inline_html_links_extract_basic_anchor() {
        let md: Markdown = r#"<a href="https://x.com">Click</a>"#.into();
        let links = md.inline_html_links();

        assert_eq!(links.len(), 1);
        assert_eq!(links[0].href(), "https://x.com");
        assert_eq!(links[0].display(), "Click");
    }

    #[test]
    fn inline_html_links_preserve_attributes_and_source_order() {
        let md: Markdown = concat!(
            r#"<a href="https://one.com" class="btn" style="margin-top: 4px;" target="_blank" title="One" data-id="1">One</a>"#,
            " and ",
            r#"<a href="./two.md" data-kind="doc">Two</a>"#,
        )
        .into();
        let links = md.inline_html_links();

        assert_eq!(links.len(), 2);
        assert_eq!(links[0].href(), "https://one.com");
        assert_eq!(links[0].class(), Some("btn"));
        assert_eq!(links[0].target(), Some(&LinkTarget::Blank));
        assert_eq!(links[0].title(), Some("One"));
        assert_eq!(links[0].data().get("id"), Some(&"1".to_string()));
        assert!(links[0].style().is_some());
        assert_eq!(links[1].href(), "./two.md");
        assert_eq!(links[1].data().get("kind"), Some(&"doc".to_string()));
    }

    #[test]
    fn inline_html_links_normalize_nested_markup() {
        let md: Markdown = concat!(
            r#"<a href="https://x.com"><strong>Bold</strong> <em>text</em> <code>let x = 1;</code><br />line 2</a>"#
        )
        .into();
        let links = md.inline_html_links();

        assert_eq!(links.len(), 1);
        assert_eq!(links[0].display(), "Bold text `let x = 1;`\nline 2");
    }

    #[test]
    fn inline_html_links_skip_malformed_nested_and_code_block_anchors() {
        let unterminated: Markdown = r#"<a href="https://x.com">Open only"#.into();
        assert!(unterminated.inline_html_links().is_empty());

        let nested: Markdown =
            r#"<a href="https://outer.com">outer <a href="https://inner.com">inner</a></a>"#.into();
        assert!(nested.inline_html_links().is_empty());

        let fenced: Markdown = "```html\n<a href=\"https://x.com\">X</a>\n```".into();
        assert!(fenced.inline_html_links().is_empty());
    }

    #[test]
    fn inline_html_links_extract_uppercase_and_multiline_anchors() {
        let md: Markdown =
            "<A HREF=\"https://x.com\">Text</A>\n\n<a href=\"./docs\">\nLine 1<br>\nLine 2\n</a>"
                .into();
        let links = md.inline_html_links();

        assert_eq!(links.len(), 2);
        assert_eq!(links[0].display(), "Text");
        assert_eq!(links[1].href(), "./docs");
        assert_eq!(links[1].display(), "Line 1\n\nLine 2");
    }

    #[test]
    fn inline_html_links_ignore_markdown_links() {
        let md: Markdown =
            "[Markdown](https://md.example)\n\n<a href=\"https://html.example\">HTML</a>".into();
        let links = md.inline_html_links();

        assert_eq!(links.len(), 1);
        assert_eq!(links[0].href(), "https://html.example");
    }

    #[test]
    fn fallback_extract_images_handles_simple_html() {
        let images = fallback_extract_images(r#"<img src="./a.png" alt="A" />"#);

        assert_eq!(images.len(), 1);
        assert_eq!(images[0].src(), Some("./a.png"));
    }

    #[test]
    fn fallback_extract_links_handles_simple_html() {
        let links = fallback_extract_links(r#"<a href="https://x.com">Click</a>"#);

        assert_eq!(links.len(), 1);
        assert_eq!(links[0].href(), "https://x.com");
        assert_eq!(links[0].display(), "Click");
    }

    #[test]
    fn collect_html_fragments_uses_char_offsets_safely() {
        let content = "é <a href=\"https://x.com\">Click</a>";
        let root = output::parse_mdast(content).expect("mdast should parse");
        let fragments = collect_html_fragments(&root, content);

        assert_eq!(fragments.len(), 2);
        assert_eq!(fragments[0].value, r#"<a href="https://x.com">"#);
        assert_eq!(fragments[1].value, "</a>");
    }

    #[test]
    fn inline_html_links_preserve_data_prompt() {
        let md: Markdown =
            r#"<a href="https://x.com" data-prompt="Summarize this page">Click</a>"#.into();
        let links = md.inline_html_links();

        assert_eq!(links.len(), 1);
        assert_eq!(links[0].href(), "https://x.com");
        assert_eq!(links[0].prompt(), Some("Summarize this page"));
    }

    #[test]
    fn inline_html_image_references_preserve_extended_attributes() {
        let md: Markdown = concat!(
            r#"<img src="./photo.png" alt="Photo" "#,
            r#"decoding="async" fetchpriority="high" "#,
            r#"sizes="(max-width: 600px) 100vw, 50vw" "#,
            r#"width="800" height="600" "#,
            r#"data-caption="A nice photo" />"#,
        )
        .into();
        let images = md.inline_html_image_references();

        assert_eq!(images.len(), 1);
        assert_eq!(images[0].src(), Some("./photo.png"));
        assert_eq!(images[0].decoding(), ImageDecoding::Async);
        assert_eq!(images[0].fetch_priority(), FetchPriority::High);
        assert_eq!(images[0].sizes(), Some("(max-width: 600px) 100vw, 50vw"));
        assert_eq!(images[0].width(), Some(800));
        assert_eq!(images[0].height(), Some(600));
        assert_eq!(
            images[0].data().get("caption"),
            Some(&"A nice photo".to_string())
        );
    }

    #[test]
    fn inline_html_methods_ignore_html_looking_frontmatter() {
        let md: Markdown = concat!(
            "---\n",
            "title: \"See <a href='https://example.com'>this</a>\"\n",
            "icon: \"<img src='icon.png' />\"\n",
            "---\n",
            "\n",
            "Just plain text content.\n",
        )
        .into();

        assert!(md.inline_html_links().is_empty());
        assert!(md.inline_html_image_references().is_empty());
    }

    #[test]
    fn inline_html_links_normalize_markdown_formatting_in_anchor_body() {
        let bold: Markdown = r#"<a href="https://x.com">**Bold**</a>"#.into();
        let bold_links = bold.inline_html_links();
        assert_eq!(bold_links.len(), 1);
        assert_eq!(bold_links[0].display(), "Bold");

        let italic: Markdown = r#"<a href="https://x.com">*italic*</a>"#.into();
        let italic_links = italic.inline_html_links();
        assert_eq!(italic_links.len(), 1);
        assert_eq!(italic_links[0].display(), "italic");

        let strikethrough: Markdown = r#"<a href="https://x.com">~~old~~</a>"#.into();
        let strike_links = strikethrough.inline_html_links();
        assert_eq!(strike_links.len(), 1);
        assert_eq!(strike_links[0].display(), "old");
    }

    #[test]
    fn strip_markdown_inline_markers_preserves_plain_text() {
        assert_eq!(strip_markdown_inline_markers("hello world"), "hello world");
        assert_eq!(strip_markdown_inline_markers(""), "");
    }

    #[test]
    fn strip_markdown_inline_markers_removes_formatting() {
        assert_eq!(strip_markdown_inline_markers("**bold**"), "bold");
        assert_eq!(strip_markdown_inline_markers("*italic*"), "italic");
        assert_eq!(strip_markdown_inline_markers("__bold__"), "bold");
        assert_eq!(strip_markdown_inline_markers("_italic_"), "italic");
        assert_eq!(strip_markdown_inline_markers("~~struck~~"), "struck");
        assert_eq!(
            strip_markdown_inline_markers("**bold** and *italic*"),
            "bold and italic"
        );
    }
}
