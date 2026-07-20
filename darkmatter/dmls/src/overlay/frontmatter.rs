//! `FrontmatterAst` — a DMLS-owned, position-aware view of a document's YAML
//! frontmatter (AD-4 construction path).
//!
//! Construction is exactly the design's path: the Phase-1 library API
//! [`extract_frontmatter_block`] yields the YAML slice plus its document byte
//! base, `rlsp-yaml-parser` (lossless mode) parses it, and the first
//! document's root mapping is lowered into a flat arena of authored entries
//! carrying dotted-path, JSON-Pointer, and byte-span (document-relative)
//! coordinates. No provider ever sees the parser type — everything past this
//! module is [`FrontmatterAst`] and [`FmEntry`].
//!
//! Malformed-YAML policy (R-3): the parser returns positioned errors but no
//! partial tree on hard errors. [`FrontmatterAst::parse`] therefore returns
//! both an optional freshly-parsed tree *and* an optional [`YamlParseError`];
//! the overlay keeps the previous good tree for completion/hover continuity
//! while surfacing the current parse error as a diagnostic.

use darkmatter::markdown::extract_frontmatter_block;
use darkmatter::markdown::span::SourceSpan;
use rlsp_yaml_parser::loader::{self, LoadError};
use rlsp_yaml_parser::node::Node;
use rlsp_yaml_parser::Span as YamlSpan;

/// The value shape of one authored frontmatter entry.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FmValueKind {
    /// A scalar value (string, number, bool, null).
    Scalar,
    /// A nested block/flow mapping.
    Mapping,
    /// A sequence.
    Sequence,
    /// A YAML alias (`*anchor`).
    Alias,
}

/// One authored key/value entry in the frontmatter mapping tree.
///
/// All spans are **document-relative** byte offsets (already shifted past the
/// opening `---` delimiter), so they convert directly through a
/// [`SourceMap`](crate::source_map::SourceMap).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FmEntry {
    /// RFC 6901 JSON Pointer to this entry's value (`/style/page/margin`).
    pub pointer: String,
    /// Dotted path in authored spelling (`style.page.margin`).
    pub dotted: String,
    /// The leaf key segment.
    pub key: String,
    /// Byte span of the key token.
    pub key_span: SourceSpan,
    /// Byte span of the value node (for a mapping/sequence, the whole subtree).
    pub value_span: SourceSpan,
    /// The value's shape.
    pub kind: FmValueKind,
    /// The scalar value text, when [`kind`](Self::kind) is
    /// [`FmValueKind::Scalar`].
    pub scalar: Option<String>,
    /// Index of the parent entry in [`FrontmatterAst::entries`], if nested.
    pub parent: Option<usize>,
    /// Nesting depth (top-level keys are depth 0).
    pub depth: usize,
}

/// A positioned YAML parse failure.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct YamlParseError {
    /// Human-readable message from the parser.
    pub message: String,
    /// Document-relative byte span of the failure (zero-width at the reported
    /// position; falls back to the opening delimiter when the parser has none).
    pub span: SourceSpan,
}

/// The lowered frontmatter tree for one document version.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FrontmatterAst {
    entries: Vec<FmEntry>,
    /// Byte span of the whole frontmatter block (opening `---` line through the
    /// closing delimiter's terminator) — the fallback range for block-level
    /// diagnostics and the frontmatter fold.
    block_span: SourceSpan,
    /// Byte span of the root mapping (its entries), the visible range for
    /// root-level missing-required diagnostics.
    root_span: SourceSpan,
}

/// The outcome of parsing a document's frontmatter.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FrontmatterParse {
    /// The lowered tree, when the YAML parsed. `None` on a hard parse error.
    pub ast: Option<FrontmatterAst>,
    /// A parse error from the current buffer, when the YAML did not parse.
    pub error: Option<YamlParseError>,
}

