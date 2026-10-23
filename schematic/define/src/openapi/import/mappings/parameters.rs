use openapiv3::{Parameter, ReferenceOr, Schema, SchemaKind, Type};

use super::super::diagnostics::OpenApiDiagnostic;
use super::super::resolver::RefResolver;
use super::import_constraints;
use crate::params::{EndpointParams, ParamDef, ParamStyle, QueryParamType};
use crate::types::RestMethod;

/// Maps one operation's effective parameters: the path item's `inherited`
/// list overlaid by the operation's `declared` list.
///
/// Local `$ref`s are resolved before merging so each parameter's identity
/// (name and location; header names compared ASCII-case-insensitively) is
/// known. An operation parameter replaces the inherited one with the same
/// identity in place; the rest are appended. An unresolvable or remote
/// `$ref` is reported as an ERROR at its RFC 6901 pointer and skipped.
///
/// `path` stays empty unless some path parameter is typed (non-string,
/// described, deprecated, or constrained); then every placeholder of `path`
/// is declared, in template order, so the endpoint passes typed-path
/// validation.
///
/// Each parameter's value rules are read from its schema at the parameter's
/// own location: the referenced component for a `$ref` parameter.
pub fn map_parameters<'a>(
    path: &str,
    method: RestMethod,
    inherited: &'a [ReferenceOr<Parameter>],
    declared: &'a [ReferenceOr<Parameter>],
    resolver: &RefResolver<'a>,
    diagnostics: &mut Vec<OpenApiDiagnostic>,
) -> EndpointParams {
    let escaped_path = path.replace('~', "~0").replace('/', "~1");
    let path_item_pointer = format!("#/paths/{escaped_path}/parameters");
    let operation_pointer = format!(
        "#/paths/{escaped_path}/{}/parameters",
        method.to_string().to_lowercase()
    );

    // (parameter, where it is referenced, where its definition sits)
    let mut merged: Vec<(&Parameter, String, String)> = Vec::new();
    for (param_refs, pointer) in [(inherited, &path_item_pointer), (declared, &operation_pointer)] {
        for (index, param_ref) in param_refs.iter().enumerate() {
            let location = format!("{pointer}/{index}");
            let Some(param) = resolve_parameter(param_ref, &location, resolver, diagnostics) else {
                continue;
            };
            let definition = match param_ref {
                ReferenceOr::Reference { reference } => reference.clone(),
                ReferenceOr::Item(_) => location.clone(),
            };
            match merged.iter().position(|(existing, ..)| same_identity(existing, param)) {
                Some(position) => merged[position] = (param, location, definition),
                None => merged.push((param, location, definition)),
            }
        }
    }

    let mut params = EndpointParams::default();
    let mut path_params: Vec<(ParamDef, String)> = Vec::new();
    for (param, location, definition) in merged {
        // `openapiv3` accepts an optional path parameter; OpenAPI does not.
        if matches!(param, Parameter::Path { .. }) && !param.parameter_data_ref().required {
            diagnostics.push(OpenApiDiagnostic::error(
                format!("{location}/required"),
                "a path parameter must declare `required: true`".to_string(),
            ));
        }
        let param_def = map_parameter(param, &format!("{definition}/schema"), resolver, diagnostics);
        match param {
            Parameter::Query { .. } => params.query.push(param_def),
            Parameter::Header { .. } => params.header.push(param_def),
            Parameter::Cookie { .. } => params.cookie.push(param_def),
            Parameter::Path { .. } => path_params.push((param_def, location)),
        }
    }
    params.path = typed_path_params(path, path_params, diagnostics);
    params
}

/// Resolves a parameter `$ref` within this document; a remote reference is
/// never fetched.
fn resolve_parameter<'a>(
    param_ref: &'a ReferenceOr<Parameter>,
    location: &str,
    resolver: &RefResolver<'a>,
    diagnostics: &mut Vec<OpenApiDiagnostic>,
) -> Option<&'a Parameter> {
    match param_ref {
        ReferenceOr::Item(param) => Some(param),
        ReferenceOr::Reference { reference } if !reference.starts_with('#') => {
            diagnostics.push(OpenApiDiagnostic::error(
                location.to_string(),
                format!("Remote parameter reference '{reference}' is not supported (never fetched); parameter skipped"),
            ));
            None
        }
        ReferenceOr::Reference { reference } => match resolver.resolve_parameter(param_ref) {
            Ok(param) => Some(param),
            Err(error) => {
                diagnostics.push(OpenApiDiagnostic::error(
                    location.to_string(),
                    format!("Unresolved parameter reference '{reference}' ({error}); parameter skipped"),
                ));
                None
            }
        },
    }
}

