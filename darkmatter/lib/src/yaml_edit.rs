//! Text-preserving edits of plain YAML files, such as another program's
//! configuration.
//!
//! [`edit_yaml_paths`] sets or removes values by [`PropertyPath`] and keeps
//! every byte outside the edited nodes: comments, blank lines, key order,
//! quoting, the byte order mark, the line terminator, and a missing final line
//! break. It shares its node-range engine with the Markdown frontmatter editor
//! ([`crate::markdown::hash::edit_frontmatter_paths`]), but edits the whole file
//! as the YAML region and never serializes the file as a whole: an edit it
//! cannot make in place is refused with a located [`YamlEditError`].
//!
//! User-facing guide: `darkmatter/docs/topics/yaml-editing.md`.

use crate::markdown::FrontmatterMap;
use crate::markdown::hash::node_edit::{
    self, Envelope, Layout, TracedNode, segment_of, unexpected_difference, unsupported,
};
use crate::markdown::hash::{FrontmatterPathSegment, PathEdit, PathEditRefusal};
use biscuit_file::{Indentation, serde_yaml_ng};
use serde_json::Value;
use simplified_schema::completion::PathStep;
use simplified_schema::span::line_col_of_offset;
use simplified_schema::{PathEditError, PropertyPath, RemovalPlan, set_at};
use std::ops::Range;

/// One change to a YAML document.
#[derive(Debug, Clone, PartialEq)]
pub enum YamlEdit {
    /// Writes `value` at `path`, creating missing mapping parents. A `null`
    /// parent becomes a mapping. When `path`'s last segment is the length of
    /// an existing sequence (`list.2` on a two-item list), the value is
    /// appended. An empty `[]` or `{}` value is written as such.
    Set {
        /// Where to write.
        path: PropertyPath,
        /// What to write.
        value: Value,
    },
    /// Removes the node at `path`. A mapping or sequence left empty stays,
    /// written `{}` or `[]`.
    Remove {
        /// What to remove.
        path: PropertyPath,
    },
}

/// A position in the source given to [`edit_yaml_paths`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct YamlLocation {
    /// Byte offset into the source, counting a byte order mark.
    pub offset: usize,
    /// 1-based line.
    pub line: usize,
    /// 1-based column, in characters.
    pub column: usize,
}

/// What kind of refusal a [`YamlEditError`] is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum YamlEditErrorKind {
    /// The source is not valid YAML, or the editor's own result would not load.
    Parse,
    /// The source's root is not a mapping (or empty).
    NotAMapping,
    /// The path cannot be applied to the values, for example an index out of
    /// range, a scalar parent, or a missing removal target.
    InvalidPath,
    /// The path reaches an alias (`*name`).
    Alias,
    /// The path reaches an anchored node (`&name`).
    Anchor,
    /// The path reaches a tagged node (`!tag`).
    Tag,
    /// The path walks into a non-empty flow collection (`[…]`, `{…}`), or the
    /// root is one.
    FlowCollection,
    /// Any other layout the editor cannot change in place.
    UnsupportedForm,
    /// The edit would change a value it does not name.
    WouldChangeUnnamed,
}

/// Why [`edit_yaml_paths`] returned no text. The source is untouched: the
/// caller still holds the original bytes and nothing was written.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("line {line}, column {column}: {reason}", line = .location.line, column = .location.column)]
pub struct YamlEditError {
    /// The kind of refusal.
    pub kind: YamlEditErrorKind,
    /// The property the refusal concerns: the edit's path, or for
    /// [`YamlEditErrorKind::WouldChangeUnnamed`] the value that would change.
    /// `None` for a refusal of the whole source, such as a parse error.
    pub path: Option<PropertyPath>,
    /// Where in the source: the node the path reached deepest, the parse
    /// error's position, or the start of the YAML content.
    pub location: YamlLocation,
    /// A human-readable explanation.
    pub reason: String,
}

