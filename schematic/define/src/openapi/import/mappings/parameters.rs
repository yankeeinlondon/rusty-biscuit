use openapiv3::{ReferenceOr, Schema, SchemaKind, Type};

use super::super::diagnostics::OpenApiDiagnostic;
use super::super::resolver::RefResolver;
use crate::params::{ParamDef, ParamStyle, QueryParamType};

pub fn map_parameters(
    parameters: &[ReferenceOr<openapiv3::Parameter>],
    resolver: &RefResolver,
    diagnostics: &mut Vec<OpenApiDiagnostic>,
) -> crate::params::EndpointParams {
    let mut params = crate::params::EndpointParams::default();

    for param_ref in parameters {
        let param = match resolver.resolve_parameter(param_ref) {
            Ok(p) => p,
            Err(e) => {
                diagnostics.push(OpenApiDiagnostic::error(
                    "parameter".to_string(),
                    format!("Failed to resolve parameter: {}", e),
                ));
                continue;
            }
        };

        if let Some(param_def) = map_parameter(param, diagnostics) {
            match param {
                openapiv3::Parameter::Query { .. } => params.query.push(param_def),
                openapiv3::Parameter::Header { .. } => params.header.push(param_def),
                openapiv3::Parameter::Cookie { .. } => params.cookie.push(param_def),
                openapiv3::Parameter::Path { .. } => {}
            }
        }
    }

    params
}

pub fn map_parameter(
    param: &openapiv3::Parameter,
    diagnostics: &mut Vec<OpenApiDiagnostic>,
) -> Option<ParamDef> {
    let (name, required, description, schema, style, explode) = match param {
        openapiv3::Parameter::Query {
            parameter_data,
            allow_reserved: _,
            style,
            allow_empty_value: _,
        } => (
            &parameter_data.name,
            parameter_data.required,
            &parameter_data.description,
            get_param_schema(parameter_data),
            map_query_style(style),
            parameter_data.explode.unwrap_or(false),
        ),
        openapiv3::Parameter::Header {
            parameter_data,
            style,
        } => (
            &parameter_data.name,
            parameter_data.required,
            &parameter_data.description,
            get_param_schema(parameter_data),
            map_header_style(style),
            parameter_data.explode.unwrap_or(false),
        ),
        openapiv3::Parameter::Cookie {
            parameter_data,
            style,
        } => (
            &parameter_data.name,
            parameter_data.required,
            &parameter_data.description,
            get_param_schema(parameter_data),
            map_cookie_style(style),
            parameter_data.explode.unwrap_or(false),
        ),
        openapiv3::Parameter::Path { .. } => {
            return None;
        }
    };

    let param_type = match schema {
        Some(s) => map_schema_to_param_type(s, diagnostics),
        None => QueryParamType::String,
    };

    Some(ParamDef {
        name: name.clone(),
        required,
        description: description.clone(),
        param_type,
        explode,
        style,
    })
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
                let values: Vec<String> =
                    st.enumeration.iter().filter_map(|v| v.clone()).collect();
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
                let values: Vec<String> =
                    st.enumeration.iter().filter_map(|v| v.clone()).collect();
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
