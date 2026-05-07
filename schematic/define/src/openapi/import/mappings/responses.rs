use openapiv3::{ReferenceOr, Schema};

use super::super::diagnostics::OpenApiDiagnostic;
use super::super::naming::sanitize_rust_ident;
use super::super::resolver::RefResolver;
use super::super::{ContentPreference, OpenApiImportOptions};
use crate::request::ApiRequest;
use crate::response::ApiResponse;

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
    } else if ct_lower.contains("text") {
        return Some(ApiRequest::text(content_type));
    } else if ct_lower.contains("form-data") {
        diagnostics.push(OpenApiDiagnostic::info(
            "request body".to_string(),
            "Multipart form-data requires manual field mapping".to_string(),
        ));
        return None;
    } else if ct_lower.contains("x-www-form-urlencoded") {
        diagnostics.push(OpenApiDiagnostic::info(
            "request body".to_string(),
            "URL-encoded form requires manual field mapping".to_string(),
        ));
        return None;
    }

    None
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

    for (ct, media_type) in &response.content {
        let ct_lower = ct.to_lowercase();

        if ct_lower.contains("json") {
            if let Some(ref schema) = media_type.schema {
                let type_name = extract_type_name_from_schema(schema, resolver, diagnostics);
                return Some(ApiResponse::json_type(&type_name));
            }
            return Some(ApiResponse::json_type("serde_json::Value"));
        } else if ct_lower.contains("octet-stream")
            || ct_lower.contains("binary")
            || ct_lower.contains("audio")
            || ct_lower.contains("image")
        {
            return Some(ApiResponse::Binary);
        } else if ct_lower.contains("text") {
            return Some(ApiResponse::Text);
        }
    }

    if response.content.is_empty() {
        return Some(ApiResponse::Empty);
    }

    None
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
                "serde_json::Value".to_string()
            }
        }
        ReferenceOr::Item(_) => "serde_json::Value".to_string(),
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
