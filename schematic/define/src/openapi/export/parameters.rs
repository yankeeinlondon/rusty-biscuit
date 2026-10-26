//! Maps [`EndpointParams`] onto OpenAPI operation parameters.
//!
//! Path parameters are handled separately in [`super::paths`] because they are
//! shared across the operations of a path item; everything here is an
//! endpoint's own query, header, and cookie parameters.
//!
//! A parameter's schema is its type's schema plus its value rules
//! ([`ParamDef::constraints`], or the effective bounds of the page size the
//! endpoint's pagination strategy adopts), in OpenAPI 3.0 form: a numeric
//! exclusive bound is the bound with `exclusiveMinimum: true`, and `const` a
//! one-value `enum`. A parameter without rules exports its type's schema
//! alone.

use indexmap::IndexMap;
use openapiv3::{
    CookieStyle, HeaderStyle, Parameter, ParameterData, ParameterSchemaOrContent, QueryStyle,
    ReferenceOr, Schema, SchemaKind, Type,
};
use serde_json::{Map, Value};

use super::super::OpenApiError;
use crate::constraints::{Constraints, EXACT_EXTENSION};
use crate::pagination::{AdoptedPageSize, ParamLocation};
use crate::params::{EndpointParams, ParamDef, ParamStyle, QueryParamType};

/// Builds the operation-level parameter list for an endpoint.
///
/// `pointer` is the JSON pointer of the operation, which locates a schema
/// that fails to convert; `adopted` is the endpoint's adopted page-size
/// parameter, whose effective rules replace that parameter's own. Returns an
/// empty vector when the endpoint declares no parameters.
///
/// ## Errors
///
/// [`OpenApiError`] when a parameter's rules do not convert to OpenAPI 3.0.3.
pub(super) fn map_parameters(
    params: Option<&EndpointParams>,
    adopted: Option<&AdoptedPageSize>,
    pointer: &str,
) -> Result<Vec<ReferenceOr<Parameter>>, OpenApiError> {
    let Some(params) = params else {
        return Ok(Vec::new());
    };

    let mut mapped = Vec::new();
    let located = [(ParamLocation::Query, &params.query), (ParamLocation::Header, &params.header), (ParamLocation::Cookie, &params.cookie)];
    for (location, defs) in located {
        for param in defs {
            let at = format!("{pointer}/parameters/{}/schema", mapped.len());
            let parameter_data = parameter_data(param, effective_constraints(param, location, adopted), &at)?;
            mapped.push(ReferenceOr::Item(match location {
                ParamLocation::Query => Parameter::Query {
                    parameter_data,
                    allow_reserved: false,
                    style: query_style(param.style),
                    // Distinct from "not required": omitted rather than asserted false.
                    allow_empty_value: None,
                },
                ParamLocation::Header => Parameter::Header { parameter_data, style: HeaderStyle::Simple },
                _ => Parameter::Cookie { parameter_data, style: CookieStyle::Form },
            }));
        }
    }
    Ok(mapped)
}

/// The rules `param`, declared at `location`, exports with: the adopted page
/// size's effective rules when `adopted` names it, else its own.
pub(super) fn effective_constraints<'a>(
    param: &'a ParamDef,
    location: ParamLocation,
    adopted: Option<&'a AdoptedPageSize>,
) -> Option<&'a Constraints> {
    match adopted {
        Some(adopted) if adopted.location == location && adopted.name == param.name => Some(&adopted.constraints),
        _ => param.constraints.as_deref(),
    }
}

/// Builds the shared parameter payload for one [`ParamDef`] whose value
/// rules are `rules`, its schema located at `pointer`.
fn parameter_data(param: &ParamDef, rules: Option<&Constraints>, pointer: &str) -> Result<ParameterData, OpenApiError> {
    Ok(ParameterData {
        name: param.name.clone(),
        description: param.description.clone(),
        required: param.required,
        deprecated: param.deprecated.then_some(true),
        format: ParameterSchemaOrContent::Schema(ReferenceOr::Item(constrained_param_schema(
            &param.param_type,
            rules,
            pointer,
        )?)),
        example: None,
        examples: IndexMap::new(),
        // OpenAPI defaults `explode` to true for `form` and false for every
        // other style; `false` is always written, `true` only where it is
        // not the default (every style but `form`).
        explode: match (param.explode, param.style) {
            (false, _) => Some(false),
            (true, ParamStyle::Form) => None,
            (true, _) => Some(true),
        },
        extensions: IndexMap::new(),
    })
}

