//! Response model v2 classification of one operation's responses (R28,
//! Default D-20): the importer's success shapes and endpoint error entries.

use indexmap::IndexMap;
use openapiv3::{MediaType, ReferenceOr, Response, Responses, Schema, SchemaKind, StatusCode};

use super::responses::{JSON_VALUE, reference_from_schema};
use crate::error_table::{ErrorEntry, StatusMatch};
use crate::openapi::OpenApiError;
use crate::openapi::import::diagnostics::OpenApiDiagnostic;
use crate::openapi::import::resolver::RefResolver;
use crate::response::{JsonTag, ResponseShape, StreamEnd, StreamFraming, SuccessShape, default_media_type, normalize_media_type};

/// One operation's responses as success shapes and endpoint error entries.
#[derive(Debug, Clone, PartialEq)]
pub struct ResponseClassification {
    /// Success shapes in the order their statuses and media types appear.
    pub successes: Vec<SuccessShape>,
    /// Endpoint-level error entries; none is hoisted to the API (D-20).
    pub errors: Vec<ErrorEntry>,
    /// What the classification could not represent, each naming the API,
    /// operation, feature, and import target.
    pub diagnostics: Vec<OpenApiDiagnostic>,
}

/// How one media type imports.
enum MediaClass {
    Json,
    Sse,
    Ndjson,
    Text,
    Binary,
}

/// Classifies `responses` of operation `endpoint` of API `api` (R28).
///
/// Media types are normalized first. `text/event-stream` is an SSE stream;
/// `application/x-ndjson` and `application/jsonl` are NDJSON streams;
/// `application/json` and `application/*+json` are JSON; other `text/*` is
/// text and anything else binary, each keeping its content type. `*/*` is the
/// R3c fallback. Several media types under one status are several shapes, and
/// a JSON `oneOf` separable by its `discriminator` or by array versus object
/// is several tagged JSON shapes. Exact 400–599 statuses and `4XX`/`5XX` are
/// endpoint error entries; a body-less status is an empty shape or entry.
///
/// Identical success shapes across statuses are one shape. When the statuses
/// other than a body-less 204 carry different sets of shapes, every shape
/// declares the exact statuses it appears under (R3a), except an empty shape
/// seen only at 204, which 204 already selects.
///
/// Reported as diagnostics, never guessed: `default`, 1xx, and 3xx
/// responses; media ranges other than `*/*`; an inseparable `oneOf` (imported
/// as one schema); a stream or JSON body without a usable schema (imported as
/// `serde_json::Value`, a stream ending at connection close); and an error
/// status whose media types need different shapes (the first is kept).
///
/// ## Errors
///
/// [`OpenApiError::Validation`] when a response reference does not resolve,
/// or when no status yields a representable success.
pub fn classify_responses<'a>(
    api: &str,
    endpoint: &str,
    responses: &'a Responses,
    resolver: &RefResolver<'a>,
) -> Result<ResponseClassification, OpenApiError> {
    let mut context = Context { api, endpoint, diagnostics: vec![] };
    let mut candidates: IndexMap<Option<u16>, Vec<SuccessShape>> = IndexMap::new();
    let mut errors = vec![];

    if responses.default.is_some() {
        context.warn("responses/default", "the OpenAPI `default` response is neither a success shape nor an API default error; \
            it is not imported into the endpoint's success shapes or error table".to_string());
    }

    for (status, response) in &responses.responses {
        let location = format!("responses/{status}");
        let response = resolve(&context, &location, response, resolver)?;
        match *status {
            StatusCode::Code(code @ 200..=299) => {
                let shapes = context.success_shapes(&location, response, resolver);
                candidates.entry(Some(code)).or_default().extend(shapes);
            }
            StatusCode::Range(2) => {
                let shapes = context.success_shapes(&location, response, resolver);
                candidates.entry(None).or_default().extend(shapes);
            }
            StatusCode::Code(code @ 400..=599) => errors.extend(context.error_entry(&location, StatusMatch::exact(code), response, resolver)),
            StatusCode::Range(class @ (4 | 5)) => errors.extend(context.error_entry(&location, StatusMatch::class(class), response, resolver)),
            _ => context.warn(&location, format!(
                "status {status} is neither a 2xx success nor a 400-599 error; its response is not imported into the endpoint's success shapes or error table"
            )),
        }
    }

    let successes = assign_statuses(candidates);
    if successes.is_empty() {
        return Err(OpenApiError::Validation {
            path: format!("{api} {endpoint} responses"),
            message: format!("API {api} endpoint {endpoint}: no 2xx response yields a representable success shape; nothing is imported for the endpoint"),
        });
    }
    Ok(ResponseClassification { successes, errors, diagnostics: context.diagnostics })
}

