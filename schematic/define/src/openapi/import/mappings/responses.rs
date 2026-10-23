use openapiv3::{ReferenceOr, Schema};

use super::super::diagnostics::OpenApiDiagnostic;
use super::super::naming::sanitize_rust_ident;
use super::super::resolver::RefResolver;
use super::super::{ContentPreference, OpenApiImportOptions};
use crate::request::{ApiRequest, FormField, FormFieldKind};
use crate::{PrimitiveType, TypeRef};

/// Reference used when a body or response has no named schema to reference.
pub(super) const JSON_VALUE: TypeRef = TypeRef::Primitive(PrimitiveType::Json);

/// Maps an operation's request body.
///
/// `location` is the operation's JSON pointer (`#/paths/{path}/{method}`);
/// form diagnostics are located beneath it.
pub fn map_request_body(
    body: &Option<ReferenceOr<openapiv3::RequestBody>>,
    location: &str,
    resolver: &RefResolver,
    diagnostics: &mut Vec<OpenApiDiagnostic>,
    options: &OpenApiImportOptions,
) -> Option<ApiRequest> {
    let body = match body {
        Some(ReferenceOr::Item(b)) => b,
        Some(ReferenceOr::Reference { reference }) => {
            diagnostics.push(OpenApiDiagnostic::warn(
                reference.clone(),
                "Request body references not supported".to_string(),
            ));
            return None;
        }
        None => return None,
    };
    if body.content.is_empty() {
        diagnostics.push(OpenApiDiagnostic::error(
            format!("{location}/requestBody/content"),
            "a request body declares no media type; the body is omitted".to_string(),
        ));
        return None;
    }

    let content_types: Vec<_> = body.content.keys().collect();
    let selected = select_content_type(&content_types, options);

    if let Some(ct) = selected
        && let Some(media_type) = body.content.get(ct)
    {
        for other_ct in &content_types {
            if *other_ct != ct {
                diagnostics.push(OpenApiDiagnostic::info(
                    "request body".to_string(),
                    format!("Also supports content type: {}", other_ct),
                ));
            }
        }

        let media_location = format!("{location}/requestBody/content/{}", pointer_token(ct));
        return map_media_type_to_request(ct, media_type, &media_location, resolver, diagnostics);
    }

    None
}

/// Escapes one RFC 6901 reference token.
fn pointer_token(token: &str) -> String {
    token.replace('~', "~0").replace('/', "~1")
}

fn map_media_type_to_request(
    content_type: &str,
    media_type: &openapiv3::MediaType,
    media_location: &str,
    resolver: &RefResolver,
    diagnostics: &mut Vec<OpenApiDiagnostic>,
) -> Option<ApiRequest> {
    let ct_lower = content_type.to_lowercase();

    if ct_lower.contains("json") {
        if let Some(ref schema) = media_type.schema {
            return Some(ApiRequest::json(reference_from_schema(schema, resolver, diagnostics)));
        }
    } else if ct_lower.contains("octet-stream") || ct_lower.contains("binary") {
        return Some(ApiRequest::binary(content_type));
    } else if ct_lower.contains("form-data") {
        return map_form_fields(media_type, media_location, resolver, diagnostics, FormFlavor::Multipart);
    } else if ct_lower.contains("x-www-form-urlencoded") {
        return map_form_fields(media_type, media_location, resolver, diagnostics, FormFlavor::UrlEncoded);
    } else if ct_lower.contains("text") {
        return Some(ApiRequest::text(content_type));
    }

    None
}

/// Which form encoding a set of fields is being mapped for.
#[derive(Clone, Copy, PartialEq, Eq)]
enum FormFlavor {
    /// `multipart/form-data`, which can carry file parts.
    Multipart,
    /// `application/x-www-form-urlencoded`, which cannot.
    UrlEncoded,
}

/// Where one form part's facts live in the source document.
struct PartLocation {
    /// The part's property schema.
    schema: String,
    /// The part's `encoding` entry.
    encoding: String,
}

