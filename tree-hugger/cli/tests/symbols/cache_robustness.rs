//! The persistent-cache reader contract for the load-bearing symbol fields,
//! pinned through `hug --json symbols` output: for each field and each
//! malformed shape, one edit to a snapshot `hug` wrote is either served (and
//! the field's public value is the stated default) or rejected (and the output
//! equals an uncached run). Every cell restores the written snapshot, renames
//! a sentinel record and applies exactly one edit; the sentinel in the output
//! shows the entry was served.
//!
//! The accepted shapes are the schema's own defaults (optional values, default
//! empty collections, an empty declaration text); see `docs/cache-design.md`.

use std::path::Path;

use serde_json::{Value, json};
use tempfile::TempDir;

use crate::cache_upgrade::{only_entry, read_json, rename_record};
use crate::{PYTHON_FIXTURE, Scratch, json_of, stdout_of};

const FILE: &str = "python.py";
const CACHED_ARGS: [&str; 3] = ["--json", "symbols", FILE];
const UNCACHED_ARGS: [&str; 4] = ["--json", "symbols", "--no-cache", FILE];
const SENTINEL_FROM: &str = "module_function";
const SENTINEL_TO: &str = "matrix_sentinel_function";
const METHOD: &str = "instance_method";
const STATIC_METHOD: &str = "static_method";
const CLASS: &str = "PlainWidget";
const FIELD: &str = "annotated_instance_field";
const PLACEHOLDER: &str = "__matrix_edit__";

#[derive(Debug, Clone, Copy)]
enum Shape {
    Control,
    Absent,
    Null,
    WrongType,
    OneWrongElement,
    EveryElementWrong,
    Empty,
    DuplicateKey,
    TrailingInvalid,
}

const SHAPES: [Shape; 9] = [
    Shape::Control,
    Shape::Absent,
    Shape::Null,
    Shape::WrongType,
    Shape::OneWrongElement,
    Shape::EveryElementWrong,
    Shape::Empty,
    Shape::DuplicateKey,
    Shape::TrailingInvalid,
];

/// `Served(value)`: the entry is read and the field's public value is `value`.
enum Outcome {
    Served(Value),
    Rejected,
}
use Outcome::{Rejected as R, Served as S};

struct Column {
    name: &'static str,
    /// The snapshot record (by name) holding the field.
    record: &'static str,
    /// Path from the record to the field.
    path: &'static [&'static str],
    /// Value set before the shape edit, so an optional field Python leaves
    /// empty is populated; `None` keeps the value `hug` wrote.
    control: Option<Value>,
    /// The field's public value in `hug --json symbols` output.
    public: fn(&Value) -> Value,
    /// One outcome per entry of [`SHAPES`], in order.
    outcomes: [Outcome; 9],
}

/// The `files[0].symbols` entry called `name` (the `SymbolInfo` view).
fn info<'a>(output: &'a Value, name: &str) -> &'a Value {
    let found: Vec<_> = output["files"][0]["symbols"]
        .as_array()
        .expect("files[0].symbols")
        .iter()
        .filter(|symbol| symbol["name"] == name)
        .collect();
    assert_eq!(found.len(), 1, "expected one `{name}` symbol");
    found[0]
}

/// The `symbol_indexes[0].symbols` record called `name` (the record view).
fn record<'a>(output: &'a Value, name: &str) -> &'a Value {
    let found: Vec<_> = output["symbol_indexes"][0]["symbols"]
        .as_array()
        .expect("symbol_indexes[0].symbols")
        .iter()
        .filter(|record| record["identity"]["name"] == name)
        .collect();
    assert_eq!(found.len(), 1, "expected one `{name}` record");
    found[0]
}

fn or_empty_list(value: &Value) -> Value {
    if value.is_null() {
        json!([])
    } else {
        value.clone()
    }
}

