//! API response type definitions.
//!
//! [`ApiResponse`] is the stored single response of an [`crate::Endpoint`].
//! [`SuccessShape`] and its parts are the response model v2 vocabulary: an
//! endpoint declares one or more success shapes, and
//! [`validate_success_shapes`] checks that a client can tell them apart from
//! the reply's status, media type, and JSON tag. Until
//! `Endpoint::response` holds success shapes, a builder stores a lone plain
//! shape as an [`ApiResponse`] and rejects the rest; see
//! `schematic/docs/responses-and-errors.md`.
//!
//! These constructors describe stored transfer data.
//! REST authors use typed endpoint builders; API build rejects JSON
//! references that do not resolve to one of the API's schema components.

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::TypeRef;

/// Describes the expected response from an API endpoint.
///
/// Each variant indicates a different response format, which affects
/// how the generated client handles the response body.
///
/// ## Examples
///
/// JSON response (most common):
///
/// ```
/// use schematic_define::ApiResponse;
///
/// // Using the convenience method
/// let response = ApiResponse::json_type("UserResponse");
/// ```
///
/// Text response:
///
/// ```
/// use schematic_define::ApiResponse;
///
/// let response = ApiResponse::Text;
/// ```
///
/// Empty response (for DELETE or 204 responses):
///
/// ```
/// use schematic_define::ApiResponse;
///
/// let response = ApiResponse::Empty;
/// ```
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ApiResponse {
    /// JSON response, referencing the API's schema components.
    ///
    /// The response body is deserialized as JSON into the referenced type.
    /// This is the most common response type for REST APIs.
    Json(TypeRef),

    /// Plain text response.
    ///
    /// The response body is returned as a `String`.
    Text,

    /// Binary data (bytes).
    ///
    /// The response body is returned as `Vec<u8>`. Use for file downloads,
    /// images, or other binary content.
    Binary,

    /// No response body expected.
    ///
    /// Used for endpoints that return 204 No Content or where the response
    /// body should be ignored.
    Empty,
}

impl ApiResponse {
    /// Creates a JSON response with the given reference.
    pub fn json(reference: TypeRef) -> Self {
        Self::Json(reference)
    }

    /// Creates a JSON response naming one schema component.
    ///
    /// ## Examples
    ///
    /// ```
    /// use schematic_define::{ApiResponse, TypeRef};
    ///
    /// let response = ApiResponse::json_type("ListModelsResponse");
    /// assert_eq!(response, ApiResponse::Json(TypeRef::Named("ListModelsResponse".into())));
    /// ```
    pub fn json_type(component: impl Into<String>) -> Self {
        Self::Json(TypeRef::Named(component.into()))
    }

    /// Creates a JSON response that is an array of one schema component.
    ///
    /// ## Examples
    ///
    /// ```
    /// use schematic_define::{ApiResponse, TypeRef};
    ///
    /// let response = ApiResponse::json_vec_type("ModelInfo");
    /// assert_eq!(response, ApiResponse::Json(TypeRef::Array(Box::new(TypeRef::Named("ModelInfo".into())))));
    /// ```
    pub fn json_vec_type(component: impl Into<String>) -> Self {
        Self::Json(TypeRef::Array(Box::new(TypeRef::Named(component.into()))))
    }

    /// Returns true if this is a JSON response.
    pub fn is_json(&self) -> bool {
        matches!(self, Self::Json(_))
    }

    /// Returns true if this is a binary response.
    pub fn is_binary(&self) -> bool {
        matches!(self, Self::Binary)
    }

    /// Returns true if this is a text response.
    pub fn is_text(&self) -> bool {
        matches!(self, Self::Text)
    }

    /// Returns true if this is an empty response.
    pub fn is_empty(&self) -> bool {
        matches!(self, Self::Empty)
    }
}