/// Derives form fields from a form media type's schema.
///
/// Returning `None` here would drop the body entirely and generate a request
/// struct with no way to send one, so an unusable schema is reported as a
/// warning rather than silently discarded. A part that cannot be mapped at all
/// (an unresolved reference) is reported as an error and skipped.
fn map_form_fields(
    media_type: &openapiv3::MediaType,
    media_location: &str,
    resolver: &RefResolver,
    diagnostics: &mut Vec<OpenApiDiagnostic>,
    flavor: FormFlavor,
) -> Option<ApiRequest> {
    let schema_ref = media_type.schema.as_ref()?;
    let schema = match resolver.resolve_schema(schema_ref) {
        Ok(schema) => schema,
        Err(e) => {
            diagnostics.push(OpenApiDiagnostic::warn(
                format!("{media_location}/schema"),
                format!("Failed to resolve form schema: {e}"),
            ));
            return None;
        }
    };
    // A referenced form schema's properties live in its component.
    let object_location = match schema_ref {
        ReferenceOr::Reference { reference } => reference.clone(),
        ReferenceOr::Item(_) => format!("{media_location}/schema"),
    };

    let object = match &schema.schema_kind {
        openapiv3::SchemaKind::Type(openapiv3::Type::Object(object)) => object,
        _ => {
            diagnostics.push(OpenApiDiagnostic::warn(
                object_location,
                "Form schema is not an object; body omitted".to_string(),
            ));
            return None;
        }
    };

    for (name, encoding) in &media_type.encoding {
        let location = format!("{media_location}/encoding/{}", pointer_token(name));
        if !object.properties.contains_key(name) {
            diagnostics.push(OpenApiDiagnostic::warn(
                location.clone(),
                format!("Encoding names no form property \"{name}\"; ignored"),
            ));
        }
        warn_unrepresentable_encoding(encoding, &location, diagnostics);
    }

    let fields: Vec<FormField> = object
        .properties
        .iter()
        .filter_map(|(name, property)| {
            let location = PartLocation {
                schema: format!("{object_location}/properties/{}", pointer_token(name)),
                encoding: format!("{media_location}/encoding/{}", pointer_token(name)),
            };
            let required = object.required.contains(name);
            let encoding = media_type.encoding.get(name);
            map_form_field(name, property, encoding, &location, resolver, diagnostics, flavor, required)
        })
        .collect();

    if fields.is_empty() {
        diagnostics.push(OpenApiDiagnostic::warn(
            object_location,
            "Form schema declares no properties; body omitted".to_string(),
        ));
        return None;
    }

    Some(match flavor {
        FormFlavor::Multipart => ApiRequest::form_data(fields),
        FormFlavor::UrlEncoded => ApiRequest::url_encoded(fields),
    })
}

/// Reports the `encoding` keys [`FormField`] has no place for: per-part
/// headers and the URL-encoding serialization controls.
fn warn_unrepresentable_encoding(
    encoding: &openapiv3::Encoding,
    location: &str,
    diagnostics: &mut Vec<OpenApiDiagnostic>,
) {
    let present = [
        ("headers", !encoding.headers.is_empty()),
        ("style", encoding.style.is_some()),
        ("explode", encoding.explode),
        ("allowReserved", encoding.allow_reserved),
    ];
    for (key, _) in present.into_iter().filter(|(_, present)| *present) {
        diagnostics.push(OpenApiDiagnostic::warn(
            format!("{location}/{key}"),
            format!("Form part encoding `{key}` cannot be represented; ignored"),
        ));
    }
}

/// Maps a single form property to a [`FormField`], or `None` when the part
/// cannot be mapped at all.
///
/// `string` + `format: binary` is the OpenAPI spelling of a file part; an array
/// of those is a repeated part sharing one name (RFC 7578). URL-encoded forms
/// have no file concept, so every property there stays a text field.
#[allow(clippy::too_many_arguments)]
fn map_form_field(
    name: &str,
    property: &ReferenceOr<Box<Schema>>,
    encoding: Option<&openapiv3::Encoding>,
    location: &PartLocation,
    resolver: &RefResolver,
    diagnostics: &mut Vec<OpenApiDiagnostic>,
    flavor: FormFlavor,
    required: bool,
) -> Option<FormField> {
    let shape = classify_form_property(property, location, resolver, diagnostics)?;
    let kind = match flavor {
        FormFlavor::UrlEncoded => {
            if matches!(shape, FormPropertyShape::File | FormPropertyShape::Files { .. }) {
                diagnostics.push(OpenApiDiagnostic::warn(
                    location.schema.clone(),
                    "URL-encoded forms cannot carry files; binary property imported as text".to_string(),
                ));
            }
            FormFieldKind::Text
        }
        FormFlavor::Multipart => {
            let content_type = encoding.and_then(|encoding| encoding.content_type.as_deref());
            multipart_kind(shape, content_type, location, diagnostics)
        }
    };

    let field = FormField { name: name.to_string(), kind, required: true, description: None };
    let field = if required { field } else { field.optional() };

    Some(match describe_form_property(property, resolver) {
        Some(description) => field.with_description(description),
        None => field,
    })
}

