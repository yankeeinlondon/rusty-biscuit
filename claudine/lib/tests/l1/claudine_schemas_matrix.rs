//! The input robustness matrix for Claudine's schema bundle read from disk
//! (`2026-10-21-claudine-schemas`, plan "Input robustness matrix" and the
//! implementation log's finalized table).
//!
//! Each column is one load-bearing field of a bundle file; each row is one
//! malformed shape of it. A cell starts from the unedited shipped bundle,
//! makes one edit to that field, writes the bundle in one syntax (block YAML,
//! or JSON syntax inside the same `.yaml` files), and asserts the public
//! result: trigger discovery or payload resolution fails visibly, or a pair of
//! probe documents gets the stated verdicts or trigger selections. One test
//! walks one column in one syntax; `matrix_tests_cover_the_table` keeps the
//! generated tests equal to the table. `claudine_schemas_embedded.rs` walks
//! the same table with the edited bundle supplied as an embedded bundle.

use biscuit_file::Yaml;
use claudine::composition::schema::bundle::{CLAUDINE_SCHEMA_BUNDLE_NAME, CLAUDINE_TRIGGER_FILES};
use darkmatter::markdown::schemas::SchemaError;
use serde_json::{Map, Value, json};

use crate::claudine_schemas::{BundleFile, Discovered, SchemaRepo, embedded_bundle, shipped_bundle};

/// Where a cell's bundle is read from.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum Origin {
    /// The repository's `schemas/` folder.
    Disk,
    /// An embedded bundle named like Claudine's, with no disk schema files.
    /// A load failure must be `SchemaError::EmbeddedTriggerLoad` (never the
    /// disk origin's `TriggerLoad`) naming a trigger file, whose payload
    /// imports the libraries, or the edited file itself: every bundle
    /// document is parsed to classify it, so a library that is not YAML is
    /// named directly.
    Embedded,
}

/// The syntax a cell's bundle is written in.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum Syntax {
    /// Block YAML. The unedited control is the shipped text itself.
    Yaml,
    /// JSON syntax on one line, still in the `.yaml` files.
    Json,
}

/// The malformed shapes, in table order.
#[derive(Clone, Copy, Debug, PartialEq)]
enum Shape {
    Absent,
    Null,
    WrongTypeWhole,
    WrongTypeOneElement,
    WrongTypeEveryElement,
    Empty,
    Duplicate,
    Trailing,
}

const SHAPES: [Shape; 8] = [
    Shape::Absent,
    Shape::Null,
    Shape::WrongTypeWhole,
    Shape::WrongTypeOneElement,
    Shape::WrongTypeEveryElement,
    Shape::Empty,
    Shape::Duplicate,
    Shape::Trailing,
];

/// What a cell must observe.
#[derive(Clone, Copy, Debug, PartialEq)]
enum Outcome {
    /// Discovery or payload resolution fails visibly; nothing is validated.
    Error,
    /// The bundle loads; the positive and violating probes are valid (`true`)
    /// or invalid (`false`).
    Verdicts(bool, bool),
    /// The bundle loads; the positive and violating probes select the
    /// column's trigger (`true`) or not (`false`).
    Selects(bool, bool),
}

use Outcome::{Error, Selects, Verdicts};

/// The control verdicts: the positive probe is valid, the violating one not.
const CONTROL: Outcome = Verdicts(true, false);

/// One load-bearing field: the file it lives in, the JSON pointer of the
/// value the shapes are applied to, its probes, and its outcome per shape.
struct Column {
    name: &'static str,
    file: &'static str,
    pointer: &'static str,
    /// The positive and violating probe documents' frontmatter.
    probes: (&'static str, &'static str),
    /// Probes judge trigger selection (of this file's trigger) rather than
    /// validity.
    selection: bool,
    outcomes: [Outcome; 8],
}

