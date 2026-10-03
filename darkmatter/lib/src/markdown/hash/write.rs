//! Write-back paths for turning a [`SaveDecision`] into Markdown text.
//!
//! The decision layer ([`Markdown::plan_hash_save`]) chooses *what* to persist;
//! map-owning callers can use [`Markdown::apply_hash_save`], while callers that
//! own the source text use [`apply_hash_save_text`] to preserve every byte
//! outside the managed hash and `last_updated` nodes.

use super::options::{LAST_UPDATED_KEY, MdHashOptions};
use super::save::SaveDecision;
use crate::markdown::{
    FrontmatterMap, Markdown, MarkdownError, MarkdownResult, extract_frontmatter_block,
};
use biscuit_file::serde_yaml_ng;
use indexmap::IndexMap;
use serde::Serialize;
use std::collections::HashSet;
use std::ops::Range;

/// A semantic change to one top-level frontmatter property.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub enum FrontmatterDeltaEntry {
    /// The current document added a property.
    Addition {
        /// Property name.
        property: String,
        /// Added value.
        value: serde_json::Value,
    },
    /// The current document replaced a property's value.
    Replacement {
        /// Property name.
        property: String,
        /// Value in the snapshot.
        previous_value: serde_json::Value,
        /// Value in the current document.
        value: serde_json::Value,
    },
    /// The current document deleted a property.
    Deletion {
        /// Property name.
        property: String,
        /// Value in the snapshot.
        previous_value: serde_json::Value,
    },
}

/// Semantic top-level frontmatter changes, in document order.
#[derive(Debug, Clone, Default, PartialEq, Serialize)]
pub struct FrontmatterDelta {
    /// Additions and replacements follow current-document order; deletions
    /// follow snapshot order.
    pub entries: Vec<FrontmatterDeltaEntry>,
}

impl FrontmatterDelta {
    /// Returns whether the two frontmatter maps are semantically equivalent.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

/// The result of restoring caller-owned frontmatter properties.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct RestoredDocument {
    /// Document text with owned properties restored from the snapshot.
    pub text: String,
    /// Owned properties whose authored text or presence was restored.
    pub restored_properties: Vec<String>,
    /// Agent-authored semantic changes excluding every owned property.
    pub frontmatter_delta: FrontmatterDelta,
}

/// Restores selected top-level frontmatter nodes from a document snapshot.
///
/// Values outside `properties` are never rewritten, and must parse to the
/// same values in the result as in `current`. The returned semantic
/// delta describes current-versus-snapshot additions, replacements, and
/// deletions while ignoring formatting-only changes and all owned properties.
/// Neither input is written or otherwise mutated.
///
/// ## Errors
///
/// Returns [`MarkdownError::FrontmatterTextEdit`] when either frontmatter block
/// is malformed, is not a block mapping, contains duplicate semantic keys, or
/// cannot be edited without ambiguity; or when a restored node's anchors would
/// make an alias outside `properties` resolve to a different value.
pub fn restore_properties_text(
    current: &str,
    snapshot: &str,
    properties: &[&str],
) -> MarkdownResult<RestoredDocument> {
    let current_frontmatter = parse_text_frontmatter(current)?;
    let snapshot_frontmatter = parse_text_frontmatter(snapshot)?;
    let owned: HashSet<&str> = properties.iter().copied().collect();
    let frontmatter_delta = semantic_frontmatter_delta(
        &snapshot_frontmatter.values,
        &current_frontmatter.values,
        &owned,
    );

    let mut text = current.to_string();
    let mut restored_properties = Vec::new();
    let mut seen = HashSet::new();
    for property in properties.iter().copied() {
        if !seen.insert(property) {
            continue;
        }

        let parsed_current = parse_text_frontmatter(&text)?;
        let current_node = parsed_current.nodes.get(property);
        let snapshot_node = snapshot_frontmatter.nodes.get(property);
        let current_source = node_source(&text, &parsed_current, current_node);
        let snapshot_source = node_source(snapshot, &snapshot_frontmatter, snapshot_node);
        if current_source == snapshot_source {
            continue;
        }

        match (current_node, snapshot_node) {
            (Some(current_node), Some(_)) => {
                let range = parsed_current.absolute_node_range(current_node);
                text.replace_range(range, snapshot_source.unwrap_or_default());
            }
            (Some(current_node), None) => {
                let range = parsed_current.absolute_node_range(current_node);
                text.replace_range(range, "");
            }
            (None, Some(_)) => {
                insert_snapshot_node(&mut text, snapshot_source.unwrap_or_default())?;
            }
            (None, None) => continue,
        }
        restored_properties.push(property.to_string());
    }

    let restored = parse_text_frontmatter(&text)?;
    ensure_unmanaged_values_kept(&current_frontmatter.values, &restored.values, |property| {
        owned.contains(property)
    })?;
    Ok(RestoredDocument {
        text,
        restored_properties,
        frontmatter_delta,
    })
}

#[derive(Debug)]
struct ParsedTextFrontmatter {
    yaml_span: Option<Range<usize>>,
    nodes: IndexMap<String, TextNode>,
    values: FrontmatterMap,
}

impl ParsedTextFrontmatter {
    fn absolute_node_range(&self, node: &TextNode) -> Range<usize> {
        absolute_range(
            self.yaml_span
                .as_ref()
                .expect("a parsed text node always belongs to frontmatter"),
            node.range.clone(),
        )
    }
}

fn parse_text_frontmatter(document: &str) -> MarkdownResult<ParsedTextFrontmatter> {
    let Some(extraction) = extract_frontmatter_block(document)? else {
        return Ok(ParsedTextFrontmatter {
            yaml_span: None,
            nodes: IndexMap::new(),
            values: FrontmatterMap::new(),
        });
    };

    validate_block_mapping(extraction.yaml)?;
    let nodes = locate_all_nodes(extraction.yaml)?;
    let values = frontmatter_values(extraction.yaml)?;
    Ok(ParsedTextFrontmatter {
        yaml_span: Some(extraction.yaml_span),
        nodes,
        values,
    })
}

/// The parsed values of a frontmatter YAML block, aliases resolved.
fn frontmatter_values(yaml: &str) -> MarkdownResult<FrontmatterMap> {
    if yaml.trim().is_empty() {
        return Ok(FrontmatterMap::new());
    }
    serde_yaml_ng::from_str(yaml)
        .map_err(|error| text_edit_error(format!("frontmatter YAML could not be parsed: {error}")))
}

/// Refuses a text edit that changed the parsed value of any property for
/// which `managed` is false.
///
/// Parsing the edited text is not enough: YAML lets an anchor name be declared
/// again, and an alias resolves to the nearest preceding declaration. Replacing
/// a node that declared or introduced an anchor can re-point a surviving alias
/// at another declaration, leaving valid YAML whose unedited bytes now mean
/// something else.
fn ensure_unmanaged_values_kept(
    before: &FrontmatterMap,
    after: &FrontmatterMap,
    managed: impl Fn(&str) -> bool,
) -> MarkdownResult<()> {
    let changed = before
        .keys()
        .chain(after.keys())
        .filter(|property| !managed(property))
        .find(|property| before.get(*property) != after.get(*property));
    match changed {
        Some(property) => Err(text_edit_error(format!(
            "the edit would change the value of `{property}`, which it does not manage \
             (an alias there likely resolves to a different anchor declaration)"
        ))),
        None => Ok(()),
    }
}

fn node_source<'a>(
    document: &'a str,
    parsed: &ParsedTextFrontmatter,
    node: Option<&TextNode>,
) -> Option<&'a str> {
    node.map(|node| &document[parsed.absolute_node_range(node)])
}

fn insert_snapshot_node(document: &mut String, node_source: &str) -> MarkdownResult<()> {
    if let Some(extraction) = extract_frontmatter_block(document)? {
        document.insert_str(extraction.yaml_span.end, node_source);
        return Ok(());
    }

    let newline = detect_newline(document);
    let mut block = format!("---{newline}{node_source}");
    if !node_source.ends_with(['\n', '\r']) {
        block.push_str(newline);
    }
    block.push_str("---");
    block.push_str(newline);
    block.push_str(document);
    *document = block;
    Ok(())
}

fn semantic_frontmatter_delta(
    snapshot: &FrontmatterMap,
    current: &FrontmatterMap,
    excluded: &HashSet<&str>,
) -> FrontmatterDelta {
    let mut entries = Vec::new();
    for (property, value) in current {
        if excluded.contains(property.as_str()) {
            continue;
        }
        match snapshot.get(property) {
            None => entries.push(FrontmatterDeltaEntry::Addition {
                property: property.clone(),
                value: value.clone(),
            }),
            Some(previous_value) if previous_value != value => {
                entries.push(FrontmatterDeltaEntry::Replacement {
                    property: property.clone(),
                    previous_value: previous_value.clone(),
                    value: value.clone(),
                });
            }
            Some(_) => {}
        }
    }
    for (property, previous_value) in snapshot {
        if !excluded.contains(property.as_str()) && !current.contains_key(property) {
            entries.push(FrontmatterDeltaEntry::Deletion {
                property: property.clone(),
                previous_value: previous_value.clone(),
            });
        }
    }
    FrontmatterDelta { entries }
}

/// Applies a hash-save decision directly to authored Markdown source.
///
/// Only the complete top-level node named by [`MdHashOptions::property`] and,
/// when requested by the decision, the `last_updated` scalar are changed. Every
/// other frontmatter and body byte is retained, including each line's own
/// terminator:
///
/// - a rewritten line keeps its original LF, CRLF, or lone-CR terminator; when
///   the replacement `hash` node has more lines than the original, each extra
///   line repeats the terminator of the line before it;
/// - an inserted property takes the terminator of the line it follows;
/// - a document without frontmatter gains a minimal block terminated like the
///   document's first line (LF when it has none), placed after a leading BOM.
///
/// An empty `last_updated:` is written as `last_updated: {today}`, keeping any
/// authored comment. A scalar date changes only its value bytes, whether it
/// follows the colon or sits alone on a following line, and comment and blank
/// lines around it are kept. The rewritten frontmatter is parsed before it is returned,
/// and every property except the managed ones must parse to the value it had
/// before the edit. A decision that needs no write returns `None` without
/// parsing anything.
///
/// ## Errors
///
/// Returns [`MarkdownError::FrontmatterTextEdit`], and no text, when the YAML
/// root is not a supported block mapping or does not parse; a managed semantic
/// key occurs more than once; a managed node cannot be replaced without
/// ambiguity; a date bump meets a `last_updated` that is a sequence or
/// mapping, carries an anchor, alias, or tag, or is a block scalar or a scalar
/// continued across lines; the rewritten frontmatter
/// does not parse; or an unmanaged value would change. The last happens when
/// the replaced hash node declared an anchor whose name an earlier declaration
/// also uses: a later alias then resolves to the earlier value.
pub fn apply_hash_save_text(
    document_text: &str,
    decision: &SaveDecision,
    options: &MdHashOptions,
    today: &str,
) -> MarkdownResult<Option<String>> {
    let Some(new_stored) = decision.new_stored.as_ref() else {
        return Ok(None);
    };
    let hash_value = new_stored.to_frontmatter_value();

    let Some(extraction) = extract_frontmatter_block(document_text)? else {
        let newline = new_block_terminator(document_text);
        let mut yaml = apply_terminators(&serialize_entry(&options.property, &hash_value)?, &[newline]);
        if decision.bump_last_updated {
            yaml.push_str(&format!("{LAST_UPDATED_KEY}: {today}{newline}"));
        }
        return validated(
            &FrontmatterMap::new(),
            new_frontmatter_block(document_text, &yaml),
            options,
            decision.bump_last_updated,
        )
        .map(Some);
    };

    validate_block_mapping(extraction.yaml)?;
    let original_values = frontmatter_values(extraction.yaml)?;
    let mut updated = document_text.to_string();
    let hash_node = locate_node(extraction.yaml, &options.property)?;
    match hash_node {
        Some(node) => {
            let node_text = &extraction.yaml[node.range.clone()];
            let replacement = apply_terminators(
                &serialize_existing_entry(node_text, &node, &hash_value)?,
                &line_terminators(node_text),
            );
            let range = absolute_range(&extraction.yaml_span, node.range);
            updated.replace_range(range, &replacement);
        }
        None => {
            let insert_at = extraction.yaml_span.end;
            let replacement = apply_terminators(
                &serialize_entry(&options.property, &hash_value)?,
                &[preceding_terminator(document_text, insert_at)],
            );
            updated.insert_str(insert_at, &replacement);
        }
    }

    if decision.bump_last_updated {
        let refreshed = extract_frontmatter_block(&updated)?.ok_or_else(|| text_edit_error(
            "frontmatter disappeared while applying the managed hash",
        ))?;
        let last_updated = locate_node(refreshed.yaml, LAST_UPDATED_KEY)?;
        match last_updated {
            Some(node) => {
                let node_text = &refreshed.yaml[node.range.clone()];
                let replacement = rewrite_date_scalar(
                    node_text,
                    &node,
                    original_values.get(LAST_UPDATED_KEY),
                    today,
                )?;
                let range = absolute_range(&refreshed.yaml_span, node.range);
                if updated[range.clone()] != replacement {
                    updated.replace_range(range, &replacement);
                }
            }
            None => {
                let insert_at = refreshed.yaml_span.end;
                let newline = preceding_terminator(&updated, insert_at);
                updated.insert_str(insert_at, &format!("{LAST_UPDATED_KEY}: {today}{newline}"));
            }
        }
    }

    validated(&original_values, updated, options, decision.bump_last_updated).map(Some)
}

/// Returns `document` when its frontmatter still parses after an edit and
/// every property other than the managed hash and, when `bumped`,
/// `last_updated` keeps the value it had in `original`.
fn validated(
    original: &FrontmatterMap,
    document: String,
    options: &MdHashOptions,
    bumped: bool,
) -> MarkdownResult<String> {
    let rewritten = parse_text_frontmatter(&document).map_err(|error| {
        text_edit_error(format!("rewritten frontmatter did not parse: {error}"))
    })?;
    ensure_unmanaged_values_kept(original, &rewritten.values, |property| {
        property == options.property || (bumped && property == LAST_UPDATED_KEY)
    })?;
    Ok(document)
}

/// Prepends a frontmatter block holding `yaml` (already terminated) to a
/// document that has none, keeping a leading BOM first.
fn new_frontmatter_block(document: &str, yaml: &str) -> String {
    let (bom, rest) = match document.strip_prefix('\u{feff}') {
        Some(rest) => ("\u{feff}", rest),
        None => ("", document),
    };
    let newline = new_block_terminator(rest);
    format!("{bom}---{newline}{yaml}---{newline}{rest}")
}

/// The terminator of a new block's lines: the document's first line
/// terminator, or LF when the document has none.
fn new_block_terminator(document: &str) -> &str {
    line_spans(document)
        .first()
        .map(|line| &document[line.content_end..line.end])
        .filter(|terminator| !terminator.is_empty())
        .unwrap_or("\n")
}

