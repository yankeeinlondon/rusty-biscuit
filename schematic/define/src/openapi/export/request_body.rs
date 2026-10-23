//! Maps an endpoint's request body onto an OpenAPI `requestBody`.
//!
//! Value rules travel as schema keywords in OpenAPI 3.0 form: the endpoint's
//! body rules ([`crate::Endpoint::request_constraints`]) on a text body or an
//! inline (non-component) JSON body, and each form field's rules
//! ([`FormField::constraints`]) on its text or JSON part. A component body's
//! rules live on its registered schema, and binary content takes none. A
//! rule that restates or overlaps a keyword the value's type already
//! declares (an integer type's `minimum`, say) is exported in an `allOf`
//! beside the type, so neither narrows the other.

use indexmap::IndexMap;
use openapiv3::{Encoding, MediaType, ReferenceOr, RequestBody};
use serde_json::{Map, Value, json};

use super::conversion::{convert_schema_at, pointer_token};
use crate::constraints::{Constraints, EXACT_EXTENSION};
use crate::request::{ApiRequest, FormField, FormFieldKind};
use crate::TypeRef;

/// Maps request body to OpenAPI RequestBody, with its form fields' rules but
/// no body-level rules (see [`map_constrained_request_body`]).
pub(crate) fn map_request_body(request: &ApiRequest) -> Result<RequestBody, crate::openapi::OpenApiError> {
    map_constrained_request_body(request, None, "#/requestBody")
}

/// Maps a request body to an OpenAPI `requestBody` whose text or inline JSON
/// root carries `body_rules` and whose form fields carry their own rules.
/// `pointer` is the `requestBody`'s JSON pointer, which locates a schema
/// that fails to convert. Without any rules the result is the body's schema
/// alone.
///
/// ## Errors
///
/// [`crate::openapi::OpenApiError`] when a wire schema is not
/// OpenAPI-compatible or a rule does not convert to OpenAPI 3.0.3.
pub(super) fn map_constrained_request_body(
    request: &ApiRequest,
    body_rules: Option<&Constraints>,
    pointer: &str,
) -> Result<RequestBody, crate::openapi::OpenApiError> {
    let body_rules = body_rules.filter(|rules| !rules.is_empty());
    let at = |media_type: &str| format!("{pointer}/content/{}/schema", pointer_token(media_type));
    let content = match request {
        ApiRequest::Json(schema) => {
            let mut content = IndexMap::new();
            let rules = body_rules.filter(|_| !matches!(schema, TypeRef::Named(_)));
            content.insert(
                "application/json".to_string(),
                MediaType {
                    schema: Some(constrained_type_schema(schema, rules, &at("application/json"))?),
                    ..Default::default()
                },
            );
            content
        }
        ApiRequest::FormData { fields } => {
            let mut content = IndexMap::new();
            content.insert(
                "multipart/form-data".to_string(),
                MediaType {
                    schema: Some(ReferenceOr::Item(map_form_fields_to_schema(fields, &at("multipart/form-data"))?)),
                    encoding: multipart_encoding(fields),
                    ..Default::default()
                },
            );
            content
        }
        ApiRequest::UrlEncoded { fields } => {
            let mut content = IndexMap::new();
            content.insert(
                "application/x-www-form-urlencoded".to_string(),
                MediaType {
                    schema: Some(ReferenceOr::Item(map_form_fields_to_schema(fields, &at("application/x-www-form-urlencoded"))?)),
                    ..Default::default()
                },
            );
            content
        }
        ApiRequest::Text { content_type } => {
            let mut content = IndexMap::new();
            content.insert(
                content_type.clone(),
                MediaType {
                    schema: Some(ReferenceOr::Item(match body_rules {
                        Some(rules) => constrained_schema(json!({"type": "string"}), rules, &at(content_type))?,
                        None => openapiv3::Schema {
                            schema_data: openapiv3::SchemaData::default(),
                            schema_kind: openapiv3::SchemaKind::Type(openapiv3::Type::String(
                                openapiv3::StringType::default(),
                            )),
                        },
                    })),
                    ..Default::default()
                },
            );
            content
        }
        ApiRequest::Binary { content_type } => {
            let mut content = IndexMap::new();
            content.insert(
                content_type.clone(),
                MediaType {
                    schema: Some(ReferenceOr::Item(openapiv3::Schema {
                        schema_data: openapiv3::SchemaData::default(),
                        schema_kind: openapiv3::SchemaKind::Type(openapiv3::Type::String(
                            openapiv3::StringType {
                                format: openapiv3::VariantOrUnknownOrEmpty::Unknown(
                                    "binary".to_string(),
                                ),
                                ..Default::default()
                            },
                        )),
                    })),
                    ..Default::default()
                },
            );
            content
        }
    };

    Ok(RequestBody {
        description: None,
        content,
        required: true,
        ..Default::default()
    })
}

