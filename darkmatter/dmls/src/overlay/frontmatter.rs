//! `FrontmatterAst` — a DMLS-owned, position-aware view of a document's YAML
//! frontmatter (AD-4 construction path).
//!
//! Construction is exactly the design's path: the Phase-1 library API
//! [`extract_frontmatter_block`] yields the YAML slice plus its document byte
//! base, `rlsp-yaml-parser` (lossless mode) parses it, and the first
//! document's root mapping is lowered into a flat arena of authored entries
//! carrying dotted-path, JSON-Pointer, and byte-span (document-relative)
//! coordinates. Lowering descends through mappings **and** sequences: every
//! sequence item is an explicit [`FmEntryRole::SequenceItem`] entry, never a
//! mapping key with a fabricated key span (design:
//! `claudine/fixes/2026-09-13-better-static-analysis/spec.md`, D6). No provider ever sees the parser type — everything past this
//! module is [`FrontmatterAst`] and [`FmEntry`].
//!
//! Malformed-YAML policy (R-3): the parser returns positioned errors but no
//! partial tree on hard errors. [`FrontmatterAst::parse`] therefore returns
//! both an optional freshly-parsed tree *and* an optional [`YamlParseError`];
//! the overlay keeps the previous good tree for completion/hover continuity
//! while surfacing the current parse error as a diagnostic.

use std::collections::HashMap;
use std::sync::Arc;

use darkmatter::markdown::extract_frontmatter_block;
use darkmatter::markdown::span::SourceSpan;
use rlsp_yaml_parser::loader::{self, LoadError};
use rlsp_yaml_parser::node::{Node, NodeMeta};
use rlsp_yaml_parser::{ScalarStyle, Span as YamlSpan};

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

/// How an entry is addressed within its parent collection.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FmEntryRole {
    /// An authored `key: value` pair in a mapping.
    MappingProperty,
    /// The item at `index` in a sequence. It has no authored key token.
    SequenceItem {
        /// Zero-based position in the parent sequence.
        index: usize,
    },
}

/// The presentation style of an authored scalar — the input to choosing a safe
/// decoded-to-authored projection.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FmScalarStyle {
    /// An unquoted plain scalar.
    Plain,
    /// A `'single-quoted'` scalar.
    SingleQuoted,
    /// A `"double-quoted"` scalar.
    DoubleQuoted,
    /// A `|` literal block scalar (any chomping or indentation indicator).
    Literal,
    /// A `>` folded block scalar (any chomping or indentation indicator).
    Folded,
}

/// One typed step of a frontmatter path.
///
/// A mapping key literally spelled `0` and sequence item `0` share a pointer
/// segment but never a typed segment, which is what lets a schema walk consume
/// an array index against an array type instead of looking up a property.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FmPathSegment<'a> {
    /// A decoded mapping key.
    Key(&'a str),
    /// A sequence item index.
    Index(usize),
}