/// Combines a part's schema shape with its `encoding.contentType`.
///
/// A JSON content type makes any non-file part a JSON part; on a file part
/// the content type becomes `accept`. Without one, a part follows OpenAPI's
/// default encoding: a primitive (or a composition of primitives) is a text
/// part, and an object-valued part, which would default to JSON, is reported
/// and kept as text, the part's form before its encoding was read. Anything
/// else the model cannot carry is reported.
fn multipart_kind(
    shape: FormPropertyShape,
    content_type: Option<&str>,
    location: &PartLocation,
    diagnostics: &mut Vec<OpenApiDiagnostic>,
) -> FormFieldKind {
    let content_type_location = format!("{}/contentType", location.encoding);
    let media_types: Option<Vec<String>> = content_type.map(|content_type| {
        content_type
            .split(',')
            .map(str::trim)
            .filter(|media_type| !media_type.is_empty())
            .map(str::to_string)
            .collect()
    });
    if media_types.as_ref().is_some_and(Vec::is_empty) {
        diagnostics.push(OpenApiDiagnostic::error(
            content_type_location.clone(),
            "Form part encoding declares an empty contentType; ignored".to_string(),
        ));
    }
    let media_types = media_types.unwrap_or_default();
    let declares_json = !media_types.is_empty() && media_types.iter().all(|media_type| is_json_media_type(media_type));

    let object_valued = match shape {
        FormPropertyShape::File => return FormFieldKind::File { accept: media_types },
        FormPropertyShape::Files { min, max } => return FormFieldKind::Files { accept: media_types, min, max },
        FormPropertyShape::Named { reference, .. } if declares_json => return FormFieldKind::Json(reference),
        _ if declares_json => return FormFieldKind::Json(JSON_VALUE),
        FormPropertyShape::TextArray => {
            diagnostics.push(OpenApiDiagnostic::warn(
                location.schema.clone(),
                "Array form part whose items are not files cannot be represented; imported as one text part".to_string(),
            ));
            return FormFieldKind::Text;
        }
        FormPropertyShape::Named { object_valued, .. } | FormPropertyShape::Composite { object_valued } => object_valued,
        FormPropertyShape::Text => false,
    };
    if object_valued {
        diagnostics.push(OpenApiDiagnostic::warn(
            location.schema.clone(),
            "Object-valued form part has no `application/json` encoding; imported as a text part".to_string(),
        ));
    } else if let Some(media_type) = media_types.iter().find(|media_type| !is_plain_text(media_type)) {
        diagnostics.push(OpenApiDiagnostic::warn(
            content_type_location,
            format!("Text form part content type \"{media_type}\" cannot be represented; imported as text"),
        ));
    }
    FormFieldKind::Text
}

/// The essence (type/subtype, lowercased, parameters dropped) of a media type.
fn media_type_essence(media_type: &str) -> String {
    media_type.split(';').next().unwrap_or_default().trim().to_ascii_lowercase()
}

/// `application/json` or any `+json` structured-syntax suffix.
fn is_json_media_type(media_type: &str) -> bool {
    let essence = media_type_essence(media_type);
    essence == "application/json" || essence.ends_with("+json")
}

/// `text/plain`, the default content type of a non-file primitive part.
fn is_plain_text(media_type: &str) -> bool {
    media_type_essence(media_type) == "text/plain"
}