/// What one success reply carries (R1).
///
/// Serialized externally tagged: `{"Json": <TypeRef>}`, `"Text"`, `"Binary"`,
/// `"Empty"`, or `{"Stream": {"framing": …, "end": …, "keep": true}}`, where
/// `keep` is omitted when false.
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ResponseShape {
    /// A JSON body decoded as the referenced component.
    Json(TypeRef),
    /// A text body.
    Text,
    /// A binary body.
    Binary,
    /// No body; a multi-shape endpoint selects it by status 204.
    Empty,
    /// An incrementally delivered body.
    Stream {
        /// How the body divides into items.
        framing: StreamFraming,
        /// What ends the stream cleanly. Required in serialized form.
        end: StreamEnd,
        /// Deliver the matching terminator as the last item instead of
        /// consuming it.
        #[serde(default, skip_serializing_if = "std::ops::Not::not")]
        keep: bool,
    },
}

/// How a stream body divides into items. More framings may be added.
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum StreamFraming {
    /// Server-sent events whose data decodes as the referenced item type.
    Sse(TypeRef),
    /// One JSON item of the referenced type per line.
    Ndjson(TypeRef),
    /// Raw byte chunks with no item schema; the shape's content type
    /// describes them.
    Bytes,
}

/// What ends a stream cleanly (Default D-4). A stream that closes before its
/// declared terminator ended early.
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum StreamEnd {
    /// An SSE event whose assembled data equals this text (`[DONE]`); the
    /// empty string is a legitimate terminator.
    DataLineEquals(String),
    /// An SSE event with this name (`message_stop`).
    EventNamed(String),
    /// An item whose field at a dotted path equals a scalar (`done` = `true`).
    /// A missing field does not match.
    FieldEquals {
        /// Dotted object path; no array fan-out.
        field: String,
        /// A string, number, or boolean.
        value: Value,
    },
    /// The server closing the connection; the only end of a byte stream.
    ConnectionClose,
}

/// The R3b tag that separates JSON shapes sharing a status and media type.
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum JsonTag {
    /// The reply's top-level field `name` equals `value`.
    Field {
        /// A top-level field name, never a path.
        name: String,
        /// A string, number, or boolean distinct within the group.
        value: Value,
    },
    /// The reply is a top-level JSON array.
    Array,
    /// The reply is a top-level JSON object.
    Object,
}

/// One declared success shape with its selection facts.
///
/// Every optional member is omitted from serialized form when absent or
/// default, and explicit `null` fails to deserialize.
///
/// ## Examples
///
/// ```
/// use schematic_define::response::{ResponseShape, SuccessShape};
///
/// let receipt = SuccessShape::new(ResponseShape::Empty);
/// assert_eq!(serde_json::to_string(&receipt).unwrap(), r#"{"shape":"Empty"}"#);
/// assert_eq!(receipt.media_type(), None);
/// ```
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SuccessShape {
    /// What the reply carries.
    pub shape: ResponseShape,
    /// A declared concrete media type; `None` uses the shape's default.
    #[serde(default, deserialize_with = "crate::serde_presence::non_null", skip_serializing_if = "Option::is_none")]
    pub content_type: Option<String>,
    /// Exact 2xx statuses that select this shape (R3a); empty declares none.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub statuses: Vec<u16>,
    /// The R3b tag, when JSON shapes share a status and media type.
    #[serde(default, deserialize_with = "crate::serde_presence::non_null", skip_serializing_if = "Option::is_none")]
    pub json_tag: Option<JsonTag>,
    /// Selected for a body-bearing reply whose media type no other shape
    /// matches (R3c).
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub fallback: bool,
    /// When the shape is returned. Documentation only: the client never
    /// reads it.
    #[serde(default, deserialize_with = "crate::serde_presence::non_null", skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
}

impl SuccessShape {
    /// A shape with no declared selection facts.
    pub fn new(shape: ResponseShape) -> Self {
        Self { shape, content_type: None, statuses: vec![], json_tag: None, fallback: false, note: None }
    }

    /// The normalized media type the shape is selected by: the declared
    /// content type, else the shape's default. `None` for an empty shape and
    /// for the fallback, which no media type selects.
    pub fn media_type(&self) -> Option<String> {
        if self.fallback || self.shape == ResponseShape::Empty { return None; }
        match &self.content_type {
            Some(declared) => Some(normalize_media_type(declared).unwrap_or_else(|_| declared.trim().to_ascii_lowercase())),
            None => default_media_type(&self.shape).map(str::to_owned),
        }
    }