fn same_identity(left: &Parameter, right: &Parameter) -> bool {
    let (left_name, right_name) = (&left.parameter_data_ref().name, &right.parameter_data_ref().name);
    match (left, right) {
        (Parameter::Header { .. }, Parameter::Header { .. }) => left_name.eq_ignore_ascii_case(right_name),
        _ => std::mem::discriminant(left) == std::mem::discriminant(right) && left_name == right_name,
    }
}

/// The endpoint's typed path parameters, or none when every declared one is
/// a plain undescribed, undeprecated, unconstrained string
/// (indistinguishable on export from an undeclared placeholder).
fn typed_path_params(
    path: &str,
    mut declared: Vec<(ParamDef, String)>,
    diagnostics: &mut Vec<OpenApiDiagnostic>,
) -> Vec<ParamDef> {
    // Typed-path validation accepts only scalar and enum path types.
    for (param, location) in &mut declared {
        if matches!(param.param_type, QueryParamType::Array(_) | QueryParamType::Json) {
            diagnostics.push(OpenApiDiagnostic::warn(
                location.clone(),
                format!(
                    "Path parameter '{}' has an array or structured schema, which a path segment cannot carry; imported as string",
                    param.name
                ),
            ));
            param.param_type = QueryParamType::String;
        }
    }
    let typed = declared.iter().any(|(param, _)| {
        param.param_type != QueryParamType::String
            || param.description.is_some()
            || param.deprecated
            || param.constraints.is_some()
    });
    if !typed {
        return Vec::new();
    }

    let mut placeholders: Vec<&str> = Vec::new();
    for name in crate::pagination::path_params(path) {
        if !placeholders.contains(&name) {
            placeholders.push(name);
        }
    }
    for (param, location) in &declared {
        if !placeholders.contains(&param.name.as_str()) {
            diagnostics.push(OpenApiDiagnostic::warn(
                location.clone(),
                format!("Path parameter '{}' has no placeholder in '{path}'; skipped", param.name),
            ));
        }
    }
    placeholders
        .into_iter()
        .map(|name| {
            declared
                .iter()
                .find(|(param, _)| param.name == name)
                .map(|(param, _)| param.clone())
                .unwrap_or_else(|| ParamDef::path(name, QueryParamType::String))
        })
        .collect()
}

/// Maps one resolved parameter. Path parameters are always required,
/// `simple` style, and not exploded.
///
/// `schema_pointer` locates the parameter's `schema` in the source document.
/// An inline schema's value rules, `items` rules included, become
/// [`ParamDef::constraints`]; a `$ref` schema carries its rules in the
/// component and contributes none. An `enum` already spelled by
/// [`QueryParamType::Enum`] is not repeated as `allowed_values`.
pub fn map_parameter(
    param: &Parameter,
    schema_pointer: &str,
    resolver: &RefResolver,
    diagnostics: &mut Vec<OpenApiDiagnostic>,
) -> ParamDef {
    let parameter_data = param.parameter_data_ref();
    let style = match param {
        Parameter::Query { style, .. } => map_query_style(style),
        Parameter::Header { style, .. } => map_header_style(style),
        Parameter::Cookie { style, .. } => map_cookie_style(style),
        Parameter::Path { .. } => ParamStyle::Simple,
    };
    let param_type = match get_param_schema(parameter_data) {
        Some(schema) => map_schema_to_param_type(schema, diagnostics),
        None => QueryParamType::String,
    };
    let constraints = match get_param_schema(parameter_data) {
        Some(schema @ ReferenceOr::Item(_)) => import_constraints(schema, schema_pointer, resolver, diagnostics),
        _ => None,
    }
    .and_then(|mut rules| {
        if matches!(param_type, QueryParamType::Enum(_)) {
            rules.allowed_values = None;
        }
        (!rules.is_empty()).then_some(rules)
    });
    let param_def = ParamDef {
        required: parameter_data.required,
        description: parameter_data.description.clone(),
        deprecated: parameter_data.deprecated == Some(true),
        // OpenAPI 3.0.3: "When style is form, the default value is true. For
        // all other styles, the default value is false."
        explode: parameter_data.explode.unwrap_or(style == ParamStyle::Form),
        style,
        constraints,
        ..ParamDef::new(parameter_data.name.clone(), param_type)
    };
    match param {
        Parameter::Path { .. } => ParamDef {
            description: param_def.description,
            deprecated: param_def.deprecated,
            constraints: param_def.constraints,
            ..ParamDef::path(param_def.name, param_def.param_type)
        },
        _ => param_def,
    }
}