const ACTION_PROBES: (&str, &str) = ("start: [{ info: hi }]", "start: [{ inf0: hi }]");
const LIFECYCLE_PROBES: (&str, &str) = (
    "start: [{ when: \"a\", then: [{ when: \"b\", then: [{ when: \"c\", then: [stop] }] }] }]",
    "initialize: [{ shell: x }]",
);

/// The field no longer constrains the probes: both are valid.
const UNCONSTRAINED: Outcome = Verdicts(true, true);

/// `null` defines the `null` type (implementation-log ruling R2): it loads,
/// and rejects every non-null probe value.
const NULL_REJECTS_BOTH: Outcome = Verdicts(false, false);

/// Every mapping-valued export: a missing export, a non-definition, one or
/// every invalid property definition, a duplicated key, and broken syntax
/// fail visibly. `null` defines the `null` type and `{}` an empty closed
/// object, so both load; the probes then pass through the other arms.
const MAPPING_EXPORT: [Outcome; 8] = [Error, CONTROL, Error, Error, Error, CONTROL, Error, Error];

/// The table. Where a cell differs from the plan's generic outcome, the
/// implementation log records why.
const COLUMNS: &[Column] = &[
    // Library envelope, through `action.yaml`.
    Column {
        name: "library_kind",
        file: "action.yaml",
        pointer: "/kind",
        probes: ACTION_PROBES,
        selection: false,
        outcomes: [Error; 8],
    },
    Column {
        name: "library_types",
        file: "action.yaml",
        pointer: "/types",
        probes: ACTION_PROBES,
        selection: false,
        outcomes: [Error; 8],
    },
    // Every `action.yaml` export.
    Column {
        name: "action_communicate",
        file: "action.yaml",
        pointer: "/types/communicate",
        probes: ACTION_PROBES,
        selection: false,
        outcomes: MAPPING_EXPORT,
    },
    Column {
        name: "action_flow",
        file: "action.yaml",
        pointer: "/types/flow",
        probes: ACTION_PROBES,
        selection: false,
        outcomes: MAPPING_EXPORT,
    },
    Column {
        name: "action_shell",
        file: "action.yaml",
        pointer: "/types/shell",
        probes: ACTION_PROBES,
        selection: false,
        outcomes: MAPPING_EXPORT,
    },
    Column {
        name: "action_expression",
        file: "action.yaml",
        pointer: "/types/expression",
        probes: ACTION_PROBES,
        selection: false,
        outcomes: MAPPING_EXPORT,
    },
    Column {
        name: "action_bundle",
        file: "action.yaml",
        pointer: "/types/bundle",
        probes: ACTION_PROBES,
        selection: false,
        outcomes: MAPPING_EXPORT,
    },
    Column {
        name: "action_initialize_bundle",
        file: "action.yaml",
        pointer: "/types/initialize-bundle",
        probes: ACTION_PROBES,
        selection: false,
        outcomes: MAPPING_EXPORT,
    },
    // Every `lifecycle.yaml` export.
    Column {
        name: "lifecycle_stack",
        file: "lifecycle.yaml",
        pointer: "/types/stack",
        probes: LIFECYCLE_PROBES,
        selection: false,
        outcomes: [Error, NULL_REJECTS_BOTH, Error, Error, Error, Error, Error, Error],
    },
    // `initialize-stack: null` rejects the violating `initialize` list, while
    // the positive probe's `start` stack does not use it.
    Column {
        name: "lifecycle_initialize_stack",
        file: "lifecycle.yaml",
        pointer: "/types/initialize-stack",
        probes: LIFECYCLE_PROBES,
        selection: false,
        outcomes: [Error, CONTROL, Error, Error, Error, Error, Error, Error],
    },
    Column {
        name: "lifecycle_loop",
        file: "lifecycle.yaml",
        pointer: "/types/loop",
        probes: LIFECYCLE_PROBES,
        selection: false,
        outcomes: MAPPING_EXPORT,
    },
    // A `linking.yaml` export.
    Column {
        name: "linking_user_invocable",
        file: "linking.yaml",
        pointer: "/types/user-invocable",
        probes: ("prompt: x\nallowed-tools: [Read]\nuser-invocable: true", "prompt: x\nuser-invocable: [1]"),
        selection: false,
        outcomes: [Error, NULL_REJECTS_BOTH, Error, Error, Error, Error, Error, Error],
    },
    // Trigger envelope, through `claudine.yaml`. A file whose `kind` is not
    // `trigger-schema` is unclassified and contributes nothing, so the
    // violating probe is no longer judged.
    Column {
        name: "trigger_kind",
        file: "claudine.yaml",
        pointer: "/kind",
        probes: ACTION_PROBES,
        selection: false,
        outcomes: [UNCONSTRAINED, UNCONSTRAINED, UNCONSTRAINED, UNCONSTRAINED, UNCONSTRAINED, UNCONSTRAINED, Error, Error],
    },
    Column {
        name: "trigger_match",
        file: "claudine.yaml",
        pointer: "/match",
        probes: ACTION_PROBES,
        selection: false,
        outcomes: [Error; 8],
    },
    Column {
        name: "trigger_payload",
        file: "claudine.yaml",
        pointer: "/$schema",
        probes: ACTION_PROBES,
        selection: false,
        outcomes: [Error, Error, Error, Error, Error, UNCONSTRAINED, Error, Error],
    },
    // A trigger condition operand: the `start: string(required)` arm.
    Column {
        name: "trigger_condition_operand",
        file: "claudine.yaml",
        pointer: "/match/any/5/start",
        probes: ("start: hi", "start: null"),
        selection: true,
        outcomes: [Error; 8],
    },
    // A trigger combinator: inline-compose's `none:` child. Absent leaves an
    // empty `{}` condition inside `all:`, which the match grammar rejects.
    Column {
        name: "trigger_combinator",
        file: "inline-compose.yaml",
        pointer: "/match/all/1/none",
        probes: ("prompt: x", "prompt: x\nsequence: 42"),
        selection: true,
        outcomes: [Error; 8],
    },
    // A payload property definition. Removing it only removes the
    // constraint; an explicit `{}` payload is likewise an empty mergeable
    // payload.
    Column {
        name: "payload_property",
        file: "claudine.yaml",
        pointer: "/$schema/timeout",
        probes: ("start: [stop]\ntimeout: \"30s\"", "start: [stop]\ntimeout: [1]"),
        selection: false,
        outcomes: [UNCONSTRAINED, NULL_REJECTS_BOTH, Error, Error, Error, Error, Error, Error],
    },
];