impl FrontmatterAst {
    /// Parses the frontmatter of `document_text`.
    ///
    /// ## Returns
    ///
    /// - `None` when the document has no frontmatter block (nothing to analyze).
    /// - `Some(parse)` otherwise, where `parse.ast` is the lowered tree (or
    ///   `None` on a hard parse error) and `parse.error` carries a positioned
    ///   failure. A document whose block parses cleanly has `ast = Some`,
    ///   `error = None`.
    pub fn parse(document_text: &str) -> Option<FrontmatterParse> {
        let extraction = match extract_frontmatter_block(document_text) {
            Ok(Some(extraction)) => extraction,
            // No frontmatter, or a near-miss fence: nothing to analyze here (a
            // fence mismatch is the substrate's concern, not the overlay's).
            Ok(None) | Err(_) => return None,
        };
        let base = extraction.yaml_span.start;
        let block_span = extraction.block_span.clone();

        match loader::load(extraction.yaml) {
            Ok(documents) => {
                let root = documents.into_iter().next().map(|document| document.root);
                let ast = lower(root.as_ref(), base, block_span.clone());
                Some(FrontmatterParse { ast: Some(ast), error: None })
            }
            Err(error) => Some(FrontmatterParse {
                ast: None,
                error: Some(load_error_to_diagnostic(&error, base, &block_span)),
            }),
        }
    }

    /// Parses a standalone YAML source into the same position-aware mapping view.
    pub fn parse_yaml(source: &str) -> FrontmatterParse {
        let block_span = 0..source.len();
        match loader::load(source) {
            Ok(documents) => {
                let root = documents.into_iter().next().map(|document| document.root);
                FrontmatterParse {
                    ast: Some(lower(root.as_ref(), 0, block_span)),
                    error: None,
                }
            }
            Err(error) => FrontmatterParse {
                ast: None,
                error: Some(load_error_to_diagnostic(&error, 0, &block_span)),
            },
        }
    }

    /// All authored entries, in document order.
    pub fn entries(&self) -> &[FmEntry] {
        &self.entries
    }

    /// The whole frontmatter block's byte span.
    pub fn block_span(&self) -> SourceSpan {
        self.block_span.clone()
    }

    /// The root mapping's byte span (its entries).
    pub fn root_span(&self) -> SourceSpan {
        self.root_span.clone()
    }

    /// The entry at an exact JSON Pointer.
    pub fn entry_by_pointer(&self, pointer: &str) -> Option<&FmEntry> {
        self.entries.iter().find(|entry| entry.pointer == pointer)
    }

    /// The entry at an exact dotted path.
    ///
    /// ## Notes
    ///
    /// `dotted` is the *authored spelling* joined by `.`, so it cannot address a
    /// key that itself contains `.`. Callers composing a path from known key
    /// segments must use [`entry_by_key_path`](Self::entry_by_key_path), which
    /// is unambiguous for every key. This accessor exists for the library
    /// contracts that already hand DMLS a dotted string (`style:` warnings).
    pub fn entry_by_dotted(&self, dotted: &str) -> Option<&FmEntry> {
        self.entries.iter().find(|entry| entry.dotted == dotted)
    }

    /// The entry addressed by a chain of **decoded** key segments, outermost
    /// first.
    ///
    /// Unlike [`entry_by_dotted`](Self::entry_by_dotted) this round-trips every
    /// key: `.`, `:`, `/`, and `~` in a segment address exactly the authored
    /// key, because the lookup goes through RFC 6901 escaping rather than a
    /// dotted join.
    pub fn entry_by_key_path(&self, path: &[&str]) -> Option<&FmEntry> {
        self.entry_by_pointer(&pointer_for(path))
    }

    /// The chain of **decoded** key segments from the root down to the entry at
    /// arena `index`, outermost first.
    ///
    /// This is the structural replacement for splitting
    /// [`FmEntry::dotted`](FmEntry::dotted) on `.`: it walks the authored
    /// [`parent`](FmEntry::parent) chain, so a key such as `build.target` stays
    /// one segment.
    pub fn key_path_at(&self, index: usize) -> Vec<&str> {
        let mut path = Vec::new();
        let mut cursor = self.entries.get(index);
        while let Some(entry) = cursor {
            path.push(entry.key.as_str());
            cursor = entry.parent.and_then(|parent| self.entries.get(parent));
        }
        path.reverse();
        path
    }