/// One step of a path from the frontmatter root to a value.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum FrontmatterPathSegment {
    /// A mapping key.
    Key(String),
    /// A sequence index.
    Index(usize),
}

/// Renders a path as `a.b[2].c`.
fn dotted_path(path: &[FrontmatterPathSegment]) -> String {
    let mut out = String::new();
    for segment in path {
        match segment {
            FrontmatterPathSegment::Key(key) => {
                if !out.is_empty() {
                    out.push('.');
                }
                out.push_str(key);
            }
            FrontmatterPathSegment::Index(index) => out.push_str(&format!("[{index}]")),
        }
    }
    out
}

/// The authored source of one frontmatter string leaf.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LeafSpan {
    /// The requested path.
    pub path: Vec<FrontmatterPathSegment>,
    /// Absolute byte range of the scalar in the document, quotes and a block
    /// scalar's header included, final line break excluded. Replacing exactly
    /// these bytes leaves every other byte, and every line ending, in place.
    pub range: Range<usize>,
    /// The text Darkmatter's compose reads for this leaf. For a clipped block
    /// scalar that ends the frontmatter this lacks the trailing newline a
    /// whole-block YAML parse reports.
    pub decoded: String,
}

/// Why a requested leaf has no exact authored span.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnlocatedLeafReason {
    /// No value exists at the path.
    Missing,
    /// The value is a mapping or sequence, not a scalar.
    NotAScalar,
    /// The leaf carries an anchor, alias, or tag, or is reached through a
    /// `<<` merge; replacing its bytes would change other values.
    NodeProperties,
    /// The YAML shape is outside what the locator models (a plain item in a
    /// flow collection, a multi-line flow collection, a nested sequence).
    UnsupportedShape,
}

impl std::fmt::Display for UnlocatedLeafReason {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::Missing => "no value exists at this path",
            Self::NotAScalar => "the value is a mapping or sequence, not a string",
            Self::NodeProperties => {
                "the value uses an anchor, alias, tag, or `<<` merge"
            }
            Self::UnsupportedShape => {
                "the value's YAML layout cannot be edited in place (a plain flow-collection item, a multi-line flow collection, or a nested sequence)"
            }
        })
    }
}

/// A leaf [`locate_frontmatter_leaves`] could not locate.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnlocatedLeaf {
    /// The requested path.
    pub path: Vec<FrontmatterPathSegment>,
    /// 1-based document line of the top-level property holding the path.
    pub line: usize,
    /// Why no span was produced.
    pub reason: UnlocatedLeafReason,
}

impl UnlocatedLeaf {
    /// The path rendered as `a.b[2].c`.
    pub fn dotted_path(&self) -> String {
        dotted_path(&self.path)
    }
}

/// Failure of [`locate_frontmatter_leaves`].
#[derive(Debug)]
pub enum LeafLocateError {
    /// The frontmatter itself is malformed or not a block mapping.
    Document(MarkdownError),
    /// One requested leaf has no exact span; no partial result is returned.
    Unlocated(UnlocatedLeaf),
}

/// Locates the exact authored bytes of frontmatter string leaves.
///
/// Each path is located inside its own top-level property, so an unmodeled
/// construct elsewhere in the frontmatter never blocks it. A leaf is accepted
/// only when the decoded source text equals the value a YAML parse of the same
/// frontmatter yields, so a returned span is always safe to replace.
///
/// ## Errors
///
/// [`LeafLocateError::Document`] when the frontmatter cannot be parsed as a
/// block mapping; [`LeafLocateError::Unlocated`] for the first path that
/// cannot be located, in request order.
pub fn locate_frontmatter_leaves(
    document: &str,
    paths: &[Vec<FrontmatterPathSegment>],
) -> Result<Vec<LeafSpan>, LeafLocateError> {
    use crate::markdown::schemas::SchemaValueKind;
    use crate::markdown::schemas::decode_scalar_node;
    use crate::markdown::schemas::simplified::locate_frontmatter_value;

    if paths.is_empty() {
        return Ok(Vec::new());
    }
    let unlocated = |path: &[FrontmatterPathSegment], line: usize, reason| {
        LeafLocateError::Unlocated(UnlocatedLeaf {
            path: path.to_vec(),
            line,
            reason,
        })
    };
    let Some(extraction) = extract_frontmatter_block(document).map_err(LeafLocateError::Document)?
    else {
        return Err(unlocated(&paths[0], 1, UnlocatedLeafReason::Missing));
    };
    validate_block_mapping(extraction.yaml).map_err(LeafLocateError::Document)?;
    let nodes = locate_all_nodes(extraction.yaml).map_err(LeafLocateError::Document)?;

    // Compose reads the YAML lines without the final terminator; decode and
    // compare against that same text (see `FrontmatterExpressionLocation`).
    let yaml_start = extraction.yaml_span.start;
    let yaml_end = extraction.yaml_span.end
        - [&b"\r\n"[..], b"\n", b"\r"]
            .into_iter()
            .find(|terminator| document.as_bytes()[..extraction.yaml_span.end].ends_with(terminator))
            .map_or(0, <[u8]>::len);
    let yaml_end = yaml_end.max(yaml_start);
    let trimmed = &document[yaml_start..yaml_end];
    let parsed: serde_json::Value = if trimmed.trim().is_empty() {
        serde_json::Value::Object(serde_json::Map::new())
    } else {
        serde_yaml_ng::from_str(trimmed).map_err(|error| {
            LeafLocateError::Document(text_edit_error(format!(
                "frontmatter YAML could not be parsed: {error}"
            )))
        })?
    };
    let prefix = &document[..yaml_end];

    let mut spans = Vec::with_capacity(paths.len());
    for path in paths {
        let Some(FrontmatterPathSegment::Key(top)) = path.first() else {
            return Err(unlocated(path, 1, UnlocatedLeafReason::Missing));
        };
        let Some(node) = nodes.get(top) else {
            return Err(unlocated(path, 1, UnlocatedLeafReason::Missing));
        };
        let node_start = yaml_start + node.range.start;
        let line = document[..node_start].matches('\n').count() + 1;
        let fail = |reason| Err(unlocated(path, line, reason));

        let mut expected = &parsed;
        for segment in path {
            let next = match segment {
                FrontmatterPathSegment::Key(key) => expected.get(key.as_str()),
                FrontmatterPathSegment::Index(index) => expected.get(*index),
            };
            match next {
                Some(value) => expected = value,
                None => return fail(UnlocatedLeafReason::Missing),
            }
        }
        let Some(expected) = expected.as_str() else {
            return fail(UnlocatedLeafReason::NotAScalar);
        };
        if path
            .iter()
            .any(|segment| matches!(segment, FrontmatterPathSegment::Key(key) if key == "<<"))
        {
            return fail(UnlocatedLeafReason::NodeProperties);
        }

        let node_end = (yaml_start + node.range.end).min(yaml_end);
        let Some(root) = locate_frontmatter_value(&document[node_start..node_end], node_start)
        else {
            return fail(UnlocatedLeafReason::UnsupportedShape);
        };
        let mut located = &root;
        let mut parent_indent = 0;
        for segment in path {
            located = match (segment, &located.kind) {
                (FrontmatterPathSegment::Key(key), SchemaValueKind::Mapping(entries)) => {
                    if entries.iter().any(|entry| entry.key == "<<") {
                        return fail(UnlocatedLeafReason::NodeProperties);
                    }
                    let Some(entry) = entries.iter().find(|entry| entry.key == *key) else {
                        return fail(UnlocatedLeafReason::UnsupportedShape);
                    };
                    parent_indent = column_of(document, entry.key_span.start);
                    &entry.value
                }
                (FrontmatterPathSegment::Index(index), SchemaValueKind::Sequence(items)) => {
                    parent_indent = column_of(document, located.span.start);
                    match items.get(*index) {
                        Some(item) => item,
                        None => return fail(UnlocatedLeafReason::UnsupportedShape),
                    }
                }
                _ => return fail(UnlocatedLeafReason::UnsupportedShape),
            };
        }
        if !matches!(located.kind, SchemaValueKind::Scalar) || located.span.is_empty() {
            return fail(UnlocatedLeafReason::UnsupportedShape);
        }
        if leading_node_property(&document.as_bytes()[located.span.start..]).is_some() {
            return fail(UnlocatedLeafReason::NodeProperties);
        }
        let Some((scalar, end)) = decode_scalar_node(prefix, located.span.start, parent_indent)
            .filter(|(scalar, _)| scalar.decoded() == expected)
        else {
            return fail(UnlocatedLeafReason::UnsupportedShape);
        };
        spans.push(LeafSpan {
            path: path.clone(),
            range: located.span.start..end,
            decoded: scalar.decoded().to_string(),
        });
    }
    Ok(spans)
}

/// The character column of byte `offset` on its line of `text`.
fn column_of(text: &str, offset: usize) -> usize {
    let line_start = text[..offset].rfind(['\n', '\r']).map_or(0, |index| index + 1);
    text[line_start..offset].chars().count()
}

#[derive(Debug)]
struct TextNode {
    range: Range<usize>,
    key_end: usize,
}

#[derive(Clone, Copy)]
struct LineSpan {
    start: usize,
    content_end: usize,
    end: usize,
}

fn text_edit_error(reason: impl Into<String>) -> MarkdownError {
    MarkdownError::FrontmatterTextEdit {
        reason: reason.into(),
    }
}

fn detect_newline(source: &str) -> &'static str {
    if source.as_bytes().windows(2).any(|pair| pair == b"\r\n") {
        "\r\n"
    } else {
        "\n"
    }
}

fn line_spans(source: &str) -> Vec<LineSpan> {
    let bytes = source.as_bytes();
    let mut spans = Vec::new();
    let mut start = 0;
    let mut cursor = 0;
    while cursor < bytes.len() {
        if bytes[cursor] == b'\n' || bytes[cursor] == b'\r' {
            let terminator = if bytes[cursor] == b'\r' && bytes.get(cursor + 1) == Some(&b'\n') {
                2
            } else {
                1
            };
            spans.push(LineSpan {
                start,
                content_end: cursor,
                end: cursor + terminator,
            });
            cursor += terminator;
            start = cursor;
        } else {
            cursor += 1;
        }
    }
    if start < source.len() {
        spans.push(LineSpan {
            start,
            content_end: source.len(),
            end: source.len(),
        });
    }
    spans
}

fn validate_block_mapping(yaml: &str) -> MarkdownResult<()> {
    let first_content = yaml
        .lines()
        .map(str::trim_start)
        .find(|line| !line.is_empty() && !line.starts_with('#'));
    if first_content.is_some_and(|line| line.starts_with('{') || line.starts_with('[')) {
        return Err(text_edit_error(
            "flow-style frontmatter roots are not supported",
        ));
    }
    match serde_yaml_ng::from_str::<serde_yaml_ng::Value>(yaml) {
        Ok(serde_yaml_ng::Value::Mapping(_)) | Ok(serde_yaml_ng::Value::Null) => Ok(()),
        Ok(_) => Err(text_edit_error(
            "frontmatter must be a top-level block mapping",
        )),
        Err(error) => Err(text_edit_error(format!(
            "frontmatter YAML could not be parsed: {error}"
        ))),
    }
}

fn locate_node(yaml: &str, target: &str) -> MarkdownResult<Option<TextNode>> {
    let lines = line_spans(yaml);
    let mut nodes = Vec::new();
    let mut index = 0;
    while index < lines.len() {
        let line = lines[index];
        let content = &yaml[line.start..line.content_end];
        if content.is_empty()
            || content.chars().next().is_some_and(char::is_whitespace)
            || content.starts_with('#')
        {
            index += 1;
            continue;
        }

        let colon = mapping_colon(content).ok_or_else(|| {
            text_edit_error(format!("unsupported top-level YAML at byte {}", line.start))
        })?;
        let key_text = content[..colon].trim_end();
        let semantic = parse_semantic_key(key_text)?;
        let mut last_included_end = line.end;
        let mut cursor = index + 1;
        while cursor < lines.len() {
            let following = lines[cursor];
            let following_text = &yaml[following.start..following.content_end];
            let indentless_sequence = following_text == "-" || following_text.starts_with("- ");
            if !following_text.is_empty()
                && !following_text
                    .chars()
                    .next()
                    .is_some_and(char::is_whitespace)
                && !indentless_sequence
            {
                break;
            }
            if !following_text.is_empty() {
                last_included_end = following.end;
            }
            cursor += 1;
        }
        if semantic == target {
            nodes.push(TextNode {
                range: line.start..last_included_end,
                key_end: line.start + colon,
            });
        }
        index = cursor;
    }

    match nodes.len() {
        0 => Ok(None),
        1 => Ok(nodes.pop()),
        count => Err(text_edit_error(format!(
            "frontmatter contains {count} occurrences of semantic key `{target}`"
        ))),
    }
}

fn locate_all_nodes(yaml: &str) -> MarkdownResult<IndexMap<String, TextNode>> {
    let lines = line_spans(yaml);
    let mut nodes = IndexMap::new();
    let mut index = 0;
    while index < lines.len() {
        let line = lines[index];
        let content = &yaml[line.start..line.content_end];
        if content.is_empty()
            || content.chars().next().is_some_and(char::is_whitespace)
            || content.starts_with('#')
        {
            index += 1;
            continue;
        }

        let colon = mapping_colon(content).ok_or_else(|| {
            text_edit_error(format!("unsupported top-level YAML at byte {}", line.start))
        })?;
        let key_text = content[..colon].trim_end();
        let semantic = parse_semantic_key(key_text)?;
        let mut last_included_end = line.end;
        let mut cursor = index + 1;
        while cursor < lines.len() {
            let following = lines[cursor];
            let following_text = &yaml[following.start..following.content_end];
            let indentless_sequence = following_text == "-" || following_text.starts_with("- ");
            if !following_text.is_empty()
                && !following_text
                    .chars()
                    .next()
                    .is_some_and(char::is_whitespace)
                && !indentless_sequence
            {
                break;
            }
            if !following_text.is_empty() {
                last_included_end = following.end;
            }
            cursor += 1;
        }
        if nodes
            .insert(
                semantic.clone(),
                TextNode {
                    range: line.start..last_included_end,
                    key_end: line.start + colon,
                },
            )
            .is_some()
        {
            return Err(text_edit_error(format!(
                "frontmatter contains more than one occurrence of semantic key `{semantic}`"
            )));
        }
        index = cursor;
    }
    Ok(nodes)
}

/// The key/value separator of a top-level key line. A colon inside a plain
/// key (`a:b: c`) or a quoted one is content, not the separator.
fn mapping_colon(line: &str) -> Option<usize> {
    crate::markdown::schemas::simplified::mapping_separator(line, false)
}

