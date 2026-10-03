//! Source-aware projection for passive SimplifiedSchema parsers.
//!
//! ## V1 presentation boundary
//!
//! The locator covers the SimplifiedSchema authoring presentations exercised by
//! the shipped schema corpus: plain, single-quoted, and double-quoted scalars;
//! block and flow sequences; implicit scalar-key block and flow mappings;
//! explicit scalar-key block mapping pairs in mapping roots or compact sequence
//! items; and the mapping-value anchors and scalar aliases used by that corpus.
//! This is not a general YAML concrete-syntax tree. Other YAML presentations
//! accepted by `serde_yaml_ng` are outside the v1 source-map contract and may
//! return a projection error even when semantic parsing succeeds.

use std::{collections::BTreeMap, ops::Range};

use serde_yaml_ng::Value as YamlValue;

use crate::markdown::schemas::errors::SchemaError;

use super::grammar;
use super::yaml_scalar::{self, DecodedScalar};
use super::{
    Constraint, PropertyAtom, PropertyDef, SchemaArm, SchemaDeclaration, SchemaShape,
    SimplifiedSchema, TypeExpr, parse_property_definition, parse_schema_declaration,
    parse_yaml_schema,
};

/// One segment in a structural path through a schema declaration.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum SchemaSourcePathSegment {
    /// A literal or pattern mapping property.
    Property(String),
    /// An arm of a property-level or root-level union.
    UnionArm(usize),
}

/// A structural path used to query a [`SchemaSourceMap`].
#[derive(Debug, Clone, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SchemaSourcePath(Vec<SchemaSourcePathSegment>);

impl SchemaSourcePath {
    /// Returns the declaration or property-definition root path.
    pub fn root() -> Self {
        Self::default()
    }

    /// Returns a child path for a mapping property.
    pub fn property(&self, name: impl Into<String>) -> Self {
        let mut path = self.clone();
        path.0.push(SchemaSourcePathSegment::Property(name.into()));
        path
    }

    /// Returns a child path for a union arm.
    pub fn union_arm(&self, index: usize) -> Self {
        let mut path = self.clone();
        path.0.push(SchemaSourcePathSegment::UnionArm(index));
        path
    }

    /// Borrows the structural segments in this path.
    pub fn segments(&self) -> &[SchemaSourcePathSegment] {
        &self.0
    }
}

/// The authored role described by one source-map span.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum SchemaSpanKind {
    /// The complete outer schema declaration.
    Declaration,
    /// A mapping key.
    MappingKey,
    /// A complete property definition, including YAML quoting or structure.
    Definition,
    /// One parsed property atom.
    Atom,
    /// A primitive type keyword.
    TypeKeyword,
    /// One complete constraint.
    Constraint,
    /// One constraint argument.
    Argument,
    /// The named-type portion to the left of `@`.
    ImportName,
    /// The file-reference portion to the right of `@`.
    ImportReference,
    /// One property-level or root-level union arm.
    UnionArm,
    /// A complete schema file reference.
    FileReference,
}

/// Authored byte ranges indexed by structural schema path and source role.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SchemaSourceMap {
    spans: BTreeMap<(SchemaSourcePath, SchemaSpanKind), Vec<Range<usize>>>,
}

impl SchemaSourceMap {
    /// Returns every authored span recorded for `path` and `kind`.
    pub fn spans(&self, path: &SchemaSourcePath, kind: SchemaSpanKind) -> &[Range<usize>] {
        self.spans
            .get(&(path.clone(), kind))
            .map(Vec::as_slice)
            .unwrap_or(&[])
    }

    /// Every recorded path and role with its spans, ordered by path and then
    /// role, so a caller can check that nothing is missing or extra.
    pub fn entries(
        &self,
    ) -> impl Iterator<Item = (&SchemaSourcePath, SchemaSpanKind, &[Range<usize>])> {
        self.spans
            .iter()
            .map(|((path, kind), spans)| (path, *kind, spans.as_slice()))
    }

    fn insert(&mut self, path: &SchemaSourcePath, kind: SchemaSpanKind, span: Range<usize>) {
        self.spans.entry((path.clone(), kind)).or_default().push(span);
    }
}

/// A passive semantic parse product paired with its structural source map.
#[derive(Debug, Clone)]
pub struct SourceAware<T> {
    /// The semantic product returned by the corresponding source-free parser.
    pub value: T,
    /// Exact authored byte ranges for the parsed structure.
    pub source_map: SchemaSourceMap,
}

/// Parses one property definition and projects its structure into authored
/// source byte ranges.
///
/// `yaml_source` must be the authored YAML representation of `value` itself.
/// `source_offset` moves all returned spans into the caller's document.
pub fn parse_property_definition_with_source(
    property: &str,
    value: &YamlValue,
    yaml_source: &str,
    source_offset: usize,
) -> Result<SourceAware<PropertyDef>, SchemaError> {
    let value = parse_property_definition(property, value)?;
    let located = locate_yaml_value(yaml_source)?;
    let mut source_map = SchemaSourceMap::default();
    project_property_source(
        &value,
        &located,
        &SchemaSourcePath::root(),
        source_offset,
        &mut source_map,
    )?;
    Ok(SourceAware { value, source_map })
}

/// Parses one complete schema declaration and projects its structure into
/// authored source byte ranges without resolving references.
pub fn parse_schema_declaration_with_source(
    value: &YamlValue,
    yaml_source: &str,
    source_offset: usize,
) -> Result<SourceAware<SchemaDeclaration>, SchemaError> {
    let value = parse_schema_declaration(value)?;
    let located = locate_yaml_value(yaml_source)?;
    let mut source_map = SchemaSourceMap::default();
    let root = SchemaSourcePath::root();
    source_map.insert(
        &root,
        SchemaSpanKind::Declaration,
        offset_span(&located.span, source_offset),
    );
    match &value {
        SchemaDeclaration::Reference(_) => {
            record_scalar_content(
                &located,
                &root,
                SchemaSpanKind::FileReference,
                source_offset,
                &mut source_map,
            )?;
        }
        SchemaDeclaration::Schema(schema) => {
            project_schema_source(schema, &located, source_offset, &mut source_map)?;
        }
    }
    Ok(SourceAware { value, source_map })
}

pub(super) fn parse_standalone_schema_payload_with_source(
    value: &YamlValue,
    yaml_source: &str,
    payload_key: &str,
) -> Result<SourceAware<SchemaDeclaration>, SchemaError> {
    let value = parse_schema_declaration(value)?;
    let located = locate_yaml_value(yaml_source)?;
    let LocatedKind::Mapping(pairs) = &located.kind else {
        return Err(projection_error());
    };
    let payload = pairs
        .iter()
        .find(|pair| pair.key == payload_key)
        .map(|pair| &pair.value)
        .ok_or_else(projection_error)?;
    let mut source_map = SchemaSourceMap::default();
    let root = SchemaSourcePath::root();
    source_map.insert(&root, SchemaSpanKind::Declaration, payload.span.clone());
    match &value {
        SchemaDeclaration::Reference(_) => {
            record_scalar_content(payload, &root, SchemaSpanKind::FileReference, 0, &mut source_map)?;
        }
        SchemaDeclaration::Schema(schema) => {
            project_schema_source(schema, payload, 0, &mut source_map)?;
        }
    }
    Ok(SourceAware { value, source_map })
}

/// One authored node inside a schema value, with its exact source span.
///
/// This is the structural half of the sidecar, for consumers that must point at
/// something *inside* a value the semantic parser rejects — where
/// [`parse_property_definition_with_source`] cannot help, because there is no
/// parse product to project. The shapes come from the same block/flow locator
/// the source-aware parsers use, so a `[]` inside a quoted scalar or a `:`
/// inside a regex is never mistaken for structure.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SchemaValueNode {
    /// Authored byte range of the complete node.
    pub span: Range<usize>,
    /// The node's authored shape, with its children.
    pub kind: SchemaValueKind,
}

impl SchemaValueNode {
    /// Every node in this subtree paired with its depth, deepest first.
    ///
    /// Ordering makes "the smallest authored node that fails" a single pass.
    pub fn deepest_first(&self) -> Vec<(usize, &SchemaValueNode)> {
        let mut out = Vec::new();
        self.collect(0, &mut out);
        out.sort_by_key(|(depth, _)| std::cmp::Reverse(*depth));
        out
    }

    fn collect<'a>(&'a self, depth: usize, out: &mut Vec<(usize, &'a SchemaValueNode)>) {
        out.push((depth, self));
        match &self.kind {
            SchemaValueKind::Scalar => {}
            SchemaValueKind::Mapping(entries) => {
                for entry in entries {
                    entry.value.collect(depth + 1, out);
                }
            }
            SchemaValueKind::Sequence(items) => {
                for item in items {
                    item.collect(depth + 1, out);
                }
            }
        }
    }
}

/// The authored shape of a [`SchemaValueNode`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SchemaValueKind {
    /// A plain, single-quoted, or double-quoted scalar.
    Scalar,
    /// A block or flow mapping.
    Mapping(Vec<SchemaValueEntry>),
    /// A block or flow sequence. Empty for `[]`.
    Sequence(Vec<SchemaValueNode>),
}

/// One key/value pair of a [`SchemaValueKind::Mapping`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SchemaValueEntry {
    /// The decoded mapping key.
    pub key: String,
    /// Authored byte range of the key token.
    pub key_span: Range<usize>,
    /// The pair's value node.
    pub value: SchemaValueNode,
}

/// Locates the authored structure of one schema value without interpreting it.
///
/// `yaml_source` is the value's own authored YAML text and `source_offset` its
/// byte offset in the caller's document. The source is never
/// line-ending-normalized.
///
/// ## Returns
///
/// `None` when the text is not a locatable YAML block or flow node — the signal
/// to fall back to the whole value's range.
pub fn locate_schema_value(yaml_source: &str, source_offset: usize) -> Option<SchemaValueNode> {
    locate_yaml_value(yaml_source)
        .ok()
        .map(|located| public_node(&located, source_offset))
}

/// [`locate_schema_value`] for a document's frontmatter, which also follows a
/// literal (`|`) or folded (`>`) block scalar and a multi-line quoted or plain
/// scalar over its continuation lines instead of giving up.
///
/// Schema documents keep [`locate_schema_value`]'s closed v1 grammar, which
/// rejects those presentations.
pub(crate) fn locate_frontmatter_value(
    yaml_source: &str,
    source_offset: usize,
) -> Option<SchemaValueNode> {
    locate_yaml_value_with(yaml_source, true)
        .ok()
        .map(|located| public_node(&located, source_offset))
}

