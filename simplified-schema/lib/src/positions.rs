//! Authored source positions for frontmatter validation problems.
//!
//! A [`PositionMap`] is built at most once per validation source snapshot and
//! shared by every problem that snapshot reports, including each root-union
//! arm. Problems are located only after a validation has decided its final
//! problem list ([`PositionMap::locate_all`]), so a deferred map
//! ([`PositionMap::deferred`]) builds nothing for a validation that reports
//! nothing. Lookup is passive: it never evaluates or resolves anything.

use std::collections::HashSet;
use std::fmt;
use std::ops::Range;
use std::sync::{Arc, OnceLock};

use biscuit_file::serde_yaml_ng;
use indexmap::IndexMap;

use super::simplified::{
    SchemaValueKind, SchemaValueNode, locate_frontmatter_value, mapping_separator,
    yaml_scalar::alias_token,
};
use super::{ValidationProblem, ValidationProblemCode};
use crate::mapping_orders::MappingOrders;

/// Source coordinates for one frontmatter snapshot, used to annotate
/// [`ValidationProblem`](super::ValidationProblem)s with a 1-based line and a
/// 1-based UTF-8 byte column.
///
/// When the authored YAML can be projected onto its parsed structure, every
/// key, value, and mapping has its own start, so a problem points at the
/// offending node. An alias (`*name`) is an authored leaf: a problem at the
/// alias, or anywhere in the value it expands to, points at the alias token.
/// Otherwise only top-level keys are located, at column 1.
/// [`PositionMap::new`] is the empty map for source-free validation: its
/// problems carry no coordinates.
///
/// A map made by [`PositionMap::deferred`] keeps its text and builds its
/// coordinates on the first query that needs one ([`Self::is_empty`],
/// [`Self::byte_range`], or [`Self::locate_all`] with at least one problem).
/// Its clones share the text and that one build, from any thread.
#[derive(Clone)]
pub struct PositionMap {
    state: MapState,
}

#[derive(Clone)]
enum MapState {
    Built(PositionSource),
    Deferred(Arc<DeferredSource>),
}

/// The exact text of a deferred map, and its coordinates once a query needs
/// them.
struct DeferredSource {
    text: Arc<str>,
    built: OnceLock<PositionSource>,
}

impl Default for PositionMap {
    fn default() -> Self {
        Self { state: MapState::Built(PositionSource::Empty) }
    }
}

/// Never forces a deferred map: an unbuilt one shows only its text length.
impl fmt::Debug for PositionMap {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut debug = f.debug_struct("PositionMap");
        match &self.state {
            MapState::Built(source) => debug.field("source", source),
            MapState::Deferred(deferred) => match deferred.built.get() {
                Some(source) => debug.field("source", source),
                None => debug.field("deferred_text_len", &deferred.text.len()),
            },
        };
        debug.finish()
    }
}

#[derive(Debug, Clone, Default)]
enum PositionSource {
    #[default]
    Empty,
    /// An authored tree whose mappings match the parsed document's, except
    /// those expanded below an alias leaf, which have no authored text.
    Located { root: SchemaValueNode, lines: LineIndex },
    /// Top-level key → `(line, 1)`; precise projection was unavailable.
    TopLevel(IndexMap<String, (u32, u32)>),
}

/// What a problem's position should identify at its instance pointer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum PositionTarget<'a> {
    /// The value at the pointer.
    Value,
    /// The named key of the mapping at the pointer.
    Key(&'a str),
    /// The mapping at the pointer, which lacks a required key.
    Mapping,
}

/// Byte offset → `(line, column)` over one text.
#[derive(Debug, Clone)]
struct LineIndex {
    /// Byte offset at which each line starts.
    starts: Vec<usize>,
    /// The line number of byte 0.
    base_line: u32,
}

impl LineIndex {
    fn new(text: &str, base_line: u32) -> Self {
        let bytes = text.as_bytes();
        let mut starts = vec![0];
        for (index, byte) in bytes.iter().enumerate() {
            let breaks = match byte {
                b'\n' => true,
                b'\r' => bytes.get(index + 1) != Some(&b'\n'),
                _ => false,
            };
            if breaks {
                starts.push(index + 1);
            }
        }
        Self { starts, base_line }
    }