/// Applies `edits` to the YAML document `source`, keeping every byte outside
/// the edited nodes.
///
/// Every `Set` is applied first, in order; then every `Remove` together, with
/// array indexes as they stand after the sets. The result is reloaded and every
/// value compared with the expected one, named or not, before it is returned.
/// Edits that leave every value as it was return `source` unchanged.
///
/// A new collection is indented like the nearest authored block of its kind,
/// or four spaces where the file has none; see
/// [`edit_yaml_paths_with_indentation`].
///
/// ## Examples
///
/// ```
/// use darkmatter::yaml_edit::{YamlEdit, edit_yaml_paths};
/// use serde_json::json;
/// use simplified_schema::PropertyPath;
///
/// let source = "# my settings\nname: 'kept'\nlist:\n  - a\n";
/// let edits = [YamlEdit::Set { path: PropertyPath::parse("list.1").unwrap(), value: json!("b") }];
/// assert_eq!(
///     edit_yaml_paths(source, &edits).unwrap(),
///     "# my settings\nname: 'kept'\nlist:\n  - a\n  - b\n"
/// );
/// ```
///
/// ## Errors
///
/// A [`YamlEditError`] naming the kind, property, and source position; see
/// [`YamlEditErrorKind`].
pub fn edit_yaml_paths(source: &str, edits: &[YamlEdit]) -> Result<String, YamlEditError> {
    edit_yaml_paths_in_layout(source, edits, &node_edit::default_layout())
}

/// [`edit_yaml_paths`] for a file whose destination prefers `fallback`.
///
/// A new collection copies the indentation of the nearest authored block of
/// its kind, and only where the file has none is it laid out with
/// `fallback`. An existing block that is replaced keeps its own offset.
///
/// ## Errors
///
/// As [`edit_yaml_paths`].
pub fn edit_yaml_paths_with_indentation(
    source: &str,
    edits: &[YamlEdit],
    fallback: Indentation,
) -> Result<String, YamlEditError> {
    let fallback = move || fallback;
    edit_yaml_paths_in_layout(source, edits, &Layout::new(&fallback, true))
}

/// [`edit_yaml_paths`] under the configuration writer's lazy layout.
pub(crate) fn edit_yaml_paths_in_layout(
    source: &str,
    edits: &[YamlEdit],
    layout: &Layout<'_>,
) -> Result<String, YamlEditError> {
    let (bom, body) = match source.strip_prefix('\u{feff}') {
        Some(rest) => ("\u{feff}", rest),
        None => ("", source),
    };
    // The engine inserts whole lines, so an unterminated last line is given
    // the document's terminator for the edit and has it taken away after.
    let newline = first_terminator(body);
    let unterminated = !body.is_empty() && !body.ends_with(['\n', '\r']);
    let working = if unterminated {
        format!("{body}{newline}")
    } else {
        body.to_string()
    };
    let original = load_values(&working).map_err(|failure| {
        Locator { source, bom: bom.len(), working: &working }.load_failure(failure, None)
    })?;
    // An empty `{}` root is an empty mapping: a `Set` turns it into a block,
    // as it does a nested `{}`. Nothing follows it but comments, so no
    // offset an error could report moves.
    let sets_any = edits.iter().any(|edit| matches!(edit, YamlEdit::Set { .. }));
    let working = match empty_flow_root(&working) {
        Some(token) if original.is_empty() && sets_any => format!("{}{}", &working[..token.start], &working[token.end..]),
        _ => working,
    };
    let at = Locator {
        source,
        bom: bom.len(),
        working: &working,
    };
    let content = content_start(&working);
    if working[content..].starts_with(['{', '[']) {
        return Err(at.error(
            YamlEditErrorKind::FlowCollection,
            None,
            content,
            "a flow-style root cannot be edited in place",
        ));
    }

    let mut after_sets = original.clone();
    let mut set_paths = Vec::new();
    let mut remove_paths = Vec::new();
    for edit in edits {
        match edit {
            YamlEdit::Set { path, value } => {
                set_value(&mut after_sets, path, value.clone())
                    .map_err(|error| at.invalid_path(&original, path, &error))?;
                set_paths.push(path);
            }
            YamlEdit::Remove { path } => remove_paths.push(path.clone()),
        }
    }
    let plan = RemovalPlan::new(&after_sets, &remove_paths).map_err(|error| {
        let path = match &error {
            PathEditError::RemovalConflict { second, .. } => {
                remove_paths.iter().find(|path| path.as_written() == second)
            }
            _ => remove_paths.iter().find(|path| path.resolve_map(&after_sets).value().is_none()),
        };
        match path {
            Some(path) => at.invalid_path(&after_sets, path, &error),
            None => at.error(YamlEditErrorKind::InvalidPath, None, content, error.to_string()),
        }
    })?;
    let mut expected = after_sets.clone();
    plan.apply(&mut expected);
    // Edits that already hold change nothing: the source is returned as it
    // is, so a repeated save is byte-identical and renders (and resolves a
    // layout for) nothing.
    if expected == original {
        return Ok(source.to_string());
    }

    let mut sets = Vec::with_capacity(set_paths.len());
    for path in &set_paths {
        let resolved = path.resolve_map(&after_sets);
        if resolved.steps().len() != path.segments().len() {
            return Err(at.error(
                YamlEditErrorKind::InvalidPath,
                Some((*path).clone()),
                content,
                "the path does not resolve after the edit",
            ));
        }
        sets.push(PathEdit::Set(resolved.steps().to_vec()));
    }
    let removes: Vec<PathEdit<'_>> = plan.targets().map(PathEdit::Remove).collect();

    for edit in sets.iter().chain(&removes) {
        refuse_unsupported_nodes(&at, edit)?;
    }
    let text = apply_phase(&at, &working, &after_sets, &sets, layout)?;
    let mut text = apply_phase(&at, &text, &expected, &removes, layout)?;
    // Removing every top-level key leaves the root written `{}`, as removing
    // a collection's last child does, so the file still holds one mapping.
    if expected.is_empty() && !original.is_empty() && content_start(&text) >= text.len() {
        text.push_str("{}");
        text.push_str(newline);
    }

    let text = match text.strip_suffix(newline) {
        Some(stripped) if unterminated => stripped.to_string(),
        _ => text,
    };
    let all: Vec<PathEdit<'_>> = sets.iter().chain(&removes).cloned().collect();
    let reloaded = load_values(&text).map_err(|failure| at.load_failure(failure, None))?;
    if let Some(difference) = unexpected_difference(&expected, &reloaded, &all) {
        return Err(at.difference(difference.path, difference.named));
    }
    Ok(format!("{bom}{text}"))
}