fn public_node(located: &LocatedValue, offset: usize) -> SchemaValueNode {
    let kind = match &located.kind {
        LocatedKind::Scalar(_) => SchemaValueKind::Scalar,
        LocatedKind::Mapping(pairs) => SchemaValueKind::Mapping(
            pairs
                .iter()
                .map(|pair| SchemaValueEntry {
                    key: pair.key.clone(),
                    key_span: offset_span(&pair.key_span, offset),
                    value: public_node(&pair.value, offset),
                })
                .collect(),
        ),
        LocatedKind::Sequence(items) => SchemaValueKind::Sequence(
            items.iter().map(|item| public_node(item, offset)).collect(),
        ),
    };
    SchemaValueNode {
        span: offset_span(&located.span, offset),
        kind,
    }
}

#[derive(Debug, Clone)]
struct LocatedValue {
    span: Range<usize>,
    kind: LocatedKind,
}

#[derive(Debug, Clone)]
enum LocatedKind {
    Scalar(DecodedScalar),
    Mapping(Vec<LocatedPair>),
    Sequence(Vec<LocatedValue>),
}

#[derive(Debug, Clone)]
struct LocatedPair {
    key: String,
    key_span: Range<usize>,
    pair_span: Range<usize>,
    value: LocatedValue,
}

#[derive(Debug, Clone)]
struct SourceLine {
    end: usize,
    indent: usize,
    content_start: usize,
}

fn locate_yaml_value(source: &str) -> Result<LocatedValue, SchemaError> {
    locate_yaml_value_with(source, false)
}

fn locate_yaml_value_with(
    source: &str,
    multi_line_scalars: bool,
) -> Result<LocatedValue, SchemaError> {
    let lines = source_lines(source);
    if lines.is_empty() {
        return Err(projection_error());
    }
    let mut parser = BlockLocator {
        source,
        lines,
        next: 0,
        multi_line_scalars,
    };
    let indent = parser.lines[0].indent;
    parser.node(indent)
}

fn source_lines(source: &str) -> Vec<SourceLine> {
    let mut lines = Vec::new();
    let mut start = 0;
    for raw in source.split_inclusive('\n') {
        let mut end = start + raw.len();
        if raw.ends_with('\n') {
            end -= 1;
        }
        if end > start && source.as_bytes()[end - 1] == b'\r' {
            end -= 1;
        }
        let line = &source[start..end];
        let indent = line.bytes().take_while(|byte| byte.is_ascii_whitespace()).count();
        let content_start = start + indent;
        if content_start < end
            && !source[content_start..end].starts_with('#')
            && !(indent == 0 && matches!(&source[content_start..end], "---" | "..."))
        {
            lines.push(SourceLine {
                end,
                indent,
                content_start,
            });
        }
        start += raw.len();
    }
    if start < source.len() {
        let end = source.len();
        let line = &source[start..end];
        let indent = line.bytes().take_while(|byte| byte.is_ascii_whitespace()).count();
        let content_start = start + indent;
        if content_start < end
            && !source[content_start..end].starts_with('#')
            && !(indent == 0 && matches!(&source[content_start..end], "---" | "..."))
        {
            lines.push(SourceLine {
                end,
                indent,
                content_start,
            });
        }
    }
    lines
}

struct BlockLocator<'a> {
    source: &'a str,
    lines: Vec<SourceLine>,
    next: usize,
    /// Whether [`BlockLocator::inline_value`] follows block and multi-line
    /// scalars and [`BlockLocator::value`] looks past a tag; off for the
    /// closed v1 schema grammar.
    multi_line_scalars: bool,
}

impl BlockLocator<'_> {
    fn node(&mut self, indent: usize) -> Result<LocatedValue, SchemaError> {
        let line = self.lines.get(self.next).ok_or_else(projection_error)?.clone();
        if line.indent != indent {
            return Err(projection_error());
        }
        let content = &self.source[line.content_start..line.end];
        if sequence_content(content).is_some() {
            self.sequence(indent)
        } else if explicit_indicator_content(content, '?').is_some()
            || mapping_separator(content, false).is_some()
        {
            self.mapping(indent)
        } else {
            self.next += 1;
            locate_inline(self.source, line.content_start..line.end)
        }
    }

    fn mapping(&mut self, indent: usize) -> Result<LocatedValue, SchemaError> {
        let mut pairs = Vec::new();
        while let Some(line) = self.lines.get(self.next).cloned() {
            if line.indent != indent
                || sequence_content(&self.source[line.content_start..line.end]).is_some()
            {
                break;
            }
            let content = &self.source[line.content_start..line.end];
            if explicit_indicator_content(content, '?').is_some() {
                self.next += 1;
                pairs.push(self.explicit_pair(line.content_start..line.end, indent)?);
            } else if mapping_separator(content, false).is_some() {
                self.next += 1;
                pairs.push(self.pair(line.content_start..line.end, indent)?);
            } else {
                break;
            }
        }
        located_mapping(pairs)
    }

    fn sequence(&mut self, indent: usize) -> Result<LocatedValue, SchemaError> {
        let start = self
            .lines
            .get(self.next)
            .map(|line| line.content_start)
            .ok_or_else(projection_error)?;
        let mut items = Vec::new();
        while let Some(line) = self.lines.get(self.next).cloned() {
            if line.indent != indent {
                break;
            }
            let content = &self.source[line.content_start..line.end];
            let Some(relative) = sequence_content(content) else {
                break;
            };
            self.next += 1;
            let item_start = line.content_start + relative;
            if item_start < line.end {
                let item_content = &self.source[item_start..line.end];
                if explicit_indicator_content(item_content, '?').is_some()
                    || mapping_separator(item_content, false).is_some()
                {
                    // The item's keys align with its first key. The closed v1
                    // grammar keeps the conventional `- ` width.
                    let item_indent = if self.multi_line_scalars {
                        indent + relative
                    } else {
                        indent + 2
                    };
                    let first = if explicit_indicator_content(item_content, '?').is_some() {
                        self.explicit_pair(item_start..line.end, item_indent)?
                    } else {
                        self.pair(item_start..line.end, item_indent)?
                    };
                    let mut pairs = vec![first];
                    while let Some(next) = self.lines.get(self.next).cloned() {
                        if next.indent != item_indent
                            || sequence_content(&self.source[next.content_start..next.end]).is_some()
                        {
                            break;
                        }
                        let next_content = &self.source[next.content_start..next.end];
                        if explicit_indicator_content(next_content, '?').is_some() {
                            self.next += 1;
                            pairs.push(
                                self.explicit_pair(next.content_start..next.end, item_indent)?,
                            );
                        } else if mapping_separator(next_content, false).is_some() {
                            self.next += 1;
                            pairs.push(self.pair(next.content_start..next.end, item_indent)?);
                        } else {
                            break;
                        }
                    }
                    items.push(located_mapping(pairs)?);
                } else {
                    items.push(self.inline_value(item_start..line.end, indent)?);
                }
            } else {
                let child_indent = self
                    .lines
                    .get(self.next)
                    .filter(|next| next.indent > indent)
                    .map(|next| next.indent)
                    .ok_or_else(projection_error)?;
                items.push(self.node(child_indent)?);
            }
        }
        let end = items.last().map(|item| item.span.end).unwrap_or(start);
        Ok(LocatedValue {
            span: start..end,
            kind: LocatedKind::Sequence(items),
        })
    }

    fn pair(&mut self, range: Range<usize>, indent: usize) -> Result<LocatedPair, SchemaError> {
        let raw = &self.source[range.clone()];
        let colon = mapping_separator(raw, false).ok_or_else(projection_error)?;
        let key_range = trim_range(self.source, range.start..range.start + colon);
        let key = decoded_text(self.source, &key_range)?;
        let value_range = trim_range(self.source, range.start + colon + 1..range.end);
        let value = self.value(value_range, indent)?;
        let pair_span = key_range.start..value.span.end;
        Ok(LocatedPair {
            key,
            key_span: key_range,
            pair_span,
            value,
        })
    }

    fn explicit_pair(
        &mut self,
        key_line: Range<usize>,
        indent: usize,
    ) -> Result<LocatedPair, SchemaError> {
        let key_content = &self.source[key_line.clone()];
        let key_relative = explicit_indicator_content(key_content, '?')
            .ok_or_else(projection_error)?;
        let key_range = trim_range(self.source, key_line.start + key_relative..key_line.end);
        let key = decoded_text(self.source, &key_range)?;

        let value_line = self.lines.get(self.next).ok_or_else(projection_error)?.clone();
        if value_line.indent != indent {
            return Err(projection_error());
        }
        let value_content = &self.source[value_line.content_start..value_line.end];
        let value_relative = explicit_indicator_content(value_content, ':')
            .ok_or_else(projection_error)?;
        self.next += 1;
        let value_range =
            trim_range(self.source, value_line.content_start + value_relative..value_line.end);
        let value = self.value(value_range, indent)?;
        let pair_span = key_range.start..value.span.end;
        Ok(LocatedPair {
            key,
            key_span: key_range,
            pair_span,
            value,
        })
    }

    fn value(
        &mut self,
        value_range: Range<usize>,
        indent: usize,
    ) -> Result<LocatedValue, SchemaError> {
        if value_range.start < value_range.end {
            if let Some(after_properties) =
                node_properties_end(self.source, &value_range, self.multi_line_scalars)
            {
                let remainder = trim_range(self.source, after_properties..value_range.end);
                let mut value = if remainder.start < remainder.end {
                    self.inline_value(remainder, indent)?
                } else {
                    let child_indent = self
                        .lines
                        .get(self.next)
                        .filter(|line| line.indent > indent)
                        .map(|line| line.indent);
                    match child_indent {
                        Some(child_indent) => self.node(child_indent)?,
                        // A tagged empty value (`key: !!str`) has no node to descend into.
                        None if self.multi_line_scalars => {
                            locate_inline(self.source, value_range.clone())?
                        }
                        None => return Err(projection_error()),
                    }
                };
                value.span.start = value_range.start;
                Ok(value)
            } else {
                self.inline_value(value_range, indent)
            }
        } else {
            let next = self.lines.get(self.next).cloned();
            match next {
                Some(line) if line.indent > indent => self.node(line.indent),
                // `serde_yaml_ng` writes a mapping's sequence value without
                // indenting it (`key:\n- a`), and an empty value is null.
                Some(line)
                    if self.multi_line_scalars
                        && line.indent == indent
                        && sequence_content(&self.source[line.content_start..line.end])
                            .is_some() =>
                {
                    self.sequence(indent)
                }
                _ if self.multi_line_scalars => Ok(LocatedValue {
                    span: value_range.start..value_range.start,
                    kind: LocatedKind::Scalar(DecodedScalar::empty(value_range.start)),
                }),
                _ => Err(projection_error()),
            }
        }
    }
}