/// The multipart shape a form property maps to.
enum FormPropertyShape {
    /// A single file part.
    File,
    /// Repeated file parts sharing one name.
    Files { min: Option<u32>, max: Option<u32> },
    /// A part referencing a named component (`$ref`, or `allOf: [$ref]`).
    Named { reference: TypeRef, object_valued: bool },
    /// An inline object or composition; not `object_valued` when every
    /// alternative is a primitive.
    Composite { object_valued: bool },
    /// An array whose items are not files.
    TextArray,
    /// A plain text part.
    Text,
}

/// Classifies a form property's schema, or `None` (with an error) when a
/// reference the classification depends on does not resolve.
fn classify_form_property(
    property: &ReferenceOr<Box<Schema>>,
    location: &PartLocation,
    resolver: &RefResolver,
    diagnostics: &mut Vec<OpenApiDiagnostic>,
) -> Option<FormPropertyShape> {
    let schema = match property {
        ReferenceOr::Reference { reference } => {
            let resolved = resolver.resolve_boxed_schema(property).map(|_| ());
            return named_part(reference, resolved, &location.schema, resolver, diagnostics);
        }
        ReferenceOr::Item(schema) => schema,
    };

    Some(match &schema.schema_kind {
        openapiv3::SchemaKind::Type(openapiv3::Type::String(string)) => {
            if is_binary_format(string) {
                FormPropertyShape::File
            } else {
                FormPropertyShape::Text
            }
        }
        openapiv3::SchemaKind::Type(openapiv3::Type::Array(array)) => {
            let items_are_binary = match &array.items {
                Some(items) => match resolver.resolve_boxed_schema(items) {
                    Ok(items) => matches!(
                        &items.schema_kind,
                        openapiv3::SchemaKind::Type(openapiv3::Type::String(string))
                            if is_binary_format(string)
                    ),
                    Err(error) => {
                        diagnostics.push(OpenApiDiagnostic::error(
                            format!("{}/items", location.schema),
                            format!("Unresolved form part items reference: {error}; part skipped"),
                        ));
                        return None;
                    }
                },
                None => false,
            };

            if items_are_binary {
                FormPropertyShape::Files {
                    min: array.min_items.and_then(|min| u32::try_from(min).ok()),
                    max: array.max_items.and_then(|max| u32::try_from(max).ok()),
                }
            } else {
                FormPropertyShape::TextArray
            }
        }
        openapiv3::SchemaKind::AllOf { all_of } if all_of.len() == 1 => match &all_of[0] {
            member @ ReferenceOr::Reference { reference } => {
                let resolved = resolver.resolve_schema(member).map(|_| ());
                return named_part(reference, resolved, &format!("{}/allOf/0", location.schema), resolver, diagnostics);
            }
            ReferenceOr::Item(member) => FormPropertyShape::Composite { object_valued: object_valued(member, resolver, 0) },
        },
        openapiv3::SchemaKind::Type(openapiv3::Type::Object(_))
        | openapiv3::SchemaKind::AllOf { .. }
        | openapiv3::SchemaKind::OneOf { .. }
        | openapiv3::SchemaKind::AnyOf { .. }
        | openapiv3::SchemaKind::Any(_) => FormPropertyShape::Composite { object_valued: object_valued(schema, resolver, 0) },
        _ => FormPropertyShape::Text,
    })
}

/// Whether a part's value is an object (or array) rather than a primitive,
/// for any alternative of a composition. An unresolvable reference counts as
/// a primitive; the reference itself is reported where it is classified.
fn object_valued(schema: &Schema, resolver: &RefResolver, depth: usize) -> bool {
    use openapiv3::{SchemaKind, Type};
    if depth > super::super::MAX_NESTING_DEPTH {
        return false;
    }
    let any_member = |members: &[ReferenceOr<Schema>]| {
        members.iter().any(|member| resolver.resolve_schema(member).is_ok_and(|member| object_valued(member, resolver, depth + 1)))
    };
    match &schema.schema_kind {
        SchemaKind::Type(Type::Object(_) | Type::Array(_)) => true,
        SchemaKind::Type(_) | SchemaKind::Not { .. } => false,
        SchemaKind::OneOf { one_of: members } | SchemaKind::AnyOf { any_of: members } | SchemaKind::AllOf { all_of: members } => any_member(members),
        SchemaKind::Any(any) => !any.properties.is_empty() || any.additional_properties.is_some() || any_member(&any.one_of) || any_member(&any.any_of) || any_member(&any.all_of),
    }
}

