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
//! field has a default. A `schema_version`, on the entry or on a record, is
//! served only when it is exactly the current version. See
//! `docs/cache-design.md`.

use std::path::Path;

use serde_json::{Value, json};
use tempfile::TempDir;

use crate::cache_upgrade::{only_entry, read_json, rename_record};
use crate::{JAVA_FIXTURE, PYTHON_FIXTURE, Scratch, json_of, stdout_of};

const FILE: &str = "python.py";
const CACHED_ARGS: [&str; 3] = ["--json-full", "symbols", FILE];
const UNCACHED_ARGS: [&str; 4] = ["--json-full", "symbols", "--no-cache", FILE];
const SENTINEL_FROM: &str = "module_function";
const SENTINEL_TO: &str = "matrix_sentinel_function";
const GENERIC: &str = "MatrixGeneric";
const METHOD: &str = "instance_method";
const STATIC_METHOD: &str = "static_method";
const CLASS: &str = "PlainWidget";
const FIELD: &str = "annotated_instance_field";
const ASYNC_FUNCTION: &str = "async_function";
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
    /// A well-formed version whose major differs from the current one.
    IncompatibleMajor,
    /// A well-formed version with the current major and another minor.
    IncompatibleMinor,
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

/// Extra shapes for a `schema_version` column. The cache serves only the
/// current version, exactly, so every one of these cells is rejected.
const VERSION_SHAPES: [Shape; 2] = [Shape::IncompatibleMajor, Shape::IncompatibleMinor];

/// `Served(value)`: the entry is read and the field's public value is `value`.
/// `Repeated`: the cell is not run, because on this column it repeats a cell
/// another column already runs (a duplicate key or trailing content is a
/// document-level error wherever it is written, and a list around a scalar
/// leaf is its wrong type).
enum Outcome {
    Served(Value),
    Rejected,
    Repeated,
}
use Outcome::{Rejected as R, Repeated as X, Served as S};

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

/// Whether the record's modifiers make it async.
fn is_async(record: &Value) -> bool {
    record["modifiers"]
        .as_array()
        .is_some_and(|modifiers| modifiers.iter().any(|modifier| modifier["type"] == "Async"))
}

/// Whether the record's modifiers make it static.
fn is_static(record: &Value) -> bool {
    record["modifiers"]
        .as_array()
        .is_some_and(|modifiers| modifiers.iter().any(|modifier| modifier["type"] == "Static"))
}

/// The names of the record's attributes, in order.
fn attribute_names(record: &Value) -> Value {
    Value::from(
        record["attributes"]
            .as_array()
            .map(|attributes| attributes.iter().map(|attribute| attribute["name"].clone()).collect::<Vec<_>>())
            .unwrap_or_default(),
    )
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
            name: "type_info.generic_params",
            target: Target::Record(GENERIC),
            path: &["type_info", "generic_params"],
            control: None,
            empty: None,
            public: |out| or_empty_list(&record(out, GENERIC)["type_info"]["generic_params"]),
            outcomes: [S(json!([{ "name": "T", "kind": "Type", "constraints": ["MatrixBase"], "default": "MatrixBase", "variance": "out", "position": 0 }])), S(json!([])), R, R, R, R, S(json!([])), R, R],
        },
        Column {
            name: "type_info.constraints",
            target: Target::Record(GENERIC),
            path: &["type_info", "constraints"],
            control: Some(json!(["T.Item: Copy"])),
            empty: None,
            public: |out| or_empty_list(&record(out, GENERIC)["type_info"]["constraints"]),
            outcomes: [S(json!(["T.Item: Copy"])), S(json!([])), R, R, R, R, S(json!([])), R, R],
        },
        Column {
            name: "relations.extends",
            target: Target::Record(GENERIC),
            path: &["relations", "extends"],
            control: Some(json!([{ "name": "MatrixBase", "id": "matrix-id", "qualified_name": "MatrixBase" }])),
            empty: None,
            public: |out| or_empty_list(&record(out, GENERIC)["relations"]["extends"]),
            outcomes: [S(json!([{ "name": "MatrixBase", "id": "matrix-id", "qualified_name": "MatrixBase" }])), S(json!([])), R, R, R, R, S(json!([])), R, R],
        },
        Column {
            name: "relations.unclassified_bases",
            target: Target::Record(GENERIC),
            path: &["relations", "unclassified_bases"],
            control: Some(json!([{ "name": "External" }])),
            empty: None,
            public: |out| or_empty_list(&record(out, GENERIC)["relations"]["unclassified_bases"]),
            outcomes: [S(json!([{ "name": "External" }])), S(json!([])), R, R, R, R, S(json!([])), R, R],
        },
        Column {
            name: "generic name",
            target: Target::Record(GENERIC),
            path: &["type_info", "generic_params", "0", "name"],
            control: None,
            empty: Some(json!("")),
            public: |out| record(out, GENERIC)["type_info"]["generic_params"][0]["name"].clone(),
            outcomes: [S(json!("T")), R, R, R, R, R, S(json!("")), R, R],
        },
        Column {
            name: "generic kind",
            target: Target::Record(GENERIC),
            path: &["type_info", "generic_params", "0", "kind"],
            control: None,
            empty: None,
            public: |out| record(out, GENERIC)["type_info"]["generic_params"][0]["kind"].clone(),
            outcomes: [S(json!("Type")), R, R, R, R, R, R, R, R],
        },
        Column {
            name: "generic constraints",
            target: Target::Record(GENERIC),
            path: &["type_info", "generic_params", "0", "constraints"],
            control: None,
            empty: Some(json!([])),
            public: |out| or_empty_list(&record(out, GENERIC)["type_info"]["generic_params"][0]["constraints"]),
            outcomes: [S(json!(["MatrixBase"])), S(json!([])), R, R, R, R, S(json!([])), R, R],
        },
        Column {
            name: "generic default",
            target: Target::Record(GENERIC),
            path: &["type_info", "generic_params", "0", "default"],
            control: None,
            empty: Some(json!("")),
            public: |out| record(out, GENERIC)["type_info"]["generic_params"][0]["default"].clone(),
            outcomes: [S(json!("MatrixBase")), S(Value::Null), S(Value::Null), R, R, R, S(json!("")), R, R],
        },
        Column {
            name: "generic variance",
            target: Target::Record(GENERIC),
            path: &["type_info", "generic_params", "0", "variance"],
            control: None,
            empty: Some(json!("")),
            public: |out| record(out, GENERIC)["type_info"]["generic_params"][0]["variance"].clone(),
            outcomes: [S(json!("out")), S(Value::Null), S(Value::Null), R, R, R, S(json!("")), R, R],
        },
        Column {
            name: "generic position",
            target: Target::Record(GENERIC),
            path: &["type_info", "generic_params", "0", "position"],
            control: None,
            empty: Some(json!(0)),
            public: |out| record(out, GENERIC)["type_info"]["generic_params"][0]["position"].clone(),
            outcomes: [S(json!(0)), R, R, R, R, R, S(json!(0)), R, R],
        },
        Column {
            name: "target name",
            target: Target::Record(GENERIC),
            path: &["relations", "extends", "0", "name"],
            control: Some(json!("MatrixBase")),
            empty: Some(json!("")),
            public: |out| record(out, GENERIC)["relations"]["extends"][0]["name"].clone(),
            outcomes: [S(json!("MatrixBase")), R, R, R, R, R, S(json!("")), R, R],
        },
        Column {
            name: "target id",
            target: Target::Record(GENERIC),
            path: &["relations", "extends", "0", "id"],
            control: Some(json!("matrix-id")),
            empty: Some(json!("")),
            public: |out| record(out, GENERIC)["relations"]["extends"][0]["id"].clone(),
            outcomes: [S(json!("matrix-id")), S(Value::Null), S(Value::Null), R, R, R, S(json!("")), R, R],
        },
        Column {
            name: "target qualified_name",
            target: Target::Record(GENERIC),
            path: &["relations", "extends", "0", "qualified_name"],
            control: Some(json!("MatrixBase")),
            empty: Some(json!("")),
            public: |out| record(out, GENERIC)["relations"]["extends"][0]["qualified_name"].clone(),
            outcomes: [S(json!("MatrixBase")), S(Value::Null), S(Value::Null), R, R, R, S(json!("")), R, R],
        },
        Column {
            name: "target source_span",
            target: Target::Record(GENERIC),
            path: &["relations", "extends", "0", "source_span"],
            control: Some(json!({ "start": { "line": 1, "column": 1 }, "end": { "line": 1, "column": 2 }, "start_byte": 0, "end_byte": 1 })),
            empty: None,
            public: |out| record(out, GENERIC)["relations"]["extends"][0]["source_span"].clone(),
            outcomes: [S(json!({ "start": { "line": 1, "column": 1 }, "end": { "line": 1, "column": 2 }, "start_byte": 0, "end_byte": 1 })), S(Value::Null), S(Value::Null), R, R, R, R, R, R],
        },
        // Every span member is required; `0` is served as written (the
        // decoder reads an unsigned integer, not a 1-based position). The
        // byte members decide the Swift `--signatures full` outline label.
        span_member_column("target source_span.start.line", &["relations", "extends", "0", "source_span", "start", "line"], 2, |out| target_span(out)["start"]["line"].clone()),
        span_member_column("target source_span.start.column", &["relations", "extends", "0", "source_span", "start", "column"], 3, |out| target_span(out)["start"]["column"].clone()),
        span_member_column("target source_span.end.line", &["relations", "extends", "0", "source_span", "end", "line"], 4, |out| target_span(out)["end"]["line"].clone()),
        span_member_column("target source_span.end.column", &["relations", "extends", "0", "source_span", "end", "column"], 5, |out| target_span(out)["end"]["column"].clone()),
        span_member_column("target source_span.start_byte", &["relations", "extends", "0", "source_span", "start_byte"], 6, |out| target_span(out)["start_byte"].clone()),
        span_member_column("target source_span.end_byte", &["relations", "extends", "0", "source_span", "end_byte"], 7, |out| target_span(out)["end_byte"].clone()),
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
        // `async` lives in the same list: an absent or empty list is a
        // declaration with no modifiers.
        Column {
            name: "modifiers (async)",
            target: Target::Record(ASYNC_FUNCTION),
            path: &["modifiers"],
            control: None,
            empty: None,
            public: |out| json!(is_async(record(out, ASYNC_FUNCTION))),
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
        // Absent and `[]` are the written encoding of "no attributes".
        Column {
            name: "attributes",
            target: Target::Record(STATIC_METHOD),
            path: &["attributes"],
            control: None,
            empty: None,
            public: |out| attribute_names(record(out, STATIC_METHOD)),
            outcomes: [
                S(json!(["staticmethod"])),
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
            name: "scope",
            target: Target::Record(METHOD),
            path: &["scope"],
            control: None,
            empty: None,
            public: |out| record(out, METHOD)["scope"].clone(),
            outcomes: [S(json!("Member")), R, R, R, R, R, R, R, R],
        },
        // The in-file name is never empty; only the current-run value is
        // servable. The report shows it behind the scan-root prefix.
        Column {
            name: "identity.qualified_name",
            target: Target::Record(METHOD),
            path: &["identity", "qualified_name"],
            control: None,
            empty: None,
            public: |out| record(out, METHOD)["identity"]["qualified_name"].clone(),
            outcomes: [S(json!("python.py#PlainWidget.instance_method")), R, R, R, R, R, R, R, R],
        },
        Column {
            name: "identity.is_namespace_rooted",
            target: Target::Record(METHOD),
            path: &["identity", "is_namespace_rooted"],
            control: None,
            empty: None,
            public: |out| record(out, METHOD)["identity"]["is_namespace_rooted"].clone(),
            outcomes: [S(json!(false)), R, R, R, R, R, R, R, R],
        },
        Column {
            name: "visibility.is_test",
            target: Target::Record(METHOD),
            path: &["visibility", "is_test"],
            control: None,
            empty: None,
            public: |out| record(out, METHOD)["visibility"]["is_test"].clone(),
            outcomes: [S(json!(false)), R, R, R, R, R, R, R, R],
        },
        // The entry claims the flags capability, so a set flag is served.
        Column {
            name: "visibility.is_deprecated",
            target: Target::Record(METHOD),
            path: &["visibility", "is_deprecated"],
            control: Some(json!(true)),
            empty: None,
            public: |out| record(out, METHOD)["visibility"]["is_deprecated"].clone(),
            outcomes: [S(json!(true)), R, R, R, R, R, R, R, R],
        },
        // The entry claims the metrics capability: every record must carry
        // the facet, and inside it every metric is required.
        Column {
            name: "metrics",
            target: Target::Record(METHOD),
            path: &["metrics"],
            control: None,
            empty: None,
            public: |out| record(out, METHOD)["metrics"].clone(),
            outcomes: [
                S(json!({ "parameters": 1, "has_docs": false, "lines": 2, "cyclomatic": null })),
                R,
                R,
                R,
                R,
                R,
                R,
                R,
                R,
            ],
        },
        // `null` is a metric the language does not support, never "not
        // computed".
        Column {
            name: "metrics.parameters",
            target: Target::Record(METHOD),
            path: &["metrics", "parameters"],
            control: None,
            empty: Some(json!("")),
            public: |out| record(out, METHOD)["metrics"]["parameters"].clone(),
            outcomes: [S(json!(1)), R, S(Value::Null), R, R, R, R, R, R],
        },
        Column {
            name: "metrics.has_docs",
            target: Target::Record(METHOD),
            path: &["metrics", "has_docs"],
            control: None,
            empty: None,
            public: |out| record(out, METHOD)["metrics"]["has_docs"].clone(),
            outcomes: [S(json!(false)), R, S(Value::Null), R, R, R, R, R, R],
        },
        // Set to a count first, so the wrong type is a string and the stored
        // value (not a recomputed one) is what reaches the output. An absent
        // key (an entry written before the key existed) is rejected, never
        // read as `null`; `0` is an empty body span.
        Column {
            name: "metrics.lines",
            target: Target::Record(METHOD),
            path: &["metrics", "lines"],
            control: Some(json!(3)),
            empty: Some(json!(0)),
            public: |out| record(out, METHOD)["metrics"]["lines"].clone(),
            outcomes: [S(json!(3)), R, S(Value::Null), R, R, R, S(json!(0)), R, R],
        },
        // Python has no counting rules, so a count is set first and the
        // stored value (not a recomputed `null`) is what reaches the output.
        // An absent key (an entry from before the key) is rejected, never
        // read as `null`, and no list or object is a count.
        Column {
            name: "metrics.cyclomatic",
            target: Target::Record(METHOD),
            path: &["metrics", "cyclomatic"],
            control: Some(json!(3)),
            empty: Some(json!([])),
            public: |out| record(out, METHOD)["metrics"]["cyclomatic"].clone(),
            outcomes: [S(json!(3)), R, S(Value::Null), R, R, R, R, R, R],
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
        Column {
            name: "record schema_version",
            target: Target::Record(METHOD),
            path: &["schema_version"],
            control: None,
            empty: None,
            public: |out| record(out, METHOD)["schema_version"].clone(),
            outcomes: [S(json!({ "major": 5, "minor": 0 })), R, R, R, R, R, R, R, R],
        },
        // Entry-level fields: none has a default. An empty record list is
        // servable only where no stored evidence names a record (a file with
        // no records of the covered kinds); here the name-only occurrences of
        // this every-kind entry name records the list no longer holds.
        entry_column("covered_kinds", Some(json!([])), [S(json!(true)), R, R, R, R, R, R, R, R]),
        entry_column("completed_capabilities", None, [S(json!(true)), R, R, R, R, R, R, R, R]),
        entry_column("schema_version", None, [S(json!(true)), R, R, R, R, R, R, R, R]),
        entry_column("fingerprint", None, [S(json!(true)), R, R, R, R, R, R, R, R]),
        entry_column("source_hash", None, [S(json!(true)), R, R, R, R, R, R, R, R]),
        entry_column("diagnostics_fingerprint", None, [S(json!(true)), R, R, R, R, R, R, R, R]),
        entry_column("records", None, [S(json!(true)), R, R, R, R, R, R, R, R]),
        // `""` names another file or no language, so the entry is for other
        // inputs.
        entry_column("file", None, [S(json!(true)), R, R, R, R, R, R, R, R]),
        entry_column("language", None, [S(json!(true)), R, R, R, R, R, R, R, R]),
        // An empty list is a file with none of them; elements are records.
        entry_column("imports", None, [S(json!(true)), R, R, R, R, R, S(json!(true)), R, R]),
        entry_column("exports", None, [S(json!(true)), R, R, R, R, R, S(json!(true)), R, R]),
        entry_column("diagnostics", None, [S(json!(true)), R, R, R, R, R, S(json!(true)), R, R]),
    ]
}

/// The populated span on `MatrixGeneric`'s first `extends` target: every
/// member distinct and nonzero, so a served member is told apart from its
/// neighbors, from the `0` cell and from a recomputed `null` span.
static TARGET_SPAN: std::sync::LazyLock<Value> = std::sync::LazyLock::new(|| {
    json!({ "start": { "line": 2, "column": 3 }, "end": { "line": 4, "column": 5 }, "start_byte": 6, "end_byte": 7 })
});

fn target_span(output: &Value) -> &Value {
    &record(output, GENERIC)["relations"]["extends"][0]["source_span"]
}

/// One required numeric member of the populated [`TARGET_SPAN`], whose
/// stored value is `populated`.
fn span_member_column(
    name: &'static str,
    path: &'static [&'static str],
    populated: u64,
    public: fn(&Value) -> Value,
) -> Column {
    Column {
        name,
        target: Target::Record(GENERIC),
        path,
        control: None,
        empty: Some(json!(0)),
        public,
        outcomes: [S(json!(populated)), R, R, R, R, R, S(json!(0)), R, R],
    }
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
            "file" => &["file"],
            "language" => &["language"],
            "imports" => &["imports"],
            "exports" => &["exports"],
            "diagnostics" => &["diagnostics"],
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
        parent = if let Ok(index) = segment.parse::<usize>() {
            &mut parent[index]
        } else {
            &mut parent[*segment]
        };
    }
    (parent, key)
}

