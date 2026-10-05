//! The persistent-cache reader contract for the load-bearing entry and symbol
//! fields, pinned through `hug --json-full symbols` output (the complete
//! index, so every stored field reaches the report): for each field and each
//! malformed shape, one edit to an entry `hug` wrote is either served (and
//! the field's public value is the stated default) or rejected (and the output
//! equals an uncached run). Every cell restores the written entry, renames
//! a sentinel record and applies exactly one edit; the sentinel in the output
//! shows the entry was served.
//!
//! The accepted record shapes are the schema's own defaults (optional values,
//! default empty collections, an empty declaration text); no entry-level
//! field has a default. See `docs/cache-design.md`.

use std::path::Path;

use serde_json::{Value, json};
use tempfile::TempDir;

use crate::cache_upgrade::{only_entry, read_json, rename_record};
use crate::{PYTHON_FIXTURE, Scratch, json_of, stdout_of};

const FILE: &str = "python.py";
const CACHED_ARGS: [&str; 3] = ["--json-full", "symbols", FILE];
const UNCACHED_ARGS: [&str; 4] = ["--json-full", "symbols", "--no-cache", FILE];
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
/// `ServedEmpty`: the entry is read and lists no symbols (so no sentinel).
enum Outcome {
    Served(Value),
    ServedEmpty,
    Rejected,
}
use Outcome::{Rejected as R, Served as S, ServedEmpty as E};

/// Where a column's field lives.
#[derive(Clone, Copy)]
enum Target {
    /// In the entry record called this (by name).
    Record(&'static str),
    /// At the top level of the entry.
    Entry,
}

struct Column {
    name: &'static str,
    target: Target,
    /// Path from the target to the field.
    path: &'static [&'static str],
    /// Value set before the shape edit, so an optional field Python leaves
    /// empty is populated; `None` keeps the value `hug` wrote.
    control: Option<Value>,
    /// The `Empty` shape's value when it is not the type's empty value.
    empty: Option<Value>,
    /// The field's public value in `hug --json-full symbols` output.
    public: fn(&Value) -> Value,
    /// One outcome per entry of [`SHAPES`], in order.
    outcomes: [Outcome; 9],
}

/// The `indexes[0].symbols` record called `name`.
fn record<'a>(output: &'a Value, name: &str) -> &'a Value {
    let found: Vec<_> = indexed_records(output)
        .iter()
        .filter(|record| record["identity"]["name"] == name)
        .collect();
    assert_eq!(found.len(), 1, "expected one `{name}` record");
    found[0]
}

fn indexed_records(output: &Value) -> &Vec<Value> {
    output["indexes"][0]["symbols"]
        .as_array()
        .expect("indexes[0].symbols")
}

/// Whether the record's modifiers make it static.
fn is_static(record: &Value) -> bool {
    record["modifiers"]
        .as_array()
        .is_some_and(|modifiers| modifiers.iter().any(|modifier| modifier["type"] == "Static"))
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
            target: Target::Record(METHOD),
            path: &["kind"],
            control: None,
            empty: None,
            public: |out| record(out, METHOD)["kind"].clone(),
            outcomes: [S(json!("Method")), R, R, R, R, R, R, R, R],
        },
        Column {
            name: "identity.container_kind",
            target: Target::Record(METHOD),
            path: &["identity", "container_kind"],
            control: None,
            empty: None,
            public: |out| record(out, METHOD)["identity"]["container_kind"].clone(),
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
            target: Target::Record(METHOD),
            path: &["visibility", "level"],
            control: Some(json!("Public")),
            empty: None,
            public: |out| record(out, METHOD)["visibility"]["level"].clone(),
            outcomes: [S(json!("Public")), R, R, R, R, R, R, R, R],
        },
        Column {
            name: "modifiers",
            target: Target::Record(STATIC_METHOD),
            path: &["modifiers"],
            control: None,
            empty: None,
            public: |out| json!(is_static(record(out, STATIC_METHOD))),
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
            target: Target::Record(METHOD),
            path: &["source", "declaration_text"],
            control: None,
            empty: None,
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
            target: Target::Record(METHOD),
            path: &["identity", "keyword"],
            control: None,
            empty: None,
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
            target: Target::Record(METHOD),
            path: &["source", "signature_text"],
            control: None,
            empty: None,
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
            target: Target::Record(METHOD),
            path: &["relations", "implements"],
            control: Some(json!([{ "name": "MatrixTrait" }])),
            empty: None,
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
            target: Target::Record(METHOD),
            path: &["kind_data", "kind_data_type"],
            control: None,
            empty: None,
            public: |out| record(out, METHOD)["kind_data"]["kind_data_type"].clone(),
            outcomes: [S(json!("Function")), R, R, R, R, R, R, R, R],
        },
        Column {
            name: "type data fields",
            target: Target::Record(CLASS),
            path: &["kind_data", "data", "fields"],
            control: None,
            empty: None,
            public: |out| {
                let fields = or_empty_list(&record(out, CLASS)["kind_data"]["data"]["fields"]);
                fields
                    .as_array()
                    .expect("kind_data.data.fields")
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
            target: Target::Record(FIELD),
            path: &["kind_data", "data", "visibility"],
            control: Some(json!("Public")),
            empty: None,
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
            target: Target::Record(FIELD),
            path: &["kind_data", "data", "is_static"],
            control: Some(json!(true)),
            empty: None,
            public: |out| record(out, FIELD)["kind_data"]["data"]["is_static"].clone(),
            outcomes: [S(json!(true)), R, R, R, R, R, R, R, R],
        },
        // Entry-level fields: none has a default; only an empty record
        // list under valid coverage is a servable shape (a file with no
        // records of the covered kinds).
        entry_column("covered_kinds", Some(json!([])), [S(json!(true)), R, R, R, R, R, R, R, R]),
        entry_column("completed_capabilities", None, [S(json!(true)), R, R, R, R, R, R, R, R]),
        entry_column("schema_version", None, [S(json!(true)), R, R, R, R, R, R, R, R]),
        entry_column("fingerprint", None, [S(json!(true)), R, R, R, R, R, R, R, R]),
        entry_column("source_hash", None, [S(json!(true)), R, R, R, R, R, R, R, R]),
        entry_column("diagnostics_fingerprint", None, [S(json!(true)), R, R, R, R, R, R, R, R]),
        entry_column("records", None, [S(json!(true)), R, R, R, R, R, E, R, R]),
    ]
}