impl BlockLocator<'_> {
    /// Locates a value that starts inside a line. With
    /// [`multi_line_scalars`](Self::multi_line_scalars) on, a block scalar or
    /// a multi-line quoted or plain scalar is followed onto its continuation
    /// lines, which are then consumed. `parent_indent` is the column of the
    /// holding mapping key or sequence entry.
    fn inline_value(
        &mut self,
        range: Range<usize>,
        parent_indent: usize,
    ) -> Result<LocatedValue, SchemaError> {
        let raw = &self.source[range.clone()];
        let continues = self
            .lines
            .get(self.next)
            .is_some_and(|line| line.indent > parent_indent);
        let multi_line = is_block_scalar_header(raw)
            || raw.starts_with(['\'', '"'])
            || (continues && !raw.starts_with(['[', '{']));
        if !(self.multi_line_scalars && multi_line) {
            return locate_inline(self.source, range);
        }
        let (scalar, end) =
            yaml_scalar::decode_scalar_node(self.source, range.start, parent_indent)
                .ok_or_else(projection_error)?;
        while self
            .lines
            .get(self.next)
            .is_some_and(|line| line.content_start < end)
        {
            self.next += 1;
        }
        Ok(LocatedValue {
            span: range.start..end,
            kind: LocatedKind::Scalar(scalar),
        })
    }
}

fn explicit_indicator_content(source: &str, indicator: char) -> Option<usize> {
    let rest = source.strip_prefix(indicator)?;
    if rest.is_empty() {
        return Some(indicator.len_utf8());
    }
    let spaces = rest.bytes().take_while(|byte| byte.is_ascii_whitespace()).count();
    (spaces > 0).then_some(indicator.len_utf8() + spaces)
}

/// Where the node properties opening `range` end: an anchor and, with `tags`
/// set, a tag, in either order. `None` when the value has none.
fn node_properties_end(source: &str, range: &Range<usize>, tags: bool) -> Option<usize> {
    let mut end = None;
    let mut pos = range.start;
    let (mut tag, mut anchor) = (false, false);
    loop {
        let rest = &source[pos..range.end];
        let token = rest
            .bytes()
            .take_while(|byte| !byte.is_ascii_whitespace())
            .count();
        match rest.as_bytes().first() {
            Some(b'&') if !anchor && token > 1 => anchor = true,
            Some(b'!') if tags && !tag => tag = true,
            _ => return end,
        }
        end = Some(pos + token);
        pos += token;
        pos += source[pos..range.end]
            .bytes()
            .take_while(u8::is_ascii_whitespace)
            .count();
    }
}

fn located_mapping(pairs: Vec<LocatedPair>) -> Result<LocatedValue, SchemaError> {
    let start = pairs.first().map(|pair| pair.key_span.start).ok_or_else(projection_error)?;
    let end = pairs.last().map(|pair| pair.value.span.end).ok_or_else(projection_error)?;
    Ok(LocatedValue {
        span: start..end,
        kind: LocatedKind::Mapping(pairs),
    })
}

fn locate_inline(source: &str, range: Range<usize>) -> Result<LocatedValue, SchemaError> {
    let range = trim_range(source, range);
    let raw = &source[range.clone()];
    if is_block_scalar_header(raw) {
        return Err(projection_error());
    }
    if raw.starts_with('[') {
        let end = flow_collection_end(source, range.start)
            .filter(|end| *end + 1 == range.end)
            .ok_or_else(projection_error)?;
        // An empty flow collection (`[]`) splits into one empty item; it has
        // no child node, and is itself the smallest authored node there is.
        let items = split_flow_entries(source, range.start + 1..end)
            .into_iter()
            .map(|item| trim_range(source, item))
            .filter(|item| item.start < item.end)
            .map(|item| locate_inline(source, item))
            .collect::<Result<Vec<_>, _>>()?;
        return Ok(LocatedValue {
            span: range,
            kind: LocatedKind::Sequence(items),
        });
    }
    if raw.starts_with('{') {
        let end = flow_collection_end(source, range.start)
            .filter(|end| *end + 1 == range.end)
            .ok_or_else(projection_error)?;
        let mut pairs = Vec::new();
        for pair in split_flow_entries(source, range.start + 1..end) {
            let pair = trim_range(source, pair);
            if pair.start == pair.end {
                continue;
            }
            let raw_pair = &source[pair.clone()];
            let colon = mapping_separator(raw_pair, true).ok_or_else(projection_error)?;
            let key_span = trim_range(source, pair.start..pair.start + colon);
            let key = decoded_text(source, &key_span)?;
            let value = locate_inline(source, pair.start + colon + 1..pair.end)?;
            let pair_span = key_span.start..value.span.end;
            pairs.push(LocatedPair {
                key,
                key_span,
                pair_span,
                value,
            });
        }
        return Ok(LocatedValue {
            span: range,
            kind: LocatedKind::Mapping(pairs),
        });
    }
    let (scalar, consumed) = yaml_scalar::decode_scalar_at(raw, range.start)
        .ok_or_else(projection_error)?;
    Ok(LocatedValue {
        span: range.start..range.start + consumed,
        kind: LocatedKind::Scalar(scalar),
    })
}

/// Where the raw YAML flow collection opening at byte `start` of `source`
/// closes: the offset of its matching `]` or `}`.
///
/// A quote opens a quoted scalar only as the scalar's first character, so the
/// apostrophe in `[don't, "v"]` is content, and so is the one in
/// `[a:'b, "v"]`, whose `:` is plain-scalar content rather than a value
/// indicator; a `]` inside `"a]"` is not structure. A `#` after whitespace
/// starts a comment that runs to the end of its line.
///
/// ## Returns
///
/// `None` when `start` does not open a flow collection or the collection is
/// not closed, including by an unterminated quoted scalar.
pub fn flow_collection_end(source: &str, start: usize) -> Option<usize> {
    if !matches!(source.as_bytes().get(start), Some(b'[' | b'{')) {
        return None;
    }
    let mut depth = 0usize;
    let mut end = None;
    scan_flow_yaml(source, start..source.len(), |index, byte| {
        match byte {
            b'[' | b'{' => depth += 1,
            b']' | b'}' if depth > 0 => {
                depth -= 1;
                if depth == 0 {
                    end = Some(index);
                    return true;
                }
            }
            _ => {}
        }
        false
    });
    end
}

/// The top-level `,`-separated entries of a raw YAML flow collection's
/// interior, with [`flow_collection_end`]'s quote and comment rules.
///
/// Only the YAML collection delimiters `[`/`{` nest. Parentheses are plain
/// scalar content, so `[a(b, c)d, v]` has three entries; the decoded
/// expression splitter ([`split_top_level`]) is the one that nests `(`/`)`.
pub(super) fn split_flow_entries(source: &str, range: Range<usize>) -> Vec<Range<usize>> {
    let mut spans = Vec::new();
    let mut start = range.start;
    let mut depth = 0usize;
    scan_flow_yaml(source, range.clone(), |index, byte| {
        match byte {
            b'[' | b'{' => depth += 1,
            b']' | b'}' => depth = depth.saturating_sub(1),
            b',' if depth == 0 => {
                spans.push(start..index);
                start = index + 1;
            }
            _ => {}
        }
        false
    });
    spans.push(start..range.end);
    spans
}

/// Calls `visit` with each byte of `range` outside quoted scalars and
/// comments, in raw YAML flow context, until `visit` returns `true`.
///
/// `range` must start where a scalar may begin (at or just inside a
/// collection's opening bracket). Scalar boundaries follow
/// [`scan_raw_yaml`]; a content `:` inside a plain scalar (`a:'b`,
/// `http://x`) is not passed to `visit`. The decoded schema-expression
/// scanner ([`scan_expression`]) does not use these rules.
pub(super) fn scan_flow_yaml(
    source: &str,
    range: Range<usize>,
    mut visit: impl FnMut(usize, u8) -> bool,
) {
    scan_raw_yaml(source, range, true, |index, byte, _| visit(index, byte));
}

/// Calls `visit` with each byte of `range` outside quoted scalars and
/// comments, and the YAML flow-collection depth before that byte, until
/// `visit` returns `true`. `flow` says whether `range` lies inside a flow
/// collection; otherwise it is block context and starts at depth `0`.
///
/// A quote opens a quoted scalar only where a scalar begins: at the start of
/// `range`, or after `[`, `{`, `,`, or a mapping value indicator, with
/// optional whitespace between. Scanning stops at an unterminated one. A `#`
/// after whitespace starts a comment that runs to the end of its line.
///
/// A `:` is a mapping value indicator when whitespace or the end of `range`
/// follows it, and in flow context also when a flow indicator follows it or
/// it directly follows a quoted scalar or collection (`{"a":b}`). Otherwise it
/// is content of a plain scalar and is not passed to `visit`, so the quote in
/// `a:'b` stays content. This is `serde_yaml_ng`'s reading. In block context,
/// `[` and `{` open a collection only where a scalar begins, and `,`, `]`, and
/// `}` are content outside one.
fn scan_raw_yaml(
    source: &str,
    range: Range<usize>,
    flow: bool,
    mut visit: impl FnMut(usize, u8, usize) -> bool,
) {
    let bytes = source.as_bytes();
    let mut depth = usize::from(flow);
    let mut scalar_start = true;
    let mut in_plain = false;
    let mut after_blank = true;
    let mut index = range.start;
    while index < range.end {
        let byte = bytes[index];
        if scalar_start && matches!(byte, b'\'' | b'"') {
            let Some(end) = quoted_flow_scalar_end(bytes, index, range.end) else {
                return;
            };
            index = end;
            scalar_start = false;
            in_plain = false;
            after_blank = false;
            continue;
        }
        if byte == b'#' && after_blank {
            while index < range.end && !matches!(bytes[index], b'\n' | b'\r') {
                index += 1;
            }
            in_plain = false;
            continue;
        }
        let in_flow = depth > 0;
        let depth_before = depth;
        match byte {
            _ if byte.is_ascii_whitespace() => {}
            b'[' | b'{' if in_flow || !in_plain => {
                depth += 1;
                scalar_start = true;
                in_plain = false;
            }
            b']' | b'}' if in_flow => {
                depth -= 1;
                scalar_start = false;
                in_plain = false;
            }
            b',' if in_flow => {
                scalar_start = true;
                in_plain = false;
            }
            b':' => {
                let next = bytes.get(index + 1).filter(|_| index + 1 < range.end);
                let indicator = next.is_none_or(u8::is_ascii_whitespace)
                    || (in_flow
                        && (!in_plain || next.is_some_and(|next| b",[]{}".contains(next))));
                if !indicator {
                    in_plain = true;
                    scalar_start = false;
                    after_blank = false;
                    index += 1;
                    continue;
                }
                scalar_start = true;
                in_plain = false;
            }
            _ => {
                scalar_start = false;
                in_plain = true;
            }
        }
        if visit(index, byte, depth_before) {
            return;
        }
        after_blank = byte.is_ascii_whitespace();
        index += 1;
    }
}