/// Renders typed segments in dotted notation: keys join with `.`, indices are
/// bracketed (`initialize.stack[0].when`), so a key named `0` stays `.0`.
///
/// This is the one dotted formatter; [`FmEntry::dotted`] is built with it.
pub fn format_dotted(segments: &[FmPathSegment<'_>]) -> String {
    let mut dotted = String::new();
    for segment in segments {
        push_dotted(&mut dotted, *segment);
    }
    dotted
}

fn push_dotted(dotted: &mut String, segment: FmPathSegment<'_>) {
    match segment {
        FmPathSegment::Key(key) => {
            if !dotted.is_empty() {
                dotted.push('.');
            }
            dotted.push_str(key);
        }
        FmPathSegment::Index(index) => {
            dotted.push('[');
            dotted.push_str(&index.to_string());
            dotted.push(']');
        }
    }
}

/// One authored entry in the frontmatter tree: a mapping property or a
/// sequence item.
///
/// All spans are **document-relative** byte offsets (already shifted past the
/// opening `---` delimiter), so they convert directly through a
/// [`SourceMap`](crate::source_map::SourceMap).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FmEntry {
    /// RFC 6901 JSON Pointer to this entry's value (`/style/page/margin`,
    /// `/tags/1`).
    pub pointer: String,
    /// Dotted path in authored spelling (`style.page.margin`, `tags[1]`).
    pub dotted: String,
    /// The decoded path segment: the key for a mapping property, the decimal
    /// index for a sequence item.
    pub key: String,
    /// Whether this entry is a mapping property or a sequence item.
    pub role: FmEntryRole,
    /// Byte span of the key token; `Some` exactly for a
    /// [`FmEntryRole::MappingProperty`].
    pub key_span: Option<SourceSpan>,
    /// Byte span of the value node (for a mapping/sequence, the whole subtree).
    pub value_span: SourceSpan,
    /// The value's shape.
    pub kind: FmValueKind,
    /// The scalar value text, when [`kind`](Self::kind) is
    /// [`FmValueKind::Scalar`].
    pub scalar: Option<String>,
    /// For an [`FmValueKind::Alias`], the text of the scalar it resolves to:
    /// the last `&name` scalar defined before it. `None` when that node is a
    /// collection. Every alias of one definition shares this allocation.
    pub alias_target: Option<Arc<str>>,
    /// For an [`FmValueKind::Alias`], the document offset where the scalar
    /// behind [`alias_target`](Self::alias_target) is authored, after its tag
    /// and anchor. `None` for a collection, and when the anchor has more than
    /// one definition before the alias: such an alias is analyzed on its own
    /// token.
    pub alias_target_start: Option<usize>,
    /// The authored scalar style, when [`kind`](Self::kind) is
    /// [`FmValueKind::Scalar`].
    pub scalar_style: Option<FmScalarStyle>,
    /// Whether the value node was authored with an explicit YAML tag (`!!str`,
    /// `!x`), which can change how its text is resolved.
    pub tagged: bool,
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

    /// The chain of **decoded** segments from the root down to the entry at
    /// arena `index`, outermost first.
    ///
    /// This is the structural replacement for splitting
    /// [`FmEntry::dotted`](FmEntry::dotted) on `.`: it walks the authored
    /// [`parent`](FmEntry::parent) chain, so a key such as `build.target` stays
    /// one segment.
    ///
    /// A sequence item contributes its decimal index, which addresses exactly
    /// like a pointer segment but cannot be told apart from a key spelled the
    /// same way. Schema resolution must use [`path_at`](Self::path_at).
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

    /// The typed path from the root down to the entry at arena `index`,
    /// outermost first. O(depth).
    pub fn path_at(&self, index: usize) -> Vec<FmPathSegment<'_>> {
        let mut path = Vec::new();
        let mut cursor = self.entries.get(index);
        while let Some(entry) = cursor {
            path.push(match entry.role {
                FmEntryRole::MappingProperty => FmPathSegment::Key(entry.key.as_str()),
                FmEntryRole::SequenceItem { index } => FmPathSegment::Index(index),
            });
            cursor = entry.parent.and_then(|parent| self.entries.get(parent));
        }
        path.reverse();
        path
    }

    /// Whether the entry at arena `index` is a sequence item or lies beneath
    /// one. O(depth).
    pub fn is_in_sequence(&self, index: usize) -> bool {
        let mut cursor = self.entries.get(index);
        while let Some(entry) = cursor {
            if matches!(entry.role, FmEntryRole::SequenceItem { .. }) {
                return true;
            }
            cursor = entry.parent.and_then(|parent| self.entries.get(parent));
        }
        false
    }

    /// The key segments of `entry`, outermost first.
    ///
    /// Convenience over [`key_path_at`](Self::key_path_at) for callers holding a
    /// borrowed entry rather than its arena index. O(n) through
    /// [`index_of`](Self::index_of).
    pub fn key_path(&self, entry: &FmEntry) -> Vec<&str> {
        self.index_of(entry).map(|index| self.key_path_at(index)).unwrap_or_default()
    }

    /// The typed path of `entry`. O(n) through [`index_of`](Self::index_of).
    pub fn path_of(&self, entry: &FmEntry) -> Vec<FmPathSegment<'_>> {
        self.index_of(entry).map(|index| self.path_at(index)).unwrap_or_default()
    }

    /// The arena index of `entry`, identified by its pointer.
    ///
    /// ## Notes
    ///
    /// This is a linear scan. Never call it — or [`key_path`](Self::key_path),
    /// [`path_of`](Self::path_of), [`entry_by_pointer`](Self::entry_by_pointer),
    /// or [`entry_by_dotted`](Self::entry_by_dotted) — inside a loop over
    /// [`entries`](Self::entries): sequence descent grows the arena to thousands
    /// of entries on real documents, and that shape is quadratic. Iterate with
    /// the index and use [`key_path_at`](Self::key_path_at) /
    /// [`path_at`](Self::path_at) instead.
    pub fn index_of(&self, entry: &FmEntry) -> Option<usize> {
        self.entries.iter().position(|candidate| candidate.pointer == entry.pointer)
    }

    /// The innermost authored mapping or sequence whose **value** subtree
    /// encloses `offset` — the container a line being authored at `offset` is a
    /// child of.
    ///
    /// Key tokens are deliberately excluded: a cursor inside `page:`'s own key
    /// belongs to `page`'s *parent*, not to `page`.
    ///
    /// A sequence-valued key owns the `- item` lines beneath it, so leaving
    /// sequences out truncates an item's ancestor chain at the nearest mapping
    /// and loses the key whose declaration the item is authored against. That
    /// only holds for a *block* sequence: `ratios: [0.25]` is the `ratios`
    /// line's own value, not an ancestor of it, so a sequence qualifies only
    /// when its key precedes `line_start` (the offset of the cursor's line).
    pub fn container_at_offset(&self, offset: usize, line_start: usize) -> Option<&FmEntry> {
        self.entries
            .iter()
            .filter(|entry| match entry.kind {
                FmValueKind::Mapping => true,
                // An item sequence has no key token; its own line is its value.
                FmValueKind::Sequence => {
                    entry.key_span.as_ref().map_or(entry.value_span.start, |key| key.end)
                        <= line_start
                }
                FmValueKind::Scalar | FmValueKind::Alias => false,
            })
            .filter(|entry| entry.value_span.contains(&offset))
            .max_by_key(|entry| entry.depth)
    }

    /// The ancestor key chain enclosing `offset`, outermost first — the
    /// structural answer to "which container is this line a child of".
    ///
    /// Empty at the top level, and empty when no authored container encloses
    /// `offset` (a blank line the parser saw no container for).
    pub fn enclosing_key_path(&self, offset: usize, line_start: usize) -> Vec<&str> {
        self.container_at_offset(offset, line_start)
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
            .filter_map(|entry| entry.key_span.as_ref().map(|key| (entry, key)))
            .filter(|(_, key)| key.start >= line_start && key.end <= offset)
            .max_by_key(|(_, key)| key.start)
            .map(|(entry, _)| entry)
    }

    /// The entry at `pointer`, else its nearest existing ancestor (R-5 mapping
    /// rules: absent nested paths fall back to the closest authored container;
    /// an authored array item is an exact hit).
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
        self.child_entry(parent_pointer, key).and_then(|entry| entry.key_span.clone())
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
                entry.key_span.as_ref().is_some_and(|key| key.contains(&offset))
                    || entry.value_span.contains(&offset)
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
            let parent = Parent { pointer: "", dotted: "", index: None, depth: 0 };
            lower_mapping(pairs, base, &parent, &mut HashMap::new(), &mut entries);
            shift(*loc, base)
        }
        // A non-mapping root (a bare scalar, or an empty document) carries no
        // authored key/value entries; the block span is the only range.
        _ => block_span.clone(),
    };
    FrontmatterAst { entries, block_span, root_span }
}