    fn position(&self, offset: usize) -> (u32, u32) {
        let line = self.starts.partition_point(|start| *start <= offset) - 1;
        let column = offset - self.starts[line] + 1;
        (
            self.base_line.saturating_add(line as u32),
            u32::try_from(column).unwrap_or(u32::MAX),
        )
    }
}

impl PositionMap {
    /// The empty map: problems validated against it carry no position.
    pub fn new() -> Self {
        Self::default()
    }

    /// The map [`build_position_map`] would build over `text`, built only
    /// when a query first needs a coordinate. Coordinates are the same as the
    /// eager map's: offsets into `text` exactly as given, with no line-ending
    /// or byte-order-mark normalization.
    pub fn deferred(text: Arc<str>) -> Self {
        Self { state: MapState::Deferred(Arc::new(DeferredSource { text, built: OnceLock::new() })) }
    }

    /// The coordinates, building a deferred map's on first use.
    fn source(&self) -> &PositionSource {
        match &self.state {
            MapState::Built(source) => source,
            MapState::Deferred(deferred) => deferred.built.get_or_init(|| build_source(&deferred.text)),
        }
    }

    /// `true` when no position can be reported.
    pub fn is_empty(&self) -> bool {
        match self.source() {
            PositionSource::Empty => true,
            PositionSource::Located { .. } => false,
            PositionSource::TopLevel(keys) => keys.is_empty(),
        }
    }

    /// Projects the YAML at `yaml` within `text` onto the document whose
    /// authored mapping keys are `orders`.
    ///
    /// `base_line` is the line number of `text`'s first byte, so positions
    /// count lines of `text` itself rather than of the YAML slice.
    ///
    /// ## Returns
    ///
    /// `None` when the locator cannot read the YAML, or what it read does not
    /// match `orders` mapping for mapping: the parsed value then has a
    /// structure the source cannot be trusted to show. An alias is an authored
    /// leaf spanning its `*name` token, and every indexed mapping at or below
    /// it is accounted for by that leaf. Any root kind (mapping, sequence, or
    /// scalar) is projected; frontmatter rejects a non-mapping root before it
    /// is validated.
    pub fn project(
        text: &str,
        yaml: Range<usize>,
        base_line: u32,
        orders: &MappingOrders,
    ) -> Option<Self> {
        project_source(text, yaml, base_line, orders).map(|source| Self { state: MapState::Built(source) })
    }

    /// Locates only the top-level keys of `yaml`, whose first line is
    /// `base_line`.
    pub fn top_level(yaml: &str, base_line: u32) -> Self {
        Self { state: MapState::Built(top_level_source(yaml, base_line)) }
    }

    /// The `(line, column)` of `target` at the decoded instance pointer
    /// `segments`.
    ///
    /// A path that leaves the authored tree falls back to the deepest
    /// authored non-root node on it, and a newly supplied top-level property
    /// has no position. A path through a scalar — content decoded from a
    /// block or quoted scalar — stops at that scalar's start.
    pub(super) fn locate(&self, segments: &[String], target: PositionTarget<'_>) -> Option<(u32, u32)> {
        match self.source() {
            PositionSource::Empty => None,
            PositionSource::TopLevel(keys) => {
                let key = match (segments.first(), target) {
                    (Some(first), _) => first.as_str(),
                    (None, PositionTarget::Key(key)) => key,
                    (None, _) => return None,
                };
                keys.get(key).copied()
            }
            PositionSource::Located { root, lines } => {
                locate_in(root, segments, target).map(|offset| lines.position(offset))
            }
        }
    }

    /// Sets `problem`'s line and column to the authored token its category
    /// identifies at its instance path, or clears them when none is located.
    fn locate_problem(&self, problem: &mut ValidationProblem) {
        // An unknown key's pointer names the parent mapping, and a missing key
        // has no node of its own, so the category picks the authored token to
        // report.
        let located = self.locate(problem.instance_path.segments(), problem_target(problem));
        problem.line = located.map(|(line, _)| line);
        problem.column = located.map(|(_, column)| column);
    }
}