/// Just past the closing quote of the quoted scalar opening at `start`,
/// honoring `''` in single quotes and `\` escapes in double quotes.
pub(super) fn quoted_flow_scalar_end(bytes: &[u8], start: usize, end: usize) -> Option<usize> {
    let quote = bytes[start];
    let mut index = start + 1;
    while index < end {
        match bytes[index] {
            b'\\' if quote == b'"' => index += 1,
            b'\'' if quote == b'\'' && index + 1 < end && bytes[index + 1] == b'\'' => index += 1,
            byte if byte == quote => return Some(index + 1),
            _ => {}
        }
        index += 1;
    }
    None
}

fn is_block_scalar_header(source: &str) -> bool {
    let header = source.split('#').next().unwrap_or(source).trim();
    let mut chars = header.chars();
    matches!(chars.next(), Some('|' | '>'))
        && chars.all(|character| matches!(character, '+' | '-' | '1'..='9'))
}

fn sequence_content(content: &str) -> Option<usize> {
    let rest = content.strip_prefix('-')?;
    if rest.is_empty() {
        return Some(1);
    }
    let spaces = rest.bytes().take_while(|byte| byte.is_ascii_whitespace()).count();
    (spaces > 0).then_some(1 + spaces)
}

/// The byte offset of the `:` separating a raw YAML mapping entry's key from
/// its value.
///
/// `source` is one entry: a block mapping line without its indentation, or,
/// with `flow` set, one entry of a flow mapping, where an adjacent `:` after
/// a quoted or collection key separates (`"a":b`). A quoted or collection key
/// is skipped whole, and a `:` inside a plain scalar is content, so `a:b: c`
/// has the key `a:b` and `a:'b: c` the key `a:'b`.
///
/// ## Examples
///
/// ```
/// use darkmatter::markdown::schemas::mapping_separator;
///
/// assert_eq!(mapping_separator("a:b: c", false), Some(3));
/// assert_eq!(mapping_separator("'x: y': z", false), Some(6));
/// assert_eq!(mapping_separator("http://x", false), None);
/// assert_eq!(mapping_separator("\"a\":b", true), Some(3));
/// ```
///
/// ## Returns
///
/// `None` when `source` holds no separator, including when a quoted scalar
/// is not closed.
pub fn mapping_separator(source: &str, flow: bool) -> Option<usize> {
    let top = usize::from(flow);
    let mut separator = None;
    scan_raw_yaml(source, 0..source.len(), flow, |index, byte, depth| {
        if byte == b':' && depth == top {
            separator = Some(index);
            return true;
        }
        false
    });
    separator
}

fn trim_range(source: &str, mut range: Range<usize>) -> Range<usize> {
    while range.start < range.end && source.as_bytes()[range.start].is_ascii_whitespace() {
        range.start += 1;
    }
    while range.end > range.start && source.as_bytes()[range.end - 1].is_ascii_whitespace() {
        range.end -= 1;
    }
    range
}

fn decoded_text(source: &str, range: &Range<usize>) -> Result<String, SchemaError> {
    yaml_scalar::decode_scalar_at(&source[range.clone()], range.start)
        .map(|(scalar, _)| scalar.decoded().to_string())
        .ok_or_else(projection_error)
}

fn project_schema_source(
    schema: &SimplifiedSchema,
    located: &LocatedValue,
    source_offset: usize,
    source_map: &mut SchemaSourceMap,
) -> Result<(), SchemaError> {
    match (schema, &located.kind) {
        (SimplifiedSchema::Single(shape), LocatedKind::Mapping(_)) => project_shape_source(
            shape,
            located,
            &SchemaSourcePath::root(),
            source_offset,
            source_map,
        ),
        (SimplifiedSchema::Union(arms), LocatedKind::Sequence(items))
            if arms.len() == items.len() =>
        {
            let root = SchemaSourcePath::root();
            for (index, (arm, item)) in arms.iter().zip(items).enumerate() {
                let path = root.union_arm(index);
                source_map.insert(
                    &path,
                    SchemaSpanKind::UnionArm,
                    offset_span(&item.span, source_offset),
                );
                match arm {
                    SchemaArm::Inline(shape) => project_shape_source(
                        shape,
                        item,
                        &path,
                        source_offset,
                        source_map,
                    )?,
                    SchemaArm::FileRef(_) => record_scalar_content(
                        item,
                        &path,
                        SchemaSpanKind::FileReference,
                        source_offset,
                        source_map,
                    )?,
                }
            }
            Ok(())
        }
        (_, LocatedKind::Scalar(scalar)) if scalar.decoded().starts_with('*') => Ok(()),
        _ => Err(projection_error()),
    }
}

fn project_shape_source(
    shape: &SchemaShape,
    located: &LocatedValue,
    path: &SchemaSourcePath,
    source_offset: usize,
    source_map: &mut SchemaSourceMap,
) -> Result<(), SchemaError> {
    let LocatedKind::Mapping(pairs) = &located.kind else {
        return Err(projection_error());
    };
    for pair in pairs {
        let property_path = path.property(&pair.key);
        source_map.insert(
            &property_path,
            SchemaSpanKind::MappingKey,
            offset_span(&pair.key_span, source_offset),
        );
        if pair.key == "$constraints" {
            project_mapping_constraints_source(
                &pair.value,
                &property_path,
                source_offset,
                source_map,
            )?;
            continue;
        }
        let definition = shape.properties.get(&pair.key).or_else(|| {
            super::PatternKey::parse(&pair.key).ok().and_then(|key| {
                shape
                    .pattern_keys
                    .iter()
                    .find(|pattern| pattern.key == key)
                    .map(|pattern| &pattern.def)
            })
        });
        let definition = definition.ok_or_else(projection_error)?;
        project_property_source(
            definition,
            &pair.value,
            &property_path,
            source_offset,
            source_map,
        )?;
    }
    Ok(())
}

fn project_mapping_constraints_source(
    located: &LocatedValue,
    path: &SchemaSourcePath,
    source_offset: usize,
    source_map: &mut SchemaSourceMap,
) -> Result<(), SchemaError> {
    let LocatedKind::Mapping(pairs) = &located.kind else {
        return Err(projection_error());
    };
    for pair in pairs {
        let constraint_path = path.property(&pair.key);
        source_map.insert(
            &constraint_path,
            SchemaSpanKind::MappingKey,
            offset_span(&pair.key_span, source_offset),
        );
        source_map.insert(
            &constraint_path,
            SchemaSpanKind::Definition,
            offset_span(&pair.value.span, source_offset),
        );
        source_map.insert(
            &constraint_path,
            SchemaSpanKind::Constraint,
            offset_span(&pair.pair_span, source_offset),
        );
        if matches!(pair.key.as_str(), "min-keys" | "max-keys" | "min-items" | "max-items") {
            source_map.insert(
                &constraint_path,
                SchemaSpanKind::Argument,
                offset_span(&pair.value.span, source_offset),
            );
        }
    }
    Ok(())
}

fn project_property_source(
    definition: &PropertyDef,
    located: &LocatedValue,
    path: &SchemaSourcePath,
    source_offset: usize,
    source_map: &mut SchemaSourceMap,
) -> Result<(), SchemaError> {
    source_map.insert(
        path,
        SchemaSpanKind::Definition,
        offset_span(&located.span, source_offset),
    );
    if let LocatedKind::Scalar(scalar) = &located.kind
        && scalar.decoded().starts_with('*')
    {
        source_map.insert(
            path,
            SchemaSpanKind::Atom,
            offset_span(&located.span, source_offset),
        );
        return Ok(());
    }
    match (definition, &located.kind) {
        (PropertyDef::Single(atom), _) => {
            record_atom_span(located, path, source_offset, source_map)?;
            project_atom_source(atom, located, path, source_offset, source_map)
        }
        (PropertyDef::Union(atoms), LocatedKind::Sequence(items)) if atoms.len() == items.len() => {
            for (index, (atom, item)) in atoms.iter().zip(items).enumerate() {
                let arm_path = path.union_arm(index);
                let span = offset_span(&item.span, source_offset);
                source_map.insert(&arm_path, SchemaSpanKind::UnionArm, span.clone());
                source_map.insert(&arm_path, SchemaSpanKind::Definition, span);
                record_atom_span(item, &arm_path, source_offset, source_map)?;
                project_atom_source(atom, item, &arm_path, source_offset, source_map)?;
            }
            Ok(())
        }
        _ => Err(projection_error()),
    }
}

fn record_atom_span(
    located: &LocatedValue,
    path: &SchemaSourcePath,
    source_offset: usize,
    source_map: &mut SchemaSourceMap,
) -> Result<(), SchemaError> {
    match &located.kind {
        LocatedKind::Scalar(_) => Ok(()),
        LocatedKind::Mapping(_) | LocatedKind::Sequence(_) => {
            source_map.insert(
                path,
                SchemaSpanKind::Atom,
                offset_span(&located.span, source_offset),
            );
            Ok(())
        }
    }
}

fn project_atom_source(
    atom: &PropertyAtom,
    located: &LocatedValue,
    path: &SchemaSourcePath,
    source_offset: usize,
    source_map: &mut SchemaSourceMap,
) -> Result<(), SchemaError> {
    match (&atom.ty, &located.kind) {
        (TypeExpr::InlineObject(shape), LocatedKind::Mapping(_)) =>
            project_shape_source(shape, located, path, source_offset, source_map),
        (TypeExpr::InlineObject(shape), LocatedKind::Scalar(scalar)) => {
            scan_expression_source(scalar, path, source_offset, source_map)?;
            project_inline_shape_source(shape, scalar, path, source_offset, source_map)
        }
        (_, LocatedKind::Scalar(scalar)) => {
            scan_expression_source(scalar, path, source_offset, source_map)
        }
        _ => Err(projection_error()),
    }
}

fn record_scalar_content(
    located: &LocatedValue,
    path: &SchemaSourcePath,
    kind: SchemaSpanKind,
    source_offset: usize,
    source_map: &mut SchemaSourceMap,
) -> Result<(), SchemaError> {
    let LocatedKind::Scalar(scalar) = &located.kind else {
        return Err(projection_error());
    };
    let span = scalar
        .project(0..scalar.decoded().len())
        .ok_or_else(projection_error)?;
    source_map.insert(path, kind, offset_span(&span, source_offset));
    Ok(())
}