/// Where the quoted scalar opening `text` ends: just past its closing quote,
/// or `0` when `text` does not open with a quote.
///
/// A quote opens quoted syntax only as a scalar's first character; inside a
/// plain scalar (`yesterday's date`, `unknown "date`) it is content, and a
/// `#` after it can still start a comment. `None` when the opening quote is
/// not closed within `text`.
fn quoted_scalar_end(text: &str) -> Option<usize> {
    let bytes = text.as_bytes();
    let quote = match bytes.first() {
        Some(&quote @ (b'\'' | b'"')) => quote,
        _ => return Some(0),
    };
    let mut index = 1;
    while index < bytes.len() {
        match bytes[index] {
            b'\\' if quote == b'"' => index += 1,
            b'\'' if quote == b'\'' && bytes.get(index + 1) == Some(&b'\'') => index += 1,
            byte if byte == quote => return Some(index + 1),
            _ => {}
        }
        index += 1;
    }
    None
}

fn parse_semantic_key(raw: &str) -> MarkdownResult<String> {
    match serde_yaml_ng::from_str::<serde_yaml_ng::Value>(raw) {
        Ok(serde_yaml_ng::Value::String(key)) => Ok(key),
        Ok(_) => Err(text_edit_error("top-level frontmatter keys must be strings")),
        Err(error) => Err(text_edit_error(format!(
            "frontmatter key could not be parsed: {error}"
        ))),
    }
}

/// Serializes one `key: value` entry with LF terminators.
fn serialize_entry(key: &str, value: &serde_json::Value) -> MarkdownResult<String> {
    let mut map = indexmap::IndexMap::new();
    map.insert(key.to_string(), value);
    serde_yaml_ng::to_string(&map)
        .map_err(|error| text_edit_error(format!("managed value could not be serialized: {error}")))
}

/// Serializes `value` under the authored key spelling of an existing node,
/// with LF terminators.
fn serialize_existing_entry(
    node_text: &str,
    node: &TextNode,
    value: &serde_json::Value,
) -> MarkdownResult<String> {
    let key_end = node.key_end - node.range.start;
    let key_source = &node_text[..key_end];
    let canonical = serialize_entry("managed", value)?;
    let colon = mapping_colon(canonical.lines().next().unwrap_or_default())
        .ok_or_else(|| text_edit_error("serialized managed value did not contain a mapping key"))?;
    Ok(format!("{key_source}{}", &canonical[colon..]))
}

/// The terminator of every line in `text`, in order (blank and comment lines
/// included); an unterminated final line contributes `""`.
fn line_terminators(text: &str) -> Vec<&str> {
    line_spans(text)
        .into_iter()
        .map(|line| &text[line.content_end..line.end])
        .collect()
}

/// Re-terminates LF-terminated `serialized` output line by line: line *i*
/// takes `terminators[i]`, and a line past the end of `terminators` (or whose
/// original was unterminated) repeats the previous line's terminator.
fn apply_terminators(serialized: &str, terminators: &[&str]) -> String {
    let mut out = String::with_capacity(serialized.len() + terminators.len());
    let mut previous = "\n";
    for (index, line) in serialized.split_inclusive('\n').enumerate() {
        let Some(content) = line.strip_suffix('\n') else {
            out.push_str(line);
            break;
        };
        if let Some(terminator) = terminators.get(index).filter(|terminator| !terminator.is_empty()) {
            previous = terminator;
        }
        out.push_str(content);
        out.push_str(previous);
    }
    out
}

/// The terminator of the line ending at `insert_at`, or LF at the start of
/// `document`.
fn preceding_terminator(document: &str, insert_at: usize) -> &'static str {
    let before = &document.as_bytes()[..insert_at];
    if before.ends_with(b"\r\n") {
        "\r\n"
    } else if before.ends_with(b"\r") {
        "\r"
    } else {
        "\n"
    }
}

/// The YAML node property (`anchor`, `alias`, or `tag`) that `value` starts
/// with, if any.
fn leading_node_property(value: &[u8]) -> Option<&'static str> {
    match value.first() {
        Some(b'&') => Some("anchor"),
        Some(b'*') => Some("alias"),
        Some(b'!') => Some("tag"),
        _ => None,
    }
}

/// Rewrites the value of a `last_updated` node to `today`.
///
/// Only the scalar's own bytes change; a value that is null by absence gains
/// the date after the colon. Every other byte of the node is kept: the
/// authored key, spacing, quote style, trailing and indented comments, blank
/// lines, and each line's terminator.
///
/// `parsed` is the node's value from a YAML parse of the same frontmatter. It,
/// not the node's line count, decides whether the value is a collection.
/// A scalar is edited in place when its whole value sits on one line, either
/// after the colon or alone on the first following line that is not blank or
/// a comment.
fn rewrite_date_scalar(
    node_text: &str,
    node: &TextNode,
    parsed: Option<&serde_json::Value>,
    today: &str,
) -> MarkdownResult<String> {
    let colon = node.key_end - node.range.start;
    let lines = line_spans(node_text);
    // Each candidate is the absolute start of a line's value-and-comment text;
    // on the key line that is the text after the colon and its spacing.
    let mut value_lines = lines.iter().enumerate().filter_map(|(index, line)| {
        let content_start = if index == 0 { colon + 1 } else { line.start };
        let content = &node_text[content_start..line.content_end];
        let start = content_start + (content.len() - content.trim_start().len());
        let text = &node_text[start..line.content_end];
        let is_value = !text.is_empty() && yaml_comment_start(text) != Some(0);
        is_value.then_some((start, text))
    });
    let value_line = value_lines.next();

    if let Some(property) =
        value_line.and_then(|(_, text)| leading_node_property(text.as_bytes()))
    {
        return Err(text_edit_error(format!(
            "`{LAST_UPDATED_KEY}` uses a YAML {property}; replacing it would change other values"
        )));
    }
    if matches!(
        parsed,
        Some(serde_json::Value::Array(_) | serde_json::Value::Object(_))
    ) {
        return Err(text_edit_error(format!(
            "`{LAST_UPDATED_KEY}` must be a scalar value, not a sequence or mapping"
        )));
    }
    let unsupported_layout = || {
        text_edit_error(format!(
            "`{LAST_UPDATED_KEY}` is a block scalar or a scalar continued across lines, \
             which cannot be rewritten in place; write it on one line"
        ))
    };

    let Some((value_start, value_text)) = value_line else {
        if !matches!(parsed, None | Some(serde_json::Value::Null)) {
            return Err(unsupported_layout());
        }
        return Ok(insert_date_after_colon(node_text, colon, today));
    };
    if value_lines.next().is_some() || value_text.starts_with(['|', '>']) {
        return Err(unsupported_layout());
    }
    let comment_start = yaml_comment_start(value_text).unwrap_or(value_text.len());
    let old_value = value_text[..comment_start].trim_end();
    // The bytes replaced must be the complete value: a quoted scalar that
    // closes on a later line, or a plain one folded over several, reads
    // differently when parsed alone.
    let alone = serde_yaml_ng::from_str::<serde_json::Value>(old_value).ok();
    if alone.as_ref() != parsed {
        return Err(unsupported_layout());
    }

    let rendered = match old_value.as_bytes().first() {
        Some(b'\'') => format!("'{today}'"),
        Some(b'"') => format!("\"{today}\""),
        _ => today.to_string(),
    };
    let value_end = value_start + old_value.len();
    Ok(format!(
        "{}{rendered}{}",
        &node_text[..value_start],
        &node_text[value_end..]
    ))
}

/// Writes `today` into a key line whose value is null by absence, keeping the
/// rest of the node. One space goes before the date, and the authored run
/// before a comment moves behind the date so `#` never touches it.
fn insert_date_after_colon(node_text: &str, colon: usize, today: &str) -> String {
    let first_line_end = line_spans(node_text)
        .first()
        .map_or(node_text.len(), |line| line.content_end);
    let key = &node_text[..=colon];
    let after_colon = &node_text[colon + 1..first_line_end];
    let rest = &node_text[first_line_end..];
    let comment = after_colon.trim_start();
    if comment.is_empty() {
        return format!("{key} {today}{rest}");
    }
    let leading = &after_colon[..after_colon.len() - comment.len()];
    let spacing = if leading.is_empty() { " " } else { leading };
    format!("{key} {today}{spacing}{comment}{rest}")
}

/// Where the comment in `value`, a scalar's text to the end of its line,
/// starts: the first `#` that opens `value` or follows whitespace, after any
/// quoted scalar that opens it (see [`quoted_scalar_end`]).
fn yaml_comment_start(value: &str) -> Option<usize> {
    let from = quoted_scalar_end(value)?;
    value[from..]
        .char_indices()
        .map(|(offset, ch)| (from + offset, ch))
        .find(|&(index, ch)| {
            ch == '#'
                && (index == 0
                    || value[..index]
                        .chars()
                        .next_back()
                        .is_some_and(char::is_whitespace))
        })
        .map(|(index, _)| index)
}

fn absolute_range(parent: &Range<usize>, child: Range<usize>) -> Range<usize> {
    parent.start + child.start..parent.start + child.end
}