fn columns() -> Vec<Column> {
    let unknown = Value::Null;
    vec![
        Column {
            name: "kind",
            record: METHOD,
            path: &["kind"],
            control: None,
            public: |out| info(out, METHOD)["kind"].clone(),
            outcomes: [S(json!("Method")), R, R, R, R, R, R, R, R],
        },
        Column {
            name: "identity.container_kind",
            record: METHOD,
            path: &["identity", "container_kind"],
            control: None,
            public: |out| info(out, METHOD)["container_kind"].clone(),
            outcomes: [
                S(json!("Class")),
                S(unknown.clone()),
                S(unknown.clone()),
                R,
                R,
                R,
                R,
                R,
                R,
            ],
        },
        Column {
            name: "visibility.level",
            record: METHOD,
            path: &["visibility", "level"],
            control: Some(json!("Public")),
            public: |out| info(out, METHOD)["visibility"].clone(),
            outcomes: [S(json!("Public")), R, R, R, R, R, R, R, R],
        },
        Column {
            name: "modifiers",
            record: STATIC_METHOD,
            path: &["modifiers"],
            control: None,
            public: |out| json!(info(out, STATIC_METHOD)["is_static"] == true),
            outcomes: [
                S(json!(true)),
                S(json!(false)),
                R,
                R,
                R,
                R,
                S(json!(false)),
                R,
                R,
            ],
        },
        Column {
            name: "source.declaration_text",
            record: METHOD,
            path: &["source", "declaration_text"],
            control: None,
            public: |out| record(out, METHOD)["source"]["declaration_text"].clone(),
            outcomes: [
                S(json!("def instance_method(self) -> int:")),
                S(unknown.clone()),
                S(unknown.clone()),
                R,
                R,
                R,
                S(json!("")),
                R,
                R,
            ],
        },
        Column {
            name: "identity.keyword",
            record: METHOD,
            path: &["identity", "keyword"],
            control: None,
            public: |out| record(out, METHOD)["identity"]["keyword"].clone(),
            outcomes: [
                S(json!("def")),
                S(unknown.clone()),
                S(unknown.clone()),
                R,
                R,
                R,
                S(json!("")),
                R,
                R,
            ],
        },
        Column {
            name: "source.signature_text",
            record: METHOD,
            path: &["source", "signature_text"],
            control: None,
            public: |out| record(out, METHOD)["source"]["signature_text"].clone(),
            outcomes: [
                S(json!("def instance_method(self) -> int")),
                S(unknown.clone()),
                S(unknown.clone()),
                R,
                R,
                R,
                S(json!("")),
                R,
                R,
            ],
        },
        Column {
            name: "relations.implements",
            record: METHOD,
            path: &["relations", "implements"],
            control: Some(json!([{ "name": "MatrixTrait" }])),
            public: |out| or_empty_list(&record(out, METHOD)["relations"]["implements"]),
            outcomes: [
                S(json!([{ "name": "MatrixTrait" }])),
                S(json!([])),
                R,
                R,
                R,
                R,
                S(json!([])),
                R,
                R,
            ],
        },
        Column {
            name: "kind_data.kind_data_type",
            record: METHOD,
            path: &["kind_data", "kind_data_type"],
            control: None,
            public: |out| record(out, METHOD)["kind_data"]["kind_data_type"].clone(),
            outcomes: [S(json!("Function")), R, R, R, R, R, R, R, R],
        },
        Column {
            name: "type data fields",
            record: CLASS,
            path: &["kind_data", "data", "fields"],
            control: None,
            public: |out| {
                let fields = or_empty_list(&info(out, CLASS)["type_metadata"]["fields"]);
                fields
                    .as_array()
                    .expect("type_metadata.fields")
                    .iter()
                    .map(|field| field["name"].clone())
                    .collect()
            },
            outcomes: [
                S(json!([
                    "annotated_instance_field",
                    "annotated_instance_field_with_value",
                    "class_var_static_field",
                    "class_var_static_field_with_value",
                    "unannotated_static_field"
                ])),
                S(json!([])),
                R,
                R,
                R,
                R,
                S(json!([])),
                R,
                R,
            ],
        },
        Column {
            name: "field data visibility",
            record: FIELD,
            path: &["kind_data", "data", "visibility"],
            control: Some(json!("Public")),
            public: |out| record(out, FIELD)["kind_data"]["data"]["visibility"].clone(),
            outcomes: [
                S(json!("Public")),
                S(unknown.clone()),
                S(unknown),
                R,
                R,
                R,
                R,
                R,
                R,
            ],
        },
        Column {
            name: "field data is_static",
            record: FIELD,
            path: &["kind_data", "data", "is_static"],
            control: Some(json!(true)),
            public: |out| record(out, FIELD)["kind_data"]["data"]["is_static"].clone(),
            outcomes: [S(json!(true)), R, R, R, R, R, R, R, R],
        },
    ]
}

fn record_mut<'a>(snapshot: &'a mut Value, name: &str) -> &'a mut Value {
    snapshot["symbols"]
        .as_array_mut()
        .expect("snapshot symbols")
        .iter_mut()
        .find(|record| record["identity"]["name"] == name)
        .unwrap_or_else(|| panic!("snapshot has no `{name}` record"))
}

/// The object holding the column's field, and the field's key.
fn parent_mut<'a>(snapshot: &'a mut Value, column: &Column) -> (&'a mut Value, &'static str) {
    let (key, parents) = column.path.split_last().expect("non-empty path");
    let mut parent = record_mut(snapshot, column.record);
    for segment in parents {
        parent = &mut parent[*segment];
    }
    (parent, key)
}

/// The column's base snapshot (sentinel renamed, control value set) and
/// the field's value in it.
fn column_base(original: &Value, column: &Column) -> (Value, Value) {
    let mut base = original.clone();
    rename_record(&mut base, SENTINEL_FROM, SENTINEL_TO);
    let (parent, key) = parent_mut(&mut base, column);
    if let Some(control) = &column.control {
        parent[key] = control.clone();
    }
    let value = parent
        .get(key)
        .cloned()
        .unwrap_or_else(|| panic!("{}: the snapshot has no value to edit", column.name));
    (base, value)
}

