//! Plain Markdown export of composition's embedded render subtrees.

use pulldown_cmark::{Event, Parser};
use renderable::tree::{EMBED_OPEN_PREFIX, decode_embedded_open, is_embedded_close};

/// Removes valid, paired render-tree marker comments, retaining their Markdown
/// fallback and all surrounding source text. Markers in code examples,
/// malformed markers, and unpaired markers remain unchanged.
///
/// Apply only at a plain-text export boundary: terminal and browser folds need
/// the embedded subtree to preserve component styling.
pub fn without_embedded_metadata(content: &str) -> String {
    if !content.contains(EMBED_OPEN_PREFIX) {
        return content.to_string();
    }
    let mut openings = Vec::new();
    let mut removals = Vec::new();
    for (event, range) in Parser::new(content).into_offset_iter() {
        let Event::Html(html) = event else {
            continue;
        };
        if decode_embedded_open(&html).is_some() {
            openings.push(range);
        } else if is_embedded_close(&html)
            && let Some(start) = openings.pop()
        {
            removals.push(start);
            removals.push(range);
        }
    }

    let mut output = content.to_string();
    removals.sort_unstable_by_key(|range| std::cmp::Reverse(range.start));
    for range in removals {
        output.replace_range(range, "");
    }
    output
}

#[cfg(test)]
mod tests {
    use super::*;
    use renderable::tree::{RenderNode, encode_embedded_subtree};

    #[test]
    fn exports_adjacent_subtrees_without_rewriting_surrounding_markdown() {
        let block = encode_embedded_subtree(&RenderNode::paragraph(vec![
            RenderNode::text("fallback"),
        ]))
        .unwrap();
        let source = format!("# Before #\n\n{block}\n{block}\n## After\n");
        let output = without_embedded_metadata(&source);
        assert!(!output.contains("bt:render-tree"));
        assert_eq!(output.matches("fallback").count(), 2);
        assert!(output.starts_with("# Before #\n"));
        assert!(output.ends_with("## After\n"));
        let (outer, close) = block.rsplit_once("<!--bt:render-tree:end-->").unwrap();
        let nested = without_embedded_metadata(&format!("{outer}\n\n{block}\n\n<!--bt:render-tree:end-->{close}"));
        assert!(!nested.contains("bt:render-tree"));
        assert_eq!(nested.matches("fallback").count(), 2);
    }

    #[test]
    fn preserves_code_examples_and_invalid_regions() {
        let block = encode_embedded_subtree(&RenderNode::text("fallback")).unwrap();
        let fenced = format!("```md\n{block}\n```\n");
        assert_eq!(without_embedded_metadata(&fenced), fenced);
        let unclosed = block.lines().next().unwrap();
        assert_eq!(without_embedded_metadata(unclosed), unclosed);
        let corrupt = "<!--bt:render-tree:v1:zz-->\n\nfallback\n\n<!--bt:render-tree:end-->\n";
        assert_eq!(without_embedded_metadata(corrupt), corrupt);
    }
}
