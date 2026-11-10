//! Where a contract violation of a loaded document was written.
//!
//! A [`SourceLocator`] answers two questions about a value path of the
//! parsed document, from the BOM-stripped text a [`DataError::ContractMismatch`]
//! keeps: the 1-based line and character column the value starts at, and its
//! place in file order. Positions come only from the format's own locator
//! (`biscuit_file::data_locations`, simplified-schema's `PositionMap` for
//! YAML, the delimited reader's field offsets for CSV/TSV); a path a locator
//! cannot prove has no position, never a guessed one. For YAML that map
//! places every value whose authored tree it can prove, including an alias
//! (at its `*name` token, for the alias and everything below it) and a flow
//! collection written across lines; a file whose layout it cannot prove is
//! reported by pointer alone.
//!
//! [`DataError::ContractMismatch`]: super::DataError::ContractMismatch

use std::collections::HashMap;
use std::path::Path;

use biscuit_file::{DataFormat, source_position};
use serde_json::Value;
use simplified_schema::PositionMap;
use simplified_schema::coercion::{PathSegment, json_pointer};

use super::contract::RowMode;
use super::error::TabularSource;
use crate::markdown::schemas::yaml_documents::{load_yaml_document, locating_problem};

/// One step of a value's place in file order, compared level by level.
///
/// A value written in the file sorts by where it was written; one whose
/// position the format cannot prove follows its located siblings; a missing
/// member or slot, which has no source order, follows both.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum OrderStep {
    /// A source offset, or for YAML the authored index within its parent.
    Placed(usize),
    Unplaced,
    Missing,
}

/// Where a violating value is, and where it sorts.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ViolationPlace {
    /// 1-based line and character column of the value, or of the nearest
    /// existing parent for a missing value; `None` when not provable.
    pub(crate) position: Option<(usize, usize)>,
    /// One step per path segment, outermost first.
    pub(crate) order: Vec<OrderStep>,
}

/// Locates value paths in one loaded document.
pub(crate) struct SourceLocator<'a> {
    text: &'a str,
    format: Format<'a>,
    offsets: HashMap<Vec<PathSegment>, Option<usize>>,
}

enum Format<'a> {
    /// A JSON Pointer locator over the text.
    Pointer(fn(&str, &str) -> Option<usize>),
    /// YAML: the shared position map (empty when the file did not load),
    /// and the parsed value in authored key order, which orders the file
    /// even when it has no positions.
    Yaml { positions: PositionMap, value: Option<Value> },
    Table(&'a TabularSource),
    Unlocated,
}

impl<'a> SourceLocator<'a> {
    /// A locator for `text` parsed as `format`; `table` is the CSV/TSV table
    /// the rows were built from.
    pub(crate) fn new(format: DataFormat, text: &'a str, table: Option<&'a TabularSource>) -> Self {
        let format = match (format, table) {
            (DataFormat::Csv | DataFormat::Tsv, Some(table)) => Format::Table(table),
            (DataFormat::Json, _) => Format::Pointer(biscuit_file::locate_json),
            (DataFormat::Jsonl, _) => Format::Pointer(biscuit_file::locate_jsonl),
            (DataFormat::Toml, _) => Format::Pointer(biscuit_file::locate_toml),
            (DataFormat::Json5, _) => Format::Pointer(biscuit_file::locate_json5),
            (DataFormat::Yaml, _) => match load_yaml_document(text, Path::new("data.yaml")) {
                // `byte_range` answers only from a projected tree, never from
                // the top-level-keys fallback, whose columns are guesses.
                Ok(loaded) => Format::Yaml { positions: loaded.positions, value: Some(loaded.json) },
                Err(_) => Format::Yaml { positions: PositionMap::new(), value: None },
            },
            _ => Format::Unlocated,
        };
        Self { text, format, offsets: HashMap::new() }
    }

    /// The place of the value at `path`. A `missing` value keeps its own
    /// path for ordering but is positioned at its parent.
    pub(crate) fn place(&mut self, path: &[PathSegment], missing: bool) -> ViolationPlace {
        let located = if missing { &path[..path.len().saturating_sub(1)] } else { path };
        let position = self.offset(located).and_then(|offset| source_position(self.text, offset));
        let order = (1..=path.len())
            .map(|depth| if missing && depth == path.len() { OrderStep::Missing } else { self.order_step(&path[..depth]) })
            .collect();
        ViolationPlace { position, order }
    }

    fn order_step(&mut self, path: &[PathSegment]) -> OrderStep {
        let placed = match &self.format {
            Format::Yaml { value, .. } => value.as_ref().and_then(|value| authored_index(value, path)),
            _ => self.offset(path),
        };
        placed.map_or(OrderStep::Unplaced, OrderStep::Placed)
    }

    /// The byte offset in the text at which the value at `path` starts.
    fn offset(&mut self, path: &[PathSegment]) -> Option<usize> {
        if let Some(known) = self.offsets.get(path) {
            return *known;
        }
        let found = match &self.format {
            Format::Pointer(locate) => locate(self.text, &json_pointer(path)),
            Format::Yaml { positions, .. } => {
                let segments: Vec<String> = path.iter().map(segment_text).collect();
                positions.byte_range(&locating_problem(&segments)).map(|range| range.start)
            }
            Format::Table(table) => table_offset(table, path),
            Format::Unlocated => None,
        };
        self.offsets.insert(path.to_vec(), found);
        found
    }
}

fn segment_text(segment: &PathSegment) -> String {
    match segment {
        PathSegment::Index(index) => index.to_string(),
        PathSegment::Key(key) => key.clone(),
    }
}