/// The column's base snapshot (sentinel renamed, control value set) and
/// the field's value in it.
fn column_base(original: &Value, column: &Column) -> (Value, Value) {
    let mut base = original.clone();
    let parameter = &mut record_mut(&mut base, GENERIC)["type_info"]["generic_params"][0];
    parameter["default"] = json!("MatrixBase");
    parameter["variance"] = json!("out");
    let target = &mut record_mut(&mut base, GENERIC)["relations"]["extends"][0];
    target["id"] = json!("matrix-id");
    target["qualified_name"] = json!("MatrixBase");
    target["source_span"] = TARGET_SPAN.clone();
    field_base(base, column)
}

/// `snapshot` with the sentinel renamed and the column's control value set,
/// and the field's value in it.
fn field_base(mut base: Value, column: &Column) -> (Value, Value) {
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
        // A number's wrong type is a string; everything else's is a number.
        Shape::WrongType => parent[key] = if value.is_number() { json!("123") } else { json!(123) },
        // An object's elements are its members: one, or every, member gets
        // a value of the wrong type.
        Shape::OneWrongElement => match value {
            Value::Object(members) => {
                let mut members = members.clone();
                let last = members.keys().next_back().expect("a member").clone();
                members.insert(last, json!("bad"));
                parent[key] = Value::Object(members);
            }
            _ => {
                let mut items = elements;
                items.push(json!(123));
                parent[key] = Value::Array(items);
            }
        },
        Shape::EveryElementWrong => match value {
            Value::Object(members) => {
                parent[key] = members.keys().map(|member| (member.clone(), json!("bad"))).collect();
            }
            _ => parent[key] = Value::Array(vec![json!(123); elements.len().max(1)]),
        },
        Shape::IncompatibleMajor => parent[key] = json!({ "major": 3, "minor": 0 }),
        Shape::IncompatibleMinor => parent[key] = json!({ "major": 5, "minor": 1 }),
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
    let source = format!("{PYTHON_FIXTURE}\nclass MatrixBase: pass\nclass MatrixGeneric[T: MatrixBase](MatrixBase): pass\n");
    let scratch = Scratch::new(&[(FILE, &source)]);
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
                    let base = column_base(original, column);
                    check_column(scratch, &CACHED_ARGS, &base, uncached, relative_entry, column)
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

const MEMBER_FILE: &str = "java.java";
const MEMBER_ARGS: [&str; 3] = ["--json-full", "symbols", MEMBER_FILE];
/// Appended to the Java fixture: a raw attribute with an argument, a Custom
/// modifier and a fixed-tag modifier, all as extraction writes them, plus a
/// `module_function` record to serve as the sentinel.
const MEMBER_SOURCE: &str = "\n@ReviewAttr(\"raw value\")\nfinal class MatrixSubject { static void unit_subject() {} }\n\nclass MatrixSentinel { void module_function() {} }\n";
const SUBJECT: &str = "MatrixSubject";
const STATIC_SUBJECT: &str = "unit_subject";

/// One column per member of a stored `Attribute` and `Modifier`, with
/// controls written by extraction (no control overrides). An absent or
/// `null` attribute `value` is the Option's `None`; every other member is
/// required, and an empty modifier tag names no variant.
fn member_columns() -> Vec<Column> {
    vec![
        Column {
            name: "attribute name",
            target: Target::Record(SUBJECT),
            path: &["attributes", "0", "name"],
            control: None,
            empty: None,
            public: |out| record(out, SUBJECT)["attributes"][0]["name"].clone(),
            outcomes: [S(json!("ReviewAttr")), R, R, R, R, R, S(json!("")), R, R],
        },
        Column {
            name: "attribute value",
            target: Target::Record(SUBJECT),
            path: &["attributes", "0", "value"],
            control: None,
            empty: None,
            public: |out| record(out, SUBJECT)["attributes"][0]["value"].clone(),
            outcomes: [S(json!("\"raw value\"")), S(Value::Null), S(Value::Null), R, R, R, S(json!("")), R, R],
        },
        Column {
            name: "modifier type (Custom)",
            target: Target::Record(SUBJECT),
            path: &["modifiers", "0", "type"],
            control: None,
            empty: None,
            public: |out| record(out, SUBJECT)["modifiers"][0]["type"].clone(),
            outcomes: [S(json!("Custom")), R, R, R, R, R, R, R, R],
        },
        Column {
            name: "modifier Custom value",
            target: Target::Record(SUBJECT),
            path: &["modifiers", "0", "value"],
            control: None,
            empty: None,
            public: |out| record(out, SUBJECT)["modifiers"][0]["value"].clone(),
            outcomes: [S(json!("final")), R, R, R, R, R, S(json!("")), R, R],
        },
        Column {
            name: "modifier type (Static)",
            target: Target::Record(STATIC_SUBJECT),
            path: &["modifiers", "0", "type"],
            control: None,
            empty: None,
            public: |out| record(out, STATIC_SUBJECT)["modifiers"][0]["type"].clone(),
            outcomes: [S(json!("Static")), R, R, R, R, R, R, R, R],
        },
    ]
}

/// The same reader contract as
/// [`persisted_symbol_fields_are_served_or_recomputed_per_shape`], for each
/// member of a stored attribute and modifier.
#[test]
fn attribute_and_modifier_members_are_served_or_recomputed_per_shape() {
    let source = format!("{JAVA_FIXTURE}{MEMBER_SOURCE}");
    let scratch = Scratch::new(&[(MEMBER_FILE, &source)]);
    let uncached = json_of(&scratch.stdout(&["--json-full", "symbols", "--no-cache", MEMBER_FILE]));
    assert_eq!(json_of(&scratch.stdout(&MEMBER_ARGS)), uncached, "the cache-writing run");
    let written = |name: &str| record(&uncached, name).clone();
    assert_eq!(
        (written(SUBJECT)["attributes"].clone(), written(SUBJECT)["modifiers"].clone()),
        (
            json!([{ "name": "ReviewAttr", "value": "\"raw value\"" }]),
            json!([{ "type": "Custom", "value": "final" }])
        ),
        "the controls come from extraction"
    );
    assert_eq!(written(STATIC_SUBJECT)["modifiers"], json!([{ "type": "Static" }]));
    let entry = only_entry(scratch.temp.path());
    let relative_entry = entry
        .strip_prefix(scratch.temp.path())
        .expect("entry under the scratch temp directory")
        .to_path_buf();
    let original = read_json(&entry);

    let columns = member_columns();
    let failures: Vec<String> = std::thread::scope(|scope| {
        let handles: Vec<_> = columns
            .iter()
            .map(|column| {
                let (scratch, original, uncached) = (&scratch, &original, &uncached);
                let relative_entry = relative_entry.as_path();
                scope.spawn(move || {
                    let base = field_base(original.clone(), column);
                    check_column(scratch, &MEMBER_ARGS, &base, uncached, relative_entry, column)
                })
            })
            .collect();
        handles
            .into_iter()
            .flat_map(|handle| handle.join().expect("column thread"))
            .collect()
    });
    assert!(failures.is_empty(), "attribute and modifier member matrix:\n{}", failures.join("\n"));
}

/// Runs every cell of `column`, returning one message per cell whose outcome
/// differs from the table.
fn check_column(
    scratch: &Scratch,
    args: &[&str],
    (base, value): &(Value, Value),
    uncached: &Value,
    relative_entry: &Path,
    column: &Column,
) -> Vec<String> {
    let temp = TempDir::new().expect("column temp");
    let entry = temp.path().join(relative_entry);
    std::fs::create_dir_all(entry.parent().expect("entry parent")).expect("create shard");
    let mut failures = Vec::new();
    let version_cells = if column.path == ["schema_version"] {
        &VERSION_SHAPES[..]
    } else {
        &[]
    };
    let cells = SHAPES
        .iter()
        .zip(&column.outcomes)
        .chain(version_cells.iter().zip(std::iter::repeat(&Outcome::Rejected)));
    for (shape, outcome) in cells {
        if matches!(outcome, Outcome::Repeated) {
            continue;
        }
        std::fs::write(&entry, edited_entry(base, column, *shape, value))
            .expect("write cache entry");
        let output = json_of(&stdout_of(scratch.hug_with_temp(temp.path(), args), args));
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
            Outcome::Repeated => unreachable!("not run"),
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
            "occurrences",
            "semantics",
            "structured_docs",
            "diagnostics",
            "flags",
            "metrics"
        ])
    );
    assert!(entry["diagnostics_fingerprint"].is_string(), "diagnostics carry their fingerprint");
    assert!(entry["occurrences"].is_array(), "the entry stores the occurrences");
    assert!(
        entry["reference_coverage"].as_array().is_some_and(|regions| !regions.is_empty()),
        "the entry stores the coverage, the file-level region at least"
    );
    // The entry writes every key, so absent, `null` and empty differ.
    assert_eq!(entry["reference_coverage"][0]["owner"], Value::Null, "the file region has a null owner");
    assert!(
        entry["reference_coverage"][0].as_object().is_some_and(|region| region.contains_key("owner")),
        "the null owner is written, not omitted"
    );
    let public_coverage: Vec<tree_hugger::ReferenceCoverage> =
        serde_json::from_value(entry["reference_coverage"].clone()).expect("stored coverage reads publicly");
    let public_coverage = serde_json::to_value(public_coverage).expect("public coverage");
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
        assert_eq!(
            index["reference_coverage"], public_coverage,
            "{label}: the complete index reports the stored coverage"
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

/// `metrics.lines` and `metrics.cyclomatic` are `u32`s: a fraction, a
/// negative count, a count past `u32::MAX`, a Boolean and an object reject
/// the entry (recomputed), while `u32::MAX` itself is served. These are the
/// wrong-type cells the matrix's one `WrongType` (a string) does not reach.
#[test]
fn metrics_counts_serve_only_a_u32() {
    let scratch = Scratch::new(&[(FILE, PYTHON_FIXTURE)]);
    let uncached = json_of(&scratch.stdout(&UNCACHED_ARGS));
    json_of(&scratch.stdout(&CACHED_ARGS));
    let entry = only_entry(scratch.temp.path());
    let relative_entry = entry
        .strip_prefix(scratch.temp.path())
        .expect("entry under the scratch temp directory")
        .to_path_buf();
    let original = read_json(&entry);

    let temp = TempDir::new().expect("cell temp");
    let cell_entry = temp.path().join(&relative_entry);
    std::fs::create_dir_all(cell_entry.parent().expect("entry parent")).expect("create shard");
    for key in ["lines", "cyclomatic"] {
        for (value, served) in [
            (json!(3.5), false),
            (json!(-1), false),
            (json!(4_294_967_296_u64), false),
            (json!(true), false),
            (json!({}), false),
            (json!(4_294_967_295_u32), true),
        ] {
            let cell = format!("{key} {value}");
            let mut edited = original.clone();
            rename_record(&mut edited, SENTINEL_FROM, SENTINEL_TO);
            record_mut(&mut edited, METHOD)["metrics"][key] = value.clone();
            std::fs::write(&cell_entry, serde_json::to_string(&edited).expect("serialize")).expect("write entry");
            let output = json_of(&stdout_of(scratch.hug_with_temp(temp.path(), &CACHED_ARGS), &CACHED_ARGS));
            if served {
                assert!(has_sentinel(&output), "{cell}: served");
                assert_eq!(record(&output, METHOD)["metrics"][key], value, "{cell}: stored value");
            } else {
                assert!(!has_sentinel(&output), "{cell}: rejected");
                assert_eq!(output, uncached, "{cell}: recomputed like an uncached run");
            }
        }
    }
}

/// An entry written without the metrics capability holds no `metrics` facet
/// at all. It is served for a request that does not ask for metrics, and a
/// request that does recomputes the facet (every key, `lines` included)
/// instead of reading the missing facet as computed `null`s.
#[test]
fn an_absent_metrics_facet_is_not_requested_never_computed_null() {
    let scratch = Scratch::new(&[(FILE, PYTHON_FIXTURE)]);
    let listing = ["--json", "functions", FILE];
    json_of(&scratch.stdout(&listing));
    let entry = only_entry(scratch.temp.path());
    let written = read_json(&entry);
    assert!(
        !written["completed_capabilities"]
            .as_array()
            .expect("completed_capabilities")
            .contains(&json!("metrics")),
        "a listing does not compute metrics"
    );
    assert!(
        written["records"]
            .as_array()
            .expect("entry records")
            .iter()
            .all(|record| record.get("metrics").is_none()),
        "no record carries a metrics facet"
    );

    let mut renamed = written.clone();
    rename_record(&mut renamed, SENTINEL_FROM, SENTINEL_TO);
    std::fs::write(&entry, serde_json::to_string(&renamed).expect("serialize")).expect("write entry");
    let served = json_of(&scratch.stdout(&listing));
    let names: Vec<&Value> = served["files"][0]["symbols"]
        .as_array()
        .or_else(|| served["symbols"].as_array())
        .map(|records| records.iter().map(|record| &record["identity"]["name"]).collect())
        .unwrap_or_default();
    assert!(
        names.contains(&&json!(SENTINEL_TO)),
        "the entry is served to a request without metrics (names: {names:?})"
    );

    std::fs::write(&entry, serde_json::to_string(&written).expect("serialize")).expect("restore entry");
    let widened = json_of(&scratch.stdout(&CACHED_ARGS));
    assert_eq!(widened, json_of(&scratch.stdout(&UNCACHED_ARGS)), "widened by metrics: an uncached result");
    assert_eq!(
        record(&widened, METHOD)["metrics"],
        json!({ "parameters": 1, "has_docs": false, "lines": 2, "cyclomatic": null }),
        "the facet is computed with every key"
    );
}

/// A TypeScript file whose evidence has every confidence, an in-file edge,
/// a limited region, direct recursion, an ownerless call, a call through an
/// import, a binding no record reports named like a declaration, a callable
/// with no occurrences, a non-ASCII character and, last, a qualified call of
/// an unresolved name (`other.thing()`).
const EVIDENCE_FILE: &str = "evidence.ts";
const EVIDENCE_SOURCE: &str = "function helper(): number { return 1; }\n\
    function module_function(): number { return helper(); }\n\
    class Box { size(): number { return 1; } }\n\
    function caller(box: Box): number { return box.size() + missing(); }\n\
    function again(n: number): number { return again(n); }\n\
    again(1);\n\
    import { external } from \"./external\";\n\
    function uses_import(): number { return external(); }\n\
    function shadow(): void { for (const helper of [1]) { helper; } }\n\
    // café\n\
    function qualified(): void { other.thing(); }\n";
const EVIDENCE_ARGS: [&str; 3] = ["--json-full", "symbols", EVIDENCE_FILE];
/// `occurrences[0]`: the `Scoped` call of `helper` in `module_function`.
const SCOPED: &str = "0";
/// `occurrences[3]`: the `NameOnly` member call `box.size()`.
const NAME_ONLY: &str = "3";
/// `occurrences[4]`: the `Unresolved` call of `missing`.
const UNRESOLVED: &str = "4";
/// `occurrences[5]`: `again`'s `Scoped` call of itself.
const SELF_CALL: &str = "5";
/// `occurrences[7]`: the file-level (ownerless) `Scoped` call of `again`.
const OWNERLESS: &str = "7";
/// `occurrences[8]`: the `Scoped` call of the import `external` (no ID).
const IMPORTED: &str = "8";
/// `occurrences[9]`: the `for` binding `helper` (`Scoped`, no ID).
const UNREPORTED: &str = "9";
/// `occurrences[11]`: `thing` in `other.thing()`, written path `other.thing`,
/// a qualified name that reaches no member.
const QUALIFIED: &str = "11";
/// `reference_coverage[1]`: `helper`'s region, which owns no occurrence.
const EMPTY_CALLABLE: &str = "1";
/// `reference_coverage[4]`: `caller`'s region, limited by `box.size()`.
const LIMITED: &str = "4";

/// A column over the entry's reference evidence, served only unedited.
fn evidence_column(name: &'static str, path: &'static [&'static str]) -> Column {
    Column {
        name,
        target: Target::Entry,
        path,
        control: None,
        empty: None,
        public: |out| json!(has_sentinel(out)),
        outcomes: [S(json!(true)), R, R, R, R, R, R, R, R],
    }
}

/// A leaf evidence field, served only unedited; `empty` is the `Empty`
/// shape's value (`""`, `0` or `{}`). Its element and document-level shapes
/// repeat cells its parent's column runs.
fn evidence_leaf(name: &'static str, path: &'static [&'static str], empty: Value) -> Column {
    Column { empty: Some(empty), outcomes: [S(json!(true)), R, R, R, X, X, R, X, X], ..evidence_column(name, path) }
}

/// A scalar evidence field, served only unedited, whose duplicate-key and
/// trailing-content cells run too; a scalar has no element shapes.
fn evidence_scalar(name: &'static str, path: &'static [&'static str], empty: Value) -> Column {
    Column { outcomes: [S(json!(true)), R, R, R, X, X, R, R, R], ..evidence_leaf(name, path, empty) }
}

/// Every evidence field the cache reader decides on. No cell but the control
/// is servable, except a `null` written path (and its `null` stored start)
/// on an unqualified name: each edit leaves evidence a fresh analysis
/// cannot produce (`[]` occurrences while a record keeps the edge they made;
/// a `null` target ID on the
/// `Scoped` occurrence that made that edge; a `NameOnly` with no candidates;
/// a `Limited` region with no reason; a written path that is empty, or that
/// disagrees with its stored start; a span leaf that no longer agrees with
/// the rest of the span or the source).
fn evidence_columns() -> Vec<Column> {
    vec![
        // Written paths: `helper()` is unqualified (its path and stored start
        // are both `null`, so their `null` cells are the control);
        // `box.size()` reaches a member and `other.thing()` none, and each
        // keeps its path.
        Column {
            outcomes: [S(json!(true)), R, S(json!(true)), R, R, R, R, R, R],
            ..evidence_leaf("unqualified target.qualified_name", &["occurrences", SCOPED, "target", "qualified_name"], json!(""))
        },
        Column {
            outcomes: [S(json!(true)), R, R, R, R, R, R, R, R],
            ..evidence_leaf("member target.qualified_name", &["occurrences", NAME_ONLY, "target", "qualified_name"], json!(""))
        },
        evidence_scalar("qualified target.qualified_name", &["occurrences", QUALIFIED, "target", "qualified_name"], json!("")),
        // The cache-only start of each written path (`Empty` is `0`).
        Column {
            outcomes: [S(json!(true)), R, S(json!(true)), R, X, X, R, R, R],
            ..evidence_leaf("unqualified path_start_byte", &["occurrences", SCOPED, "path_start_byte"], json!(0))
        },
        evidence_scalar("member path_start_byte", &["occurrences", NAME_ONLY, "path_start_byte"], json!(0)),
        evidence_scalar("qualified path_start_byte", &["occurrences", QUALIFIED, "path_start_byte"], json!(0)),
        evidence_scalar("occurrence kind", &["occurrences", SCOPED, "kind"], json!("")),
        evidence_column("occurrences", &["occurrences"]),
        evidence_column("reference_coverage", &["reference_coverage"]),
        evidence_column("occurrence confidence", &["occurrences", SCOPED, "confidence"]),
        evidence_column("occurrence target.id", &["occurrences", SCOPED, "target", "id"]),
        evidence_column("occurrence candidates", &["occurrences", NAME_ONLY, "candidates"]),
        evidence_column("occurrence span", &["occurrences", SCOPED, "span"]),
        evidence_column("occurrence owner", &["occurrences", SCOPED, "owner"]),
        evidence_column("occurrence owner_span", &["occurrences", SCOPED, "owner_span"]),
        evidence_column("ownerless occurrence target.id", &["occurrences", OWNERLESS, "target", "id"]),
        evidence_column("coverage owner", &["reference_coverage", LIMITED, "owner"]),
        evidence_column("coverage state", &["reference_coverage", LIMITED, "state"]),
        evidence_column("coverage reasons", &["reference_coverage", LIMITED, "reasons"]),
        evidence_column("coverage span", &["reference_coverage", LIMITED, "span"]),
    ]
}

/// The leaves inside the evidence's owner, target and candidate references
/// (names, IDs, qualified names), each served only unedited.
fn reference_leaf_columns() -> Vec<Column> {
    vec![
        evidence_leaf("occurrence target.name", &["occurrences", SCOPED, "target", "name"], json!("")),
        evidence_leaf("occurrence owner.id", &["occurrences", SCOPED, "owner", "id"], json!("")),
        evidence_leaf("occurrence owner.name", &["occurrences", SCOPED, "owner", "name"], json!("")),
        evidence_leaf("occurrence owner.qualified_name", &["occurrences", SCOPED, "owner", "qualified_name"], json!("")),
        evidence_leaf("candidate id", &["occurrences", NAME_ONLY, "candidates", "0", "id"], json!("")),
        evidence_leaf("candidate name", &["occurrences", NAME_ONLY, "candidates", "0", "name"], json!("")),
        evidence_leaf("candidate qualified_name", &["occurrences", NAME_ONLY, "candidates", "0", "qualified_name"], json!("")),
        evidence_leaf("coverage owner.id", &["reference_coverage", LIMITED, "owner", "id"], json!("")),
        evidence_leaf("coverage owner.name", &["reference_coverage", LIMITED, "owner", "name"], json!("")),
        evidence_leaf("coverage owner.qualified_name", &["reference_coverage", LIMITED, "owner", "qualified_name"], json!("")),
    ]
}

/// The leaves inside the evidence's spans (byte offsets, points and their
/// line and column), each served only unedited, in every one of the three
/// spans; the owner and coverage span points also run their duplicate-key
/// and trailing-content cells.
fn span_leaf_columns() -> Vec<Column> {
    let zero = || json!(0);
    let point = || json!({});
    vec![
        evidence_leaf("occurrence span.start_byte", &["occurrences", SCOPED, "span", "start_byte"], zero()),
        evidence_leaf("occurrence span.end_byte", &["occurrences", SCOPED, "span", "end_byte"], zero()),
        evidence_leaf("occurrence span.start", &["occurrences", SCOPED, "span", "start"], point()),
        evidence_leaf("occurrence span.end", &["occurrences", SCOPED, "span", "end"], point()),
        evidence_leaf("occurrence span.start.line", &["occurrences", SCOPED, "span", "start", "line"], zero()),
        evidence_leaf("occurrence span.start.column", &["occurrences", SCOPED, "span", "start", "column"], zero()),
        evidence_leaf("occurrence span.end.line", &["occurrences", SCOPED, "span", "end", "line"], zero()),
        evidence_leaf("occurrence span.end.column", &["occurrences", SCOPED, "span", "end", "column"], zero()),
        evidence_leaf("owner_span.start_byte", &["occurrences", SCOPED, "owner_span", "start_byte"], zero()),
        evidence_leaf("owner_span.end_byte", &["occurrences", SCOPED, "owner_span", "end_byte"], zero()),
        evidence_leaf("owner_span.start", &["occurrences", SCOPED, "owner_span", "start"], point()),
        evidence_leaf("owner_span.end", &["occurrences", SCOPED, "owner_span", "end"], point()),
        evidence_scalar("owner_span.start.line", &["occurrences", SCOPED, "owner_span", "start", "line"], zero()),
        evidence_scalar("owner_span.start.column", &["occurrences", SCOPED, "owner_span", "start", "column"], zero()),
        evidence_scalar("owner_span.end.line", &["occurrences", SCOPED, "owner_span", "end", "line"], zero()),
        evidence_scalar("owner_span.end.column", &["occurrences", SCOPED, "owner_span", "end", "column"], zero()),
        evidence_leaf("coverage span.start_byte", &["reference_coverage", LIMITED, "span", "start_byte"], zero()),
        evidence_leaf("coverage span.end_byte", &["reference_coverage", LIMITED, "span", "end_byte"], zero()),
        evidence_leaf("coverage span.start", &["reference_coverage", LIMITED, "span", "start"], point()),
        evidence_leaf("coverage span.end", &["reference_coverage", LIMITED, "span", "end"], point()),
        evidence_scalar("coverage span.start.line", &["reference_coverage", LIMITED, "span", "start", "line"], zero()),
        evidence_scalar("coverage span.start.column", &["reference_coverage", LIMITED, "span", "start", "column"], zero()),
        evidence_scalar("coverage span.end.line", &["reference_coverage", LIMITED, "span", "end", "line"], zero()),
        evidence_scalar("coverage span.end.column", &["reference_coverage", LIMITED, "span", "end", "column"], zero()),
    ]
}

/// The occurrence-shaped reference to the stored record called `name`.
fn reference_to(entry: &mut Value, name: &str) -> Value {
    let record = record_mut(entry, name);
    json!({ "id": record["id"], "name": name, "qualified_name": record["identity"]["qualified_name"] })
}

fn region<'a>(entry: &'a mut Value, index: &str) -> &'a mut Value {
    &mut entry["reference_coverage"][index.parse::<usize>().expect("index")]
}

fn occurrence<'a>(entry: &'a mut Value, index: &str) -> &'a mut Value {
    &mut entry["occurrences"][index.parse::<usize>().expect("index")]
}