/// Maps form fields to an OpenAPI object schema located at `pointer`; a
/// text or JSON field's property schema carries the field's rules.
pub(super) fn map_form_fields_to_schema(fields: &[FormField], pointer: &str) -> Result<openapiv3::Schema, crate::openapi::OpenApiError> {
    let mut properties = IndexMap::new();
    let mut required = Vec::new();

    for field in fields {
        let rules = field.constraints.as_deref().filter(|rules| !rules.is_empty());
        let at = format!("{pointer}/properties/{}", pointer_token(&field.name));
        let field_schema = match &field.kind {
            FormFieldKind::Text if let Some(rules) = rules => {
                let mut schema = json!({"type": "string"});
                if let Some(description) = &field.description {
                    schema["description"] = Value::String(description.clone());
                }
                constrained_schema(schema, rules, &at)?
            }
            FormFieldKind::Text => openapiv3::Schema {
                schema_data: openapiv3::SchemaData {
                    description: field.description.clone(),
                    ..Default::default()
                },
                schema_kind: openapiv3::SchemaKind::Type(openapiv3::Type::String(
                    openapiv3::StringType::default(),
                )),
            },
            FormFieldKind::File { accept: _ } => openapiv3::Schema {
                schema_data: openapiv3::SchemaData {
                    description: field.description.clone(),
                    ..Default::default()
                },
                schema_kind: openapiv3::SchemaKind::Type(openapiv3::Type::String(
                    openapiv3::StringType {
                        format: openapiv3::VariantOrUnknownOrEmpty::Unknown("binary".to_string()),
                        ..Default::default()
                    },
                )),
            },
            FormFieldKind::Files { accept: _, min, max } => openapiv3::Schema {
                schema_data: openapiv3::SchemaData {
                    description: field.description.clone(),
                    ..Default::default()
                },
                schema_kind: openapiv3::SchemaKind::Type(openapiv3::Type::Array(
                    openapiv3::ArrayType {
                        items: Some(ReferenceOr::Item(Box::new(openapiv3::Schema {
                            schema_data: Default::default(),
                            schema_kind: openapiv3::SchemaKind::Type(openapiv3::Type::String(
                                openapiv3::StringType {
                                    format: openapiv3::VariantOrUnknownOrEmpty::Unknown(
                                        "binary".to_string(),
                                    ),
                                    ..Default::default()
                                },
                            )),
                        }))),
                        min_items: min.map(|min| min as usize),
                        max_items: max.map(|max| max as usize),
                        unique_items: false,
                    },
                )),
            },
            FormFieldKind::Json(schema) => openapiv3::Schema {
                schema_data: openapiv3::SchemaData {
                    description: field.description.clone(),
                    ..Default::default()
                },
                schema_kind: openapiv3::SchemaKind::AllOf {
                    all_of: vec![constrained_type_schema(schema, rules, &format!("{at}/allOf/0"))?],
                },
            },
        };

        properties.insert(
            field.name.clone(),
            ReferenceOr::Item(Box::new(field_schema)),
        );

        if field.required {
            required.push(field.name.clone());
        }
    }

    Ok(openapiv3::Schema {
        schema_data: openapiv3::SchemaData::default(),
        schema_kind: openapiv3::SchemaKind::Type(openapiv3::Type::Object(openapiv3::ObjectType {
            properties,
            required,
            ..Default::default()
        })),
    })
}

/// The schema of a JSON value of type `reference` carrying `rules`, at
/// `pointer`: the type's wire schema alone without rules.
fn constrained_type_schema(
    reference: &TypeRef,
    rules: Option<&Constraints>,
    pointer: &str,
) -> Result<ReferenceOr<openapiv3::Schema>, crate::openapi::OpenApiError> {
    match rules.filter(|rules| !rules.is_empty()) {
        Some(rules) => constrained_schema(reference.wire_schema(), rules, pointer).map(ReferenceOr::Item),
        None => super::responses::body_schema(reference),
    }
}

