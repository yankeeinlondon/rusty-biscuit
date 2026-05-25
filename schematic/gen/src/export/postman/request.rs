//! Request-level builders: URL, headers, body, and form parameters.

use schematic_define::{Endpoint, RestApi, RestMethod};

use crate::export::body::{ExportBody, FormField, FormFieldExportKind};

use super::types::*;

/// Builds the URL object for a request.
pub(crate) fn build_url(
    _base_url: &str,
    path: &str,
    path_params: &[&str],
    endpoint: &Endpoint,
    base_url_var: &str,
) -> PostmanUrl {
    // Build raw URL with the chosen base-URL variable and path params as :param
    let path_with_colon = path_params.iter().fold(path.to_string(), |acc, param| {
        acc.replace(&format!("{{{}}}", param), &format!(":{}", param))
    });
    let raw = format!("{{{{{}}}}}{}", base_url_var, path_with_colon);

    // Split path into segments, converting {param} to :param
    let path_segments: Vec<String> = path
        .trim_start_matches('/')
        .split('/')
        .filter(|s| !s.is_empty())
        .map(|segment| {
            if segment.starts_with('{') && segment.ends_with('}') {
                format!(":{}", segment.trim_start_matches('{').trim_end_matches('}'))
            } else {
                segment.to_string()
            }
        })
        .collect();

    // Build path variables
    let variables: Vec<PostmanVariable> = path_params
        .iter()
        .map(|param| PostmanVariable {
            key: param.to_string(),
            value: Some(format!("<{}>", param)),
            description: None,
        })
        .collect();

    // Build query parameters
    let query_params = if let Some(ref params) = endpoint.params {
        params
            .query
            .iter()
            .map(|param| PostmanQuery {
                key: param.name.clone(),
                value: None, // Postman can populate this
                description: param.description.clone(),
            })
            .collect()
    } else {
        vec![]
    };

    PostmanUrl {
        raw,
        host: vec![format!("{{{{{}}}}}", base_url_var)],
        path: path_segments,
        query: query_params,
        variable: variables,
    }
}

/// Builds headers for a request.
pub(crate) fn build_headers(endpoint: &Endpoint, api: &RestApi) -> Vec<PostmanHeader> {
    let mut headers = Vec::new();

    // Add API-level headers
    for (key, value) in &api.headers {
        headers.push(PostmanHeader {
            key: key.clone(),
            value: value.clone(),
        });
    }

    // Add endpoint-level headers (override API-level)
    for (key, value) in &endpoint.headers {
        // Remove any API-level header with same key (case-insensitive)
        headers.retain(|h| !h.key.eq_ignore_ascii_case(key));
        headers.push(PostmanHeader {
            key: key.clone(),
            value: value.clone(),
        });
    }

    headers
}

/// Builds the body object for a request.
pub(crate) fn build_body(export_body: &ExportBody) -> PostmanBody {
    match export_body {
        ExportBody::Json { .. } => PostmanBody {
            mode: "raw".to_string(),
            raw: Some("{}".to_string()),
            formdata: None,
            urlencoded: None,
            options: Some(PostmanBodyOptions {
                raw: PostmanRawOptions {
                    language: "json".to_string(),
                },
            }),
        },
        ExportBody::FormData { fields } => PostmanBody {
            mode: "formdata".to_string(),
            raw: None,
            formdata: Some(fields.iter().map(build_form_param).collect()),
            urlencoded: None,
            options: None,
        },
        ExportBody::UrlEncoded { fields } => PostmanBody {
            mode: "urlencoded".to_string(),
            raw: None,
            formdata: None,
            urlencoded: Some(fields.iter().map(build_form_param).collect()),
            options: None,
        },
        ExportBody::Text { .. } => PostmanBody {
            mode: "raw".to_string(),
            raw: Some(String::new()),
            formdata: None,
            urlencoded: None,
            options: Some(PostmanBodyOptions {
                raw: PostmanRawOptions {
                    language: "text".to_string(),
                },
            }),
        },
        ExportBody::Binary => PostmanBody {
            mode: "file".to_string(),
            raw: None,
            formdata: None,
            urlencoded: None,
            options: None,
        },
    }
}

/// Builds a Postman form parameter, branching on field kind.
///
/// File and Files fields emit `type: "file"` with no `value` — Postman
/// expects users to attach a file via its `src` array, which we leave
/// unset so importers prompt for a path. Text and JSON-part fields emit
/// `type: "text"` with an empty default value (Postman has no native
/// JSON-part concept; preserving the discriminator on
/// [`FormFieldExportKind::Json`] lets future exporters opt into a richer
/// representation without re-deriving kind from the field name).
pub(crate) fn build_form_param(field: &FormField) -> PostmanFormParam {
    match &field.kind {
        FormFieldExportKind::File { .. } | FormFieldExportKind::Files { .. } => PostmanFormParam {
            key: field.name.clone(),
            value: None,
            description: field.description.clone(),
            type_field: "file".to_string(),
        },
        FormFieldExportKind::Text | FormFieldExportKind::Json => PostmanFormParam {
            key: field.name.clone(),
            value: Some(String::new()),
            description: field.description.clone(),
            type_field: "text".to_string(),
        },
    }
}

/// Converts RestMethod to uppercase string.
pub(crate) fn rest_method_to_string(method: &RestMethod) -> String {
    method.to_string()
}

/// Capitalizes the first letter of a folder name.
pub(crate) fn capitalize_folder_name(name: &str) -> String {
    let mut chars = name.chars();
    match chars.next() {
        None => String::new(),
        Some(first) => first.to_uppercase().chain(chars).collect(),
    }
}
