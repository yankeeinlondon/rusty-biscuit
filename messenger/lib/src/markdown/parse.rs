use pulldown_cmark::{Event, Options, Parser, Tag, TagEnd};

use super::ast::RichNode;

/// Parse a Markdown string into a rich-text AST.
pub fn parse_markdown(input: &str) -> Vec<RichNode> {
    let options = Options::ENABLE_STRIKETHROUGH;
    let parser = Parser::new_ext(input, options);
    let events: Vec<Event<'_>> = parser.collect();
    parse_events(&events)
}

fn parse_events(events: &[Event<'_>]) -> Vec<RichNode> {
    let mut nodes = Vec::new();
    let mut i = 0;

    while i < events.len() {
        match &events[i] {
            Event::Start(tag) => {
                let end_tag = tag_to_end(tag);
                let (children_events, end_idx) = collect_until_end(events, i + 1, &end_tag);
                let children = parse_events(&children_events);

                let node = match tag {
                    Tag::Paragraph => RichNode::Paragraph(children),
                    Tag::Heading { level, .. } => RichNode::Heading {
                        level: *level as u8,
                        children,
                    },
                    Tag::Strong => RichNode::Bold(children),
                    Tag::Emphasis => RichNode::Italic(children),
                    Tag::Strikethrough => RichNode::Strikethrough(children),
                    Tag::Link { dest_url, .. } => RichNode::Link {
                        url: dest_url.to_string(),
                        children,
                    },
                    Tag::List(start) => {
                        // Children are ListItem paragraphs — group them
                        let items = group_list_items(&children_events);
                        RichNode::List {
                            ordered: start.is_some(),
                            items,
                        }
                    }
                    Tag::Item => {
                        // Items are handled inside List
                        i = end_idx + 1;
                        nodes.extend(children);
                        continue;
                    }
                    Tag::CodeBlock(kind) => {
                        let language = match kind {
                            pulldown_cmark::CodeBlockKind::Fenced(lang) => {
                                let l = lang.to_string();
                                if l.is_empty() { None } else { Some(l) }
                            }
                            pulldown_cmark::CodeBlockKind::Indented => None,
                        };
                        let code = extract_text(&children_events);
                        RichNode::CodeBlock { language, code }
                    }
                    _ => {
                        // For unrecognized tags, just include children
                        i = end_idx + 1;
                        nodes.extend(children);
                        continue;
                    }
                };

                nodes.push(node);
                i = end_idx + 1;
            }
            Event::Text(text) => {
                nodes.push(RichNode::Text(text.to_string()));
                i += 1;
            }
            Event::Code(code) => {
                nodes.push(RichNode::Code(code.to_string()));
                i += 1;
            }
            Event::SoftBreak => {
                nodes.push(RichNode::SoftBreak);
                i += 1;
            }
            Event::HardBreak => {
                nodes.push(RichNode::HardBreak);
                i += 1;
            }
            Event::End(_) => {
                // Should not hit this in normal flow
                i += 1;
            }
            _ => {
                i += 1;
            }
        }
    }

    nodes
}

fn tag_to_end(tag: &Tag<'_>) -> TagEnd {
    match tag {
        Tag::Paragraph => TagEnd::Paragraph,
        Tag::Heading { level, .. } => TagEnd::Heading(*level),
        Tag::Strong => TagEnd::Strong,
        Tag::Emphasis => TagEnd::Emphasis,
        Tag::Strikethrough => TagEnd::Strikethrough,
        Tag::Link { .. } => TagEnd::Link,
        Tag::List(start) => TagEnd::List(start.is_some()),
        Tag::Item => TagEnd::Item,
        Tag::CodeBlock(_) => TagEnd::CodeBlock,
        Tag::BlockQuote(kind) => TagEnd::BlockQuote(*kind),
        Tag::Table(_) => TagEnd::Table,
        Tag::TableHead => TagEnd::TableHead,
        Tag::TableRow => TagEnd::TableRow,
        Tag::TableCell => TagEnd::TableCell,
        Tag::Image { .. } => TagEnd::Image,
        Tag::FootnoteDefinition(_) => TagEnd::FootnoteDefinition,
        Tag::DefinitionList => TagEnd::DefinitionList,
        Tag::DefinitionListTitle => TagEnd::DefinitionListTitle,
        Tag::DefinitionListDefinition => TagEnd::DefinitionListDefinition,
        Tag::MetadataBlock(kind) => TagEnd::MetadataBlock(*kind),
        Tag::HtmlBlock => TagEnd::HtmlBlock,
        Tag::Superscript => TagEnd::Superscript,
        Tag::Subscript => TagEnd::Subscript,
    }
}

/// Collect events until the matching end tag, returning inner events and the end index.
fn collect_until_end<'a>(
    events: &'a [Event<'a>],
    start: usize,
    end_tag: &TagEnd,
) -> (Vec<Event<'a>>, usize) {
    let mut depth = 1usize;
    let mut i = start;

    while i < events.len() {
        match &events[i] {
            Event::Start(tag) if tag_to_end(tag) == *end_tag => {
                depth += 1;
            }
            Event::End(tag) if *tag == *end_tag => {
                depth -= 1;
                if depth == 0 {
                    return (events[start..i].to_vec(), i);
                }
            }
            _ => {}
        }
        i += 1;
    }

    (events[start..].to_vec(), events.len() - 1)
}

