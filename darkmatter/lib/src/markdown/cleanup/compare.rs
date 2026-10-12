//! The fidelity comparison: is a cleanup candidate equivalent to its input?
//!
//! Both texts are read the same way: the same protection policy, the same
//! parser options, and the same normalization. A [`Snapshot`] holds the
//! normalized structural and inline event sequence plus, compared separately so
//! masking cannot hide them, the exact protected lines (bytes, neighboring
//! blank state, position in the event sequence, and container placement),
//! expression spellings, reference definitions, list marker numbers, code-block
//! bytes and terminators, and fence info strings.
//!
//! The only allowances are the ones the resolved rules authorize: soft breaks
//! become separators under `unwrap`/`fixed-width`, an unlabeled fence may gain
//! the `default-language`, ordered numbers after the first may be renumbered
//! sequentially under `ordered-lists`, and a list may change between tight and
//! loose only when an explicitly selected padding rule reaches it.

use std::collections::BTreeSet;
use std::ops::Range;

use pulldown_cmark::{CodeBlockKind, Event, Tag, TagEnd};

use super::protect::{Lines, Protection, depth_base, is_blank_line};
use super::rules::ResolvedMarkdownRules;
use super::structure::{Definition, Model};
use super::transform::join_separator;

/// Stands for a soft break inside coalesced text until its neighbors are known.
/// The parser never emits NUL (it replaces it with U+FFFD).
const SOFT_BREAK: char = '\0';

