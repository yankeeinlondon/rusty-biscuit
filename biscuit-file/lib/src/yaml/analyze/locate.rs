//! Path-to-source lookup over the lexical source map.
//!
//! Schema-aware consumers (Darkmatter's clean repair layer) validate the
//! parsed document and receive JSON-pointer instance paths; applying a
//! format-preserving repair requires mapping such a path back to the authored
//! bytes holding that value. [`locate_yaml_value`] answers that query against
//! the same lexical [`SourceMap`](super::scan) the analyzer uses, so spans
//! are exact for nested mappings/sequences, CRLF sources, comments, and
//! multibyte text — without reparsing or reserializing the document.
//!
//! Lookup is deliberately conservative: only block mapping values and block
//! sequence entries are locatable. Values inside flow collections, values
//! whose context path cannot be represented (e.g. a non-plain ancestor key),
//! and multi-line values yield `None`, which callers must treat as
//! "report-only — do not auto-edit".

use crate::span::SourceSpan;

use super::scan::{KeyOccurrence, LineKind, PathSegment, SourceMap};

/// One segment of a path into a YAML document value.
///
/// This is the public counterpart to the scanner's internal path segments,
/// shaped for consumers holding a JSON-pointer-style path: object keys map
/// to [`YamlPathSegment::Key`], sequence indices to
/// [`YamlPathSegment::Index`].
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum YamlPathSegment {
    /// A mapping key.
    Key(String),
    /// A block-sequence index.
    Index(usize),
}

/// The authored source location of a YAML value.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct YamlValueLocation {
    /// Byte span of the authored value text on its line: trailing whitespace
    /// and any trailing comment are excluded.
    pub span: SourceSpan,
    /// Byte span of the mapping key when the value sits under a block
    /// mapping key; `None` for block-sequence entries.
    pub key_span: Option<SourceSpan>,
    /// `true` when the value text is a plain (unquoted) scalar outside every
    /// flow collection — the only shape format-preserving scalar-quoting
    /// repairs may act on.
    pub plain: bool,
    /// Node properties and alias written at the start of the value text.
    ///
    /// [`span`](Self::span) still begins at the first of these tokens, so
    /// replacing `span` replaces the properties along with the value;
    /// callers that must edit only the value content should refuse a
    /// location whose properties are not
    /// [`is_empty`](YamlValueProperties::is_empty).
    pub properties: YamlValueProperties,
}

/// Anchor, tag, and alias tokens found at the start of a located value.
///
/// Each span covers the whole token including its indicator (`&name`,
/// `!tag`/`!!tag`/`!<uri>`, `*name`). Detection is lexical: a value text
/// cannot begin with `&`, `!`, or `*` except as one of these tokens.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct YamlValueProperties {
    /// The `&name` anchor declared on the value.
    pub anchor: Option<SourceSpan>,
    /// The `!tag` on the value.
    pub tag: Option<SourceSpan>,
    /// The `*name` alias, when the value is a reference to another node.
    pub alias: Option<SourceSpan>,
}

impl YamlValueProperties {
    /// `true` when the value carries no anchor, tag, or alias.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.anchor.is_none() && self.tag.is_none() && self.alias.is_none()
    }

    /// Reads the leading property and alias tokens of `source[span]`, in
    /// either order (`&a !!str x` and `!!str &a x` are both valid YAML).
    fn scan(source: &str, span: &SourceSpan) -> Self {
        let bytes = source.as_bytes();
        let mut properties = Self::default();
        let mut cursor = span.start;
        while cursor < span.end {
            let token_end = (cursor..span.end)
                .find(|&index| matches!(bytes[index], b' ' | b'\t'))
                .unwrap_or(span.end);
            let slot = match bytes[cursor] {
                b'&' => &mut properties.anchor,
                b'!' => &mut properties.tag,
                b'*' => &mut properties.alias,
                _ => break,
            };
            if slot.is_some() {
                break;
            }
            *slot = Some(cursor..token_end);
            cursor = (token_end..span.end)
                .find(|&index| !matches!(bytes[index], b' ' | b'\t'))
                .unwrap_or(span.end);
        }
        properties
    }
}