/// A labeled edit to a cache entry.
type EntryEdit = (&'static str, fn(&mut Value));

/// Edits whose evidence is well-formed JSON but contradicts the source,
/// itself, the capability claim or the records derived from it.
const INCONSISTENT_EVIDENCE: &[EntryEdit] = &[
    ("zero-length span", |entry| {
        let span = &mut occurrence(entry, SCOPED)["span"];
        span["end"] = span["start"].clone();
        span["end_byte"] = span["start_byte"].clone();
    }),
    ("span ending before it starts", |entry| {
        let span = &mut occurrence(entry, SCOPED)["span"];
        let (start, start_byte) = (span["start"].clone(), span["start_byte"].clone());
        span["start"] = span["end"].clone();
        span["start_byte"] = span["end_byte"].clone();
        span["end"] = start;
        span["end_byte"] = start_byte;
    }),
    ("span past the end of the source", |entry| {
        let span = &mut occurrence(entry, SCOPED)["span"];
        span["end_byte"] = json!(EVIDENCE_SOURCE.len() + 1);
    }),
    ("span off a character boundary", |entry| {
        let inside = EVIDENCE_SOURCE.find('é').expect("é") + 1;
        let span = &mut entry["reference_coverage"][0]["span"];
        span["end_byte"] = json!(inside);
    }),
    ("span points that disagree with its offsets", |entry| {
        let span = &mut occurrence(entry, SCOPED)["span"];
        span["start"]["column"] = json!(span["start"]["column"].as_u64().expect("column") + 1);
    }),
    ("target name that is not the text at its span", |entry| {
        occurrence(entry, SCOPED)["target"]["name"] = json!("other");
    }),
    ("Scoped with candidates", |entry| {
        let candidates = occurrence(entry, NAME_ONLY)["candidates"].clone();
        occurrence(entry, SCOPED)["candidates"] = candidates;
    }),
    ("NameOnly with a target ID", |entry| {
        let id = occurrence(entry, SCOPED)["target"]["id"].clone();
        occurrence(entry, NAME_ONLY)["target"]["id"] = id;
    }),
    ("NameOnly with a repeated candidate", |entry| {
        let candidates = occurrence(entry, NAME_ONLY)["candidates"].as_array_mut().expect("candidates");
        candidates.push(candidates[0].clone());
    }),
    ("Unresolved with a target ID", |entry| {
        let id = occurrence(entry, SCOPED)["target"]["id"].clone();
        occurrence(entry, UNRESOLVED)["target"]["id"] = id;
    }),
    ("Unresolved with candidates", |entry| {
        let candidates = occurrence(entry, NAME_ONLY)["candidates"].clone();
        occurrence(entry, UNRESOLVED)["candidates"] = candidates;
    }),
    ("a repeated occurrence", |entry| {
        let list = entry["occurrences"].as_array_mut().expect("occurrences");
        list.insert(1, list[0].clone());
    }),
    ("occurrences out of order", |entry| {
        entry["occurrences"].as_array_mut().expect("occurrences").swap(0, 1);
    }),
    ("an owner that owns no coverage region", |entry| {
        let other = occurrence(entry, NAME_ONLY)["owner"].clone();
        occurrence(entry, SCOPED)["owner"] = other;
    }),
    ("a region owner whose name is not its record's", |entry| {
        entry["reference_coverage"][LIMITED.parse::<usize>().expect("index")]["owner"]["qualified_name"] =
            json!("elsewhere");
    }),
    ("a candidate whose name is not its record's", |entry| {
        occurrence(entry, NAME_ONLY)["candidates"][0]["name"] = json!("elsewhere");
    }),
    ("Complete coverage with a reason", |entry| {
        entry["reference_coverage"][1]["reasons"] = json!(["indirect_dispatch"]);
    }),
    ("repeated coverage reasons", |entry| {
        entry["reference_coverage"][LIMITED.parse::<usize>().expect("index")]["reasons"] =
            json!(["indirect_dispatch", "indirect_dispatch"]);
    }),
    ("no file-level region", |entry| {
        entry["reference_coverage"].as_array_mut().expect("coverage").remove(0);
    }),
    ("coverage out of order", |entry| {
        entry["reference_coverage"].as_array_mut().expect("coverage").swap(1, 2);
    }),
    ("evidence without the occurrences capability", |entry| {
        entry["completed_capabilities"]
            .as_array_mut()
            .expect("capabilities")
            .retain(|capability| capability != "occurrences" && capability != "semantics");
    }),
    ("a reference edge whose occurrence is gone", |entry| {
        entry["occurrences"].as_array_mut().expect("occurrences").remove(0);
    }),
    ("a referenced-by edge no occurrence made", |entry| {
        let caller = record_mut(entry, "caller")["id"].clone();
        record_mut(entry, "size")["relations"]["referenced_by"] = json!([{ "id": caller, "name": "caller" }]);
    }),
    ("direct recursion with no call to itself", |entry| {
        record_mut(entry, "helper")["semantics"]["is_recursive"] = json!("Yes");
    }),
    // A target ID is checked against the record it names wherever that
    // record is retained, with or without an owner to make an edge.
    ("an ownerless Scoped target naming another declaration", |entry| {
        occurrence(entry, OWNERLESS)["target"]["id"] = record_mut(entry, "helper")["id"].clone();
    }),
    ("an ownerless Scoped target with a malformed ID", |entry| {
        occurrence(entry, OWNERLESS)["target"]["id"] = json!("bogus");
    }),
    ("an ownerless Scoped target that lost its ID", |entry| {
        occurrence(entry, OWNERLESS)["target"]["id"] = Value::Null;
    }),
    ("a named owner's Scoped target naming another declaration", |entry| {
        occurrence(entry, SELF_CALL)["target"]["id"] = record_mut(entry, "helper")["id"].clone();
    }),
    ("a named owner's Scoped target with a malformed ID", |entry| {
        occurrence(entry, SELF_CALL)["target"]["id"] = json!("bogus");
    }),
    ("a candidate naming a declaration of another name", |entry| {
        occurrence(entry, NAME_ONLY)["candidates"][0] = reference_to(entry, "helper");
    }),
    ("a region owner with a malformed ID", |entry| {
        region(entry, LIMITED)["owner"]["id"] = json!("bogus");
    }),
    ("a region owned by a declaration that is not a callable", |entry| {
        region(entry, EMPTY_CALLABLE)["owner"] = reference_to(entry, "Box");
    }),
    ("a repeated coverage region", |entry| {
        let list = entry["reference_coverage"].as_array_mut().expect("coverage");
        let limited = LIMITED.parse::<usize>().expect("index");
        list.insert(limited, list[limited].clone());
    }),
    ("a callable's region removed although it owns no occurrence", |entry| {
        entry["reference_coverage"]
            .as_array_mut()
            .expect("coverage")
            .remove(EMPTY_CALLABLE.parse::<usize>().expect("index"));
    }),
    ("an occurrence owned by a region around its innermost one", |entry| {
        let file_span = entry["reference_coverage"][0]["span"].clone();
        let occurrence = occurrence(entry, NAME_ONLY);
        occurrence["owner"] = Value::Null;
        occurrence["owner_span"] = file_span;
    }),
    ("an unsupported-language limit in a resolving language", |entry| {
        let region = region(entry, LIMITED);
        region["reasons"] = json!(["indirect_dispatch", "unsupported_language"]);
    }),
    ("a Scoped occurrence whose reference edge is gone", |entry| {
        record_mut(entry, SENTINEL_TO)["relations"]["references"] = json!([]);
    }),
    ("a dependency no import occurrence made", |entry| {
        let relations = &mut record_mut(entry, "uses_import")["relations"];
        for summary in ["references", "dependencies"] {
            relations[summary].as_array_mut().expect("summary").push(json!({ "name": "ghost" }));
        }
    }),
    ("a referenced-by entry whose name is not its record's", |entry| {
        record_mut(entry, "helper")["relations"]["referenced_by"][0]["name"] = json!("elsewhere");
    }),
    ("no recursion while the region is limited", |entry| {
        record_mut(entry, "caller")["semantics"]["is_recursive"] = json!("No");
    }),
    ("no recursion beside a Scoped call to itself", |entry| {
        record_mut(entry, "again")["semantics"]["is_recursive"] = json!("No");
    }),
    ("unknown recursion over a complete region with no call", |entry| {
        record_mut(entry, "helper")["semantics"]["is_recursive"] = json!("Unknown");
    }),
    ("semantics without the semantics capability", |entry| {
        entry["completed_capabilities"]
            .as_array_mut()
            .expect("capabilities")
            .retain(|capability| capability != "semantics");
    }),
];

/// A scratch repository holding [`EVIDENCE_SOURCE`], its uncached output,
/// and the entry `hug` wrote for it (path relative to the scratch temp
/// directory, and contents).
fn written_evidence() -> (Scratch, Value, std::path::PathBuf, Value) {
    let scratch = Scratch::new(&[(EVIDENCE_FILE, EVIDENCE_SOURCE)]);
    let uncached = json_of(&scratch.stdout(&["--json-full", "symbols", "--no-cache", EVIDENCE_FILE]));
    assert_eq!(json_of(&scratch.stdout(&EVIDENCE_ARGS)), uncached, "the cache-writing run");
    let entry = only_entry(scratch.temp.path());
    let relative_entry = entry
        .strip_prefix(scratch.temp.path())
        .expect("entry under the scratch temp directory")
        .to_path_buf();
    let original = read_json(&entry);
    (scratch, uncached, relative_entry, original)
}

/// Every cell of `columns` over the written evidence entry, one thread per
/// column.
fn check_evidence_columns(
    scratch: &Scratch,
    original: &Value,
    uncached: &Value,
    relative_entry: &Path,
    columns: &[Column],
) -> Vec<String> {
    std::thread::scope(|scope| {
        let handles: Vec<_> = columns
            .iter()
            .map(|column| {
                scope.spawn(move || {
                    let base = field_base(original.clone(), column);
                    check_column(scratch, &EVIDENCE_ARGS, &base, uncached, relative_entry, column)
                })
            })
            .collect();
        handles
            .into_iter()
            .flat_map(|handle| handle.join().expect("column thread"))
            .collect()
    })
}

/// The leaves of the evidence's references, each shape of each one a miss
/// equal to an uncached run; the unedited entry is served.
#[test]
fn reference_evidence_reference_leaves_are_served_only_when_consistent() {
    let (scratch, uncached, relative_entry, original) = written_evidence();
    let failures = check_evidence_columns(&scratch, &original, &uncached, &relative_entry, &reference_leaf_columns());
    assert!(failures.is_empty(), "reference leaf matrix:\n{}", failures.join("\n"));
}

/// The leaves of the evidence's spans, each shape of each one a miss equal to
/// an uncached run; the unedited entry is served.
#[test]
fn reference_evidence_span_leaves_are_served_only_when_consistent() {
    let (scratch, uncached, relative_entry, original) = written_evidence();
    let failures = check_evidence_columns(&scratch, &original, &uncached, &relative_entry, &span_leaf_columns());
    assert!(failures.is_empty(), "span leaf matrix:\n{}", failures.join("\n"));
}

/// The cache reader's contract for the reference evidence of an entry that
/// claims the occurrences capability: each matrix shape of each evidence
/// field, and each internally inconsistent edit, is a miss whose output
/// equals an uncached run, never evidence read as empty; the unedited entry,
/// and a file that truly has no occurrences, are served.
#[test]
fn reference_evidence_is_served_only_when_consistent() {
    let (scratch, uncached, relative_entry, original) = written_evidence();

    // The cells name these entries by position.
    let stored = |list: &str, index: &str| original[list][index.parse::<usize>().expect("index")].clone();
    assert_eq!(
        (stored("occurrences", SCOPED)["target"]["name"].clone(), stored("occurrences", SCOPED)["confidence"].clone()),
        (json!("helper"), json!("Scoped"))
    );
    assert_eq!(stored("occurrences", NAME_ONLY)["confidence"], "NameOnly");
    assert_eq!(stored("occurrences", UNRESOLVED)["confidence"], "Unresolved");
    assert_eq!(
        (stored("reference_coverage", LIMITED)["owner"]["name"].clone(), stored("reference_coverage", LIMITED)["state"].clone()),
        (json!("caller"), json!("Limited"))
    );
    assert_eq!(record(&uncached, "again")["semantics"]["is_recursive"], "Yes");
    // The control cells serve these legitimate shapes: an ownerless target
    // with an ID, and `Scoped` targets with no ID bound to an import and to
    // a binding no record reports, beside a declaration of the same name.
    let scoped = |index: &str| {
        let occurrence = stored("occurrences", index);
        (occurrence["owner"]["name"].clone(), occurrence["target"]["name"].clone(), occurrence["target"]["id"].is_string())
    };
    assert_eq!(scoped(SELF_CALL), (json!("again"), json!("again"), true));
    assert_eq!(scoped(OWNERLESS), (Value::Null, json!("again"), true));
    assert_eq!(scoped(IMPORTED), (json!("uses_import"), json!("external"), false));
    assert_eq!(scoped(UNREPORTED), (json!("shadow"), json!("helper"), false));
    assert_eq!(stored("reference_coverage", EMPTY_CALLABLE)["owner"]["name"], "helper");

    let mut failures = check_evidence_columns(&scratch, &original, &uncached, &relative_entry, &evidence_columns());

    let mut base = original.clone();
    rename_record(&mut base, SENTINEL_FROM, SENTINEL_TO);
    // Each edit chunk serves its edits from its own temp directory.
    let edit_failures: Vec<String> = std::thread::scope(|scope| {
        let handles: Vec<_> = INCONSISTENT_EVIDENCE
            .chunks(INCONSISTENT_EVIDENCE.len().div_ceil(4))
            .map(|edits| {
                let (scratch, base, uncached) = (&scratch, &base, &uncached);
                let relative_entry = relative_entry.as_path();
                scope.spawn(move || {
                    let temp = TempDir::new().expect("edit temp");
                    let edited_path = temp.path().join(relative_entry);
                    std::fs::create_dir_all(edited_path.parent().expect("entry parent")).expect("create shard");
                    let mut failures = Vec::new();
                    for (label, edit) in edits {
                        let mut edited = base.clone();
                        edit(&mut edited);
                        std::fs::write(&edited_path, serde_json::to_string(&edited).expect("serialize"))
                            .expect("write entry");
                        let output =
                            json_of(&stdout_of(scratch.hug_with_temp(temp.path(), &EVIDENCE_ARGS), &EVIDENCE_ARGS));
                        if has_sentinel(&output) {
                            failures.push(format!("{label}: expected rejected, entry was served"));
                        } else if output != *uncached {
                            failures.push(format!("{label}: rejected, but output differs from an uncached run"));
                        }
                    }
                    failures
                })
            })
            .collect();
        handles.into_iter().flat_map(|handle| handle.join().expect("edit thread")).collect()
    });
    failures.extend(edit_failures);
    assert!(failures.is_empty(), "reference evidence matrix:\n{}", failures.join("\n"));

    // An empty list is servable when the file truly has no occurrences.
    let quiet = Scratch::new(&[("quiet.ts", "function module_function(): void {}\n")]);
    let quiet_args = ["--json-full", "symbols", "quiet.ts"];
    let cold = json_of(&quiet.stdout(&quiet_args));
    let quiet_entry = only_entry(quiet.temp.path());
    let mut written = read_json(&quiet_entry);
    assert_eq!(written["occurrences"], json!([]), "the file has no occurrences");
    assert_eq!(cold["indexes"][0]["reference_coverage"][0]["state"], "Complete");
    rename_record(&mut written, SENTINEL_FROM, SENTINEL_TO);
    std::fs::write(&quiet_entry, serde_json::to_string(&written).expect("serialize")).expect("write entry");
    assert!(has_sentinel(&json_of(&quiet.stdout(&quiet_args))), "empty evidence of a quiet file is served");
}

/// The primary symbol fixture of every language. TypeScript and JavaScript
/// gain a member call, the only way their primary fixtures make `NameOnly`
/// evidence with candidates.
const MEMBER_PROBE: &str = "\nfunction phantomProbe(o) { return o.instanceMethod(); }\n";
const LANGUAGE_FIXTURES: [(&str, &str, bool); 16] = [
    ("rust.rs", include_str!("../../../lib/tests/fixtures/symbols/rust.rs"), false),
    ("typescript.ts", include_str!("../../../lib/tests/fixtures/symbols/typescript.ts"), true),
    ("javascript.js", include_str!("../../../lib/tests/fixtures/symbols/javascript.js"), true),
    ("go.go", include_str!("../../../lib/tests/fixtures/symbols/go.go"), false),
    ("python.py", include_str!("../../../lib/tests/fixtures/symbols/python.py"), false),
    ("java.java", include_str!("../../../lib/tests/fixtures/symbols/java.java"), false),
    ("c_sharp.cs", include_str!("../../../lib/tests/fixtures/symbols/c_sharp.cs"), false),
    ("c.c", include_str!("../../../lib/tests/fixtures/symbols/c.c"), false),
    ("cpp.cpp", include_str!("../../../lib/tests/fixtures/symbols/cpp.cpp"), false),
    ("swift.swift", include_str!("../../../lib/tests/fixtures/symbols/swift.swift"), false),
    ("scala.scala", include_str!("../../../lib/tests/fixtures/symbols/scala.scala"), false),
    ("php.php", include_str!("../../../lib/tests/fixtures/symbols/php.php"), false),
    ("perl.pl", include_str!("../../../lib/tests/fixtures/symbols/perl.pl"), false),
    ("bash.sh", include_str!("../../../lib/tests/fixtures/symbols/bash.sh"), false),
    ("zsh.zsh", include_str!("../../../lib/tests/fixtures/symbols/zsh.zsh"), false),
    ("lua.lua", include_str!("../../../lib/tests/fixtures/symbols/lua.lua"), false),
];

/// Languages whose occurrences can be `Scoped` to a declaration.
const RESOLVING: [&str; 3] = ["rust.rs", "typescript.ts", "javascript.js"];

/// A well-formed ID no declaration has.
const PHANTOM_ID: &str = "0000000000000000000000000000000000000000000000000000000000000000";

/// One identity edit of the evidence matrix: `None` when the entry has no
/// site for it (the reason is the row's business, asserted by the test).
type IdentityEdit = (&'static str, fn(&mut Value) -> Option<()>);

fn occurrences_mut(entry: &mut Value) -> &mut Vec<Value> {
    entry["occurrences"].as_array_mut().expect("occurrences")
}

const IDENTITY_EDITS: [IdentityEdit; 5] = [
    // A copy of the last region, owned by a declaration no record is: order
    // and spans stay valid.
    ("phantom coverage owner", |entry| {
        let regions = entry["reference_coverage"].as_array_mut().expect("coverage");
        let mut phantom = regions.last()?.clone();
        phantom["owner"] = json!({ "id": PHANTOM_ID, "name": "phantom", "qualified_name": "phantom" });
        regions.push(phantom);
        Some(())
    }),
    ("phantom candidate ID", |entry| {
        let occurrence = occurrences_mut(entry).iter_mut().find(|occurrence| occurrence["confidence"] == "NameOnly")?;
        occurrence["candidates"][0]["id"] = json!(PHANTOM_ID);
        Some(())
    }),
    ("phantom ownerless established target ID", |entry| {
        let occurrence = occurrences_mut(entry).iter_mut().find(|occurrence| {
            occurrence["confidence"] == "Scoped" && occurrence["owner"].is_null() && occurrence["target"]["id"].is_string()
        })?;
        occurrence["target"]["id"] = json!(PHANTOM_ID);
        Some(())
    }),
    // Control: the target an edge is derived from.
    ("phantom named-owner target ID", |entry| {
        let occurrence = occurrences_mut(entry).iter_mut().find(|occurrence| {
            occurrence["confidence"] == "Scoped" && occurrence["owner"].is_object() && occurrence["target"]["id"].is_string()
        })?;
        occurrence["target"]["id"] = json!(PHANTOM_ID);
        Some(())
    }),
    // A named callable's region no occurrence names: removing it leaves the
    // rest of the evidence consistent.
    ("removed empty named callable region", |entry| {
        let owning: std::collections::HashSet<Value> = occurrences_mut(entry)
            .iter()
            .map(|occurrence| occurrence["owner"]["id"].clone())
            .collect();
        let regions = entry["reference_coverage"].as_array_mut().expect("coverage");
        let at = regions
            .iter()
            .position(|region| region["owner"].is_object() && !owning.contains(&region["owner"]["id"]))?;
        regions.remove(at);
        Some(())
    }),
];

/// The lookup the `--verbose` stats line reports (a refused entry's reason
/// is printed above it).
fn lookup_of(stderr: &[u8]) -> &'static str {
    let stderr = String::from_utf8_lossy(stderr);
    let stats = |counts: &str| stderr.lines().any(|line| line.starts_with(counts));
    if stats("cache: 1 hits, 0 partial hits, 0 misses; parsed 0 files") {
        "hit"
    } else if stats("cache: 0 hits, 0 partial hits, 1 misses; parsed 1 files") {
        "miss"
    } else {
        panic!("unexpected stats line: {stderr}")
    }
}

/// An every-kind entry omits no declaration, so every ID its evidence holds
/// (occurrence targets and candidates, owners, coverage owners) names a
/// retained record, and every named callable with a body owns its region,
/// Python and Lua included: in every language, each edit of a tool-written
/// entry below is a miss whose output equals an uncached run, and the
/// unedited entry is a hit. Scala's primary fixture has no named callable
/// without occurrences; only the resolving languages have `Scoped` targets.
#[test]
fn every_kind_evidence_identities_name_retained_declarations_in_every_language() {
    let sources: Vec<(&str, String)> = LANGUAGE_FIXTURES
        .iter()
        .map(|&(file, source, probe)| (file, if probe { format!("{source}{MEMBER_PROBE}") } else { source.to_string() }))
        .collect();
    let files: Vec<(&str, &str)> = sources.iter().map(|(file, source)| (*file, source.as_str())).collect();
    let scratch = Scratch::new(&files);
    let mut write_args = vec!["--json-full", "symbols"];
    write_args.extend(LANGUAGE_FIXTURES.iter().map(|(file, _, _)| *file));
    scratch.stdout(&write_args);
    let written: Vec<(String, std::path::PathBuf, Value)> = crate::cache_entries(scratch.temp.path())
        .into_iter()
        .map(|path| {
            let entry = read_json(&path);
            let file = Path::new(entry["file"].as_str().expect("entry file"))
                .file_name()
                .expect("file name")
                .to_string_lossy()
                .into_owned();
            let relative = path.strip_prefix(scratch.temp.path()).expect("under temp").to_path_buf();
            (file, relative, entry)
        })
        .collect();
    assert_eq!(written.len(), LANGUAGE_FIXTURES.len(), "one entry per language");

    let failures: Vec<String> = std::thread::scope(|scope| {
        let handles: Vec<_> = written
            .chunks(written.len().div_ceil(4))
            .map(|chunk| {
                let scratch = &scratch;
                scope.spawn(move || {
                    let temp = TempDir::new().expect("cell temp");
                    let mut failures = Vec::new();
                    for (file, relative, original) in chunk {
                        let args = ["--verbose", "--json-full", "symbols", file.as_str()];
                        let uncached = json_of(&scratch.stdout(&["--json-full", "symbols", "--no-cache", file.as_str()]));
                        let serve = |entry: &Value| {
                            let path = temp.path().join(relative);
                            std::fs::create_dir_all(path.parent().expect("entry parent")).expect("create shard");
                            std::fs::write(&path, serde_json::to_vec(entry).expect("serialize")).expect("write entry");
                            let output = scratch.hug_with_temp(temp.path(), &args);
                            let lookup = lookup_of(&output.stderr);
                            (lookup, json_of(&stdout_of(output, &args)))
                        };
                        let (lookup, output) = serve(original);
                        if lookup != "hit" || output != uncached {
                            failures.push(format!("{file}: unedited: {lookup}, equal output: {}", output == uncached));
                        }
                        for (label, edit) in IDENTITY_EDITS {
                            let mut edited = original.clone();
                            let applicable = edit(&mut edited).is_some();
                            let expected = match label {
                                "phantom ownerless established target ID" | "phantom named-owner target ID" => {
                                    RESOLVING.contains(&file.as_str())
                                }
                                "removed empty named callable region" => file != "scala.scala",
                                _ => true,
                            };
                            if applicable != expected {
                                failures.push(format!("{file}: {label}: site present {applicable}, expected {expected}"));
                                continue;
                            }
                            if !applicable {
                                continue;
                            }
                            let (lookup, output) = serve(&edited);
                            if lookup != "miss" {
                                failures.push(format!("{file}: {label}: expected a miss, entry was served"));
                            } else if output != uncached {
                                failures.push(format!("{file}: {label}: missed, but output differs from an uncached run"));
                            }
                        }
                    }
                    failures
                })
            })
            .collect();
        handles.into_iter().flat_map(|handle| handle.join().expect("language thread")).collect()
    });
    assert!(failures.is_empty(), "every-kind identity matrix:\n{}", failures.join("\n"));
}

/// What each language's primary fixture gains for the source-fact edits:
/// two same-named members, functions or prototypes and one reference of the
/// name (`cand`, `Cand` in Go and C#), so a `NameOnly` occurrence holds two
/// candidates; Scala also gains a named callable with no occurrences. Lua's
/// lines go before its final `return Widget`.
const CANDIDATE_PROBES: [(&str, &str); 16] = [
    ("rust.rs", "\nstruct CandA;\nimpl CandA { fn cand(&self) {} }\nstruct CandB;\nimpl CandB { fn cand(&self) {} }\nfn cand_probe(x: CandA) { x.cand(); }\n"),
    ("typescript.ts", "\nclass CandA { cand(): void {} }\nclass CandB { cand(): void {} }\nfunction candProbe(x: CandA) { x.cand(); }\n"),
    ("javascript.js", "\nclass CandA { cand() {} }\nclass CandB { cand() {} }\nfunction candProbe(x) { x.cand(); }\n"),
    ("go.go", "\ntype CandA struct{}\n\nfunc (CandA) Cand() {}\n\ntype CandB struct{}\n\nfunc (CandB) Cand() {}\n\nfunc candProbe(x CandA) { x.Cand() }\n"),
    ("python.py", "\n\nclass CandA:\n    def cand(self):\n        pass\n\n\nclass CandB:\n    def cand(self):\n        pass\n\n\ndef cand_probe(x):\n    x.cand()\n"),
    ("java.java", "\nclass CandHolder {\n    void cand() {}\n    void cand(int x) {}\n    void probe() { cand(); }\n}\n"),
    ("c_sharp.cs", "\nclass CandHolder\n{\n    void Cand() { }\n    void Cand(int x) { }\n    void Probe() { Cand(); }\n}\n"),
    ("c.c", "\nvoid cand(void);\nvoid cand(void);\nvoid cand_probe(void) { cand(); }\n"),
    ("cpp.cpp", "\nvoid cand(int);\nvoid cand(double);\nvoid cand_probe() { cand(1); }\n"),
    ("swift.swift", "\nfunc cand(_ x: Int) {}\nfunc cand(_ x: String) {}\nfunc candProbe() { cand(1) }\n"),
    ("scala.scala", "\nobject CandHolder {\n  def cand(x: Int) = {}\n  def cand(x: String) = {}\n  def probe() = cand(1)\n}\nobject CoverageControl { def empty() = {} }\n"),
    ("php.php", "\nclass CandA { function cand() {} }\nclass CandB { function cand() {} }\nfunction cand_probe($x) { $x->cand(); }\n"),
    ("perl.pl", "\npackage CandA;\nsub cand { }\npackage CandB;\nsub cand { }\npackage main;\ncand();\n"),
    ("bash.sh", "\ncand() { :; }\ncand() { :; }\ncand\n"),
    ("zsh.zsh", "\ncand() { :; }\ncand() { :; }\ncand\n"),
    ("lua.lua", "function cand() end\nfunction cand() end\ncand()\n\n"),
];

/// One source-fact edit of a tool-written entry and the refusal it must get.
type SourceFactEdit = (&'static str, &'static str, fn(&mut Value) -> Option<()>);

const SOURCE_FACT_EDITS: [SourceFactEdit; 4] = [
    // A named callable's region that owns no occurrence shrinks to the
    // record's name: its retained body is outside it.
    ("region", "a callable whose coverage region does not match its body", |entry| {
        let owning: std::collections::HashSet<Value> =
            occurrences_mut(entry).iter().map(|occurrence| occurrence["owner"]["id"].clone()).collect();
        let records = entry["records"].as_array().expect("records").clone();
        let regions = entry["reference_coverage"].as_array_mut().expect("coverage");
        let (region, name_span) = regions.iter_mut().find_map(|region| {
            let id = &region["owner"]["id"];
            if !id.is_string() || owning.contains(id) {
                return None;
            }
            let record = records.iter().find(|record| record["id"] == *id)?;
            let name_span = record["source"]["name_span"].clone();
            (name_span.is_object() && record["source"]["body_span"].is_object()).then_some((region, name_span))
        })?;
        region["span"] = name_span;
        Some(())
    }),
    // A written path the source never writes, on the first occurrence.
    ("path", "an occurrence whose written path is not the source text ending at its name", |entry| {
        let target = &mut occurrences_mut(entry).first_mut()?["target"];
        target["qualified_name"] = json!(format!("__never_written_review3__.{}", target["name"].as_str()?));
        Some(())
    }),
    // The probe's two candidates, reversed.
    ("candidates", "candidates out of declaration order", |entry| {
        let occurrence = occurrences_mut(entry).iter_mut().find(|occurrence| {
            matches!(occurrence["target"]["name"].as_str(), Some("cand" | "Cand"))
                && occurrence["confidence"] == "NameOnly"
                && occurrence["candidates"].as_array().is_some_and(|candidates| candidates.len() == 2)
        })?;
        occurrence["candidates"].as_array_mut()?.reverse();
        Some(())
    }),
    // A language without binding rules makes no edge: one fabricated between
    // two records is refused. Not applicable to Rust, TypeScript and
    // JavaScript, whose edges are checked by the other edits.
    ("fabricated edge", "a reference with no scoped occurrence", |entry| {
        if entry["language"].as_str().is_some_and(|language| ["Rust", "TypeScript", "JavaScript"].contains(&language)) {
            return None;
        }
        let records = entry["records"].as_array_mut().expect("records");
        let callables: Vec<usize> = records
            .iter()
            .enumerate()
            .filter(|(_, record)| matches!(record["kind"].as_str(), Some("Function" | "Method")))
            .map(|(idx, _)| idx)
            .take(2)
            .collect();
        let [from, to] = callables[..] else { return None };
        let reference = |record: &Value| {
            json!({ "id": record["id"], "name": record["identity"]["name"], "qualified_name": record["identity"]["qualified_name"] })
        };
        let (target, owner) = (reference(&records[to]), reference(&records[from]));
        // An empty summary list is not written.
        let mut push = |record: usize, list: &str, reference: Value| {
            let summary = &mut records[record]["relations"][list];
            if summary.is_null() {
                *summary = json!([]);
            }
            summary.as_array_mut().map(|summary| summary.push(reference))
        };
        push(from, "references", target)?;
        push(to, "referenced_by", owner)?;
        Some(())
    }),
];

/// The source facts an entry retains are checked against its source in
/// every language: a named callable's region shrunk to its name (outside its
/// retained body), an invented written path and reversed candidates are each
/// a miss whose output equals the unedited entry's (itself an actual hit
/// equal to an uncached run), with the refusal named on stderr; in the
/// thirteen languages without binding rules a fabricated summary edge is
/// refused too.
#[test]
fn retained_geometry_written_paths_and_candidate_order_are_checked_in_every_language() {
    let sources: Vec<(&str, String)> = LANGUAGE_FIXTURES
        .iter()
        .zip(CANDIDATE_PROBES)
        .map(|(&(file, source, _), (probe_file, probe))| {
            assert_eq!(file, probe_file);
            let source = match source.rsplit_once("return Widget") {
                Some((before, after)) if file == "lua.lua" => format!("{before}{probe}return Widget{after}"),
                _ => format!("{source}{probe}"),
            };
            (file, source)
        })
        .collect();
    let files: Vec<(&str, &str)> = sources.iter().map(|(file, source)| (*file, source.as_str())).collect();
    let scratch = Scratch::new(&files);
    let mut write_args = vec!["--json-full", "symbols"];
    write_args.extend(files.iter().map(|(file, _)| *file));
    scratch.stdout(&write_args);
    let written: Vec<(String, std::path::PathBuf, Value)> = crate::cache_entries(scratch.temp.path())
        .into_iter()
        .map(|path| {
            let entry = read_json(&path);
            let file = Path::new(entry["file"].as_str().expect("entry file"))
                .file_name()
                .expect("file name")
                .to_string_lossy()
                .into_owned();
            let relative = path.strip_prefix(scratch.temp.path()).expect("under temp").to_path_buf();
            (file, relative, entry)
        })
        .collect();
    assert_eq!(written.len(), LANGUAGE_FIXTURES.len(), "one entry per language");

    let failures: Vec<String> = std::thread::scope(|scope| {
        let handles: Vec<_> = written
            .chunks(written.len().div_ceil(4))
            .map(|chunk| {
                let scratch = &scratch;
                scope.spawn(move || {
                    let temp = TempDir::new().expect("cell temp");
                    let mut failures = Vec::new();
                    for (file, relative, original) in chunk {
                        let args = ["--verbose", "--json-full", "symbols", file.as_str()];
                        let uncached = json_of(&scratch.stdout(&["--json-full", "symbols", "--no-cache", file.as_str()]));
                        let serve = |entry: &Value| {
                            let path = temp.path().join(relative);
                            std::fs::create_dir_all(path.parent().expect("entry parent")).expect("create shard");
                            std::fs::write(&path, serde_json::to_vec(entry).expect("serialize")).expect("write entry");
                            let output = scratch.hug_with_temp(temp.path(), &args);
                            let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
                            (lookup_of(&output.stderr), stderr, json_of(&stdout_of(output, &args)))
                        };
                        let (lookup, _, served) = serve(original);
                        if lookup != "hit" || served != uncached {
                            failures.push(format!("{file}: unedited: {lookup}, equal output: {}", served == uncached));
                            continue;
                        }
                        for (label, reason, edit) in SOURCE_FACT_EDITS {
                            let mut edited = original.clone();
                            let applicable = edit(&mut edited).is_some();
                            let expected = label != "fabricated edge" || !RESOLVING.contains(&file.as_str());
                            if applicable != expected {
                                failures.push(format!("{file}: {label}: site present {applicable}, expected {expected}"));
                                continue;
                            }
                            if !applicable {
                                continue;
                            }
                            let (lookup, stderr, output) = serve(&edited);
                            if lookup != "miss" {
                                failures.push(format!("{file}: {label}: expected a miss, entry was served"));
                            } else if !stderr.contains(&format!("inconsistent entry: {reason}")) {
                                failures.push(format!("{file}: {label}: refused for another reason: {stderr}"));
                            } else if output != served {
                                failures.push(format!("{file}: {label}: missed, but output differs from the original"));
                            }
                        }
                    }
                    failures
                })
            })
            .collect();
        handles.into_iter().flat_map(|handle| handle.join().expect("language thread")).collect()
    });
    assert!(failures.is_empty(), "source-fact matrix:\n{}", failures.join("\n"));
}

/// One written path of the deleted-path matrix: the scratch file, what the
/// language's primary fixture gains (`None`: the fixture already writes it;
/// a `rust.rs` row without a fixture is the minimal source alone), the
/// occurrence's name, and its written path (`None`: the shell command
/// spelling, which stays whole in the name and has no path).
struct WrittenPathRow {
    file: &'static str,
    source: &'static str,
    probe: Option<&'static str>,
    name: &'static str,
    path: Option<&'static str>,
}

fn fixture_of(file: &str) -> &'static str {
    LANGUAGE_FIXTURES.iter().find(|(name, _, _)| *name == file).map(|&(_, source, _)| source).expect("a fixture")
}