/// The pure form of a `Set`: [`set_at`], plus appending at a sequence's
/// length and replacing a `null` parent with a mapping.
fn set_value(map: &mut FrontmatterMap, path: &PropertyPath, value: Value) -> Result<(), PathEditError> {
    let segments = path.segments();
    if let Some((last, parents)) = segments.split_last()
        && !parents.is_empty()
        && let Ok(parent) = PropertyPath::from_segments(parents.iter().cloned())
        && let Some(Value::Array(items)) = parent.resolve_map(map).value()
        && *last == items.len().to_string()
    {
        let mut items = items.clone();
        items.push(value);
        return set_at(map, &parent, Value::Array(items));
    }
    for depth in 1..segments.len() {
        let Ok(prefix) = PropertyPath::from_segments(segments[..depth].iter().cloned()) else {
            break;
        };
        match prefix.resolve_map(map).value() {
            Some(Value::Null) => {
                set_at(map, &prefix, Value::Object(serde_json::Map::new()))?;
                break;
            }
            Some(_) => {}
            None => break,
        }
    }
    set_at(map, path, value)
}

/// Applies one phase's edits one at a time, so a refusal names its edit, then
/// verifies the phase against `expected`.
fn apply_phase(
    at: &Locator<'_>,
    text: &str,
    expected: &FrontmatterMap,
    edits: &[PathEdit<'_>],
    layout: &Layout<'_>,
) -> Result<String, YamlEditError> {
    let mut text = text.to_string();
    for edit in node_edit::application_order(edits) {
        text = node_edit::apply_edits(&WholeDocument, &text, expected, std::slice::from_ref(edit), layout)
            .map_err(|refusal| at.refusal(refusal, edit))?;
    }
    let reloaded = load_values(&text).map_err(|failure| at.load_failure(failure, None))?;
    match unexpected_difference(expected, &reloaded, edits) {
        Some(difference) => Err(at.difference(difference.path, difference.named)),
        None => Ok(text),
    }
}

/// Refuses an edit whose path reaches an alias, anchor, or tag, or walks into
/// a flow collection that is not an empty one a `Set` replaces whole.
fn refuse_unsupported_nodes(at: &Locator<'_>, edit: &PathEdit<'_>) -> Result<(), YamlEditError> {
    let (PathEdit::Set(steps) | PathEdit::Remove(steps)) = edit;
    let traced = at.trace(steps);
    let setting = matches!(edit, PathEdit::Set(_));
    for (depth, node) in traced.iter().enumerate() {
        let walked_into = depth + 1 < steps.len();
        let refusal = if node.alias {
            Some((YamlEditErrorKind::Alias, "an alias"))
        } else if node.anchor {
            Some((YamlEditErrorKind::Anchor, "an anchored node"))
        } else if node.tag {
            Some((YamlEditErrorKind::Tag, "a tagged node"))
        } else if node.flow && walked_into && !(setting && node.empty_collection) {
            Some((YamlEditErrorKind::FlowCollection, "a flow collection"))
        } else {
            None
        };
        if let Some((kind, what)) = refusal {
            let reached = property_path(&steps[..=depth]);
            return Err(at.error(
                kind,
                property_path(steps),
                node.start,
                format!(
                    "the edit reaches {what} at `{}`, which cannot be edited in place",
                    reached.map_or_else(String::new, |path| path.to_string())
                ),
            ));
        }
    }
    Ok(())
}

/// The whole file is the YAML region, past a leading `---` marker line.
struct WholeDocument;

impl Envelope for WholeDocument {
    fn yaml_span(&self, document: &str) -> Result<Option<Range<usize>>, PathEditRefusal> {
        Ok(Some(document_start(document)..document.len()))
    }

    fn with_new_block(&self, document: &str, yaml: &str) -> String {
        // Unreachable: `yaml_span` always finds a region.
        format!("{document}{yaml}")
    }

    fn reload(&self, text: &str) -> Result<FrontmatterMap, PathEditRefusal> {
        load_values(text).map_err(|failure| match failure {
            LoadFailure::Parse { message, .. } if message.contains("unknown anchor") => {
                PathEditRefusal::WouldChangeUnnamed {
                    reason: format!(
                        "the edit would remove an anchor that an alias outside the edited paths \
                         still reads: {message}"
                    ),
                }
            }
            LoadFailure::Parse { message, .. } => {
                unsupported(format!("the edited text does not load: {message}"))
            }
            LoadFailure::NotAMapping => unsupported("the edited text is not a mapping"),
        })
    }
}

enum LoadFailure {
    Parse { offset: usize, message: String },
    NotAMapping,
}

/// The values of a YAML document, read with the same number admission and
/// duplicate-key rejection as frontmatter. Empty and comment-only text is an
/// empty mapping.
fn load_values(text: &str) -> Result<FrontmatterMap, LoadFailure> {
    let parse = |error: serde_yaml_ng::Error| LoadFailure::Parse {
        offset: error.location().map_or(0, |location| location.index()),
        message: error.to_string(),
    };
    match simplified_schema::numeric::load_admitted_yaml(text).map_err(parse)? {
        serde_yaml_ng::Value::Null => Ok(FrontmatterMap::new()),
        value @ serde_yaml_ng::Value::Mapping(_) => serde_yaml_ng::from_value(value).map_err(parse),
        _ => Err(LoadFailure::NotAMapping),
    }
}

/// Turns offsets in the working text into [`YamlEditError`]s located in the
/// caller's source.
struct Locator<'s> {
    source: &'s str,
    bom: usize,
    /// The source without its byte order mark, with a terminator added to an
    /// unterminated last line. Offsets in it are offsets in the source less
    /// `bom`.
    working: &'s str,
}