fn resolve<'a>(context: &Context<'_>, location: &str, response: &'a ReferenceOr<Response>, resolver: &RefResolver<'a>) -> Result<&'a Response, OpenApiError> {
    resolver.resolve_response(response).map_err(|error| OpenApiError::Validation {
        path: location.to_string(),
        message: format!("API {} endpoint {}: {location} does not resolve: {error}", context.api, context.endpoint),
    })
}

/// Deduplicates identical shapes; declares exact statuses only when the
/// statuses carry different shape sets (see [`classify_responses`]).
fn assign_statuses(candidates: IndexMap<Option<u16>, Vec<SuccessShape>>) -> Vec<SuccessShape> {
    let mut distinct: Vec<(SuccessShape, Vec<u16>)> = vec![];
    for (status, shapes) in &candidates {
        for shape in shapes {
            let index = match distinct.iter().position(|(seen, _)| seen == shape) {
                Some(index) => index,
                None => { distinct.push((shape.clone(), vec![])); distinct.len() - 1 }
            };
            if let Some(code) = status && !distinct[index].1.contains(code) { distinct[index].1.push(*code); }
        }
    }
    // 204 selects the empty shape whatever the other statuses declare.
    let no_content = |status: &Option<u16>, shapes: &Vec<SuccessShape>| *status == Some(204) && shapes.iter().all(|shape| shape.shape == ResponseShape::Empty);
    let uniform = candidates.iter().filter(|(status, shapes)| !no_content(status, shapes)).map(|(_, shapes)| shapes).all(|shapes| {
        distinct.iter().all(|(shape, statuses)| shapes.contains(shape) || (shape.shape == ResponseShape::Empty && statuses == &[204]))
    });
    distinct.into_iter().map(|(mut shape, statuses)| {
        let empty_at_no_content = shape.shape == ResponseShape::Empty && statuses == [204];
        if !uniform && !empty_at_no_content { shape.statuses = statuses; }
        shape
    }).collect()
}

struct Context<'c> {
    api: &'c str,
    endpoint: &'c str,
    diagnostics: Vec<OpenApiDiagnostic>,
}