/// `schema` (JSON Schema) carrying `rules`, converted to OpenAPI 3.0.3 at
/// `pointer`.
fn constrained_schema(schema: Value, rules: &Constraints, pointer: &str) -> Result<openapiv3::Schema, crate::openapi::OpenApiError> {
    convert_schema_at(&with_rules(schema, rules.to_schema()), pointer)
}

/// `schema` with the keywords of `rules` added. An `items` object joins the
/// schema's own `items`; when any other rule overlaps a keyword the schema
/// already has (the same keyword, the other spelling of a bound, or `const`
/// beside `enum`), both apply as `allOf` members instead.
fn with_rules(schema: Value, rules: Map<String, Value>) -> Value {
    let Value::Object(mut schema) = schema else {
        return json!({"allOf": [schema, rules]});
    };
    let overlaps = |keyword: &str| -> &'static [&'static str] {
        match keyword {
            "minimum" | "exclusiveMinimum" => &["minimum", "exclusiveMinimum"],
            "maximum" | "exclusiveMaximum" => &["maximum", "exclusiveMaximum"],
            "enum" | "const" => &["enum", "const"],
            _ => &[],
        }
    };
    let conflicts = rules.iter().any(|(keyword, value)| {
        let nested_items = keyword == "items" && value.is_object() && schema.get("items").is_some_and(Value::is_object);
        keyword != EXACT_EXTENSION
            && !nested_items
            && (schema.contains_key(keyword) || overlaps(keyword).iter().any(|other| schema.contains_key(*other)))
    });
    if conflicts {
        return json!({"allOf": [schema, rules]});
    }
    for (keyword, value) in rules {
        match (schema.get_mut(&keyword), value) {
            (Some(existing), Value::Object(inner)) if keyword == "items" => {
                *existing = with_rules(existing.take(), inner);
            }
            (Some(Value::Object(existing)), Value::Object(inner)) if keyword == EXACT_EXTENSION => existing.extend(inner),
            (_, value) => {
                schema.insert(keyword, value);
            }
        }
    }
    Value::Object(schema)
}

/// The multipart `encoding` entries, in field order: each file part's
/// `accept` list as its `contentType`, and `application/json` for JSON parts.
/// Text parts and files accepting any type use the default and get no entry.
fn multipart_encoding(fields: &[FormField]) -> IndexMap<String, Encoding> {
    fields
        .iter()
        .filter_map(|field| {
            let content_type = match &field.kind {
                FormFieldKind::File { accept } | FormFieldKind::Files { accept, .. } if !accept.is_empty() => {
                    accept.join(", ")
                }
                FormFieldKind::Json(_) => "application/json".to_string(),
                _ => return None,
            };
            Some((field.name.clone(), Encoding { content_type: Some(content_type), ..Default::default() }))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn map_request_body_json() {
        let request = ApiRequest::json_type("CreateUserBody");
        let body = map_request_body(&request).unwrap();

        assert!(body.content.contains_key("application/json"));
        assert!(body.required);
    }

    #[test]
    fn map_request_body_form_data() {
        let request =
            ApiRequest::form_data(vec![FormField::file("document"), FormField::text("name")]);
        let body = map_request_body(&request).unwrap();

        assert!(body.content.contains_key("multipart/form-data"));
    }

    #[test]
    fn map_request_body_url_encoded() {
        let request = ApiRequest::url_encoded(vec![
            FormField::text("username"),
            FormField::text("password"),
        ]);
        let body = map_request_body(&request).unwrap();

        assert!(
            body.content
                .contains_key("application/x-www-form-urlencoded")
        );
    }

    #[test]
    fn map_request_body_text() {
        let request = ApiRequest::text("text/csv");
        let body = map_request_body(&request).unwrap();

        assert!(body.content.contains_key("text/csv"));
    }

    #[test]
    fn map_request_body_binary() {
        let request = ApiRequest::binary("application/octet-stream");
        let body = map_request_body(&request).unwrap();

        assert!(body.content.contains_key("application/octet-stream"));
    }
}