fn scan_expression_source(
    scalar: &DecodedScalar,
    path: &SchemaSourcePath,
    source_offset: usize,
    source_map: &mut SchemaSourceMap,
) -> Result<(), SchemaError> {
    let source = scalar.decoded();
    scan_expression_range(
        scalar,
        path,
        0..source.len(),
        source_offset,
        source_map,
    )
}

fn scan_expression_range(
    scalar: &DecodedScalar,
    path: &SchemaSourcePath,
    range: Range<usize>,
    source_offset: usize,
    source_map: &mut SchemaSourceMap,
) -> Result<(), SchemaError> {
    let source = scalar.decoded();
    let arrow = find_top_level_arrow(source, range.clone());
    let expression = trim_local(source, range.start..arrow.unwrap_or(range.end));
    let atom = scalar.project(expression.clone()).ok_or_else(projection_error)?;
    source_map.insert(path, SchemaSpanKind::Atom, offset_span(&atom, source_offset));

    if source.as_bytes().get(expression.start) == Some(&b'{') {
        return scan_constraint_groups(scalar, path, expression, source_offset, source_map);
    }

    let name = leading_identifier(source, expression.clone()).ok_or_else(projection_error)?;
    if let Some(at) = find_top_level_byte(
        source,
        expression.clone(),
        ExpressionContext::Expression,
        b'@',
    ) {
        insert_projected(
            scalar,
            path,
            SchemaSpanKind::ImportName,
            name,
            source_offset,
            source_map,
        )?;
        let reference = trim_local(source, at + 1..expression.end);
        insert_projected(
            scalar,
            path,
            SchemaSpanKind::ImportReference,
            reference,
            source_offset,
            source_map,
        )?;
    } else {
        insert_projected(
            scalar,
            path,
            SchemaSpanKind::TypeKeyword,
            name,
            source_offset,
            source_map,
        )?;
    }
    scan_constraint_groups(scalar, path, expression, source_offset, source_map)
}

fn project_inline_shape_source(
    shape: &SchemaShape,
    scalar: &DecodedScalar,
    path: &SchemaSourcePath,
    source_offset: usize,
    source_map: &mut SchemaSourceMap,
) -> Result<(), SchemaError> {
    let source = scalar.decoded();
    project_inline_shape_range(
        shape,
        scalar,
        path,
        0..source.len(),
        source_offset,
        source_map,
    )
}

fn project_inline_shape_range(
    shape: &SchemaShape,
    scalar: &DecodedScalar,
    path: &SchemaSourcePath,
    range: Range<usize>,
    source_offset: usize,
    source_map: &mut SchemaSourceMap,
) -> Result<(), SchemaError> {
    let source = scalar.decoded();
    let expression_end = find_top_level_arrow(source, range.clone()).unwrap_or(range.end);
    let expression = trim_local(source, range.start..expression_end);
    let close = inline_object_end(source, expression.clone()).ok_or_else(projection_error)?;
    let body = expression.start + 1..close;
    for pair in split_top_level(source, body, ExpressionContext::ObjectBody, b',') {
        let pair = trim_local(source, pair);
        if pair.start == pair.end {
            continue;
        }
        let colon = find_top_level_byte(source, pair.clone(), ExpressionContext::ObjectBody, b':')
            .ok_or_else(projection_error)?;
        let key_span = trim_local(source, pair.start..colon);
        let key = source[key_span.clone()].to_string();
        let property_path = path.property(&key);
        insert_projected(
            scalar,
            &property_path,
            SchemaSpanKind::MappingKey,
            key_span,
            source_offset,
            source_map,
        )?;
        let definition_span = trim_local(source, colon + 1..pair.end);
        insert_projected(
            scalar,
            &property_path,
            SchemaSpanKind::Definition,
            definition_span.clone(),
            source_offset,
            source_map,
        )?;
        let definition = shape.properties.get(&key).or_else(|| {
            super::PatternKey::parse(&key).ok().and_then(|pattern_key| {
                shape
                    .pattern_keys
                    .iter()
                    .find(|pattern| pattern.key == pattern_key)
                    .map(|pattern| &pattern.def)
            })
        });
        let PropertyDef::Single(atom) = definition.ok_or_else(projection_error)? else {
            return Err(projection_error());
        };
        scan_expression_range(
            scalar,
            &property_path,
            definition_span.clone(),
            source_offset,
            source_map,
        )?;
        if let TypeExpr::InlineObject(nested) = &atom.ty {
            project_inline_shape_range(
                nested,
                scalar,
                &property_path,
                definition_span,
                source_offset,
                source_map,
            )?;
        }
    }
    Ok(())
}

/// Records the constraints and arguments of every `(…)` constraint list in
/// `range`, a type expression without its description. Lists inside an inline
/// object belong to its properties and are skipped here.
fn scan_constraint_groups(
    scalar: &DecodedScalar,
    path: &SchemaSourcePath,
    range: Range<usize>,
    source_offset: usize,
    source_map: &mut SchemaSourceMap,
) -> Result<(), SchemaError> {
    let source = scalar.decoded();
    let mut groups = Vec::new();
    let mut open = None;
    scan_expression(source, range, ExpressionContext::Expression, |index, byte, level| {
        if level.objects > 0 {
            return false;
        }
        match (byte, level.groups) {
            (b'(', 0) => open = Some(index),
            (b')', 1) => groups.extend(open.take().map(|open| open..index)),
            _ => {}
        }
        false
    });
    if open.is_some() {
        return Err(projection_error());
    }
    for group in groups {
        let list = group.start + 1..group.end;
        for constraint in split_top_level(source, list, ExpressionContext::Arguments, b';') {
            let constraint = trim_local(source, constraint);
            if constraint.start == constraint.end {
                continue;
            }
            insert_projected(
                scalar,
                path,
                SchemaSpanKind::Constraint,
                constraint.clone(),
                source_offset,
                source_map,
            )?;
            let Some(arguments) = constraint_call_arguments(source, constraint)? else {
                continue;
            };
            for argument in split_top_level(source, arguments, ExpressionContext::Arguments, b',') {
                let argument = trim_local(source, argument);
                if argument.start < argument.end {
                    insert_projected(
                        scalar,
                        path,
                        SchemaSpanKind::Argument,
                        argument,
                        source_offset,
                        source_map,
                    )?;
                }
            }
        }
    }
    Ok(())
}

/// The interior of the argument list of the constraint call in `constraint`
/// (text inside a constraint list), or `None` for a bare keyword or positional
/// members such as `enum('a(b', c)`'s, whose quoted `(` is argument text.
fn constraint_call_arguments(
    source: &str,
    constraint: Range<usize>,
) -> Result<Option<Range<usize>>, SchemaError> {
    let mut open = None;
    let mut close = None;
    scan_expression(source, constraint, ExpressionContext::Arguments, |index, byte, level| {
        match (byte, level.groups) {
            (b'(', 1) if open.is_none() => open = Some(index),
            (b')', 2) if open.is_some() => {
                close = Some(index);
                return true;
            }
            _ => {}
        }
        false
    });
    match (open, close) {
        (None, _) => Ok(None),
        (Some(open), Some(close)) => Ok(Some(open + 1..close)),
        (Some(_), None) => Err(projection_error()),
    }
}

fn insert_projected(
    scalar: &DecodedScalar,
    path: &SchemaSourcePath,
    kind: SchemaSpanKind,
    span: Range<usize>,
    source_offset: usize,
    source_map: &mut SchemaSourceMap,
) -> Result<(), SchemaError> {
    let span = scalar.project(span).ok_or_else(projection_error)?;
    source_map.insert(path, kind, offset_span(&span, source_offset));
    Ok(())
}

fn leading_identifier(source: &str, range: Range<usize>) -> Option<Range<usize>> {
    let mut end = range.start;
    while end < range.end {
        let byte = source.as_bytes()[end];
        if byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_') {
            end += 1;
        } else {
            break;
        }
    }
    (end > range.start).then_some(range.start..end)
}

/// The `->` opening the top-level description of the type expression in
/// `range`.
fn find_top_level_arrow(source: &str, range: Range<usize>) -> Option<usize> {
    let bytes = source.as_bytes();
    let mut found = None;
    scan_expression(source, range, ExpressionContext::Expression, |index, byte, level| {
        if level.is_top() && byte == b'-' && bytes.get(index + 1) == Some(&b'>') {
            found = Some(index);
            return true;
        }
        false
    });
    found
}

/// The first `needle` read as structure outside every inline object and
/// constraint list in `range`.
fn find_top_level_byte(
    source: &str,
    range: Range<usize>,
    context: ExpressionContext,
    needle: u8,
) -> Option<usize> {
    let mut found = None;
    scan_expression(source, range, context, |index, byte, level| {
        if level.is_top() && byte == needle {
            found = Some(index);
            return true;
        }
        false
    });
    found
}

/// The `}` closing the inline object that opens at `range.start`.
fn inline_object_end(source: &str, range: Range<usize>) -> Option<usize> {
    let mut found = None;
    scan_expression(source, range, ExpressionContext::Expression, |index, byte, level| {
        if byte == b'}' && level.objects == 1 && level.groups == 0 {
            found = Some(index);
            return true;
        }
        false
    });
    found
}

/// Splits `range` at each `separator` read as structure at the level where
/// `range` starts: between properties for [`ExpressionContext::ObjectBody`],
/// between constraints or arguments for [`ExpressionContext::Arguments`].
/// Raw YAML uses [`split_flow_entries`].
fn split_top_level(
    source: &str,
    range: Range<usize>,
    context: ExpressionContext,
    separator: u8,
) -> Vec<Range<usize>> {
    let mut spans = Vec::new();
    let mut start = range.start;
    scan_expression(source, range.clone(), context, |index, byte, level| {
        if byte == separator && level.is_top() {
            spans.push(start..index);
            start = index + 1;
        }
        false
    });
    spans.push(start..range.end);
    spans
}

/// Where a [`scan_expression`] range sits in the decoded type-expression
/// grammar, which decides where a description ends.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum ExpressionContext {
    /// A whole type expression. A top-level description runs to the end of
    /// the range, as the parser reads it.
    Expression,
    /// A whole type expression written as a plain arm of a raw YAML flow
    /// sequence, where YAML ends a top-level description at the next `,` or
    /// `]`.
    FlowArm,
    /// The body of an inline object, so a description ends at the object's
    /// next `,` or `}`.
    ObjectBody,
    /// The inside of a constraint or argument list.
    Arguments,
}

