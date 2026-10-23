//! Success shapes and the resolved error table as an OpenAPI responses map
//! (R21, R27, Default D-25).

use indexmap::IndexMap;
use openapiv3::{Discriminator, MediaType, ReferenceOr, Response, Responses, Schema, SchemaKind, StatusCode};

use crate::error_table::{ErrorEntry, ErrorTable, StatusMatch};
use crate::response::{JsonTag, ResponseShape, StreamFraming, SuccessShape, default_media_type, normalize_media_type};

/// The media range the R3c fallback shape exports under.
const FALLBACK_MEDIA: &str = "*/*";

/// The body schema of a JSON reference: nominal `$ref`s plus wrapper shape.
pub(super) fn body_schema(reference: &crate::TypeRef) -> Result<ReferenceOr<openapiv3::Schema>, crate::openapi::OpenApiError> {
    serde_json::from_value(reference.wire_schema()).map_err(|error| crate::openapi::OpenApiError::Validation {
        path: format!("{reference:?}"), message: format!("wire schema is not OpenAPI-compatible: {error}"),
    })
}

fn string_schema(binary: bool) -> ReferenceOr<Schema> {
    ReferenceOr::Item(Schema {
        schema_data: Default::default(),
        schema_kind: SchemaKind::Type(openapiv3::Type::String(openapiv3::StringType {
            format: if binary { openapiv3::VariantOrUnknownOrEmpty::Item(openapiv3::StringFormat::Binary) } else { openapiv3::VariantOrUnknownOrEmpty::Empty },
            ..Default::default()
        })),
    })
}

/// The schema of one body shape: the referenced component for JSON, the item
/// type for SSE and NDJSON, and a (binary) string otherwise. `None` for empty.
fn shape_schema(shape: &ResponseShape) -> Result<Option<ReferenceOr<Schema>>, crate::openapi::OpenApiError> {
    Ok(match shape {
        ResponseShape::Json(reference)
        | ResponseShape::Stream { framing: StreamFraming::Sse(reference) | StreamFraming::Ndjson(reference), .. } => Some(body_schema(reference)?),
        ResponseShape::Text => Some(string_schema(false)),
        ResponseShape::Binary | ResponseShape::Stream { .. } => Some(string_schema(true)),
        ResponseShape::Empty => None,
    })
}

/// The media key a body-bearing shape exports under.
fn content_key(declared: Option<&str>, shape: &ResponseShape) -> Option<String> {
    match declared {
        Some(declared) => Some(normalize_media_type(declared).unwrap_or_else(|_| declared.trim().to_ascii_lowercase())),
        None => default_media_type(shape).map(str::to_owned),
    }
}

/// The description a lone shape has always exported with.
fn shape_description(shape: &ResponseShape) -> &'static str {
    match shape {
        ResponseShape::Json(_) => "Successful JSON response",
        ResponseShape::Text => "Successful text response",
        ResponseShape::Binary => "Successful binary response",
        ResponseShape::Empty => "No content",
        ResponseShape::Stream { framing: StreamFraming::Sse(_), .. } => "Successful server-sent event stream",
        ResponseShape::Stream { framing: StreamFraming::Ndjson(_), .. } => "Successful NDJSON stream",
        ResponseShape::Stream { .. } => "Successful byte stream",
    }
}

/// The status keys a success shape exports under: its declared statuses,
/// else 204 for an empty shape and 200 for any other.
fn success_statuses(shape: &SuccessShape) -> Vec<u16> {
    if !shape.statuses.is_empty() { return shape.statuses.clone(); }
    if shape.shape == ResponseShape::Empty { vec![204] } else { vec![200] }
}

/// One media entry's members before they become a schema: a lone member
/// keeps its schema, several become a `oneOf`.
struct MediaMembers<'a> {
    schemas: Vec<ReferenceOr<Schema>>,
    tags: Vec<Option<&'a JsonTag>>,
}

