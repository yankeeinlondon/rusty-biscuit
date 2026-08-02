//! Maps [`EndpointParams`] onto OpenAPI operation parameters.
//!
//! Path parameters are handled separately in [`super::paths`] because they are
//! recovered from the path template rather than declared; everything here comes
//! from an endpoint's explicit `params`.

use indexmap::IndexMap;
use openapiv3::{
    CookieStyle, HeaderStyle, Parameter, ParameterData, ParameterSchemaOrContent, QueryStyle,
    ReferenceOr, Schema, SchemaKind, Type,
};

use crate::params::{EndpointParams, ParamDef, ParamStyle, QueryParamType};

/// Builds the operation-level parameter list for an endpoint.
///
/// Returns an empty vector when the endpoint declares no parameters.
pub(super) fn map_parameters(params: Option<&EndpointParams>) -> Vec<ReferenceOr<Parameter>> {
    let Some(params) = params else {
        return Vec::new();
    };

    let query = params.query.iter().map(|param| {
        Parameter::Query {
            parameter_data: parameter_data(param),
            allow_reserved: false,
            style: query_style(param.style),
            // Distinct from "not required": omitted rather than asserted false.
            allow_empty_value: None,
        }
    });

    let header = params.header.iter().map(|param| Parameter::Header {
        parameter_data: parameter_data(param),
        style: HeaderStyle::Simple,
    });

    let cookie = params.cookie.iter().map(|param| Parameter::Cookie {
        parameter_data: parameter_data(param),
        style: CookieStyle::Form,
    });

    query
        .chain(header)
        .chain(cookie)
        .map(ReferenceOr::Item)
        .collect()
}

/// Builds the shared parameter payload for one [`ParamDef`].
fn parameter_data(param: &ParamDef) -> ParameterData {
    ParameterData {
        name: param.name.clone(),
        description: param.description.clone(),
        required: param.required,
        deprecated: None,
        format: ParameterSchemaOrContent::Schema(ReferenceOr::Item(param_schema(
            &param.param_type,
        ))),
        example: None,
        examples: IndexMap::new(),
        // Only emitted when it differs from the style's default, which for
        // `form` is `explode: true`.
        explode: (!param.explode).then_some(false),
        extensions: IndexMap::new(),
    }
}

/// Maps a parameter's declared type onto a JSON Schema.
fn param_schema(param_type: &QueryParamType) -> Schema {
    let schema_kind = match param_type {
        QueryParamType::String => SchemaKind::Type(Type::String(Default::default())),
        QueryParamType::Integer => SchemaKind::Type(Type::Integer(Default::default())),
        QueryParamType::Number => SchemaKind::Type(Type::Number(Default::default())),
        QueryParamType::Boolean => SchemaKind::Type(Type::Boolean(Default::default())),
        QueryParamType::Array(inner) => SchemaKind::Type(Type::Array(openapiv3::ArrayType {
            items: Some(ReferenceOr::Item(Box::new(param_schema(inner)))),
            min_items: None,
            max_items: None,
            unique_items: false,
        })),
        QueryParamType::Enum(values) => SchemaKind::Type(Type::String(openapiv3::StringType {
            enumeration: values.iter().map(|value| Some(value.clone())).collect(),
            ..Default::default()
        })),
        // No `type`, so any JSON value validates.
        QueryParamType::Json => SchemaKind::Any(openapiv3::AnySchema::default()),
    };

    Schema {
        schema_data: Default::default(),
        schema_kind,
    }
}

