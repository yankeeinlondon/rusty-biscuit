//! Authored source positions for frontmatter validation problems.
//!
//! A [`PositionMap`] is built once per validation source snapshot and shared
//! by every problem that snapshot produces, including each root-union arm.
//! Lookup is passive: it never parses, evaluates, or resolves anything.

use std::ops::Range;

use biscuit_file::serde_yaml_ng;
use indexmap::IndexMap;

use super::simplified::{
    SchemaValueKind, SchemaValueNode, locate_frontmatter_value, mapping_separator,
    yaml_scalar::alias_token,
};
use crate::markdown::MappingOrders;

/// Source coordinates for one frontmatter snapshot, used to annotate
/// [`ValidationProblem`](super::ValidationProblem)s with a 1-based line and a
/// 1-based UTF-8 byte column.
///
/// When the authored YAML can be projected onto its parsed structure, every
/// key, value, and mapping has its own start, so a problem points at the
/// offending node. Otherwise only top-level keys are located, at column 1.
/// [`PositionMap::new`] is the empty map for source-free validation: its
/// problems carry no coordinates.
#[derive(Debug, Clone, Default)]
pub struct PositionMap {
    source: PositionSource,
}

#[derive(Debug, Clone, Default)]
enum PositionSource {
    #[default]
    Empty,
    /// An authored tree whose mappings match the parsed document's.
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

    /// `true` when no position can be reported.
    pub fn is_empty(&self) -> bool {
        match &self.source {
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
    /// `None` when the locator cannot read the YAML, its root is not a
    /// mapping, or what it read does not match `orders` mapping for mapping or
    /// contains an alias: the parsed value then has a structure the source
    /// cannot be trusted to show.
    pub(super) fn project(
        text: &str,
        yaml: Range<usize>,
        base_line: u32,
        orders: &MappingOrders,
    ) -> Option<Self> {
        let root = locate_frontmatter_value(&text[yaml.clone()], yaml.start)?;
        if !matches!(root.kind, SchemaValueKind::Mapping(_)) {
            return None;
        }
        let mut mappings = 0;
        let mut pointer = String::new();
        if !matches_orders(&root, text, orders, &mut pointer, &mut mappings)
            || mappings != orders.len()
        {
            return None;
        }
        Some(Self {
            source: PositionSource::Located {
                root,
                lines: LineIndex::new(&text[..yaml.end], base_line),
            },
        })
    }

    /// Locates only the top-level keys of `yaml`, whose first line is
    /// `base_line`.
    pub(super) fn top_level(yaml: &str, base_line: u32) -> Self {
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
        Self {
            source: PositionSource::TopLevel(keys),
        }
    }

    /// The `(line, column)` of `target` at the decoded instance pointer
    /// `segments`.
    ///
    /// A path that leaves the authored tree falls back to the deepest
    /// authored non-root node on it, and a newly supplied top-level property
    /// has no position. A path through a scalar — content decoded from a
    /// block or quoted scalar — stops at that scalar's start.
    pub(super) fn locate(&self, segments: &[String], target: PositionTarget<'_>) -> Option<(u32, u32)> {
        match &self.source {
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
}

/// The authored start selected for `target` at `segments` below `root`.
fn locate_in(root: &SchemaValueNode, segments: &[String], target: PositionTarget<'_>) -> Option<usize> {
    let mut node = root;
    let mut deepest_non_root = None;
    for segment in segments {
        let child = match &node.kind {
            SchemaValueKind::Scalar => return Some(node.span.start),
            SchemaValueKind::Mapping(entries) => entries
                .iter()
                .find(|entry| entry.key == *segment)
                .map(|entry| &entry.value),
            SchemaValueKind::Sequence(items) => {
                segment.parse::<usize>().ok().and_then(|index| items.get(index))
            }
        };
        match child {
            Some(child) => {
                node = child;
                deepest_non_root = Some(child.span.start);
            }
            None => return deepest_non_root,
        }
    }
    match (target, &node.kind) {
        (PositionTarget::Key(key), SchemaValueKind::Mapping(entries)) => entries
            .iter()
            .find(|entry| entry.key == key)
            .map(|entry| entry.key_span.start)
            .or(deepest_non_root),
        (PositionTarget::Key(_), SchemaValueKind::Scalar) => Some(node.span.start),
        (PositionTarget::Key(_), SchemaValueKind::Sequence(_)) => deepest_non_root,
        (PositionTarget::Value | PositionTarget::Mapping, _) => Some(node.span.start),
    }
}

/// Whether every mapping in `node`'s subtree carries exactly the keys
/// `orders` records at its pointer, in order, and no scalar is an alias.
/// Counts the mappings visited into `mappings`.
fn matches_orders(
    node: &SchemaValueNode,
    text: &str,
    orders: &MappingOrders,
    pointer: &mut String,
    mappings: &mut usize,
) -> bool {
    match &node.kind {
        SchemaValueKind::Scalar => alias_token(text, node.span.start).is_none(),
        SchemaValueKind::Mapping(entries) => {
            *mappings += 1;
            let Some(keys) = orders.get(pointer) else {
                return false;
            };
            if keys.len() != entries.len()
                || keys.iter().zip(entries).any(|(key, entry)| *key != entry.key)
            {
                return false;
            }
            entries.iter().all(|entry| {
                with_child(pointer, &entry.key, |pointer| {
                    matches_orders(&entry.value, text, orders, pointer, mappings)
                })
            })
        }
        SchemaValueKind::Sequence(items) => items.iter().enumerate().all(|(index, item)| {
            with_child(pointer, &index.to_string(), |pointer| {
                matches_orders(item, text, orders, pointer, mappings)
            })
        }),
    }
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
/// are located, at column 1. Quoted keys (`"foo"`, `'foo'`) are located by
/// their decoded key string.
pub fn build_position_map(yaml: &str) -> PositionMap {
    serde_yaml_ng::from_str::<serde_yaml_ng::Value>(yaml)
        .ok()
        .and_then(|value| {
            PositionMap::project(yaml, 0..yaml.len(), 1, &MappingOrders::collect(&value))
        })
        .unwrap_or_else(|| PositionMap::top_level(yaml, 1))
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
        matches!(map.source, PositionSource::Located { .. })
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
        let block = crate::markdown::extract_frontmatter_block(document).unwrap().unwrap();
        let orders = MappingOrders::collect(&serde_yaml_ng::from_str(block.yaml).unwrap());
        let map = PositionMap::project(document, block.yaml_span.clone(), 1, &orders).unwrap();
        let PositionSource::Located { root, lines } = &map.source else {
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

    /// Aliases, merge keys, and unquoted templates have no trusted nested
    /// projection; top-level keys are still located.
    #[test]
    fn unsupported_yaml_keeps_the_top_level_fallback() {
        for (yaml, other_line) in [
            ("base: &b\n  x: 1\nother: *b\n", 3),
            ("base: &b\n  x: 1\nother:\n  <<: *b\n  y: 2\n", 3),
            ("title: x\nother: {{ value }}\n", 2),
            ("list: &l\n  - 1\nother: *l\n", 3),
        ] {
            let map = build_position_map(yaml);
            assert!(!is_located(&map), "{yaml:?} must not be located");
            assert_eq!(at(&map, "/other/x", PositionTarget::Value), Some((other_line, 1)), "{yaml:?}");
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
