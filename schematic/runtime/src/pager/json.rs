//! Reading values out of a decoded page: path lookups, the record fan-out
//! of an item selector, and the interpretation of continuation, flag, and
//! total values.

use http::HeaderMap;
use serde_json::Value;

use super::spec::{ItemSelector, Source, Step};

/// The value at `path` (object keys from `value`), or `None` when a key is
/// missing or an intermediate value is not an object. The empty path is
/// `value` itself.
pub fn lookup<'v>(value: &'v Value, path: &[&str]) -> Option<&'v Value> {
    path.iter().try_fold(value, |node, key| node.as_object()?.get(*key))
}

/// Why an authored item location could not be read.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CollectionError {
    /// A key on the record path is missing.
    Missing,
    /// A value on the record path is not an object where a key is needed,
    /// or not an array where a fan-out is needed.
    NotAnArray,
    /// A record lacks its projection.
    Projection,
}

/// The records a selector reaches in `page`, in document order.
///
/// Every fan-out must meet an array and every key an object; anything else
/// is an error, never an empty collection.
pub fn records<'v>(page: &'v Value, selector: &ItemSelector) -> Result<Vec<&'v Value>, CollectionError> {
    let mut nodes = vec![page];
    for step in selector.record {
        let mut next = Vec::new();
        for node in nodes {
            match step {
                Step::Key(key) => match node {
                    Value::Object(map) => next.push(map.get(*key).ok_or(CollectionError::Missing)?),
                    _ => return Err(CollectionError::NotAnArray),
                },
                Step::Each => match node {
                    Value::Array(elements) => next.extend(elements),
                    _ => return Err(CollectionError::NotAnArray),
                },
            }
        }
        nodes = next;
    }
    Ok(nodes)
}

/// The item a record projects to.
pub fn project<'v>(record: &'v Value, selector: &ItemSelector) -> Result<&'v Value, CollectionError> {
    lookup(record, selector.projection).ok_or(CollectionError::Projection)
}

/// A response value read from a [`Source`]: the JSON value at a body path,
/// or a header's text.
#[derive(Debug, Clone, PartialEq)]
pub enum Read<'v> {
    /// The source is absent: a missing key or header.
    Absent,
    Json(&'v Value),
    Header(String),
    /// A header whose value is not text.
    InvalidHeader,
}

/// Reads `source` from a page's body and headers. A repeated header reads
/// its first value.
pub fn read<'v>(source: Source, body: &'v Value, headers: &HeaderMap) -> Read<'v> {
    match source {
        Source::Body(path) => lookup(body, path).map_or(Read::Absent, Read::Json),
        Source::Header(name) => match headers.get(name) {
            None => Read::Absent,
            Some(value) => match value.to_str() {
                Ok(text) => Read::Header(text.to_owned()),
                Err(_) => Read::InvalidHeader,
            },
        },
    }
}

/// What a continuation value means.
#[derive(Debug, Clone, PartialEq)]
pub enum Continuation {
    /// Missing, null, empty, or an authored terminal value: no continuation.
    End,
    /// A usable value, sent as read.
    Value(Value),
    /// Present but unusable: a structured value bound for a query or header,
    /// or non-text header bytes.
    Invalid,
}

/// Interprets a continuation read from a page.
///
/// A string, number, or boolean is a value: zero and `false` are not
/// terminal. An array or object is a value only when `structured` (the
/// destination is a JSON body field). Missing, `null`, the empty string, and
/// every `terminal` string end the traversal.
pub fn continuation(read: &Read<'_>, terminal: &[&str], structured: bool) -> Continuation {
    match read {
        Read::Absent | Read::Json(Value::Null) => Continuation::End,
        Read::Json(Value::String(text)) | Read::Header(text) => {
            if text.is_empty() || terminal.contains(&text.as_str()) {
                Continuation::End
            } else {
                Continuation::Value(Value::String(text.clone()))
            }
        }
        Read::Json(value @ (Value::Number(_) | Value::Bool(_))) => Continuation::Value((*value).clone()),
        Read::Json(value @ (Value::Array(_) | Value::Object(_))) if structured => {
            Continuation::Value((*value).clone())
        }
        Read::Json(_) | Read::InvalidHeader => Continuation::Invalid,
    }
}