/// Maps a parameter's declared type onto a JSON Schema, without value rules.
pub(super) fn param_schema(param_type: &QueryParamType) -> Schema {
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

/// A parameter's type schema ([`param_schema`]) carrying `rules`, converted
/// to OpenAPI 3.0.3 at `pointer`. Without rules, or with empty ones, this is
/// [`param_schema`] unchanged.
///
/// An `enum` the type already declares ([`QueryParamType::Enum`]) is the
/// parameter's value domain, so the rules' `enum` and `const` are not
/// exported beside it.
///
/// ## Errors
///
/// [`OpenApiError`] when the combined schema does not convert.
pub(super) fn constrained_param_schema(
    param_type: &QueryParamType,
    rules: Option<&Constraints>,
    pointer: &str,
) -> Result<Schema, OpenApiError> {
    let typed = param_schema(param_type);
    let Some(rules) = rules.filter(|rules| !rules.is_empty()) else {
        return Ok(typed);
    };
    let mut schema = match serde_json::to_value(&typed) {
        Ok(Value::Object(schema)) => schema,
        _ => Map::new(),
    };
    merge_rules(&mut schema, rules.to_schema());
    super::conversion::convert_schema_at(&Value::Object(schema), pointer)
}

/// Adds the keywords of `rules` (from [`Constraints::to_schema`]) to
/// `schema`: an `items` object joins the schema's own `items`, exact-number
/// entries join its [`EXACT_EXTENSION`], and an `enum` already on `schema`
/// keeps out the rules' `enum` and `const`.
pub(super) fn merge_rules(schema: &mut Map<String, Value>, rules: Map<String, Value>) {
    let typed_enum = schema.contains_key("enum");
    for (keyword, value) in rules {
        match (keyword.as_str(), value) {
            ("enum" | "const", _) if typed_enum => {}
            ("items" | EXACT_EXTENSION, Value::Object(inner)) if schema.get(&keyword).is_some_and(Value::is_object) => {
                let Some(Value::Object(existing)) = schema.get_mut(&keyword) else { continue };
                if keyword == "items" {
                    merge_rules(existing, inner);
                } else {
                    existing.extend(inner);
                }
            }
            (_, value) => {
                schema.insert(keyword, value);
            }
        }
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
        ParamDef { constraints: None, deprecated: false, rust_name: None, secret: false, pager_managed: false,
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
        assert!(map_parameters(None, None, "#").unwrap().is_empty());
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

        let mapped = map_parameters(Some(&params), None, "#").unwrap();
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

        let mapped = map_parameters(Some(&params), None, "#").unwrap();
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

        let mapped = map_parameters(Some(&params), None, "#").unwrap();
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

        let mapped = map_parameters(Some(&params), None, "#").unwrap();
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

        let mapped = map_parameters(Some(&params), None, "#").unwrap();
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

    #[test]
    fn only_deprecated_parameters_carry_the_flag() {
        let mut old = query_param("old", QueryParamType::String, false);
        old.deprecated = true;
        let params = EndpointParams {
            query: vec![old.clone(), query_param("new", QueryParamType::String, false)],
            header: vec![old.clone()],
            cookie: vec![old],
            ..Default::default()
        };
        let mapped = serde_json::to_value(map_parameters(Some(&params), None, "#").unwrap()).unwrap();
        assert_eq!(mapped[0]["deprecated"], true);
        assert!(mapped[1].get("deprecated").is_none());
        assert_eq!(mapped[2]["deprecated"], true);
        assert_eq!(mapped[3]["deprecated"], true);
    }
}
