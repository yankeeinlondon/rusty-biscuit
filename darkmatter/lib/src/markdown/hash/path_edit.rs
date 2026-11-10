//! Text-preserving edits of nested frontmatter nodes.
//!
//! [`restore_properties_text`](super::restore_properties_text) rewrites whole
//! top-level nodes, so using it for `author.name` would lose the comments on
//! `author`'s other children. This editor changes only the named node, and
//! keeps every other byte apart from the separators a container needs to stay
//! valid.
//!
//! An edit is never trusted because it was computed: the result is reloaded
//! and compared with the values the caller expects, before anything is
//! returned. See [`edit_frontmatter_paths`].

use super::write::{
    FrontmatterPathSegment, apply_terminators, column_of, dotted_path, locate_all_nodes,
    mapping_colon, new_block_terminator, new_frontmatter_block, preceding_terminator,
    serialize_entry, validate_block_mapping, yaml_comment_start,
};
use crate::markdown::{FrontmatterMap, Markdown, MarkdownError, extract_frontmatter_block};
use biscuit_file::serde_yaml_ng;
use serde_json::Value;
use simplified_schema::completion::PathStep;
use simplified_schema::simplified::locate_frontmatter_value;
use simplified_schema::{SchemaValueEntry, SchemaValueKind, SchemaValueNode};
use std::cmp::Ordering;
use std::ops::Range;

/// One change to make to a document's frontmatter text.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PathEdit<'a> {
    /// Writes the value `expected` holds at this path, creating any missing
    /// mapping parents. The steps describe `expected`, as
    /// `PropertyPath::resolve_map` reports them for the edited map.
    Set(Vec<PathStep<'a>>),
    /// Removes the node at this path. The steps describe the unedited source,
    /// as `RemovalPlan::targets` reports them; array indexes are the original
    /// ones, whatever order the removals are listed in.
    Remove(Vec<PathStep<'a>>),
}

/// Why a frontmatter edit produced no text.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum PathEditRefusal {
    /// The source is written in a form this editor cannot change safely. Whole
    /// frontmatter serialization may still be able to express the edit.
    #[error("the frontmatter cannot be edited in place: {reason}")]
    UnsupportedForm {
        /// What the editor met.
        reason: String,
    },
    /// The edit would change a value it does not name, normally because an
    /// alias reads an anchor inside the edited node. Serializing instead would
    /// hide this, so it has no fallback.
    #[error("{reason}")]
    WouldChangeUnnamed {
        /// Which value would change.
        reason: String,
    },
}

