//! Collection, type-conversion, and list-formatting functions: pure
//! handlers over bound arguments.

use serde_json::Value;

use super::{FunctionBinding, bound, json_number, numberlike_value, scalar_string};
use crate::markdown::coercion::ValueKind;
use crate::markdown::compose::expression::call_binding::BoundCall;

pub(super) const BINDINGS: &[FunctionBinding] = &[
    FunctionBinding::pure("first", &[], first),
    FunctionBinding::pure("last", &[], last),
    FunctionBinding::pure("has_key", &["haskey"], has_key),
    FunctionBinding::pure("contains", &[], contains),
    FunctionBinding::pure("length", &[], length).rejecting(&[("val", &[ValueKind::Boolean])]),
    FunctionBinding::pure("number", &[], number).recovering("x", "fallback"),
    FunctionBinding::pure("keys", &[], keys_fn),
    FunctionBinding::pure("values", &[], values_fn),
    FunctionBinding::pure("as_line_separated", &["aslineseparated"], as_line_separated),
    FunctionBinding::pure("as_csv", &["ascsv"], as_csv),
    FunctionBinding::pure("as_tsv", &["astsv"], as_tsv),
    FunctionBinding::pure("as_space_separated", &["asspaceseparated"], as_space_separated),
    FunctionBinding::pure("as_unordered_list", &["asunorderedlist"], as_unordered_list),
    FunctionBinding::pure("as_ordered_list", &["asorderedlist"], as_ordered_list),
    FunctionBinding::pure("as_json", &["asjson"], as_json),
    FunctionBinding::pure("as_json5", &["asjson5"], as_json5),
];

/// `first(x: any[] | null)`: the first element; null for an empty or null
/// list.
fn first(call: &BoundCall<'_>) -> Result<Value, String> {
    Ok(call.args().list(0).and_then(<[Value]>::first).cloned().unwrap_or(Value::Null))
}

/// `last(x: any[] | null)`: the last element, with its type intact; null for
/// an empty or null list.
fn last(call: &BoundCall<'_>) -> Result<Value, String> {
    Ok(call.args().list(0).and_then(<[Value]>::last).cloned().unwrap_or(Value::Null))
}

/// `has_key(obj: object | null, key: string | null)`: a null object or key
/// is `false`.
fn has_key(call: &BoundCall<'_>) -> Result<Value, String> {
    let args = call.args();
    let found = args.object(0).zip(args.text(1)).is_some_and(|(object, key)| object.contains_key(key));
    Ok(Value::Bool(found))
}

/// `contains(haystack, needle: unknown | null)` (charter M10): a text
/// haystack is a case-sensitive substring search; array elements and object
/// values (never keys) are compared to the needle by rendered text, so `4`
/// matches `"4"`. The needle renders through [`scalar_string`] (null is empty
/// text); a null haystack is `false`.
fn contains(call: &BoundCall<'_>) -> Result<Value, String> {
    let args = call.args();
    let needle = scalar_string(bound(args.get(1), "contains", 1)?);
    let matches = |value: &Value| scalar_string(value) == needle;
    let found = match bound(args.get(0), "contains", 0)? {
        Value::String(text) => text.contains(needle.as_str()),
        Value::Array(items) => items.iter().any(matches),
        Value::Object(map) => map.values().any(matches),
        _ => false,
    };
    Ok(Value::Bool(found))
}

/// `length(val: string | any[] | object)` (charter M11): Unicode characters
/// of text, elements of a list, keys of an object. A number arrives as its
/// displayed text; a boolean never binds (registered exclusion).
fn length(call: &BoundCall<'_>) -> Result<Value, String> {
    let count = match bound(call.args().get(0), "length", 0)? {
        Value::String(text) => text.chars().count(),
        Value::Array(items) => items.len(),
        Value::Object(map) => map.len(),
        other => return Err(format!("length() received an unbound {} argument", ValueKind::of(other))),
    };
    Ok(Value::Number(count.into()))
}

/// `number(x: numberlike, fallback?: number)` (charter M12–M13): the numeric
/// value of `x` as sent, or the fallback when `x` did not bind.
fn number(call: &BoundCall<'_>) -> Result<Value, String> {
    let selected = bound(call.args().selected(0), "number", 0)?;
    json_number(numberlike_value(selected, "number")?)
}

/// `keys(obj: object | null): string[]`: the object's keys in insertion
/// order; `[]` for null.
pub(super) fn keys_fn(call: &BoundCall<'_>) -> Result<Value, String> {
    let keys = call.args().object(0).into_iter().flat_map(|object| object.keys());
    Ok(Value::Array(keys.cloned().map(Value::String).collect()))
}

/// `values(obj: object | null): any[]`: the object's values in insertion
/// order; `[]` for null.
pub(super) fn values_fn(call: &BoundCall<'_>) -> Result<Value, String> {
    let values = call.args().object(0).into_iter().flat_map(|object| object.values());
    Ok(Value::Array(values.cloned().collect()))
}

// ── List formatting ─────────────────────────────────────────────────
//
// Each function renders a bound list (`list: any[] | null`) to text; null
// passes through. The flat joiners render each top-level element via
// [`scalar_string`]; the Markdown-list renderers recurse into nested arrays
// and object-array shapes (`depends_on` / `used_by`).

