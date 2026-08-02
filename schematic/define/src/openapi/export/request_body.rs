use indexmap::IndexMap;
use openapiv3::{MediaType, ReferenceOr, RequestBody};

use crate::request::{ApiRequest, FormField, FormFieldKind};

/// Maps request body to OpenAPI RequestBody.
pub(super) fn map_request_body(request: &ApiRequest) -> RequestBody {
    let content = match request {
        ApiRequest::Json(schema) => {
            let mut content = IndexMap::new();
            content.insert(
                "application/json".to_string(),
                MediaType {
                    schema: Some(super::responses::schema_reference(&schema.type_name)),
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
                    schema: Some(ReferenceOr::Item(map_form_fields_to_schema(fields))),
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
                    schema: Some(ReferenceOr::Item(map_form_fields_to_schema(fields))),
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
                    schema: Some(ReferenceOr::Item(openapiv3::Schema {
                        schema_data: openapiv3::SchemaData::default(),
                        schema_kind: openapiv3::SchemaKind::Type(openapiv3::Type::String(
                            openapiv3::StringType::default(),
                        )),
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

    RequestBody {
        description: None,
        content,
        required: true,
        ..Default::default()
    }
}

/// Maps form fields to an OpenAPI schema.
pub(super) fn map_form_fields_to_schema(fields: &[FormField]) -> openapiv3::Schema {
    let mut properties = IndexMap::new();
    let mut required = Vec::new();

    for field in fields {
        let field_schema = match &field.kind {
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
            FormFieldKind::Files {
                accept: _,
                min: _,
                max: _,
            } => openapiv3::Schema {
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
                        min_items: None,
                        max_items: None,
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
                    all_of: vec![super::responses::schema_reference(&schema.type_name)],
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

    openapiv3::Schema {
        schema_data: openapiv3::SchemaData::default(),
        schema_kind: openapiv3::SchemaKind::Type(openapiv3::Type::Object(openapiv3::ObjectType {
            properties,
            required,
            ..Default::default()
        })),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn map_request_body_json() {
        let request = ApiRequest::json_type("CreateUserBody");
        let body = map_request_body(&request);

        assert!(body.content.contains_key("application/json"));
        assert!(body.required);
    }

    #[test]
    fn map_request_body_form_data() {
        let request =
            ApiRequest::form_data(vec![FormField::file("document"), FormField::text("name")]);
        let body = map_request_body(&request);

        assert!(body.content.contains_key("multipart/form-data"));
    }

    #[test]
    fn map_request_body_url_encoded() {
        let request = ApiRequest::url_encoded(vec![
            FormField::text("username"),
            FormField::text("password"),
        ]);
        let body = map_request_body(&request);

        assert!(
            body.content
                .contains_key("application/x-www-form-urlencoded")
        );
    }

    #[test]
    fn map_request_body_text() {
        let request = ApiRequest::text("text/csv");
        let body = map_request_body(&request);

        assert!(body.content.contains_key("text/csv"));
    }

    #[test]
    fn map_request_body_binary() {
        let request = ApiRequest::binary("application/octet-stream");
        let body = map_request_body(&request);

        assert!(body.content.contains_key("application/octet-stream"));
    }
}