/// The index of the last segment of `path` among its parent's entries, in
/// the order the parsed value keeps (authored order for YAML).
fn authored_index(root: &Value, path: &[PathSegment]) -> Option<usize> {
    let (last, parents) = path.split_last()?;
    let mut parent = root;
    for segment in parents {
        parent = match (parent, segment) {
            (Value::Object(members), PathSegment::Key(key)) => members.get(key)?,
            (Value::Array(items), PathSegment::Index(index)) => items.get(*index)?,
            _ => return None,
        };
    }
    match (parent, last) {
        (Value::Object(members), PathSegment::Key(key)) => members.keys().position(|name| name == key),
        (Value::Array(items), PathSegment::Index(index)) => (*index < items.len()).then_some(*index),
        _ => None,
    }
}

/// A row is [`TabularSource::row`] `i` (a header is never a row; without one
/// the first record is row 0), and a cell is named by its header in object
/// mode and by its index in list mode. The root is the first record. Nothing
/// below a cell has a position of its own.
fn table_offset(table: &TabularSource, path: &[PathSegment]) -> Option<usize> {
    let data = &table.data;
    let row = match path.first() {
        None => return Some(data.headers.as_ref().map_or(0, |header| header.offset)),
        Some(PathSegment::Index(index)) => table.row(*index)?,
        Some(PathSegment::Key(_)) => return None,
    };
    let column = match (path.get(1), table.mode) {
        (None, _) => return Some(row.offset),
        (Some(PathSegment::Key(name)), RowMode::Object) => {
            data.headers.as_ref()?.fields.iter().position(|field| field.text == *name)?
        }
        (Some(PathSegment::Index(index)), RowMode::List) => *index,
        _ => return None,
    };
    if path.len() > 2 {
        return None;
    }
    row.fields.get(column).map(|field| field.offset)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    fn key(name: &str) -> PathSegment {
        PathSegment::Key(name.to_string())
    }

    #[test]
    fn a_missing_member_is_positioned_at_its_parent_and_sorts_after_its_siblings() {
        let text = "[\n  {\"name\": \"a\"},\n  {\"name\": 1}\n]";
        let mut locator = SourceLocator::new(DataFormat::Json, text, None);
        let missing = locator.place(&[PathSegment::Index(1), key("port")], true);
        let present = locator.place(&[PathSegment::Index(1), key("name")], false);
        assert_eq!(missing.position, Some((3, 3)));
        assert_eq!(present.position, Some((3, 12)));
        assert!(present.order < missing.order);
    }

    #[test]
    fn yaml_orders_by_authored_index_and_places_an_alias() {
        let text = "base: &b {x: 1}\nzeta: *b\nalpha: 2\n";
        let mut locator = SourceLocator::new(DataFormat::Yaml, text, None);
        let zeta = locator.place(&[key("zeta")], false);
        let alpha = locator.place(&[key("alpha")], false);
        assert_eq!((zeta.position, alpha.position), (Some((2, 7)), Some((3, 8))));
        assert!(zeta.order < alpha.order);
    }

    #[test]
    fn a_multi_line_flow_collection_keeps_yaml_positions() {
        let mut locator = SourceLocator::new(DataFormat::Yaml, "items: [\n  x\n]\n", None);
        assert_eq!(locator.place(&[key("items"), PathSegment::Index(0)], false).position, Some((2, 3)));
    }

    #[test]
    fn an_unproven_yaml_layout_is_unpositioned_but_keeps_authored_order() {
        let mut locator = SourceLocator::new(DataFormat::Yaml, "&k a: 1\nb: 2\n", None);
        let first = locator.place(&[key("a")], false);
        let second = locator.place(&[key("b")], false);
        assert_eq!((first.position, second.position), (None, None));
        assert_eq!((first.order, second.order), (vec![OrderStep::Placed(0)], vec![OrderStep::Placed(1)]));
    }

    #[test]
    fn a_list_mode_cell_is_its_raw_field_offset() {
        let text = "a,b\n1,\"x\"\n";
        let data = biscuit_file::read_delimited(text, b',').expect("reads");
        let table = TabularSource { mode: RowMode::List, header: true, data: Arc::new(data) };
        let mut locator = SourceLocator::new(DataFormat::Csv, text, Some(&table));
        assert_eq!(locator.place(&[PathSegment::Index(0), PathSegment::Index(1)], false).position, Some((2, 3)));
        assert_eq!(locator.place(&[PathSegment::Index(0), PathSegment::Index(2)], true).position, Some((2, 1)));
    }

    #[test]
    fn headerless_root_and_rows_use_the_logical_view() {
        let text = "a,b\n1,2\n";
        let data = Arc::new(biscuit_file::read_delimited(text, b',').expect("reads"));
        let at = |table: &TabularSource, path: &[PathSegment]| {
            SourceLocator::new(DataFormat::Csv, text, Some(table)).place(path, false).position
        };
        let cell = |row, column| [PathSegment::Index(row), PathSegment::Index(column)];
        let headerless = TabularSource { mode: RowMode::List, header: false, data: data.clone() };
        assert_eq!(at(&headerless, &[]), Some((1, 1)), "the root is the first record");
        assert_eq!(at(&headerless, &cell(0, 1)), Some((1, 3)), "/0/1 is the first record's second cell");
        assert_eq!(at(&headerless, &cell(1, 0)), Some((2, 1)));
        assert_eq!(at(&headerless, &cell(2, 0)), None);

        let headed = TabularSource { mode: RowMode::List, header: true, data };
        assert_eq!(at(&headed, &[]), Some((1, 1)));
        assert_eq!(at(&headed, &cell(0, 1)), Some((2, 3)), "a header is skipped");
        assert_eq!(at(&headed, &cell(1, 0)), None);
    }
}