    /// The stored single response this shape lowers to while
    /// `Endpoint::response` is an [`ApiResponse`]: only a non-stream shape
    /// without selection facts or a note.
    pub fn legacy_response(&self) -> Option<ApiResponse> {
        if self.content_type.is_some() || !self.statuses.is_empty() || self.json_tag.is_some() || self.fallback || self.note.is_some() {
            return None;
        }
        match &self.shape {
            ResponseShape::Json(reference) => Some(ApiResponse::Json(reference.clone())),
            ResponseShape::Text => Some(ApiResponse::Text),
            ResponseShape::Binary => Some(ApiResponse::Binary),
            ResponseShape::Empty => Some(ApiResponse::Empty),
            ResponseShape::Stream { .. } => None,
        }
    }
}

impl From<ApiResponse> for SuccessShape {
    fn from(response: ApiResponse) -> Self {
        Self::new(match response {
            ApiResponse::Json(reference) => ResponseShape::Json(reference),
            ApiResponse::Text => ResponseShape::Text,
            ApiResponse::Binary => ResponseShape::Binary,
            ApiResponse::Empty => ResponseShape::Empty,
        })
    }
}

/// The media type a shape exports and is selected by when it declares none
/// (R27). `None` for an empty shape.
pub fn default_media_type(shape: &ResponseShape) -> Option<&'static str> {
    match shape {
        ResponseShape::Json(_) => Some("application/json"),
        ResponseShape::Text => Some("text/plain"),
        ResponseShape::Binary | ResponseShape::Stream { framing: StreamFraming::Bytes, .. } => Some("application/octet-stream"),
        ResponseShape::Stream { framing: StreamFraming::Sse(_), .. } => Some("text/event-stream"),
        ResponseShape::Stream { framing: StreamFraming::Ndjson(_), .. } => Some("application/x-ndjson"),
        ResponseShape::Empty => None,
    }
}

/// The comparable form of a concrete media type: `type/subtype` in lowercase,
/// parameters and surrounding space removed.
///
/// ## Errors
///
/// A message when `text` is not `type/subtype` of HTTP token characters, or
/// is a wildcard range such as `*/*` or `image/*`.
///
/// ## Examples
///
/// ```
/// use schematic_define::response::normalize_media_type;
///
/// assert_eq!(normalize_media_type("Application/JSON; charset=utf-8").unwrap(), "application/json");
/// assert!(normalize_media_type("image/*").is_err());
/// ```
pub fn normalize_media_type(text: &str) -> Result<String, String> {
    let essence = text.split(';').next().unwrap_or_default().trim().to_ascii_lowercase();
    let token = |part: &str| !part.is_empty() && part.bytes().all(|c| c.is_ascii_alphanumeric() || b"!#$%&'+-.^_`|~".contains(&c));
    match essence.split_once('/') {
        Some((kind, subtype)) if token(kind) && token(subtype) => Ok(essence),
        _ => Err(format!("media type `{text}` is not a concrete `type/subtype`; wildcard ranges are not supported")),
    }
}

/// Whether `value` may be a tag or terminator value: a string, number, or
/// boolean.
pub(crate) fn is_scalar(value: &Value) -> bool {
    matches!(value, Value::String(_) | Value::Number(_) | Value::Bool(_))
}

fn describe(index: usize, shape: &SuccessShape) -> String {
    let kind = match &shape.shape {
        ResponseShape::Json(_) => "JSON",
        ResponseShape::Text => "text",
        ResponseShape::Binary => "binary",
        ResponseShape::Empty => "empty",
        ResponseShape::Stream { framing: StreamFraming::Sse(_), .. } => "SSE stream",
        ResponseShape::Stream { framing: StreamFraming::Ndjson(_), .. } => "NDJSON stream",
        ResponseShape::Stream { .. } => "byte stream",
    };
    format!("success shape {} ({kind})", index + 1)
}