/// Locates the authored source span of the value at `path`.
///
/// `path` walks the document from the root: each [`YamlPathSegment::Key`]
/// descends into a block mapping, each [`YamlPathSegment::Index`] into a
/// block sequence. A sequence written at its owning key's indentation
/// (`key:\n- item`) is addressed under that key. Returns the first exact
/// match in source order, or `None` when no single-line block value lives at
/// that path (flow context, unrepresentable context, absent value, or a
/// plain, quoted, or flow value continuing onto later lines).
///
/// The scan is purely lexical and works on any source the analyzer's source
/// map covers, including documents that fail to parse; no YAML reparse is
/// performed.
#[must_use]
pub fn locate_yaml_value(source: &str, path: &[YamlPathSegment]) -> Option<YamlValueLocation> {
    let path: Vec<PathSegment> = path
        .iter()
        .map(|segment| match segment {
            YamlPathSegment::Key(key) => PathSegment::Key(key.clone()),
            YamlPathSegment::Index(index) => PathSegment::Index(*index),
        })
        .collect();
    let map = SourceMap::new(source);
    for line in 0..map.lines().len() {
        let entry = map.entry(line);
        let (value_span, key_span) = if let Some(mapping) = &entry.mapping {
            (mapping.value.clone(), Some(mapping.key.clone()))
        } else if entry.dash.is_some() {
            (entry.dash_value.clone(), None)
        } else {
            (None, None)
        };
        let Some(value_span) = value_span else {
            continue;
        };
        let Some(context) = map.block_value_context(source, line) else {
            continue;
        };
        if context.path == path {
            if continues_past_line(&map, line, &value_span, key_span.as_ref()) {
                return None;
            }
            let plain =
                !map.quoted_intersects(&value_span) && !map.flow_intersects(&value_span);
            let properties = YamlValueProperties::scan(source, &value_span);
            return Some(YamlValueLocation {
                span: value_span,
                key_span,
                plain,
                properties,
            });
        }
    }
    None
}

/// `true` when the value starting on `line` does not end there: a quoted
/// scalar or flow collection closing on a later line, or a following content
/// line indented past the entry's key (or dash) column, which YAML folds
/// into the value as a continuation.
fn continues_past_line(
    map: &SourceMap,
    line: usize,
    value_span: &SourceSpan,
    key_span: Option<&SourceSpan>,
) -> bool {
    let line_end = map.lines()[line].content.end;
    let spills = |start: usize, end: usize| {
        value_span.start <= start && start < value_span.end && end > line_end
    };
    if map
        .quoted_scalars()
        .iter()
        .any(|quoted| spills(quoted.span.start, quoted.span.end))
        || map
            .flow_regions()
            .iter()
            .any(|flow| spills(flow.span.start, flow.span.end))
    {
        return true;
    }
    let content_start = map.lines()[line].content.start;
    let column = match (key_span, map.entry(line).dash) {
        (Some(key), _) => key.start - content_start,
        (None, Some(dash)) => dash - content_start,
        (None, None) => map.lines()[line].indent,
    };
    map.lines()[line + 1..]
        .iter()
        .find(|next| next.kind != LineKind::Blank)
        .is_some_and(|next| next.kind == LineKind::Content && next.indent > column)
}

/// Locates the authored source span of the mapping **key** at `path`.
///
/// `path` is inclusive of the key itself (unlike [`locate_yaml_value`],
/// which takes the path to a value): `[style, page]` locates the `page` key
/// nested under `style`. Returns `None` when no block-mapping key lives at
/// that path. Diagnostics that concern a key rather than a value — a missing
/// required property's parent, or an undeclared key — anchor here.
#[must_use]
pub fn locate_yaml_key(source: &str, path: &[YamlPathSegment]) -> Option<SourceSpan> {
    if path.is_empty() {
        return None;
    }
    let path: Vec<PathSegment> = path
        .iter()
        .map(|segment| match segment {
            YamlPathSegment::Key(key) => PathSegment::Key(key.clone()),
            YamlPathSegment::Index(index) => PathSegment::Index(*index),
        })
        .collect();
    let map = SourceMap::new(source);
    map.key_occurrences(source)
        .into_iter()
        .find(|occurrence| occurrence.path == path)
        .map(|KeyOccurrence { key_span, .. }| key_span)
}