    /// The key segments of `entry`, outermost first.
    ///
    /// Convenience over [`key_path_at`](Self::key_path_at) for callers holding a
    /// borrowed entry rather than its arena index.
    pub fn key_path(&self, entry: &FmEntry) -> Vec<&str> {
        self.index_of(entry).map(|index| self.key_path_at(index)).unwrap_or_default()
    }

    /// The arena index of `entry`, identified by its pointer.
    pub fn index_of(&self, entry: &FmEntry) -> Option<usize> {
        self.entries.iter().position(|candidate| candidate.pointer == entry.pointer)
    }

    /// The innermost authored mapping whose **value** subtree encloses `offset`
    /// — the container a line being authored at `offset` is a child of.
    ///
    /// Key tokens are deliberately excluded: a cursor inside `page:`'s own key
    /// belongs to `page`'s *parent*, not to `page`.
    pub fn container_at_offset(&self, offset: usize) -> Option<&FmEntry> {
        self.entries
            .iter()
            .filter(|entry| entry.kind == FmValueKind::Mapping)
            .filter(|entry| entry.value_span.contains(&offset))
            .max_by_key(|entry| entry.depth)
    }

    /// The ancestor key chain enclosing `offset`, outermost first — the
    /// structural answer to "which mapping is this line a child of".
    ///
    /// Empty at the top level, and empty when no authored mapping encloses
    /// `offset` (a blank line the parser saw no container for).
    pub fn enclosing_key_path(&self, offset: usize) -> Vec<&str> {
        self.container_at_offset(offset)
            .map(|entry| self.key_path(entry))
            .unwrap_or_default()
    }

    /// The direct children of the mapping addressed by `path`.
    pub fn children_of_key_path(&self, path: &[&str]) -> Vec<&FmEntry> {
        let pointer = pointer_for(path);
        let Some(parent) = self.entries.iter().position(|entry| entry.pointer == pointer) else {
            return if path.is_empty() {
                self.entries.iter().filter(|entry| entry.parent.is_none()).collect()
            } else {
                Vec::new()
            };
        };
        self.entries.iter().filter(|entry| entry.parent == Some(parent)).collect()
    }

    /// The entry whose **key token** lies on the cursor's line and ends at or
    /// before `offset` — the key whose value the cursor is authoring.
    ///
    /// Returning the authored entry (rather than the text before the first raw
    /// `:`) is what makes a quoted key correct: `"build.target": …` and
    /// `"host: port": …` both report their whole decoded key.
    pub fn key_entry_on_line(&self, line_start: usize, offset: usize) -> Option<&FmEntry> {
        self.entries
            .iter()
            .filter(|entry| entry.key_span.start >= line_start && entry.key_span.end <= offset)
            .max_by_key(|entry| entry.key_span.start)
    }

    /// The entry at `pointer`, else its nearest existing ancestor (R-5 mapping
    /// rules: array items and absent nested paths fall back to the closest
    /// authored container).
    pub fn entry_or_ancestor(&self, pointer: &str) -> Option<&FmEntry> {
        if let Some(entry) = self.entry_by_pointer(pointer) {
            return Some(entry);
        }
        let mut segments: Vec<&str> = split_pointer(pointer);
        while segments.pop().is_some() {
            let ancestor = join_pointer(&segments);
            if ancestor.is_empty() {
                return None;
            }
            if let Some(entry) = self.entry_by_pointer(&ancestor) {
                return Some(entry);
            }
        }
        None
    }

    /// The visible range for a value at `pointer` — the value node's span, or
    /// the nearest ancestor's, or the whole block.
    pub fn value_range(&self, pointer: &str) -> SourceSpan {
        self.entry_or_ancestor(pointer)
            .map(|entry| entry.value_span.clone())
            .unwrap_or_else(|| self.root_span.clone())
    }

