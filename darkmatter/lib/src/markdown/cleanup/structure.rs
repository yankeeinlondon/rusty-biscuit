//! The block model cleanup plans its edits against.
//!
//! One counted parse of the [`ParseView`] yields events whose ranges are
//! mapped back to source offsets; [`Model`] then records the containers,
//! lists, items, inline runs, tables, and fences that the rules edit. The same
//! parse feeds the fidelity comparison of the input, so planning costs no
//! extra parse.

use std::ops::Range;

use pulldown_cmark::{Alignment, CodeBlockKind, Event, Tag, TagEnd};

use super::protect::{Lines, ParseView, content_start, depth_base};

/// A link reference definition, which the event stream does not carry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct Definition {
    pub label: String,
    pub destination: String,
    pub title: Option<String>,
    /// Source offset of the definition.
    pub at: usize,
}

/// The parse of a [`ParseView`], in source coordinates.
pub(super) struct Parsed<'v> {
    pub events: Vec<(Event<'v>, Range<usize>)>,
    pub definitions: Vec<Definition>,
}

/// Parses `view` once (a counted cleanup parse) and maps every range to source
/// offsets.
pub(super) fn parse(view: &ParseView) -> Parsed<'_> {
    let parser = super::cleanup_parser(&view.text);
    let mut definitions: Vec<Definition> = parser
        .reference_definitions()
        .iter()
        .map(|(label, definition)| Definition {
            label: label.to_string(),
            destination: definition.dest.to_string(),
            title: definition.title.as_ref().map(ToString::to_string),
            at: view.start_to_source(definition.span.start),
        })
        .collect();
    definitions.sort_by(|left, right| left.label.cmp(&right.label));
    let events = parser
        .into_offset_iter()
        .map(|(event, range)| (event, view.range_to_source(&range)))
        .collect();
    Parsed { events, definitions }
}

/// The kind of a block, as far as the rules care.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum BlockKind {
    Paragraph,
    List { ordered: bool, start: Option<u64>, first_item_empty: bool },
    FencedCode,
    IndentedCode,
    BlockQuote,
    Heading { setext: bool },
    Table,
    Html,
    Rule,
    Other,
}

/// One child block of a container.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct Block {
    pub kind: BlockKind,
    pub range: Range<usize>,
}

/// What directly contains a list.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Container {
    Document,
    BlockQuote,
    Item,
    Other,
}

#[derive(Debug, Clone)]
pub(super) struct ListModel {
    pub range: Range<usize>,
    pub start: Option<u64>,
    /// Block-quote depth of the list's content.
    pub depth: usize,
    pub container: Container,
    /// The item this list is a direct child of.
    pub parent_item: Option<usize>,
    pub items: Vec<usize>,
    /// Sibling blocks in the containing block.
    pub previous: Option<Block>,
    pub next: Option<Block>,
}

#[derive(Debug, Clone)]
pub(super) struct ItemModel {
    pub range: Range<usize>,
    pub list: usize,
    /// The marker bytes: one bullet character, or digits plus `.`/`)`.
    pub marker: Range<usize>,
    /// Spaces between the marker and the first content; `None` when the first
    /// line is empty after the marker or the spacing is not plain spaces.
    pub padding: Option<usize>,
    /// Column of the marker relative to the list's depth base; `None` when a
    /// tab or a lazy line makes it ambiguous.
    pub marker_column: Option<usize>,
    pub children: Vec<Block>,
}

impl ItemModel {
    /// The content column relative to the depth base.
    pub(super) fn content_column(&self) -> Option<usize> {
        let marker_column = self.marker_column?;
        let width = self.marker.len();
        // Five or more spaces start indented code: the content column is one
        // past the marker.
        let padding = match self.padding {
            Some(padding) if padding <= 4 => padding,
            _ => 1,
        };
        Some(marker_column + width + padding)
    }
}

/// A run of inline content: a paragraph, or the implicit paragraph of a tight
/// list item.
#[derive(Debug, Clone)]
pub(super) struct InlineRun {
    /// Indices into the events, exclusive of the paragraph tags.
    pub events: Range<usize>,
    pub range: Range<usize>,
}

#[derive(Debug, Clone)]
pub(super) struct RowModel {
    pub range: Range<usize>,
    pub cells: Vec<Range<usize>>,
}

