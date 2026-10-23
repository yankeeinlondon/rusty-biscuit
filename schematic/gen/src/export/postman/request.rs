//! Request-level builders: URL, headers, body, and form parameters.

use schematic_define::{Endpoint, ParamDef, QueryParamType, RestApi, RestMethod};
use serde_json::Value;

use crate::export::auth::{ApiKeyLocation, ExportAuth};
use crate::export::body::{ExportBody, FormField, FormFieldExportKind};

use super::types::*;
use super::variables::{Collected, reference};

/// Builds the URL object for a request.
///
/// Path variables and query parameters are valued with collection variables
/// ([`Collected::parameter`]); an optional query parameter is `disabled`, so
/// Postman sends it only once the user enables it. A query parameter that
/// shares its name with a query API key is left to the auth.
pub(crate) fn build_url(
    path: &str,
    path_params: &[&str],
    endpoint: &Endpoint,
    base_url_var: &str,
    auth: &ExportAuth,
    collected: &mut Collected,
) -> PostmanUrl {
    // Build raw URL with the chosen base-URL variable and path params as :param
    let path_with_colon = path_params.iter().fold(path.to_string(), |acc, param| {
        acc.replace(&format!("{{{}}}", param), &format!(":{}", param))
    });
    let raw = format!("{}{}", reference(base_url_var), path_with_colon);

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

    let declared = endpoint.params.as_ref().map_or(&[][..], |params| params.path.as_slice());
    let variables: Vec<PostmanVariable> = path_params
        .iter()
        .map(|param| {
            let typed = declared.iter().find(|declared| declared.name == *param);
            let key = collected.parameter("path", param, typed.is_some_and(|typed| typed.secret));
            PostmanVariable {
                key: param.to_string(),
                value: Some(reference(&key)),
                description: typed.map(path_variable_description),
            }
        })
        .collect();

    let query_key = match auth {
        ExportAuth::ApiKey { header, location: ApiKeyLocation::Query, .. } => Some(header.as_str()),
        _ => None,
    };
    let query_params = endpoint
        .params
        .iter()
        .flat_map(|params| &params.query)
        .filter(|param| Some(param.name.as_str()) != query_key)
        .map(|param| PostmanQuery {
            key: param.name.clone(),
            value: Some(reference(&collected.parameter("query", &param.name, param.secret))),
            disabled: !param.required,
            description: param.description.clone(),
        })
        .collect();

    PostmanUrl {
        raw,
        host: vec![reference(base_url_var)],
        path: path_segments,
        query: query_params,
        variable: variables,
    }
}

/// The description of a declared path parameter's URL variable: its own
/// description, if any, followed by its type, since a Postman variable has no
/// type of its own (`Item id (integer)`, `one of: cat, dog`).
fn path_variable_description(param: &ParamDef) -> String {
    let type_text = match &param.param_type {
        QueryParamType::Integer => "integer".to_string(),
        QueryParamType::Number => "number".to_string(),
        QueryParamType::Boolean => "boolean".to_string(),
        QueryParamType::Enum(values) => format!("one of: {}", values.join(", ")),
        _ => "string".to_string(),
    };
    match &param.description {
        Some(description) if !description.is_empty() => format!("{description} ({type_text})"),
        _ => type_text,
    }
}