    /// The visible range for the *parent mapping* of `pointer` — a real range,
    /// never zero-width, for missing-required diagnostics. The root parent
    /// (empty pointer) ranges the whole root mapping.
    pub fn parent_mapping_range(&self, parent_pointer: &str) -> SourceSpan {
        if parent_pointer.is_empty() {
            return self.root_span.clone();
        }
        self.entry_by_pointer(parent_pointer)
            .map(|entry| entry.value_span.clone())
            .unwrap_or_else(|| self.root_span.clone())
    }

    /// The entry for a child `key` under `parent_pointer`.
    pub fn child_entry(&self, parent_pointer: &str, key: &str) -> Option<&FmEntry> {
        let pointer = format!("{parent_pointer}/{}", encode_pointer_segment(key));
        self.entry_by_pointer(&pointer)
    }

    /// The key-token span for a child `key` under `parent_pointer` — the
    /// precise range for an unknown-key diagnostic.
    pub fn key_span_for(&self, parent_pointer: &str, key: &str) -> Option<SourceSpan> {
        self.child_entry(parent_pointer, key).map(|entry| entry.key_span.clone())
    }

    /// The `$schema` top-level entry, if present.
    pub fn schema_entry(&self) -> Option<&FmEntry> {
        self.entry_by_pointer("/$schema")
    }

    /// The deepest entry whose key or value span contains `offset`.
    pub fn entry_at_offset(&self, offset: usize) -> Option<&FmEntry> {
        self.entries
            .iter()
            .filter(|entry| {
                entry.key_span.contains(&offset) || entry.value_span.contains(&offset)
            })
            .max_by_key(|entry| entry.depth)
    }

    /// The top-level entries, in document order.
    pub fn top_level(&self) -> impl Iterator<Item = &FmEntry> {
        self.entries.iter().filter(|entry| entry.depth == 0)
    }

    /// Whether `offset` falls anywhere inside the frontmatter block.
    pub fn contains_offset(&self, offset: usize) -> bool {
        self.block_span.contains(&offset)
    }
}

/// Lowers the parsed root node into the flat entry arena.
fn lower(root: Option<&Node<YamlSpan>>, base: usize, block_span: SourceSpan) -> FrontmatterAst {
    let mut entries = Vec::new();
    let root_span = match root {
        Some(Node::Mapping { entries: pairs, loc, .. }) => {
            lower_mapping(pairs, base, "", "", None, 0, &mut entries);
            shift(*loc, base)
        }
        // A non-mapping root (a bare scalar, or an empty document) carries no
        // authored key/value entries; the block span is the only range.
        _ => block_span.clone(),
    };
    FrontmatterAst { entries, block_span, root_span }
}

/// Lowers one mapping's entries, recursing into nested mappings.
fn lower_mapping(
    pairs: &[(Node<YamlSpan>, Node<YamlSpan>)],
    base: usize,
    parent_pointer: &str,
    parent_dotted: &str,
    parent: Option<usize>,
    depth: usize,
    out: &mut Vec<FmEntry>,
) {
    for (key_node, value_node) in pairs {
        let Node::Scalar { value: key, loc: key_loc, .. } = key_node else {
            // Non-scalar keys (complex keys) are not addressable by dotted path;
            // skip them rather than invent a pointer.
            continue;
        };
        let pointer = format!("{parent_pointer}/{}", encode_pointer_segment(key));
        let dotted = if parent_dotted.is_empty() {
            key.clone()
        } else {
            format!("{parent_dotted}.{key}")
        };
        let (kind, scalar) = classify(value_node);
        let index = out.len();
        out.push(FmEntry {
            pointer: pointer.clone(),
            dotted: dotted.clone(),
            key: key.clone(),
            key_span: shift(*key_loc, base),
            value_span: value_span(value_node, base),
            kind,
            scalar,
            parent,
            depth,
        });
        if let Node::Mapping { entries: nested, .. } = value_node {
            lower_mapping(nested, base, &pointer, &dotted, Some(index), depth + 1, out);
        }
    }
}

