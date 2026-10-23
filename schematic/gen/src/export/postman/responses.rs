//! Saved example responses.
//!
//! One response per declared success shape, in declaration order, then one
//! per entry of the effective error table. JSON bodies come from
//! [`ExampleSource`], so each validates against its registered schema; a
//! shape whose example cannot be built is saved without a body and reported.

use schematic_define::response::{JsonTag, ResponseShape, StreamEnd, StreamFraming, SuccessShape, default_media_type};
use schematic_define::{ErrorEntry, ErrorTable, StatusMatch, TypeRef};
use serde_json::{Map, Value};

use crate::export::examples::{ExampleError, ExampleSource};

use super::types::{PostmanHeader, PostmanResponse};

/// The body of a text response.
const TEXT_PLACEHOLDER: &str = "string";

/// The saved responses of one endpoint or operation, and each example that
/// could not be built with the role it was for
/// (`200 OK (application/json) response`).
pub(crate) fn saved_responses(
    examples: &ExampleSource,
    success: &[SuccessShape],
    errors: &ErrorTable,
) -> (Vec<PostmanResponse>, Vec<(String, ExampleError)>) {
    let mut responses = Vec::new();
    let mut failures = Vec::new();
    let several = success.len() > 1;
    for shape in success {
        let code = success_status(shape);
        let media = shape.content_type.clone().or_else(|| default_media_type(&shape.shape).map(str::to_owned));
        let body = match &shape.shape {
            ResponseShape::Json(reference) => json_body(examples, reference, shape.json_tag.as_ref()),
            ResponseShape::Stream { framing, end, keep } => stream_body(examples, framing, end, *keep, media.as_deref()),
            other => Ok(plain_body(other, media.as_deref())),
        };
        let mut name = response_name(code, media.as_deref().filter(|_| shape.shape != ResponseShape::Empty));
        if several && let Some(variant) = &shape.variant {
            name.push_str(&format!(" — {variant}"));
        }
        responses.push(response(name.clone(), code, media.as_deref(), body.map_err(|error| failures.push((format!("{name} response"), error))).ok().flatten()));
    }
    for entry in errors.entries() {
        let Some(code) = error_status(entry, errors) else { continue };
        let media = entry.content_type.clone().or_else(|| default_media_type(&entry.shape).map(str::to_owned));
        let body = match &entry.shape {
            ResponseShape::Json(reference) => json_body(examples, reference, None),
            other => Ok(plain_body(other, media.as_deref())),
        };
        let name = response_name(code, media.as_deref().filter(|_| entry.shape != ResponseShape::Empty));
        responses.push(response(name.clone(), code, media.as_deref(), body.map_err(|error| failures.push((format!("{name} response"), error))).ok().flatten()));
    }
    (responses, failures)
}

/// A shape's first declared status; otherwise 204 for a shape without a
/// body and 200 for any other.
fn success_status(shape: &SuccessShape) -> u16 {
    match shape.statuses.first() {
        Some(status) => *status,
        None if shape.shape == ResponseShape::Empty => 204,
        None => 200,
    }
}

/// The status an error entry is saved under: its exact status, or the
/// lowest status of its range that the table resolves to it, so a narrower
/// entry overriding part of the range keeps its own statuses. `None` when
/// narrower entries cover the whole range.
fn error_status(entry: &ErrorEntry, table: &ErrorTable) -> Option<u16> {
    match entry.status {
        StatusMatch::Exact(status) => Some(status),
        StatusMatch::Range { lo, hi } => (lo..=hi).find(|status| table.lookup(*status) == Some(entry)),
        // `StatusMatch` is non-exhaustive; a later form is not saved.
        _ => None,
    }
}

/// A JSON response with `body`, or without a body when there is none.
pub(crate) fn json_response(code: u16, body: Option<String>) -> PostmanResponse {
    response(response_name(code, Some("application/json")), code, Some("application/json"), body)
}

fn response(name: String, code: u16, media: Option<&str>, body: Option<String>) -> PostmanResponse {
    let header = match (media, &body) {
        (Some(media), Some(_)) => vec![PostmanHeader { key: "Content-Type".to_string(), value: media.to_string(), disabled: false, description: None }],
        _ => Vec::new(),
    };
    PostmanResponse { name, status: reason(code).to_string(), code, header, body }
}

/// `200 OK (application/json)`, or without the media type for no body.
fn response_name(code: u16, media: Option<&str>) -> String {
    match media {
        Some(media) => format!("{code} {} ({media})", reason(code)),
        None => format!("{code} {}", reason(code)),
    }
}