impl Context<'_> {
    fn warn(&mut self, location: &str, message: String) {
        self.diagnostics.push(OpenApiDiagnostic::warn(location.to_string(), format!("API {} endpoint {}: {message}", self.api, self.endpoint)));
    }

    /// The media types of `response` this classification can represent, each
    /// with its class; `*/*` is `None`. Other media ranges and unparsable
    /// media types are reported and skipped.
    fn media<'r>(&mut self, location: &str, response: &'r Response) -> Vec<(Option<(String, MediaClass)>, &'r MediaType)> {
        let mut media = vec![];
        for (key, entry) in &response.content {
            if key.trim() == "*/*" { media.push((None, entry)); continue; }
            match normalize_media_type(key) {
                Ok(normalized) => { let class = classify_media(&normalized); media.push((Some((normalized, class)), entry)); }
                Err(_) => self.warn(&format!("{location}/content/{key}"), format!(
                    "media range `{key}` cannot select a shape (only `*/*`, as the fallback, can); it is not imported into the endpoint's success shapes or error table"
                )),
            }
        }
        media
    }

    fn success_shapes(&mut self, location: &str, response: &Response, resolver: &RefResolver<'_>) -> Vec<SuccessShape> {
        if response.content.is_empty() { return vec![SuccessShape::new(ResponseShape::Empty)]; }
        let mut shapes = vec![];
        for (media, entry) in self.media(location, response) {
            let at = |key: &str| format!("{location}/content/{key}");
            let Some((media, class)) = media else {
                let mut fallback = SuccessShape::new(if is_text_schema(entry.schema.as_ref(), resolver) { ResponseShape::Text } else { ResponseShape::Binary });
                fallback.fallback = true;
                shapes.push(fallback);
                continue;
            };
            match class {
                MediaClass::Json => shapes.extend(self.json_shapes(&at(&media), &media, entry, resolver)),
                MediaClass::Sse | MediaClass::Ndjson => {
                    let item = self.usable_reference(&at(&media), entry, resolver, "stream item", "items decode as `serde_json::Value` and the stream ends at connection close");
                    let framing = if matches!(class, MediaClass::Sse) { StreamFraming::Sse(item) } else { StreamFraming::Ndjson(item) };
                    shapes.push(with_content_type(ResponseShape::Stream { framing, end: StreamEnd::ConnectionClose, keep: false }, &media));
                }
                MediaClass::Text => shapes.push(with_content_type(ResponseShape::Text, &media)),
                MediaClass::Binary => shapes.push(with_content_type(ResponseShape::Binary, &media)),
            }
        }
        shapes
    }

    /// The media entry's schema reference, or `serde_json::Value` with a
    /// diagnostic when it has none or only an inline schema.
    fn usable_reference(&mut self, location: &str, entry: &MediaType, resolver: &RefResolver<'_>, role: &str, consequence: &str) -> crate::TypeRef {
        let reference = entry.schema.as_ref().map(|schema| reference_from_schema(schema, resolver, &mut self.diagnostics));
        match reference {
            Some(reference) if reference != JSON_VALUE => reference,
            _ => {
                self.warn(location, format!("the {role} has no named schema; {consequence}"));
                JSON_VALUE
            }
        }
    }

    fn json_shapes(&mut self, location: &str, media: &str, entry: &MediaType, resolver: &RefResolver<'_>) -> Vec<SuccessShape> {
        if let Some(ReferenceOr::Item(schema)) = &entry.schema
            && let SchemaKind::OneOf { one_of } = &schema.schema_kind {
            match separate_one_of(schema, one_of, resolver) {
                Some(members) => return members.into_iter().map(|(reference, tag)| {
                    let mut shape = with_content_type(ResponseShape::Json(reference), media);
                    shape.json_tag = Some(tag);
                    shape
                }).collect(),
                None => self.warn(location, "the `oneOf` members are separable neither by a `discriminator` nor as one array and one object; \
                    the body imports as one `serde_json::Value` JSON shape".to_string()),
            }
            return vec![with_content_type(ResponseShape::Json(JSON_VALUE), media)];
        }
        let reference = self.usable_reference(location, entry, resolver, "JSON body", "it decodes as `serde_json::Value`");
        vec![with_content_type(ResponseShape::Json(reference), media)]
    }

    fn error_entry(&mut self, location: &str, status: StatusMatch, response: &Response, resolver: &RefResolver<'_>) -> Option<ErrorEntry> {
        if response.content.is_empty() { return Some(ErrorEntry::new(status, ResponseShape::Empty)); }
        let mut kept: Option<ErrorEntry> = None;
        for (media, entry) in self.media(location, response) {
            let Some((media, class)) = media else {
                self.warn(&format!("{location}/content/*/*"), "an error body under `*/*` has no media type an error entry can declare; it is not imported into the error table".to_string());
                continue;
            };
            let shape = match class {
                MediaClass::Json => {
                    let mut shapes = self.json_shapes(&format!("{location}/content/{media}"), &media, entry, resolver);
                    if shapes.len() > 1 {
                        self.warn(&format!("{location}/content/{media}"), "an error entry holds one shape; the separable `oneOf` imports as one `serde_json::Value` JSON shape".to_string());
                        ResponseShape::Json(JSON_VALUE)
                    } else { shapes.remove(0).shape }
                }
                MediaClass::Text | MediaClass::Sse => ResponseShape::Text,
                MediaClass::Binary | MediaClass::Ndjson => ResponseShape::Binary,
            };
            let candidate = ErrorEntry { content_type: (default_media_type(&shape) != Some(media.as_str())).then_some(media), ..ErrorEntry::new(status, shape) };
            match &kept {
                None => kept = Some(candidate),
                Some(first) if first.shape == candidate.shape => {}
                Some(first) => self.warn(location, format!(
                    "error {status} declares media types needing different shapes; the error table holds one shape per status, so only `{}` is imported",
                    first.content_type.as_deref().or(default_media_type(&first.shape)).unwrap_or("no content"),
                )),
            }
        }
        kept
    }
}