/// The structural nesting before a byte passed to [`scan_expression`],
/// relative to the start of the scanned range.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct ExpressionLevel {
    /// Enclosing inline objects `{ … }`.
    pub(super) objects: usize,
    /// Enclosing constraint and argument lists `( … )`, counting the list an
    /// [`ExpressionContext::Arguments`] range starts inside.
    pub(super) groups: usize,
    base_groups: usize,
}

impl ExpressionLevel {
    /// Whether the byte is at the level where the scanned range starts.
    pub(super) fn is_top(self) -> bool {
        self.objects == 0 && self.groups == self.base_groups
    }
}

/// Calls `visit` with each byte of `range` that the type-expression grammar
/// reads as structure, and the nesting before it, until `visit` returns `true`.
///
/// The grammar has one lexical mode per kind of text, and this scanner follows
/// the same mode the parser is in rather than inferring one from delimiter
/// depth:
///
/// - **Type text** (`{`, `}`, `[]`, `(`, `@`, `->`, `,`, `:`, identifiers) is
///   visited. `{` opens an inline object and `(` a constraint list.
/// - **Argument lists**, inside a constraint list's `(…)`: `(`/`)` nest and a
///   quote opens a string the lexer reads with `\` escapes, so
///   `suggest('a)b', c)` holds two arguments. `[`, `{`, and the rest are
///   argument text. String contents are not visited.
/// - **Descriptions** after `->` are prose, never visited: quotes and brackets
///   in them mean nothing, so `(it's fine)` and `plain [x` hide no boundary.
///   Inside an inline object one ends where
///   [`grammar::inline_description_end`] ends it.
/// - **Imported file references** after `@` are opaque, never visited, and
///   end where [`grammar::file_reference_end`] ends them, so
///   `Name@./a{b.yaml -> d` has a top-level `->`.
/// - **Pattern keys** `<…>` are opaque to their first `>`.
///
/// The byte that ends a description or reference is visited as type text.
/// Scanning stops at an unterminated string or pattern key, and at an
/// unbalanced `)` in a description, which the parser rejects.
pub(super) fn scan_expression(
    source: &str,
    range: Range<usize>,
    context: ExpressionContext,
    mut visit: impl FnMut(usize, u8, ExpressionLevel) -> bool,
) {
    let bytes = source.as_bytes();
    let end = range.end;
    let base_groups = usize::from(context == ExpressionContext::Arguments);
    let mut level = ExpressionLevel {
        objects: 0,
        groups: base_groups,
        base_groups,
    };
    let mut index = range.start;
    while index < end {
        let byte = bytes[index];
        if level.groups > 0 {
            if matches!(byte, b'\'' | b'"') {
                let Some(next) = grammar::quoted_end(bytes, index, end) else {
                    return;
                };
                index = next;
                continue;
            }
            if visit(index, byte, level) {
                return;
            }
            match byte {
                b'(' => level.groups += 1,
                b')' => level.groups -= 1,
                _ => {}
            }
            index += 1;
            continue;
        }
        if visit(index, byte, level) {
            return;
        }
        index = match byte {
            b'(' => {
                level.groups = 1;
                index + 1
            }
            b'{' => {
                level.objects += 1;
                index + 1
            }
            b'}' => {
                level.objects = level.objects.saturating_sub(1);
                index + 1
            }
            b'<' => match grammar::pattern_key_end(bytes, index, end) {
                Some(next) => next,
                None => return,
            },
            b'@' => grammar::file_reference_end(bytes, index + 1, end),
            b'-' if bytes.get(index + 1) == Some(&b'>') => {
                let start = index + 2;
                if level.objects > 0 || context == ExpressionContext::ObjectBody {
                    match grammar::inline_description_end(bytes, start, end) {
                        Ok(next) => next,
                        Err(_) => return,
                    }
                } else if context == ExpressionContext::FlowArm {
                    (start..end)
                        .find(|&next| matches!(bytes[next], b',' | b']'))
                        .unwrap_or(end)
                } else {
                    end
                }
            }
            _ => index + 1,
        };
    }
}

fn trim_local(source: &str, mut range: Range<usize>) -> Range<usize> {
    while range.start < range.end && source.as_bytes()[range.start].is_ascii_whitespace() {
        range.start += 1;
    }
    while range.end > range.start && source.as_bytes()[range.end - 1].is_ascii_whitespace() {
        range.end -= 1;
    }
    range
}

fn offset_span(span: &Range<usize>, offset: usize) -> Range<usize> {
    offset + span.start..offset + span.end
}

/// Parses a SimplifiedSchema value and projects suggestion spans into its YAML
/// source.
///
/// `source_offset` is added after YAML scalar projection. Pass zero for a
/// standalone YAML buffer, or the frontmatter YAML block's document offset for
/// inline Markdown. The source is never line-ending-normalized.
pub fn parse_yaml_schema_with_source(
    value: &YamlValue,
    yaml_source: &str,
    source_offset: usize,
) -> Result<SimplifiedSchema, SchemaError> {
    let mut schema = parse_yaml_schema(value)?;
    project_suggestion_spans(&mut schema, yaml_source, source_offset)?;
    Ok(schema)
}

/// Projects expression-relative suggestion spans through YAML scalar quoting
/// and escaping into byte ranges in the caller's source document.
///
/// Plain, single-quoted, and double-quoted YAML scalars are supported, including
/// CRLF input and UTF-8 text. A projection mismatch is a grammar error rather
/// than a silently approximate range.
pub fn project_suggestion_spans(
    schema: &mut SimplifiedSchema,
    yaml_source: &str,
    source_offset: usize,
) -> Result<(), SchemaError> {
    let scalars = scan_value_scalars(yaml_source);
    let mut projector = Projector {
        scalars: &scalars,
        next_scalar: 0,
        source_offset,
    };
    match schema {
        SimplifiedSchema::Single(shape) => projector.shape(shape),
        SimplifiedSchema::Union(arms) => {
            for arm in arms {
                if let SchemaArm::Inline(shape) = arm {
                    projector.shape(shape)?;
                }
            }
            Ok(())
        }
    }
}

struct Projector<'a> {
    scalars: &'a [DecodedScalar],
    next_scalar: usize,
    source_offset: usize,
}

impl Projector<'_> {
    fn shape(&mut self, shape: &mut SchemaShape) -> Result<(), SchemaError> {
        for def in shape.properties.values_mut() {
            self.property(def)?;
        }
        for pattern in &mut shape.pattern_keys {
            self.property(&mut pattern.def)?;
        }
        Ok(())
    }

    fn property(&mut self, def: &mut PropertyDef) -> Result<(), SchemaError> {
        match def {
            PropertyDef::Single(atom) => self.atom(atom),
            PropertyDef::Union(atoms) => {
                for atom in atoms {
                    self.atom(atom)?;
                }
                Ok(())
            }
        }
    }

    fn atom(&mut self, atom: &mut PropertyAtom) -> Result<(), SchemaError> {
        if atom_has_suggestions(atom)
            && let Some((scalar_index, scalar)) = self.scalars[self.next_scalar..]
                .iter()
                .enumerate()
                .find(|(_, scalar)| scalar_matches_atom(scalar, atom))
                .map(|(relative, scalar)| (self.next_scalar + relative, scalar))
        {
            self.next_scalar = scalar_index + 1;
            return project_atom_suggestions(atom, scalar, self.source_offset);
        }

        if let TypeExpr::InlineObject(shape) = &mut atom.ty {
            self.shape(shape)?;
        }
        if atom.constraints.iter().any(|constraint| matches!(constraint, Constraint::Suggest(_))) {
            return Err(projection_error());
        }
        Ok(())
    }
}

fn atom_has_suggestions(atom: &PropertyAtom) -> bool {
    atom.constraints.iter().any(|constraint| matches!(constraint, Constraint::Suggest(_)))
        || match &atom.ty {
            TypeExpr::InlineObject(shape) => shape.properties.values().any(property_has_suggestions)
                || shape.pattern_keys.iter().any(|pattern| property_has_suggestions(&pattern.def)),
            TypeExpr::Primitive(_) | TypeExpr::Imported { .. } => false,
        }
}

fn property_has_suggestions(def: &PropertyDef) -> bool {
    match def {
        PropertyDef::Single(atom) => atom_has_suggestions(atom),
        PropertyDef::Union(atoms) => atoms.iter().any(atom_has_suggestions),
    }
}

fn scalar_matches_atom(scalar: &DecodedScalar, expected: &PropertyAtom) -> bool {
    super::grammar::parse_type_expr("<source>", scalar.decoded())
        .is_ok_and(|actual| actual == *expected)
}

fn project_atom_suggestions(
    atom: &mut PropertyAtom,
    scalar: &DecodedScalar,
    source_offset: usize,
) -> Result<(), SchemaError> {
    if let TypeExpr::InlineObject(shape) = &mut atom.ty {
        for def in shape.properties.values_mut() {
            project_property_suggestions(def, scalar, source_offset)?;
        }
        for pattern in &mut shape.pattern_keys {
            project_property_suggestions(&mut pattern.def, scalar, source_offset)?;
        }
    }
    if let Some(candidates) = atom.constraints.iter_mut().find_map(|constraint| {
        if let Constraint::Suggest(candidates) = constraint {
            Some(candidates)
        } else {
            None
        }
    }) {
        for candidate in candidates {
            let start = scalar.raw_offset(candidate.span.start).ok_or_else(projection_error)?;
            let end = scalar.raw_offset(candidate.span.end).ok_or_else(projection_error)?;
            candidate.span = source_offset + start..source_offset + end;
        }
    }
    Ok(())
}

fn project_property_suggestions(
    def: &mut PropertyDef,
    scalar: &DecodedScalar,
    source_offset: usize,
) -> Result<(), SchemaError> {
    match def {
        PropertyDef::Single(atom) => project_atom_suggestions(atom, scalar, source_offset),
        PropertyDef::Union(atoms) => {
            for atom in atoms {
                project_atom_suggestions(atom, scalar, source_offset)?;
            }
            Ok(())
        }
    }
}

fn projection_error() -> SchemaError {
    SchemaError::Grammar {
        property: "<source>".into(),
        message: "could not project SimplifiedSchema expression spans through YAML source".into(),
        span: 0..0,
    }
}