/// Group list-level events into per-item ASTs.
fn group_list_items(events: &[Event<'_>]) -> Vec<Vec<RichNode>> {
    let mut items = Vec::new();
    let mut i = 0;

    while i < events.len() {
        if matches!(&events[i], Event::Start(Tag::Item)) {
            let (item_events, end_idx) = collect_until_end(events, i + 1, &TagEnd::Item);
            items.push(parse_events(&item_events));
            i = end_idx + 1;
        } else {
            i += 1;
        }
    }

    items
}

/// Extract raw text from events (for code blocks).
fn extract_text(events: &[Event<'_>]) -> String {
    let mut s = String::new();
    for event in events {
        if let Event::Text(t) = event {
            s.push_str(t);
        }
    }
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_bold_and_italic() {
        let nodes = parse_markdown("**bold** and _italic_");
        assert_eq!(nodes.len(), 1); // one paragraph
        if let RichNode::Paragraph(children) = &nodes[0] {
            assert!(matches!(&children[0], RichNode::Bold(_)));
            assert!(matches!(&children[1], RichNode::Text(_)));
            assert!(matches!(&children[2], RichNode::Italic(_)));
        } else {
            panic!("expected paragraph");
        }
    }

    #[test]
    fn parse_code_block() {
        let nodes = parse_markdown("```rust\nfn main() {}\n```");
        assert_eq!(nodes.len(), 1);
        if let RichNode::CodeBlock { language, code } = &nodes[0] {
            assert_eq!(language.as_deref(), Some("rust"));
            assert_eq!(code, "fn main() {}\n");
        } else {
            panic!("expected code block, got {nodes:?}");
        }
    }

    #[test]
    fn parse_inline_code() {
        let nodes = parse_markdown("use `foo` here");
        if let RichNode::Paragraph(children) = &nodes[0] {
            assert!(matches!(&children[1], RichNode::Code(s) if s == "foo"));
        } else {
            panic!("expected paragraph");
        }
    }

    #[test]
    fn parse_link() {
        let nodes = parse_markdown("[click](https://example.com)");
        if let RichNode::Paragraph(children) = &nodes[0] {
            assert!(matches!(&children[0], RichNode::Link { url, .. } if url == "https://example.com"));
        } else {
            panic!("expected paragraph");
        }
    }

    #[test]
    fn parse_unordered_list() {
        let nodes = parse_markdown("- one\n- two\n- three");
        assert_eq!(nodes.len(), 1);
        if let RichNode::List { ordered, items } = &nodes[0] {
            assert!(!ordered);
            assert_eq!(items.len(), 3);
        } else {
            panic!("expected list, got {nodes:?}");
        }
    }
}