/// The already-lowered collection whose children are being lowered.
struct Parent<'a> {
    pointer: &'a str,
    dotted: &'a str,
    index: Option<usize>,
    depth: usize,
}

/// Lowers one mapping's entries, recursing into nested collections.
///
/// `anchors` holds each anchor's definition as of the node being lowered, in
/// document order, so an alias reads the last definition before it.
fn lower_mapping(
    pairs: &[(Node<YamlSpan>, Node<YamlSpan>)],
    base: usize,
    parent: &Parent<'_>,
    anchors: &mut HashMap<String, AnchorDefinition>,
    out: &mut Vec<FmEntry>,
) {
    for (key_node, value_node) in pairs {
        record_anchors(key_node, anchors);
        let Node::Scalar { value: key, loc: key_loc, .. } = key_node else {
            // Non-scalar keys (complex keys) are not addressable by dotted path;
            // skip them rather than invent a pointer.
            continue;
        };
        lower_entry(
            value_node,
            base,
            parent,
            key,
            FmEntryRole::MappingProperty,
            Some(shift(*key_loc, base)),
            anchors,
            out,
        );
    }
}

/// The node an anchor name currently resolves to.
struct AnchorDefinition {
    /// The node's text, when it is a scalar, shared by every alias of it.
    text: Option<Arc<str>>,
    /// YAML-relative offset of the node, which the parser reports after the
    /// node's tag and anchor.
    start: usize,
    /// An earlier node already defined the same name.
    redefined: bool,
}