/// Classifies a value node into a kind + optional scalar text.
fn classify(node: &Node<YamlSpan>) -> (FmValueKind, Option<String>) {
    match node {
        Node::Scalar { value, .. } => (FmValueKind::Scalar, Some(value.clone())),
        Node::Mapping { .. } => (FmValueKind::Mapping, None),
        Node::Sequence { .. } => (FmValueKind::Sequence, None),
        Node::Alias { .. } => (FmValueKind::Alias, None),
    }
}

/// The document-relative span of any value node.
fn value_span(node: &Node<YamlSpan>, base: usize) -> SourceSpan {
    let loc = match node {
        Node::Scalar { loc, .. }
        | Node::Mapping { loc, .. }
        | Node::Sequence { loc, .. }
        | Node::Alias { loc, .. } => *loc,
    };
    shift(loc, base)
}

/// Shifts a YAML-relative span into document coordinates.
fn shift(span: YamlSpan, base: usize) -> SourceSpan {
    (base + span.start as usize)..(base + span.end as usize)
}

/// Maps a loader error onto a document-relative diagnostic.
fn load_error_to_diagnostic(error: &LoadError, base: usize, block_span: &SourceSpan) -> YamlParseError {
    let (offset, message) = match error {
        LoadError::Parse { pos, message, .. } => (Some(pos.byte_offset), message.clone()),
        LoadError::NestingDepthLimitExceeded { pos, .. } => {
            (Some(pos.byte_offset), "nesting depth limit exceeded".to_string())
        }
        LoadError::AnchorCountLimitExceeded { pos, .. } => {
            (Some(pos.byte_offset), "anchor count limit exceeded".to_string())
        }
        LoadError::AliasExpansionLimitExceeded { pos, .. } => {
            (Some(pos.byte_offset), "alias expansion limit exceeded".to_string())
        }
        LoadError::CircularAlias { pos, name } => {
            (Some(pos.byte_offset), format!("circular alias `{name}`"))
        }
        LoadError::UndefinedAlias { pos, name } => {
            (Some(pos.byte_offset), format!("undefined alias `{name}`"))
        }
        LoadError::UnresolvedScalar { pos, .. } => {
            (Some(pos.byte_offset), "scalar does not match any type".to_string())
        }
        LoadError::UnexpectedEndOfStream => (None, "unexpected end of frontmatter".to_string()),
        // `LoadError` is `#[non_exhaustive]`; any future variant ranges the block.
        _ => (None, "invalid frontmatter YAML".to_string()),
    };
    let span = match offset {
        Some(offset) => {
            let at = base + offset;
            at..at
        }
        None => block_span.start..block_span.start,
    };
    YamlParseError { message, span }
}

/// RFC 6901 escapes a pointer segment (`~` → `~0`, `/` → `~1`).
///
/// `~` must be escaped before `/` so an authored `~1` is not confused with the
/// encoding of `/`.
fn encode_pointer_segment(segment: &str) -> String {
    segment.replace('~', "~0").replace('/', "~1")
}

/// Builds an RFC 6901 pointer from decoded key segments.
fn pointer_for(path: &[&str]) -> String {
    let mut pointer = String::new();
    for segment in path {
        pointer.push('/');
        pointer.push_str(&encode_pointer_segment(segment));
    }
    pointer
}

/// Splits a pointer into its raw (still-escaped) segments.
fn split_pointer(pointer: &str) -> Vec<&str> {
    if pointer.is_empty() {
        return Vec::new();
    }
    pointer.strip_prefix('/').unwrap_or(pointer).split('/').collect()
}