/// Check one endpoint's success shapes (R1, R3, R3a, R3b, R3c, R23): every
/// shape is well formed, and a client can select exactly one of several
/// shapes from the reply's status, media type, and JSON tag.
///
/// Selection, which this mirrors: status 204 selects the one empty shape; a
/// status some shape declares chooses among the shapes declaring it, any other
/// 2xx among the shapes declaring none; within that set the normalized media
/// type chooses, then the JSON tag, and an unmatched media type takes the
/// fallback. A single shape is never checked against the reply.
///
/// ## Errors
///
/// A message naming the offending shapes (numbered from 1), statuses, and
/// media types, without API or endpoint context.
pub fn validate_success_shapes(shapes: &[SuccessShape]) -> Result<(), String> {
    if shapes.is_empty() { return Err("declares no success shape".into()); }
    let several = shapes.len() > 1;
    for (index, shape) in shapes.iter().enumerate() {
        validate_shape(shape, several).map_err(|message| format!("{}: {message}", describe(index, shape)))?;
    }
    if !several { return Ok(()); }
    let empties: Vec<usize> = (0..shapes.len()).filter(|&i| shapes[i].shape == ResponseShape::Empty).collect();
    if empties.len() > 1 {
        return Err(format!("{} and {} are both empty; at most one empty shape is allowed", describe(empties[0], &shapes[empties[0]]), describe(empties[1], &shapes[empties[1]])));
    }
    let fallbacks: Vec<usize> = (0..shapes.len()).filter(|&i| shapes[i].fallback).collect();
    if fallbacks.len() > 1 {
        return Err(format!("{} and {} are both the fallback; at most one is allowed", describe(fallbacks[0], &shapes[fallbacks[0]]), describe(fallbacks[1], &shapes[fallbacks[1]])));
    }
    let bodies: Vec<usize> = (0..shapes.len()).filter(|&i| shapes[i].shape != ResponseShape::Empty).collect();
    let mut declared: Vec<u16> = bodies.iter().flat_map(|&i| shapes[i].statuses.iter().copied()).collect();
    declared.sort_unstable();
    declared.dedup();
    for status in declared {
        let candidates: Vec<usize> = bodies.iter().copied().filter(|&i| shapes[i].statuses.contains(&status)).collect();
        check_candidates(shapes, &candidates, &format!("status {status}"))?;
    }
    let statusless: Vec<usize> = bodies.into_iter().filter(|&i| shapes[i].statuses.is_empty()).collect();
    check_candidates(shapes, &statusless, "statuses no shape declares")
}

fn validate_shape(shape: &SuccessShape, several: bool) -> Result<(), String> {
    if let Some(content_type) = &shape.content_type {
        if shape.shape == ResponseShape::Empty { return Err("an empty shape has no media type".into()); }
        if shape.fallback { return Err("the fallback declares no content type; it takes every unmatched media type".into()); }
        normalize_media_type(content_type)?;
    }
    for (index, &status) in shape.statuses.iter().enumerate() {
        if !(200..=299).contains(&status) { return Err(format!("declared status {status} is outside 200-299")); }
        if shape.statuses[..index].contains(&status) { return Err(format!("declares status {status} more than once")); }
        if several && (status == 204) != (shape.shape == ResponseShape::Empty) {
            return Err(if status == 204 { "declares status 204, which selects the empty shape".into() }
                else { format!("an empty shape is selected by status 204, not {status}") });
        }
    }
    match &shape.json_tag {
        Some(_) if !matches!(shape.shape, ResponseShape::Json(_)) => return Err("only a JSON shape takes a JSON tag".into()),
        Some(JsonTag::Field { name, value }) => {
            if name.trim().is_empty() || name.contains('.') { return Err(format!("JSON tag field `{name}` must be one nonempty top-level field name")); }
            if !is_scalar(value) { return Err(format!("JSON tag field `{name}` value must be a string, number, or boolean")); }
        }
        _ => {}
    }
    if shape.fallback {
        if !matches!(shape.shape, ResponseShape::Text | ResponseShape::Binary) { return Err("only a text or binary shape may be the fallback".into()); }
        if !several { return Err("a fallback on a single-shape endpoint is redundant".into()); }
    }
    if let ResponseShape::Stream { framing, end, keep } = &shape.shape {
        validate_stream_end(framing, end, *keep)?;
    }
    Ok(())
}