/// Records the anchor `node` itself defines.
fn record_anchor(node: &Node<YamlSpan>, anchors: &mut HashMap<String, AnchorDefinition>) {
    if let Some(name) = node.anchor() {
        let (text, loc) = match node {
            Node::Scalar { value, loc, .. } => (Some(Arc::from(value.as_str())), loc),
            Node::Mapping { loc, .. } | Node::Sequence { loc, .. } | Node::Alias { loc, .. } => {
                (None, loc)
            }
        };
        let definition = AnchorDefinition {
            text,
            start: loc.start as usize,
            redefined: anchors.contains_key(name),
        };
        anchors.insert(name.to_string(), definition);
    }
}

/// Records every anchor in `node`'s subtree, in document order.
fn record_anchors(node: &Node<YamlSpan>, anchors: &mut HashMap<String, AnchorDefinition>) {
    record_anchor(node, anchors);
    match node {
        Node::Mapping { entries, .. } => {
            for (key, value) in entries {
                record_anchors(key, anchors);
                record_anchors(value, anchors);
            }
        }
        Node::Sequence { items, .. } => {
            for item in items {
                record_anchors(item, anchors);
            }
        }
        Node::Scalar { .. } | Node::Alias { .. } => {}
    }
}

/// Lowers one entry and, when its value is a collection, its children.
///
/// An alias is resolved against `anchors` *before* this node's own anchor is
/// recorded, so it sees exactly the definitions authored before it.
#[allow(clippy::too_many_arguments)]
fn lower_entry(
    value_node: &Node<YamlSpan>,
    base: usize,
    parent: &Parent<'_>,
    key: &str,
    role: FmEntryRole,
    key_span: Option<SourceSpan>,
    anchors: &mut HashMap<String, AnchorDefinition>,
    out: &mut Vec<FmEntry>,
) {
    let pointer = format!("{}/{}", parent.pointer, encode_pointer_segment(key));
    let mut dotted = parent.dotted.to_string();
    push_dotted(
        &mut dotted,
        match role {
            FmEntryRole::MappingProperty => FmPathSegment::Key(key),
            FmEntryRole::SequenceItem { index } => FmPathSegment::Index(index),
        },
    );
    let (kind, scalar, scalar_style, tagged) = classify(value_node);
    let definition = match value_node {
        Node::Alias { name, .. } => anchors.get(name),
        _ => None,
    };
    let alias_target = definition.and_then(|definition| definition.text.clone());
    let alias_target_start = definition
        .filter(|definition| definition.text.is_some() && !definition.redefined)
        .map(|definition| base + definition.start);
    let index = out.len();
    out.push(FmEntry {
        pointer: pointer.clone(),
        dotted: dotted.clone(),
        key: key.to_string(),
        role,
        key_span,
        value_span: value_span(value_node, base),
        kind,
        scalar,
        alias_target,
        alias_target_start,
        scalar_style,
        tagged,
        parent: parent.index,
        depth: parent.depth,
    });
    // Every collection is lowered below, so each node records its own anchor
    // here, once, in document order.
    record_anchor(value_node, anchors);
    let children = Parent {
        pointer: &pointer,
        dotted: &dotted,
        index: Some(index),
        depth: parent.depth + 1,
    };
    match value_node {
        Node::Mapping { entries: nested, .. } => lower_mapping(nested, base, &children, anchors, out),
        Node::Sequence { items, .. } => {
            for (item_index, item) in items.iter().enumerate() {
                lower_entry(
                    item,
                    base,
                    &children,
                    &item_index.to_string(),
                    FmEntryRole::SequenceItem { index: item_index },
                    None,
                    anchors,
                    out,
                );
            }
        }
        Node::Scalar { .. } | Node::Alias { .. } => {}
    }
}

