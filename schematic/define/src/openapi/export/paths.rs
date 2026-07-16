use indexmap::IndexMap;
use openapiv3::{Operation, PathItem, Paths, ReferenceOr};

use super::super::OpenApiError;
use super::super::extensions::SchematicOpExtension;
use super::super::options::ExportOptions;
use super::components::SchemaRegistryLike;
use super::request_body::map_request_body;
use super::responses::map_responses;
use crate::types::{Endpoint, RestMethod};

/// Maps all endpoints to OpenAPI paths.
pub(super) fn map_paths<R: SchemaRegistryLike>(
    api: &crate::types::RestApi,
    registry: &R,
    options: &ExportOptions,
) -> Result<Paths, OpenApiError> {
    let mut paths = IndexMap::new();

    for endpoint in &api.endpoints {
        let operation = map_operation(endpoint, registry, options)?;
        let path_params = extract_path_params(&endpoint.path);

        // A reserved-expansion marker (`{+name}`) is a runtime encoding hint;
        // the exported OpenAPI path key must render as valid `{name}`.
        let path_key = strip_reserved_markers(&endpoint.path);
        let path_item = paths.entry(path_key).or_insert_with(PathItem::default);

        match endpoint.method {
            RestMethod::Get => path_item.get = Some(operation),
            RestMethod::Post => path_item.post = Some(operation),
            RestMethod::Put => path_item.put = Some(operation),
            RestMethod::Patch => path_item.patch = Some(operation),
            RestMethod::Delete => path_item.delete = Some(operation),
            RestMethod::Head => path_item.head = Some(operation),
            RestMethod::Options => path_item.options = Some(operation),
        }

        for param in path_params {
            path_item.parameters.push(ReferenceOr::Item(param));
        }
    }

    Ok(Paths {
        paths: paths
            .into_iter()
            .map(|(k, v)| (k, ReferenceOr::Item(v)))
            .collect(),
        ..Default::default()
    })
}

/// Maps a single endpoint to an OpenAPI operation.
pub(super) fn map_operation<R: SchemaRegistryLike>(
    endpoint: &Endpoint,
    _registry: &R,
    options: &ExportOptions,
) -> Result<Operation, OpenApiError> {
    let request_body = endpoint
        .request
        .as_ref()
        .map(|req| ReferenceOr::Item(map_request_body(req)));

    let responses = map_responses(&endpoint.response);

    let mut extensions = IndexMap::new();
    if !options.skip_extensions {
        let op_ext = SchematicOpExtension {
            request: endpoint.request.clone(),
            response: endpoint.response.clone(),
            headers: endpoint.headers.clone(),
            oauth_scopes: endpoint.oauth_scopes.clone(),
        };
        extensions.insert("x-schematic".to_string(), op_ext.into());
    }

    Ok(Operation {
        operation_id: Some(endpoint.id.clone()),
        summary: Some(endpoint.description.clone()),
        description: Some(endpoint.description.clone()),
        request_body,
        responses,
        parameters: vec![],
        extensions,
        ..Default::default()
    })
}

/// Rewrites RFC 6570 reserved-expansion markers (`{+name}`) to plain `{name}`.
///
/// The `+` is a runtime encoding hint; exported OpenAPI paths and parameter
/// names must never carry it.
fn strip_reserved_markers(path: &str) -> String {
    let mut result = String::with_capacity(path.len());
    let mut chars = path.chars().peekable();
    while let Some(c) = chars.next() {
        result.push(c);
        if c == '{' && chars.peek() == Some(&'+') {
            chars.next();
        }
    }
    result
}

/// Extracts path parameters from a path template.
///
/// Parses `{param}` (and reserved-expansion `{+param}`) segments from the path
/// and returns OpenAPI parameter definitions. A leading `+` is stripped so the
/// exported parameter name is always the bare spelling.
pub fn extract_path_params(path: &str) -> Vec<openapiv3::Parameter> {
    let mut params = Vec::new();

    let mut chars = path.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '{' {
            let mut param_name = String::new();
            while let Some(&next) = chars.peek() {
                if next == '}' {
                    chars.next();
                    break;
                }
                param_name.push(chars.next().unwrap());
            }

            let param_name = param_name
                .strip_prefix('+')
                .map(str::to_string)
                .unwrap_or(param_name);

            if !param_name.is_empty() {
                params.push(openapiv3::Parameter::Path {
                    parameter_data: openapiv3::ParameterData {
                        name: param_name,
                        description: None,
                        required: true,
                        format: openapiv3::ParameterSchemaOrContent::Schema(ReferenceOr::Item(
                            openapiv3::Schema {
                                schema_data: Default::default(),
                                schema_kind: openapiv3::SchemaKind::Type(openapiv3::Type::String(
                                    openapiv3::StringType::default(),
                                )),
                            },
                        )),
                        deprecated: None,
                        example: None,
                        examples: IndexMap::new(),
                        explode: None,
                        extensions: IndexMap::new(),
                    },
                    style: openapiv3::PathStyle::Simple,
                });
            }
        }
    }

    params
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extract_path_params_no_params() {
        let params = extract_path_params("/models");
        assert!(params.is_empty());
    }

    #[test]
    fn extract_path_params_single_param() {
        let params = extract_path_params("/models/{model}");
        assert_eq!(params.len(), 1);

        if let openapiv3::Parameter::Path { parameter_data, .. } = &params[0] {
            assert_eq!(parameter_data.name, "model");
            assert!(parameter_data.required);
        }
    }

    #[test]
    fn extract_path_params_multiple_params() {
        let params = extract_path_params("/users/{user_id}/posts/{post_id}");
        assert_eq!(params.len(), 2);

        let names: Vec<_> = params
            .iter()
            .map(|p| {
                if let openapiv3::Parameter::Path { parameter_data, .. } = p {
                    parameter_data.name.clone()
                } else {
                    String::new()
                }
            })
            .collect();

        assert!(names.contains(&"user_id".to_string()));
        assert!(names.contains(&"post_id".to_string()));
    }

    #[test]
    fn extract_path_params_strips_reserved_marker() {
        let params = extract_path_params("/models/{+repo_id}");
        assert_eq!(params.len(), 1);

        if let openapiv3::Parameter::Path { parameter_data, .. } = &params[0] {
            assert_eq!(parameter_data.name, "repo_id");
        } else {
            panic!("expected a path parameter");
        }
    }

    #[test]
    fn strip_reserved_markers_rewrites_plus_form() {
        assert_eq!(
            strip_reserved_markers("/repos/{owner}/{repo}/contents/{+path}"),
            "/repos/{owner}/{repo}/contents/{path}"
        );
        assert_eq!(strip_reserved_markers("/models/{model}"), "/models/{model}");
    }

    #[test]
    fn extract_path_params_typed_as_string() {
        let params = extract_path_params("/items/{item_id}");

        if let openapiv3::Parameter::Path { parameter_data, .. } = &params[0]
            && let openapiv3::ParameterSchemaOrContent::Schema(ReferenceOr::Item(schema)) =
                &parameter_data.format
        {
            match &schema.schema_kind {
                openapiv3::SchemaKind::Type(openapiv3::Type::String(_)) => {}
                _ => panic!("Expected string type"),
            }
        }
    }
}