impl PositionMap {
    /// The authored byte range of the token `problem` is reported at (the
    /// node at its instance path, or the offending key of an unknown-key
    /// problem), as offsets into the text the map was built from. For an
    /// alias, or a path below one, it is the `*name` token. `None` when the
    /// map holds no authored tree (empty, or top-level keys only).
    pub fn byte_range(&self, problem: &ValidationProblem) -> Option<Range<usize>> {
        let PositionSource::Located { root, .. } = self.source() else {
            return None;
        };
        locate_range_in(root, problem.instance_path.segments(), problem_target(problem))
    }

    /// Locates every problem of a settled report; does nothing when
    /// `problems` is empty.
    pub fn locate_all(&self, problems: &mut [ValidationProblem]) {
        if problems.is_empty() {
            return;
        }
        for problem in problems {
            self.locate_problem(problem);
        }
    }
}

/// The authored token a problem's category identifies.
fn problem_target(problem: &ValidationProblem) -> PositionTarget<'_> {
    match (problem.code, problem.offending_property.as_deref()) {
        (ValidationProblemCode::MissingRequired, _) => PositionTarget::Mapping,
        (_, Some(key)) => PositionTarget::Key(key),
        _ => PositionTarget::Value,
    }
}

/// The authored start selected for `target` at `segments` below `root`.
fn locate_in(root: &SchemaValueNode, segments: &[String], target: PositionTarget<'_>) -> Option<usize> {
    locate_range_in(root, segments, target).map(|range| range.start)
}

/// The authored byte range selected for `target` at `segments` below `root`:
/// the node's whole span, or the key's for [`PositionTarget::Key`]. A path
/// that leaves the authored tree yields the deepest authored non-root node.
fn locate_range_in(root: &SchemaValueNode, segments: &[String], target: PositionTarget<'_>) -> Option<Range<usize>> {
    let mut node = root;
    let mut deepest_non_root = None;
    for segment in segments {
        let child = match &node.kind {
            SchemaValueKind::Scalar => return Some(node.span.clone()),
            SchemaValueKind::Mapping(entries) => entries
                .iter()
                .find(|entry| {
                    crate::note_work!(PositionKeyComparison);
                    entry.key == *segment
                })
                .map(|entry| &entry.value),
            SchemaValueKind::Sequence(items) => {
                segment.parse::<usize>().ok().and_then(|index| items.get(index))
            }
        };
        match child {
            Some(child) => {
                node = child;
                deepest_non_root = Some(child.span.clone());
            }
            None => return deepest_non_root,
        }
    }
    match (target, &node.kind) {
        (PositionTarget::Key(key), SchemaValueKind::Mapping(entries)) => entries
            .iter()
            .find(|entry| {
                crate::note_work!(PositionKeyComparison);
                entry.key == key
            })
            .map(|entry| entry.key_span.clone())
            .or(deepest_non_root),
        (PositionTarget::Key(_), SchemaValueKind::Scalar) => Some(node.span.clone()),
        (PositionTarget::Key(_), SchemaValueKind::Sequence(_)) => deepest_non_root,
        (PositionTarget::Value | PositionTarget::Mapping, _) => Some(node.span.clone()),
    }
}

/// Whether every mapping in `node`'s subtree carries exactly the keys
/// `orders` records at its pointer, in order. Counts the mappings visited
/// into `mappings`, and records the pointer of each alias leaf in `aliases`
/// after narrowing its span to the `*name` token.
fn matches_orders(
    node: &mut SchemaValueNode,
    text: &str,
    orders: &MappingOrders,
    pointer: &mut String,
    mappings: &mut usize,
    aliases: &mut HashSet<String>,
) -> bool {
    match &mut node.kind {
        SchemaValueKind::Scalar => {
            if let Some(token) = alias_token(text, node.span.start) {
                node.span = token;
                aliases.insert(pointer.clone());
            }
            true
        }
        SchemaValueKind::Mapping(entries) => {
            *mappings += 1;
            let Some(keys) = orders.get(pointer) else {
                return false;
            };
            if keys.len() != entries.len()
                || keys.iter().zip(entries.iter()).any(|(key, entry)| *key != entry.key)
            {
                return false;
            }
            entries.iter_mut().all(|entry| {
                with_child(pointer, &entry.key, |pointer| {
                    matches_orders(&mut entry.value, text, orders, pointer, mappings, aliases)
                })
            })
        }
        SchemaValueKind::Sequence(items) => items.iter_mut().enumerate().all(|(index, item)| {
            with_child(pointer, &index.to_string(), |pointer| {
                matches_orders(item, text, orders, pointer, mappings, aliases)
            })
        }),
    }
}