/// Applies `edits` to `source`'s frontmatter, keeping every byte outside the
/// edited nodes.
///
/// The result has been reloaded by the document loader and every value compared
/// with `expected`, the map the same edits produced on the parsed document, so
/// a returned text is safe to write. Only a path's own node is replaced, a new
/// key goes at the end of its parent, and a removed node takes its own lines. A
/// mapping or sequence left empty stays, written `{}` or `[]`.
///
/// ## Errors
///
/// [`PathEditRefusal::UnsupportedForm`] for a source this editor cannot change
/// safely: frontmatter that is indented, flow-style, or not strict YAML; a path
/// that walks into a flow collection, an alias, or a tagged node; or a node the
/// locator cannot bound. It is also the outcome when the named value itself
/// does not reload as expected.
///
/// [`PathEditRefusal::WouldChangeUnnamed`] when any other value would differ
/// from `expected`, or an alias would lose its anchor.
pub fn edit_frontmatter_paths(
    source: &str,
    expected: &FrontmatterMap,
    edits: &[PathEdit<'_>],
) -> Result<String, PathEditRefusal> {
    let mut text = source.to_string();
    for edit in application_order(edits) {
        text = apply_edit(&text, expected, edit)?;
        // Checked per edit because the next one begins by requiring strict
        // YAML: an alias orphaned here would otherwise read as an unsupported
        // form, and the fallback would hide it.
        reload_values(&text)?;
    }
    verify_edited(&text, expected, edits)?;
    Ok(text)
}

/// The text to write for `edited`, whose frontmatter already holds `edits`.
///
/// Edits the text `edited` was loaded from. Where the editor refuses the form,
/// or there is no source text, the whole frontmatter is serialized instead,
/// which loses comments and spelling in the frontmatter but is verified the same
/// way. Either way, the fences and body keep the source's bytes.
///
/// ## Errors
///
/// [`PathEditRefusal::WouldChangeUnnamed`] from the editor, or when the
/// serialized text would not reload to the same values.
/// [`PathEditRefusal::UnsupportedForm`] when the serialized text does not
/// reload at all.
pub fn edited_document_text(
    edited: &Markdown,
    edits: &[PathEdit<'_>],
) -> Result<String, PathEditRefusal> {
    let expected = edited.frontmatter().as_map();
    let Some(source) = edited.source_text() else {
        return serialized_document_text(edited);
    };
    match edit_frontmatter_paths(source, expected, edits) {
        Err(PathEditRefusal::UnsupportedForm { .. }) => serialized_document_text(edited),
        result => result,
    }
}

fn serialized_document_text(edited: &Markdown) -> Result<String, PathEditRefusal> {
    let text = serialized_into_source(edited)?;
    let reloaded = reload_values(&text)?;
    match differences(edited.frontmatter().as_map(), &reloaded).first() {
        Some(path) => Err(would_change(path)),
        None => Ok(text),
    }
}

/// The frontmatter serialized between the source's own fences.
///
/// `Markdown::as_string` is not enough when there is a source: the loaded body
/// has lost its final line break and its line endings.
fn serialized_into_source(edited: &Markdown) -> Result<String, PathEditRefusal> {
    let Some(source) = edited.source_text() else {
        return Ok(edited.as_string());
    };
    let map = edited.frontmatter().as_map();
    let yaml = if map.is_empty() {
        String::new()
    } else {
        serde_yaml_ng::to_string(map)
            .map_err(|error| unsupported(format!("the frontmatter could not be serialized: {error}")))?
    };
    let frontmatter =
        extract_frontmatter_block(source).map_err(|error| unsupported(error.to_string()))?;
    let Some(frontmatter) = frontmatter else {
        return Ok(new_frontmatter_block(
            source,
            &apply_terminators(&yaml, &[new_block_terminator(source)]),
        ));
    };
    let span = frontmatter.yaml_span;
    let newline = preceding_terminator(source, span.end);
    Ok(format!(
        "{}{}{}",
        &source[..span.start],
        apply_terminators(&yaml, &[newline]),
        &source[span.end..]
    ))
}

/// Sets first, then removals with higher indexes ahead of lower ones, so each
/// removal's original index still names the same element when its turn comes.
fn application_order<'e, 'a>(edits: &'e [PathEdit<'a>]) -> Vec<&'e PathEdit<'a>> {
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

/// The frontmatter values of `text`, read as a document load reads them.
fn reload_values(text: &str) -> Result<FrontmatterMap, PathEditRefusal> {
    match Markdown::try_from_content(text) {
        Ok(document) => Ok(document.frontmatter().as_map().clone()),
        // The parser's wording is the only signal that this is an orphaned
        // alias rather than some other parse failure.
        Err(MarkdownError::FrontmatterParse { source, .. })
            if source.to_string().contains("unknown anchor") =>
        {
            Err(PathEditRefusal::WouldChangeUnnamed {
                reason: format!(
                    "the edit would remove an anchor that an alias outside the edited paths \
                     still reads: {source}"
                ),
            })
        }
        Err(error) => Err(unsupported(format!("the edited text does not load: {error}"))),
    }
}

fn verify_edited(
    text: &str,
    expected: &FrontmatterMap,
    edits: &[PathEdit<'_>],
) -> Result<(), PathEditRefusal> {
    let reloaded = reload_values(text)?;
    let found = differences(expected, &reloaded);
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
        return Err(would_change(path));
    }
    match found.first() {
        Some(path) => Err(unsupported(format!(
            "`{}` does not read back as the value it was given",
            dotted_path(path)
        ))),
        None => Ok(()),
    }
}

fn segment_of(step: &PathStep<'_>) -> FrontmatterPathSegment {
    match step {
        PathStep::Key(key) => FrontmatterPathSegment::Key((*key).to_string()),
        PathStep::Index(index) => FrontmatterPathSegment::Index(*index),
    }
}

fn would_change(path: &[FrontmatterPathSegment]) -> PathEditRefusal {
    PathEditRefusal::WouldChangeUnnamed {
        reason: format!(
            "the edit would change the value of `{}`, which it does not name \
             (an alias there likely reads an anchor in the edited node)",
            dotted_path(path)
        ),
    }
}

fn unsupported(reason: impl Into<String>) -> PathEditRefusal {
    PathEditRefusal::UnsupportedForm {
        reason: reason.into(),
    }
}

/// Every path where `expected` and `actual` differ, as deep as they agree on
/// shape. A sequence of another length differs as a whole.
fn differences(
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
    document: &str,
    expected: &FrontmatterMap,
    edit: &PathEdit<'_>,
) -> Result<String, PathEditRefusal> {
    let (PathEdit::Set(steps) | PathEdit::Remove(steps)) = edit;
    let Some(PathStep::Key(top)) = steps.first() else {
        return Err(unsupported("a frontmatter path must start with a key"));
    };
    let creating = matches!(edit, PathEdit::Set(_));
    let frontmatter = extract_frontmatter_block(document)
        .map_err(|error| unsupported(error.to_string()))?;
    let Some(frontmatter) = frontmatter else {
        if !creating {
            return Err(unsupported("the document has no frontmatter"));
        }
        let newline = new_block_terminator(document);
        let entry = entry_lines(top, expected_at(expected, &steps[..1])?, 0, newline)?;
        return Ok(new_frontmatter_block(document, &entry));
    };
    validate_block_mapping(frontmatter.yaml).map_err(|error| unsupported(error.to_string()))?;
    let nodes =
        locate_all_nodes(frontmatter.yaml).map_err(|error| unsupported(error.to_string()))?;
    let yaml_start = frontmatter.yaml_span.start;
    let yaml_end = frontmatter.yaml_span.end;

    let Some(node) = nodes.get(*top) else {
        if !creating {
            return Err(unsupported(format!("`{top}` is not in the frontmatter")));
        }
        let newline = preceding_terminator(document, yaml_end);
        let entry = entry_lines(top, expected_at(expected, &steps[..1])?, 0, newline)?;
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
                match frames[frames.len() - 1].holder {
                    Holder::Entry(entry) => replace_entry(document, entry, value)?,
                    Holder::Item(item) => replace_item(document, item, value)?,
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
            let value = expected_at(expected, &steps[..=frames])?;
            vec![Splice::insert(insert_at, entry_lines(key, value, column, newline)?)]
        }
    };
    Ok(spliced(document, splices))
}

/// Where a new key goes in a block mapping: on the line after its last child.
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
    /// `frames` counts the steps already walked.
    MissingKey {
        frames: usize,
        mapping: &'t SchemaValueNode,
        key: &'t str,
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
            let empty_mapping = matches!(&node.kind, SchemaValueKind::Mapping(entries) if entries.is_empty());
            if creating && empty_mapping && matches!(step, PathStep::Key(_)) {
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
                            frames: frames.len(),
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
    tag: bool,
    flow: bool,
}

fn authored_shape(node_text: &str) -> AuthoredShape {
    let (content, tag) = past_node_properties(node_text);
    AuthoredShape {
        alias: content.starts_with('*'),
        tag,
        flow: content.starts_with(['[', '{']),
    }
}

/// `node_text` past any anchor and tag, and whether there was a tag.
fn past_node_properties(node_text: &str) -> (&str, bool) {
    let mut content = node_text;
    let mut tag = false;
    while content.starts_with(['&', '!']) {
        tag |= content.starts_with('!');
        let token = content.find(char::is_whitespace).unwrap_or(content.len());
        content = content[token..].trim_start();
    }
    (content, tag)
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
) -> Result<Vec<Splice>, PathEditRefusal> {
    let colon = entry_colon(document, entry)?;
    let after_key_line = line_end(document, colon);
    let key_column = column_of(document, entry.key_span.start);
    let newline = line_terminator(document, colon);
    let old = &entry.value;
    let becomes_block = is_block_value(value);

    if holds_block(document, old) {
        let old_lines = after_key_line..line_end(document, old.span.end);
        if becomes_block {
            let column = first_child_column(document, old).unwrap_or(key_column + 2);
            return Ok(vec![Splice {
                range: old_lines,
                text: block_lines(value, column, newline)?,
            }]);
        }
        let text = inline_text(value, key_column, newline)?;
        let head = key_head_end(document, colon);
        return Ok(vec![
            in_place(document, head..head, &format!(" {text}"), newline, ""),
            Splice::delete(old_lines),
        ]);
    }

    if old.span.is_empty() {
        return Ok(vec![if becomes_block {
            Splice::insert(after_key_line, block_lines(value, key_column + 2, newline)?)
        } else {
            let text = format!(" {}", inline_text(value, key_column, newline)?);
            in_place(document, colon + 1..colon + 1, &text, newline, "")
        }]);
    }
    if !becomes_block {
        let text = inline_text(value, key_column, newline)?;
        return Ok(vec![in_place(document, old.span.clone(), &text, newline, "")]);
    }
    let block = block_lines(value, key_column + 2, newline)?;
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
) -> Result<Vec<Splice>, PathEditRefusal> {
    let start = item.span.start;
    let newline = line_terminator(document, start);
    if item.span.is_empty() {
        let padding = if document[..start].ends_with([' ', '\t']) { "" } else { " " };
        let gap = if document[start..].starts_with('#') { " " } else { "" };
        let column = column_of(document, start) + padding.len();
        let text = format!("{padding}{}", inline_text(value, column, newline)?);
        return Ok(vec![in_place(document, start..start, &text, newline, gap)]);
    }
    let end = if holds_block(document, item) {
        content_end(document, item.span.end)
    } else {
        item.span.end
    };
    let text = inline_text(value, column_of(document, start), newline)?;
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

fn yaml_text(value: &Value) -> Result<String, PathEditRefusal> {
    let text = serde_yaml_ng::to_string(value)
        .map_err(|error| unsupported(format!("the value could not be serialized: {error}")))?;
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
fn inline_text(value: &Value, column: usize, newline: &str) -> Result<String, PathEditRefusal> {
    let text = indented(&yaml_text(value)?, column, 1);
    Ok(apply_terminators(&text, &[newline]))
}

/// `value` as whole lines at `column`, each terminated.
fn block_lines(value: &Value, column: usize, newline: &str) -> Result<String, PathEditRefusal> {
    let mut text = indented(&yaml_text(value)?, column, 0);
    text.push('\n');
    Ok(apply_terminators(&text, &[newline]))
}

/// A new `key: value` entry as whole lines at `column`, each terminated.
fn entry_lines(
    key: &str,
    value: &Value,
    column: usize,
    newline: &str,
) -> Result<String, PathEditRefusal> {
    let entry = serialize_entry(key, value).map_err(|error| unsupported(error.to_string()))?;
    let text = indented(entry.strip_suffix('\n').unwrap_or(&entry), column, 0);
    Ok(apply_terminators(&format!("{text}\n"), &[newline]))
}