/// Check that `end` suits `framing` (R11b) and `keep` has a terminator.
pub(crate) fn validate_stream_end(framing: &StreamFraming, end: &StreamEnd, keep: bool) -> Result<(), String> {
    let sse = matches!(framing, StreamFraming::Sse(_));
    match end {
        StreamEnd::DataLineEquals(_) if !sse => Err("a data-line terminator needs SSE framing".into()),
        StreamEnd::EventNamed(_) if !sse => Err("an event-name terminator needs SSE framing".into()),
        StreamEnd::EventNamed(name) if name.trim().is_empty() => Err("an event-name terminator needs a nonempty event name".into()),
        StreamEnd::FieldEquals { .. } if matches!(framing, StreamFraming::Bytes) => Err("a field terminator needs SSE or NDJSON framing".into()),
        StreamEnd::FieldEquals { field, value } => {
            if field.split('.').any(|segment| segment.trim().is_empty() || segment.contains(['[', ']', '*'])) {
                return Err(format!("terminator field `{field}` must be a dotted object path without array fan-out"));
            }
            if !is_scalar(value) { return Err(format!("terminator field `{field}` value must be a string, number, or boolean")); }
            Ok(())
        }
        StreamEnd::ConnectionClose if keep => Err("`keep` needs a terminator; connection close delivers nothing to keep".into()),
        _ => Ok(()),
    }
}

/// Shapes in one selection set must differ by media type, or be JSON shapes
/// an R3b tag separates. The fallback has no media type and is set aside.
fn check_candidates(shapes: &[SuccessShape], candidates: &[usize], context: &str) -> Result<(), String> {
    let mut groups: Vec<(String, Vec<usize>)> = Vec::new();
    for &index in candidates {
        let Some(media) = shapes[index].media_type() else { continue };
        match groups.iter_mut().find(|(other, _)| *other == media) {
            Some((_, members)) => members.push(index),
            None => groups.push((media, vec![index])),
        }
    }
    for (media, members) in groups.iter().filter(|(_, members)| members.len() > 1) {
        let named = members.iter().map(|&i| describe(i, &shapes[i])).collect::<Vec<_>>().join(", ");
        if members.iter().any(|&i| !matches!(shapes[i].shape, ResponseShape::Json(_))) {
            return Err(format!("{named} share media type `{media}` for {context}; only JSON shapes may share one, separated by a JSON tag"));
        }
        let tags: Vec<Option<&JsonTag>> = members.iter().map(|&i| shapes[i].json_tag.as_ref()).collect();
        let discriminated = match tags[0] {
            Some(JsonTag::Field { name, .. }) => tags.iter().enumerate().all(|(index, tag)| matches!(tag,
                Some(JsonTag::Field { name: other, value }) if other == name
                    && !tags[..index].iter().any(|seen| matches!(seen, Some(JsonTag::Field { value: earlier, .. }) if earlier == value)))),
            _ => false,
        };
        let by_kind = tags.iter().filter(|tag| matches!(tag, Some(JsonTag::Array))).count() == 1
            && tags.iter().filter(|tag| matches!(tag, Some(JsonTag::Object))).count() == 1
            && members.len() == 2;
        if !discriminated && !by_kind {
            return Err(format!("{named} share media type `{media}` for {context} and no JSON tag separates them; \
                tag every shape with the same top-level field and distinct values, or declare one array and one object shape"));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn is_json_returns_true_for_json_response() {
        let response = ApiResponse::json_type("TestResponse");
        assert!(response.is_json());
        assert!(!response.is_binary());
        assert!(!response.is_text());
        assert!(!response.is_empty());
    }

    #[test]
    fn is_binary_returns_true_for_binary_response() {
        let response = ApiResponse::Binary;
        assert!(!response.is_json());
        assert!(response.is_binary());
        assert!(!response.is_text());
        assert!(!response.is_empty());
    }

    #[test]
    fn is_text_returns_true_for_text_response() {
        let response = ApiResponse::Text;
        assert!(!response.is_json());
        assert!(!response.is_binary());
        assert!(response.is_text());
        assert!(!response.is_empty());
    }

    #[test]
    fn is_empty_returns_true_for_empty_response() {
        let response = ApiResponse::Empty;
        assert!(!response.is_json());
        assert!(!response.is_binary());
        assert!(!response.is_text());
        assert!(response.is_empty());
    }
}
