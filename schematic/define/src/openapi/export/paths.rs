use indexmap::IndexMap;
use openapiv3::{Operation, Parameter, PathItem, Paths, ReferenceOr, SecurityRequirement, SecurityScheme};

use super::super::OpenApiError;
use super::super::extensions::{AuthDetail, ParamDetail, SchematicOpExtension};
use super::super::options::ExportOptions;
use super::headers::{HeaderGap, apply_fixed_headers, credential_headers};
use super::conversion::pointer_token;
use super::parameters::{constrained_param_schema, effective_constraints, map_parameters};
use super::request_body::map_constrained_request_body;
use super::responses::map_responses;
use super::security::endpoint_security;
use crate::auth::EndpointAuth;
use crate::pagination::{AdoptedPageSize, ParamLocation, adopted_page_size};
use crate::params::{EndpointParams, ParamDef};
use crate::request::ApiRequest;
use crate::response::ResponseShape;
use crate::types::{Endpoint, RestMethod};

/// Maps all endpoints to OpenAPI paths.
///
/// Path parameters live on the path item. When the operations of a path
/// declare one `(name, in: path)` differently, the path item holds the
/// declaration of the operation first in [`METHOD_ORDER`] and every operation
/// that differs carries its own declaration as an operation-level override,
/// so the result never depends on endpoint order.
///
/// Endpoint security schemes are registered in `security_schemes`. Fixed
/// headers become operation-level header parameters; the ones that cannot
/// are appended to `header_gaps`.
///
/// Every parameter carries its value rules; the page-size parameter an
/// endpoint's pagination strategy adopts carries the strategy's effective
/// bounds ([`adopted_page_size`]).
pub(super) fn map_paths(
    api: &crate::types::RestApi,
    options: &ExportOptions,
    document_security: &[SecurityRequirement],
    security_schemes: &mut IndexMap<String, ReferenceOr<SecurityScheme>>,
    header_gaps: &mut Vec<HeaderGap>,
) -> Result<Paths, OpenApiError> {
    let mut paths: IndexMap<String, PathItem> = IndexMap::new();
    let mut declarations: IndexMap<String, Vec<(RestMethod, Vec<Parameter>)>> = IndexMap::new();

    for endpoint in &api.endpoints {
        let (security, auth_details) = match endpoint_security(endpoint, security_schemes) {
            Some((requirements, details)) => (Some(requirements), details),
            None => (None, IndexMap::new()),
        };
        let adopted = adopted_page_size(api, endpoint);
        let mut operation = map_operation(api, endpoint, options, auth_details)?;
        operation.security = security;
        let credentials = credential_headers(
            operation.security.as_deref().unwrap_or(document_security),
            security_schemes,
        );
        header_gaps.extend(apply_fixed_headers(api, endpoint, &mut operation, &credentials));

        // A reserved-expansion marker (`{+name}`) is a runtime encoding hint;
        // the exported OpenAPI path key must render as valid `{name}`.
        let path_key = strip_reserved_markers(&endpoint.path);
        let path_parameters = endpoint_path_parameters(endpoint, adopted.as_ref(), &format!("#/paths/{}", pointer_token(&path_key)))?;
        let path_item = paths.entry(path_key.clone()).or_default();
        *operation_slot(path_item, endpoint.method) = Some(operation);
        declarations
            .entry(path_key)
            .or_default()
            .push((endpoint.method, path_parameters));
    }

    for (path_key, operations) in declarations {
        if let Some(path_item) = paths.get_mut(&path_key) {
            share_path_parameters(path_item, &operations);
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

/// OpenAPI path-item method order, which picks the common declaration of a
/// path parameter the operations disagree on.
const METHOD_ORDER: [RestMethod; 7] = [
    RestMethod::Get,
    RestMethod::Put,
    RestMethod::Post,
    RestMethod::Delete,
    RestMethod::Options,
    RestMethod::Head,
    RestMethod::Patch,
];

fn method_rank(method: RestMethod) -> usize {
    METHOD_ORDER.iter().position(|candidate| *candidate == method).unwrap_or(METHOD_ORDER.len())
}

fn operation_slot(item: &mut PathItem, method: RestMethod) -> &mut Option<Operation> {
    match method {
        RestMethod::Get => &mut item.get,
        RestMethod::Post => &mut item.post,
        RestMethod::Put => &mut item.put,
        RestMethod::Patch => &mut item.patch,
        RestMethod::Delete => &mut item.delete,
        RestMethod::Head => &mut item.head,
        RestMethod::Options => &mut item.options,
    }
}

/// One endpoint's path parameters, one per name in first-occurrence order.
///
/// Declared (typed) parameters follow the template's placeholder order; an
/// endpoint that declares none gets today's untyped string parameter per
/// placeholder. `adopted` is the endpoint's adopted page-size parameter and
/// `pointer` the path item's JSON pointer, which locates a schema that fails
/// to convert.
///
/// ## Errors
///
/// [`OpenApiError`] when a parameter's rules do not convert to OpenAPI 3.0.3.
fn endpoint_path_parameters(endpoint: &Endpoint, adopted: Option<&AdoptedPageSize>, pointer: &str) -> Result<Vec<Parameter>, OpenApiError> {
    let declared = endpoint.params.as_ref().map_or(&[][..], |params| params.path.as_slice());
    if declared.is_empty() {
        let mut params: Vec<Parameter> = Vec::new();
        for param in extract_path_params(&endpoint.path) {
            if !params.iter().any(|existing| existing.parameter_data_ref().name == param.parameter_data_ref().name) {
                params.push(param);
            }
        }
        return Ok(params);
    }
    endpoint
        .path_placeholders()
        .into_iter()
        .filter_map(|name| declared.iter().find(|param| param.name == name))
        .enumerate()
        .map(|(index, param)| {
            typed_path_parameter(param, effective_constraints(param, ParamLocation::Path, adopted), &format!("{pointer}/parameters/{index}/schema"))
        })
        .collect()
}

/// A declared path parameter: its type's schema with `rules`, located at
/// `pointer`, its description, and deprecation; always required, `simple`
/// style with the default explode.
fn typed_path_parameter(param: &ParamDef, rules: Option<&crate::constraints::Constraints>, pointer: &str) -> Result<Parameter, OpenApiError> {
    Ok(Parameter::Path {
        parameter_data: openapiv3::ParameterData {
            name: param.name.clone(),
            description: param.description.clone(),
            required: true,
            deprecated: param.deprecated.then_some(true),
            format: openapiv3::ParameterSchemaOrContent::Schema(ReferenceOr::Item(constrained_param_schema(
                &param.param_type,
                rules,
                pointer,
            )?)),
            example: None,
            examples: IndexMap::new(),
            explode: None,
            extensions: IndexMap::new(),
        },
        style: openapiv3::PathStyle::Simple,
    })
}

/// Declares each path parameter of `operations` once on `item`, with an
/// operation-level override wherever an operation's declaration differs from
/// the common one.
fn share_path_parameters(item: &mut PathItem, operations: &[(RestMethod, Vec<Parameter>)]) {
    let mut names: Vec<&str> = Vec::new();
    for (_, params) in operations {
        for param in params {
            let name = param.parameter_data_ref().name.as_str();
            if !names.contains(&name) {
                names.push(name);
            }
        }
    }

    // Overrides lead each operation's list, in path-parameter order.
    let mut inserted: IndexMap<RestMethod, usize> = IndexMap::new();
    for name in names {
        let declared: Vec<(RestMethod, &Parameter)> = operations
            .iter()
            .filter_map(|(method, params)| {
                params
                    .iter()
                    .find(|param| param.parameter_data_ref().name == name)
                    .map(|param| (*method, param))
            })
            .collect();
        let Some(&(_, common)) = declared.iter().min_by_key(|(method, _)| method_rank(*method)) else {
            continue;
        };
        item.parameters.push(ReferenceOr::Item(common.clone()));
        for (method, param) in declared {
            if param == common {
                continue;
            }
            if let Some(operation) = operation_slot(item, method).as_mut() {
                let index = inserted.entry(method).or_insert(0);
                operation.parameters.insert(*index, ReferenceOr::Item(param.clone()));
                *index += 1;
            }
        }
    }
}

/// Maps a single endpoint to an OpenAPI operation.
///
/// Path parameters and `security` are added by [`map_paths`];
/// `auth_details` are those of the schemes the operation's own `security`
/// names. Parameters and the request body carry their value rules, the
/// adopted page-size parameter its effective bounds.
pub(super) fn map_operation(
    api: &crate::types::RestApi,
    endpoint: &Endpoint,
    options: &ExportOptions,
    auth_details: IndexMap<String, AuthDetail>,
) -> Result<Operation, OpenApiError> {
    let pointer = format!("#/paths/{}/{}", pointer_token(&strip_reserved_markers(&endpoint.path)), endpoint.method.to_string().to_lowercase());
    let request_body = endpoint
        .request
        .as_ref()
        .map(|req| {
            map_constrained_request_body(req, endpoint.request_constraints.as_deref(), &format!("{pointer}/requestBody"))
                .map(ReferenceOr::Item)
        })
        .transpose()?;
    let parameters = map_parameters(endpoint.params.as_ref(), adopted_page_size(api, endpoint).as_ref(), &pointer)?;

    let shapes = endpoint.success_shapes();
    let responses = map_responses(shapes, &api.effective_errors(endpoint))?;

    let mut extensions = IndexMap::new();
    if !options.skip_extensions {
        let op_ext = SchematicOpExtension {
            // `requestBody` alone carries a JSON, text, or binary body, and
            // a form with no fields has no detail to add.
            request: endpoint.request.clone().filter(|request| {
                matches!(request, ApiRequest::FormData { fields } | ApiRequest::UrlEncoded { fields } if !fields.is_empty())
            }),
            response: shapes.first().map(|first| first.shape.clone()).unwrap_or(ResponseShape::Empty),
            headers: endpoint.headers.clone(),
            rate_limit: endpoint.rate_limit.clone(),
            retry_guidance: endpoint.retry_guidance.clone(),
            // `response` already says everything about a lone plain shape.
            success: match shapes {
                [only] if only.plain().is_some() => vec![],
                _ => shapes.to_vec(),
            },
            errors: endpoint.errors.clone(),
            no_declared_errors: endpoint.no_declared_errors,
            cross_origin: endpoint.cross_origin.clone(),
            auth: auth_details,
            env_mapping: match &endpoint.auth {
                Some(EndpointAuth::Policy { env_mapping, .. }) => env_mapping.clone(),
                _ => None,
            },
            path_template: endpoint.path.contains("{+").then(|| endpoint.path.clone()),
            params: param_details(endpoint.params.as_ref()),
        };
        extensions.insert("x-schematic".to_string(), op_ext.into());
    }

    Ok(Operation {
        operation_id: Some(endpoint.id.clone()),
        summary: Some(endpoint.description.clone()),
        description: Some(endpoint.description.clone()),
        request_body,
        responses,
        parameters,
        deprecated: endpoint.deprecated,
        extensions,
        ..Default::default()
    })
}

/// The secret markers and Rust names of `params`, by location and wire name.
fn param_details(params: Option<&EndpointParams>) -> IndexMap<String, IndexMap<String, ParamDetail>> {
    let mut details = IndexMap::new();
    let Some(params) = params else { return details };
    for (location, defs) in [("query", &params.query), ("header", &params.header), ("cookie", &params.cookie), ("path", &params.path)] {
        let entries: IndexMap<String, ParamDetail> = defs
            .iter()
            .filter(|param| param.secret || param.rust_name.is_some())
            .map(|param| (param.name.clone(), ParamDetail { secret: param.secret, rust_name: param.rust_name.clone() }))
            .collect();
        if !entries.is_empty() {
            details.insert(location.to_string(), entries);
        }
    }
    details
}

/// Rewrites RFC 6570 reserved-expansion markers (`{+name}`) to plain `{name}`.
///
/// The `+` is a runtime encoding hint; exported OpenAPI paths and parameter
/// names must never carry it.
pub(crate) fn strip_reserved_markers(path: &str) -> String {
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

    use crate::{DefinitionContext, QueryParamType, RestApi};
    use serde_json::{Value, json};

    fn export_json(endpoints: Vec<Endpoint>) -> Value {
        let ctx = DefinitionContext::types_in("::models");
        let api = RestApi::builder(&ctx, "Paths", "https://example.com")
            .describes("Paths")
            .no_auth()
            .endpoints(endpoints)
            .build()
            .unwrap();
        serde_json::to_value(super::super::export(&api, &ExportOptions::new().skip_extensions()).unwrap()).unwrap()
    }

    fn item_id(description: &str) -> ParamDef {
        ParamDef::path("item_id", QueryParamType::Integer).describes(description)
    }

    #[test]
    fn agreeing_typed_declarations_are_declared_once_on_the_path_item() {
        let doc = export_json(vec![
            Endpoint::get("Get", "/items/{item_id}").describes("Get").returns_empty().path_param(item_id("Item")).build().unwrap(),
            Endpoint::delete("Delete", "/items/{item_id}").describes("Delete").returns_empty().path_param(item_id("Item")).build().unwrap(),
        ]);
        let item = &doc["paths"]["/items/{item_id}"];
        assert_eq!(item["parameters"], json!([{
            "in": "path", "name": "item_id", "description": "Item", "required": true,
            "schema": {"type": "integer"}, "style": "simple",
        }]));
        assert!(item["get"].get("parameters").is_none());
        assert!(item["delete"].get("parameters").is_none());
    }

    #[test]
    fn differing_declarations_take_the_get_declaration_and_override_the_rest_in_any_visit_order() {
        let get = Endpoint::get("Get", "/items/{item_id}").describes("Get").returns_empty()
            .path_param(item_id("Item to read")).build().unwrap();
        let delete = Endpoint::delete("Delete", "/items/{item_id}").describes("Delete").returns_empty()
            .path_param(item_id("Item to delete").deprecated()).build().unwrap();

        let delete_first = export_json(vec![delete.clone(), get.clone()]);
        let item = &delete_first["paths"]["/items/{item_id}"];
        assert_eq!(item["parameters"][0]["description"], "Item to read");
        assert_eq!(item["parameters"].as_array().unwrap().len(), 1);
        assert!(item["get"].get("parameters").is_none());
        assert_eq!(item["delete"]["parameters"], json!([{
            "in": "path", "name": "item_id", "description": "Item to delete", "required": true,
            "deprecated": true, "schema": {"type": "integer"}, "style": "simple",
        }]));

        let get_first = export_json(vec![get, delete]);
        assert_eq!(get_first["paths"]["/items/{item_id}"]["parameters"], item["parameters"]);
        assert_eq!(get_first["paths"]["/items/{item_id}"]["delete"], item["delete"]);
    }

    #[test]
    fn put_outranks_post_when_no_get_declares_the_parameter() {
        let post = Endpoint::post("Post", "/items/{item_id}").describes("Post").returns_empty()
            .path_param(item_id("posted")).build().unwrap();
        let put = Endpoint::put("Put", "/items/{item_id}").describes("Put").returns_empty()
            .path_param(item_id("put")).build().unwrap();
        let doc = export_json(vec![post, put]);
        let item = &doc["paths"]["/items/{item_id}"];
        assert_eq!(item["parameters"][0]["description"], "put");
        assert_eq!(item["post"]["parameters"][0]["description"], "posted");
    }

    #[test]
    fn typed_and_untyped_operations_on_one_path_override_the_odd_one_out() {
        let get = Endpoint::get("Get", "/items/{item_id}").describes("Get").returns_empty()
            .path_param(item_id("Item")).build().unwrap();
        let delete = Endpoint::delete("Delete", "/items/{item_id}").describes("Delete").returns_empty().build().unwrap();
        let doc = export_json(vec![delete, get]);
        let item = &doc["paths"]["/items/{item_id}"];
        assert_eq!(item["parameters"][0]["schema"], json!({"type": "integer"}));
        assert_eq!(item["delete"]["parameters"][0]["schema"], json!({"type": "string"}));
    }

    #[test]
    fn repeated_and_reserved_placeholders_declare_one_parameter_each_in_template_order() {
        let doc = export_json(vec![
            Endpoint::get("Get", "/{owner}/{+rest}/{owner}").describes("Get").returns_empty()
                .path_param(ParamDef::path("rest", QueryParamType::String).describes("Remaining path"))
                .path_param(ParamDef::path("owner", QueryParamType::Boolean))
                .build().unwrap(),
        ]);
        let params = &doc["paths"]["/{owner}/{rest}/{owner}"]["parameters"];
        let names: Vec<&str> = params.as_array().unwrap().iter().map(|param| param["name"].as_str().unwrap()).collect();
        assert_eq!(names, ["owner", "rest"]);
        assert_eq!(params[0]["schema"], json!({"type": "boolean"}));
        assert_eq!(params[1]["description"], "Remaining path");
    }

    #[test]
    fn enum_and_number_path_parameters_export_their_schema() {
        let doc = export_json(vec![
            Endpoint::get("Get", "/{kind}/{ratio}").describes("Get").returns_empty()
                .path_param(ParamDef::path("kind", QueryParamType::Enum(vec!["cat".into(), "dog".into()])))
                .path_param(ParamDef::path("ratio", QueryParamType::Number))
                .build().unwrap(),
        ]);
        let params = &doc["paths"]["/{kind}/{ratio}"]["parameters"];
        assert_eq!(params[0]["schema"], json!({"type": "string", "enum": ["cat", "dog"]}));
        assert_eq!(params[1]["schema"], json!({"type": "number"}));
    }

    #[test]
    fn deprecated_operations_say_so_and_others_emit_nothing() {
        let doc = export_json(vec![
            Endpoint::get("Old", "/old").describes("Old").returns_empty().deprecated().build().unwrap(),
            Endpoint::get("New", "/new").describes("New").returns_empty().build().unwrap(),
        ]);
        assert_eq!(doc["paths"]["/old"]["get"]["deprecated"], true);
        assert!(doc["paths"]["/new"]["get"].get("deprecated").is_none());
    }
}