fn scan_value_scalars(source: &str) -> Vec<DecodedScalar> {
    let mut scalars = Vec::new();
    let mut line_start = 0;
    for line_with_ending in source.split_inclusive('\n') {
        let line = line_with_ending
            .strip_suffix('\n')
            .unwrap_or(line_with_ending)
            .strip_suffix('\r')
            .unwrap_or_else(|| line_with_ending.strip_suffix('\n').unwrap_or(line_with_ending));
        let trimmed = line.trim_start();
        let indent = line.len() - trimmed.len();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            line_start += line_with_ending.len();
            continue;
        }
        let value_at = if let Some(rest) = trimmed.strip_prefix('-') {
            if rest.starts_with(char::is_whitespace) {
                let leading = rest.len() - rest.trim_start().len();
                let item = rest.trim_start();
                mapping_value_offset(item)
                    .map(|offset| {
                        let value = &item[offset..];
                        indent + 1 + leading + offset + (value.len() - value.trim_start().len())
                    })
                    .or(Some(indent + 1 + leading))
            } else {
                None
            }
        } else {
            mapping_value_offset(line).map(|offset| {
                let rest = &line[offset..];
                offset + (rest.len() - rest.trim_start().len())
            })
        };
        if let Some(value_at) = value_at.filter(|offset| *offset < line.len()) {
            scan_value(&line[value_at..], line_start + value_at, &mut scalars);
        }
        line_start += line_with_ending.len();
    }
    // `split_inclusive` produces no item for an empty trailing remainder, and
    // handles a final line without `\n` in the ordinary case above.
    scalars
}

/// Just past the key/value separator of a raw block YAML line, as found by
/// [`mapping_separator`].
fn mapping_value_offset(line: &str) -> Option<usize> {
    mapping_separator(line, false).map(|colon| colon + 1)
}