fn query_style(style: ParamStyle) -> QueryStyle {
    match style {
        ParamStyle::SpaceDelimited => QueryStyle::SpaceDelimited,
        ParamStyle::PipeDelimited => QueryStyle::PipeDelimited,
        ParamStyle::DeepObject => QueryStyle::DeepObject,
        // `Simple` is not a legal query style; `form` is the OpenAPI default.
        _ => QueryStyle::Form,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn query_param(name: &str, param_type: QueryParamType, required: bool) -> ParamDef {
        ParamDef {
            name: name.to_string(),
            required,
            description: Some(format!("The {name}")),
            param_type,
            explode: true,
            style: ParamStyle::Form,
        }
    }

    #[test]
    fn no_params_yields_no_parameters() {
        assert!(map_parameters(None).is_empty());
    }

    #[test]
    fn query_params_become_query_parameters() {
        let params = EndpointParams {
            query: vec![
                query_param("limit", QueryParamType::Integer, false),
                query_param("cursor", QueryParamType::String, true),
            ],
            ..Default::default()
        };

        let mapped = map_parameters(Some(&params));
        assert_eq!(mapped.len(), 2);

        let ReferenceOr::Item(Parameter::Query { parameter_data, .. }) = &mapped[0] else {
            panic!("expected a query parameter");
        };
        assert_eq!(parameter_data.name, "limit");
        assert!(!parameter_data.required);
        assert_eq!(parameter_data.description.as_deref(), Some("The limit"));

        let ReferenceOr::Item(Parameter::Query { parameter_data, .. }) = &mapped[1] else {
            panic!("expected a query parameter");
        };
        assert!(parameter_data.required);
    }

    #[test]
    fn integer_params_export_an_integer_schema() {
        let params = EndpointParams {
            query: vec![query_param("limit", QueryParamType::Integer, false)],
            ..Default::default()
        };

        let mapped = map_parameters(Some(&params));
        let ReferenceOr::Item(Parameter::Query { parameter_data, .. }) = &mapped[0] else {
            panic!("expected a query parameter");
        };
        let ParameterSchemaOrContent::Schema(ReferenceOr::Item(schema)) = &parameter_data.format
        else {
            panic!("expected an inline schema");
        };
        assert!(matches!(
            schema.schema_kind,
            SchemaKind::Type(Type::Integer(_))
        ));
    }

    #[test]
    fn enum_params_carry_their_value_domain() {
        let params = EndpointParams {
            query: vec![query_param(
                "order",
                QueryParamType::Enum(vec!["asc".to_string(), "desc".to_string()]),
                false,
            )],
            ..Default::default()
        };

        let mapped = map_parameters(Some(&params));
        let ReferenceOr::Item(Parameter::Query { parameter_data, .. }) = &mapped[0] else {
            panic!("expected a query parameter");
        };
        let ParameterSchemaOrContent::Schema(ReferenceOr::Item(schema)) = &parameter_data.format
        else {
            panic!("expected an inline schema");
        };
        let SchemaKind::Type(Type::String(string)) = &schema.schema_kind else {
            panic!("expected a string schema");
        };
        assert_eq!(string.enumeration.len(), 2);
    }

    #[test]
    fn array_params_export_an_items_schema() {
        let params = EndpointParams {
            query: vec![query_param(
                "ids",
                QueryParamType::Array(Box::new(QueryParamType::String)),
                false,
            )],
            ..Default::default()
        };

        let mapped = map_parameters(Some(&params));
        let ReferenceOr::Item(Parameter::Query { parameter_data, .. }) = &mapped[0] else {
            panic!("expected a query parameter");
        };
        let ParameterSchemaOrContent::Schema(ReferenceOr::Item(schema)) = &parameter_data.format
        else {
            panic!("expected an inline schema");
        };
        let SchemaKind::Type(Type::Array(array)) = &schema.schema_kind else {
            panic!("expected an array schema");
        };
        assert!(array.items.is_some());
    }

    #[test]
    fn header_and_cookie_params_land_in_their_own_locations() {
        let params = EndpointParams {
            header: vec![query_param("X-Trace", QueryParamType::String, true)],
            cookie: vec![query_param("session", QueryParamType::String, false)],
            ..Default::default()
        };

        let mapped = map_parameters(Some(&params));
        assert_eq!(mapped.len(), 2);
        assert!(matches!(
            &mapped[0],
            ReferenceOr::Item(Parameter::Header { .. })
        ));
        assert!(matches!(
            &mapped[1],
            ReferenceOr::Item(Parameter::Cookie { .. })
        ));
    }
}
