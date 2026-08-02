use openapiv3::{ReferenceOr, Schema};

use super::super::diagnostics::OpenApiDiagnostic;
use super::super::naming::sanitize_rust_ident;
use super::super::resolver::RefResolver;
use super::super::{ContentPreference, OpenApiImportOptions};
use crate::request::{ApiRequest, FormField};
use crate::response::ApiResponse;

/// Type path used when a body or response has no named schema to reference.
///
/// This is a path, not an identifier; code generation must render it with
/// `syn` parsing rather than `format_ident!`.
const JSON_VALUE_TYPE: &str = "serde_json::Value";

pub fn map_request_body(
    body: &Option<ReferenceOr<openapiv3::RequestBody>>,
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

        return map_media_type_to_request(ct, media_type, resolver, diagnostics);
    }

    None
}

fn map_media_type_to_request(
    content_type: &str,
    media_type: &openapiv3::MediaType,
    resolver: &RefResolver,
    diagnostics: &mut Vec<OpenApiDiagnostic>,
) -> Option<ApiRequest> {
    let ct_lower = content_type.to_lowercase();

    if ct_lower.contains("json") {
        if let Some(ref schema) = media_type.schema {
            let type_name = extract_type_name_from_schema(schema, resolver, diagnostics);
            return Some(ApiRequest::json_type(&type_name));
        }
    } else if ct_lower.contains("octet-stream") || ct_lower.contains("binary") {
        return Some(ApiRequest::binary(content_type));
    } else if ct_lower.contains("form-data") {
        return map_form_fields(media_type, resolver, diagnostics, FormFlavor::Multipart);
    } else if ct_lower.contains("x-www-form-urlencoded") {
        return map_form_fields(media_type, resolver, diagnostics, FormFlavor::UrlEncoded);
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

/// Derives form fields from a form media type's schema.
///
/// Returning `None` here would drop the body entirely and generate a request
/// struct with no way to send one, so an unusable schema is reported as a
/// warning rather than silently discarded.
fn map_form_fields(
    media_type: &openapiv3::MediaType,
    resolver: &RefResolver,
    diagnostics: &mut Vec<OpenApiDiagnostic>,
    flavor: FormFlavor,
) -> Option<ApiRequest> {
    let schema_ref = media_type.schema.as_ref()?;
    let schema = match resolver.resolve_schema(schema_ref) {
        Ok(schema) => schema,
        Err(e) => {
            diagnostics.push(OpenApiDiagnostic::warn(
                "request body".to_string(),
                format!("Failed to resolve form schema: {e}"),
            ));
            return None;
        }
    };

    let object = match &schema.schema_kind {
        openapiv3::SchemaKind::Type(openapiv3::Type::Object(object)) => object,
        _ => {
            diagnostics.push(OpenApiDiagnostic::warn(
                "request body".to_string(),
                "Form schema is not an object; body omitted".to_string(),
            ));
            return None;
        }
    };

    let fields: Vec<FormField> = object
        .properties
        .iter()
        .map(|(name, property)| {
            let required = object.required.contains(name);
            map_form_field(name, property, resolver, flavor, required)
        })
        .collect();

    if fields.is_empty() {
        diagnostics.push(OpenApiDiagnostic::warn(
            "request body".to_string(),
            "Form schema declares no properties; body omitted".to_string(),
        ));
        return None;
    }

    Some(match flavor {
        FormFlavor::Multipart => ApiRequest::form_data(fields),
        FormFlavor::UrlEncoded => ApiRequest::url_encoded(fields),
    })
}

/// Maps a single form property to a [`FormField`].
///
/// `string` + `format: binary` is the OpenAPI spelling of a file part; an array
/// of those is a repeated part sharing one name (RFC 7578). URL-encoded forms
/// have no file concept, so every property there stays a text field.
fn map_form_field(
    name: &str,
    property: &ReferenceOr<Box<Schema>>,
    resolver: &RefResolver,
    flavor: FormFlavor,
    required: bool,
) -> FormField {
    let field = if flavor == FormFlavor::UrlEncoded {
        FormField::text(name)
    } else {
        match classify_form_property(property, resolver) {
            FormPropertyShape::File => FormField::file(name),
            FormPropertyShape::Files => FormField::files(name),
            FormPropertyShape::Text => FormField::text(name),
        }
    };

    let field = if required { field } else { field.optional() };

    match describe_form_property(property, resolver) {
        Some(description) => field.with_description(description),
        None => field,
    }
}

/// The multipart shape a form property maps to.
enum FormPropertyShape {
    /// A single file part.
    File,
    /// Repeated file parts sharing one name.
    Files,
    /// A plain text part.
    Text,
}

fn classify_form_property(
    property: &ReferenceOr<Box<Schema>>,
    resolver: &RefResolver,
) -> FormPropertyShape {
    let Ok(schema) = resolver.resolve_boxed_schema(property) else {
        return FormPropertyShape::Text;
    };

    match &schema.schema_kind {
        openapiv3::SchemaKind::Type(openapiv3::Type::String(string)) => {
            if is_binary_format(string) {
                FormPropertyShape::File
            } else {
                FormPropertyShape::Text
            }
        }
        openapiv3::SchemaKind::Type(openapiv3::Type::Array(array)) => {
            let items_are_binary = array
                .items
                .as_ref()
                .and_then(|items| resolver.resolve_boxed_schema(items).ok())
                .is_some_and(|items| {
                    matches!(
                        &items.schema_kind,
                        openapiv3::SchemaKind::Type(openapiv3::Type::String(string))
                            if is_binary_format(string)
                    )
                });

            if items_are_binary {
                FormPropertyShape::Files
            } else {
                FormPropertyShape::Text
            }
        }
        _ => FormPropertyShape::Text,
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

pub fn map_responses(
    responses: &openapiv3::Responses,
    resolver: &RefResolver,
    diagnostics: &mut Vec<OpenApiDiagnostic>,
    _options: &OpenApiImportOptions,
) -> ApiResponse {
    let status_priority = ["200", "201", "204"];

    for status in &status_priority {
        if let Some(resp) = responses
            .responses
            .get(&openapiv3::StatusCode::Code(status.parse().unwrap()))
        {
            if *status == "204" {
                return ApiResponse::Empty;
            }
            if let Some(response) = map_response_ref(resp, resolver, diagnostics) {
                return response;
            }
        }
    }

    for (status, resp) in &responses.responses {
        if let openapiv3::StatusCode::Code(code) = status
            && (200..300).contains(&(*code as i32))
        {
            if *code == 204 {
                return ApiResponse::Empty;
            }
            if let Some(response) = map_response_ref(resp, resolver, diagnostics) {
                return response;
            }
        }
    }

    if let Some(ref default) = responses.default
        && let Some(response) = map_response_ref(default, resolver, diagnostics)
    {
        return response;
    }

    diagnostics.push(OpenApiDiagnostic::warn(
        "responses".to_string(),
        "No suitable response found, defaulting to Empty".to_string(),
    ));
    ApiResponse::Empty
}

fn map_response_ref(
    resp: &ReferenceOr<openapiv3::Response>,
    resolver: &RefResolver,
    diagnostics: &mut Vec<OpenApiDiagnostic>,
) -> Option<ApiResponse> {
    let response = match resp {
        ReferenceOr::Item(r) => r,
        ReferenceOr::Reference { reference } => match resolver.resolve_response(resp) {
            Ok(r) => r,
            Err(e) => {
                diagnostics.push(OpenApiDiagnostic::error(
                    reference.clone(),
                    format!("Failed to resolve response reference: {}", e),
                ));
                return None;
            }
        },
    };

    if response.content.is_empty() {
        return Some(ApiResponse::Empty);
    }

    // Selected by preference rather than by map order: an operation that offers
    // both `application/json` and `text/event-stream` must resolve to the JSON
    // shape regardless of which the spec happens to list first.
    let selected = response
        .content
        .iter()
        .min_by_key(|(ct, _)| classify_response_content_type(ct))?;

    let (content_type, media_type) = selected;

    match classify_response_content_type(content_type) {
        ResponseContentClass::Json => match media_type.schema {
            Some(ref schema) => {
                let type_name = extract_type_name_from_schema(schema, resolver, diagnostics);
                Some(ApiResponse::json_type(&type_name))
            }
            None => Some(ApiResponse::json_type(JSON_VALUE_TYPE)),
        },
        ResponseContentClass::Binary => Some(ApiResponse::Binary),
        ResponseContentClass::EventStream => {
            // Server-sent events are surfaced as raw bytes for the caller to frame,
            // matching how ollama's streaming endpoints are defined.
            diagnostics.push(OpenApiDiagnostic::info(
                content_type.clone(),
                "Server-sent event stream mapped to a binary response".to_string(),
            ));
            Some(ApiResponse::Binary)
        }
        ResponseContentClass::Text => Some(ApiResponse::Text),
        ResponseContentClass::Other => None,
    }
}

/// Response content-type classes, ordered by selection preference.
///
/// `Ord` is the selection rule: the lowest-ranked class present on an operation
/// wins, so adding a variant changes precedence.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum ResponseContentClass {
    Json,
    Binary,
    EventStream,
    Text,
    Other,
}

fn classify_response_content_type(content_type: &str) -> ResponseContentClass {
    let ct = content_type.to_lowercase();

    // Checked before the generic `text` arm, which `text/event-stream` also matches.
    if ct.contains("event-stream") {
        return ResponseContentClass::EventStream;
    }
    if ct.contains("json") {
        return ResponseContentClass::Json;
    }
    if ct.contains("octet-stream")
        || ct.contains("binary")
        || ct.starts_with("audio/")
        || ct.starts_with("image/")
        || ct.starts_with("video/")
        || ct.starts_with("application/pdf")
        || ct.contains("zip")
    {
        return ResponseContentClass::Binary;
    }
    if ct.contains("text") || ct.contains("xml") {
        return ResponseContentClass::Text;
    }

    ResponseContentClass::Other
}

fn extract_type_name_from_schema(
    schema: &ReferenceOr<Schema>,
    _resolver: &RefResolver,
    diagnostics: &mut Vec<OpenApiDiagnostic>,
) -> String {
    match schema {
        ReferenceOr::Reference { reference } => {
            if let Some(name) = reference.strip_prefix("#/components/schemas/") {
                sanitize_rust_ident(name)
            } else {
                diagnostics.push(OpenApiDiagnostic::warn(
                    reference.clone(),
                    "Unexpected reference format".to_string(),
                ));
                JSON_VALUE_TYPE.to_string()
            }
        }
        ReferenceOr::Item(_) => JSON_VALUE_TYPE.to_string(),
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
