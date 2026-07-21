//! Source-aware projection for passive SimplifiedSchema parsers.

use std::{collections::BTreeMap, ops::Range};

use serde_yaml_ng::Value as YamlValue;

use crate::markdown::schemas::errors::SchemaError;

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
    let lines = source_lines(source);
    if lines.is_empty() {
        return Err(projection_error());
    }
    let mut parser = BlockLocator {
        source,
        lines,
        next: 0,
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
            || mapping_separator(content).is_some()
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
            } else if mapping_separator(content).is_some() {
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
                if mapping_separator(&self.source[item_start..line.end]).is_some() {
                    let first = self.pair(item_start..line.end, indent + 2)?;
                    let mut pairs = vec![first];
                    while let Some(next) = self.lines.get(self.next).cloned() {
                        if next.indent != indent + 2
                            || sequence_content(&self.source[next.content_start..next.end]).is_some()
                        {
                            break;
                        }
                        self.next += 1;
                        pairs.push(self.pair(next.content_start..next.end, indent + 2)?);
                    }
                    items.push(located_mapping(pairs)?);
                } else {
                    items.push(locate_inline(self.source, item_start..line.end)?);
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
        let colon = mapping_separator(raw).ok_or_else(projection_error)?;
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
            if let Some(after_anchor) = anchor_value_start(self.source, &value_range) {
                let remainder = trim_range(self.source, after_anchor..value_range.end);
                let mut value = if remainder.start < remainder.end {
                    locate_inline(self.source, remainder)?
                } else {
                    let child_indent = self
                        .lines
                        .get(self.next)
                        .filter(|line| line.indent > indent)
                        .map(|line| line.indent)
                        .ok_or_else(projection_error)?;
                    self.node(child_indent)?
                };
                value.span.start = value_range.start;
                Ok(value)
            } else {
                locate_inline(self.source, value_range)
            }
        } else {
            let child_indent = self
                .lines
                .get(self.next)
                .filter(|line| line.indent > indent)
                .map(|line| line.indent)
                .ok_or_else(projection_error)?;
            self.node(child_indent)
        }
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

fn anchor_value_start(source: &str, range: &Range<usize>) -> Option<usize> {
    let raw = &source[range.clone()];
    let rest = raw.strip_prefix('&')?;
    let name_len = rest
        .bytes()
        .take_while(|byte| !byte.is_ascii_whitespace())
        .count();
    (name_len > 0).then_some(range.start + 1 + name_len)
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
    if raw.starts_with('[') {
        let end = matching_delimiter(source, range.start, b'[', b']')
            .filter(|end| *end + 1 == range.end)
            .ok_or_else(projection_error)?;
        // An empty flow collection (`[]`) splits into one empty item; it has
        // no child node, and is itself the smallest authored node there is.
        let items = split_top_level(source, range.start + 1..end, b',')
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
        let end = matching_delimiter(source, range.start, b'{', b'}')
            .filter(|end| *end + 1 == range.end)
            .ok_or_else(projection_error)?;
        let mut pairs = Vec::new();
        for pair in split_top_level(source, range.start + 1..end, b',') {
            let pair = trim_range(source, pair);
            if pair.start == pair.end {
                continue;
            }
            let raw_pair = &source[pair.clone()];
            let colon = mapping_separator(raw_pair).ok_or_else(projection_error)?;
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

fn sequence_content(content: &str) -> Option<usize> {
    let rest = content.strip_prefix('-')?;
    if rest.is_empty() {
        return Some(1);
    }
    let spaces = rest.bytes().take_while(|byte| byte.is_ascii_whitespace()).count();
    (spaces > 0).then_some(1 + spaces)
}

fn mapping_separator(source: &str) -> Option<usize> {
    let mut quote = None;
    let mut escaped = false;
    let mut flow_depth = 0usize;
    for (index, byte) in source.bytes().enumerate() {
        if escaped {
            escaped = false;
            continue;
        }
        match (quote, byte) {
            (Some(b'\"'), b'\\') => escaped = true,
            (Some(active), current) if active == current => quote = None,
            (None, b'\'' | b'\"') => quote = Some(byte),
            (None, b'[' | b'{') => flow_depth += 1,
            (None, b']' | b'}') => flow_depth = flow_depth.saturating_sub(1),
            (None, b':') if flow_depth == 0 => return Some(index),
            _ => {}
        }
    }
    None
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
    let arrow = find_top_level_arrow(&source[range.clone()]).map(|index| range.start + index);
    let expression = trim_local(source, range.start..arrow.unwrap_or(range.end));
    let atom = scalar.project(expression.clone()).ok_or_else(projection_error)?;
    source_map.insert(path, SchemaSpanKind::Atom, offset_span(&atom, source_offset));

    if source.as_bytes().get(expression.start) == Some(&b'{') {
        return scan_constraint_groups(scalar, path, expression, source_offset, source_map);
    }

    let name = leading_identifier(source, expression.clone()).ok_or_else(projection_error)?;
    if let Some(at) = find_top_level_byte(source, expression.clone(), b'@') {
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
    let expression_end = find_top_level_arrow(&source[range.clone()])
        .map(|index| range.start + index)
        .unwrap_or(range.end);
    let expression = trim_local(source, range.start..expression_end);
    let close = matching_delimiter(source, expression.start, b'{', b'}')
        .ok_or_else(projection_error)?;
    let body = expression.start + 1..close;
    for pair in split_top_level(source, body, b',') {
        let pair = trim_local(source, pair);
        if pair.start == pair.end {
            continue;
        }
        let colon = find_top_level_byte(source, pair.clone(), b':').ok_or_else(projection_error)?;
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

fn scan_constraint_groups(
    scalar: &DecodedScalar,
    path: &SchemaSourcePath,
    range: Range<usize>,
    source_offset: usize,
    source_map: &mut SchemaSourceMap,
) -> Result<(), SchemaError> {
    let source = scalar.decoded();
    let bytes = source.as_bytes();
    let mut quote = None;
    let mut escaped = false;
    let mut brace_depth = 0usize;
    let mut index = range.start;
    while index < range.end {
        let byte = bytes[index];
        if escaped {
            escaped = false;
            index += 1;
            continue;
        }
        match (quote, byte) {
            (Some(b'\"'), b'\\') => escaped = true,
            (Some(active), current) if active == current => quote = None,
            (None, b'\'' | b'\"') => quote = Some(byte),
            (None, b'{') => brace_depth += 1,
            (None, b'}') => brace_depth = brace_depth.saturating_sub(1),
            (None, b'(') if brace_depth == 0 => {
                let end = matching_delimiter(source, index, b'(', b')')
                    .ok_or_else(projection_error)?;
                for constraint in split_top_level(source, index + 1..end, b';') {
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
                    if let Some(open) = source[constraint.clone()]
                        .find('(')
                        .map(|relative| constraint.start + relative)
                    {
                        let close = matching_delimiter(source, open, b'(', b')')
                            .ok_or_else(projection_error)?;
                        for argument in split_top_level(source, open + 1..close, b',') {
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
                index = end;
            }
            _ => {}
        }
        index += 1;
    }
    Ok(())
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

fn find_top_level_arrow(source: &str) -> Option<usize> {
    let bytes = source.as_bytes();
    let mut quote = None;
    let mut escaped = false;
    let mut paren = 0usize;
    let mut brace = 0usize;
    for index in 0..bytes.len().saturating_sub(1) {
        let byte = bytes[index];
        if escaped {
            escaped = false;
            continue;
        }
        match (quote, byte) {
            (Some(b'\"'), b'\\') => escaped = true,
            (Some(active), current) if active == current => quote = None,
            (None, b'\'' | b'\"') => quote = Some(byte),
            (None, b'(') => paren += 1,
            (None, b')') => paren = paren.saturating_sub(1),
            (None, b'{') => brace += 1,
            (None, b'}') => brace = brace.saturating_sub(1),
            (None, b'-')
                if paren == 0 && brace == 0 && bytes.get(index + 1) == Some(&b'>') =>
            {
                return Some(index);
            }
            _ => {}
        }
    }
    None
}

fn find_top_level_byte(source: &str, range: Range<usize>, needle: u8) -> Option<usize> {
    let bytes = source.as_bytes();
    let mut quote = None;
    let mut escaped = false;
    let mut paren = 0usize;
    let mut brace = 0usize;
    for index in range {
        let byte = bytes[index];
        if escaped {
            escaped = false;
            continue;
        }
        match (quote, byte) {
            (Some(b'\"'), b'\\') => escaped = true,
            (Some(active), current) if active == current => quote = None,
            (None, b'\'' | b'\"') => quote = Some(byte),
            (None, b'(') => paren += 1,
            (None, b')') => paren = paren.saturating_sub(1),
            (None, b'{') => brace += 1,
            (None, b'}') => brace = brace.saturating_sub(1),
            (None, current) if current == needle && paren == 0 && brace == 0 => {
                return Some(index);
            }
            _ => {}
        }
    }
    None
}

fn matching_delimiter(source: &str, start: usize, open: u8, close: u8) -> Option<usize> {
    let bytes = source.as_bytes();
    let mut quote = None;
    let mut escaped = false;
    let mut depth = 0usize;
    for (index, byte) in bytes.iter().copied().enumerate().skip(start) {
        if escaped {
            escaped = false;
            continue;
        }
        match (quote, byte) {
            (Some(b'\"'), b'\\') => escaped = true,
            (Some(active), current) if active == current => quote = None,
            (None, b'\'' | b'\"') => quote = Some(byte),
            (None, current) if current == open => depth += 1,
            (None, current) if current == close => {
                depth -= 1;
                if depth == 0 {
                    return Some(index);
                }
            }
            _ => {}
        }
    }
    None
}

fn split_top_level(source: &str, range: Range<usize>, separator: u8) -> Vec<Range<usize>> {
    let bytes = source.as_bytes();
    let mut spans = Vec::new();
    let mut start = range.start;
    let mut quote = None;
    let mut escaped = false;
    let mut depth = 0usize;
    for index in range.clone() {
        let byte = bytes[index];
        if escaped {
            escaped = false;
            continue;
        }
        match (quote, byte) {
            (Some(b'\"'), b'\\') => escaped = true,
            (Some(active), current) if active == current => quote = None,
            (None, b'\'' | b'\"') => quote = Some(byte),
            (None, b'(' | b'[' | b'{') => depth += 1,
            (None, b')' | b']' | b'}') => depth = depth.saturating_sub(1),
            (None, current) if current == separator && depth == 0 => {
                spans.push(start..index);
                start = index + 1;
            }
            _ => {}
        }
    }
    spans.push(start..range.end);
    spans
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

fn mapping_value_offset(line: &str) -> Option<usize> {
    let mut quote = None;
    let mut escaped = false;
    for (index, ch) in line.char_indices() {
        if escaped {
            escaped = false;
            continue;
        }
        match (quote, ch) {
            (Some('"'), '\\') => escaped = true,
            (Some(active), current) if active == current => quote = None,
            (None, '\'' | '"') => quote = Some(ch),
            (None, ':') => return Some(index + 1),
            _ => {}
        }
    }
    None
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