/// The entry text for one cell: exactly one edit to `base`.
fn edited_entry(base: &Value, column: &Column, shape: Shape, value: &Value) -> String {
    let mut edited = base.clone();
    let (parent, key) = parent_mut(&mut edited, column);
    let elements = match value {
        Value::Array(items) => items.clone(),
        scalar => vec![scalar.clone()],
    };
    match shape {
        Shape::Control => {}
        Shape::Absent => {
            parent
                .as_object_mut()
                .expect("field parent object")
                .remove(key);
        }
        Shape::Null => parent[key] = Value::Null,
        Shape::WrongType => parent[key] = json!(123),
        Shape::OneWrongElement => {
            let mut items = elements;
            items.push(json!(123));
            parent[key] = Value::Array(items);
        }
        Shape::EveryElementWrong => {
            parent[key] = Value::Array(vec![json!(123); elements.len().max(1)]);
        }
        Shape::Empty => {
            parent[key] = match value {
                Value::Array(_) => json!([]),
                Value::String(_) | Value::Bool(_) => json!(""),
                other => panic!("{}: no empty shape for {other}", column.name),
            };
        }
        // `Value` cannot hold a duplicate key or invalid JSON, so these
        // edits are made on the serialized text at a placeholder.
        Shape::DuplicateKey | Shape::TrailingInvalid => {
            parent[key] = json!(PLACEHOLDER);
            let text = serde_json::to_string(&edited).expect("serialize");
            let marker = format!("\"{key}\":\"{PLACEHOLDER}\"");
            assert_eq!(
                text.matches(&marker).count(),
                1,
                "{}: one placeholder",
                column.name
            );
            let field = format!("\"{key}\":{value}");
            let replacement = match shape {
                Shape::DuplicateKey => format!("{field},{field}"),
                _ => format!("{field} trailing"),
            };
            return text.replacen(&marker, &replacement, 1);
        }
    }
    serde_json::to_string(&edited).expect("serialize")
}

fn has_sentinel(output: &Value) -> bool {
    output["files"][0]["symbols"]
        .as_array()
        .expect("files[0].symbols")
        .iter()
        .any(|symbol| symbol["name"] == SENTINEL_TO)
}

#[test]
fn persisted_symbol_fields_are_served_or_recomputed_per_shape() {
    let scratch = Scratch::new(&[(FILE, PYTHON_FIXTURE)]);
    let uncached = json_of(&scratch.stdout(&UNCACHED_ARGS));
    assert_eq!(
        json_of(&scratch.stdout(&CACHED_ARGS)),
        uncached,
        "the cache-writing run must match an uncached run"
    );
    let entry = only_entry(scratch.temp.path());
    let relative_entry = entry
        .strip_prefix(scratch.temp.path())
        .expect("entry under the scratch temp directory")
        .to_path_buf();
    let original = read_json(&entry);

    // Columns run concurrently, each against its own copy of the cache
    // directory; cells within a column run in order.
    let columns = columns();
    let failures: Vec<String> = std::thread::scope(|scope| {
        let handles: Vec<_> = columns
            .iter()
            .map(|column| {
                let (scratch, original, uncached) = (&scratch, &original, &uncached);
                let relative_entry = relative_entry.as_path();
                scope.spawn(move || {
                    check_column(scratch, original, uncached, relative_entry, column)
                })
            })
            .collect();
        handles
            .into_iter()
            .flat_map(|handle| handle.join().expect("column thread"))
            .collect()
    });
    assert!(
        failures.is_empty(),
        "cache reader matrix:\n{}",
        failures.join("\n")
    );
}

/// Runs every cell of `column`, returning one message per cell whose outcome
/// differs from the table.
fn check_column(
    scratch: &Scratch,
    original: &Value,
    uncached: &Value,
    relative_entry: &Path,
    column: &Column,
) -> Vec<String> {
    let temp = TempDir::new().expect("column temp");
    let entry = temp.path().join(relative_entry);
    std::fs::create_dir_all(entry.parent().expect("entry parent")).expect("create shard");
    let (base, value) = column_base(original, column);
    let mut failures = Vec::new();
    for (shape, outcome) in SHAPES.iter().zip(&column.outcomes) {
        std::fs::write(&entry, edited_entry(&base, column, *shape, &value))
            .expect("write cache entry");
        let output = json_of(&stdout_of(
            scratch.hug_with_temp(temp.path(), &CACHED_ARGS),
            &CACHED_ARGS,
        ));
        let cell = format!("{} / {shape:?}", column.name);
        match outcome {
            Outcome::Served(expected) => {
                if !has_sentinel(&output) {
                    failures.push(format!("{cell}: expected served, entry was recomputed"));
                } else if (column.public)(&output) != *expected {
                    failures.push(format!(
                        "{cell}: served value {} != expected {expected}",
                        (column.public)(&output)
                    ));
                }
            }
            Outcome::Rejected => {
                if has_sentinel(&output) {
                    failures.push(format!("{cell}: expected rejected, entry was served"));
                } else if output != *uncached {
                    failures.push(format!(
                        "{cell}: rejected, but output differs from an uncached run"
                    ));
                }
            }
        }
    }
    failures
}