/// A key whose line a duplicate cell rewrites into a repeat of the edited
/// key, after serialization.
const DUPLICATE_MARK: &str = "__duplicate_mark__";

/// The parent of `pointer` and its last segment.
fn split_pointer(pointer: &str) -> (&str, String) {
    let (parent, last) = pointer.rsplit_once('/').expect("a pointer below the root");
    (parent, last.replace("~1", "/").replace("~0", "~"))
}

/// Applies `shape` to the value at `pointer` in `document`. Returns the key
/// to repeat for a duplicate cell. A trailing cell edits nothing here; its
/// garbage is appended to the serialized text.
fn edit(document: &mut Value, pointer: &str, shape: Shape) -> Option<(String, Value)> {
    let (parent_pointer, key) = split_pointer(pointer);
    let parent = document.pointer_mut(parent_pointer).expect("the column's parent exists");
    let target = match parent {
        Value::Object(map) => map.get(&key).cloned().expect("the column's field exists"),
        Value::Array(items) => items[key.parse::<usize>().expect("an index")].clone(),
        _ => panic!("{pointer} has a scalar parent"),
    };
    let replacement = match shape {
        Shape::Absent => {
            match parent {
                Value::Object(map) => {
                    map.remove(&key);
                }
                Value::Array(items) => {
                    items.remove(key.parse::<usize>().expect("an index"));
                }
                _ => unreachable!(),
            }
            return None;
        }
        Shape::Null => Value::Null,
        Shape::WrongTypeWhole => json!(42),
        Shape::WrongTypeOneElement => wrong_elements(&target, false),
        Shape::WrongTypeEveryElement => wrong_elements(&target, true),
        Shape::Empty => match target {
            Value::Object(_) => json!({}),
            Value::Array(_) => json!([]),
            _ => json!(""),
        },
        Shape::Duplicate => {
            // A repeated key sits beside the original; a list element has
            // no key, so the list's own key is repeated instead.
            return match parent {
                Value::Object(map) => {
                    map.insert(DUPLICATE_MARK.to_string(), Value::Null);
                    Some((key, target))
                }
                Value::Array(_) => {
                    let (grandparent, list_key) = split_pointer(parent_pointer);
                    let list = document.pointer(parent_pointer).expect("list").clone();
                    let holder = document.pointer_mut(grandparent).expect("holder");
                    holder
                        .as_object_mut()
                        .expect("a list held by a mapping")
                        .insert(DUPLICATE_MARK.to_string(), Value::Null);
                    Some((list_key, list))
                }
                _ => unreachable!(),
            };
        }
        Shape::Trailing => return None,
    };
    match parent {
        Value::Object(map) => {
            map.insert(key, replacement);
        }
        Value::Array(items) => items[key.parse::<usize>().expect("an index")] = replacement,
        _ => unreachable!(),
    }
    None
}