impl Locator<'_> {
    fn error(
        &self,
        kind: YamlEditErrorKind,
        path: Option<PropertyPath>,
        working_offset: usize,
        reason: impl Into<String>,
    ) -> YamlEditError {
        let offset = (working_offset + self.bom).min(self.source.len());
        let (line, column) = line_col_of_offset(self.source, offset);
        YamlEditError {
            kind,
            path,
            location: YamlLocation { offset, line, column },
            reason: reason.into(),
        }
    }

    /// The nodes `steps` reaches in the unedited source.
    fn trace(&self, steps: &[PathStep<'_>]) -> Vec<TracedNode> {
        node_edit::trace_path(self.working, document_start(self.working)..self.working.len(), steps)
    }

    /// Where `steps` reaches deepest in the unedited source, or the start of
    /// its content.
    fn deepest(&self, steps: &[PathStep<'_>]) -> usize {
        self.trace(steps)
            .last()
            .map_or_else(|| content_start(self.working), |node| node.start)
    }

    fn load_failure(&self, failure: LoadFailure, path: Option<PropertyPath>) -> YamlEditError {
        match failure {
            LoadFailure::Parse { offset, message } => {
                self.error(YamlEditErrorKind::Parse, path, offset, format!("the YAML could not be parsed: {message}"))
            }
            LoadFailure::NotAMapping => self.error(
                YamlEditErrorKind::NotAMapping,
                path,
                content_start(self.working),
                "the document's root is not a mapping",
            ),
        }
    }

    fn invalid_path(&self, values: &FrontmatterMap, path: &PropertyPath, error: &PathEditError) -> YamlEditError {
        let reached = path.resolve_map(values);
        self.error(
            YamlEditErrorKind::InvalidPath,
            Some(path.clone()),
            self.deepest(reached.steps()),
            error.to_string(),
        )
    }

    fn refusal(&self, refusal: PathEditRefusal, edit: &PathEdit<'_>) -> YamlEditError {
        let (PathEdit::Set(steps) | PathEdit::Remove(steps)) = edit;
        let (kind, reason) = match refusal {
            PathEditRefusal::UnsupportedForm { reason } => (YamlEditErrorKind::UnsupportedForm, reason),
            PathEditRefusal::WouldChangeUnnamed { reason } => (YamlEditErrorKind::WouldChangeUnnamed, reason),
        };
        self.error(kind, property_path(steps), self.deepest(steps), reason)
    }

    fn difference(&self, path: Vec<FrontmatterPathSegment>, named: bool) -> YamlEditError {
        let steps: Vec<PathStep<'_>> = path
            .iter()
            .map(|segment| match segment {
                FrontmatterPathSegment::Key(key) => PathStep::Key(key),
                FrontmatterPathSegment::Index(index) => PathStep::Index(*index),
            })
            .collect();
        let property = property_path(&steps);
        let written = property.as_ref().map_or_else(String::new, ToString::to_string);
        let (kind, reason) = if named {
            (YamlEditErrorKind::UnsupportedForm, format!("`{written}` does not read back as the value it was given"))
        } else {
            (
                YamlEditErrorKind::WouldChangeUnnamed,
                format!(
                    "the edit would change the value of `{written}`, which it does not name \
                     (an alias there likely reads an anchor in the edited node)"
                ),
            )
        };
        self.error(kind, property, self.deepest(&steps), reason)
    }
}

fn property_path(steps: &[PathStep<'_>]) -> Option<PropertyPath> {
    PropertyPath::from_segments(steps.iter().map(|step| match segment_of(step) {
        FrontmatterPathSegment::Key(key) => key,
        FrontmatterPathSegment::Index(index) => index.to_string(),
    }))
    .ok()
}

/// The span of an empty `{}` root, with the blanks after it and, when nothing
/// else is on its line, the line's terminator.
fn empty_flow_root(text: &str) -> Option<Range<usize>> {
    let start = content_start(text);
    let rest = text[start..].strip_prefix('{')?.trim_start_matches([' ', '\t']).strip_prefix('}')?;
    let after = rest.trim_start_matches([' ', '\t']);
    let mut end = text.len() - after.len();
    if after.starts_with("\r\n") {
        end += 2;
    } else if after.starts_with(['\n', '\r']) {
        end += 1;
    }
    Some(start..end)
}

/// The document's first line terminator, or LF when it has none.
fn first_terminator(text: &str) -> &'static str {
    match text.find(['\n', '\r']) {
        Some(index) if text[index..].starts_with("\r\n") => "\r\n",
        Some(index) if text[index..].starts_with('\r') => "\r",
        _ => "\n",
    }
}

/// Where the YAML region starts: past a leading `---` document marker line,
/// which comments and blank lines may precede.
fn document_start(text: &str) -> usize {
    let mut offset = 0;
    for line in text.split_inclusive('\n') {
        let content = line.trim_end_matches(['\n', '\r']);
        let trimmed = content.trim_start();
        let marker = content == "---"
            || content
                .strip_prefix("---")
                .is_some_and(|rest| rest.trim_start().starts_with('#') && rest.starts_with([' ', '\t']));
        if marker {
            return offset + line.len();
        }
        if !trimmed.is_empty() && !trimmed.starts_with('#') {
            return 0;
        }
        offset += line.len();
    }
    0
}

/// The offset of the first character that is neither blank nor in a comment,
/// past the document marker; the region's end when there is none.
fn content_start(text: &str) -> usize {
    let mut offset = document_start(text);
    for line in text[offset..].split_inclusive('\n') {
        let trimmed = line.trim_start();
        if !trimmed.trim_end().is_empty() && !trimmed.starts_with('#') {
            return offset + (line.len() - trimmed.len());
        }
        offset += line.len();
    }
    offset
}