#[derive(Debug, Clone)]
pub(super) struct TableModel {
    pub range: Range<usize>,
    pub alignments: Vec<Alignment>,
    pub rows: Vec<RowModel>,
}

#[derive(Debug, Clone, Copy)]
pub(super) struct FenceModel {
    /// Offset of the opening fence's first fence character.
    pub start: usize,
    pub info_empty: bool,
}

/// Everything the rules and the comparison need to know about a parse.
#[derive(Debug, Default)]
pub(super) struct Model {
    pub lists: Vec<ListModel>,
    pub items: Vec<ItemModel>,
    pub runs: Vec<InlineRun>,
    pub tables: Vec<TableModel>,
    pub fences: Vec<FenceModel>,
    /// Every fenced and indented code block's range.
    pub code_blocks: Vec<Range<usize>>,
    /// Fenced code blocks with no closing fence: they run to the end of their
    /// container, so a blank line added after one becomes code.
    pub unclosed_fences: Vec<Range<usize>>,
}

#[derive(Debug)]
enum FrameKind {
    Document,
    BlockQuote,
    List(usize),
    Item(usize),
    Paragraph(usize),
    Leaf,
    OtherContainer,
}

#[derive(Debug)]
struct Frame {
    kind: FrameKind,
    children: Vec<Block>,
    /// The list index of each child that is a list, by child position.
    child_lists: Vec<Option<usize>>,
    /// The implicit paragraph currently open in a tight item.
    implicit: Option<usize>,
}

impl Frame {
    fn new(kind: FrameKind) -> Self {
        Self { kind, children: Vec::new(), child_lists: Vec::new(), implicit: None }
    }
}