/// Renders the bound list with `render`; null passes through.
fn render_list(call: &BoundCall<'_>, render: impl FnOnce(&[Value]) -> String) -> Result<Value, String> {
    Ok(call.args().list(0).map_or(Value::Null, |items| Value::String(render(items))))
}

/// Shared body for the flat joiners.
fn join_list(call: &BoundCall<'_>, separator: &str) -> Result<Value, String> {
    render_list(call, |items| items.iter().map(scalar_string).collect::<Vec<_>>().join(separator))
}

/// `as_line_separated(list)`: newline-joined, one element per line.
fn as_line_separated(call: &BoundCall<'_>) -> Result<Value, String> {
    join_list(call, "\n")
}

/// `as_csv(list)`: uses `", "` so it reproduces the historical pre-rendered
/// CSV form of the collapsed `ctx.*` variables byte-for-byte.
fn as_csv(call: &BoundCall<'_>) -> Result<Value, String> {
    join_list(call, ", ")
}

fn as_tsv(call: &BoundCall<'_>) -> Result<Value, String> {
    join_list(call, "\t")
}

fn as_space_separated(call: &BoundCall<'_>) -> Result<Value, String> {
    join_list(call, " ")
}

/// `as_json(list)`: compact JSON; the explicit spelling of the default
/// bare-array rendering, and byte-identical to it for every array. Unlike
/// the joiners, an empty list keeps its container and renders `[]`.
fn as_json(call: &BoundCall<'_>) -> Result<Value, String> {
    serialize_list(call, Value::to_string)
}

/// `as_json5(list)`: compact single-line JSON5, delegated to `biscuit-file`,
/// the repository's JSON5 authority, so the quoting and unquoted-key rules
/// stay in one place.
fn as_json5(call: &BoundCall<'_>) -> Result<Value, String> {
    serialize_list(call, biscuit_file::json5::to_json5_compact)
}

/// Serializes the bound list as a whole value; null passes through.
fn serialize_list(call: &BoundCall<'_>, render: fn(&Value) -> String) -> Result<Value, String> {
    Ok(match call.args().get(0) {
        Some(list @ Value::Array(_)) => Value::String(render(list)),
        _ => Value::Null,
    })
}

/// `as_unordered_list(list)`: Markdown unordered list with recursive
/// auto-nesting of nested arrays and object-array shapes.
fn as_unordered_list(call: &BoundCall<'_>) -> Result<Value, String> {
    render_list(call, |items| render_markdown_list(items, false))
}

/// `as_ordered_list(list)`: Markdown ordered list with recursive auto-nesting.
fn as_ordered_list(call: &BoundCall<'_>) -> Result<Value, String> {
    render_list(call, |items| render_markdown_list(items, true))
}

fn render_markdown_list(items: &[Value], ordered: bool) -> String {
    let mut lines = Vec::new();
    render_list_level(items, ordered, 0, &mut lines);
    lines.join("\n")
}

/// Indentation for a Markdown-list nesting level (two spaces per level).
fn list_indent(depth: usize) -> String {
    "  ".repeat(depth)
}

/// The bullet marker for the `n`-th item at a level.
fn list_marker(ordered: bool, n: usize) -> String {
    if ordered { format!("{n}.") } else { "-".to_string() }
}

/// Renders one object element as a bullet plus recursive sublists.
///
/// Scalar-valued fields form the bullet label (joined by a space, in key order);
/// array- and object-valued fields become indented sublists. This models the
/// `depends_on` / `used_by` object-array shape — `{ package, dependencies }`
/// renders `package` as the bullet and each dependency as a sub-bullet — without
/// any hard-coded field names.
fn render_object_item(
    map: &serde_json::Map<String, Value>,
    ordered: bool,
    depth: usize,
    n: usize,
    out: &mut Vec<String>,
) {
    let mut label_parts = Vec::new();
    let mut containers = Vec::new();
    for value in map.values() {
        match value {
            Value::Array(_) | Value::Object(_) => containers.push(value),
            scalar => label_parts.push(scalar_string(scalar)),
        }
    }
    out.push(format!("{}{} {}", list_indent(depth), list_marker(ordered, n), label_parts.join(" ")));
    for container in containers {
        match container {
            Value::Array(children) => render_list_level(children, ordered, depth + 1, out),
            Value::Object(child) => render_object_item(child, ordered, depth + 1, 1, out),
            _ => {}
        }
    }
}

/// Recursively renders the items at one nesting level into `out`.
///
/// A scalar element becomes a bullet; an object element delegates to
/// [`render_object_item`]; an array element renders its contents as an indented
/// sublist one level deeper (auto-nesting — nesting is a property of the data,
/// not a separate function).
fn render_list_level(items: &[Value], ordered: bool, depth: usize, out: &mut Vec<String>) {
    let mut counter = 0usize;
    for item in items {
        match item {
            Value::Array(children) => render_list_level(children, ordered, depth + 1, out),
            Value::Object(map) => {
                counter += 1;
                render_object_item(map, ordered, depth, counter, out);
            }
            scalar => {
                counter += 1;
                out.push(format!(
                    "{}{} {}",
                    list_indent(depth),
                    list_marker(ordered, counter),
                    scalar_string(scalar)
                ));
            }
        }
    }
}