/// The value with one (or every) element replaced by `42`: a mapping's
/// property values, a list's items, or, for a scalar, a list holding the
/// scalar beside `42` (or two numbers).
fn wrong_elements(target: &Value, every: bool) -> Value {
    match target {
        Value::Object(map) => {
            // `$`-keys such as `$constraints` are not definitions.
            let first = map.keys().find(|key| !key.starts_with('$')).cloned();
            let mut edited = Map::new();
            for (key, value) in map {
                let wrong = !key.starts_with('$') && (every || Some(key) == first.as_ref());
                edited.insert(key.clone(), if wrong { json!(42) } else { value.clone() });
            }
            Value::Object(edited)
        }
        Value::Array(items) => Value::Array(
            items
                .iter()
                .enumerate()
                .map(|(index, item)| if every || index == 0 { json!(42) } else { item.clone() })
                .collect(),
        ),
        scalar if every => {
            let _ = scalar;
            json!([42, 43])
        }
        scalar => json!([scalar, 42]),
    }
}

/// A JSON value as block YAML: mappings and lists in block style, scalars as
/// JSON (which YAML reads as double-quoted strings, numbers, booleans, and
/// null), and empty collections as `{}` and `[]`.
fn block_yaml(value: &Value, indent: usize, out: &mut String) {
    let pad = " ".repeat(indent);
    match value {
        Value::Object(map) if !map.is_empty() => {
            for (key, item) in map {
                out.push_str(&format!("{pad}{}:", serde_json::to_string(key).expect("key")));
                nested_yaml(item, indent, out);
            }
        }
        Value::Array(items) if !items.is_empty() => {
            for item in items {
                out.push_str(&format!("{pad}-"));
                nested_yaml(item, indent, out);
            }
        }
        scalar => out.push_str(&format!("{pad}{}\n", serde_json::to_string(scalar).expect("scalar"))),
    }
}

fn nested_yaml(value: &Value, indent: usize, out: &mut String) {
    match value {
        Value::Object(map) if !map.is_empty() => {
            out.push('\n');
            block_yaml(value, indent + 4, out);
        }
        Value::Array(items) if !items.is_empty() => {
            out.push('\n');
            block_yaml(value, indent + 4, out);
        }
        scalar => out.push_str(&format!(" {}\n", serde_json::to_string(scalar).expect("scalar"))),
    }
}