/// R3-F3.S09r: one qualified occurrence per language (two forms in Rust and
/// Perl), plus the shell command-spelling controls.
fn written_path_rows() -> Vec<WrittenPathRow> {
    let row = |file, probe, name, path| WrittenPathRow { file, source: fixture_of(file), probe, name, path };
    vec![
        WrittenPathRow { file: "minimal.rs", source: "", probe: Some("fn g() { other::external(); }\n"), name: "external", path: Some("other::external") },
        row("rust.rs", None, "outer_module", Some("crate::outer_module")),
        row("typescript.ts", Some("\nfunction s09rProbe() { other.external(); }\n"), "external", Some("other.external")),
        row("javascript.js", Some("\nfunction s09rProbe() { other.external(); }\n"), "external", Some("other.external")),
        row("go.go", None, "Sprint", Some("fmt.Sprint")),
        row("python.py", None, "assigned_through_self", Some("self.assigned_through_self")),
        // The fixtures' `package com.example;` and `namespace Example.Symbols`
        // declare their components, so the probes write a qualified call.
        row("java.java", Some("\nclass S09rProbe { void probe() { Other.external(); } }\n"), "external", Some("Other.external")),
        row("c_sharp.cs", Some("\nclass S09rProbe { void Probe() { Other.External(); } }\n"), "External", Some("Other.External")),
        row("c.c", Some("\nvoid s09r_probe(void) { o.external(); }\n"), "external", Some("o.external")),
        row("cpp.cpp", None, "deprecated", Some("gnu::deprecated")),
        row("swift.swift", Some("\nfunc s09rProbe() { Other.external() }\n"), "external", Some("Other.external")),
        row("scala.scala", Some("\nobject S09rProbe { def probe() = Other.external() }\n"), "external", Some("Other.external")),
        row("php.php", None, "privateField", Some("$this->privateField")),
        row("lua.lua", Some("other.external()\n\n"), "external", Some("other.external")),
        row("perl.pl", Some("\nsub s09r_probe { R4::r4f(); }\n"), "r4f", Some("R4::r4f")),
        WrittenPathRow { file: "receiver.pl", source: fixture_of("perl.pl"), probe: Some("\nsub s09r_probe { R4->r4f(); }\n"), name: "r4f", path: Some("R4->r4f") },
        row("bash.sh", Some("\nother/external\n"), "other/external", None),
        row("zsh.zsh", Some("\nother/external\n"), "other/external", None),
    ]
}