fn scan_value(raw: &str, base: usize, out: &mut Vec<DecodedScalar>) {
    if raw.starts_with('[') {
        let mut offset = 1;
        while offset < raw.len() {
            offset += raw[offset..].len() - raw[offset..].trim_start().len();
            if raw[offset..].starts_with(']') {
                break;
            }
            if let Some((scalar, consumed)) = yaml_scalar::decode_scalar_at(&raw[offset..], base + offset) {
                out.push(scalar);
                offset += consumed;
            } else {
                break;
            }
            if let Some(comma) = raw[offset..].find(',') {
                offset += comma + 1;
            } else {
                break;
            }
        }
    } else if let Some((scalar, _)) = yaml_scalar::decode_scalar_at(raw, base) {
        out.push(scalar);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn candidate_span(yaml: &str, offset: usize) -> std::ops::Range<usize> {
        let value: YamlValue = serde_yaml_ng::from_str(yaml).unwrap();
        let mut schema = parse_yaml_schema_with_source(&value, yaml, offset).unwrap();
        let SimplifiedSchema::Single(shape) = &mut schema else {
            panic!("expected single shape");
        };
        let PropertyDef::Single(atom) = &shape.properties["value"] else {
            panic!("expected single atom");
        };
        let Constraint::Suggest(candidates) = atom
            .constraints
            .iter()
            .find(|constraint| matches!(constraint, Constraint::Suggest(_)))
            .unwrap()
        else {
            unreachable!()
        };
        candidates[1].span.clone()
    }

    #[test]
    fn mapping_separator_treats_a_quote_as_quoting_only_where_a_scalar_begins() {
        for (line, expected) in [
            ("it's: x", Some(4)),
            ("say \"hi: x", Some(7)),
            ("a 'b: c'", Some(4)),
            ("'a:b': c", Some(5)),
            ("'it''s: x': y", Some(10)),
            ("\"a\\\": b\": c", Some(8)),
            ("[a, 'b: c']: x", Some(11)),
            ("{k: 'v: w'}: x", Some(11)),
            ("'unterminated: x", None),
            ("plain scalar", None),
        ] {
            assert_eq!(mapping_separator(line, false), expected, "{line:?}");
        }
    }

    /// A `:` separates a key only as a YAML value indicator; inside a plain
    /// scalar it is content, and the quote after it stays content too.
    #[test]
    fn mapping_separator_skips_content_colons_inside_plain_scalars() {
        for (line, flow, expected) in [
            // Block context: only `: ` or a line-ending `:` separates.
            ("a:b: c", false, Some(3)),
            ("a:'b: c", false, Some(4)),
            ("a:\"b: c", false, Some(4)),
            ("url: http://x:8080", false, Some(3)),
            ("key:", false, Some(3)),
            ("key:value", false, None),
            ("'a':b", false, None),
            ("a #b: c", false, None),
            // Flow context adds a `:` right after a quoted or collection key.
            ("a:'b: \"v\"", true, Some(4)),
            ("a:\"b: 'v'", true, Some(4)),
            ("\"a\":b", true, Some(3)),
            ("'a' :b", true, Some(4)),
            ("[x]:b", true, Some(3)),
            ("a :b", true, None),
            ("http://x", true, None),
        ] {
            assert_eq!(mapping_separator(line, flow), expected, "{line:?} flow={flow}");
        }
    }

    /// Renders a located node as its authored scalars, `[a|b]` sequences, and
    /// `{key=value|…}` mappings, so one string pins structure and spans.
    fn located_shape(source: &str, node: &SchemaValueNode) -> String {
        let join = |parts: Vec<String>| parts.join("|");
        match &node.kind {
            SchemaValueKind::Scalar => source[node.span.clone()].to_string(),
            SchemaValueKind::Sequence(items) => format!(
                "[{}]",
                join(items.iter().map(|item| located_shape(source, item)).collect())
            ),
            SchemaValueKind::Mapping(entries) => format!(
                "{{{}}}",
                join(
                    entries
                        .iter()
                        .map(|entry| format!("{}={}", entry.key, located_shape(source, &entry.value)))
                        .collect()
                )
            ),
        }
    }

    #[test]
    fn raw_flow_collections_open_a_quote_only_where_a_scalar_begins() {
        for (yaml, expected) in [
            // Controls: ordinary plain siblings, genuinely quoted spellings,
            // and delimiters inside real quoted scalars.
            ("[ordinary, \"v\"]", "[ordinary|\"v\"]"),
            ("['don''t', \"v\"]", "['don''t'|\"v\"]"),
            ("{'don''t': \"v\"}", "{don't=\"v\"}"),
            ("[\"a, b\", 'c'']d', \"e\\\"]f\"]", "[\"a, b\"|'c'']d'|\"e\\\"]f\"]"),
            // A quote inside a plain scalar is content.
            ("[don't, \"v\"]", "[don't|\"v\"]"),
            ("[say \"hi, 'v']", "[say \"hi|'v']"),
            ("[don't, can't, \"v\"]", "[don't|can't|\"v\"]"),
            ("{don't: \"v\"}", "{don't=\"v\"}"),
            ("{say \"hi: 'v'}", "{say \"hi='v'}"),
            ("{a: don't, b: \"v\"}", "{a=don't|b=\"v\"}"),
            ("{a: say \"hi, b: 'v'}", "{a=say \"hi|b='v'}"),
            // Nested collections take the same rule.
            ("{list: [don't, \"v\"], z: 1}", "{list=[don't|\"v\"]|z=1}"),
            ("[{k: don't}, {k: 'v'}]", "[{k=don't}|{k='v'}]"),
            ("[don't, {can't: \"v\"}]", "[don't|{can't=\"v\"}]"),
            ("key: [don't, can't, \"v\"]", "{key=[don't|can't|\"v\"]}"),
            ("key: {a: say \"hi, b: 'v'}", "{key={a=say \"hi|b='v'}}"),
        ] {
            let node = locate_schema_value(yaml, 0)
                .unwrap_or_else(|| panic!("{yaml:?} was not located"));
            assert_eq!(located_shape(yaml, &node), expected, "{yaml:?}");
        }
    }

    /// Every row is first read by `serde_yaml_ng`, whose structure the
    /// located shape must match, under LF and CRLF.
    #[test]
    fn a_content_colon_does_not_reopen_quote_mode_in_raw_flow_yaml() {
        for (flow, expected) in [
            // Controls: ordinary and `don't` siblings, genuinely quoted
            // colons, and adjacent JSON-like keys, which do separate.
            ("[ordinary, \"v\"]", "[ordinary|\"v\"]"),
            ("[don't, \"v\"]", "[don't|\"v\"]"),
            ("['a:''b', \"v\"]", "['a:''b'|\"v\"]"),
            ("[\"a:b\", 'v']", "[\"a:b\"|'v']"),
            ("{\"a\":b, c: 'v'}", "{a=b|c='v'}"),
            ("{'a':'v'}", "{a='v'}"),
            ("[http://x, 'v']", "[http://x|'v']"),
            ("{a:b: c, d: 'v'}", "{a:b=c|d='v'}"),
            // A quote after a content colon is content.
            ("[a:'b, \"v\"]", "[a:'b|\"v\"]"),
            ("[a:\"b, 'v']", "[a:\"b|'v']"),
            ("[a:'b, c:'d, \"v\"]", "[a:'b|c:'d|\"v\"]"),
            ("[a:\"b, c:\"d, 'v']", "[a:\"b|c:\"d|'v']"),
            ("{a:'b: \"v\"}", "{a:'b=\"v\"}"),
            ("{a:\"b: 'v'}", "{a:\"b='v'}"),
            ("{a: a:'b, b: \"v\"}", "{a=a:'b|b=\"v\"}"),
            ("{a: a:\"b, b: 'v'}", "{a=a:\"b|b='v'}"),
            // Nested collections take the same rule.
            ("{list: [a:'b, \"v\"]}", "{list=[a:'b|\"v\"]}"),
            ("{list: [a:\"b, c:\"d, 'v'], z: 1}", "{list=[a:\"b|c:\"d|'v']|z=1}"),
            ("[{k: a:'b}, {k: 'v'}]", "[{k=a:'b}|{k='v'}]"),
            ("[a:'b, {c:'d: \"v\"}]", "[a:'b|{c:'d=\"v\"}]"),
        ] {
            let parsed: YamlValue = serde_yaml_ng::from_str(flow)
                .unwrap_or_else(|error| panic!("{flow:?} is not YAML: {error}"));
            for newline in ["\n", "\r\n"] {
                let yaml = format!("a:'b: {flow}{newline}q: {flow}{newline}");
                let node = locate_schema_value(&yaml, 0)
                    .unwrap_or_else(|| panic!("{yaml:?} was not located"));
                assert_eq!(
                    located_shape(&yaml, &node),
                    format!("{{a:'b={expected}|q={expected}}}"),
                    "{yaml:?}"
                );
                let SchemaValueKind::Mapping(entries) = &node.kind else {
                    unreachable!()
                };
                assert_eq!(entry_count(&entries[1].value), entry_count_of(&parsed), "{yaml:?}");
            }
            let end = flow_collection_end(flow, 0);
            assert_eq!(end, Some(flow.len() - 1), "{flow:?}");
        }
    }

    /// Parentheses are plain-scalar content in raw YAML: they neither hide a
    /// `,` between entries nor, when they balance across two entries, merge
    /// those entries.
    #[test]
    fn parentheses_are_content_in_raw_flow_yaml() {
        for (flow, expected) in [
            // Controls: no parenthesis, and parentheses inside quotes.
            ("[ordinary, \"v\"]", "[ordinary|\"v\"]"),
            ("['a(b', \"v\"]", "['a(b'|\"v\"]"),
            ("[\"a(b, c)d\", 'v']", "[\"a(b, c)d\"|'v']"),
            ("[a(b)c, \"v\"]", "[a(b)c|\"v\"]"),
            // Unbalanced and cross-entry balanced parentheses.
            ("[a(b, \"v\"]", "[a(b|\"v\"]"),
            ("[a)b, \"v\"]", "[a)b|\"v\"]"),
            ("[a(b, c)d, \"v\"]", "[a(b|c)d|\"v\"]"),
            ("{a: a(b, b: \"v\"}", "{a=a(b|b=\"v\"}"),
            ("{a(b: x, c)d: \"v\"}", "{a(b=x|c)d=\"v\"}"),
            // Nested collections take the same rule.
            ("{list: [a(b, \"v\"]}", "{list=[a(b|\"v\"]}"),
            ("[{k: a(b}, {k: c)d}, 'v']", "[{k=a(b}|{k=c)d}|'v']"),
            ("[[a(b, c)d], \"v\"]", "[[a(b|c)d]|\"v\"]"),
        ] {
            let parsed: YamlValue = serde_yaml_ng::from_str(flow)
                .unwrap_or_else(|error| panic!("{flow:?} is not YAML: {error}"));
            for newline in ["\n", "\r\n"] {
                let yaml = format!("p(x: {flow}{newline}q: {flow}{newline}");
                let node = locate_schema_value(&yaml, 0)
                    .unwrap_or_else(|| panic!("{yaml:?} was not located"));
                assert_eq!(
                    located_shape(&yaml, &node),
                    format!("{{p(x={expected}|q={expected}}}"),
                    "{yaml:?}"
                );
                let SchemaValueKind::Mapping(entries) = &node.kind else {
                    unreachable!()
                };
                assert_eq!(entry_count(&entries[1].value), entry_count_of(&parsed), "{yaml:?}");
            }
            assert_eq!(flow_collection_end(flow, 0), Some(flow.len() - 1), "{flow:?}");
        }
    }

    fn entry_count(node: &SchemaValueNode) -> usize {
        match &node.kind {
            SchemaValueKind::Scalar => 0,
            SchemaValueKind::Sequence(items) => items.len(),
            SchemaValueKind::Mapping(entries) => entries.len(),
        }
    }

    fn entry_count_of(value: &YamlValue) -> usize {
        match value {
            YamlValue::Sequence(items) => items.len(),
            YamlValue::Mapping(entries) => entries.len(),
            _ => 0,
        }
    }

    fn suggestion_candidates(yaml: &str, property: &str) -> Vec<String> {
        let value: YamlValue = serde_yaml_ng::from_str(yaml).unwrap();
        let schema = parse_yaml_schema_with_source(&value, yaml, 0)
            .unwrap_or_else(|error| panic!("{yaml:?}: {error}"));
        let SimplifiedSchema::Single(shape) = schema else {
            panic!("expected single shape");
        };
        let mut atom = match &shape.properties[property] {
            PropertyDef::Single(atom) => atom,
            PropertyDef::Union(_) => panic!("expected single atom"),
        };
        while let TypeExpr::InlineObject(nested) = &atom.ty {
            let PropertyDef::Single(inner) = nested.properties.values().next().unwrap() else {
                panic!("expected single nested atom");
            };
            atom = inner;
        }
        atom.constraints
            .iter()
            .find_map(|constraint| match constraint {
                Constraint::Suggest(candidates) => Some(candidates),
                _ => None,
            })
            .unwrap()
            .iter()
            .map(|candidate| yaml[candidate.span.clone()].to_string())
            .collect()
    }

    #[test]
    fn a_schema_key_holding_a_quote_inside_a_plain_scalar_projects_its_suggestions() {
        for yaml in ["it's: string(suggest(alpha, beta))\n", "say \"hi: string(suggest(alpha, beta))\n"] {
            let key = yaml.split(": ").next().unwrap();
            assert_eq!(suggestion_candidates(yaml, key), ["alpha", "beta"], "{yaml:?}");
        }
        let quoted = "'it''s': string(suggest(alpha, beta))\n";
        assert_eq!(suggestion_candidates(quoted, "it's"), ["alpha", "beta"]);
    }

    #[test]
    fn a_schema_key_holding_a_content_colon_projects_its_suggestions() {
        for (key, newline) in [
            ("a:'b", "\n"),
            ("a:\"b", "\n"),
            ("a:b", "\n"),
            ("a:'b", "\r\n"),
            ("a:\"b", "\r\n"),
        ] {
            let yaml = format!("{key}: string(suggest(alpha, beta)){newline}other: string{newline}");
            assert_eq!(suggestion_candidates(&yaml, key), ["alpha", "beta"], "{yaml:?}");
        }
    }

    /// The decoded expression grammar is not YAML: its quotes open anywhere,
    /// so a quoted argument after `(` keeps its `,` and `}`.
    #[test]
    fn expression_quotes_open_anywhere_in_the_decoded_grammar() {
        assert_eq!(
            suggestion_candidates("value: string(suggest('a,b', c))\n", "value"),
            ["'a,b'", "c"]
        );
        assert_eq!(
            suggestion_candidates("value: \"{ mode: string(suggest('x}y', z)) }\"\n", "value"),
            ["'x}y'", "z"]
        );
    }

    /// The decoded helpers follow the expression lexer's quoting: `\` escapes
    /// in either quote style, a quote opens a string only inside an argument
    /// list, and braces inside an argument list are argument text.
    #[test]
    fn expression_projection_follows_the_lexer_quote_rule() {
        let root = SchemaSourcePath::root();
        let rows: &[(&str, &str, &[&str], &[&str])] = &[
            // Controls.
            ("string(suggest('a,b', c); min(1))", "", &["suggest('a,b', c)", "min(1)"], &["'a,b'", "c", "1"]),
            ("string(suggest(\"a\\\")b\", c))", "", &["suggest(\"a\\\")b\", c)"], &["\"a\\\")b\"", "c"]),
            // A backslash escapes inside single quotes too.
            ("string(suggest('a\\')b', c); min(1))", "", &["suggest('a\\')b', c)", "min(1)"], &["'a\\')b'", "c", "1"]),
            // A quote outside parentheses is description prose.
            ("\"{ a: string -> it's, b: string(suggest(x, y)) }\"", "b", &["suggest(x, y)"], &["x", "y"]),
            ("\"{ a: string -> say \\\"hi, b: string(suggest(x, y)) }\"", "b", &["suggest(x, y)"], &["x", "y"]),
            ("'{ a: string(suggest(''p,q'', r)) -> it''s here, b: number(min(2)) }'", "b", &["min(2)"], &["2"]),
            // A brace inside an argument is argument text.
            ("\"{ a: string(pattern(^[}]$)), b: number(min(2)) }\"", "b", &["min(2)"], &["2"]),
        ];
        for (yaml, property, constraints, arguments) in rows {
            let value: YamlValue = serde_yaml_ng::from_str(yaml).unwrap();
            let projected = parse_property_definition_with_source("p", &value, yaml, 0)
                .unwrap_or_else(|error| panic!("{yaml:?}: {error:?}"));
            let path = if property.is_empty() { root.clone() } else { root.property(*property) };
            let texts = |kind| {
                projected
                    .source_map
                    .spans(&path, kind)
                    .iter()
                    .map(|span| &yaml[span.clone()])
                    .collect::<Vec<_>>()
            };
            assert_eq!(texts(SchemaSpanKind::Constraint), *constraints, "{yaml:?}");
            assert_eq!(texts(SchemaSpanKind::Argument), *arguments, "{yaml:?}");
        }
    }

    /// The suggestion projector reads semantic candidate spans and YAML
    /// decoding rather than expression structure, so description and filename
    /// punctuation beside a suggestion leaves its spans exact.
    #[test]
    fn suggestion_spans_ignore_description_and_file_reference_punctuation() {
        for yaml in [
            "value: \"{ a: string(suggest(p, q)) -> (it's fine), b: Name@./a(b.yaml }\"\n",
            "value: '{ a: string(suggest(p, q)) -> (say \"hi), b: string -> plain [x }'\n",
            "value: \"{ a: string(suggest(p, q)) -> ({x} it's), b: Name@./a{b.yaml -> d }\"\r\n",
        ] {
            assert_eq!(suggestion_candidates(yaml, "value"), ["p", "q"], "{yaml:?}");
        }
    }

    #[test]
    fn projects_plain_single_and_double_quoted_scalars() {
        for yaml in [
            "value: string(suggest(alpha, 'café'))\n",
            "value: 'string(suggest(alpha, ''café''))'\n",
            "value: \"string(suggest(alpha, 'caf\\u00e9'))\"\n",
        ] {
            let span = candidate_span(yaml, 0);
            assert!(yaml[span].contains("caf"), "{yaml:?}");
        }
    }

    #[test]
    fn projection_preserves_crlf_and_document_offset() {
        let yaml = "café: string\r\nvalue: string(suggest(alpha, beta))\r\n";
        let span = candidate_span(yaml, 100);
        assert_eq!(&yaml[span.start - 100..span.end - 100], "beta");
    }

    #[test]
    fn projects_nested_inline_object_candidates_through_containing_scalar() {
        let yaml = "value: \"{ mode: string(min(5); suggest(no, valid)) }\"\n";
        let value: YamlValue = serde_yaml_ng::from_str(yaml).unwrap();
        let schema = parse_yaml_schema_with_source(&value, yaml, 0).unwrap();
        let SimplifiedSchema::Single(shape) = schema else {
            panic!("expected single shape");
        };
        let PropertyDef::Single(value) = &shape.properties["value"] else {
            panic!("expected single value atom");
        };
        let TypeExpr::InlineObject(nested) = &value.ty else {
            panic!("expected inline object");
        };
        let PropertyDef::Single(mode) = &nested.properties["mode"] else {
            panic!("expected single mode atom");
        };
        let Constraint::Suggest(candidates) = mode
            .constraints
            .iter()
            .find(|constraint| matches!(constraint, Constraint::Suggest(_)))
            .unwrap()
        else {
            unreachable!()
        };
        let expected = yaml.find("no").unwrap();
        assert_eq!(candidates[0].span, expected..expected + 2);
    }
}
