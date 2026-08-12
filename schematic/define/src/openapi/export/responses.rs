use indexmap::IndexMap;
use openapiv3::{MediaType, ReferenceOr, Response, Responses, StatusCode};

use crate::response::ApiResponse;

/// Builds the schema entry for a named body or response type.
///
/// A path-shaped name (`serde_json::Value`) names a foreign Rust type that has
/// no component entry, so it is emitted as an unconstrained inline schema. A
/// `$ref` to it would dangle and fail export's reference-closure check.
pub(super) fn schema_reference(type_name: &str) -> ReferenceOr<openapiv3::Schema> {
    if type_name.contains("::") {
        return ReferenceOr::Item(openapiv3::Schema {
            schema_data: Default::default(),
            schema_kind: openapiv3::SchemaKind::Any(openapiv3::AnySchema::default()),
        });
    }

    ReferenceOr::Reference {
        reference: format!("#/components/schemas/{type_name}"),
    }
}

/// Maps response type to OpenAPI Responses.
pub(super) fn map_responses(response: &ApiResponse) -> Responses {
    let mut responses = IndexMap::new();

    let (status_code, content_type, schema) = match response {
        ApiResponse::Json(schema_def) => (
            "200",
            "application/json",
            Some(schema_reference(&schema_def.type_name)),
        ),
        ApiResponse::Text => (
            "200",
            "text/plain",
            Some(ReferenceOr::Item(openapiv3::Schema {
                schema_data: Default::default(),
                schema_kind: openapiv3::SchemaKind::Type(openapiv3::Type::String(
                    openapiv3::StringType::default(),
                )),
            })),
        ),
        ApiResponse::Binary => (
            "200",
            "application/octet-stream",
            Some(ReferenceOr::Item(openapiv3::Schema {
                schema_data: Default::default(),
                schema_kind: openapiv3::SchemaKind::Type(openapiv3::Type::String(
                    openapiv3::StringType {
                        format: openapiv3::VariantOrUnknownOrEmpty::Unknown("binary".to_string()),
                        ..Default::default()
                    },
                )),
            })),
        ),
        ApiResponse::Empty => ("204", "", None),
    };

    let mut content = IndexMap::new();
    if let Some(schema) = schema {
        content.insert(
            content_type.to_string(),
            MediaType {
                schema: Some(schema),
                ..Default::default()
            },
        );
    }

    let response_obj = Response {
        description: match response {
            ApiResponse::Json(_) => "Successful JSON response".to_string(),
            ApiResponse::Text => "Successful text response".to_string(),
            ApiResponse::Binary => "Successful binary response".to_string(),
            ApiResponse::Empty => "No content".to_string(),
        },
        content: if content.is_empty() {
            IndexMap::new()
        } else {
            content
        },
        ..Default::default()
    };

    responses.insert(
        StatusCode::Code(status_code.parse().unwrap_or(200)),
        ReferenceOr::Item(response_obj),
    );

    Responses {
        responses,
        ..Default::default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn map_responses_json() {
        let response = ApiResponse::json_type("UserResponse");
        let responses = map_responses(&response);

        assert!(responses.responses.contains_key(&StatusCode::Code(200)));
    }

    #[test]
    fn map_responses_text() {
        let response = ApiResponse::Text;
        let responses = map_responses(&response);

        let resp = responses.responses.get(&StatusCode::Code(200)).unwrap();
        if let ReferenceOr::Item(r) = resp {
            assert!(r.content.contains_key("text/plain"));
        }
    }

    #[test]
    fn map_responses_binary() {
        let response = ApiResponse::Binary;
        let responses = map_responses(&response);

        let resp = responses.responses.get(&StatusCode::Code(200)).unwrap();
        if let ReferenceOr::Item(r) = resp {
            assert!(r.content.contains_key("application/octet-stream"));
        }
    }

    #[test]
    fn map_responses_empty() {
        let response = ApiResponse::Empty;
        let responses = map_responses(&response);

        assert!(responses.responses.contains_key(&StatusCode::Code(204)));
    }
}