/// The stored occurrence of `name` whose written path is `path`.
fn written_occurrence<'a>(entry: &'a mut Value, name: &str, path: Option<&str>) -> Option<&'a mut Value> {
    occurrences_mut(entry)
        .iter_mut()
        .find(|occurrence| occurrence["target"]["name"] == name && occurrence["target"]["qualified_name"].as_str() == path)
}

/// Whether the served output has an occurrence of `name` written as `path`.
fn serves_path(output: &Value, name: &str, path: Option<&str>) -> bool {
    output["indexes"][0]["occurrences"].as_array().is_some_and(|occurrences| {
        occurrences.iter().any(|occurrence| occurrence["target"]["name"] == name && occurrence["target"]["qualified_name"].as_str() == path)
    })
}

/// R3-F3.S09r: in every language, deleting a written path from a tool-written
/// entry (`target.qualified_name` set to `null`), or deleting only its
/// cache-only start (`path_start_byte`), is a miss naming the disagreement
/// whose recomputed output keeps the path and equals the unedited entry's
/// (itself an actual hit, equal to an uncached run, serving the path). The
/// stored start is where the path starts in the source. Bash and Zsh keep a
/// path command's whole spelling in the name: no path, no start, a hit.
#[test]
fn deleted_written_paths_are_recomputed_in_every_language() {
    let rows = written_path_rows();
    let sources: Vec<(&str, String)> = rows
        .iter()
        .map(|row| {
            let probe = row.probe.unwrap_or("");
            let source = match row.source.rsplit_once("return Widget") {
                Some((before, after)) if row.file == "lua.lua" => format!("{before}{probe}return Widget{after}"),
                _ => format!("{}{probe}", row.source),
            };
            (row.file, source)
        })
        .collect();
    let files: Vec<(&str, &str)> = sources.iter().map(|(file, source)| (*file, source.as_str())).collect();
    let scratch = Scratch::new(&files);
    let failures: Vec<String> = std::thread::scope(|scope| {
        let handles: Vec<_> = rows
            .chunks(rows.len().div_ceil(4))
            .map(|chunk| {
                let (scratch, sources) = (&scratch, &sources);
                scope.spawn(move || {
                    let mut failures = Vec::new();
                    for row in chunk {
                        let file = row.file;
                        let source = &sources.iter().find(|(name, _)| *name == file).expect("source").1;
                        let temp = TempDir::new().expect("row temp");
                        let args = ["--verbose", "--json-full", "symbols", file];
                        let uncached = json_of(&scratch.stdout(&["--json-full", "symbols", "--no-cache", file]));
                        let written_run = scratch.hug_with_temp(temp.path(), &args);
                        assert_eq!(lookup_of(&written_run.stderr), "miss", "{file}: the writing run");
                        let entry = only_entry(temp.path());
                        let original = read_json(&entry);
                        let serve = |edited: &Value| {
                            std::fs::write(&entry, serde_json::to_vec(edited).expect("serialize")).expect("write entry");
                            let output = scratch.hug_with_temp(temp.path(), &args);
                            let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
                            (lookup_of(&output.stderr), stderr, json_of(&stdout_of(output, &args)))
                        };
                        let (lookup, _, served) = serve(&original);
                        if lookup != "hit" || served != uncached || !serves_path(&served, row.name, row.path) {
                            failures.push(format!(
                                "{file}: unedited: {lookup}, equal output: {}, path served: {}",
                                served == uncached,
                                serves_path(&served, row.name, row.path)
                            ));
                            continue;
                        }
                        let mut probe = original.clone();
                        let Some(occurrence) = written_occurrence(&mut probe, row.name, row.path) else {
                            failures.push(format!("{file}: no stored `{}` written {:?}", row.name, row.path));
                            continue;
                        };
                        let start = occurrence["path_start_byte"].as_u64().map(|start| start as usize);
                        let end = occurrence["span"]["end_byte"].as_u64().expect("end byte") as usize;
                        if start.map(|start| &source[start..end]) != row.path {
                            failures.push(format!("{file}: stored start {start:?} does not start {:?}", row.path));
                            continue;
                        }
                        if row.path.is_none() {
                            continue;
                        }
                        for (label, key) in [("path", None), ("start", Some("path_start_byte"))] {
                            let mut edited = original.clone();
                            let occurrence = written_occurrence(&mut edited, row.name, row.path).expect("the occurrence");
                            match key {
                                Some(key) => occurrence[key] = Value::Null,
                                None => occurrence["target"]["qualified_name"] = Value::Null,
                            }
                            let (lookup, stderr, output) = serve(&edited);
                            if lookup != "miss" {
                                failures.push(format!("{file}: {label} deleted: expected a miss, entry was served"));
                            } else if !stderr.contains(&format!("inconsistent entry: {PATH_START}")) {
                                failures.push(format!("{file}: {label} deleted: refused for another reason: {stderr}"));
                            } else if output != served || !serves_path(&output, row.name, row.path) {
                                failures.push(format!("{file}: {label} deleted: the recomputed output lost the path"));
                            }
                        }
                    }
                    failures
                })
            })
            .collect();
        handles.into_iter().flat_map(|handle| handle.join().expect("language thread")).collect()
    });
    assert!(failures.is_empty(), "deleted written path matrix:\n{}", failures.join("\n"));
}

