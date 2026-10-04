/// A small rich-text AST node for provider-agnostic rendering.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RichNode {
    Text(String),
    Bold(Vec<RichNode>),
    Italic(Vec<RichNode>),
    Strikethrough(Vec<RichNode>),
    Code(String),
    CodeBlock {
        language: Option<String>,
        code: String,
    },
    Link {
        url: String,
        children: Vec<RichNode>,
    },
    List {
        ordered: bool,
        items: Vec<Vec<RichNode>>,
    },
    Paragraph(Vec<RichNode>),
    Heading {
        level: u8,
        children: Vec<RichNode>,
    },
    SoftBreak,
    HardBreak,
}

/// Splits a delimiter wrapper's `children` into the whitespace and breaks at
/// their start, the content between, and the whitespace and breaks at their
/// end, looking through nested `Bold`/`Italic`/`Strikethrough` wrappers.
///
/// A delimiter run followed (or preceded) by whitespace does not open (or
/// close) emphasis in Discord or Slack, so renderers write the edges outside
/// their delimiters. Children without such edges come back unchanged as the
/// content.
pub(crate) fn split_edges(children: &[RichNode]) -> (Vec<RichNode>, Vec<RichNode>, Vec<RichNode>) {
    let mut content = children.to_vec();
    let leading = take_edge(&mut content, Edge::Leading);
    let trailing = take_edge(&mut content, Edge::Trailing);
    (leading, content, trailing)
}

#[derive(Clone, Copy)]
enum Edge {
    Leading,
    Trailing,
}

fn wrapper_children_mut(node: &mut RichNode) -> Option<&mut Vec<RichNode>> {
    match node {
        RichNode::Bold(children)
        | RichNode::Italic(children)
        | RichNode::Strikethrough(children) => Some(children),
        _ => None,
    }
}

/// Whether `node` renders nothing: empty text or a wrapper of such nodes.
fn is_blank(node: &RichNode) -> bool {
    match node {
        RichNode::Text(text) => text.is_empty(),
        RichNode::Bold(children)
        | RichNode::Italic(children)
        | RichNode::Strikethrough(children) => children.iter().all(is_blank),
        _ => false,
    }
}

/// Removes the whitespace and breaks at one edge of `nodes` and returns them
/// in document order.
fn take_edge(nodes: &mut Vec<RichNode>, edge: Edge) -> Vec<RichNode> {
    let mut moved = Vec::new();
    let place = |moved: &mut Vec<RichNode>, items: Vec<RichNode>| match edge {
        Edge::Leading => moved.extend(items),
        Edge::Trailing => {
            moved.splice(0..0, items);
        }
    };
    loop {
        let index = match edge {
            Edge::Leading => nodes.iter().position(|node| !is_blank(node)),
            Edge::Trailing => nodes.iter().rposition(|node| !is_blank(node)),
        };
        let Some(index) = index else { break };
        match &mut nodes[index] {
            RichNode::SoftBreak | RichNode::HardBreak => {
                let node = nodes.remove(index);
                place(&mut moved, vec![node]);
            }
            RichNode::Text(text) => {
                let kept = match edge {
                    Edge::Leading => text.trim_start(),
                    Edge::Trailing => text.trim_end(),
                };
                if kept.len() == text.len() {
                    break;
                }
                let (kept, outside) = match edge {
                    Edge::Leading => (
                        kept.to_string(),
                        text[..text.len() - kept.len()].to_string(),
                    ),
                    Edge::Trailing => (kept.to_string(), text[kept.len()..].to_string()),
                };
                place(&mut moved, vec![RichNode::Text(outside)]);
                if kept.is_empty() {
                    nodes.remove(index);
                } else {
                    *text = kept;
                    break;
                }
            }
            node => {
                let Some(children) = wrapper_children_mut(node) else {
                    break;
                };
                let inner = take_edge(children, edge);
                if inner.is_empty() {
                    break;
                }
                place(&mut moved, inner);
                // A wrapper that still renders something ends the edge.
                if !is_blank(node) {
                    break;
                }
            }
        }
    }
    moved
}