/// Serializes `document`, replacing the duplicate marker with a repeat of
/// `duplicate`'s key, and appending garbage for a trailing cell.
fn serialize(document: &Value, syntax: Syntax, duplicate: Option<&(String, Value)>, trailing: bool) -> String {
    let mut text = match syntax {
        Syntax::Yaml => {
            let mut out = String::new();
            block_yaml(document, 0, &mut out);
            out
        }
        // One line: Darkmatter's strict schema-source locator cannot project
        // spans through a multi-line flow collection, so pretty-printed JSON
        // fails to load (recorded in the implementation log).
        Syntax::Json => serde_json::to_string(document).expect("json"),
    };
    if let Some((key, value)) = duplicate {
        let mark = serde_json::to_string(DUPLICATE_MARK).expect("mark");
        let repeated = format!(
            "{}: {}",
            serde_json::to_string(key).expect("key"),
            serde_json::to_string(value).expect("value")
        );
        let marked = match syntax {
            Syntax::Yaml => format!("{mark}: null"),
            Syntax::Json => format!("{mark}:null"),
        };
        assert!(text.contains(&marked), "the duplicate marker was serialized: {text}");
        text = text.replacen(&marked, &repeated, 1);
    }
    if trailing {
        text.push_str(match syntax {
            Syntax::Yaml => "\n]] trailing garbage [[\n",
            Syntax::Json => "\n} trailing garbage\n",
        });
    }
    text
}

/// The shipped bundle with one file in `syntax`, unedited: block YAML is the
/// shipped text itself, and JSON syntax is that text's value as JSON.
fn bundle_in(shipped: &[BundleFile], syntax: Syntax) -> Vec<BundleFile> {
    shipped
        .iter()
        .map(|(name, text)| {
            let text = match syntax {
                Syntax::Yaml => text.clone(),
                Syntax::Json => serialize(&as_json(text), Syntax::Json, None, false),
            };
            (*name, text)
        })
        .collect()
}

fn as_json(text: &str) -> Value {
    Yaml::from_str(text).expect("shipped YAML").as_json().expect("JSON-compatible")
}

/// The bundle's discovery from `origin`, or `None` when it fails visibly.
fn discover_from(bundle: &[BundleFile], edited: &str, origin: Origin) -> (SchemaRepo, Option<Discovered>) {
    match origin {
        Origin::Disk => {
            let repo = SchemaRepo::new(bundle);
            let discovered = repo.discover().ok();
            (repo, discovered)
        }
        Origin::Embedded => {
            let repo = SchemaRepo::new(&[]);
            let discovered = match repo.discover_embedded(embedded_bundle(bundle)) {
                Ok(discovered) => Some(discovered),
                Err(SchemaError::EmbeddedTriggerLoad { bundle, name, .. }) => {
                    assert_eq!(bundle, CLAUDINE_SCHEMA_BUNDLE_NAME);
                    assert!(
                        CLAUDINE_TRIGGER_FILES.contains(&name.as_str()) || name == edited,
                        "the failure names a trigger or the edited {edited}: {name}"
                    );
                    None
                }
                Err(other) => panic!("an embedded bundle fails as EmbeddedTriggerLoad, not: {other}"),
            };
            (repo, discovered)
        }
    }
}

/// The outcome of one bundle, through public discovery and validation.
fn observe(bundle: &[BundleFile], column: &Column, origin: Origin) -> Outcome {
    let (repo, discovered) = discover_from(bundle, column.file, origin);
    let Some(discovered) = discovered else {
        return Error;
    };
    let claudine = text_of(bundle, "claudine.yaml");
    let inline = text_of(bundle, "inline-compose.yaml");
    let mut results = [false; 2];
    for (index, probe) in [column.probes.0, column.probes.1].into_iter().enumerate() {
        let path = repo.write_document(&format!("probe-{index}.md"), &format!("---\n{probe}\n---\n"));
        let Ok(judged) = discovered.judge(&path, claudine, inline) else {
            return Error;
        };
        results[index] = if column.selection {
            if column.file == "inline-compose.yaml" { judged.inline } else { judged.claudine }
        } else {
            judged.report.valid
        };
    }
    if column.selection { Selects(results[0], results[1]) } else { Verdicts(results[0], results[1]) }
}