/// A top-level entry field whose served public value is the sentinel's
/// presence.
fn entry_column(name: &'static str, empty: Option<Value>, outcomes: [Outcome; 9]) -> Column {
    Column {
        name,
        target: Target::Entry,
        path: match name {
            "covered_kinds" => &["covered_kinds"],
            "completed_capabilities" => &["completed_capabilities"],
            "schema_version" => &["schema_version"],
            "fingerprint" => &["fingerprint"],
            "source_hash" => &["source_hash"],
            "diagnostics_fingerprint" => &["diagnostics_fingerprint"],
            "records" => &["records"],
            other => panic!("no entry column {other}"),
        },
        control: None,
        empty,
        public: |out| json!(has_sentinel(out)),
        outcomes,
    }
}

fn record_mut<'a>(snapshot: &'a mut Value, name: &str) -> &'a mut Value {
    snapshot["records"]
        .as_array_mut()
        .expect("entry records")
        .iter_mut()
        .find(|record| record["identity"]["name"] == name)
        .unwrap_or_else(|| panic!("snapshot has no `{name}` record"))
}

/// The object holding the column's field, and the field's key.
fn parent_mut<'a>(snapshot: &'a mut Value, column: &Column) -> (&'a mut Value, &'static str) {
    let (key, parents) = column.path.split_last().expect("non-empty path");
    let mut parent = match column.target {
        Target::Record(name) => record_mut(snapshot, name),
        Target::Entry => snapshot,
    };
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
            parent[key] = match (&column.empty, value) {
                (Some(empty), _) => empty.clone(),
                (None, Value::Array(_)) => json!([]),
                (None, Value::Object(_)) => json!({}),
                (None, Value::String(_) | Value::Bool(_)) => json!(""),
                (None, other) => panic!("{}: no empty shape for {other}", column.name),
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
    indexed_records(output)
        .iter()
        .any(|record| record["identity"]["name"] == SENTINEL_TO)
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
            Outcome::ServedEmpty => {
                let listed = indexed_records(&output).len();
                if listed != 0 || output == *uncached {
                    failures.push(format!("{cell}: expected an empty served entry"));
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

/// A cache entry records exactly the coverage it holds: a `--json-full`
/// run's complete index stores every kind and capability, and the
/// diagnostics it stores carry the rules fingerprint they were made under. A
/// warm run serves them unchanged.
#[test]
fn the_cache_entry_records_the_coverage_it_holds() {
    const SOURCE: &str = "import os\n\n\ndef unused_import_owner():\n    return 1\n";
    let scratch = Scratch::new(&[("lint_me.py", SOURCE)]);
    let args = ["--json-full", "symbols", "lint_me.py"];
    let uncached = json_of(&scratch.stdout(&["--json-full", "symbols", "--no-cache", "lint_me.py"]));
    let cold = json_of(&scratch.stdout(&args));

    let entry = read_json(&only_entry(scratch.temp.path()));
    assert_eq!(entry["covered_kinds"], json!("all"));
    assert_eq!(
        entry["completed_capabilities"],
        json!([
            "declarations",
            "containment",
            "references",
            "semantics",
            "structured_docs",
            "diagnostics"
        ])
    );
    assert!(entry["diagnostics_fingerprint"].is_string(), "diagnostics carry their fingerprint");
    let stored_rules: Vec<&str> = entry["diagnostics"]
        .as_array()
        .expect("entry diagnostics")
        .iter()
        .filter_map(|diagnostic| diagnostic["rule"].as_str())
        .collect();
    assert_eq!(stored_rules, ["unused-import"]);

    let warm = json_of(&scratch.stdout(&args));
    for (label, output) in [("cold", &cold), ("warm", &warm)] {
        assert_eq!(output, &uncached, "{label} run must match an uncached run");
        let index = &output["indexes"][0];
        assert_eq!(
            index["completed_passes"],
            json!(["Parse", "Bind", "Semantic", "Docs", "Diagnostics"]),
            "{label}"
        );
        let rules: Vec<&str> = index["diagnostics"]
            .as_array()
            .expect("diagnostics")
            .iter()
            .filter_map(|diagnostic| diagnostic["rule"].as_str())
            .collect();
        assert_eq!(rules, ["unused-import"], "{label}");
    }
}