/// Classifies a value node into its kind, scalar text and style, and tag
/// presence.
fn classify(node: &Node<YamlSpan>) -> (FmValueKind, Option<String>, Option<FmScalarStyle>, bool) {
    match node {
        Node::Scalar { value, style, meta, .. } => {
            let style = match style {
                ScalarStyle::Plain => FmScalarStyle::Plain,
                ScalarStyle::SingleQuoted => FmScalarStyle::SingleQuoted,
                ScalarStyle::DoubleQuoted => FmScalarStyle::DoubleQuoted,
                ScalarStyle::Literal(_) => FmScalarStyle::Literal,
                ScalarStyle::Folded(_) => FmScalarStyle::Folded,
            };
            (FmValueKind::Scalar, Some(value.clone()), Some(style), explicit_tag(meta.as_deref()))
        }
        Node::Mapping { meta, .. } => (FmValueKind::Mapping, None, None, explicit_tag(meta.as_deref())),
        Node::Sequence { meta, .. } => {
            (FmValueKind::Sequence, None, None, explicit_tag(meta.as_deref()))
        }
        Node::Alias { .. } => (FmValueKind::Alias, None, None, false),
    }
}

/// Whether a node was authored with a tag. The loader resolves a schema tag
/// onto every untagged node, so only a source tag location proves authorship.
fn explicit_tag(meta: Option<&NodeMeta<YamlSpan>>) -> bool {
    meta.is_some_and(|meta| meta.tag_loc.is_some())
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
        assert!(ast.entries().iter().all(|e| e.role == FmEntryRole::MappingProperty));
        let keys: Vec<&str> = ast.top_level().map(|e| e.key.as_str()).collect();
        assert_eq!(keys, vec!["title", "draft"]);
        let title = ast.entry_by_dotted("title").unwrap();
        assert_eq!(&text[title.key_span.clone().unwrap()], "title");
        assert_eq!(&text[title.value_span.clone()], "Hello");
        assert_eq!(title.scalar.as_deref(), Some("Hello"));
        assert_eq!(title.kind, FmValueKind::Scalar);
    }

    #[test]
    fn test_an_alias_resolves_to_the_last_scalar_anchor_defined_before_it() {
        let text = concat!(
            "---\n",
            "list:\n  - &item first\n",
            "early: *item\n",
            "nested:\n  inner: &item second\n",
            "late: *item\n",
            "map: &shape\n  a: 1\n",
            "collection: *shape\n",
            "redefined: &item third\n",
            "---\n",
        );
        let ast = ast(text);
        let target = |key: &str| {
            let entry = ast.entry_by_dotted(key).unwrap();
            assert_eq!((entry.kind, entry.scalar.as_deref()), (FmValueKind::Alias, None));
            entry.alias_target.clone()
        };
        assert_eq!(target("early").as_deref(), Some("first"));
        assert_eq!(target("late").as_deref(), Some("second"));
        assert_eq!(target("collection"), None);
        assert_eq!(ast.entry_by_dotted("redefined").unwrap().alias_target, None);

        // Only a single definition before the alias carries its start; the
        // redefinition after `early` does not count against it.
        let start = |key: &str| ast.entry_by_dotted(key).unwrap().alias_target_start;
        assert_eq!(start("early"), text.find("first"));
        assert_eq!(start("late"), None);
        assert_eq!(start("collection"), None);
        assert_eq!(start("redefined"), None);
    }

    #[test]
    fn test_an_alias_target_start_skips_the_nodes_tag_and_anchor() {
        for text in [
            "---\nnaïve: &a !!str 'café x'\nblock:\n  - !!str &b |-\n    y\none: *a\ntwo: *b\n---\n",
            "---\r\nnaïve: &a !!str 'café x'\r\nblock:\r\n  - !!str &b |-\r\n    y\r\none: *a\r\ntwo: *b\r\n---\r\n",
        ] {
            let ast = ast(text);
            let start = |key: &str| ast.entry_by_dotted(key).unwrap().alias_target_start;
            assert_eq!(start("one"), text.find("'café"));
            assert_eq!(start("two"), text.find("|-"));
        }
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
    fn test_entry_or_ancestor_is_exact_for_array_item() {
        // Deliberate design switch (item-entries descent, spec D6): this test
        // was `test_entry_or_ancestor_falls_back_for_array_index`, which pinned
        // `/tags/1` falling back to the `/tags` sequence. An authored item is now
        // an exact scalar hit; only an absent index still falls back.
        let text = "---\ntags:\n  - a\n  - b\n---\n\nbody\n";
        let ast = ast(text);
        let entry = ast.entry_or_ancestor("/tags/1").unwrap();
        assert_eq!(entry.pointer, "/tags/1");
        assert_eq!(entry.kind, FmValueKind::Scalar);
        assert_eq!(&text[entry.value_span.clone()], "b");
        assert_eq!(ast.value_range("/tags/1"), entry.value_span);
        let absent = ast.entry_or_ancestor("/tags/5").unwrap();
        assert_eq!(absent.pointer, "/tags");
        assert_eq!(absent.kind, FmValueKind::Sequence);
    }

    #[test]
    fn test_sequence_items_are_explicit_entries() {
        let text = "---\ntitle: T\ntags:\n  - a\n  - b\n---\n\nbody\n";
        let ast = ast(text);
        let tags_index = ast.entries().iter().position(|e| e.pointer == "/tags").unwrap();
        let tags = &ast.entries()[tags_index];
        assert_eq!(tags.role, FmEntryRole::MappingProperty);
        assert_eq!(&text[tags.key_span.clone().unwrap()], "tags");

        let item = ast.entry_by_pointer("/tags/1").unwrap();
        assert_eq!(item.role, FmEntryRole::SequenceItem { index: 1 });
        assert_eq!(item.key_span, None);
        assert_eq!(item.key, "1");
        assert_eq!(item.dotted, "tags[1]");
        assert_eq!(item.kind, FmValueKind::Scalar);
        assert_eq!(item.scalar.as_deref(), Some("b"));
        assert_eq!(&text[item.value_span.clone()], "b");
        assert_eq!(item.parent, Some(tags_index));
        assert_eq!(item.depth, 1);
        let item_index = ast.index_of(item).unwrap();
        assert_eq!(ast.key_path_at(item_index), vec!["tags", "1"]);
        assert_eq!(
            ast.path_at(item_index),
            vec![FmPathSegment::Key("tags"), FmPathSegment::Index(1)]
        );
        assert!(ast.is_in_sequence(item_index));
        assert!(!ast.is_in_sequence(tags_index));
        // Arena order is document order: the sequence precedes its items.
        let pointers: Vec<&str> = ast.entries().iter().map(|e| e.pointer.as_str()).collect();
        assert_eq!(pointers, vec!["/title", "/tags", "/tags/0", "/tags/1"]);
    }

    #[test]
    fn test_mapping_items_descend_to_their_keys() {
        let text = concat!(
            "---\n",
            "initialize:\n",
            "  stack:\n",
            "    - when: ctx.ok\n",
            "      action: stop\n",
            "    - action:\n",
            "        - skip\n",
            "---\n\nbody\n",
        );
        let ast = ast(text);
        let when_index = ast
            .entries()
            .iter()
            .position(|e| e.pointer == "/initialize/stack/0/when")
            .unwrap();
        let when = &ast.entries()[when_index];
        assert_eq!(when.dotted, "initialize.stack[0].when");
        assert_eq!(when.role, FmEntryRole::MappingProperty);
        assert_eq!(&text[when.key_span.clone().unwrap()], "when");
        assert_eq!(&text[when.value_span.clone()], "ctx.ok");
        assert_eq!(when.depth, 3);
        let item = &ast.entries()[when.parent.unwrap()];
        assert_eq!(item.pointer, "/initialize/stack/0");
        assert_eq!(item.role, FmEntryRole::SequenceItem { index: 0 });
        assert_eq!(item.kind, FmValueKind::Mapping);
        assert_eq!(item.depth, 2);
        assert_eq!(ast.entries()[item.parent.unwrap()].pointer, "/initialize/stack");
        assert_eq!(
            ast.path_at(when_index),
            vec![
                FmPathSegment::Key("initialize"),
                FmPathSegment::Key("stack"),
                FmPathSegment::Index(0),
                FmPathSegment::Key("when"),
            ]
        );
        assert_eq!(format_dotted(&ast.path_at(when_index)), when.dotted);

        let skip = ast.entry_by_dotted("initialize.stack[1].action[0]").unwrap();
        assert_eq!(skip.pointer, "/initialize/stack/1/action/0");
        assert_eq!(skip.scalar.as_deref(), Some("skip"));
        assert_eq!(skip.depth, 4);
        assert_eq!(
            ast.entry_by_key_path(&["initialize", "stack", "1", "action", "0"]).unwrap().pointer,
            skip.pointer
        );
        // The cursor on `when`'s value belongs to the item mapping, not the
        // `stack` sequence.
        let offset = when.value_span.start;
        assert_eq!(ast.entry_at_offset(offset).unwrap().pointer, when.pointer);
    }

    #[test]
    fn test_index_and_numeric_key_paths_stay_distinct() {
        let text = concat!(
            "---\n",
            "numbered:\n  \"0\": zero\n",
            "listed:\n  - zero\n",
            "matrix:\n  - [1, 2]\n",
            "a/b~c:\n  - x\n",
            "---\n\nbody\n",
        );
        let ast = ast(text);
        let key_zero = ast.entry_by_pointer("/numbered/0").unwrap();
        assert_eq!(key_zero.role, FmEntryRole::MappingProperty);
        assert_eq!(key_zero.dotted, "numbered.0");
        let item_zero = ast.entry_by_pointer("/listed/0").unwrap();
        assert_eq!(item_zero.role, FmEntryRole::SequenceItem { index: 0 });
        assert_eq!(item_zero.dotted, "listed[0]");
        let key_index = ast.index_of(key_zero).unwrap();
        let item_index = ast.index_of(item_zero).unwrap();
        assert_eq!(ast.key_path_at(key_index), vec!["numbered", "0"]);
        assert_eq!(ast.key_path_at(item_index), vec!["listed", "0"]);
        assert_ne!(ast.path_at(key_index)[1], ast.path_at(item_index)[1]);

        let cell = ast.entry_by_pointer("/matrix/0/1").unwrap();
        assert_eq!(cell.dotted, "matrix[0][1]");
        assert_eq!(cell.scalar.as_deref(), Some("2"));
        let cell_index = ast.index_of(cell).unwrap();
        assert_eq!(format_dotted(&ast.path_at(cell_index)), "matrix[0][1]");

        // RFC 6901 escaping round-trips through an index-bearing pointer.
        let escaped = ast.entry_by_pointer("/a~1b~0c/0").unwrap();
        assert_eq!(escaped.dotted, "a/b~c[0]");
        assert_eq!(ast.entry_by_key_path(&["a/b~c", "0"]).unwrap().pointer, escaped.pointer);
        assert_eq!(ast.entry_or_ancestor("/a~1b~0c/0/deeper").unwrap().pointer, escaped.pointer);
    }

    #[test]
    fn test_scalar_style_and_tag_are_retained() {
        let text = concat!(
            "---\n",
            "plain: a\n",
            "single: 'a'\n",
            "double: \"a\"\n",
            "literal: |-\n  a\n",
            "kept: |+\n  a\n",
            "indented: |2-\n   a\n",
            "folded: >\n  a\n",
            "folded_strip: >-\n  a\n",
            "tagged: !!str a\n",
            "anchor: &shared a\n",
            "alias: *shared\n",
            "items:\n  - 'q'\n",
            "---\n\nbody\n",
        );
        let ast = ast(text);
        let style = |pointer: &str| {
            let entry = ast.entry_by_pointer(pointer).unwrap();
            (entry.scalar_style, entry.tagged)
        };
        assert_eq!(style("/plain"), (Some(FmScalarStyle::Plain), false));
        assert_eq!(style("/single"), (Some(FmScalarStyle::SingleQuoted), false));
        assert_eq!(style("/double"), (Some(FmScalarStyle::DoubleQuoted), false));
        assert_eq!(style("/literal"), (Some(FmScalarStyle::Literal), false));
        assert_eq!(style("/kept"), (Some(FmScalarStyle::Literal), false));
        assert_eq!(style("/indented"), (Some(FmScalarStyle::Literal), false));
        assert_eq!(style("/folded"), (Some(FmScalarStyle::Folded), false));
        assert_eq!(style("/folded_strip"), (Some(FmScalarStyle::Folded), false));
        assert_eq!(style("/tagged"), (Some(FmScalarStyle::Plain), true));
        // The value span excludes the tag token.
        assert_eq!(&text[ast.entry_by_pointer("/tagged").unwrap().value_span.clone()], "a");
        assert_eq!(style("/anchor"), (Some(FmScalarStyle::Plain), false));
        assert_eq!(style("/items/0"), (Some(FmScalarStyle::SingleQuoted), false));
        let alias = ast.entry_by_pointer("/alias").unwrap();
        assert_eq!(alias.kind, FmValueKind::Alias);
        assert_eq!((alias.scalar_style, alias.tagged), (None, false));
        assert_eq!(ast.entry_by_pointer("/items").unwrap().scalar_style, None);
    }

    #[test]
    fn test_container_at_offset_reaches_nested_item_sequences() {
        let text = "---\nouter:\n  -\n    - a\n    - b\n---\n\nbody\n";
        let ast = ast(text);
        let b = ast.entry_by_pointer("/outer/0/1").unwrap();
        let line_start = text[..b.value_span.start].rfind('\n').unwrap() + 1;
        let container = ast.container_at_offset(b.value_span.start, line_start).unwrap();
        assert_eq!(container.pointer, "/outer/0");
        assert_eq!(container.key_span, None);
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
        assert_eq!(title.key_span.as_ref().unwrap().start, 4);
    }
}
