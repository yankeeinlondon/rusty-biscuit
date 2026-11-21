//! The node-range YAML edit engine behind both text-preserving editors: the
//! Markdown frontmatter path editor ([`super::edit_frontmatter_paths`]) and the
//! plain-YAML editor ([`crate::yaml_edit`]).
//!
//! The engine edits one block-mapping YAML region of a larger text. An
//! [`Envelope`] says where that region is (a frontmatter block, or a whole
//! file) and how the edited text reloads, so neither caller fabricates the
//! other's wrapper. Only a path's own node is replaced, a new key goes at the
//! end of its parent, a new sequence item after the last one, and a removed
//! node takes its own lines; every other byte is kept.
//!
//! New collections are laid out by [`Layout`]: an existing block keeps its
//! measured child offset, a new block copies the nearest authored block of
//! the same kind, and only where the text has none does the destination's
//! resolved preference apply. User-facing rules:
//! `darkmatter/docs/topics/yaml-editing.md#indentation-of-new-values`.

use super::path_edit::{PathEdit, PathEditRefusal};
use super::write::{
    FrontmatterPathSegment, apply_terminators, column_of, dotted_path, locate_all_nodes,
    mapping_colon, new_block_terminator, preceding_terminator, validate_block_mapping,
    yaml_comment_start,
};
use crate::markdown::FrontmatterMap;
use biscuit_file::Indentation;
use serde::Serialize;
use serde_json::Value;
use simplified_schema::completion::PathStep;
use simplified_schema::simplified::locate_frontmatter_value;
use simplified_schema::{SchemaValueEntry, SchemaValueKind, SchemaValueNode};
use std::cell::OnceCell;
use std::cmp::{Ordering, Reverse};
use std::num::NonZeroU8;
use std::ops::Range;

/// Where the edited YAML sits in a text, and how that text reloads.
pub(crate) trait Envelope {
    /// The byte range of the YAML block mapping in `document`, or `None` when
    /// `document` holds no YAML region yet.
    fn yaml_span(&self, document: &str) -> Result<Option<Range<usize>>, PathEditRefusal>;

    /// `document` with a new YAML region holding `yaml` (already terminated).
    /// Called only when [`Self::yaml_span`] found none.
    fn with_new_block(&self, document: &str, yaml: &str) -> String;

    /// The values of `text` as its own loader reads them. An orphaned alias is
    /// [`PathEditRefusal::WouldChangeUnnamed`]; any other failure is
    /// [`PathEditRefusal::UnsupportedForm`].
    fn reload(&self, text: &str) -> Result<FrontmatterMap, PathEditRefusal>;
}

/// How the engine lays out the collections it renders.
///
/// `fallback` is asked at most once, and only when a rendered collection has
/// no authored example in the text: a save whose edits all find one, or
/// render nothing, never resolves the destination's preference.
pub(crate) struct Layout<'f> {
    fallback: &'f dyn Fn() -> Indentation,
    resolved: OnceCell<usize>,
    /// Whether the text's own collections are examples. A document generated
    /// from a value has none of its author's, so its edits use `fallback`.
    infer_local: bool,
}

impl<'f> Layout<'f> {
    pub(crate) fn new(fallback: &'f dyn Fn() -> Indentation, infer_local: bool) -> Self {
        Self { fallback, resolved: OnceCell::new(), infer_local }
    }

    fn fallback_width(&self) -> usize {
        *self.resolved.get_or_init(|| usize::from((self.fallback)().width.get()))
    }
}

/// The layout of the editors' default wrappers: local examples first, then
/// [`Indentation::default`].
pub(crate) fn default_layout() -> Layout<'static> {
    Layout::new(&Indentation::default, true)
}