#[derive(Debug, Clone, PartialEq, Eq)]
struct Token {
    key: String,
    at: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ProtectedLine {
    bytes: String,
    blank_before: bool,
    blank_after: bool,
    anchor: usize,
    placement: String,
    at: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct CodeBlock {
    info: Option<String>,
    text: String,
    terminators: Vec<String>,
    at: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct OrderedList {
    numbers: Vec<u64>,
    at: usize,
}

/// One side of a fidelity comparison.
#[derive(Debug, Clone)]
pub(super) struct Snapshot {
    tokens: Vec<Token>,
    protected: Vec<ProtectedLine>,
    expressions: Vec<(String, usize)>,
    definitions: Vec<Definition>,
    code_blocks: Vec<CodeBlock>,
    ordered_lists: Vec<OrderedList>,
    len: usize,
}

/// Where and how a candidate first differs from its input.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct Divergence {
    /// Byte offset in the input of the first affected block (the nearest input
    /// boundary, or the end, when the difference is output-only).
    pub offset: usize,
    pub detail: String,
}

/// The lists whose tight/loose state an explicitly selected padding rule may
/// change: any list with siblings under `list-item-padding`, and any list with
/// an item holding more than one block under `list-interior-padding`.
pub(super) fn tightness_exempt_lists(model: &Model, rules: &ResolvedMarkdownRules) -> BTreeSet<usize> {
    model
        .lists
        .iter()
        .enumerate()
        .filter(|(_, list)| {
            (rules.permits_item_padding_change() && list.items.len() > 1)
                || (rules.permits_interior_padding_change()
                    && list.items.iter().any(|item| model.items[*item].children.len() > 1))
        })
        .map(|(index, _)| index)
        .collect()
}

#[derive(Debug)]
enum Frame {
    List(usize),
    Item { list: usize },
    SkippedParagraph,
    Other,
}

impl Snapshot {
    /// Builds the normalized view of one side.
    #[allow(clippy::too_many_arguments)]
    pub(super) fn new(
        source: &str,
        lines: &Lines,
        protection: &Protection,
        events: &[(Event<'_>, Range<usize>)],
        definitions: &[Definition],
        model: &Model,
        rules: &ResolvedMarkdownRules,
        exempt: &BTreeSet<usize>,
    ) -> Self {
        let joins = rules.joins_soft_breaks();
        let mut tokens: Vec<Token> = Vec::new();
        let mut code_blocks: Vec<CodeBlock> = Vec::new();
        let mut ordered_lists: Vec<OrderedList> = Vec::new();
        let mut stack: Vec<Frame> = Vec::new();
        let mut text = String::new();
        let mut text_at = 0usize;
        let mut in_code: Option<usize> = None;
        let mut list_count = 0usize;
        let mut item_count = 0usize;
        // The token count before each event that starts something, used to
        // anchor protected lines.
        let mut anchors: Vec<(usize, usize)> = Vec::new();
        let mut placements: Vec<(usize, Vec<Container>)> = Vec::new();
        let mut open: Vec<Container> = Vec::new();

        let flush = |text: &mut String, tokens: &mut Vec<Token>, at: usize| {
            if !text.is_empty() {
                tokens.push(Token { key: format!("Text({:?})", resolve_soft_breaks(text)), at });
                text.clear();
            }
        };

        for (event, range) in events {
            if !matches!(event, Event::End(_)) {
                anchors.push((range.start, tokens.len() + usize::from(!text.is_empty())));
            }
            if let Some(code) = in_code {
                match event {
                    Event::Text(content) => {
                        code_blocks[code].text.push_str(content);
                        continue;
                    }
                    Event::End(TagEnd::CodeBlock) => in_code = None,
                    _ => {}
                }
            }
            match event {
                Event::Text(content) => {
                    if text.is_empty() {
                        text_at = range.start;
                    }
                    text.push_str(content);
                }
                Event::SoftBreak if joins => {
                    if text.is_empty() {
                        text_at = range.start;
                    }
                    text.push(SOFT_BREAK);
                }
                _ => {
                    flush(&mut text, &mut tokens, text_at);
                    let key = match event {
                        Event::Start(Tag::List(start)) => {
                            let ordinal = list_count;
                            list_count += 1;
                            stack.push(Frame::List(ordinal));
                            if start.is_some() {
                                ordered_lists.push(OrderedList { numbers: Vec::new(), at: range.start });
                            }
                            Some(format!("List(ordered: {})", start.is_some()))
                        }
                        Event::Start(Tag::Item) => {
                            let list = match stack.last() {
                                Some(Frame::List(list)) => *list,
                                _ => usize::MAX,
                            };
                            let item_index = item_count;
                            item_count += 1;
                            stack.push(Frame::Item { list });
                            let item = model.items.get(item_index);
                            open.push(Container::Item(item.and_then(super::structure::ItemModel::content_column)));
                            let marker = item.map_or("", |item| &source[item.marker.clone()]);
                            let bullet = marker.chars().next().filter(|c| matches!(c, '-' | '*' | '+'));
                            match bullet {
                                Some(bullet) => Some(format!("Item({bullet})")),
                                None => {
                                    if let (Some(list), Ok(number)) =
                                        (ordered_lists.last_mut(), marker[..marker.len().saturating_sub(1)].parse())
                                    {
                                        list.numbers.push(number);
                                    }
                                    Some(format!("Item({})", &marker[marker.len().saturating_sub(1)..]))
                                }
                            }
                        }
                        Event::Start(Tag::Paragraph) => {
                            if let Some(Frame::Item { list, .. }) = stack.last()
                                && exempt.contains(list)
                            {
                                stack.push(Frame::SkippedParagraph);
                                None
                            } else {
                                stack.push(Frame::Other);
                                Some("Paragraph".to_string())
                            }
                        }
                        Event::Start(Tag::BlockQuote(kind)) => {
                            stack.push(Frame::Other);
                            open.push(Container::Quote);
                            Some(format!("BlockQuote({kind:?})"))
                        }
                        Event::Start(Tag::CodeBlock(kind)) => {
                            stack.push(Frame::Other);
                            in_code = Some(code_blocks.len());
                            code_blocks.push(CodeBlock {
                                info: match kind {
                                    CodeBlockKind::Fenced(info) => Some(info.to_string()),
                                    CodeBlockKind::Indented => None,
                                },
                                text: String::new(),
                                terminators: terminators(source, lines, range),
                                at: range.start,
                            });
                            Some(format!("CodeBlock(fenced: {})", matches!(kind, CodeBlockKind::Fenced(_))))
                        }
                        Event::Start(tag) => {
                            stack.push(Frame::Other);
                            Some(tag_key(tag))
                        }
                        Event::End(tag_end) => match stack.pop() {
                            Some(Frame::SkippedParagraph) => None,
                            frame => {
                                if matches!(frame, Some(Frame::Item { .. })) || matches!(tag_end, TagEnd::BlockQuote(_)) {
                                    open.pop();
                                }
                                Some(match tag_end {
                                    TagEnd::List(_) => "End(List)".to_string(),
                                    other => format!("End({other:?})"),
                                })
                            }
                        },
                        other => Some(event_key(other)),
                    };
                    if let Some(key) = key {
                        tokens.push(Token { key, at: range.start });
                    }
                }
            }
            if !matches!(event, Event::End(_)) {
                placements.push((range.start, open.clone()));
            }
        }
        flush(&mut text, &mut tokens, text_at);

        let protected = protection
            .lines
            .iter()
            .map(|range| {
                let first = lines.index_of(range.start);
                let last = lines.index_of(range.end.saturating_sub(1).max(range.start));
                let anchor = anchors
                    .iter()
                    .find(|(start, _)| *start >= range.start)
                    .map_or(tokens.len(), |(_, anchor)| *anchor);
                let containers = placements
                    .iter()
                    .rev()
                    .find(|(start, _)| *start < range.start)
                    .map(|(_, open)| open.as_slice())
                    .unwrap_or(&[]);
                ProtectedLine {
                    bytes: source[range.clone()].to_string(),
                    blank_before: first > 0 && is_blank_line(source, lines.get(first - 1)),
                    blank_after: last + 1 < lines.len() && is_blank_line(source, lines.get(last + 1)),
                    anchor,
                    placement: placement(source, lines, range.start, containers),
                    at: range.start,
                }
            })
            .collect();

        Self {
            tokens,
            protected,
            expressions: protection.inline.iter().map(|span| (source[span.clone()].to_string(), span.start)).collect(),
            definitions: definitions.to_vec(),
            code_blocks,
            ordered_lists,
            len: source.len(),
        }
    }

    /// Compares `self` (the input) with `output`.
    ///
    /// ## Errors
    ///
    /// Returns the earliest [`Divergence`] across every compared aspect.
    pub(super) fn compare(&self, output: &Snapshot, rules: &ResolvedMarkdownRules) -> Result<(), Divergence> {
        let end = self.len;
        let mut found: Vec<Divergence> = Vec::new();

        if let Some(index) = first_difference(&self.tokens, &output.tokens, |a, b| a.key == b.key) {
            let detail = match (self.tokens.get(index), output.tokens.get(index)) {
                (Some(input), Some(candidate)) => format!("{} became {}", short(&input.key), short(&candidate.key)),
                (Some(input), None) => format!("{} was removed", short(&input.key)),
                (None, Some(candidate)) => format!("{} was added", short(&candidate.key)),
                (None, None) => String::new(),
            };
            found.push(Divergence { offset: at(&self.tokens, index, end, |token| token.at), detail });
        }
        if let Some(index) = first_difference(&self.protected, &output.protected, |a, b| {
            (&a.bytes, a.blank_before, a.blank_after, a.anchor, &a.placement)
                == (&b.bytes, b.blank_before, b.blank_after, b.anchor, &b.placement)
        }) {
            found.push(Divergence {
                offset: at(&self.protected, index, end, |line| line.at),
                detail: "a directive line moved, changed, or gained a blank neighbor".to_string(),
            });
        }
        if let Some(index) = first_difference(&self.expressions, &output.expressions, |a, b| a.0 == b.0) {
            found.push(Divergence {
                offset: at(&self.expressions, index, end, |expression| expression.1),
                detail: "an expression's spelling changed".to_string(),
            });
        }
        if let Some(index) = first_difference(&self.definitions, &output.definitions, |a, b| {
            (&a.label, &a.destination, &a.title) == (&b.label, &b.destination, &b.title)
        }) {
            found.push(Divergence {
                offset: at(&self.definitions, index, end, |definition| definition.at),
                detail: "a link reference definition changed".to_string(),
            });
        }
        let language = rules.default_language.value.as_str();
        if let Some(index) = first_difference(&self.code_blocks, &output.code_blocks, |a, b| {
            let info_kept = a.info == b.info
                || (!language.is_empty() && a.info.as_deref() == Some("") && b.info.as_deref() == Some(language));
            info_kept && a.text == b.text && a.terminators == b.terminators
        }) {
            found.push(Divergence {
                offset: at(&self.code_blocks, index, end, |block| block.at),
                detail: "a code block's content, terminators, or info string changed".to_string(),
            });
        }
        if let Some(index) = first_difference(&self.ordered_lists, &output.ordered_lists, |a, b| {
            a.numbers == b.numbers
                || (rules.ordered_lists.value
                    && a.numbers.len() == b.numbers.len()
                    && a.numbers.first() == b.numbers.first()
                    && b.numbers.iter().enumerate().all(|(index, number)| *number == b.numbers[0] + index as u64))
        }) {
            found.push(Divergence {
                offset: at(&self.ordered_lists, index, end, |list| list.at),
                detail: "an ordered list's numbers changed".to_string(),
            });
        }

        match found.into_iter().min_by_key(|divergence| divergence.offset) {
            Some(divergence) => Err(divergence),
            None => Ok(()),
        }
    }
}

/// A comparison key for a tag. Spelled out field by field because the
/// parser's string type prints its storage variant under `Debug`, which can
/// differ for equal values.
fn tag_key(tag: &Tag<'_>) -> String {
    match tag {
        Tag::Heading { level, id, classes, attrs } => format!(
            "Heading({level:?}, id: {:?}, classes: {:?}, attrs: {:?})",
            id.as_deref(),
            classes.iter().map(AsRef::as_ref).collect::<Vec<&str>>(),
            attrs.iter().map(|(key, value)| (key.as_ref(), value.as_deref())).collect::<Vec<_>>(),
        ),
        Tag::Link { link_type, dest_url, title, id } => {
            format!("Link({link_type:?}, {:?}, title {:?}, id {:?})", dest_url.as_ref(), title.as_ref(), id.as_ref())
        }
        Tag::Image { link_type, dest_url, title, id } => {
            format!("Image({link_type:?}, {:?}, title {:?}, id {:?})", dest_url.as_ref(), title.as_ref(), id.as_ref())
        }
        Tag::FootnoteDefinition(label) => format!("FootnoteDefinition({:?})", label.as_ref()),
        other => format!("{other:?}"),
    }
}

/// A comparison key for a non-text, non-tag event.
fn event_key(event: &Event<'_>) -> String {
    match event {
        Event::Code(code) => format!("Code({:?})", code.as_ref()),
        Event::InlineMath(math) => format!("InlineMath({:?})", math.as_ref()),
        Event::DisplayMath(math) => format!("DisplayMath({:?})", math.as_ref()),
        Event::Html(html) => format!("Html({:?})", html.as_ref()),
        Event::InlineHtml(html) => format!("InlineHtml({:?})", html.as_ref()),
        Event::FootnoteReference(label) => format!("FootnoteReference({:?})", label.as_ref()),
        Event::Text(text) => format!("Text({:?})", text.as_ref()),
        other => format!("{other:?}"),
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Container {
    Quote,
    Item(Option<usize>),
}

/// How many of the `containers` open at a protected line its prefix actually
/// satisfies, with their kinds: the container compose will splice it into.
fn placement(source: &str, lines: &Lines, start: usize, containers: &[Container]) -> String {
    let line = lines.get(lines.index_of(start));
    let mut depth = 0usize;
    let mut satisfied = 0usize;
    // Item columns are measured from the content start of the innermost quote.
    let mut quote_base = line.start;
    for container in containers {
        match container {
            Container::Quote => match depth_base(source, line, depth + 1) {
                Some(next) => {
                    depth += 1;
                    quote_base = next;
                }
                None => break,
            },
            Container::Item(column) => {
                let indent = source[quote_base..line.end].bytes().take_while(|byte| *byte == b' ').count();
                if !column.is_some_and(|column| indent >= column) {
                    break;
                }
            }
        }
        satisfied += 1;
    }
    format!("{:?}", &containers[..satisfied])
}

/// The terminator of every line a code block spans.
fn terminators(source: &str, lines: &Lines, range: &Range<usize>) -> Vec<String> {
    if range.is_empty() {
        return Vec::new();
    }
    let first = lines.index_of(range.start);
    let last = lines.index_of(range.end - 1);
    (first..=last).map(|index| lines.terminator(source, index).to_string()).collect()
}

/// Replaces each soft-break placeholder with the separator a join would use.
fn resolve_soft_breaks(text: &str) -> String {
    if !text.contains(SOFT_BREAK) {
        return text.to_string();
    }
    let chars: Vec<char> = text.chars().collect();
    let mut resolved = String::with_capacity(text.len());
    for (index, c) in chars.iter().enumerate() {
        if *c == SOFT_BREAK {
            let previous = chars[..index].iter().rev().find(|c| **c != SOFT_BREAK).copied();
            let next = chars[index + 1..].iter().find(|c| **c != SOFT_BREAK).copied();
            resolved.push_str(join_separator(previous, next));
        } else {
            resolved.push(*c);
        }
    }
    resolved
}

fn first_difference<T>(input: &[T], output: &[T], same: impl Fn(&T, &T) -> bool) -> Option<usize> {
    let shared = input.len().min(output.len());
    (0..shared)
        .find(|index| !same(&input[*index], &output[*index]))
        .or((input.len() != output.len()).then_some(shared))
}

fn at<T>(items: &[T], index: usize, end: usize, offset: impl Fn(&T) -> usize) -> usize {
    items.get(index).map_or(end, offset)
}

fn short(key: &str) -> String {
    const LIMIT: usize = 60;
    if key.chars().count() <= LIMIT {
        key.to_string()
    } else {
        format!("{}…", key.chars().take(LIMIT).collect::<String>())
    }
}