/// The full shape column of a qualified name that reaches no member
/// (`fn g() { other::external(); }`): its written path and its stored start,
/// each unedited (a hit serving the path), absent, `null`, of the wrong type,
/// empty, repeated or followed by trailing content (each a miss whose output
/// equals an uncached run and serves the path).
#[test]
fn a_qualified_nonmember_path_and_its_start_are_served_only_unedited() {
    const SOURCE: &str = "fn g() { other::external(); }\nfn module_function() {}\n";
    let scratch = Scratch::new(&[("qualified.rs", SOURCE)]);
    let args = ["--json-full", "symbols", "qualified.rs"];
    let uncached = json_of(&scratch.stdout(&["--json-full", "symbols", "--no-cache", "qualified.rs"]));
    assert!(serves_path(&uncached, "external", Some("other::external")));
    assert_eq!(json_of(&scratch.stdout(&args)), uncached, "the cache-writing run");
    let entry = only_entry(scratch.temp.path());
    let relative_entry = entry.strip_prefix(scratch.temp.path()).expect("under temp").to_path_buf();
    let original = read_json(&entry);
    let at = original["occurrences"]
        .as_array()
        .expect("occurrences")
        .iter()
        .position(|occurrence| occurrence["target"]["qualified_name"] == "other::external")
        .expect("the qualified call");
    assert_eq!(at, 1, "the qualified call follows its qualifier `other`");
    let columns = [
        evidence_scalar("qualified target.qualified_name", &["occurrences", "1", "target", "qualified_name"], json!("")),
        evidence_scalar("qualified path_start_byte", &["occurrences", "1", "path_start_byte"], json!(0)),
    ];
    let failures: Vec<String> = columns
        .iter()
        .flat_map(|column| {
            let base = field_base(original.clone(), column);
            check_column(&scratch, &args, &base, &uncached, &relative_entry, column)
        })
        .collect();
    assert!(failures.is_empty(), "qualified nonmember path matrix:\n{}", failures.join("\n"));
}