/// Applies `edits` to `source` in [`application_order`], each against the text
/// the previous one left. The result loads after every edit, but is not yet
/// compared with `expected`; see [`verify_edited`].
pub(crate) fn apply_edits(
    envelope: &impl Envelope,
    source: &str,
    expected: &FrontmatterMap,
    edits: &[PathEdit<'_>],
    layout: &Layout<'_>,
) -> Result<String, PathEditRefusal> {
    let mut text = source.to_string();
    for edit in application_order(edits) {
        text = apply_edit(envelope, &text, expected, edit, layout)?;
        // Checked per edit because the next one begins by requiring strict
        // YAML: an alias orphaned here would otherwise read as an unsupported
        // form, and the Markdown fallback would hide it.
        envelope.reload(&text)?;
    }
    Ok(text)
}

/// The authored shape of one node a path passes through, as found in the
/// unedited source.
#[derive(Debug, Clone, Copy)]
pub(crate) struct TracedNode {
    /// Where the node is named: its key for a mapping entry, its content for a
    /// sequence item.
    pub(crate) start: usize,
    pub(crate) alias: bool,
    pub(crate) anchor: bool,
    pub(crate) tag: bool,
    pub(crate) flow: bool,
    /// A mapping or sequence with no children.
    pub(crate) empty_collection: bool,
}

/// The nodes `steps` reaches in `document`'s YAML region `yaml`, outermost
/// first, stopping where the source no longer holds the path. Empty when the
/// top-level key is absent or the region cannot be located.
pub(crate) fn trace_path(document: &str, yaml: Range<usize>, steps: &[PathStep<'_>]) -> Vec<TracedNode> {
    let mut traced = Vec::new();
    let Some((PathStep::Key(top), rest)) = steps.split_first() else {
        return traced;
    };
    let Ok(nodes) = locate_all_nodes(&document[yaml.clone()]) else {
        return traced;
    };
    let Some(node) = nodes.get(*top) else {
        return traced;
    };
    let node_start = yaml.start + node.range.start;
    let node_end = yaml.start + node.range.end;
    let Some(tree) = locate_frontmatter_value(&document[node_start..node_end], node_start) else {
        return traced;
    };
    let SchemaValueKind::Mapping(entries) = &tree.kind else {
        return traced;
    };
    let Some(entry) = entries.iter().find(|entry| entry.key == *top) else {
        return traced;
    };
    let mut current = &entry.value;
    traced.push(traced_node(document, entry.key_span.start, current));
    for step in rest {
        let next = match (&current.kind, step) {
            (SchemaValueKind::Mapping(entries), PathStep::Key(key)) => entries
                .iter()
                .find(|entry| entry.key == *key)
                .map(|entry| (entry.key_span.start, &entry.value)),
            (SchemaValueKind::Sequence(items), PathStep::Index(index)) => {
                items.get(*index).map(|item| (item.span.start, item))
            }
            _ => None,
        };
        let Some((start, node)) = next else {
            break;
        };
        traced.push(traced_node(document, start, node));
        current = node;
    }
    traced
}

fn traced_node(document: &str, start: usize, node: &SchemaValueNode) -> TracedNode {
    let shape = authored_shape(&document[node.span.clone()]);
    let empty_collection = match &node.kind {
        SchemaValueKind::Mapping(entries) => entries.is_empty(),
        SchemaValueKind::Sequence(items) => items.is_empty(),
        SchemaValueKind::Scalar => false,
    };
    TracedNode {
        start,
        alias: shape.alias,
        anchor: shape.anchor,
        tag: shape.tag,
        flow: shape.flow,
        empty_collection,
    }
}

/// Sets first, then removals with higher indexes ahead of lower ones, so each
/// removal's original index still names the same element when its turn comes.
pub(crate) fn application_order<'e, 'a>(edits: &'e [PathEdit<'a>]) -> Vec<&'e PathEdit<'a>> {
    let mut ordered: Vec<&PathEdit> = edits.iter().collect();
    ordered.sort_by(|left, right| match (left, right) {
        (PathEdit::Set(_), PathEdit::Set(_)) => Ordering::Equal,
        (PathEdit::Set(_), PathEdit::Remove(_)) => Ordering::Less,
        (PathEdit::Remove(_), PathEdit::Set(_)) => Ordering::Greater,
        (PathEdit::Remove(left), PathEdit::Remove(right)) => removal_order(left, right),
    });
    ordered
}

/// A total order, because Rust's sort may panic on an inconsistent one.
fn removal_order(left: &[PathStep<'_>], right: &[PathStep<'_>]) -> Ordering {
    for (left, right) in left.iter().zip(right) {
        let order = match (left, right) {
            (PathStep::Index(left), PathStep::Index(right)) => right.cmp(left),
            (PathStep::Key(left), PathStep::Key(right)) => left.cmp(right),
            (PathStep::Key(_), PathStep::Index(_)) => Ordering::Less,
            (PathStep::Index(_), PathStep::Key(_)) => Ordering::Greater,
        };
        if order != Ordering::Equal {
            return order;
        }
    }
    right.len().cmp(&left.len())
}

/// Compares `text`, reloaded, with `expected`.
///
/// ## Errors
///
/// [`PathEditRefusal::WouldChangeUnnamed`] when a value no edit names differs,
/// and [`PathEditRefusal::UnsupportedForm`] when a named one does not read back.
pub(crate) fn verify_edited(
    envelope: &impl Envelope,
    text: &str,
    expected: &FrontmatterMap,
    edits: &[PathEdit<'_>],
) -> Result<(), PathEditRefusal> {
    let reloaded = envelope.reload(text)?;
    match unexpected_difference(expected, &reloaded, edits) {
        Some(Difference { path, named: false }) => Err(would_change(&path)),
        Some(Difference { path, named: true }) => Err(unsupported(format!(
            "`{}` does not read back as the value it was given",
            dotted_path(&path)
        ))),
        None => Ok(()),
    }
}

/// A path where reloaded values differ from the expected ones.
pub(crate) struct Difference {
    pub(crate) path: Vec<FrontmatterPathSegment>,
    /// Whether an edit names the path, or a parent or child of it.
    pub(crate) named: bool,
}

/// The first difference between `expected` and `reloaded`, a value no edit
/// names ahead of any named one.
pub(crate) fn unexpected_difference(
    expected: &FrontmatterMap,
    reloaded: &FrontmatterMap,
    edits: &[PathEdit<'_>],
) -> Option<Difference> {
    let found = differences(expected, reloaded);
    let named: Vec<Vec<FrontmatterPathSegment>> = edits
        .iter()
        .map(|edit| match edit {
            PathEdit::Set(steps) | PathEdit::Remove(steps) => {
                steps.iter().map(segment_of).collect()
            }
        })
        .collect();
    let is_named = |path: &Vec<FrontmatterPathSegment>| {
        named
            .iter()
            .any(|name| path.starts_with(name) || name.starts_with(path))
    };
    if let Some(path) = found.iter().find(|path| !is_named(path)) {
        return Some(Difference { path: path.clone(), named: false });
    }
    found.into_iter().next().map(|path| Difference { path, named: true })
}

pub(crate) fn segment_of(step: &PathStep<'_>) -> FrontmatterPathSegment {
    match step {
        PathStep::Key(key) => FrontmatterPathSegment::Key((*key).to_string()),
        PathStep::Index(index) => FrontmatterPathSegment::Index(*index),
    }
}

pub(crate) fn would_change(path: &[FrontmatterPathSegment]) -> PathEditRefusal {
    PathEditRefusal::WouldChangeUnnamed {
        reason: format!(
            "the edit would change the value of `{}`, which it does not name \
             (an alias there likely reads an anchor in the edited node)",
            dotted_path(path)
        ),
    }
}

pub(crate) fn unsupported(reason: impl Into<String>) -> PathEditRefusal {
    PathEditRefusal::UnsupportedForm {
        reason: reason.into(),
    }
}

/// Every path where `expected` and `actual` differ, as deep as they agree on
/// shape. A sequence of another length differs as a whole.
pub(crate) fn differences(
    expected: &FrontmatterMap,
    actual: &FrontmatterMap,
) -> Vec<Vec<FrontmatterPathSegment>> {
    let mut found = Vec::new();
    let mut path = Vec::new();
    for key in expected.keys().chain(actual.keys().filter(|key| !expected.contains_key(*key))) {
        path.push(FrontmatterPathSegment::Key(key.clone()));
        compare_values(expected.get(key), actual.get(key), &mut path, &mut found);
        path.pop();
    }
    found
}

fn compare_values(
    expected: Option<&Value>,
    actual: Option<&Value>,
    path: &mut Vec<FrontmatterPathSegment>,
    found: &mut Vec<Vec<FrontmatterPathSegment>>,
) {
    match (expected, actual) {
        (Some(Value::Object(expected)), Some(Value::Object(actual))) => {
            for key in expected.keys().chain(actual.keys().filter(|key| !expected.contains_key(*key))) {
                path.push(FrontmatterPathSegment::Key(key.clone()));
                compare_values(expected.get(key), actual.get(key), path, found);
                path.pop();
            }
        }
        (Some(Value::Array(expected)), Some(Value::Array(actual))) if expected.len() == actual.len() => {
            for (index, (expected, actual)) in expected.iter().zip(actual).enumerate() {
                path.push(FrontmatterPathSegment::Index(index));
                compare_values(Some(expected), Some(actual), path, found);
                path.pop();
            }
        }
        (expected, actual) if expected != actual => found.push(path.clone()),
        _ => {}
    }
}

/// A byte range of the current text and what goes there.
struct Splice {
    range: Range<usize>,
    text: String,
}

impl Splice {
    fn insert(at: usize, text: String) -> Self {
        Self { range: at..at, text }
    }

    fn delete(range: Range<usize>) -> Self {
        Self { range, text: String::new() }
    }
}

fn spliced(document: &str, mut splices: Vec<Splice>) -> String {
    let mut text = document.to_string();
    splices.sort_by_key(|splice| std::cmp::Reverse(splice.range.start));
    for splice in splices {
        text.replace_range(splice.range, &splice.text);
    }
    text
}

fn apply_edit(
    envelope: &impl Envelope,
    document: &str,
    expected: &FrontmatterMap,
    edit: &PathEdit<'_>,
    layout: &Layout<'_>,
) -> Result<String, PathEditRefusal> {
    let (PathEdit::Set(steps) | PathEdit::Remove(steps)) = edit;
    let Some(PathStep::Key(top)) = steps.first() else {
        return Err(unsupported("a frontmatter path must start with a key"));
    };
    let creating = matches!(edit, PathEdit::Set(_));
    let Some(yaml_span) = envelope.yaml_span(document)? else {
        if !creating {
            return Err(unsupported("the document has no frontmatter"));
        }
        let newline = new_block_terminator(document);
        let value = expected_at(expected, &steps[..1])?;
        let widths = Widths { layout, examples: &[], at: 0, chain: &[] };
        let entry = entry_lines(top, value, 0, newline, &widths)?;
        return Ok(envelope.with_new_block(document, &entry));
    };
    let yaml = &document[yaml_span.clone()];
    validate_block_mapping(yaml).map_err(|error| unsupported(error.to_string()))?;
    let nodes = locate_all_nodes(yaml).map_err(|error| unsupported(error.to_string()))?;
    let yaml_start = yaml_span.start;
    let yaml_end = yaml_span.end;
    let examples = if layout.infer_local && creating {
        collect_examples(document, yaml_span.clone(), &nodes)
    } else {
        Vec::new()
    };

    let Some(node) = nodes.get(*top) else {
        if !creating {
            return Err(unsupported(format!("`{top}` is not in the frontmatter")));
        }
        let newline = preceding_terminator(document, yaml_end);
        let value = expected_at(expected, &steps[..1])?;
        let widths = Widths { layout, examples: &examples, at: yaml_end, chain: &[] };
        let entry = entry_lines(top, value, 0, newline, &widths)?;
        return Ok(spliced(document, vec![Splice::insert(yaml_end, entry)]));
    };

    let node_start = yaml_start + node.range.start;
    let node_end = yaml_start + node.range.end;
    let tree = locate_frontmatter_value(&document[node_start..node_end], node_start)
        .ok_or_else(|| unsupported(format!("`{top}` has a layout that cannot be located")))?;
    let SchemaValueKind::Mapping(top_entries) = &tree.kind else {
        return Err(unsupported(format!("`{top}` has a layout that cannot be located")));
    };
    let top_entry = top_entries
        .iter()
        .find(|entry| entry.key == *top)
        .ok_or_else(|| unsupported(format!("`{top}` has a layout that cannot be located")))?;

    let splices = match walk(document, top_entry, steps, creating)? {
        Walked::Node(frames) => match edit {
            PathEdit::Set(_) => {
                let value = expected_at(expected, &steps[..frames.len()])?;
                let last = &frames[frames.len() - 1];
                let chain = frame_chain(&frames[..frames.len() - 1]);
                let widths = Widths { layout, examples: &examples, at: last.holder.start(), chain: &chain };
                match last.holder {
                    Holder::Entry(entry) => replace_entry(document, entry, value, &widths)?,
                    Holder::Item(item) => replace_item(document, item, value, &widths)?,
                }
            }
            PathEdit::Remove(_) => remove_node(document, &frames)?,
        },
        Walked::MissingKey { frames, mapping, key } => {
            let SchemaValueKind::Mapping(entries) = &mapping.kind else {
                return Err(unsupported("a new key's parent is not a mapping"));
            };
            let column = entries
                .first()
                .map_or(0, |entry| column_of(document, entry.key_span.start));
            let insert_at = after_last_child(document, mapping);
            let newline = preceding_terminator(document, insert_at);
            let value = expected_at(expected, &steps[..=frames.len()])?;
            let chain = frame_chain(&frames);
            let widths = Widths { layout, examples: &examples, at: insert_at, chain: &chain };
            vec![Splice::insert(insert_at, entry_lines(key, value, column, newline, &widths)?)]
        }
        Walked::MissingItem { frames, sequence } => {
            let SchemaValueKind::Sequence(items) = &sequence.kind else {
                return Err(unsupported("a new item's parent is not a sequence"));
            };
            let first = items
                .first()
                .ok_or_else(|| unsupported("a new item's block sequence has no items"))?;
            let column = column_of(document, item_dash(document, first.span.start)?);
            let insert_at = after_last_child(document, sequence);
            let newline = preceding_terminator(document, insert_at);
            let value = expected_at(expected, &steps[..=frames.len()])?;
            let chain = frame_chain(&frames);
            let widths = Widths { layout, examples: &examples, at: insert_at, chain: &chain };
            let text = inline_text(value, column + 2, newline, widths.of(value))?;
            vec![Splice::insert(insert_at, format!("{}- {text}{newline}", " ".repeat(column)))]
        }
    };
    Ok(spliced(document, splices))
}

/// The authored shape of a collection in the text: an example of how far its
/// author indents a block under its key.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CollectionKind {
    Mapping,
    Sequence,
}

impl CollectionKind {
    fn of_value(value: &Value) -> Option<Self> {
        match value {
            Value::Object(_) => Some(Self::Mapping),
            Value::Array(_) => Some(Self::Sequence),
            _ => None,
        }
    }

    fn of_node(node: &SchemaValueNode) -> Option<Self> {
        match node.kind {
            SchemaValueKind::Mapping(_) => Some(Self::Mapping),
            SchemaValueKind::Sequence(_) => Some(Self::Sequence),
            SchemaValueKind::Scalar => None,
        }
    }
}

/// A block collection under a mapping key, and how far its children sit past
/// that key.
#[derive(Debug)]
struct Example {
    kind: CollectionKind,
    /// Columns from the key to the first child (its key, or its dash).
    offset: usize,
    /// Where its key is.
    start: usize,
    /// Where the holders enclosing its key are, outermost first.
    chain: Vec<usize>,
}

/// Every block collection the YAML region holds under a mapping key.
///
/// Only actual structure counts: flow collections, the two columns a compact
/// `- key:` item puts after its dash, the content lines of multiline scalars,
/// and indentless sequences (an offset of zero) are never examples.
fn collect_examples(
    document: &str,
    yaml: Range<usize>,
    nodes: &indexmap::IndexMap<String, super::write::TextNode>,
) -> Vec<Example> {
    let mut examples = Vec::new();
    for node in nodes.values() {
        let start = yaml.start + node.range.start;
        let end = yaml.start + node.range.end;
        if let Some(tree) = locate_frontmatter_value(&document[start..end], start) {
            collect_in(document, &tree, &mut Vec::new(), &mut examples);
        }
    }
    examples
}

fn collect_in(document: &str, node: &SchemaValueNode, chain: &mut Vec<usize>, out: &mut Vec<Example>) {
    match &node.kind {
        SchemaValueKind::Mapping(entries) => {
            for entry in entries {
                let value = &entry.value;
                if holds_block(document, value)
                    && let (Some(kind), Some(child)) = (CollectionKind::of_node(value), first_child_column(document, value))
                {
                    let key = column_of(document, entry.key_span.start);
                    if child > key {
                        out.push(Example { kind, offset: child - key, start: entry.key_span.start, chain: chain.clone() });
                    }
                }
                chain.push(entry.key_span.start);
                collect_in(document, value, chain, out);
                chain.pop();
            }
        }
        SchemaValueKind::Sequence(items) => {
            for item in items {
                chain.push(item.span.start);
                collect_in(document, item, chain, out);
                chain.pop();
            }
        }
        SchemaValueKind::Scalar => {}
    }
}

/// Columns per level for `value` written as a new top-level entry of the
/// YAML region `yaml` at `at`, chosen as the editors choose, with `fallback`
/// where the region holds no example.
pub(crate) fn top_level_width(document: &str, yaml: Range<usize>, value: &Value, at: usize, fallback: Indentation) -> usize {
    let examples = locate_all_nodes(&document[yaml.clone()])
        .map(|nodes| collect_examples(document, yaml, &nodes))
        .unwrap_or_default();
    let fallback = move || fallback;
    let layout = Layout::new(&fallback, true);
    Widths { layout: &layout, examples: &examples, at, chain: &[] }.of(value)
}

/// `value` laid out with `width` columns per level, with its final line
/// break; the writer's error as text.
pub(crate) fn layout_text<T: Serialize + ?Sized>(value: &T, width: usize) -> Result<String, String> {
    let width = NonZeroU8::new(u8::try_from(width).unwrap_or(u8::MAX)).unwrap_or(Indentation::DEFAULT_WIDTH);
    biscuit_file::to_yaml_string(value, Indentation::spaces(width)).map_err(|error| error.to_string())
}

fn frame_chain(frames: &[Frame<'_>]) -> Vec<usize> {
    frames.iter().map(|frame| frame.holder.start()).collect()
}

/// The width choice for one rendered value at one place in the text.
struct Widths<'l, 'e> {
    layout: &'l Layout<'l>,
    examples: &'e [Example],
    /// Where the value goes.
    at: usize,
    /// The holders enclosing the value's own holder, as [`Example::chain`].
    chain: &'e [usize],
}

impl Widths<'_, '_> {
    /// Columns per level for `value`: the nearest example of its kind (a
    /// scalar takes any kind), preferring one inside the deepest holder it
    /// shares with the value, then the closest by bytes, then the earlier;
    /// the resolved preference when there is none.
    fn of(&self, value: &Value) -> usize {
        self.for_kind(CollectionKind::of_value(value))
    }

    fn for_kind(&self, kind: Option<CollectionKind>) -> usize {
        self.examples
            .iter()
            .filter(|example| kind.is_none_or(|kind| example.kind == kind))
            .min_by_key(|example| {
                let shared = self.chain.iter().zip(&example.chain).take_while(|(a, b)| a == b).count();
                (Reverse(shared), example.start.abs_diff(self.at), example.start)
            })
            .map_or_else(|| self.layout.fallback_width(), |example| example.offset)
    }
}

/// Where a new key goes in a block mapping, or a new item in a block sequence:
/// on the line after its last child.
///
/// A located scalar ends at its last text line, but a `|+` or `>+` scalar owns
/// the blank lines after that, and a key written among them would shorten it.
fn after_last_child(document: &str, mapping: &SchemaValueNode) -> usize {
    let mut last = mapping;
    loop {
        last = match &last.kind {
            SchemaValueKind::Mapping(entries) => match entries.last() {
                Some(entry) => &entry.value,
                None => break,
            },
            SchemaValueKind::Sequence(items) => match items.last() {
                Some(item) => item,
                None => break,
            },
            SchemaValueKind::Scalar => break,
        };
    }
    let mut after = line_end(document, mapping.span.end);
    if keeps_trailing_lines(&document[last.span.clone()]) {
        while after < document.len() && content_end(document, after) == after {
            after = line_end(document, after);
        }
    }
    after
}

/// Whether scalar text opens with a block header holding the keep indicator.
fn keeps_trailing_lines(scalar_text: &str) -> bool {
    let content = past_node_properties(scalar_text).0;
    let header = content.split_whitespace().next().unwrap_or_default();
    header.starts_with(['|', '>']) && header.contains('+')
}

/// The value `expected` holds at `steps`. A `null` there is a value.
fn expected_at<'v>(
    expected: &'v FrontmatterMap,
    steps: &[PathStep<'_>],
) -> Result<&'v Value, PathEditRefusal> {
    let missing = || unsupported("the expected values do not hold the edited path");
    let Some((PathStep::Key(top), rest)) = steps.split_first() else {
        return Err(missing());
    };
    let mut value = expected.get(*top).ok_or_else(missing)?;
    for step in rest {
        value = match step {
            PathStep::Key(key) => value.get(*key),
            PathStep::Index(index) => value.get(*index),
        }
        .ok_or_else(missing)?;
    }
    Ok(value)
}

/// What holds a node: its pair in a mapping, or its place in a sequence.
#[derive(Clone, Copy)]
enum Holder<'t> {
    Entry(&'t SchemaValueEntry),
    Item(&'t SchemaValueNode),
}

impl<'t> Holder<'t> {
    /// Where the holder is named, as [`Example::start`] and its chains record.
    fn start(self) -> usize {
        match self {
            Self::Entry(entry) => entry.key_span.start,
            Self::Item(item) => item.span.start,
        }
    }

    fn node(self) -> &'t SchemaValueNode {
        match self {
            Self::Entry(entry) => &entry.value,
            Self::Item(item) => item,
        }
    }
}

/// One step of a walk: the holder reached, and its position in its container.
struct Frame<'t> {
    holder: Holder<'t>,
    position: usize,
}

enum Walked<'t> {
    /// The frames from the top-level property down. Shorter than the path only
    /// when a `Set` reached an empty `{}`, which it replaces whole.
    Node(Vec<Frame<'t>>),
    /// The path leaves a block mapping at `key`, which a `Set` adds.
    /// `frames` holds the steps already walked.
    MissingKey {
        frames: Vec<Frame<'t>>,
        mapping: &'t SchemaValueNode,
        key: &'t str,
    },
    /// A `Set`'s last step is the index one past a block sequence's last item,
    /// which appends. `frames` holds the steps already walked.
    MissingItem {
        frames: Vec<Frame<'t>>,
        sequence: &'t SchemaValueNode,
    },
}

fn walk<'t>(
    document: &str,
    top_entry: &'t SchemaValueEntry,
    steps: &[PathStep<'t>],
    creating: bool,
) -> Result<Walked<'t>, PathEditRefusal> {
    let mut frames = vec![Frame {
        holder: Holder::Entry(top_entry),
        position: 0,
    }];
    for step in &steps[1..] {
        let node = frames[frames.len() - 1].holder.node();
        let shape = authored_shape(&document[node.span.clone()]);
        let walked = || steps_label(&steps[..frames.len()]);
        if shape.alias {
            return Err(unsupported(format!("the path walks through an alias at `{}`", walked())));
        }
        if shape.tag {
            return Err(unsupported(format!("the path walks through a tagged node at `{}`", walked())));
        }
        if shape.flow {
            // An empty `{}` or `[]` holds nothing to keep, so a `Set` below it
            // replaces it whole.
            let empty_collection = match (&node.kind, step) {
                (SchemaValueKind::Mapping(entries), PathStep::Key(_)) => entries.is_empty(),
                (SchemaValueKind::Sequence(items), PathStep::Index(_)) => items.is_empty(),
                _ => false,
            };
            if creating && empty_collection {
                return Ok(Walked::Node(frames));
            }
            return Err(unsupported(format!("the path walks into a flow collection at `{}`", walked())));
        }
        let next = match (&node.kind, step) {
            (SchemaValueKind::Mapping(entries), PathStep::Key(key)) => {
                match entries.iter().position(|entry| entry.key == *key) {
                    Some(position) => Frame {
                        holder: Holder::Entry(&entries[position]),
                        position,
                    },
                    None if creating => {
                        return Ok(Walked::MissingKey {
                            frames,
                            mapping: node,
                            key,
                        });
                    }
                    None => return Err(unsupported(format!("`{key}` is not under `{}`", walked()))),
                }
            }
            (SchemaValueKind::Sequence(items), PathStep::Index(index)) if *index < items.len() => {
                Frame {
                    holder: Holder::Item(&items[*index]),
                    position: *index,
                }
            }
            (SchemaValueKind::Sequence(items), PathStep::Index(index))
                if creating && *index == items.len() && frames.len() + 1 == steps.len() =>
            {
                return Ok(Walked::MissingItem {
                    frames,
                    sequence: node,
                });
            }
            // A scalar (normally `null`) where the expected values hold a
            // mapping: the caller's pure edit replaced it, so the `Set` does too.
            (SchemaValueKind::Scalar, PathStep::Key(_)) if creating => return Ok(Walked::Node(frames)),
            _ => return Err(unsupported(format!("the path does not match the source at `{}`", walked()))),
        };
        frames.push(next);
    }
    Ok(Walked::Node(frames))
}