/// The example of a JSON shape, pretty-printed. A field tag sets its
/// top-level field; an array or object tag must match the example's kind.
fn json_body(examples: &ExampleSource, reference: &TypeRef, tag: Option<&JsonTag>) -> Result<Option<String>, ExampleError> {
    let mut value = examples.example(reference)?;
    match tag {
        Some(JsonTag::Field { name, value: tagged }) => {
            if let Value::Object(object) = &mut value {
                object.insert(name.clone(), tagged.clone());
            }
            examples.check(reference, &value)?;
        }
        Some(JsonTag::Array) if !value.is_array() => return Err(tag_mismatch("array")),
        Some(JsonTag::Object) if !value.is_object() => return Err(tag_mismatch("object")),
        _ => {}
    }
    Ok(Some(serde_json::to_string_pretty(&value).unwrap_or_default()))
}

fn tag_mismatch(kind: &str) -> ExampleError {
    ExampleError { schema_path: "#".to_string(), reason: format!("the shape is tagged as a JSON {kind}, but its example is not one") }
}

/// A text placeholder, a documented binary placeholder, or no body.
fn plain_body(shape: &ResponseShape, media: Option<&str>) -> Option<String> {
    match shape {
        ResponseShape::Text => Some(TEXT_PLACEHOLDER.to_string()),
        ResponseShape::Binary => Some(binary_placeholder(media)),
        _ => None,
    }
}

fn binary_placeholder(media: Option<&str>) -> String {
    format!("<binary {} body; not representable as an example>", media.unwrap_or("application/octet-stream"))
}

/// A finite stream: one item, then the declared terminator. Each frame is
/// validated on its own, never the stream as one JSON value.
///
/// A terminator event the runtime consumes (`keep` false) carries `{}` as
/// its data; one delivered as the last item (`keep` true) carries the item
/// example.
fn stream_body(examples: &ExampleSource, framing: &StreamFraming, end: &StreamEnd, keep: bool, media: Option<&str>) -> Result<Option<String>, ExampleError> {
    let reference = match framing {
        StreamFraming::Sse(reference) | StreamFraming::Ndjson(reference) => reference,
        _ => return Ok(Some(binary_placeholder(media))),
    };
    let item = examples.example(reference)?;
    let terminator = match end {
        StreamEnd::FieldEquals { field, value } => {
            let mut last = item.clone();
            set_field(&mut last, field, value.clone());
            examples.check(reference, &last)?;
            Some(last)
        }
        _ => None,
    };
    let compact = |value: &Value| serde_json::to_string(value).unwrap_or_default();
    let body = match framing {
        StreamFraming::Sse(_) => {
            let mut text = format!("data: {}\n\n", compact(&item));
            match end {
                StreamEnd::DataLineEquals(marker) => text.push_str(&format!("data: {marker}\n\n")),
                StreamEnd::EventNamed(event) => {
                    let data = if keep { compact(&item) } else { "{}".to_string() };
                    text.push_str(&format!("event: {event}\ndata: {data}\n\n"));
                }
                _ => {}
            }
            if let Some(last) = &terminator {
                text.push_str(&format!("data: {}\n\n", compact(last)));
            }
            text
        }
        _ => {
            let mut text = format!("{}\n", compact(&item));
            if let Some(last) = &terminator {
                text.push_str(&format!("{}\n", compact(last)));
            }
            text
        }
    };
    Ok(Some(body))
}

/// Sets the value at a dotted object path, creating objects on the way.
fn set_field(value: &mut Value, path: &str, field: Value) {
    let mut current = value;
    let mut segments = path.split('.').peekable();
    while let Some(segment) = segments.next() {
        if !current.is_object() {
            *current = Value::Object(Map::new());
        }
        let object = current.as_object_mut().expect("just made an object");
        if segments.peek().is_none() {
            object.insert(segment.to_string(), field);
            return;
        }
        current = object.entry(segment.to_string()).or_insert_with(|| Value::Object(Map::new()));
    }
}

/// The reason phrase of a status code.
pub(crate) fn reason(code: u16) -> &'static str {
    match code {
        200 => "OK",
        201 => "Created",
        202 => "Accepted",
        203 => "Non-Authoritative Information",
        204 => "No Content",
        205 => "Reset Content",
        206 => "Partial Content",
        207 => "Multi-Status",
        400 => "Bad Request",
        401 => "Unauthorized",
        402 => "Payment Required",
        403 => "Forbidden",
        404 => "Not Found",
        405 => "Method Not Allowed",
        406 => "Not Acceptable",
        408 => "Request Timeout",
        409 => "Conflict",
        410 => "Gone",
        411 => "Length Required",
        412 => "Precondition Failed",
        413 => "Content Too Large",
        415 => "Unsupported Media Type",
        416 => "Range Not Satisfiable",
        422 => "Unprocessable Content",
        423 => "Locked",
        424 => "Failed Dependency",
        428 => "Precondition Required",
        429 => "Too Many Requests",
        451 => "Unavailable For Legal Reasons",
        500 => "Internal Server Error",
        501 => "Not Implemented",
        502 => "Bad Gateway",
        503 => "Service Unavailable",
        504 => "Gateway Timeout",
        529 => "Site Overloaded",
        _ if (200..300).contains(&code) => "Success",
        _ if (400..500).contains(&code) => "Client Error",
        _ => "Server Error",
    }
}