/// How many indexed mappings sit at or below an alias leaf in `aliases`.
///
/// Pointers are RFC 6901 escaped, so `/` only separates segments and a prefix
/// ending before a `/` is an ancestor: `/a` covers `/a/b` but never `/ab`.
fn covered_by_aliases(orders: &MappingOrders, aliases: &HashSet<String>) -> usize {
    if aliases.is_empty() {
        return 0;
    }
    orders
        .pointers()
        .filter(|pointer| {
            aliases.contains(*pointer)
                || pointer
                    .match_indices('/')
                    .any(|(at, _)| at > 0 && aliases.contains(&pointer[..at]))
        })
        .count()
}

fn with_child(pointer: &mut String, segment: &str, visit: impl FnOnce(&mut String) -> bool) -> bool {
    let parent = pointer.len();
    *pointer = MappingOrders::child_pointer(pointer, segment);
    let matched = visit(pointer);
    pointer.truncate(parent);
    matched
}

/// Builds the [`PositionMap`] for a frontmatter YAML text.
///
/// Positions count lines of `yaml` itself, from 1, and columns in UTF-8
/// bytes from 1. `yaml` is parsed once to check that its authored structure
/// can be trusted; when it cannot (or does not parse), only top-level keys
/// are located, at column 1. Aliases are projected as leaves located at their
/// `*name` token. Quoted keys (`"foo"`, `'foo'`) are located by
/// their decoded key string. [`PositionMap::deferred`] builds the same map
/// when its first coordinate is requested.
pub fn build_position_map(yaml: &str) -> PositionMap {
    PositionMap { state: MapState::Built(build_source(yaml)) }
}

/// The one construction of a map from text, eager or deferred.
fn build_source(yaml: &str) -> PositionSource {
    crate::note_work!(PositionMapConstruction);
    serde_yaml_ng::from_str::<serde_yaml_ng::Value>(yaml)
        .ok()
        .and_then(|value| project_source(yaml, 0..yaml.len(), 1, &MappingOrders::collect(&value)))
        .unwrap_or_else(|| top_level_source(yaml, 1))
}

/// The projection [`PositionMap::project`] makes.
fn project_source(text: &str, yaml: Range<usize>, base_line: u32, orders: &MappingOrders) -> Option<PositionSource> {
    let mut root = locate_frontmatter_value(&text[yaml.clone()], yaml.start)?;
    let mut mappings = 0;
    let mut pointer = String::new();
    let mut aliases = HashSet::new();
    if !matches_orders(&mut root, text, orders, &mut pointer, &mut mappings, &mut aliases)
        || mappings + covered_by_aliases(orders, &aliases) != orders.len()
    {
        return None;
    }
    Some(PositionSource::Located {
        root,
        lines: LineIndex::new(&text[..yaml.end], base_line),
    })
}