fn steps_label(steps: &[PathStep<'_>]) -> String {
    dotted_path(&steps.iter().map(segment_of).collect::<Vec<_>>())
}

/// What a located node's text starts with, past its properties.
struct AuthoredShape {
    alias: bool,
    anchor: bool,
    tag: bool,
    flow: bool,
}

fn authored_shape(node_text: &str) -> AuthoredShape {
    let (content, Properties { tag, anchor }) = past_node_properties(node_text);
    AuthoredShape {
        alias: content.starts_with('*'),
        anchor,
        tag,
        flow: content.starts_with(['[', '{']),
    }
}

/// The node properties found before a node's content.
struct Properties {
    tag: bool,
    anchor: bool,
}

/// `node_text` past any anchor and tag, and which of the two there were.
fn past_node_properties(node_text: &str) -> (&str, Properties) {
    let mut content = node_text;
    let mut properties = Properties { tag: false, anchor: false };
    while content.starts_with(['&', '!']) {
        properties.tag |= content.starts_with('!');
        properties.anchor |= content.starts_with('&');
        let token = content.find(char::is_whitespace).unwrap_or(content.len());
        content = content[token..].trim_start();
    }
    (content, properties)
}

/// Whether the node is a block mapping or block sequence, which occupies lines
/// of its own.
fn holds_block(document: &str, node: &SchemaValueNode) -> bool {
    !matches!(node.kind, SchemaValueKind::Scalar)
        && !authored_shape(&document[node.span.clone()]).flow
}

fn replace_entry(
    document: &str,
    entry: &SchemaValueEntry,
    value: &Value,
    widths: &Widths<'_, '_>,
) -> Result<Vec<Splice>, PathEditRefusal> {
    let colon = entry_colon(document, entry)?;
    let after_key_line = line_end(document, colon);
    let key_column = column_of(document, entry.key_span.start);
    let newline = line_terminator(document, colon);
    let old = &entry.value;
    let becomes_block = is_block_value(value);
    let width = widths.of(value);

    if holds_block(document, old) {
        let old_lines = after_key_line..line_end(document, old.span.end);
        if becomes_block {
            // The touched block's own offset wins, an indentless sequence's
            // zero included.
            let column = first_child_column(document, old).unwrap_or(key_column + width);
            return Ok(vec![Splice {
                range: old_lines,
                text: block_lines(value, column, newline, width)?,
            }]);
        }
        let text = inline_text(value, key_column, newline, width)?;
        let head = key_head_end(document, colon);
        return Ok(vec![
            in_place(document, head..head, &format!(" {text}"), newline, ""),
            Splice::delete(old_lines),
        ]);
    }

    if old.span.is_empty() {
        return Ok(vec![if becomes_block {
            Splice::insert(after_key_line, block_lines(value, key_column + width, newline, width)?)
        } else {
            let text = format!(" {}", inline_text(value, key_column, newline, width)?);
            in_place(document, colon + 1..colon + 1, &text, newline, "")
        }]);
    }
    if !becomes_block {
        let text = inline_text(value, key_column, newline, width)?;
        return Ok(vec![in_place(document, old.span.clone(), &text, newline, "")]);
    }
    let block = block_lines(value, key_column + width, newline, width)?;
    let after_old = line_end(document, old.span.end);
    if old.span.start < after_key_line {
        // A comment after the old value stays, now on the key's line.
        Ok(vec![
            Splice::delete(colon + 1..old.span.end),
            Splice::insert(after_old, block),
        ])
    } else {
        Ok(vec![Splice {
            range: line_start(document, old.span.start)..after_old,
            text: block,
        }])
    }
}

fn replace_item(
    document: &str,
    item: &SchemaValueNode,
    value: &Value,
    widths: &Widths<'_, '_>,
) -> Result<Vec<Splice>, PathEditRefusal> {
    let start = item.span.start;
    let newline = line_terminator(document, start);
    let width = widths.of(value);
    if item.span.is_empty() {
        let padding = if document[..start].ends_with([' ', '\t']) { "" } else { " " };
        let gap = if document[start..].starts_with('#') { " " } else { "" };
        let column = column_of(document, start) + padding.len();
        let text = format!("{padding}{}", inline_text(value, column, newline, width)?);
        return Ok(vec![in_place(document, start..start, &text, newline, gap)]);
    }
    let end = if holds_block(document, item) {
        content_end(document, item.span.end)
    } else {
        item.span.end
    };
    let text = inline_text(value, column_of(document, start), newline, width)?;
    Ok(vec![in_place(document, start..end, &text, newline, "")])
}

/// Writes `text` over `range`, keeping what follows it on the line.
///
/// A longer `text` keeps that comment on its first line, because the last line
/// of a block scalar would read the comment as content. `gap` separates the two
/// when `range` ends right at the comment.
fn in_place(document: &str, range: Range<usize>, text: &str, newline: &str, gap: &str) -> Splice {
    match text.split_once(newline) {
        Some((first, rest)) => {
            let line_end = content_end(document, range.end);
            let comment = &document[range.end..line_end];
            Splice {
                range: range.start..line_end,
                text: format!("{first}{gap}{comment}{newline}{rest}"),
            }
        }
        None => Splice {
            range,
            text: format!("{text}{gap}"),
        },
    }
}

fn remove_node(document: &str, frames: &[Frame<'_>]) -> Result<Vec<Splice>, PathEditRefusal> {
    let target = &frames[frames.len() - 1];
    let parent = frames.len().checked_sub(2).map(|index| frames[index].holder);
    match target.holder {
        Holder::Entry(entry) => {
            let start = entry.key_span.start;
            let own_lines = line_start(document, start)..line_end(document, entry.value.span.end);
            let Some(parent) = parent else {
                return Ok(vec![Splice::delete(own_lines)]);
            };
            let SchemaValueKind::Mapping(entries) = &parent.node().kind else {
                return Err(unsupported("a key's parent is not a mapping"));
            };
            if entries.len() == 1 {
                return emptied_container(document, parent, "{}", own_lines);
            }
            if opens_its_line(document, start) {
                return Ok(vec![Splice::delete(own_lines)]);
            }
            // `- first: 1`: the next key moves up behind the dash.
            let next = entries
                .get(target.position + 1)
                .ok_or_else(|| unsupported("a key shares its line but has no next key"))?;
            Ok(vec![Splice::delete(start..next.key_span.start)])
        }
        Holder::Item(item) => {
            let dash = item_dash(document, item.span.start)?;
            let own_lines = line_start(document, dash)..line_end(document, item.span.end);
            let Some(parent) = parent else {
                return Err(unsupported("a sequence item has no parent"));
            };
            let SchemaValueKind::Sequence(items) = &parent.node().kind else {
                return Err(unsupported("an item's parent is not a sequence"));
            };
            if items.len() == 1 {
                return emptied_container(document, parent, "[]", own_lines);
            }
            Ok(vec![Splice::delete(own_lines)])
        }
    }
}

/// Removes a container's only child, and writes `marker` where the container's
/// value goes. A comment line elsewhere in the container stays.
fn emptied_container(
    document: &str,
    container: Holder<'_>,
    marker: &str,
    child_lines: Range<usize>,
) -> Result<Vec<Splice>, PathEditRefusal> {
    match container {
        Holder::Entry(entry) => {
            let colon = entry_colon(document, entry)?;
            Ok(vec![
                Splice::insert(key_head_end(document, colon), format!(" {marker}")),
                Splice::delete(child_lines),
            ])
        }
        Holder::Item(item) => Ok(vec![Splice {
            range: item.span.start..content_end(document, item.span.end),
            text: marker.to_string(),
        }]),
    }
}

/// The offset of the `:` after an entry's key.
fn entry_colon(document: &str, entry: &SchemaValueEntry) -> Result<usize, PathEditRefusal> {
    let start = entry.key_span.start;
    mapping_colon(&document[start..content_end(document, start)])
        .map(|colon| start + colon)
        .ok_or_else(|| unsupported(format!("`{}` is not a plain `key: value` line", entry.key)))
}

/// Where the text on a key's line ends, before any comment: past the colon, and
/// past a block value's anchor or tag.
fn key_head_end(document: &str, colon: usize) -> usize {
    let after_colon = &document[colon + 1..content_end(document, colon)];
    let comment = yaml_comment_start(after_colon).unwrap_or(after_colon.len());
    colon + 1 + after_colon[..comment].trim_end().len()
}

/// The column of a block container's children, so a replacement keeps it.
fn first_child_column(document: &str, node: &SchemaValueNode) -> Option<usize> {
    match &node.kind {
        SchemaValueKind::Mapping(entries) => {
            entries.first().map(|entry| column_of(document, entry.key_span.start))
        }
        SchemaValueKind::Sequence(items) => items
            .first()
            .and_then(|item| item_dash(document, item.span.start).ok())
            .map(|dash| column_of(document, dash)),
        SchemaValueKind::Scalar => None,
    }
}

/// The `-` that introduces the sequence item whose content starts at `start`:
/// on the same line, or alone on an earlier line past blanks and comments.
fn item_dash(document: &str, start: usize) -> Result<usize, PathEditRefusal> {
    let first_line = line_start(document, start);
    let before = document[first_line..start].trim_end();
    if before.ends_with('-') {
        return Ok(first_line + before.len() - 1);
    }
    let mut line = first_line;
    while line > 0 {
        line = line_start(document, previous_line_offset(document, line));
        let text = &document[line..content_end(document, line)];
        let trimmed = text.trim_start();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        let without_comment = trimmed.split(" #").next().unwrap_or(trimmed).trim_end();
        if without_comment == "-" {
            return Ok(line + (text.len() - trimmed.len()));
        }
        break;
    }
    Err(unsupported("a sequence item's dash could not be located"))
}

/// An offset on the line before the one starting at `line`.
fn previous_line_offset(document: &str, line: usize) -> usize {
    let before = document[..line].strip_suffix('\n').unwrap_or(&document[..line]);
    let before = before.strip_suffix('\r').unwrap_or(before);
    before.len()
}

/// Whether only indentation comes before `offset` on its line.
fn opens_its_line(document: &str, offset: usize) -> bool {
    document[line_start(document, offset)..offset].trim().is_empty()
}

fn line_start(text: &str, offset: usize) -> usize {
    text[..offset].rfind(['\n', '\r']).map_or(0, |index| index + 1)
}

fn content_end(text: &str, offset: usize) -> usize {
    text[offset..].find(['\n', '\r']).map_or(text.len(), |index| offset + index)
}

/// The start of the line after the one holding `offset`.
fn line_end(text: &str, offset: usize) -> usize {
    let end = content_end(text, offset);
    if text[end..].starts_with("\r\n") {
        end + 2
    } else if end < text.len() {
        end + 1
    } else {
        end
    }
}

/// The terminator of the line holding `offset`; LF for an unterminated line.
fn line_terminator(text: &str, offset: usize) -> &str {
    let terminator = &text[content_end(text, offset)..line_end(text, offset)];
    if terminator.is_empty() { "\n" } else { terminator }
}

/// A non-empty mapping or sequence, the values that need lines of their own.
fn is_block_value(value: &Value) -> bool {
    match value {
        Value::Object(map) => !map.is_empty(),
        Value::Array(items) => !items.is_empty(),
        _ => false,
    }
}

/// `value` laid out with `width` columns per level, without its final line
/// break.
fn yaml_text<T: Serialize + ?Sized>(value: &T, width: usize) -> Result<String, PathEditRefusal> {
    let text = layout_text(value, width).map_err(|error| unsupported(format!("the value could not be laid out: {error}")))?;
    Ok(text.strip_suffix('\n').map(str::to_string).unwrap_or(text))
}

/// `text` with `column` spaces before each non-empty line, from `first` on.
fn indented(text: &str, column: usize, first: usize) -> String {
    let padding = " ".repeat(column);
    text.split('\n')
        .enumerate()
        .map(|(index, line)| {
            if index < first || line.is_empty() {
                line.to_string()
            } else {
                format!("{padding}{line}")
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// `value` written to start in place, its later lines at `column`.
fn inline_text(value: &Value, column: usize, newline: &str, width: usize) -> Result<String, PathEditRefusal> {
    let text = indented(&yaml_text(value, width)?, column, 1);
    Ok(apply_terminators(&text, &[newline]))
}

/// `value` as whole lines at `column`, each terminated.
fn block_lines(value: &Value, column: usize, newline: &str, width: usize) -> Result<String, PathEditRefusal> {
    let mut text = indented(&yaml_text(value, width)?, column, 0);
    text.push('\n');
    Ok(apply_terminators(&text, &[newline]))
}

/// A new `key: value` entry as whole lines at `column`, each terminated, its
/// value one level of the width [`Widths`] picks for it past the key.
fn entry_lines(
    key: &str,
    value: &Value,
    column: usize,
    newline: &str,
    widths: &Widths<'_, '_>,
) -> Result<String, PathEditRefusal> {
    let mut entry = indexmap::IndexMap::new();
    entry.insert(key, value);
    let text = indented(&yaml_text(&entry, widths.of(value))?, column, 0);
    Ok(apply_terminators(&format!("{text}\n"), &[newline]))
}