/// A value as query or header text: a string as written, a number or
/// boolean as its JSON text; `None` for anything structured.
pub fn as_text(value: &Value) -> Option<String> {
    match value {
        Value::String(text) => Some(text.clone()),
        Value::Number(_) | Value::Bool(_) => Some(value.to_string()),
        _ => None,
    }
}

/// Interprets a boolean flag: a JSON boolean, or a header spelled `true` or
/// `false` in any case. `Ok(None)` when it is absent; `Err` when it is
/// present with another type.
pub fn flag(read: &Read<'_>) -> Result<Option<bool>, ()> {
    match read {
        Read::Absent => Ok(None),
        Read::Json(Value::Bool(value)) => Ok(Some(*value)),
        Read::Header(text) if text.eq_ignore_ascii_case("true") => Ok(Some(true)),
        Read::Header(text) if text.eq_ignore_ascii_case("false") => Ok(Some(false)),
        _ => Err(()),
    }
}

/// Whether a value is present and non-empty: anything but missing, `null`,
/// and the empty string.
pub fn present(read: &Read<'_>) -> bool {
    match read {
        Read::Absent | Read::Json(Value::Null) => false,
        Read::Json(Value::String(text)) | Read::Header(text) => !text.trim().is_empty(),
        Read::Json(_) | Read::InvalidHeader => true,
    }
}

/// Interprets a total: a non-negative JSON integer, or text holding one.
/// `Ok(None)` when it is absent or `null`; `Err` when it is negative, not an
/// integer, or not a number.
pub fn total(read: &Read<'_>) -> Result<Option<u64>, ()> {
    match read {
        Read::Absent | Read::Json(Value::Null) => Ok(None),
        Read::Json(Value::Number(number)) => number.as_u64().map(Some).ok_or(()),
        Read::Json(Value::String(text)) | Read::Header(text) => text.trim().parse::<u64>().map(Some).map_err(drop),
        _ => Err(()),
    }
}

#[cfg(test)]
mod tests {
    //! Ported from the pagination prototype: p01, p05, g4, and g6.

    use serde_json::json;

    use super::*;

    #[test]
    fn a_dotted_key_is_one_segment() {
        let page = json!({ "@odata.nextLink": "https://a.test/users?$skiptoken=X", "value": [] });
        assert_eq!(lookup(&page, &["@odata.nextLink"]), Some(&json!("https://a.test/users?$skiptoken=X")));
        assert_eq!(lookup(&page, &["@odata", "nextLink"]), None);
    }

    #[test]
    fn a_nested_path_reaches_through_objects_only() {
        let page = json!({ "members": [], "response_metadata": { "next_cursor": "dXNlcg==" } });
        assert_eq!(lookup(&page, &["response_metadata", "next_cursor"]), Some(&json!("dXNlcg==")));
        assert_eq!(lookup(&page, &["members", "0"]), None, "arrays are not indexed by key");
        assert_eq!(lookup(&page, &[]), Some(&page));
    }

    const REDDIT: ItemSelector = ItemSelector {
        record: &[Step::Key("data"), Step::Key("children"), Step::Each],
        projection: &["data"],
    };

    const MULTI_SEARCH: ItemSelector = ItemSelector {
        record: &[Step::Key("responses"), Step::Each, Step::Key("hits"), Step::Key("hits"), Step::Each],
        projection: &["_source"],
    };

    #[test]
    fn records_fan_out_and_project_in_document_order() {
        let page = json!({ "kind": "Listing", "data": { "after": "t3_b", "children": [
            { "kind": "t3", "data": { "name": "t3_a" } },
            { "kind": "t3", "data": { "name": "t3_b" } },
        ] } });
        let records = records(&page, &REDDIT).expect("records");
        let items: Vec<&Value> = records.iter().map(|record| project(record, &REDDIT).expect("projection")).collect();
        assert_eq!(items, [&json!({ "name": "t3_a" }), &json!({ "name": "t3_b" })]);
    }