/// Builds headers for a request, in this order:
///
/// 1. The API's fixed headers, each replaced by an endpoint header of the
///    same name (case-insensitive).
/// 2. The body's `Content-Type` for a text or binary body, unless a fixed
///    header already sets it.
/// 3. Declared header parameters, valued with collection variables. A fixed
///    header of the same name wins, and a parameter naming the credential
///    header of `auth` (or `Cookie`) is left out.
/// 4. Declared cookie parameters merged into one `Cookie` header of
///    `name={{cookie_name}}` pairs joined by `; `, the runtime's cookie
///    syntax. A cookie the API-key auth sends is left out, and a fixed
///    `Cookie` header wins over all of them.
///
/// An optional parameter is `disabled`, so Postman sends it only once the
/// user enables it. A `Cookie` header holds the required cookies and names
/// the optional ones in its description; when every cookie is optional, the
/// header holds them all and is disabled.
pub(crate) fn build_headers(
    endpoint: &Endpoint,
    api: &RestApi,
    auth: &ExportAuth,
    body: Option<&ExportBody>,
    collected: &mut Collected,
) -> Vec<PostmanHeader> {
    let fixed = |key: &String, value: &String| PostmanHeader {
        key: key.clone(),
        value: value.clone(),
        disabled: false,
        description: None,
    };
    let mut headers: Vec<PostmanHeader> = api.headers.iter().map(|(key, value)| fixed(key, value)).collect();
    for (key, value) in &endpoint.headers {
        headers.retain(|h| !h.key.eq_ignore_ascii_case(key));
        headers.push(fixed(key, value));
    }
    let is_fixed = |headers: &[PostmanHeader], name: &str| headers.iter().any(|h| h.key.eq_ignore_ascii_case(name));

    let content_type = match body {
        Some(ExportBody::Text { content_type } | ExportBody::Binary { content_type }) => Some(content_type),
        _ => None,
    };
    if let Some(content_type) = content_type
        && !is_fixed(&headers, "Content-Type")
    {
        headers.push(fixed(&"Content-Type".to_string(), content_type));
    }

    let Some(params) = &endpoint.params else {
        return headers;
    };
    let credential_header = match auth {
        ExportAuth::Bearer { .. } | ExportAuth::Basic { .. } | ExportAuth::OAuth2 { .. } => Some("Authorization"),
        ExportAuth::ApiKey { header, location: ApiKeyLocation::Header, .. } => Some(header.as_str()),
        _ => None,
    };
    let credential_cookie = match auth {
        ExportAuth::ApiKey { header, location: ApiKeyLocation::Cookie, .. } => Some(header.as_str()),
        _ => None,
    };

    let mut declared = Vec::new();
    for param in &params.header {
        let excluded = is_fixed(&headers, &param.name)
            || param.name.eq_ignore_ascii_case("Cookie")
            || credential_header.is_some_and(|credential| credential.eq_ignore_ascii_case(&param.name));
        if excluded {
            continue;
        }
        declared.push(PostmanHeader {
            key: param.name.clone(),
            value: reference(&collected.parameter("header", &param.name, param.secret)),
            disabled: !param.required,
            description: param.description.clone(),
        });
    }

    let cookies: Vec<&ParamDef> = params.cookie.iter().filter(|param| Some(param.name.as_str()) != credential_cookie).collect();
    if !cookies.is_empty() && !is_fixed(&headers, "Cookie") {
        let required: Vec<&ParamDef> = cookies.iter().copied().filter(|param| param.required).collect();
        let (sent, disabled, description) = if required.is_empty() {
            (cookies.clone(), true, None)
        } else {
            let optional: Vec<&str> = cookies.iter().filter(|param| !param.required).map(|param| param.name.as_str()).collect();
            let description = (!optional.is_empty()).then(|| format!("Optional cookies, sent only when added: {}.", optional.join(", ")));
            (required, false, description)
        };
        let pairs: Vec<String> = sent
            .iter()
            .map(|param| format!("{}={}", param.name, reference(&collected.parameter("cookie", &param.name, param.secret))))
            .collect();
        declared.push(PostmanHeader { key: "Cookie".to_string(), value: pairs.join("; "), disabled, description });
    }

    headers.extend(declared);
    headers
}

/// Builds the body object for a request. A JSON body holds `example`
/// pretty-printed, or nothing when no valid example exists.
pub(crate) fn build_body(export_body: &ExportBody, example: Option<&Value>) -> PostmanBody {
    let raw = |text: String, language: &str| PostmanBody {
        mode: "raw".to_string(),
        raw: Some(text),
        formdata: None,
        urlencoded: None,
        options: Some(PostmanBodyOptions {
            raw: PostmanRawOptions {
                language: language.to_string(),
            },
        }),
    };
    match export_body {
        ExportBody::Json { .. } => raw(
            example.map(|value| serde_json::to_string_pretty(value).unwrap_or_default()).unwrap_or_default(),
            "json",
        ),
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
        ExportBody::Text { .. } => raw(String::new(), "text"),
        ExportBody::Binary { .. } => PostmanBody {
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