impl Model {
    pub(super) fn build(source: &str, lines: &Lines, events: &[(Event<'_>, Range<usize>)]) -> Self {
        let mut model = Model::default();
        let mut stack = vec![Frame::new(FrameKind::Document)];
        let mut depth = 0usize;
        let mut table: Option<usize> = None;

        for (index, (event, range)) in events.iter().enumerate() {
            match event {
                Event::Start(tag) if is_inline_tag(tag) => {
                    model.extend_implicit(&mut stack, index, range);
                }
                Event::End(tag_end) if is_inline_end(*tag_end) => {
                    model.extend_implicit(&mut stack, index, range);
                }
                Event::Start(Tag::TableHead | Tag::TableRow) if table.is_some() => {
                    if let Some(table) = table {
                        model.tables[table].rows.push(RowModel { range: range.clone(), cells: Vec::new() });
                    }
                    stack.push(Frame::new(FrameKind::Leaf));
                }
                Event::Start(Tag::TableCell) if table.is_some() => {
                    if let Some(row) = table.and_then(|table| model.tables[table].rows.last_mut()) {
                        row.cells.push(range.clone());
                    }
                    stack.push(Frame::new(FrameKind::Leaf));
                }
                Event::Start(tag) => {
                    close_implicit(&mut stack);
                    let kind = block_kind(source, tag, range, events, index);
                    let list_index = matches!(tag, Tag::List(_)).then_some(model.lists.len());
                    model.push_child(&mut stack, Block { kind, range: range.clone() }, list_index);
                    let frame = match tag {
                        Tag::BlockQuote(_) => {
                            depth += 1;
                            FrameKind::BlockQuote
                        }
                        Tag::List(start) => {
                            let (container, parent_item) = match stack.last().map(|frame| &frame.kind) {
                                Some(FrameKind::Document) => (Container::Document, None),
                                Some(FrameKind::BlockQuote) => (Container::BlockQuote, None),
                                Some(FrameKind::Item(item)) => (Container::Item, Some(*item)),
                                _ => (Container::Other, None),
                            };
                            let parent = stack.last().expect("the document frame is never popped");
                            let previous = parent.children.len().checked_sub(2).map(|i| parent.children[i].clone());
                            model.lists.push(ListModel {
                                range: range.clone(),
                                start: *start,
                                depth,
                                container,
                                parent_item,
                                items: Vec::new(),
                                previous,
                                next: None,
                            });
                            FrameKind::List(model.lists.len() - 1)
                        }
                        Tag::Item => {
                            let list = match stack.last().map(|frame| &frame.kind) {
                                Some(FrameKind::List(list)) => *list,
                                _ => usize::MAX,
                            };
                            let item = item_model(source, lines, range, list, depth);
                            model.items.push(item);
                            let item_index = model.items.len() - 1;
                            if let Some(list) = model.lists.get_mut(list) {
                                list.items.push(item_index);
                            }
                            FrameKind::Item(item_index)
                        }
                        Tag::Paragraph => {
                            model.runs.push(InlineRun { events: index + 1..index + 1, range: range.start..range.start });
                            FrameKind::Paragraph(model.runs.len() - 1)
                        }
                        Tag::CodeBlock(kind) => {
                            model.code_blocks.push(range.clone());
                            if matches!(kind, CodeBlockKind::Fenced(_)) && !fence_is_closed(source, lines, range) {
                                model.unclosed_fences.push(range.clone());
                            }
                            if let CodeBlockKind::Fenced(info) = kind {
                                model.fences.push(FenceModel { start: range.start, info_empty: info.is_empty() });
                            }
                            FrameKind::Leaf
                        }
                        Tag::Table(alignments) => {
                            model.tables.push(TableModel {
                                range: range.clone(),
                                alignments: alignments.clone(),
                                rows: Vec::new(),
                            });
                            table = Some(model.tables.len() - 1);
                            FrameKind::Leaf
                        }
                        Tag::FootnoteDefinition(_) => FrameKind::OtherContainer,
                        _ => FrameKind::Leaf,
                    };
                    stack.push(Frame::new(frame));
                }
                Event::End(tag_end) => {
                    close_implicit(&mut stack);
                    if let Some(frame) = stack.pop() {
                        match frame.kind {
                            FrameKind::BlockQuote => depth -= 1,
                            FrameKind::Item(item) => model.items[item].children = frame.children,
                            FrameKind::Paragraph(run) => {
                                let run = &mut model.runs[run];
                                run.events = run.events.start..index;
                                run.range = inline_range(events, run.events.clone()).unwrap_or(range.start..range.start);
                            }
                            _ => {}
                        }
                    }
                    if matches!(tag_end, TagEnd::Table) {
                        table = None;
                    }
                }
                _ if table.is_some() => {}
                _ => {
                    match event {
                        Event::Rule => {
                            close_implicit(&mut stack);
                            model.push_child(&mut stack, Block { kind: BlockKind::Rule, range: range.clone() }, None);
                        }
                        Event::TaskListMarker(_) | Event::Html(_) => {}
                        _ => model.extend_implicit(&mut stack, index, range),
                    }
                }
            }
        }
        for run in &mut model.runs {
            if !run.range.is_empty() {
                run.range.start = content_start(source, run.range.start, lines.get(lines.index_of(run.range.start)).start);
            }
        }
        model
    }

    fn push_child(&mut self, stack: &mut [Frame], block: Block, list: Option<usize>) {
        let frame = stack.last_mut().expect("the document frame is never popped");
        if let Some(Some(previous_list)) = frame.child_lists.last() {
            self.lists[*previous_list].next = Some(block.clone());
        }
        frame.children.push(block);
        frame.child_lists.push(list);
    }

    /// Adds an inline event to the open implicit paragraph of a tight item,
    /// starting one when needed. Inline events elsewhere belong to an explicit
    /// paragraph, heading, or table cell and are not runs of their own.
    fn extend_implicit(&mut self, stack: &mut [Frame], index: usize, range: &Range<usize>) {
        let Some(frame) = stack.last_mut() else { return };
        if !matches!(frame.kind, FrameKind::Item(_)) {
            return;
        }
        match frame.implicit {
            Some(run) => {
                let run = &mut self.runs[run];
                run.events.end = index + 1;
                run.range.end = run.range.end.max(range.end);
            }
            None => {
                self.runs.push(InlineRun { events: index..index + 1, range: range.clone() });
                frame.implicit = Some(self.runs.len() - 1);
                let block = Block { kind: BlockKind::Paragraph, range: range.clone() };
                if let Some(Some(previous_list)) = frame.child_lists.last() {
                    self.lists[*previous_list].next = Some(block.clone());
                }
                frame.children.push(block);
                frame.child_lists.push(None);
            }
        }
        if let (Some(run), Some(block)) = (frame.implicit, frame.children.last_mut()) {
            block.range = self.runs[run].range.clone();
        }
    }
}

fn close_implicit(stack: &mut [Frame]) {
    if let Some(frame) = stack.last_mut() {
        frame.implicit = None;
    }
}

fn is_inline_tag(tag: &Tag<'_>) -> bool {
    matches!(
        tag,
        Tag::Emphasis
            | Tag::Strong
            | Tag::Strikethrough
            | Tag::Superscript
            | Tag::Subscript
            | Tag::Link { .. }
            | Tag::Image { .. }
    )
}

fn is_inline_end(tag_end: TagEnd) -> bool {
    matches!(
        tag_end,
        TagEnd::Emphasis
            | TagEnd::Strong
            | TagEnd::Strikethrough
            | TagEnd::Superscript
            | TagEnd::Subscript
            | TagEnd::Link
            | TagEnd::Image
    )
}

fn inline_range(events: &[(Event<'_>, Range<usize>)], indices: Range<usize>) -> Option<Range<usize>> {
    let start = events[indices.clone()]
        .iter()
        .find(|(event, _)| !matches!(event, Event::TaskListMarker(_)))?
        .1
        .start;
    let end = events[indices].iter().map(|(_, range)| range.end).max()?;
    Some(start..end)
}

fn block_kind(
    source: &str,
    tag: &Tag<'_>,
    range: &Range<usize>,
    events: &[(Event<'_>, Range<usize>)],
    index: usize,
) -> BlockKind {
    match tag {
        Tag::Paragraph => BlockKind::Paragraph,
        Tag::List(start) => {
            // The first item is empty when its end follows its start directly.
            let first_item_empty = matches!(
                (events.get(index + 1), events.get(index + 2)),
                (Some((Event::Start(Tag::Item), _)), Some((Event::End(TagEnd::Item), _)))
            );
            BlockKind::List { ordered: start.is_some(), start: *start, first_item_empty }
        }
        Tag::CodeBlock(CodeBlockKind::Fenced(_)) => BlockKind::FencedCode,
        Tag::CodeBlock(CodeBlockKind::Indented) => BlockKind::IndentedCode,
        Tag::BlockQuote(_) => BlockKind::BlockQuote,
        Tag::Heading { .. } => {
            let text = source[range.clone()].trim_end_matches(['\n', '\r']);
            BlockKind::Heading { setext: text.contains(['\n', '\r']) }
        }
        Tag::Table(_) => BlockKind::Table,
        Tag::HtmlBlock => BlockKind::Html,
        _ => BlockKind::Other,
    }
}

/// Reads an item's marker from source. Nested item ranges start at the
/// parent's content column, or, after tab indentation, at the end of the line
/// before, so the marker is found past leading whitespace and line breaks.
fn item_model(source: &str, lines: &Lines, range: &Range<usize>, list: usize, depth: usize) -> ItemModel {
    let bytes = source.as_bytes();
    let mut start = range.start;
    while start < range.end && matches!(bytes[start], b' ' | b'\t' | b'\n' | b'\r') {
        start += 1;
    }
    let mut end = start;
    if end < bytes.len() && matches!(bytes[end], b'-' | b'*' | b'+') {
        end += 1;
    } else {
        while end < bytes.len() && bytes[end].is_ascii_digit() {
            end += 1;
        }
        if end < bytes.len() && matches!(bytes[end], b'.' | b')') {
            end += 1;
        }
    }
    let line = lines.get(lines.index_of(start));
    let mut cursor = end;
    while cursor < line.end && bytes[cursor] == b' ' {
        cursor += 1;
    }
    let padding = if cursor >= line.end || bytes[cursor] == b'\t' { None } else { Some(cursor - end) };
    let marker_column = depth_base(source, line, depth).and_then(|base| {
        let leading = source.get(base..start)?;
        (!leading.contains('\t') && base <= start).then_some(start - base)
    });
    ItemModel { range: range.clone(), list, marker: start..end, padding, marker_column, children: Vec::new() }
}

/// Whether a fenced code block's last line is a closing fence.
fn fence_is_closed(source: &str, lines: &Lines, range: &Range<usize>) -> bool {
    let first = lines.index_of(range.start);
    let last = lines.index_of(range.end.saturating_sub(1).max(range.start));
    if last == first {
        return false;
    }
    let opener = source[range.start..].trim_start_matches([' ', '>']);
    let Some(fence_char) = opener.chars().next().filter(|c| matches!(c, '`' | '~')) else {
        return false;
    };
    let line = lines.get(last);
    let text = source[line.start..line.end].trim_start_matches([' ', '\t', '>']).trim_end();
    text.len() >= 3 && text.chars().all(|c| c == fence_char)
}