fn classify_media(media: &str) -> MediaClass {
    let (kind, subtype) = media.split_once('/').unwrap_or((media, ""));
    match (kind, subtype) {
        ("text", "event-stream") => MediaClass::Sse,
        ("application", "x-ndjson" | "jsonl") => MediaClass::Ndjson,
        ("application", "json") => MediaClass::Json,
        ("application", subtype) if subtype.ends_with("+json") => MediaClass::Json,
        ("text", _) => MediaClass::Text,
        _ => MediaClass::Binary,
    }
}

/// A shape whose content type is recorded only when it differs from the
/// shape's default, so export followed by import is stable.
fn with_content_type(shape: ResponseShape, media: &str) -> SuccessShape {
    let declared = (default_media_type(&shape) != Some(media)).then(|| media.to_string());
    let mut success = SuccessShape::new(shape);
    success.content_type = declared;
    success
}

fn is_text_schema(schema: Option<&ReferenceOr<Schema>>, resolver: &RefResolver<'_>) -> bool {
    let Some(schema) = schema else { return false };
    let Ok(schema) = resolver.resolve_schema(schema) else { return false };
    matches!(&schema.schema_kind, SchemaKind::Type(openapiv3::Type::String(string))
        if matches!(string.format, openapiv3::VariantOrUnknownOrEmpty::Empty))
}

/// The R3b-separable members of a `oneOf`: by its `discriminator` when every
/// member is a component reference, else by one array and one object.
fn separate_one_of(schema: &Schema, members: &[ReferenceOr<Schema>], resolver: &RefResolver<'_>) -> Option<Vec<(crate::TypeRef, JsonTag)>> {
    if let Some(discriminator) = &schema.schema_data.discriminator {
        return members.iter().map(|member| {
            let ReferenceOr::Reference { reference } = member else { return None };
            let component = reference.strip_prefix("#/components/schemas/")?;
            // An unmapped member's value is its component name (OpenAPI 3.0.3).
            let value = discriminator.mapping.iter().find(|(_, target)| *target == reference || target.as_str() == component)
                .map_or(component, |(value, _)| value.as_str());
            let tag = JsonTag::Field { name: discriminator.property_name.clone(), value: serde_json::Value::String(value.to_string()) };
            Some((reference_from_schema(member, resolver, &mut vec![]), tag))
        }).collect();
    }
    let [first, second] = members else { return None };
    let kind = |member| match resolver.resolve_schema(member).map(|schema| &schema.schema_kind) {
        Ok(SchemaKind::Type(openapiv3::Type::Array(_))) => Some(JsonTag::Array),
        Ok(SchemaKind::Type(openapiv3::Type::Object(_))) => Some(JsonTag::Object),
        _ => None,
    };
    match (kind(first)?, kind(second)?) {
        (a, b) if a == b => None,
        (a, b) => Some(vec![(reference_from_schema(first, resolver, &mut vec![]), a), (reference_from_schema(second, resolver, &mut vec![]), b)]),
    }
}
