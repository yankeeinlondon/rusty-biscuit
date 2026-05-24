//! Schema conversion utilities.
//!
//! Converts schemars JSON Schema format to OpenAPI 3.0 schema format.

use schemars::Schema;

/// Converts a schemars `Schema` to an `openapiv3::Schema`.
///
/// This function handles the conversion from schemars JSON Schema format
/// to OpenAPI 3.0 schema format. The conversion preserves:
/// - Type information (string, number, object, array, etc.)
/// - Property definitions
/// - Required fields
/// - Descriptions from doc comments
pub(crate) fn convert_schema_to_openapi(schema: &Schema) -> openapiv3::Schema {
    // Get the underlying JSON value
    let json_value = schema.as_value();
    convert_json_schema_to_openapi(json_value)
}

/// Converts a JSON Schema value to an OpenAPI schema.
pub(crate) fn convert_json_schema_to_openapi(value: &serde_json::Value) -> openapiv3::Schema {
    use openapiv3::{ObjectType, Schema, SchemaData, SchemaKind, Type};

    // Handle boolean schemas
    if let Some(b) = value.as_bool() {
        return if b {
            // true = any schema
            Schema {
                schema_data: SchemaData::default(),
                schema_kind: SchemaKind::Any(openapiv3::AnySchema::default()),
            }
        } else {
            // false = never matches (we'll represent as object with no valid values)
            Schema {
                schema_data: SchemaData::default(),
                schema_kind: SchemaKind::Not {
                    not: Box::new(openapiv3::ReferenceOr::Item(Schema {
                        schema_data: SchemaData::default(),
                        schema_kind: SchemaKind::Any(openapiv3::AnySchema::default()),
                    })),
                },
            }
        };
    }

    let obj = match value.as_object() {
        Some(obj) => obj,
        None => {
            return Schema {
                schema_data: SchemaData::default(),
                schema_kind: SchemaKind::Any(openapiv3::AnySchema::default()),
            };
        }
    };

    // Extract schema data (metadata)
    let data = SchemaData {
        title: obj.get("title").and_then(|v| v.as_str()).map(String::from),
        description: obj
            .get("description")
            .and_then(|v| v.as_str())
            .map(String::from),
        ..Default::default()
    };

    // Handle $ref (references to definitions)
    if let Some(ref_value) = obj.get("$ref")
        && let Some(ref_str) = ref_value.as_str()
    {
        // Convert schemars $ref format to OpenAPI $ref format
        let openapi_ref = ref_str.replace("#/$defs/", "#/components/schemas/");
        let openapi_ref = openapi_ref.replace("#/definitions/", "#/components/schemas/");
        return Schema {
            schema_data: data,
            schema_kind: SchemaKind::AllOf {
                all_of: vec![openapiv3::ReferenceOr::Reference {
                    reference: openapi_ref,
                }],
            },
        };
    }

    // Determine the schema kind from type
    let schema_kind = if let Some(type_value) = obj.get("type") {
        match type_value.as_str() {
            Some("object") => {
                let properties = obj
                    .get("properties")
                    .and_then(|p| p.as_object())
                    .map(|props| {
                        props
                            .iter()
                            .map(|(k, v)| {
                                let prop_schema = convert_json_schema_to_openapi(v);
                                (
                                    k.clone(),
                                    openapiv3::ReferenceOr::Item(Box::new(prop_schema)),
                                )
                            })
                            .collect()
                    })
                    .unwrap_or_default();

                let required = obj
                    .get("required")
                    .and_then(|r| r.as_array())
                    .map(|arr| {
                        arr.iter()
                            .filter_map(|v| v.as_str().map(String::from))
                            .collect()
                    })
                    .unwrap_or_default();

                SchemaKind::Type(Type::Object(ObjectType {
                    properties,
                    required,
                    ..Default::default()
                }))
            }
            Some("array") => {
                let items = obj.get("items").map(|items_schema| {
                    let item_schema = convert_json_schema_to_openapi(items_schema);
                    openapiv3::ReferenceOr::Item(Box::new(item_schema))
                });
                SchemaKind::Type(Type::Array(openapiv3::ArrayType {
                    items,
                    min_items: None,
                    max_items: None,
                    unique_items: false,
                }))
            }
            Some("string") => {
                let format = obj
                    .get("format")
                    .and_then(|f| f.as_str())
                    .map(|s| openapiv3::VariantOrUnknownOrEmpty::Unknown(s.to_string()))
                    .unwrap_or(openapiv3::VariantOrUnknownOrEmpty::Empty);
                SchemaKind::Type(Type::String(openapiv3::StringType {
                    format,
                    ..Default::default()
                }))
            }
            Some("integer") => {
                let format = obj
                    .get("format")
                    .and_then(|f| f.as_str())
                    .map(|s| openapiv3::VariantOrUnknownOrEmpty::Unknown(s.to_string()))
                    .unwrap_or(openapiv3::VariantOrUnknownOrEmpty::Empty);
                SchemaKind::Type(Type::Integer(openapiv3::IntegerType {
                    format,
                    ..Default::default()
                }))
            }
            Some("number") => {
                let format = obj
                    .get("format")
                    .and_then(|f| f.as_str())
                    .map(|s| openapiv3::VariantOrUnknownOrEmpty::Unknown(s.to_string()))
                    .unwrap_or(openapiv3::VariantOrUnknownOrEmpty::Empty);
                SchemaKind::Type(Type::Number(openapiv3::NumberType {
                    format,
                    ..Default::default()
                }))
            }
            Some("boolean") => SchemaKind::Type(Type::Boolean(openapiv3::BooleanType::default())),
            _ => SchemaKind::Any(openapiv3::AnySchema::default()),
        }
    } else if obj.contains_key("properties") {
        // Treat as object if it has properties but no explicit type
        let properties = obj
            .get("properties")
            .and_then(|p| p.as_object())
            .map(|props| {
                props
                    .iter()
                    .map(|(k, v)| {
                        let prop_schema = convert_json_schema_to_openapi(v);
                        (
                            k.clone(),
                            openapiv3::ReferenceOr::Item(Box::new(prop_schema)),
                        )
                    })
                    .collect()
            })
            .unwrap_or_default();

        let required = obj
            .get("required")
            .and_then(|r| r.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().map(String::from))
                    .collect()
            })
            .unwrap_or_default();

        SchemaKind::Type(Type::Object(ObjectType {
            properties,
            required,
            ..Default::default()
        }))
    } else {
        SchemaKind::Any(openapiv3::AnySchema::default())
    };

    Schema {
        schema_data: data,
        schema_kind,
    }
}