/// Maps an endpoint's success shapes and its resolved error table to one
/// response object per status (R27).
///
/// Every shape under one status merges into that status's response object,
/// one `content` entry per media type; shapes sharing a status and media type
/// become a `oneOf` whose `discriminator` names the R3b field when every
/// member declares one. The fallback exports under `*/*`, and each shape's
/// note joins the description. Error keys follow [`error_keys`]; success
/// statuses are 2xx and error statuses 400–599, so the keys never collide.
///
/// ## Errors
///
/// [`crate::openapi::OpenApiError::Validation`] when a JSON reference's wire
/// schema is not OpenAPI-compatible.
pub(crate) fn map_responses(shapes: &[SuccessShape], errors: &ErrorTable) -> Result<Responses, crate::openapi::OpenApiError> {
    let mut by_status: IndexMap<u16, Vec<&SuccessShape>> = IndexMap::new();
    for shape in shapes {
        for status in success_statuses(shape) {
            by_status.entry(status).or_default().push(shape);
        }
    }

    let mut responses = IndexMap::new();
    for (status, members) in by_status {
        let mut media: IndexMap<String, MediaMembers<'_>> = IndexMap::new();
        for shape in &members {
            let key = if shape.fallback { Some(FALLBACK_MEDIA.to_string()) } else { content_key(shape.content_type.as_deref(), &shape.shape) };
            let (Some(key), Some(schema)) = (key, shape_schema(&shape.shape)?) else { continue };
            let entry = media.entry(key).or_insert_with(|| MediaMembers { schemas: vec![], tags: vec![] });
            entry.schemas.push(schema);
            entry.tags.push(shape.json_tag.as_ref());
        }
        let content = media.into_iter().map(|(key, members)| (key, MediaType { schema: Some(media_schema(members)), ..Default::default() })).collect();

        let base = match members.as_slice() {
            [only] => shape_description(&only.shape),
            _ => "Successful response",
        };
        let notes = members.iter().filter_map(|shape| shape.note.as_ref().map(|note| {
            let media = if shape.fallback { Some(FALLBACK_MEDIA.to_string()) } else { shape.media_type() };
            format!("{}: {note}", media.unwrap_or_else(|| "No content".to_string()))
        }));
        let description = std::iter::once(base.to_string()).chain(notes).collect::<Vec<_>>().join("\n\n");
        responses.insert(StatusCode::Code(status), ReferenceOr::Item(Response { description, content, ..Default::default() }));
    }

    for (key, entry) in error_keys(errors) {
        let mut content = IndexMap::new();
        if let (Some(media), Some(schema)) = (content_key(entry.content_type.as_deref(), &entry.shape), shape_schema(&entry.shape)?) {
            content.insert(media, MediaType { schema: Some(schema), ..Default::default() });
        }
        let description = format!("Declared error {}", entry.status);
        responses.insert(key, ReferenceOr::Item(Response { description, content, ..Default::default() }));
    }

    Ok(Responses { responses, ..Default::default() })
}

fn media_schema(members: MediaMembers<'_>) -> ReferenceOr<Schema> {
    let MediaMembers { mut schemas, tags } = members;
    if schemas.len() == 1 { return schemas.remove(0); }
    let field = match tags.first() {
        Some(Some(JsonTag::Field { name, .. })) if tags.iter().all(|tag| matches!(tag, Some(JsonTag::Field { name: other, .. }) if other == name)) => Some(name),
        _ => None,
    };
    let discriminator = field.map(|name| Discriminator {
        property_name: name.clone(),
        // OpenAPI maps string values to member references only.
        mapping: tags.iter().zip(&schemas).filter_map(|(tag, schema)| match (tag, schema) {
            (Some(JsonTag::Field { value: serde_json::Value::String(value), .. }), ReferenceOr::Reference { reference }) => Some((value.clone(), reference.clone())),
            _ => None,
        }).collect(),
        ..Default::default()
    });
    ReferenceOr::Item(Schema {
        schema_data: openapiv3::SchemaData { discriminator, ..Default::default() },
        schema_kind: SchemaKind::OneOf { one_of: schemas },
    })
}

