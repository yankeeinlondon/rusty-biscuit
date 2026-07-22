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

use super::scan::{KeyOccurrence, PathSegment, SourceMap};

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
}

/// Locates the authored source span of the value at `path`.
///
/// `path` walks the document from the root: each [`YamlPathSegment::Key`]
/// descends into a block mapping, each [`YamlPathSegment::Index`] into a
/// block sequence. Returns the first exact match in source order, or `None`
/// when no block value lives at that path (flow context, unrepresentable
/// context, or absent value).
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
            let plain =
                !map.quoted_intersects(&value_span) && !map.flow_intersects(&value_span);
            return Some(YamlValueLocation {
                span: value_span,
                key_span,
                plain,
            });
        }
    }
    None
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