/// Rejoins raw pointer segments.
fn join_pointer(segments: &[&str]) -> String {
    if segments.is_empty() {
        return String::new();
    }
    let mut out = String::new();
    for segment in segments {
        out.push('/');
        out.push_str(segment);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ast(text: &str) -> FrontmatterAst {
        FrontmatterAst::parse(text)
            .expect("has frontmatter")
            .ast
            .expect("parses")
    }

    #[test]
    fn test_no_frontmatter_returns_none() {
        assert!(FrontmatterAst::parse("# Just a body\n").is_none());
    }

    #[test]
    fn test_top_level_entries_and_spans() {
        let text = "---\ntitle: Hello\ndraft: true\n---\n\n# Body\n";
        let ast = ast(text);
        let keys: Vec<&str> = ast.top_level().map(|e| e.key.as_str()).collect();
        assert_eq!(keys, vec!["title", "draft"]);
        let title = ast.entry_by_dotted("title").unwrap();
        assert_eq!(&text[title.key_span.clone()], "title");
        assert_eq!(&text[title.value_span.clone()], "Hello");
        assert_eq!(title.scalar.as_deref(), Some("Hello"));
        assert_eq!(title.kind, FmValueKind::Scalar);
    }

    #[test]
    fn test_nested_mapping_dotted_and_pointer_paths() {
        let text = "---\nstyle:\n  page:\n    margin: 2\n---\n\nbody\n";
        let ast = ast(text);
        let margin = ast.entry_by_dotted("style.page.margin").unwrap();
        assert_eq!(margin.pointer, "/style/page/margin");
        assert_eq!(margin.depth, 2);
        assert_eq!(&text[margin.value_span.clone()], "2");
        // The parent `style` mapping is addressable and its value span covers
        // the nested block.
        let style = ast.entry_by_pointer("/style").unwrap();
        assert_eq!(style.kind, FmValueKind::Mapping);
        assert!(text[style.value_span.clone()].contains("margin"));
    }

    #[test]
    fn test_entry_or_ancestor_falls_back_for_array_index() {
        let text = "---\ntags:\n  - a\n  - b\n---\n\nbody\n";
        let ast = ast(text);
        // `/tags/1` (an array item) is not an authored mapping entry; it falls
        // back to the `/tags` sequence node.
        let entry = ast.entry_or_ancestor("/tags/1").unwrap();
        assert_eq!(entry.pointer, "/tags");
        assert_eq!(entry.kind, FmValueKind::Sequence);
    }

    #[test]
    fn test_schema_entry_and_key_span_for() {
        let text = "---\n$schema: { title: string }\nextra: x\n---\n\nbody\n";
        let ast = ast(text);
        assert!(ast.schema_entry().is_some());
        // The offending key `extra` under the root has a precise key span.
        let span = ast.key_span_for("", "extra").unwrap();
        assert_eq!(&text[span], "extra");
    }

    #[test]
    fn test_parent_mapping_range_root_is_root_span() {
        let text = "---\ntitle: Hi\n---\n\nbody\n";
        let ast = ast(text);
        let range = ast.parent_mapping_range("");
        assert!(text[range].contains("title"));
    }

    #[test]
    fn test_entry_at_offset_prefers_deepest() {
        let text = "---\nstyle:\n  margin: 2\n---\n\nbody\n";
        let ast = ast(text);
        let margin = ast.entry_by_dotted("style.margin").unwrap();
        let offset = margin.value_span.start;
        assert_eq!(ast.entry_at_offset(offset).unwrap().key, "margin");
    }

    #[test]
    fn test_malformed_yaml_reports_error_no_tree() {
        // A tab-indented mapping value is a hard YAML error.
        let text = "---\nkey:\n\tbad: 1\n---\n\nbody\n";
        let parse = FrontmatterAst::parse(text).expect("has frontmatter");
        assert!(parse.ast.is_none());
        let error = parse.error.expect("parse error");
        // The error span is inside the frontmatter block.
        assert!(error.span.start >= 4);
    }

    #[test]
    fn test_offsets_are_document_relative() {
        let text = "---\ntitle: X\n---\n\nbody\n";
        let ast = ast(text);
        let title = ast.entry_by_dotted("title").unwrap();
        // `title` starts at byte 4 in the document, not byte 0 of the YAML.
        assert_eq!(title.key_span.start, 4);
    }
}