    #[test]
    fn two_fan_outs_flatten_and_item_paths_read_the_record() {
        let page = json!({ "responses": [
            { "hits": { "hits": [{ "_source": { "n": 1 }, "sort": [1] }] } },
            { "hits": { "hits": [] } },
            { "hits": { "hits": [{ "_source": { "n": 2 }, "sort": [2] }, { "_source": { "n": 3 }, "sort": [3] }] } },
        ] });
        let records = records(&page, &MULTI_SEARCH).expect("records");
        let items: Vec<&Value> = records.iter().map(|record| project(record, &MULTI_SEARCH).expect("projection")).collect();
        assert_eq!(items, [&json!({ "n": 1 }), &json!({ "n": 2 }), &json!({ "n": 3 })]);
        let last = records.last().expect("a record");
        assert_eq!(lookup(last, &["sort"]), Some(&json!([3])), "cursors read the hit, not the projection");
    }

    #[test]
    fn a_wrong_shape_is_an_error_never_an_empty_collection() {
        assert_eq!(records(&json!([1, 2]), &ItemSelector::ROOT).map(|found| found.len()), Ok(2));
        assert_eq!(records(&json!({ "data": [] }), &ItemSelector::ROOT), Err(CollectionError::NotAnArray));
        assert_eq!(records(&json!({}), &REDDIT), Err(CollectionError::Missing));
        assert_eq!(records(&json!({ "data": { "children": {} } }), &REDDIT), Err(CollectionError::NotAnArray));
        assert_eq!(
            records(&json!({ "responses": [{ "hits": { "hits": [] } }, { "hits": 3 }] }), &MULTI_SEARCH),
            Err(CollectionError::NotAnArray),
            "every branch of a fan-out must match"
        );
        assert_eq!(project(&json!({ "kind": "t3" }), &REDDIT), Err(CollectionError::Projection));
    }

    #[test]
    fn terminal_values_end_and_other_scalars_are_values() {
        let terminal = &["null"];
        let read = |value: &Value| continuation(&Read::Json(value), terminal, false);
        assert_eq!(continuation(&Read::Absent, terminal, false), Continuation::End);
        assert_eq!(read(&Value::Null), Continuation::End);
        assert_eq!(read(&json!("")), Continuation::End);
        assert_eq!(read(&json!("null")), Continuation::End);
        assert_eq!(continuation(&Read::Header("null".into()), terminal, false), Continuation::End);
        assert_eq!(read(&json!("MjAwMDAw")), Continuation::Value(json!("MjAwMDAw")));
        assert_eq!(read(&json!(0)), Continuation::Value(json!(0)));
        assert_eq!(read(&json!(false)), Continuation::Value(json!(false)));
        assert_eq!(continuation(&Read::Json(&json!("null")), &[], false), Continuation::Value(json!("null")));
    }

    #[test]
    fn a_structured_cursor_is_a_value_only_for_a_body_input() {
        let sort = json!([1, "a"]);
        assert_eq!(continuation(&Read::Json(&sort), &[], false), Continuation::Invalid);
        assert_eq!(continuation(&Read::Json(&sort), &[], true), Continuation::Value(sort.clone()));
        assert_eq!(continuation(&Read::InvalidHeader, &[], true), Continuation::Invalid);
    }

    #[test]
    fn flags_and_totals_accept_only_their_types() {
        assert_eq!(flag(&Read::Json(&json!(true))), Ok(Some(true)));
        assert_eq!(flag(&Read::Header("FALSE".into())), Ok(Some(false)));
        assert_eq!(flag(&Read::Absent), Ok(None));
        assert_eq!(flag(&Read::Json(&json!("true"))), Err(()));
        assert_eq!(flag(&Read::Json(&json!(1))), Err(()));
        assert_eq!(total(&Read::Json(&json!(3))), Ok(Some(3)));
        assert_eq!(total(&Read::Header(" 12 ".into())), Ok(Some(12)));
        assert_eq!(total(&Read::Json(&Value::Null)), Ok(None));
        for bad in [json!(-1), json!(1.5), json!("x"), json!(true), json!([])] {
            assert_eq!(total(&Read::Json(&bad)), Err(()), "{bad}");
        }
    }
}