/// A part that references a component, or `None` (with an error) when the
/// reference does not resolve.
fn named_part(
    reference: &str,
    resolved: Result<(), super::super::resolver::ResolveError>,
    location: &str,
    resolver: &RefResolver,
    diagnostics: &mut Vec<OpenApiDiagnostic>,
) -> Option<FormPropertyShape> {
    match resolved {
        // Resolution succeeded, so the reference has the component prefix.
        Ok(()) => {
            let schema = ReferenceOr::Reference { reference: reference.to_string() };
            let object_valued = resolver.resolve_schema(&schema).is_ok_and(|schema| object_valued(schema, resolver, 0));
            Some(FormPropertyShape::Named {
                reference: TypeRef::Named(sanitize_rust_ident(reference.trim_start_matches("#/components/schemas/"))),
                object_valued,
            })
        }
        Err(error) => {
            diagnostics.push(OpenApiDiagnostic::error(
                location.to_string(),
                format!("Unresolved form part reference \"{reference}\": {error}; part skipped"),
            ));
            None
        }
    }
}

/// Reports whether a string schema carries a binary (file) format.
fn is_binary_format(string: &openapiv3::StringType) -> bool {
    match &string.format {
        openapiv3::VariantOrUnknownOrEmpty::Item(openapiv3::StringFormat::Binary) => true,
        openapiv3::VariantOrUnknownOrEmpty::Unknown(format) => {
            format.eq_ignore_ascii_case("binary") || format.eq_ignore_ascii_case("base64")
        }
        _ => false,
    }
}

fn describe_form_property(
    property: &ReferenceOr<Box<Schema>>,
    resolver: &RefResolver,
) -> Option<String> {
    resolver
        .resolve_boxed_schema(property)
        .ok()
        .and_then(|schema| schema.schema_data.description.clone())
}

pub(super) fn reference_from_schema(
    schema: &ReferenceOr<Schema>,
    _resolver: &RefResolver,
    diagnostics: &mut Vec<OpenApiDiagnostic>,
) -> TypeRef {
    match schema {
        ReferenceOr::Reference { reference } => {
            if let Some(name) = reference.strip_prefix("#/components/schemas/") {
                TypeRef::Named(sanitize_rust_ident(name))
            } else {
                diagnostics.push(OpenApiDiagnostic::warn(
                    reference.clone(),
                    "Unexpected reference format".to_string(),
                ));
                JSON_VALUE
            }
        }
        ReferenceOr::Item(_) => JSON_VALUE,
    }
}

pub(super) fn select_content_type<'a>(
    content_types: &[&'a String],
    options: &OpenApiImportOptions,
) -> Option<&'a String> {
    match options.content_preference {
        ContentPreference::PreferJson => {
            for ct in content_types {
                if ct.contains("json") {
                    return Some(ct);
                }
            }
            for ct in content_types {
                if ct.contains("text") {
                    return Some(ct);
                }
            }
            for ct in content_types {
                if ct.contains("octet-stream") || ct.contains("binary") {
                    return Some(ct);
                }
            }
            content_types.first().copied()
        }
        ContentPreference::FirstAvailable => content_types.first().copied(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn select_content_type_prefers_json() {
        let xml_str = "application/xml".to_string();
        let json_str = "application/json".to_string();
        let text_str = "text/plain".to_string();
        let types = vec![&xml_str, &json_str, &text_str];
        let options = OpenApiImportOptions::default();

        let result = select_content_type(&types, &options);
        assert_eq!(result.map(|s| s.as_str()), Some("application/json"));
    }

    #[test]
    fn select_content_type_falls_back_to_text() {
        let json_str = "text/plain".to_string();
        let xml_str = "application/xml".to_string();
        let types = vec![&xml_str, &json_str];
        let options = OpenApiImportOptions::default();

        let result = select_content_type(&types, &options);
        assert_eq!(result.map(|s| s.as_str()), Some("text/plain"));
    }
}