fn get_param_schema(data: &openapiv3::ParameterData) -> Option<&ReferenceOr<Schema>> {
    match &data.format {
        openapiv3::ParameterSchemaOrContent::Schema(s) => Some(s),
        openapiv3::ParameterSchemaOrContent::Content(_) => None,
    }
}

fn map_query_style(style: &openapiv3::QueryStyle) -> ParamStyle {
    match style {
        openapiv3::QueryStyle::Form => ParamStyle::Form,
        openapiv3::QueryStyle::SpaceDelimited => ParamStyle::SpaceDelimited,
        openapiv3::QueryStyle::PipeDelimited => ParamStyle::PipeDelimited,
        openapiv3::QueryStyle::DeepObject => ParamStyle::DeepObject,
    }
}

fn map_header_style(style: &openapiv3::HeaderStyle) -> ParamStyle {
    match style {
        openapiv3::HeaderStyle::Simple => ParamStyle::Simple,
    }
}

fn map_cookie_style(style: &openapiv3::CookieStyle) -> ParamStyle {
    match style {
        openapiv3::CookieStyle::Form => ParamStyle::Form,
    }
}

fn map_schema_to_param_type(
    schema: &ReferenceOr<Schema>,
    diagnostics: &mut Vec<OpenApiDiagnostic>,
) -> QueryParamType {
    match schema {
        ReferenceOr::Reference { reference } => {
            diagnostics.push(OpenApiDiagnostic::info(
                reference.clone(),
                "Parameter references schema component".to_string(),
            ));
            QueryParamType::Json
        }
        ReferenceOr::Item(s) => match &s.schema_kind {
            SchemaKind::Type(Type::String(st)) => {
                let values: Vec<String> = st.enumeration.iter().filter_map(|v| v.clone()).collect();
                if !values.is_empty() {
                    return QueryParamType::Enum(values);
                }
                QueryParamType::String
            }
            SchemaKind::Type(Type::Integer(_)) => QueryParamType::Integer,
            SchemaKind::Type(Type::Number(_)) => QueryParamType::Number,
            SchemaKind::Type(Type::Boolean(_)) => QueryParamType::Boolean,
            SchemaKind::Type(Type::Array(arr)) => {
                let item_type = if let Some(ref items) = arr.items {
                    Box::new(map_schema_to_param_type_boxed(items, diagnostics))
                } else {
                    Box::new(QueryParamType::String)
                };
                QueryParamType::Array(item_type)
            }
            _ => QueryParamType::Json,
        },
    }
}

fn map_schema_to_param_type_boxed(
    schema: &ReferenceOr<Box<Schema>>,
    diagnostics: &mut Vec<OpenApiDiagnostic>,
) -> QueryParamType {
    match schema {
        ReferenceOr::Reference { reference } => {
            diagnostics.push(OpenApiDiagnostic::info(
                reference.clone(),
                "Parameter references schema component".to_string(),
            ));
            QueryParamType::Json
        }
        ReferenceOr::Item(s) => match &s.schema_kind {
            SchemaKind::Type(Type::String(st)) => {
                let values: Vec<String> = st.enumeration.iter().filter_map(|v| v.clone()).collect();
                if !values.is_empty() {
                    return QueryParamType::Enum(values);
                }
                QueryParamType::String
            }
            SchemaKind::Type(Type::Integer(_)) => QueryParamType::Integer,
            SchemaKind::Type(Type::Number(_)) => QueryParamType::Number,
            SchemaKind::Type(Type::Boolean(_)) => QueryParamType::Boolean,
            SchemaKind::Type(Type::Array(arr)) => {
                let item_type = if let Some(ref items) = arr.items {
                    Box::new(map_schema_to_param_type_boxed(items, diagnostics))
                } else {
                    Box::new(QueryParamType::String)
                };
                QueryParamType::Array(item_type)
            }
            _ => QueryParamType::Json,
        },
    }
}