fn text_of<'a>(bundle: &'a [BundleFile], name: &str) -> &'a str {
    &bundle.iter().find(|(file, _)| *file == name).expect("bundle file").1
}

/// Every column's name, in table order.
pub(crate) fn column_names() -> Vec<&'static str> {
    COLUMNS.iter().map(|column| column.name).collect()
}

/// Walks one column in one syntax: the unedited control, then every shape.
fn walk(column_name: &str, syntax: Syntax, shipped: &[BundleFile]) {
    walk_from(column_name, syntax, shipped, Origin::Disk);
}

/// [`walk`] with the bundle read from `origin`. Both origins share the
/// table: no cell's outcome depends on where the bundle is read from.
pub(crate) fn walk_from(column_name: &str, syntax: Syntax, shipped: &[BundleFile], origin: Origin) {
    let column = COLUMNS.iter().find(|column| column.name == column_name).expect("a table column");
    let control = bundle_in(shipped, syntax);
    // A trigger may reject the shipped probe values only through the
    // payload, so the control of a selection column is a selection pair.
    let expected_control = if column.selection { Selects(true, false) } else { CONTROL };
    assert_eq!(observe(&control, column, origin), expected_control, "{column_name} ({syntax:?}, {origin:?}): control");

    let source = as_json(text_of(shipped, column.file));
    let mut failures = Vec::new();
    for (shape, expected) in SHAPES.iter().zip(column.outcomes) {
        let expected = unclaimed_trigger(column, *shape).unwrap_or(expected);
        let mut document = source.clone();
        let duplicate = edit(&mut document, column.pointer, *shape);
        let edited = serialize(&document, syntax, duplicate.as_ref(), *shape == Shape::Trailing);
        let bundle: Vec<BundleFile> = control
            .iter()
            .map(|(name, text)| (*name, if *name == column.file { edited.clone() } else { text.clone() }))
            .collect();
        let observed = observe(&bundle, column, origin);
        if observed != expected {
            failures.push(format!("{shape:?}: expected {expected:?}, observed {observed:?}"));
        }
    }
    assert!(failures.is_empty(), "{column_name} ({syntax:?}, {origin:?}):\n{}", failures.join("\n"));
}

/// The outcome of a trigger file that a duplicate key or trailing garbage
/// makes invalid YAML. Darkmatter claims an unparseable file as a trigger
/// envelope only through a column-0 `kind: trigger-schema` line with a plain
/// `kind` key, and this matrix writes every key quoted (`"kind":`, in both
/// syntaxes), so the file is unclassified: its trigger is gone, the probes are
/// unconstrained, and nothing is selected.
fn unclaimed_trigger(column: &Column, shape: Shape) -> Option<Outcome> {
    let unparseable = matches!(shape, Shape::Duplicate | Shape::Trailing);
    (unparseable && CLAUDINE_TRIGGER_FILES.contains(&column.file))
        .then_some(if column.selection { Selects(false, false) } else { UNCONSTRAINED })
}

macro_rules! matrix_tests {
    ($($test:ident => ($column:literal, $syntax:ident);)*) => {
        $(
            #[test]
            fn $test() {
                walk($column, Syntax::$syntax, &shipped_bundle!());
            }
        )*

        /// Every generated test, as `(column, syntax)`.
        const GENERATED: &[(&str, Syntax)] = &[$(($column, Syntax::$syntax)),*];
    };
}