/// The status keys of the resolved error table, with the entry
/// [`ErrorTable::lookup`] picks at each (Default D-25).
///
/// An exact entry keys its status. A class-aligned range (`400-499`,
/// `500-599`) keys `4XX`/`5XX` when it wins every status of its class;
/// otherwise, and for every other range, each status it matches keys the
/// winning entry, so the export represents lookup rather than the declared
/// overlap. Keys are ordered by their first status, exact before class.
pub(super) fn error_keys(table: &ErrorTable) -> Vec<(StatusCode, &ErrorEntry)> {
    let mut keyed: IndexMap<StatusCode, &ErrorEntry> = IndexMap::new();
    for entry in table.entries() {
        match entry.status {
            StatusMatch::Range { lo, hi } if lo % 100 == 0 && hi == lo + 99
                && (lo..=hi).all(|status| table.lookup(status).is_some_and(|winner| std::ptr::eq(winner, entry))) => {
                keyed.insert(StatusCode::Range(lo / 100), entry);
            }
            StatusMatch::Range { lo, hi } => {
                for status in lo..=hi {
                    if let Some(winner) = table.lookup(status) { keyed.insert(StatusCode::Code(status), winner); }
                }
            }
            StatusMatch::Exact(status) => {
                if let Some(winner) = table.lookup(status) { keyed.insert(StatusCode::Code(status), winner); }
            }
        }
    }
    let mut keys: Vec<_> = keyed.into_iter().collect();
    keys.sort_by_key(|(key, _)| match key { StatusCode::Code(code) => (*code, 0), StatusCode::Range(class) => (class * 100, 1) });
    keys
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lone(response: SuccessShape) -> Responses {
        map_responses(&[response], &ErrorTable::resolve(&[], &[], false)).unwrap()
    }

    #[test]
    fn map_responses_json() {
        let responses = lone(SuccessShape::json_type("UserResponse"));

        assert!(responses.responses.contains_key(&StatusCode::Code(200)));
    }

    #[test]
    fn map_responses_text() {
        let responses = lone(SuccessShape::text());

        let resp = responses.responses.get(&StatusCode::Code(200)).unwrap();
        if let ReferenceOr::Item(r) = resp {
            assert!(r.content.contains_key("text/plain"));
        }
    }

    #[test]
    fn map_responses_binary() {
        let responses = lone(SuccessShape::binary());

        let resp = responses.responses.get(&StatusCode::Code(200)).unwrap();
        if let ReferenceOr::Item(r) = resp {
            assert!(r.content.contains_key("application/octet-stream"));
        }
    }

    #[test]
    fn map_responses_empty() {
        let responses = lone(SuccessShape::empty());

        assert!(responses.responses.contains_key(&StatusCode::Code(204)));
    }

    #[test]
    fn a_class_range_superseded_in_part_expands_to_its_exact_statuses() {
        let api = [ErrorEntry::new(StatusMatch::class(4), ResponseShape::Text), ErrorEntry::new(StatusMatch::class(5), ResponseShape::Empty)];
        let own = [ErrorEntry::new(StatusMatch::exact(404), ResponseShape::Binary)];
        let table = ErrorTable::resolve(&api, &own, false);

        let keys = error_keys(&table);

        assert_eq!(keys.len(), 101, "400-499 expanded plus 5XX");
        assert!(keys.iter().all(|(key, _)| *key != StatusCode::Range(4)));
        assert_eq!(keys.iter().find(|(key, _)| *key == StatusCode::Code(404)).unwrap().1.shape, ResponseShape::Binary);
        assert_eq!(keys.iter().find(|(key, _)| *key == StatusCode::Code(418)).unwrap().1.shape, ResponseShape::Text);
        assert_eq!(keys.last().unwrap().0, StatusCode::Range(5));
    }

    #[test]
    fn a_range_that_is_not_a_class_exports_its_exact_statuses() {
        let table = ErrorTable::resolve(&[ErrorEntry::new(StatusMatch::range(429, 431), ResponseShape::Empty)], &[], false);

        let keys: Vec<StatusCode> = error_keys(&table).into_iter().map(|(key, _)| key).collect();

        assert_eq!(keys, [StatusCode::Code(429), StatusCode::Code(430), StatusCode::Code(431)]);
    }
}