impl Markdown {
    /// Applies a [`SaveDecision`] and returns the serialized Markdown to write,
    /// or `None` when the decision requires no change.
    ///
    /// The active hash property is set from `options.property`, preserving its
    /// existing position in the frontmatter when present and appending it when
    /// new (`IndexMap` insertion semantics). When the decision bumps
    /// `last_updated`, that key is set to `today`, which the caller supplies as a
    /// `YYYY-MM-DD` string so the library stays deterministic and clock-free.
    ///
    /// The source document is never mutated; mutation happens on a clone.
    /// This API reserializes the complete frontmatter map. Callers for which
    /// the authored YAML text is authoritative must use
    /// [`apply_hash_save_text`] instead.
    ///
    /// ## Returns
    ///
    /// `Some(markdown)` to write, or `None` when [`SaveDecision::new_stored`] is
    /// `None` (nothing changed, so the file is left untouched).
    ///
    /// ## Examples
    ///
    /// ```
    /// use darkmatter::markdown::{Markdown, MdHashOptions};
    ///
    /// let doc: Markdown = "---\ntitle: T\n---\n# H\n\nBody.".into();
    /// let opts = MdHashOptions::default();
    /// let decision = doc.plan_hash_save(None, &opts).unwrap();
    /// let written = doc.apply_hash_save(&decision, &opts, "2026-05-28").unwrap();
    /// assert!(written.ends_with("# H\n\nBody."));
    /// ```
    pub fn apply_hash_save(
        &self,
        decision: &SaveDecision,
        options: &MdHashOptions,
        today: &str,
    ) -> Option<String> {
        let new_stored = decision.new_stored.as_ref()?;

        let mut updated = self.clone();
        let map = updated.frontmatter_mut().as_map_mut();
        // Direct map insertion keeps the serialized value identical to the
        // parsed `StoredHash` shape and preserves key position for an existing
        // hash property; a new property appends at the end.
        map.insert(options.property.clone(), new_stored.to_frontmatter_value());
        if decision.bump_last_updated {
            map.insert(
                LAST_UPDATED_KEY.to_string(),
                serde_json::Value::String(today.to_string()),
            );
        }
        Some(updated.as_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::markdown::hash::MdHashKind;

    fn md(content: &str) -> Markdown {
        content.into()
    }

    /// The serialized body must equal the original `content()` byte-for-byte,
    /// so the written document always ends with the untouched body.
    fn assert_body_preserved(original: &Markdown, serialized: &str) {
        assert!(
            serialized.ends_with(original.content()),
            "body not preserved verbatim.\noriginal body: {:?}\nserialized: {:?}",
            original.content(),
            serialized
        );
    }

    #[test]
    fn no_change_returns_none() {
        let doc = md("---\ntitle: T\n---\n# H\n\nBody.");
        let opts = MdHashOptions::default();
        let stored = StoredFromDoc::stored(&doc, MdHashKind::Simple, &opts);

        let decision = doc.plan_hash_save(Some(&stored), &opts).unwrap();
        assert!(decision.new_stored.is_none());
        assert!(doc.apply_hash_save(&decision, &opts, "2026-05-28").is_none());
    }

    #[test]
    fn first_baseline_adds_hash_and_preserves_body() {
        let doc = md("---\ntitle: T\n---\n# H\n\nBody.");
        let opts = MdHashOptions::default();

        let decision = doc.plan_hash_save(None, &opts).unwrap();
        let written = doc.apply_hash_save(&decision, &opts, "2026-05-28").unwrap();

        assert!(written.contains("hash:"));
        assert!(written.contains("title: T"));
        assert_body_preserved(&doc, &written);
    }

    #[test]
    fn first_baseline_does_not_bump_last_updated() {
        let doc = md("---\ntitle: T\n---\n# H\n\nBody.");
        let opts = MdHashOptions::default();

        let decision = doc.plan_hash_save(None, &opts).unwrap();
        let written = doc.apply_hash_save(&decision, &opts, "2026-05-28").unwrap();

        assert!(!written.contains("last_updated"));
    }

    #[test]
    fn document_without_frontmatter_gains_a_valid_block() {
        let doc = md("# H\n\nBody only, no frontmatter.\n");
        assert!(doc.frontmatter().is_empty());
        let opts = MdHashOptions::default();

        let decision = doc.plan_hash_save(None, &opts).unwrap();
        let written = doc.apply_hash_save(&decision, &opts, "2026-05-28").unwrap();

        // A fresh frontmatter block wraps the body, which is left untouched
        // after the closing delimiter.
        assert!(written.starts_with("---\n"));
        assert!(written.contains("\n---\n"));
        assert!(written.contains("hash:"));
        assert_body_preserved(&doc, &written);

        // The written document re-parses with a populated frontmatter block.
        let reparsed: Markdown = written.into();
        assert!(!reparsed.frontmatter().is_empty());
        assert!(reparsed.content().contains("Body only, no frontmatter."));
    }

    #[test]
    fn content_change_bumps_last_updated() {
        let original = md("---\ntitle: T\n---\n# H\n\nBody.");
        let opts = MdHashOptions::default();
        let stored = StoredFromDoc::stored(&original, MdHashKind::Simple, &opts);

        let edited = md("---\ntitle: T\n---\n# H\n\nNew body.");
        let decision = edited.plan_hash_save(Some(&stored), &opts).unwrap();
        let written = edited.apply_hash_save(&decision, &opts, "2026-05-28").unwrap();

        assert!(written.contains("last_updated: 2026-05-28"));
        assert_body_preserved(&edited, &written);
    }

    #[test]
    fn updating_existing_hash_preserves_key_order() {
        // `hash` sits in the middle: its position must be preserved on update.
        let opts = MdHashOptions::default();
        let stored = crate::markdown::hash::StoredHash::parse(
            &serde_json::json!("aaaa111111111111-bbbb222222222222"),
            "hash",
        )
        .unwrap();

        // Force a body change so a rewrite is required.
        let edited = md(
            "---\ntitle: T\nhash: aaaa111111111111-bbbb222222222222\nauthor: A\n---\n# H\n\nChanged body.",
        );
        let decision = edited.plan_hash_save(Some(&stored), &opts).unwrap();
        let written = edited.apply_hash_save(&decision, &opts, "2026-05-28").unwrap();

        let reparsed: Markdown = written.into();
        let keys: Vec<&str> = reparsed
            .frontmatter()
            .as_map()
            .keys()
            .map(String::as_str)
            .collect();
        // Original three keys keep their relative order; `last_updated` is the
        // only newcomer and lands at the end.
        assert_eq!(keys, vec!["title", "hash", "author", "last_updated"]);
    }

    #[test]
    fn body_with_irregular_spacing_and_code_fences_is_byte_exact() {
        let body = "#   Heading\n\n\n   indented prose   \n\n```rust\nfn main() {\n    let x = 1;\n}\n```\n\n\ntrailing text\n\n\n";
        let source = format!("---\ntitle: T\n---\n{body}");
        let doc = md(&source);
        let opts = MdHashOptions::default();

        let decision = doc.plan_hash_save(None, &opts).unwrap();
        let written = doc.apply_hash_save(&decision, &opts, "2026-05-28").unwrap();

        assert_body_preserved(&doc, &written);
    }

    #[test]
    fn body_without_trailing_newline_is_preserved() {
        let doc = md("---\ntitle: T\n---\n# H\n\nNo trailing newline.");
        assert!(!doc.content().ends_with('\n'));
        let opts = MdHashOptions::default();

        let decision = doc.plan_hash_save(None, &opts).unwrap();
        let written = doc.apply_hash_save(&decision, &opts, "2026-05-28").unwrap();

        assert_body_preserved(&doc, &written);
    }

    #[test]
    fn custom_property_name_is_used_for_write() {
        let doc = md("---\ntitle: T\n---\n# H\n\nBody.");
        let opts = MdHashOptions {
            property: "fingerprint".to_string(),
            ..MdHashOptions::default()
        };

        let decision = doc.plan_hash_save(None, &opts).unwrap();
        let written = doc.apply_hash_save(&decision, &opts, "2026-05-28").unwrap();

        assert!(written.contains("fingerprint:"));
        assert!(!written.contains("\nhash:"));
        assert_body_preserved(&doc, &written);
    }

    fn textual_decision(stored: crate::markdown::hash::StoredHash, bump: bool) -> SaveDecision {
        SaveDecision {
            kind: stored.kind,
            new_stored: Some(stored),
            bump_last_updated: bump,
            comparison: None,
        }
    }

    fn simple_stored(value: &str) -> crate::markdown::hash::StoredHash {
        crate::markdown::hash::StoredHash {
            kind: MdHashKind::Simple,
            value: crate::markdown::hash::StoredHashValue::Flat(value.to_string()),
            ignored: Vec::new(),
        }
    }

    #[test]
    fn textual_save_preserves_authored_lf_frontmatter_and_quote_style() {
        let source = concat!(
            "---\n",
            "title: Kept # authored\n",
            "prompt: |-\n",
            "    First line  \n",
            "\n",
            "    Literal \\\"quote\\\".\n",
            "'hash': old\n",
            "last_updated: '2026-01-01' # keep\n",
            "---\n",
            "# Body\n"
        );
        let decision = textual_decision(simple_stored("1111111111111111-2222222222222222"), true);
        let written = apply_hash_save_text(
            source,
            &decision,
            &MdHashOptions::default(),
            "2026-09-01",
        )
        .unwrap()
        .unwrap();

        assert!(written.contains("title: Kept # authored\n"));
        assert!(written.contains("prompt: |-\n    First line  \n\n    Literal \\\"quote\\\".\n"));
        assert!(written.contains("'hash': 1111111111111111-2222222222222222\n"));
        assert!(written.contains("last_updated: '2026-09-01' # keep\n"));
        assert!(written.ends_with("---\n# Body\n"));
    }

    #[test]
    fn textual_save_replaces_block_node_and_preserves_crlf() {
        let source = concat!(
            "---\r\n",
            "title: Kept\r\n",
            "hash:\r\n",
            "  kind: structured\r\n",
            "  value: old\r\n",
            "  # managed comment\r\n",
            "# boundary comment\r\n",
            "author: A\r\n",
            "---\r\n",
            "Body  \r\n"
        );
        let stored = crate::markdown::hash::StoredHash {
            kind: MdHashKind::Body,
            value: crate::markdown::hash::StoredHashValue::Flat("1111111111111111".to_string()),
            ignored: Vec::new(),
        };
        let written = apply_hash_save_text(
            source,
            &textual_decision(stored, false),
            &MdHashOptions::default(),
            "2026-09-01",
        )
        .unwrap()
        .unwrap();

        assert!(written.contains("hash:\r\n  kind: body\r\n  value: '1111111111111111'\r\n"));
        assert!(written.contains("# boundary comment\r\nauthor: A\r\n"));
        assert!(written.ends_with("---\r\nBody  \r\n"));
        assert!(!written.replace("\r\n", "").contains('\n'));
    }

    #[test]
    fn textual_save_adds_minimal_frontmatter_without_changing_body() {
        let source = "# Body\r\n\r\nKept.\r\n";
        let written = apply_hash_save_text(
            source,
            &textual_decision(simple_stored("1111111111111111-2222222222222222"), false),
            &MdHashOptions::default(),
            "2026-09-01",
        )
        .unwrap()
        .unwrap();
        assert!(written.starts_with("---\r\nhash: 1111111111111111-2222222222222222\r\n---\r\n"));
        assert!(written.ends_with(source));
    }

    #[test]
    fn textual_save_rejects_duplicate_semantic_managed_keys() {
        let source = "---\nhash: old\n\"hash\": newer\n---\nBody\n";
        let err = apply_hash_save_text(
            source,
            &textual_decision(simple_stored("1111111111111111-2222222222222222"), false),
            &MdHashOptions::default(),
            "2026-09-01",
        )
        .unwrap_err();
        assert!(matches!(err, MarkdownError::FrontmatterTextEdit { .. }));
    }

    #[test]
    fn textual_save_rejects_flow_style_root() {
        let source = "---\n{title: Kept, hash: old}\n---\nBody\n";
        let err = apply_hash_save_text(
            source,
            &textual_decision(simple_stored("1111111111111111-2222222222222222"), false),
            &MdHashOptions::default(),
            "2026-09-01",
        )
        .unwrap_err();
        assert!(matches!(err, MarkdownError::FrontmatterTextEdit { .. }));
    }

    #[test]
    fn textual_no_change_does_not_parse_unsupported_source() {
        let decision = SaveDecision {
            kind: MdHashKind::Simple,
            new_stored: None,
            bump_last_updated: false,
            comparison: None,
        };
        assert!(
            apply_hash_save_text(
                "---\n{hash: duplicate, hash: ambiguous}\n---\nBody\n",
                &decision,
                &MdHashOptions::default(),
                "2026-09-01",
            )
            .unwrap()
            .is_none()
        );
    }

    #[test]
    fn textual_save_treats_indentless_sequence_as_one_node() {
        let source = concat!(
            "---\n",
            "reviewers:\n",
            "- routers\n",
            "- generated_by\n",
            "hash: old\n",
            "next: kept\n",
            "---\n",
            "Body\n"
        );
        let written = apply_hash_save_text(
            source,
            &textual_decision(simple_stored("1111111111111111-2222222222222222"), false),
            &MdHashOptions::default(),
            "2026-09-01",
        )
        .unwrap()
        .unwrap();

        assert!(written.contains(
            "reviewers:\n- routers\n- generated_by\nhash: 1111111111111111-2222222222222222\nnext: kept\n"
        ));
    }

    #[test]
    fn textual_save_preservation_matrix_covers_representations_and_newlines() {
        struct Case {
            name: &'static str,
            kind: MdHashKind,
            property: &'static str,
            authored_key: &'static str,
            old_node: &'static str,
        }

        let cases = [
            Case {
                name: "simple",
                kind: MdHashKind::Simple,
                property: "hash",
                authored_key: "hash",
                old_node: "hash: 0000000000000000-0000000000000000",
            },
            Case {
                name: "structured",
                kind: MdHashKind::Structured,
                property: "hash",
                authored_key: "hash",
                old_node: concat!(
                    "hash:\n",
                    "  kind: structured\n",
                    "  value: 0000000000000000-0000000000000000-0000000000000000-0000000000000000"
                ),
            },
            Case {
                name: "detailed",
                kind: MdHashKind::Detailed,
                property: "hash",
                authored_key: "hash",
                old_node: "hash: old",
            },
            Case {
                name: "custom-property",
                kind: MdHashKind::Simple,
                property: "fingerprint",
                authored_key: "fingerprint",
                old_node: "fingerprint: 0000000000000000-0000000000000000",
            },
            Case {
                name: "quoted-key",
                kind: MdHashKind::Simple,
                property: "hash",
                authored_key: "\"hash\"",
                old_node: "\"hash\": 0000000000000000-0000000000000000",
            },
        ];

        for newline in ["\n", "\r\n"] {
            for case in &cases {
                let prefix = [
                    "---",
                    "title: Kept # authored",
                    "prompt: |-",
                    "    First line  ",
                    "",
                    "    Second line.",
                ]
                .join(newline)
                    + newline;
                let old_node = case.old_node.replace('\n', newline);
                let suffix = [
                    "# boundary comment",
                    "author: A",
                    "---",
                    "# Heading",
                    "",
                    "Body with trailing spaces.  ",
                    "",
                ]
                .join(newline);
                let source = format!(
                    "{prefix}{old_node}{newline}last_updated: '2026-01-01' # managed{newline}{suffix}"
                );
                let options = MdHashOptions {
                    property: case.property.to_string(),
                    ..MdHashOptions::default()
                };
                let doc = md(&source);
                let stored = StoredFromDoc::stored(&doc, case.kind, &options);
                let written = apply_hash_save_text(
                    &source,
                    &textual_decision(stored, true),
                    &options,
                    "2026-09-01",
                )
                .unwrap()
                .unwrap();

                assert!(
                    written.starts_with(&prefix),
                    "{} {newline:?} changed the authored prefix:\n{written}",
                    case.name
                );
                assert!(
                    written.ends_with(&suffix),
                    "{} {newline:?} changed the authored suffix:\n{written}",
                    case.name
                );
                assert!(
                    written.contains(&format!("{}:", case.authored_key)),
                    "{} {newline:?} did not preserve the managed key spelling:\n{written}",
                    case.name
                );
                if matches!(case.kind, MdHashKind::Structured | MdHashKind::Detailed) {
                    assert!(
                        written.contains(&format!("kind: {}", case.kind)),
                        "{} {newline:?} did not use longhand output:\n{written}",
                        case.name
                    );
                }
                if newline == "\r\n" {
                    assert!(
                        !written.replace("\r\n", "").contains('\n'),
                        "{} introduced a bare LF into CRLF output:\n{written}",
                        case.name
                    );
                }
            }
        }
    }

    #[test]
    fn textual_save_flow_root_no_write_matrix_covers_newlines() {
        for newline in ["\n", "\r\n"] {
            let source = [
                "---",
                "{title: Kept, prompt: \"First line  ",
                "  Second line\", hash: 0000000000000000-0000000000000000}",
                "---",
                "First body line.  ",
                "Second body line.",
                "",
            ]
            .join(newline);
            let error = apply_hash_save_text(
                &source,
                &textual_decision(
                    simple_stored("1111111111111111-2222222222222222"),
                    false,
                ),
                &MdHashOptions::default(),
                "2026-09-01",
            )
            .unwrap_err();

            assert!(matches!(error, MarkdownError::FrontmatterTextEdit { .. }));
            assert_eq!(
                source,
                [
                    "---",
                    "{title: Kept, prompt: \"First line  ",
                    "  Second line\", hash: 0000000000000000-0000000000000000}",
                    "---",
                    "First body line.  ",
                    "Second body line.",
                    "",
                ]
                .join(newline)
            );
        }
    }

    const CANONICAL_HASH: &str = "aaaa000000000000-bbbb000000000000";
    const TODAY: &str = "2026-09-28";

    fn save_canonical(source: &str, bump: bool) -> MarkdownResult<Option<String>> {
        apply_hash_save_text(
            source,
            &textual_decision(simple_stored(CANONICAL_HASH), bump),
            &MdHashOptions::default(),
            TODAY,
        )
    }

    fn saved_canonical(source: &str, bump: bool) -> String {
        let written = save_canonical(source, bump)
            .unwrap_or_else(|error| panic!("save of {source:?} failed: {error}"))
            .expect("a decision with a new stored hash always writes");
        assert_fidelity(source, &written, bump);
        written
    }

    /// The body of `document`: the bytes after a frontmatter block, or the
    /// whole text after a leading BOM when there is no block.
    fn body_of(document: &str) -> &str {
        match extract_frontmatter_block(document).unwrap() {
            Some(extraction) => &document[extraction.body_span],
            None => document.strip_prefix('\u{feff}').unwrap_or(document),
        }
    }

    /// Re-parses both documents and asserts the output differs from the input
    /// only at the managed `hash` and, when `bumped`, `last_updated`; that the
    /// body bytes are identical; and that a leading BOM stays first.
    fn assert_fidelity(input: &str, output: &str, bumped: bool) {
        let before = parse_text_frontmatter(input)
            .unwrap_or_else(|error| panic!("input {input:?} did not parse: {error}"));
        let after = parse_text_frontmatter(output)
            .unwrap_or_else(|error| panic!("output {output:?} did not parse: {error}"));
        assert!(after.yaml_span.is_some(), "output has no frontmatter: {output:?}");
        assert!(
            after.values.contains_key("hash"),
            "output lost the managed hash: {output:?}"
        );
        if bumped {
            assert_eq!(
                after.values.get(LAST_UPDATED_KEY),
                Some(&serde_json::json!(TODAY)),
                "output did not bump `last_updated`: {output:?}"
            );
        }

        let unmanaged = |values: &FrontmatterMap| {
            let mut values = values.clone();
            values.shift_remove("hash");
            if bumped {
                values.shift_remove(LAST_UPDATED_KEY);
            }
            values
        };
        assert_eq!(
            unmanaged(&after.values),
            unmanaged(&before.values),
            "an unmanaged property changed.\ninput:  {input:?}\noutput: {output:?}"
        );
        assert_eq!(body_of(output), body_of(input), "body bytes changed: {output:?}");
        assert_eq!(
            output.starts_with('\u{feff}'),
            input.starts_with('\u{feff}'),
            "a leading BOM moved: {output:?}"
        );
    }

    /// Asserts the writer refuses `input` with `FrontmatterTextEdit`, leaving
    /// the caller's text untouched, and returns the refusal reason.
    fn assert_refused(input: &str, decision: &SaveDecision) -> String {
        let caller_owned = input.to_string();
        let before = caller_owned.clone();
        let error = apply_hash_save_text(
            &caller_owned,
            decision,
            &MdHashOptions::default(),
            TODAY,
        )
        .map(|written| panic!("expected a refusal for {input:?}, got {written:?}"))
        .unwrap_err();
        assert_eq!(caller_owned, before, "the caller's input changed");
        match error {
            MarkdownError::FrontmatterTextEdit { reason } => reason,
            other => panic!("expected FrontmatterTextEdit for {input:?}, got {other:?}"),
        }
    }

    fn assert_refused_canonical(input: &str, bump: bool) -> String {
        assert_refused(
            input,
            &textual_decision(simple_stored(CANONICAL_HASH), bump),
        )
    }

    #[test]
    fn textual_save_case1_empty_date_gets_one_space() {
        let source = "---\nhash: aaaa000000000000-bbbb000000000000\nlast_updated:\ntitle: x\n---\n";
        assert_eq!(
            saved_canonical(source, true),
            "---\nhash: aaaa000000000000-bbbb000000000000\nlast_updated: 2026-09-28\ntitle: x\n---\n"
        );
    }

    #[test]
    fn textual_save_case1_empty_date_gets_one_space_crlf() {
        let source = "---\r\nhash: aaaa000000000000-bbbb000000000000\r\nlast_updated:\r\n---\r\n";
        assert_eq!(
            saved_canonical(source, true),
            "---\r\nhash: aaaa000000000000-bbbb000000000000\r\nlast_updated: 2026-09-28\r\n---\r\n"
        );
    }

    #[test]
    fn textual_save_case2_empty_date_keeps_comment_spacing() {
        let source = "---\nhash: aaaa000000000000-bbbb000000000000\nlast_updated:   # todo\n---\n";
        assert_eq!(
            saved_canonical(source, true),
            "---\nhash: aaaa000000000000-bbbb000000000000\nlast_updated: 2026-09-28   # todo\n---\n"
        );
    }

    #[test]
    fn textual_save_case3_lf_date_line_in_crlf_file_keeps_lf() {
        let source = "---\r\nhash: aaaa000000000000-bbbb000000000000\r\nlast_updated: 2026-01-01\n---\r\nBody\r\n";
        assert_eq!(
            saved_canonical(source, true),
            "---\r\nhash: aaaa000000000000-bbbb000000000000\r\nlast_updated: 2026-09-28\n---\r\nBody\r\n"
        );
    }

    #[test]
    fn textual_save_case4_lone_cr_terminators_survive() {
        let source = "---\rhash: aaaa000000000000-bbbb000000000000\rlast_updated: 2026-01-01\r---\rBody\r";
        assert_eq!(
            saved_canonical(source, true),
            "---\rhash: aaaa000000000000-bbbb000000000000\rlast_updated: 2026-09-28\r---\rBody\r"
        );
    }

    #[test]
    fn textual_save_case5_bom_stays_before_new_block() {
        let source = "\u{feff}# Title\n\nBody\n";
        assert_eq!(
            saved_canonical(source, true),
            "\u{feff}---\nhash: aaaa000000000000-bbbb000000000000\nlast_updated: 2026-09-28\n---\n# Title\n\nBody\n"
        );
    }

    #[test]
    fn textual_save_case6_refuses_anchored_date() {
        let reason = assert_refused_canonical(
            "---\nhash: aaaa000000000000-bbbb000000000000\nlast_updated: &lu 2026-01-01\nreviewed: *lu\n---\n",
            true,
        );
        assert!(reason.contains(LAST_UPDATED_KEY), "{reason}");
        assert!(reason.contains("anchor"), "{reason}");
    }

    #[test]
    fn textual_save_refuses_aliased_tagged_and_bare_anchored_dates() {
        for (source, property) in [
            (
                "---\nd: &d 2026-01-01\nhash: aaaa000000000000-bbbb000000000000\nlast_updated: *d\n---\nBody\n",
                "alias",
            ),
            (
                "---\nhash: aaaa000000000000-bbbb000000000000\nlast_updated: !!str 2026-01-01\n---\nBody\n",
                "tag",
            ),
            (
                "---\nhash: aaaa000000000000-bbbb000000000000\nlast_updated: &a\n---\nBody\n",
                "anchor",
            ),
        ] {
            let reason = assert_refused_canonical(source, true);
            assert!(reason.contains(LAST_UPDATED_KEY), "{source:?}: {reason}");
            assert!(reason.contains(property), "{source:?}: {reason}");
        }
    }

    #[test]
    fn textual_save_repeated_save_is_byte_stable() {
        // Saving the written document again with the same decision reproduces
        // it exactly, so terminators, spacing, and a BOM do not drift.
        for source in [
            "---\nhash: aaaa000000000000-bbbb000000000000\nlast_updated:\ntitle: x\n---\n",
            "---\nhash: aaaa000000000000-bbbb000000000000\nlast_updated:   # todo\n---\n",
            "---\r\nhash: aaaa000000000000-bbbb000000000000\r\nlast_updated: 2026-01-01\n---\r\nBody\r\n",
            "---\rhash: aaaa000000000000-bbbb000000000000\rlast_updated: 2026-01-01\r---\rBody\r",
            "\u{feff}# Title\n\nBody\n",
            "---\nhash:\r\n  kind: structured\n  value: old\r\ntitle: x\r\n---\nBody\n",
        ] {
            let first = saved_canonical(source, true);
            let second = saved_canonical(&first, true);
            assert_eq!(second, first, "{source:?}");
            assert_fidelity(source, &second, true);
        }
    }

    #[test]
    fn textual_save_without_bump_ignores_date_node_properties() {
        // Only a date bump inspects `last_updated`; a hash-only save leaves an
        // anchored date and its alias untouched.
        let source = "---\nhash: old\nlast_updated: &lu 2026-01-01\nreviewed: *lu\n---\nBody\n";
        assert_eq!(
            saved_canonical(source, false),
            "---\nhash: aaaa000000000000-bbbb000000000000\nlast_updated: &lu 2026-01-01\nreviewed: *lu\n---\nBody\n"
        );
    }

    #[test]
    fn textual_save_inserted_date_inherits_preceding_terminator_in_mixed_file() {
        // An LF line before the insertion point in an otherwise CRLF file.
        assert_eq!(
            saved_canonical(
                "---\r\nhash: aaaa000000000000-bbbb000000000000\r\ntitle: x\n---\r\nBody\r\n",
                true
            ),
            "---\r\nhash: aaaa000000000000-bbbb000000000000\r\ntitle: x\nlast_updated: 2026-09-28\n---\r\nBody\r\n"
        );
        // A CRLF line before the insertion point in an otherwise LF file; the
        // LF `hash` line keeps its LF.
        assert_eq!(
            saved_canonical(
                "---\nhash: aaaa000000000000-bbbb000000000000\ntitle: x\r\n---\nBody\n",
                true
            ),
            "---\nhash: aaaa000000000000-bbbb000000000000\ntitle: x\r\nlast_updated: 2026-09-28\r\n---\nBody\n"
        );
        // Empty frontmatter: the inserted `hash` takes the opening delimiter's
        // terminator.
        assert_eq!(
            saved_canonical("---\n---\r\nBody\r\n", false),
            "---\nhash: aaaa000000000000-bbbb000000000000\n---\r\nBody\r\n"
        );
    }

    fn body_kind_stored() -> crate::markdown::hash::StoredHash {
        crate::markdown::hash::StoredHash {
            kind: MdHashKind::Body,
            value: crate::markdown::hash::StoredHashValue::Flat("1111111111111111".to_string()),
            ignored: Vec::new(),
        }
    }

    fn saved_body_kind(source: &str) -> String {
        let written = apply_hash_save_text(
            source,
            &textual_decision(body_kind_stored(), false),
            &MdHashOptions::default(),
            TODAY,
        )
        .unwrap()
        .unwrap();
        assert_fidelity(source, &written, false);
        written
    }

    #[test]
    fn textual_save_hash_only_multi_line_node_keeps_each_line_terminator() {
        // Same line count: line i keeps original line i's terminator.
        assert_eq!(
            saved_body_kind(
                "---\nhash:\r\n  kind: structured\n  value: old\r\ntitle: x\r\n---\nBody\n"
            ),
            "---\nhash:\r\n  kind: body\n  value: '1111111111111111'\r\ntitle: x\r\n---\nBody\n"
        );
        // Longer replacement: extra lines take the previous replacement line's
        // terminator.
        assert_eq!(
            saved_body_kind("---\ntitle: x\nhash: old\r\n---\nBody\n"),
            "---\ntitle: x\nhash:\r\n  kind: body\r\n  value: '1111111111111111'\r\n---\nBody\n"
        );
        // Shorter replacement: comment lines inside the replaced range count
        // as original lines.
        assert_eq!(
            saved_body_kind(
                "---\nhash:\n  kind: structured\r\n  # managed\n  value: old\r\ntitle: x\r\n---\nBody\n"
            ),
            "---\nhash:\n  kind: body\r\n  value: '1111111111111111'\ntitle: x\r\n---\nBody\n"
        );
    }

    #[test]
    fn textual_save_lone_cr_hash_only_replacement() {
        assert_eq!(
            saved_canonical("---\rtitle: x\rhash: old\r---\rBody\r", false),
            "---\rtitle: x\rhash: aaaa000000000000-bbbb000000000000\r---\rBody\r"
        );
        assert_eq!(
            saved_body_kind("---\rtitle: x\rhash: old\r---\rBody\r"),
            "---\rtitle: x\rhash:\r  kind: body\r  value: '1111111111111111'\r---\rBody\r"
        );
    }

    #[test]
    fn textual_save_keeps_bom_before_existing_frontmatter() {
        assert_eq!(
            saved_canonical(
                "\u{feff}---\nhash: old\nlast_updated: 2026-01-01\n---\nBody\n",
                true
            ),
            "\u{feff}---\nhash: aaaa000000000000-bbbb000000000000\nlast_updated: 2026-09-28\n---\nBody\n"
        );
    }

    #[test]
    fn textual_save_body_without_final_terminator_stays_unterminated() {
        assert_eq!(
            saved_canonical("---\nhash: old\n---\nBody", true),
            "---\nhash: aaaa000000000000-bbbb000000000000\nlast_updated: 2026-09-28\n---\nBody"
        );
        // No frontmatter and no terminator anywhere: the new block uses LF.
        assert_eq!(
            saved_canonical("Body", true),
            "---\nhash: aaaa000000000000-bbbb000000000000\nlast_updated: 2026-09-28\n---\nBody"
        );
    }

    #[test]
    fn textual_save_new_block_uses_first_body_terminator() {
        assert_eq!(
            saved_canonical("# T\rBody\r", true),
            "---\rhash: aaaa000000000000-bbbb000000000000\rlast_updated: 2026-09-28\r---\r# T\rBody\r"
        );
        assert_eq!(
            saved_canonical("# T\nBody\r\n", false),
            "---\nhash: aaaa000000000000-bbbb000000000000\n---\n# T\nBody\r\n"
        );
    }

    #[test]
    fn textual_save_refuses_hash_replacement_that_orphans_an_alias() {
        assert_refused_canonical("---\nhash: &h old\nother: *h\n---\nBody\n", false);
    }

    /// The parsed value of `property` in `document`'s frontmatter.
    fn parsed_value(document: &str, property: &str) -> Option<serde_json::Value> {
        parse_text_frontmatter(document)
            .unwrap()
            .values
            .get(property)
            .cloned()
    }

    #[test]
    fn textual_save_refuses_hash_replacement_that_repoints_a_reused_anchor() {
        let source = concat!(
            "---\n",
            "earlier: &h before\n",
            "hash: &h aaaa111111111111-bbbb222222222222\n",
            "mirror: *h\n",
            "last_updated: 2026-01-01\n",
            "---\n",
            "Changed body\n",
        );
        assert_eq!(
            parsed_value(source, "mirror"),
            Some(serde_json::json!("aaaa111111111111-bbbb222222222222"))
        );
        for bump in [false, true] {
            let reason = assert_refused_canonical(source, bump);
            assert!(reason.contains("`mirror`"), "bump {bump}: {reason}");
        }
    }

    #[test]
    fn textual_save_refuses_collection_hash_replacement_that_repoints_a_nested_anchor() {
        let source = concat!(
            "---\n",
            "earlier: &h before\n",
            "hash:\n",
            "  kind: simple\n",
            "  value: &h after\n",
            "mirror: *h\n",
            "last_updated: 2026-01-01\n",
            "---\n",
            "Changed body\n",
        );
        assert_eq!(parsed_value(source, "mirror"), Some(serde_json::json!("after")));
        for bump in [false, true] {
            let reason = assert_refused_canonical(source, bump);
            assert!(reason.contains("`mirror`"), "bump {bump}: {reason}");
        }
    }

    #[test]
    fn textual_save_keeps_an_alias_bound_to_a_later_declaration() {
        let source = concat!(
            "---\n",
            "earlier: &h before\n",
            "hash: &h aaaa111111111111-bbbb222222222222\n",
            "later: &h after\n",
            "mirror: *h\n",
            "last_updated: 2026-01-01\n",
            "---\n",
            "Changed body\n",
        );
        for bump in [false, true] {
            let written = saved_canonical(source, bump);
            assert_eq!(parsed_value(&written, "mirror"), Some(serde_json::json!("after")));
            assert_eq!(body_of(&written), "Changed body\n");
        }
    }

    #[test]
    fn restore_refuses_a_restoration_that_repoints_an_unowned_alias() {
        // Restoring drops the anchor `mirror` resolved to, or adds one that
        // captures it; either way an unowned value would change.
        let cases = [
            (
                "---\nearlier: &h before\nhash: &h current\nmirror: *h\n---\nBody\n",
                "---\nhash: snapshot\n---\nOld body\n",
            ),
            (
                "---\nearlier: &h before\nhash: current\nmirror: *h\n---\nBody\n",
                "---\nhash: &h snapshot\n---\nOld body\n",
            ),
        ];
        for (current, snapshot) in cases {
            let error = restore_properties_text(current, snapshot, &["hash"]).unwrap_err();
            let MarkdownError::FrontmatterTextEdit { reason } = error else {
                panic!("expected FrontmatterTextEdit for {current:?}, got {error:?}");
            };
            assert!(reason.contains("`mirror`"), "{reason}");
        }

        let restored = restore_properties_text(
            "---\nearlier: &h before\nhash: current\nmirror: *h\n---\nBody\n",
            "---\nhash: snapshot\n---\nOld body\n",
            &["hash"],
        )
        .unwrap();
        assert_eq!(parsed_value(&restored.text, "mirror"), Some(serde_json::json!("before")));
        assert_eq!(parsed_value(&restored.text, "hash"), Some(serde_json::json!("snapshot")));
    }

    #[test]
    fn textual_save_last_updated_robustness_matrix() {
        const CONTROL: &str = concat!(
            "---\n",
            "title: Kept # c\n",
            "reviewed: &r 2026-01-01\n",
            "hash: aaaa000000000000-bbbb000000000000\n",
            "last_updated: 2026-01-01\n",
            "author: A\n",
            "---\n",
            "Body\n",
        );
        const DATE_LINE: &str = "last_updated: 2026-01-01\n";
        const HASH_LINE: &str = "hash: aaaa000000000000-bbbb000000000000\n";

        const BUMPED_LINE: &str = "last_updated: 2026-09-28\n";

        enum Outcome {
            /// The written document is `CONTROL` with its date line replaced
            /// by this line (a hash row's hash is rewritten to the control's
            /// canonical line).
            Date(&'static str),
            /// The written document is the edited source with this one
            /// further edit.
            Written { from: &'static str, to: &'static str },
            Refused,
        }
        use Outcome::{Date, Refused, Written};

        let rows: [(&str, &str, &str, Outcome); 58] = [
            ("control", DATE_LINE, DATE_LINE, Date(BUMPED_LINE)),
            (
                "empty with trailing whitespace",
                DATE_LINE,
                "last_updated:   \n",
                Date(BUMPED_LINE),
            ),
            (
                "absent",
                DATE_LINE,
                "",
                Written {
                    from: "author: A\n",
                    to: "author: A\nlast_updated: 2026-09-28\n",
                },
            ),
            ("empty", DATE_LINE, "last_updated:\n", Date(BUMPED_LINE)),
            (
                "empty with comment",
                DATE_LINE,
                "last_updated:   # todo\n",
                Date("last_updated: 2026-09-28   # todo\n"),
            ),
            (
                "empty with single-space comment",
                DATE_LINE,
                "last_updated: # todo\n",
                Date("last_updated: 2026-09-28 # todo\n"),
            ),
            ("spelled null ~", DATE_LINE, "last_updated: ~\n", Date(BUMPED_LINE)),
            (
                "spelled null keeps leading whitespace",
                DATE_LINE,
                "last_updated:   null\n",
                Date("last_updated:   2026-09-28\n"),
            ),
            (
                "empty double-quoted string",
                DATE_LINE,
                "last_updated: \"\"\n",
                Date("last_updated: \"2026-09-28\"\n"),
            ),
            (
                "empty single-quoted string",
                DATE_LINE,
                "last_updated: ''\n",
                Date("last_updated: '2026-09-28'\n"),
            ),
            (
                "scalar with indented comment",
                DATE_LINE,
                "last_updated: 2026-01-01\n  # set this when the body changes\n",
                Date("last_updated: 2026-09-28\n  # set this when the body changes\n"),
            ),
            (
                "empty with indented comment",
                DATE_LINE,
                "last_updated:\n  # set this when the body changes\n",
                Date("last_updated: 2026-09-28\n  # set this when the body changes\n"),
            ),
            (
                "empty with unindented comment",
                DATE_LINE,
                "last_updated:\n# set this when the body changes\n",
                Date("last_updated: 2026-09-28\n# set this when the body changes\n"),
            ),
            (
                "plain scalar on the following line",
                DATE_LINE,
                "last_updated:\n  2026-01-01\n",
                Date("last_updated:\n  2026-09-28\n"),
            ),
            ("block scalar", DATE_LINE, "last_updated: |-\n  2026-01-01\n", Refused),
            ("block collection", DATE_LINE, "last_updated:\n  - a\n", Refused),
            ("flow sequence", DATE_LINE, "last_updated: [a]\n", Refused),
            ("flow mapping", DATE_LINE, "last_updated: {a: 1}\n", Refused),
            ("anchor", DATE_LINE, "last_updated: &lu 2026-01-01\n", Refused),
            ("alias", DATE_LINE, "last_updated: *r\n", Refused),
            ("tag", DATE_LINE, "last_updated: !!str 2026-01-01\n", Refused),
            ("bare anchor", DATE_LINE, "last_updated: &lu\n", Refused),
            (
                "duplicate key",
                DATE_LINE,
                "last_updated: 2026-01-01\n'last_updated': 2026-02-02\n",
                Refused,
            ),
            ("invalid YAML", DATE_LINE, "last_updated: [unclosed\n", Refused),
            (
                "trailing content after a quoted date",
                DATE_LINE,
                "last_updated: \"2026-01-01\" trailing\n",
                Refused,
            ),
            (
                "trailing invalid line inside frontmatter",
                "author: A\n",
                "author: A\n@invalid\n",
                Refused,
            ),
            ("number", DATE_LINE, "last_updated: 42\n", Date(BUMPED_LINE)),
            ("float", DATE_LINE, "last_updated: 1.5\n", Date(BUMPED_LINE)),
            ("boolean", DATE_LINE, "last_updated: true\n", Date(BUMPED_LINE)),
            ("mixed flow sequence", DATE_LINE, "last_updated: [a, 1]\n", Refused),
            (
                "mixed block sequence",
                DATE_LINE,
                "last_updated:\n  - a\n  - 1\n",
                Refused,
            ),
            ("all-number sequence", DATE_LINE, "last_updated: [1, 2]\n", Refused),
            ("empty sequence", DATE_LINE, "last_updated: []\n", Refused),
            ("empty mapping", DATE_LINE, "last_updated: {}\n", Refused),
            (
                "plain scalar with an apostrophe and a comment",
                DATE_LINE,
                "last_updated: yesterday's date   # keep this explanation\n",
                Date("last_updated: 2026-09-28   # keep this explanation\n"),
            ),
            (
                "plain scalar with a double quote and a comment",
                DATE_LINE,
                "last_updated: unknown \"date   # keep this explanation\n",
                Date("last_updated: 2026-09-28   # keep this explanation\n"),
            ),
            (
                "single-quoted scalar with an escaped quote and a comment",
                DATE_LINE,
                "last_updated: 'yesterday''s date'   # keep this explanation\n",
                Date("last_updated: '2026-09-28'   # keep this explanation\n"),
            ),
            (
                "double-quoted scalar and a comment",
                DATE_LINE,
                "last_updated: \"unknown date\"   # keep this explanation\n",
                Date("last_updated: \"2026-09-28\"   # keep this explanation\n"),
            ),
            (
                "ordinary plain scalar and a comment",
                DATE_LINE,
                "last_updated: ordinary   # keep this explanation\n",
                Date("last_updated: 2026-09-28   # keep this explanation\n"),
            ),
            (
                "plain scalar with an apostrophe on the following line",
                DATE_LINE,
                "last_updated:\n  yesterday's date   # keep this explanation\n",
                Date("last_updated:\n  2026-09-28   # keep this explanation\n"),
            ),
            (
                "plain scalar with a double quote on the following line",
                DATE_LINE,
                "last_updated:\n  unknown \"date   # keep this explanation\n",
                Date("last_updated:\n  2026-09-28   # keep this explanation\n"),
            ),
            (
                "unterminated quote after a leading quote",
                DATE_LINE,
                "last_updated: 'unterminated # not a comment\n",
                Refused,
            ),
            ("hash number", HASH_LINE, "hash: 42\n", Date(BUMPED_LINE)),
            ("hash boolean", HASH_LINE, "hash: false\n", Date(BUMPED_LINE)),
            ("hash explicit null", HASH_LINE, "hash: null\n", Date(BUMPED_LINE)),
            ("hash mixed sequence", HASH_LINE, "hash: [a, 1]\n", Date(BUMPED_LINE)),
            (
                "hash mixed block sequence",
                HASH_LINE,
                "hash:\n  - a\n  - 1\n",
                Date(BUMPED_LINE),
            ),
            ("hash all-number sequence", HASH_LINE, "hash: [1, 2]\n", Date(BUMPED_LINE)),
            ("hash empty sequence", HASH_LINE, "hash: []\n", Date(BUMPED_LINE)),
            ("hash empty mapping", HASH_LINE, "hash: {}\n", Date(BUMPED_LINE)),
            (
                "hash trailing content after a quoted value",
                HASH_LINE,
                "hash: \"x\" trailing\n",
                Refused,
            ),
            (
                "hash absent",
                HASH_LINE,
                "",
                Written {
                    from: "last_updated: 2026-01-01\nauthor: A\n",
                    to: "last_updated: 2026-09-28\nauthor: A\nhash: aaaa000000000000-bbbb000000000000\n",
                },
            ),
            ("hash empty", HASH_LINE, "hash:\n", Date(BUMPED_LINE)),
            ("hash block collection", HASH_LINE, "hash:\n  - a\n", Date(BUMPED_LINE)),
            ("hash flow collection", HASH_LINE, "hash: [a]\n", Date(BUMPED_LINE)),
            ("hash alias", HASH_LINE, "hash: *r\n", Date(BUMPED_LINE)),
            ("hash duplicate key", HASH_LINE, "hash: x\n'hash': y\n", Refused),
            (
                "hash edit orphans an alias",
                HASH_LINE,
                "hash: &h aaaa000000000000-bbbb000000000000\nmirror: *h\n",
                Refused,
            ),
        ];

        for (name, from, to, outcome) in rows {
            let source = CONTROL.replacen(from, to, 1);
            assert!(
                name == "control" || source != CONTROL,
                "{name}: the edit did not change the fixture"
            );
            match outcome {
                Written { from, to } => {
                    let written = save_canonical(&source, true)
                        .unwrap_or_else(|error| panic!("{name}: refused: {error}"))
                        .unwrap();
                    let expected = source.replacen(from, to, 1);
                    assert_eq!(written, expected, "{name}");
                    assert_fidelity(&source, &written, true);
                }
                Date(date_line) => {
                    let written = save_canonical(&source, true)
                        .unwrap_or_else(|error| panic!("{name}: refused: {error}"))
                        .unwrap();
                    assert_eq!(written, CONTROL.replacen(DATE_LINE, date_line, 1), "{name}");
                    assert_fidelity(&source, &written, true);
                }
                Refused => {
                    assert_refused_canonical(&source, true);
                }
            }
        }
    }

    /// `lines` joined and terminated with `newline`.
    fn joined(lines: &[&str], newline: &str) -> String {
        lines.iter().map(|line| format!("{line}{newline}")).collect()
    }

    /// A document whose frontmatter holds the canonical hash, `date_lines`,
    /// and `author: A`, with every line terminated by `newline`.
    fn dated_document(date_lines: &[&str], newline: &str) -> String {
        let mut lines = vec!["---", "hash: aaaa000000000000-bbbb000000000000"];
        lines.extend_from_slice(date_lines);
        lines.extend_from_slice(&["author: A", "---", "Changed body"]);
        joined(&lines, newline)
    }

    #[test]
    fn textual_save_date_keeps_indented_comments_for_every_terminator() {
        let cases: [(&str, &[&str], &[&str]); 5] = [
            (
                "scalar then indented comment",
                &["last_updated: 2026-01-01", "  # set this when the body changes"],
                &["last_updated: 2026-09-28", "  # set this when the body changes"],
            ),
            (
                "empty then indented comment",
                &["last_updated:", "  # set this when the body changes"],
                &["last_updated: 2026-09-28", "  # set this when the body changes"],
            ),
            (
                "empty, blank lines, then indented comment",
                &["last_updated:", "", "   ", "  # set this when the body changes"],
                &["last_updated: 2026-09-28", "", "   ", "  # set this when the body changes"],
            ),
            (
                "empty with trailing comment then indented comment",
                &["last_updated:   # todo", "    # and more"],
                &["last_updated: 2026-09-28   # todo", "    # and more"],
            ),
            (
                "quoted scalar with trailing comment then indented comment",
                &["last_updated: \"2026-01-01\" # keep", "  # and more"],
                &["last_updated: \"2026-09-28\" # keep", "  # and more"],
            ),
        ];
        for newline in ["\n", "\r\n", "\r"] {
            for (name, before, after) in cases {
                let source = dated_document(before, newline);
                let written = saved_canonical(&source, true);
                assert_eq!(written, dated_document(after, newline), "{name} {newline:?}");
                assert_eq!(saved_canonical(&written, true), written, "{name} {newline:?}");
            }
        }
    }

    #[test]
    fn textual_save_date_rewrites_a_scalar_on_the_following_line() {
        let cases: [(&[&str], &[&str]); 6] = [
            (&["last_updated:", "  2026-01-01"], &["last_updated:", "  2026-09-28"]),
            (
                &["last_updated:", "    \"2026-01-01\""],
                &["last_updated:", "    \"2026-09-28\""],
            ),
            (&["last_updated:", "  '2026-01-01'"], &["last_updated:", "  '2026-09-28'"]),
            (&["last_updated:", "  ~"], &["last_updated:", "  2026-09-28"]),
            (
                &["last_updated: # todo", "  # why", "", "  2026-01-01  # was", "  # after"],
                &["last_updated: # todo", "  # why", "", "  2026-09-28  # was", "  # after"],
            ),
            (
                &["last_updated:", "  2026-01-01", "  # set this when the body changes"],
                &["last_updated:", "  2026-09-28", "  # set this when the body changes"],
            ),
        ];
        for newline in ["\n", "\r\n", "\r"] {
            for (before, after) in cases {
                let source = dated_document(before, newline);
                assert_eq!(
                    saved_canonical(&source, true),
                    dated_document(after, newline),
                    "{before:?} {newline:?}"
                );
            }
        }
    }

    #[test]
    fn textual_save_refuses_unsupported_scalar_layouts_without_calling_them_collections() {
        for date_lines in [
            &["last_updated: |-", "  2026-01-01"][..],
            &["last_updated: >", "  2026-01-01"],
            &["last_updated:", "  |-", "    2026-01-01"],
            &["last_updated: 2026-01", "  -01"],
            &["last_updated:", "  2026-01", "  -01"],
            &["last_updated: \"2026-01", "  -01\""],
            &["last_updated: \"2026-01-01", "  # inside the quotes\""],
        ] {
            let source = dated_document(date_lines, "\n");
            let reason = assert_refused_canonical(&source, true);
            assert!(reason.contains("continued across lines"), "{date_lines:?}: {reason}");
            assert!(!reason.contains("sequence or mapping"), "{date_lines:?}: {reason}");
        }
    }

    #[test]
    fn textual_save_still_refuses_a_collection_date() {
        for date_lines in [
            &["last_updated:", "  - 2026-01-01"][..],
            &["last_updated:", "- 2026-01-01"],
            &["last_updated:", "  # a comment first", "  - 2026-01-01"],
            &["last_updated:", "  on: 2026-01-01"],
            &["last_updated: [2026-01-01]"],
            &["last_updated: {on: 2026-01-01}"],
        ] {
            let source = dated_document(date_lines, "\n");
            let reason = assert_refused_canonical(&source, true);
            assert!(reason.contains("sequence or mapping"), "{date_lines:?}: {reason}");
        }
    }

    #[test]
    fn textual_save_refuses_node_properties_on_a_following_line_date() {
        for (date_lines, property) in [
            (&["last_updated:", "  &lu 2026-01-01"][..], "anchor"),
            (&["last_updated:", "  !!str 2026-01-01"], "tag"),
        ] {
            let reason = assert_refused_canonical(&dated_document(date_lines, "\n"), true);
            assert!(reason.contains(property), "{date_lines:?}: {reason}");
        }
    }

    #[test]
    fn textual_save_without_bump_leaves_an_empty_date_with_indented_comment() {
        let source = dated_document(&["last_updated:", "  # set this when the body changes"], "\r\n")
            .replace("hash: aaaa000000000000-bbbb000000000000", "hash: old");
        assert_eq!(
            saved_canonical(&source, false),
            dated_document(&["last_updated:", "  # set this when the body changes"], "\r\n")
        );
    }

    /// A date spelling and its bumped spelling: two plain scalars holding a
    /// quote as content, their quoted counterparts, and an ordinary control.
    const QUOTE_SPELLINGS: [(&str, &str); 5] = [
        ("yesterday's date", "2026-09-28"),
        ("unknown \"date", "2026-09-28"),
        ("'yesterday''s date'", "'2026-09-28'"),
        ("\"unknown date\"", "\"2026-09-28\""),
        ("ordinary", "2026-09-28"),
    ];

    #[test]
    fn textual_save_date_keeps_the_comment_after_a_quote_inside_a_plain_scalar() {
        for newline in ["\n", "\r\n", "\r"] {
            for (authored, bumped) in QUOTE_SPELLINGS {
                let layouts = [
                    (
                        vec![format!("last_updated: {authored}   # keep this explanation")],
                        vec![format!("last_updated: {bumped}   # keep this explanation")],
                    ),
                    (
                        vec![
                            "last_updated:".to_string(),
                            format!("  {authored}   # keep this explanation"),
                        ],
                        vec![
                            "last_updated:".to_string(),
                            format!("  {bumped}   # keep this explanation"),
                        ],
                    ),
                ];
                for (before, after) in &layouts {
                    let before: Vec<&str> = before.iter().map(String::as_str).collect();
                    let after: Vec<&str> = after.iter().map(String::as_str).collect();
                    let source = dated_document(&before, newline);
                    let written = saved_canonical(&source, true);
                    assert_eq!(written, dated_document(&after, newline), "{before:?} {newline:?}");
                    assert_eq!(saved_canonical(&written, true), written, "{before:?} {newline:?}");

                    let unbumped = source
                        .replace("hash: aaaa000000000000-bbbb000000000000", "hash: old");
                    assert_eq!(
                        saved_canonical(&unbumped, false),
                        source,
                        "no bump: {before:?} {newline:?}"
                    );
                }
            }
        }
    }

    #[test]
    fn textual_save_and_restore_accept_a_quote_inside_a_plain_key() {
        for newline in ["\n", "\r\n", "\r"] {
            let source = joined(
                &[
                    "---",
                    "author's note: it's \"fine\"   # keep",
                    "say \"hi: there",
                    "hash: old",
                    "last_updated: yesterday's date   # keep this explanation",
                    "---",
                    "Body",
                ],
                newline,
            );
            let written = saved_canonical(&source, true);
            let expected = source
                .replace("hash: old", "hash: aaaa000000000000-bbbb000000000000")
                .replace("yesterday's date", TODAY);
            assert_eq!(written, expected, "{newline:?}");

            let restored = restore_properties_text(&written, &source, &["hash", "author's note"])
                .unwrap_or_else(|error| panic!("{newline:?}: {error}"));
            assert_eq!(
                restored.text,
                source.replace("yesterday's date", TODAY),
                "{newline:?}"
            );
            assert_eq!(restored.restored_properties, ["hash"], "{newline:?}");
        }
    }

    /// A `:` inside a plain top-level key (`a:b: one`) is content: the key is
    /// `a:b`, not `a`, so it neither duplicates `a` nor shadows `hash`.
    #[test]
    fn textual_save_and_restore_read_a_content_colon_as_part_of_a_plain_key() {
        for newline in ["\n", "\r\n", "\r"] {
            let source = joined(
                &[
                    "---",
                    "a:b: one",
                    "a: two",
                    "a:'b: it's",
                    "hash:x: keep",
                    "hash: old",
                    "last_updated:x: yesterday",
                    "last_updated: yesterday",
                    "---",
                    "Body",
                ],
                newline,
            );
            let written = saved_canonical(&source, true);
            let expected = source
                .replace("hash: old", "hash: aaaa000000000000-bbbb000000000000")
                .replace("last_updated: yesterday", &format!("last_updated: {TODAY}"));
            assert_eq!(written, expected, "{newline:?}");

            let restored =
                restore_properties_text(&written, &source, &["hash", "a:b", "a:'b", "hash:x"])
                    .unwrap_or_else(|error| panic!("{newline:?}: {error}"));
            assert_eq!(
                restored.text,
                source.replace("last_updated: yesterday", &format!("last_updated: {TODAY}")),
                "{newline:?}"
            );
            assert_eq!(restored.restored_properties, ["hash"], "{newline:?}");
        }
    }

    /// Test helper: a stored hash computed from a document at a kind.
    struct StoredFromDoc;
    impl StoredFromDoc {
        fn stored(
            doc: &Markdown,
            kind: MdHashKind,
            opts: &MdHashOptions,
        ) -> crate::markdown::hash::StoredHash {
            crate::markdown::hash::StoredHash {
                kind,
                value: doc.compute_hash(kind, opts).to_stored_value(),
                ignored: Vec::new(),
            }
        }
    }

    #[test]
    fn restore_reports_semantic_additions_replacements_and_deletions() {
        let snapshot = concat!(
            "---\n",
            "title: Original\n",
            "replaced: before\n",
            "typed: 42\n",
            "deleted: remove me\n",
            "prompt: |-\n",
            "    Keep this exact prompt.  \n",
            "---\n",
            "Original body.\n",
        );
        let current = concat!(
            "---\n",
            "title: 'Original'\n",
            "replaced: after\n",
            "typed: '42'\n",
            "added: 42\n",
            "prompt: agent rewrite\n",
            "---\n",
            "Agent body.\n",
        );

        let restored = restore_properties_text(current, snapshot, &["prompt"]).unwrap();

        assert_eq!(restored.restored_properties, ["prompt"]);
        assert!(restored.text.contains("title: 'Original'\n"));
        assert!(restored.text.contains("prompt: |-\n    Keep this exact prompt.  \n"));
        assert!(restored.text.ends_with("---\nAgent body.\n"));
        assert_eq!(
            restored.frontmatter_delta.entries,
            [
                FrontmatterDeltaEntry::Replacement {
                    property: "replaced".into(),
                    previous_value: serde_json::json!("before"),
                    value: serde_json::json!("after"),
                },
                FrontmatterDeltaEntry::Replacement {
                    property: "typed".into(),
                    previous_value: serde_json::json!(42),
                    value: serde_json::json!("42"),
                },
                FrontmatterDeltaEntry::Addition {
                    property: "added".into(),
                    value: serde_json::json!(42),
                },
                FrontmatterDeltaEntry::Deletion {
                    property: "deleted".into(),
                    previous_value: serde_json::json!("remove me"),
                },
            ]
        );
    }

    #[test]
    fn restore_adds_and_deletes_owned_properties_from_the_snapshot() {
        let added = restore_properties_text(
            "---\ntitle: T\n---\nBody\n",
            "---\ntitle: T\nprompt: original\n---\nOld body\n",
            &["prompt"],
        )
        .unwrap();
        assert_eq!(
            added.text,
            "---\ntitle: T\nprompt: original\n---\nBody\n"
        );
        assert_eq!(added.restored_properties, ["prompt"]);
        assert!(added.frontmatter_delta.is_empty());

        let added_block = restore_properties_text(
            "Body\n",
            "---\nprompt: original\n---\nOld body\n",
            &["prompt"],
        )
        .unwrap();
        assert_eq!(added_block.text, "---\nprompt: original\n---\nBody\n");
        assert_eq!(added_block.restored_properties, ["prompt"]);

        let deleted = restore_properties_text(
            "---\ntitle: T\nprompt: added by agent\n---\nBody\n",
            "---\ntitle: T\n---\nOld body\n",
            &["prompt"],
        )
        .unwrap();
        assert_eq!(deleted.text, "---\ntitle: T\n---\nBody\n");
        assert_eq!(deleted.restored_properties, ["prompt"]);
        assert!(deleted.frontmatter_delta.is_empty());
    }

    #[test]
    fn restore_preserves_byte_forms_and_round_trips_for_lf_and_crlf() {
        for newline in ["\n", "\r\n"] {
            let snapshot = [
                "---",
                "title: Kept # authored",
                "prompt: |-",
                "    First line  ",
                "",
                "    Second line.",
                r"windows_path: C:\Users\Ken Snyder\notes.md",
                "unix_path: /Users/Ken Snyder/notes.md",
                "ordered_after: true",
                "---",
                "Original body.",
                "",
            ]
            .join(newline);
            let current = [
                "---",
                "title: Kept # authored",
                "prompt: agent rewrite",
                r"windows_path: C:\Users\Ken Snyder\notes.md",
                "unix_path: /Users/Ken Snyder/notes.md",
                "ordered_after: true",
                "---",
                "Agent body with trailing spaces.  ",
                "",
            ]
            .join(newline);
            let expected = [
                "---",
                "title: Kept # authored",
                "prompt: |-",
                "    First line  ",
                "",
                "    Second line.",
                r"windows_path: C:\Users\Ken Snyder\notes.md",
                "unix_path: /Users/Ken Snyder/notes.md",
                "ordered_after: true",
                "---",
                "Agent body with trailing spaces.  ",
                "",
            ]
            .join(newline);

            let first = restore_properties_text(&current, &snapshot, &["prompt"]).unwrap();
            assert_eq!(first.text, expected, "newline {newline:?}");
            assert_eq!(first.restored_properties, ["prompt"]);
            assert!(first.frontmatter_delta.is_empty());

            let second = restore_properties_text(&first.text, &snapshot, &["prompt"]).unwrap();
            assert_eq!(second.text, first.text);
            assert!(second.restored_properties.is_empty());
            assert!(second.frontmatter_delta.is_empty());
        }
    }

    #[test]
    fn restore_rejects_malformed_and_duplicate_frontmatter_without_output() {
        let malformed = "---\nprompt: [unterminated\n---\nBody\n";
        let duplicate = "---\nprompt: first\n\"prompt\": second\n---\nBody\n";
        let snapshot = "---\nprompt: original\n---\nOld body\n";

        for current in [malformed, duplicate] {
            let error = restore_properties_text(current, snapshot, &["prompt"]).unwrap_err();
            assert!(matches!(error, MarkdownError::FrontmatterTextEdit { .. }));
        }

        let error = restore_properties_text(
            "---\nprompt: current\n---\nBody\n",
            duplicate,
            &["prompt"],
        )
        .unwrap_err();
        assert!(matches!(error, MarkdownError::FrontmatterTextEdit { .. }));
    }

    #[test]
    fn non_strict_body_hash_ignores_boundaries_but_not_internal_whitespace() {
        let baseline = md("---\ntitle: T\n---\nFirst line\nSecond line\n");
        let boundary_only = md("---\ntitle: T\n---\n\n  First line\nSecond line  \n\n");
        let internal_change = md("---\ntitle: T\n---\nFirst  line\nSecond line\n");

        assert_eq!(baseline.hash_body(false), boundary_only.hash_body(false));
        assert_ne!(baseline.hash_body(false), internal_change.hash_body(false));
    }
}

#[cfg(test)]
mod leaf_tests {
    //! `locate_frontmatter_leaves`: the spike S2 shape table, one case per row.

    use super::*;
    use FrontmatterPathSegment::{Index, Key};

    const TOKEN: &str = "\"{{!data:v1:SGk}}\"";

    fn key(name: &str) -> FrontmatterPathSegment {
        Key(name.to_string())
    }

    /// Replaces the leaf at `path` with a quoted token and checks the reparse
    /// changes that leaf only and keeps every line ending.
    fn replace_one(document: &str, path: Vec<FrontmatterPathSegment>) -> String {
        let spans = locate_frontmatter_leaves(document, std::slice::from_ref(&path))
            .unwrap_or_else(|error| panic!("{path:?} in {document:?}: {error:?}"));
        let span = &spans[0];
        let mut replaced = document.to_string();
        replaced.replace_range(span.range.clone(), TOKEN);

        let before = parse_text_frontmatter(document).unwrap().values;
        let after = parse_text_frontmatter(&replaced).unwrap().values;
        let mut expected = serde_json::Value::Object(before.into_iter().collect());
        let mut slot = &mut expected;
        for segment in &path {
            slot = match segment {
                Key(key) => slot.get_mut(key.as_str()).unwrap(),
                Index(index) => slot.get_mut(*index).unwrap(),
            };
        }
        *slot = serde_json::Value::String("{{!data:v1:SGk}}".to_string());
        assert_eq!(
            serde_json::Value::Object(after.into_iter().collect()),
            expected,
            "{replaced}"
        );
        if document.contains("\r\n") {
            assert!(
                !replaced.replace("\r\n", "").contains('\n'),
                "a line ending changed: {replaced:?}"
            );
        }
        replaced
    }

    fn reason(document: &str, path: Vec<FrontmatterPathSegment>) -> UnlocatedLeafReason {
        match locate_frontmatter_leaves(document, &[path]) {
            Err(LeafLocateError::Unlocated(unlocated)) => unlocated.reason,
            other => panic!("expected an unlocated leaf, got {other:?}"),
        }
    }

    #[test]
    fn top_level_scalars_in_every_quoting_style_keep_their_comment() {
        let document = "---\na: plain {{x}} # note\nb: 'single ''q'' {{x}}'\nc: \"double \\\" {{x}}\"\n---\nBody\n";
        let replaced = replace_one(document, vec![key("a")]);
        assert!(replaced.contains(&format!("a: {TOKEN} # note\n")), "{replaced}");
        replace_one(document, vec![key("b")]);
        replace_one(document, vec![key("c")]);
        let spans = locate_frontmatter_leaves(document, &[vec![key("b")]]).unwrap();
        assert_eq!(spans[0].decoded, "single 'q' {{x}}");
    }

    #[test]
    fn keys_and_values_holding_a_quote_inside_a_plain_scalar_are_located() {
        // Nested leaves are not located under lone-CR line endings at all.
        for newline in ["\n", "\r\n"] {
            let document = [
                "---",
                "it's: v {{x}} # it's a note",
                "meta:",
                "  author's note: n {{x}}",
                "  say \"hi: h {{x}}   # keep",
                "  title: t {{x}}",
                "list:",
                "  - don't: d {{x}}",
                "    other: o",
                "flow: {'k''s': 'v {{x}}'}",
                "---",
                "Body",
                "",
            ]
            .join(newline);
            let replaced = replace_one(&document, vec![key("it's")]);
            assert!(
                replaced.contains(&format!("it's: {TOKEN} # it's a note{newline}")),
                "{replaced:?}"
            );
            replace_one(&document, vec![key("meta"), key("author's note")]);
            let replaced = replace_one(&document, vec![key("meta"), key("say \"hi")]);
            assert!(
                replaced.contains(&format!("say \"hi: {TOKEN}   # keep{newline}")),
                "{replaced:?}"
            );
            replace_one(&document, vec![key("meta"), key("title")]);
            replace_one(&document, vec![key("list"), Index(0), key("don't")]);
            replace_one(&document, vec![key("flow"), key("k's")]);
        }
    }

    #[test]
    fn nested_maps_sequences_and_block_scalars_are_located() {
        let document = "---\nouter:\n  inner:\n    deep: v {{x}}\nlist:\n  - one\n  - name: n {{x}}\n    other: o\nblock: |-\n  line one {{x}}\n  line two\nlast: >\n  folded {{x}}\n---\n";
        replace_one(document, vec![key("outer"), key("inner"), key("deep")]);
        replace_one(document, vec![key("list"), Index(0)]);
        replace_one(document, vec![key("list"), Index(1), key("other")]);
        let replaced = replace_one(document, vec![key("block")]);
        assert!(replaced.contains(&format!("block: {TOKEN}\nlast:")), "{replaced}");
        // A clipped block ending the frontmatter decodes as compose reads it.
        let spans = locate_frontmatter_leaves(document, &[vec![key("last")]]);
        assert_eq!(spans.unwrap()[0].decoded, "folded {{x}}");
    }

    #[test]
    fn indentless_sequences_empty_values_and_wide_markers_are_located() {
        // `serde_yaml_ng` writes sequences without indenting them.
        let document = "---\nitems:\n- a {{x}}\n- name: n\n  note: see {{x}}\nempty:\nwide:\n-   k: kv {{x}}\n    j: jv\n---\n";
        replace_one(document, vec![key("items"), Index(0)]);
        replace_one(document, vec![key("items"), Index(1), key("note")]);
        replace_one(document, vec![key("wide"), Index(0), key("j")]);
        assert_eq!(reason(document, vec![key("empty")]), UnlocatedLeafReason::NotAScalar);
    }

    #[test]
    fn crlf_is_kept_at_every_depth() {
        let document = "---\r\na: top {{x}}\r\nm:\r\n  k: nested {{x}}\r\nb: |\r\n  one {{x}}\r\n  two\r\nz: 1\r\n---\r\nBody\r\n";
        replace_one(document, vec![key("a")]);
        replace_one(document, vec![key("m"), key("k")]);
        replace_one(document, vec![key("b")]);
    }

    #[test]
    fn quoted_flow_items_are_located_and_plain_ones_are_not() {
        let document = "---\nq: [a, \"b {{x}}\"]\nm: {k: 'v {{x}}'}\np: [a, b, c]\n---\n";
        replace_one(document, vec![key("q"), Index(1)]);
        replace_one(document, vec![key("m"), key("k")]);
        assert_eq!(
            reason(document, vec![key("p"), Index(1)]),
            UnlocatedLeafReason::UnsupportedShape
        );
    }

    #[test]
    fn quoted_flow_targets_are_located_beside_plain_scalars_holding_a_quote() {
        let rows: &[(&str, &str, Vec<FrontmatterPathSegment>)] = &[
            // Controls: ordinary plain siblings and genuinely quoted spellings.
            ("[ordinary, \"{{x}}\"]", "\"{{x}}\"", vec![Index(1)]),
            ("['don''t', '{{x}}']", "'{{x}}'", vec![Index(1)]),
            ("{ordinary: '{{x}}'}", "'{{x}}'", vec![key("ordinary")]),
            ("{'don''t': \"{{x}}\"}", "\"{{x}}\"", vec![key("don't")]),
            // An internal apostrophe or double quote in a plain sibling or key.
            ("[don't, \"{{x}}\"]", "\"{{x}}\"", vec![Index(1)]),
            ("[say \"hi, '{{x}}']", "'{{x}}'", vec![Index(1)]),
            ("[don't, can't, \"{{x}}\"]", "\"{{x}}\"", vec![Index(2)]),
            ("{don't: \"{{x}}\"}", "\"{{x}}\"", vec![key("don't")]),
            ("{say \"hi: '{{x}}'}", "'{{x}}'", vec![key("say \"hi")]),
            ("{a: don't, b: \"{{x}}\"}", "\"{{x}}\"", vec![key("b")]),
            ("{a: say \"hi, b: '{{x}}'}", "'{{x}}'", vec![key("b")]),
            // Nested collections.
            ("{list: [don't, \"{{x}}\"], z: 1}", "\"{{x}}\"", vec![key("list"), Index(1)]),
            ("{list: [don't, can't, '{{x}}']}", "'{{x}}'", vec![key("list"), Index(2)]),
            ("[{k: don't}, {k: '{{x}}'}]", "'{{x}}'", vec![Index(1), key("k")]),
            ("[don't, {can't: \"{{x}}\"}]", "\"{{x}}\"", vec![Index(1), key("can't")]),
        ];
        for newline in ["\n", "\r\n"] {
            for (flow, target, inner) in rows {
                let document =
                    ["---", "prompt: test", &format!("q: {flow}"), "after: z", "---", "Body", ""]
                        .join(newline);
                let mut path = vec![key("q")];
                path.extend(inner.iter().cloned());
                let spans = locate_frontmatter_leaves(&document, std::slice::from_ref(&path))
                    .unwrap_or_else(|error| panic!("{flow:?} {newline:?}: {error:?}"));
                let start = document.find(target).unwrap();
                assert_eq!(spans[0].range, start..start + target.len(), "{flow:?} {newline:?}");
                assert_eq!(spans[0].decoded, "{{x}}", "{flow:?}");
                replace_one(&document, path);
            }
        }
    }

    /// A `:` inside a plain scalar (`a:'b`) is content, so the quote after it
    /// is content too; the quoted target beside it is still located exactly.
    #[test]
    fn quoted_flow_targets_are_located_beside_content_colons() {
        let rows: &[(&str, &str, Vec<FrontmatterPathSegment>)] = &[
            // Controls: ordinary and `don't` siblings, genuinely quoted
            // colons, and adjacent JSON-like keys.
            ("[ordinary, \"{{x}}\"]", "\"{{x}}\"", vec![Index(1)]),
            ("[don't, \"{{x}}\"]", "\"{{x}}\"", vec![Index(1)]),
            ("['a:''b', \"{{x}}\"]", "\"{{x}}\"", vec![Index(1)]),
            ("[\"a:b\", '{{x}}']", "'{{x}}'", vec![Index(1)]),
            ("{\"a\":b, c: '{{x}}'}", "'{{x}}'", vec![key("c")]),
            ("{\"k\":'{{x}}'}", "'{{x}}'", vec![key("k")]),
            ("[http://x, \"{{x}}\"]", "\"{{x}}\"", vec![Index(1)]),
            ("{url: http://x:8080, v: '{{x}}'}", "'{{x}}'", vec![key("v")]),
            // Both quote spellings after a content colon.
            ("[a:'b, \"{{x}}\"]", "\"{{x}}\"", vec![Index(1)]),
            ("[a:\"b, '{{x}}']", "'{{x}}'", vec![Index(1)]),
            ("[a:'b, c:'d, \"{{x}}\"]", "\"{{x}}\"", vec![Index(2)]),
            ("[a:\"b, c:\"d, '{{x}}']", "'{{x}}'", vec![Index(2)]),
            // Plain keys and mapping values.
            ("{a:'b: \"{{x}}\"}", "\"{{x}}\"", vec![key("a:'b")]),
            ("{a:\"b: '{{x}}'}", "'{{x}}'", vec![key("a:\"b")]),
            ("{a: a:'b, b: \"{{x}}\"}", "\"{{x}}\"", vec![key("b")]),
            ("{a: a:\"b, b: '{{x}}'}", "'{{x}}'", vec![key("b")]),
            // Nested collections.
            ("{list: [a:'b, \"{{x}}\"]}", "\"{{x}}\"", vec![key("list"), Index(1)]),
            ("{list: [a:\"b, c:\"d, '{{x}}'], z: 1}", "'{{x}}'", vec![key("list"), Index(2)]),
            ("[{k: a:'b}, {k: '{{x}}'}]", "'{{x}}'", vec![Index(1), key("k")]),
            ("[a:'b, {c:'d: \"{{x}}\"}]", "\"{{x}}\"", vec![Index(1), key("c:'d")]),
        ];
        for newline in ["\n", "\r\n"] {
            for (flow, target, inner) in rows {
                // The top-level key also holds a content colon and a quote.
                for top in ["q", "a:'q"] {
                    let document = [
                        "---",
                        "prompt: test",
                        &format!("{top}: {flow}"),
                        "after: z",
                        "---",
                        "Body",
                        "",
                    ]
                    .join(newline);
                    let mut path = vec![key(top)];
                    path.extend(inner.iter().cloned());
                    let spans = locate_frontmatter_leaves(&document, std::slice::from_ref(&path))
                        .unwrap_or_else(|error| panic!("{document:?}: {error:?}"));
                    let start = document.find(target).unwrap();
                    assert_eq!(spans[0].range, start..start + target.len(), "{document:?}");
                    assert_eq!(spans[0].decoded, "{{x}}", "{document:?}");
                    let replaced = replace_one(&document, path);
                    assert_eq!(
                        replaced,
                        format!("{}{TOKEN}{}", &document[..start], &document[start + target.len()..]),
                        "{document:?}"
                    );
                }
            }
        }
    }

    /// A parenthesis is plain-scalar content in YAML, so it never hides the
    /// `,` before a quoted target, even when a later entry closes it.
    #[test]
    fn quoted_flow_targets_are_located_beside_parentheses() {
        let rows: &[(&str, &str, Vec<FrontmatterPathSegment>)] = &[
            // Controls: parentheses inside quotes and balanced in one entry.
            ("['a(b', \"{{x}}\"]", "\"{{x}}\"", vec![Index(1)]),
            ("[\"a(b, c)d\", '{{x}}']", "'{{x}}'", vec![Index(1)]),
            ("[a(b)c, \"{{x}}\"]", "\"{{x}}\"", vec![Index(1)]),
            // Unbalanced and cross-entry balanced parentheses.
            ("[a(b, \"{{x}}\"]", "\"{{x}}\"", vec![Index(1)]),
            ("[a)b, '{{x}}']", "'{{x}}'", vec![Index(1)]),
            ("[a(b, c)d, \"{{x}}\"]", "\"{{x}}\"", vec![Index(2)]),
            ("{a: a(b, b: \"{{x}}\"}", "\"{{x}}\"", vec![key("b")]),
            ("{a(b: x, c)d: '{{x}}'}", "'{{x}}'", vec![key("c)d")]),
            // Nested collections.
            ("{list: [a(b, \"{{x}}\"]}", "\"{{x}}\"", vec![key("list"), Index(1)]),
            ("[{k: a(b}, {k: c)d}, '{{x}}']", "'{{x}}'", vec![Index(2)]),
            ("[[a(b, c)d], \"{{x}}\"]", "\"{{x}}\"", vec![Index(1)]),
        ];
        for newline in ["\n", "\r\n"] {
            for (flow, target, inner) in rows {
                let document =
                    ["---", "prompt: test", &format!("q: {flow}"), "after: z", "---", "Body", ""]
                        .join(newline);
                let mut path = vec![key("q")];
                path.extend(inner.iter().cloned());
                let spans = locate_frontmatter_leaves(&document, std::slice::from_ref(&path))
                    .unwrap_or_else(|error| panic!("{document:?}: {error:?}"));
                let start = document.find(target).unwrap();
                assert_eq!(spans[0].range, start..start + target.len(), "{document:?}");
                assert_eq!(spans[0].decoded, "{{x}}", "{document:?}");
                let replaced = replace_one(&document, path);
                assert_eq!(
                    replaced,
                    format!("{}{TOKEN}{}", &document[..start], &document[start + target.len()..]),
                    "{document:?}"
                );
            }
        }
    }

    #[test]
    fn node_properties_and_unmodeled_shapes_fail_closed() {
        let anchors = "---\na: &anc shared\nb: *anc\nc: !!str tagged\nbase: &base {k: v}\nmerged:\n  <<: *base\n  own: o\n---\n";
        assert_eq!(reason(anchors, vec![key("a")]), UnlocatedLeafReason::NodeProperties);
        assert_eq!(reason(anchors, vec![key("b")]), UnlocatedLeafReason::NodeProperties);
        assert_eq!(reason(anchors, vec![key("c")]), UnlocatedLeafReason::NodeProperties);
        assert_eq!(
            reason(anchors, vec![key("merged"), key("own")]),
            UnlocatedLeafReason::NodeProperties
        );

        // A kept block ending the frontmatter is located as compose reads it
        // (one trailing newline, not the whole block's two).
        let kept_end = "---\na: x\nkept: |+\n  keep\n\n---\n";
        let spans = locate_frontmatter_leaves(kept_end, &[vec![key("kept")]]).unwrap();
        assert_eq!(spans[0].decoded, "keep\n");

        let nested = "---\nn:\n  - - inner\n---\n";
        assert_ne!(
            locate_frontmatter_leaves(nested, &[vec![key("n"), Index(0), Index(0)]]).ok(),
            Some(Vec::new())
        );
        assert!(locate_frontmatter_leaves(nested, &[vec![key("n"), Index(0), Index(0)]]).is_err());

        let multi_line_flow = "---\nf: [\n  \"a\",\n  \"b\"\n  ]\n---\n";
        assert_eq!(
            reason(multi_line_flow, vec![key("f"), Index(0)]),
            UnlocatedLeafReason::UnsupportedShape
        );
    }

    #[test]
    fn an_unmodeled_construct_in_another_property_does_not_block() {
        let document = "---\nflow: [\n  a,\n  b\n  ]\nnote: see {{x}}\n---\n";
        replace_one(document, vec![key("note")]);
    }

    #[test]
    fn a_failure_names_the_path_and_the_line_of_its_property() {
        let document = "---\ntitle: t\nnested:\n  a: &x v\n---\n";
        let Err(LeafLocateError::Unlocated(unlocated)) =
            locate_frontmatter_leaves(document, &[vec![key("title")], vec![key("nested"), key("a")]])
        else {
            panic!("the anchored leaf must fail");
        };
        assert_eq!(unlocated.line, 3);
        assert_eq!(unlocated.dotted_path(), "nested.a");
        assert_eq!(
            reason(document, vec![key("absent")]),
            UnlocatedLeafReason::Missing
        );
    }

    #[test]
    fn malformed_frontmatter_is_a_document_error() {
        let document = "---\na: [unclosed\n---\n";
        assert!(matches!(
            locate_frontmatter_leaves(document, &[vec![key("a")]]),
            Err(LeafLocateError::Document(_))
        ));
    }
}
