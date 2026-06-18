use openapiv3::ReferenceOr;

use super::super::OpenApiError;

/// Validates that every `$ref` in the OpenAPI document points to a schema that exists
/// in `components.schemas`.
///
/// This prevents dangling references in generated OpenAPI artifacts.
pub(super) fn validate_ref_closure(openapi: &openapiv3::OpenAPI) -> Result<(), OpenApiError> {
    let schema_names = openapi
        .components
        .as_ref()
        .map(|c| {
            c.schemas
                .keys()
                .map(|k| k.as_str())
                .collect::<std::collections::HashSet<_>>()
        })
        .unwrap_or_default();

    let mut missing = Vec::new();
    let mut visitor = |reference: &str| {
        if let Some(name) = reference.strip_prefix("#/components/schemas/")
            && !schema_names.contains(name)
        {
            missing.push(name.to_string());
        }
    };

    for (_path, item) in &openapi.paths.paths {
        if let ReferenceOr::Item(item) = item {
            visit_operation(item.get.as_ref(), &mut visitor);
            visit_operation(item.post.as_ref(), &mut visitor);
            visit_operation(item.put.as_ref(), &mut visitor);
            visit_operation(item.patch.as_ref(), &mut visitor);
            visit_operation(item.delete.as_ref(), &mut visitor);
            visit_operation(item.head.as_ref(), &mut visitor);
            visit_operation(item.options.as_ref(), &mut visitor);
        }
    }

    if let Some(components) = &openapi.components {
        for (_name, schema) in &components.schemas {
            if let ReferenceOr::Item(schema) = schema {
                visit_schema(schema, &mut visitor);
            }
        }
    }

    if missing.is_empty() {
        Ok(())
    } else {
        missing.sort();
        missing.dedup();
        Err(OpenApiError::UnresolvedRefs { refs: missing })
    }
}

/// Visits all `$ref` references inside an `Operation`.
fn visit_operation<F: FnMut(&str)>(op: Option<&openapiv3::Operation>, visitor: &mut F) {
    let Some(op) = op else { return };

    if let Some(ReferenceOr::Item(body)) = &op.request_body {
        for (_content_type, media_type) in &body.content {
            if let Some(schema) = &media_type.schema {
                visit_ref_or_schema(schema, visitor);
            }
        }
    }

    for (_status, response) in &op.responses.responses {
        if let ReferenceOr::Item(response) = response {
            for (_content_type, media_type) in &response.content {
                if let Some(schema) = &media_type.schema {
                    visit_ref_or_schema(schema, visitor);
                }
            }
        }
    }
}

/// Visits all `$ref` references inside a `Schema` recursively.
fn visit_schema<F: FnMut(&str)>(schema: &openapiv3::Schema, visitor: &mut F) {
    match &schema.schema_kind {
        openapiv3::SchemaKind::Type(openapiv3::Type::Object(obj)) => {
            for (_prop, prop_schema) in &obj.properties {
                visit_ref_or_boxed_schema(prop_schema, visitor);
            }
            if let Some(additional) = &obj.additional_properties {
                match additional {
                    openapiv3::AdditionalProperties::Any(b) => {
                        if *b {
                            // any additional properties - nothing to visit
                        }
                    }
                    openapiv3::AdditionalProperties::Schema(s) => {
                        visit_boxed_ref_or_schema(s, visitor);
                    }
                }
            }
        }
        openapiv3::SchemaKind::Type(openapiv3::Type::Array(arr)) => {
            if let Some(items) = &arr.items {
                visit_ref_or_boxed_schema(items, visitor);
            }
        }
        openapiv3::SchemaKind::AllOf { all_of } => {
            for schema in all_of {
                visit_ref_or_schema(schema, visitor);
            }
        }
        openapiv3::SchemaKind::AnyOf { any_of } => {
            for schema in any_of {
                visit_ref_or_schema(schema, visitor);
            }
        }
        openapiv3::SchemaKind::OneOf { one_of } => {
            for schema in one_of {
                visit_ref_or_schema(schema, visitor);
            }
        }
        openapiv3::SchemaKind::Not { not } => {
            visit_boxed_ref_or_schema(not, visitor);
        }
        _ => {}
    }
}

/// Visits a `ReferenceOr<Box<Schema>>`.
fn visit_ref_or_boxed_schema<F: FnMut(&str)>(
    reference_or: &ReferenceOr<Box<openapiv3::Schema>>,
    visitor: &mut F,
) {
    match reference_or {
        ReferenceOr::Reference { reference } => visitor(reference),
        ReferenceOr::Item(schema) => visit_schema(schema, visitor),
    }
}

/// Visits a `ReferenceOr<Schema>`.
fn visit_ref_or_schema<F: FnMut(&str)>(
    reference_or: &ReferenceOr<openapiv3::Schema>,
    visitor: &mut F,
) {
    match reference_or {
        ReferenceOr::Reference { reference } => visitor(reference),
        ReferenceOr::Item(schema) => visit_schema(schema, visitor),
    }
}

/// Visits a `Box<ReferenceOr<Schema>>`.
fn visit_boxed_ref_or_schema<F: FnMut(&str)>(
    boxed: &ReferenceOr<openapiv3::Schema>,
    visitor: &mut F,
) {
    visit_ref_or_schema(boxed, visitor);
}