/// `g` references `f`, `h` and the imports `a`, `b` in that order, `f` is
/// referenced by `g` then `j`, `p` calls the member `x.m()` (two candidates)
/// in a region limited for two reasons, in each resolving language.
const SUMMARY_SOURCES: [(&str, &str); 3] = [
    (
        "summaries.rs",
        "use other::a;\nuse other::b;\nfn f() {}\nfn h() {}\nfn g() { f(); h(); a(); b(); }\nfn j() { f(); }\n\
         struct K;\nimpl K { fn m(&self) {} }\nstruct L;\nimpl L { fn m(&self) {} }\n\
         fn p(x: K) { x.m(); println!(\"{}\", 1); }\n",
    ),
    (
        "summaries.ts",
        "import { a, b } from \"./other\";\nfunction f(): void {}\nfunction h(): void {}\n\
         function g(): void { f(); h(); a(); b(); }\nfunction j(): void { f(); }\n\
         class K { m(): void {} }\nclass L { m(): void {} }\nfunction p(x: K): void { x.m(); eval(\"1\"); }\n",
    ),
    (
        "summaries.js",
        "import { a, b } from \"./other\";\nfunction f() {}\nfunction h() {}\nfunction g() { f(); h(); a(); b(); }\n\
         function j() { f(); }\nclass K { m() {} }\nclass L { m() {} }\nfunction p(x) { x.m(); eval(\"1\"); }\n",
    ),
];