/// The top-level keys [`PositionMap::top_level`] locates.
fn top_level_source(yaml: &str, base_line: u32) -> PositionSource {
    let mut keys = IndexMap::new();
    for (idx, line) in yaml.lines().enumerate() {
        let Some(first) = line.chars().next() else {
            continue;
        };
        if first.is_whitespace() || first == '#' || first == '-' || first == '.' {
            continue;
        }
        let Some(colon) = mapping_separator(line, false) else {
            continue;
        };
        let raw = line[..colon].trim();
        let key = raw
            .trim_start_matches(['"', '\''])
            .trim_end_matches(['"', '\''])
            .to_string();
        if !key.is_empty() {
            keys.insert(key, (base_line.saturating_add(idx as u32), 1));
        }
    }
    PositionSource::TopLevel(keys)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn segments(pointer: &str) -> Vec<String> {
        super::super::JsonPointer::parse(pointer).segments().to_vec()
    }

    fn at(map: &PositionMap, pointer: &str, target: PositionTarget<'_>) -> Option<(u32, u32)> {
        map.locate(&segments(pointer), target)
    }

    fn is_located(map: &PositionMap) -> bool {
        matches!(map.source(), PositionSource::Located { .. })
    }

    const ACCEPTANCE_YAML: &str = "start:\n  - then:\n      - say: hello\n      - sya: goodbye\n";

    /// The YAML slice counts its own lines; a complete document counts the
    /// opening delimiter too.
    #[test]
    fn nested_key_is_located_relative_to_the_indexed_text() {
        let slice = build_position_map(ACCEPTANCE_YAML);
        assert!(is_located(&slice));
        assert_eq!(at(&slice, "/start/0/then/1", PositionTarget::Key("sya")), Some((4, 9)));
        assert_eq!(at(&slice, "/start/0/then/1/sya", PositionTarget::Value), Some((4, 14)));

        let document = format!("---\n{ACCEPTANCE_YAML}---\nbody\n");
        let orders = MappingOrders::collect(&serde_yaml_ng::from_str(ACCEPTANCE_YAML).unwrap());
        let yaml = 4..4 + ACCEPTANCE_YAML.len();
        let map = PositionMap::project(&document, yaml, 1, &orders).expect("projects");
        assert_eq!(at(&map, "/start/0/then/1", PositionTarget::Key("sya")), Some((5, 9)));
    }

    #[test]
    fn crlf_offsets_address_the_original_bytes() {
        let document = "---\r\nstart:\r\n  - then:\r\n      - say: hello\r\n      - sya: goodbye\r\n---\r\n";
        let block = biscuit_file::frontmatter::extract_frontmatter_block(document).unwrap().unwrap();
        let orders = MappingOrders::collect(&serde_yaml_ng::from_str(block.yaml).unwrap());
        let map = PositionMap::project(document, block.yaml_span.clone(), 1, &orders).unwrap();
        let PositionSource::Located { root, lines } = map.source() else {
            panic!("expected a located map");
        };
        let offset = locate_in(root, &segments("/start/0/then/1"), PositionTarget::Key("sya")).unwrap();
        assert_eq!(offset, 52);
        assert_eq!(&document[offset..offset + 3], "sya");
        assert_eq!(lines.position(offset), (5, 9));
    }

    #[test]
    fn keys_and_values_have_distinct_starts_in_flow_collections() {
        let map = build_position_map("title: x\nm: {a: 1, b: x}\ns: [1, two, 3]\n");
        assert_eq!(at(&map, "/m", PositionTarget::Key("b")), Some((2, 11)));
        assert_eq!(at(&map, "/m/b", PositionTarget::Value), Some((2, 14)));
        assert_eq!(at(&map, "/s/1", PositionTarget::Value), Some((3, 8)));
    }

    #[test]
    fn a_multiline_flow_collection_keeps_every_position_in_the_document() {
        let yaml = "title: x\ns: [ # first\n  1, # one, two ]\n  # between\n  two\n]\nm: {\n  a: 1,\n  b: x # end\n}\nafter: y\n";
        let map = build_position_map(yaml);
        assert_eq!(at(&map, "/s/0", PositionTarget::Value), Some((3, 3)));
        assert_eq!(at(&map, "/s/1", PositionTarget::Value), Some((5, 3)));
        assert_eq!(at(&map, "/m", PositionTarget::Key("b")), Some((9, 3)));
        assert_eq!(at(&map, "/m/b", PositionTarget::Value), Some((9, 6)));
        assert_eq!(at(&map, "/after", PositionTarget::Value), Some((11, 8)));
    }

    #[test]
    fn block_list_items_exclude_the_dash() {
        let map = build_position_map("tags:\n  - 1\n  - oops\nitems:\n  - name: a\n    id: 1\n  - name: b\n");
        assert_eq!(at(&map, "/tags/1", PositionTarget::Value), Some((3, 5)));
        assert_eq!(at(&map, "/items/1", PositionTarget::Mapping), Some((7, 5)));
    }

    #[test]
    fn the_root_mapping_starts_at_its_first_key() {
        let map = build_position_map("# comment\n\n# again\ntitle: x\n");
        assert_eq!(at(&map, "", PositionTarget::Mapping), Some((4, 1)));
        assert_eq!(at(&map, "", PositionTarget::Value), Some((4, 1)));
    }

    /// Decoded keys are matched as keys; only a sequence reads an index.
    #[test]
    fn decoded_keys_and_indices_follow_the_node_kind() {
        let yaml = "m:\n  \"0\": a\n  \"a/b\": b\n  \"a~b\": c\n  \"\": d\nl:\n  - p\n";
        let map = build_position_map(yaml);
        assert!(is_located(&map));
        assert_eq!(at(&map, "/m/0", PositionTarget::Value), Some((2, 8)));
        assert_eq!(at(&map, "/m/a~1b", PositionTarget::Value), Some((3, 10)));
        assert_eq!(at(&map, "/m/a~0b", PositionTarget::Value), Some((4, 10)));
        assert_eq!(at(&map, "/m/", PositionTarget::Value), Some((5, 7)));
        assert_eq!(at(&map, "/m", PositionTarget::Key("")), Some((5, 3)));
        assert_eq!(at(&map, "/l/0", PositionTarget::Value), Some((7, 5)));
        // `1` is not a key of `m`, so the nearest authored ancestor is used.
        assert_eq!(at(&map, "/m/1", PositionTarget::Value), Some((2, 3)));
    }

    #[test]
    fn decoded_scalar_descendants_use_the_scalar_start() {
        let map = build_position_map("doc: |\n  {\"a\": {\"b\": 1}}\ncfg: '{\"x\": 1}'\n");
        assert_eq!(at(&map, "/doc", PositionTarget::Value), Some((1, 6)));
        assert_eq!(at(&map, "/doc/a/b", PositionTarget::Value), Some((1, 6)));
        assert_eq!(at(&map, "/doc/a", PositionTarget::Key("zz")), Some((1, 6)));
        assert_eq!(at(&map, "/cfg/x", PositionTarget::Mapping), Some((3, 6)));
    }

    #[test]
    fn unauthored_paths_fall_back_to_the_nearest_authored_non_root_node() {
        let map = build_position_map("title: x\nm:\n  a: 1\n");
        assert_eq!(at(&map, "/m/new/deep", PositionTarget::Value), Some((3, 3)));
        assert_eq!(at(&map, "/m", PositionTarget::Key("new")), Some((3, 3)));
        assert_eq!(at(&map, "/new", PositionTarget::Value), None);
        assert_eq!(at(&map, "/new/deep", PositionTarget::Value), None);
        assert_eq!(at(&map, "", PositionTarget::Key("new")), None);
        assert_eq!(at(&map, "", PositionTarget::Key("title")), Some((1, 1)));
    }

    #[test]
    fn columns_count_utf8_bytes() {
        let map = build_position_map("x: {k: 1, \"😀\": 2}\n");
        assert_eq!(at(&map, "/x", PositionTarget::Key("😀")), Some((1, 11)));
        assert_eq!(at(&map, "/x/😀", PositionTarget::Value), Some((1, 19)));
    }

    /// Unquoted templates have no trusted nested projection; top-level keys
    /// are still located.
    #[test]
    fn an_unquoted_template_keeps_the_top_level_fallback() {
        let map = build_position_map("title: x\nother: {{ value }}\n");
        assert!(!is_located(&map));
        assert_eq!(at(&map, "/other/x", PositionTarget::Value), Some((2, 1)));
    }

    /// An alias is a leaf at its `*name` token, also for a path below it, and
    /// its neighbors keep their own positions.
    #[test]
    fn aliases_and_literal_merge_keys_are_projected() {
        for (yaml, pointer, alias_at, neighbor) in [
            ("base: &b\n  x: 1\nother: *b\n", "/other/x", (3, 8), ("/base/x", (2, 6))),
            ("base: &b\n  x: 1\nother:\n  <<: *b\n  y: 2\n", "/other/<</x", (4, 7), ("/other/y", (5, 6))),
            ("list: &l\n  - 1\nother: *l\n", "/other/0", (3, 8), ("/list/0", (2, 5))),
        ] {
            let map = build_position_map(yaml);
            assert!(is_located(&map), "{yaml:?} must be located");
            assert_eq!(at(&map, pointer, PositionTarget::Value), Some(alias_at), "{yaml:?}");
            assert_eq!(at(&map, neighbor.0, PositionTarget::Value), Some(neighbor.1), "{yaml:?}");
        }
    }

    fn range_text<'a>(map: &PositionMap, yaml: &'a str, pointer: &str) -> &'a str {
        let PositionSource::Located { root, .. } = map.source() else {
            panic!("{yaml:?} must be located");
        };
        &yaml[locate_range_in(root, &segments(pointer), PositionTarget::Value).expect("located")]
    }

    #[test]
    fn a_projected_alias_leaf_spans_only_its_token() {
        let yaml = "base: &b\n  x: 1\none: *b # note\nflow: {k: *b }\n";
        let map = build_position_map(yaml);
        assert_eq!(range_text(&map, yaml, "/one"), "*b");
        assert_eq!(range_text(&map, yaml, "/one/x"), "*b");
        assert_eq!(range_text(&map, yaml, "/flow/k"), "*b");
        assert_eq!(range_text(&map, yaml, "/flow/k/x"), "*b");
    }

    /// A `*` in a comment, a quoted scalar, or a block scalar is not an alias:
    /// it adds no boundary and the document stays projected.
    #[test]
    fn a_trailing_comment_and_a_quoted_or_block_star_are_not_aliases() {
        let yaml = "a: 1 # *b\nq: \"*b\"\nblock: |\n  *b\nm:\n  k: v\n";
        let map = build_position_map(yaml);
        assert!(is_located(&map));
        let PositionSource::Located { root, .. } = map.source() else { unreachable!() };
        let mut root = root.clone();
        let orders = MappingOrders::collect(&serde_yaml_ng::from_str(yaml).unwrap());
        let mut aliases = HashSet::new();
        assert!(matches_orders(&mut root, yaml, &orders, &mut String::new(), &mut 0, &mut aliases));
        assert!(aliases.is_empty(), "{aliases:?}");
        assert_eq!(range_text(&map, yaml, "/a"), "1");
        assert_eq!(range_text(&map, yaml, "/q"), "\"*b\"");
        assert_eq!(at(&map, "/block", PositionTarget::Value), Some((3, 8)));
        assert_eq!(at(&map, "/m/k", PositionTarget::Value), Some((6, 6)));
    }

    /// A boundary covers its own pointer and its descendants, never a sibling
    /// sharing its prefix.
    #[test]
    fn an_alias_boundary_covers_only_whole_segments() {
        let orders = MappingOrders::collect(
            &serde_yaml_ng::from_str("a: {p: {q: 1}}\nab: {r: 1}\n\"a/b\": {s: 1}\n").unwrap(),
        );
        let boundaries = |pointers: &[&str]| pointers.iter().map(|p| p.to_string()).collect::<HashSet<_>>();
        assert_eq!(covered_by_aliases(&orders, &boundaries(&["/a"])), 2);
        assert_eq!(covered_by_aliases(&orders, &boundaries(&["/a/p"])), 1);
        assert_eq!(covered_by_aliases(&orders, &boundaries(&["/ab"])), 1);
        assert_eq!(covered_by_aliases(&orders, &boundaries(&["/a~1b"])), 1);
        assert_eq!(covered_by_aliases(&orders, &boundaries(&[])), 0);
    }

    /// Alias leaves are projected without searching for their anchors, so the
    /// work does not grow with the number of uses.
    #[test]
    fn projecting_many_aliases_searches_no_anchor() {
        let mut yaml = String::from("base: &b {port: 1}\n");
        for index in 0..200 {
            yaml.push_str(&format!("use{index}: *b\n"));
        }
        let before = crate::simplified::yaml_scalar::alias_search_work();
        let map = build_position_map(&yaml);
        assert_eq!(crate::simplified::yaml_scalar::alias_search_work(), before);
        assert!(is_located(&map));
        assert_eq!(at(&map, "/use199/port", PositionTarget::Value), Some((201, 9)));
    }

    #[test]
    fn a_list_root_is_projected() {
        let yaml = "- macOS\n- linux\n";
        let map = build_position_map(yaml);
        let PositionSource::Located { root, .. } = map.source() else {
            panic!("a list root must be located");
        };
        let range = locate_range_in(root, &segments("/0"), PositionTarget::Value).unwrap();
        assert_eq!(range.start, 2);
        assert!(yaml[range].starts_with("macOS"));
        assert_eq!(at(&map, "/1", PositionTarget::Value), Some((2, 3)));
    }

    #[test]
    fn a_scalar_root_is_projected() {
        let yaml = "# note\n42\n";
        let map = build_position_map(yaml);
        assert!(is_located(&map));
        assert_eq!(at(&map, "", PositionTarget::Value), Some((2, 1)));
    }

    #[test]
    fn an_alias_under_a_list_root_is_projected() {
        for (yaml, pointer, token) in [("- &a x\n- *a\n", "/1", "*a"), ("- base: &b\n    x: 1\n- *b\n", "/1/x", "*b")] {
            let value: serde_yaml_ng::Value = serde_yaml_ng::from_str(yaml).unwrap();
            let orders = MappingOrders::collect(&value);
            let map = PositionMap::project(yaml, 0..yaml.len(), 1, &orders).expect("projects");
            assert_eq!(range_text(&map, yaml, pointer), token, "{yaml:?}");
            assert_eq!(at(&map, pointer, PositionTarget::Value), Some((yaml.lines().count() as u32, 3)));
        }
    }

    #[test]
    fn a_mismatched_structure_is_not_projected() {
        let yaml = "m:\n  a: 1\n";
        let mut orders_yaml: serde_yaml_ng::Value = serde_yaml_ng::from_str("m:\n  b: 1\n").unwrap();
        assert!(PositionMap::project(yaml, 0..yaml.len(), 1, &MappingOrders::collect(&orders_yaml)).is_none());
        orders_yaml = serde_yaml_ng::from_str("m: 1\n").unwrap();
        assert!(PositionMap::project(yaml, 0..yaml.len(), 1, &MappingOrders::collect(&orders_yaml)).is_none());
        assert!(PositionMap::project(yaml, 0..yaml.len(), 1, &MappingOrders::default()).is_none());
    }

    #[test]
    fn the_empty_map_has_no_positions() {
        let map = PositionMap::new();
        assert!(map.is_empty());
        assert_eq!(at(&map, "", PositionTarget::Mapping), None);
        assert_eq!(at(&map, "/a", PositionTarget::Value), None);
        assert!(build_position_map("").is_empty());
    }

    /// A `:` inside a plain or quoted key is content, not the separator.
    #[test]
    fn top_level_keys_keep_a_colon_inside_a_key() {
        let yaml = "a:b: one\na:'c: two\n'x: y': three\n\"u:v\": four\n";
        for map in [build_position_map(yaml), PositionMap::top_level(yaml, 1)] {
            assert_eq!(at(&map, "", PositionTarget::Key("a:b")), Some((1, 1)));
            assert_eq!(at(&map, "", PositionTarget::Key("a:'c")), Some((2, 1)));
            assert_eq!(at(&map, "", PositionTarget::Key("x: y")), Some((3, 1)));
            assert_eq!(at(&map, "", PositionTarget::Key("u:v")), Some((4, 1)));
            assert_eq!(at(&map, "", PositionTarget::Key("a")), None);
        }
    }

    #[test]
    fn top_level_fallback_offsets_lines_by_its_base() {
        let map = PositionMap::top_level("title: hi\nbody: stuff\n  nested: skip\n", 2);
        assert_eq!(at(&map, "/title", PositionTarget::Value), Some((2, 1)));
        assert_eq!(at(&map, "/body/nested", PositionTarget::Value), Some((3, 1)));
        assert_eq!(at(&map, "/nested", PositionTarget::Value), None);
        assert_eq!(at(&map, "", PositionTarget::Key("body")), Some((3, 1)));
        assert_eq!(at(&map, "", PositionTarget::Mapping), None);
    }
}