matrix_tests! {
    yaml_library_kind => ("library_kind", Yaml);
    yaml_library_types => ("library_types", Yaml);
    yaml_action_communicate => ("action_communicate", Yaml);
    yaml_action_flow => ("action_flow", Yaml);
    yaml_action_shell => ("action_shell", Yaml);
    yaml_action_expression => ("action_expression", Yaml);
    yaml_action_bundle => ("action_bundle", Yaml);
    yaml_action_initialize_bundle => ("action_initialize_bundle", Yaml);
    yaml_lifecycle_stack => ("lifecycle_stack", Yaml);
    yaml_lifecycle_initialize_stack => ("lifecycle_initialize_stack", Yaml);
    yaml_lifecycle_loop => ("lifecycle_loop", Yaml);
    yaml_linking_user_invocable => ("linking_user_invocable", Yaml);
    yaml_trigger_kind => ("trigger_kind", Yaml);
    yaml_trigger_match => ("trigger_match", Yaml);
    yaml_trigger_payload => ("trigger_payload", Yaml);
    yaml_trigger_condition_operand => ("trigger_condition_operand", Yaml);
    yaml_trigger_combinator => ("trigger_combinator", Yaml);
    yaml_payload_property => ("payload_property", Yaml);
    json_library_kind => ("library_kind", Json);
    json_library_types => ("library_types", Json);
    json_action_communicate => ("action_communicate", Json);
    json_action_flow => ("action_flow", Json);
    json_action_shell => ("action_shell", Json);
    json_action_expression => ("action_expression", Json);
    json_action_bundle => ("action_bundle", Json);
    json_action_initialize_bundle => ("action_initialize_bundle", Json);
    json_lifecycle_stack => ("lifecycle_stack", Json);
    json_lifecycle_initialize_stack => ("lifecycle_initialize_stack", Json);
    json_lifecycle_loop => ("lifecycle_loop", Json);
    json_linking_user_invocable => ("linking_user_invocable", Json);
    json_trigger_kind => ("trigger_kind", Json);
    json_trigger_match => ("trigger_match", Json);
    json_trigger_payload => ("trigger_payload", Json);
    json_trigger_condition_operand => ("trigger_condition_operand", Json);
    json_trigger_combinator => ("trigger_combinator", Json);
    json_payload_property => ("payload_property", Json);
}

#[test]
fn matrix_tests_cover_the_table() {
    for syntax in [Syntax::Yaml, Syntax::Json] {
        let generated: Vec<&str> =
            GENERATED.iter().filter(|(_, generated)| *generated == syntax).map(|(column, _)| *column).collect();
        let table: Vec<&str> = COLUMNS.iter().map(|column| column.name).collect();
        assert_eq!(generated, table, "one {syntax:?} test per column");
    }
}

#[test]
fn json_syntax_bundle_is_the_shipped_bundle() {
    // The JSON-syntax control holds the same values as the shipped YAML, so
    // every JSON cell differs from the shipped bundle by its edit alone.
    for (name, text) in shipped_bundle!() {
        let json_text = serialize(&as_json(&text), Syntax::Json, None, false);
        assert!(json_text.trim_start().starts_with('{'), "{name} is written in JSON syntax");
        assert_eq!(as_json(&json_text), as_json(&text), "{name}");
        let mut block = String::new();
        block_yaml(&as_json(&text), 0, &mut block);
        assert_eq!(as_json(&block), as_json(&text), "{name}'s block YAML round trip");
    }
}

/// The trigger envelope's extra cell (implementation-log ruling R3): a
/// non-string top-level key is an unknown key, not silently ignored. Only
/// block YAML can spell one; every JSON key is a string, which the envelope
/// already rejects as unknown.
#[test]
fn a_non_string_top_level_trigger_key_is_an_error() {
    assert_non_string_trigger_key_errors(&shipped_bundle!(), Origin::Disk);
}

/// The R3 cell from `origin`: the control, then three non-string keys.
pub(crate) fn assert_non_string_trigger_key_errors(shipped: &[BundleFile], origin: Origin) {
    let column = COLUMNS.iter().find(|column| column.name == "trigger_kind").expect("a table column");
    assert_eq!(observe(shipped, column, origin), CONTROL, "control");
    for key in ["42", "true", "[a, b]"] {
        let bundle: Vec<BundleFile> = shipped
            .iter()
            .map(|(name, text)| {
                let text = if *name == "claudine.yaml" { format!("{text}\n{key}: extra\n") } else { text.clone() };
                (*name, text)
            })
            .collect();
        assert_eq!(observe(&bundle, column, origin), Error, "`{key}: extra` in claudine.yaml");
    }
}