fn summary_mut<'a>(entry: &'a mut Value, name: &str, list: &str) -> &'a mut Vec<Value> {
    record_mut(entry, name)["relations"][list].as_array_mut().expect("summary list")
}

fn member_call(entry: &mut Value) -> &mut Value {
    occurrences_mut(entry).iter_mut().find(|occurrence| occurrence["target"]["name"] == "m").expect("`x.m()`")
}

fn limited_region(entry: &mut Value) -> &mut Value {
    entry["reference_coverage"]
        .as_array_mut()
        .expect("coverage")
        .iter_mut()
        .find(|region| region["owner"]["name"] == "p")
        .expect("`p` region")
}

fn repeat_first(list: &mut Vec<Value>) {
    list.insert(1, list[0].clone());
}

const REPEATED_EDGE: &str = "a reference summary that repeats an edge";
const EDGE_ORDER: &str = "a reference summary out of first-occurrence order";
const OCCURRENCE_ORDER: &str = "occurrences out of order or repeated";
const REASONS: &str = "coverage reasons that do not match its state";
const WRITTEN_PATH: &str = "an occurrence whose written path is not the source text ending at its name";
const PATH_START: &str = "an occurrence whose written path disagrees with its stored start";

/// One labeled list or path edit (site ID, mutation) and the refusal it gets.
type ListEdit = (&'static str, &'static str, fn(&mut Value));

const LIST_EDITS: &[ListEdit] = &[
    ("S01 duplicate", REPEATED_EDGE, |entry| repeat_first(summary_mut(entry, "g", "references"))),
    ("S01 reverse", EDGE_ORDER, |entry| summary_mut(entry, "g", "references").reverse()),
    ("S02 duplicate", REPEATED_EDGE, |entry| repeat_first(summary_mut(entry, "f", "referenced_by"))),
    ("S02 reverse", EDGE_ORDER, |entry| summary_mut(entry, "f", "referenced_by").reverse()),
    ("S03 duplicate", REPEATED_EDGE, |entry| repeat_first(summary_mut(entry, "g", "dependencies"))),
    ("S03 reverse", EDGE_ORDER, |entry| summary_mut(entry, "g", "dependencies").reverse()),
    ("S04 duplicate", "an occurrence whose target ID and candidates contradict its confidence", |entry| {
        repeat_first(member_call(entry)["candidates"].as_array_mut().expect("candidates"));
    }),
    ("S04 reverse", "candidates out of declaration order", |entry| {
        member_call(entry)["candidates"].as_array_mut().expect("candidates").reverse();
    }),
    ("S05 duplicate", OCCURRENCE_ORDER, |entry| repeat_first(occurrences_mut(entry))),
    ("S05 reverse", OCCURRENCE_ORDER, |entry| occurrences_mut(entry).reverse()),
    ("S06 duplicate", "a repeated coverage region", |entry| {
        let regions = entry["reference_coverage"].as_array_mut().expect("coverage");
        let last = regions.last().expect("a region").clone();
        regions.push(last);
    }),
    // The file region stays first; the rest are reversed.
    ("S06 reverse", "coverage out of order", |entry| {
        entry["reference_coverage"].as_array_mut().expect("coverage")[1..].reverse();
    }),
    ("S07 duplicate", REASONS, |entry| repeat_first(limited_region(entry)["reasons"].as_array_mut().expect("reasons"))),
    ("S07 reverse", REASONS, |entry| limited_region(entry)["reasons"].as_array_mut().expect("reasons").reverse()),
    ("S09 null path on a member call", PATH_START, |entry| {
        member_call(entry)["target"]["qualified_name"] = Value::Null;
    }),
    ("S10 empty path on a member call", WRITTEN_PATH, |entry| member_call(entry)["target"]["qualified_name"] = json!("")),
    ("S10 empty path on an unqualified call", WRITTEN_PATH, |entry| {
        let call = occurrences_mut(entry).iter_mut().find(|occurrence| occurrence["target"]["name"] == "f").expect("`f()`");
        assert!(call["target"]["qualified_name"].is_null(), "an unqualified call has no written path");
        call["target"]["qualified_name"] = json!("");
    }),
];

/// Summaries list each edge once in first-occurrence order, and the other
/// evidence lists keep their canonical order without repeats, in Rust,
/// TypeScript and JavaScript: each duplicate or reversal (and an empty or
/// lost written path) is a miss naming its refusal whose output equals the
/// unedited entry's, itself an actual hit equal to an uncached run.
#[test]
fn summary_and_evidence_list_edits_are_recomputed_in_every_resolving_language() {
    let scratch = Scratch::new(&SUMMARY_SOURCES);
    let failures: Vec<String> = std::thread::scope(|scope| {
        let handles: Vec<_> = SUMMARY_SOURCES
            .iter()
            .map(|&(file, _)| {
                let scratch = &scratch;
                scope.spawn(move || {
                    let temp = TempDir::new().expect("cell temp");
                    let args = ["--verbose", "--json-full", "symbols", file];
                    let uncached = json_of(&scratch.stdout(&["--json-full", "symbols", "--no-cache", file]));
                    let written_run = scratch.hug_with_temp(temp.path(), &args);
                    assert_eq!(lookup_of(&written_run.stderr), "miss", "{file}: the writing run");
                    let entry = only_entry(temp.path());
                    let original = read_json(&entry);
                    let serve = |edited: &Value| {
                        std::fs::write(&entry, serde_json::to_vec(edited).expect("serialize")).expect("write entry");
                        let output = scratch.hug_with_temp(temp.path(), &args);
                        let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
                        (lookup_of(&output.stderr), stderr, json_of(&stdout_of(output, &args)))
                    };
                    let mut failures = Vec::new();
                    let (lookup, _, served) = serve(&original);
                    if lookup != "hit" || served != uncached {
                        return vec![format!("{file}: unedited: {lookup}, equal output: {}", served == uncached)];
                    }
                    let summary = |name: &str, list: &str| -> Vec<Value> {
                        record(&served, name)["relations"][list]
                            .as_array()
                            .map(|list| list.iter().map(|reference| reference["name"].clone()).collect())
                            .unwrap_or_default()
                    };
                    assert_eq!(summary("g", "references"), ["f", "h", "a", "b"], "{file}");
                    assert_eq!(summary("g", "dependencies"), ["a", "b"], "{file}");
                    assert_eq!(summary("f", "referenced_by"), ["g", "j"], "{file}");
                    for &(label, reason, edit) in LIST_EDITS {
                        let mut edited = original.clone();
                        edit(&mut edited);
                        let (lookup, stderr, output) = serve(&edited);
                        if lookup != "miss" {
                            failures.push(format!("{file}: {label}: expected a miss, entry was served"));
                        } else if !stderr.contains(&format!("inconsistent entry: {reason}")) {
                            failures.push(format!("{file}: {label}: refused for another reason: {stderr}"));
                        } else if output != served {
                            failures.push(format!("{file}: {label}: missed, but output differs from the original"));
                        }
                    }
                    failures
                })
            })
            .collect();
        handles.into_iter().flat_map(|handle| handle.join().expect("language thread")).collect()
    });
    assert!(failures.is_empty(), "summary and list matrix:\n{}", failures.join("\n"));
}
