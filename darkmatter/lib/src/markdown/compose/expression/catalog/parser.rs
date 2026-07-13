//! Validation parser for authored expression-function catalogs.

use std::collections::HashSet;

use serde::Deserialize;

use super::{
    DataType,
    ast::{
        CatalogExample, CatalogFunction, CatalogOverload, CatalogParam, CatalogReturn,
        CatalogVerification, ExpressionFunctionCatalog,
    },
};
use crate::markdown::schemas::{parse_yaml_schema, to_json_schema};

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("catalog error{location}: {kind}", location = format_location(.function.as_deref(), *.overload, .field.as_deref()))]
pub(crate) struct CatalogParseError {
    pub function: Option<String>,
    pub overload: Option<usize>,
    pub field: Option<String>,
    pub kind: CatalogErrorKind,
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub(crate) enum CatalogErrorKind {
    #[error("invalid YAML: {0}")]
    InvalidYaml(String),
    #[error("catalog $schema declaration is invalid: {0}")]
    SchemaDeclaration(String),
    #[error("catalog document fails its $schema: {0}")]
    SchemaInstance(String),
    #[error("`kind` must be `expression-function-catalog`")] 
    InvalidKind,
    #[error("at least one function is required")]
    EmptyCatalog,
    #[error("at least one overload is required")]
    EmptyOverloads,
    #[error("invalid identifier `{0}`")]
    InvalidIdentifier(String),
    #[error("value must not be empty")]
    EmptyValue,
    #[error("order must be a non-negative integer")]
    InvalidOrder,
    #[error("duplicate order {0}")]
    DuplicateOrder(usize),
    #[error("duplicate function name `{0}`")]
    DuplicateFunction(String),
    #[error("duplicate parameter name `{0}`")]
    DuplicateParameter(String),
    #[error("unknown or unsupported type keyword `{0}`")]
    UnknownType(String),
    #[error("a required parameter cannot follow an optional parameter")]
    RequiredAfterOptional,
    #[error("a variadic parameter must be last")]
    NonFinalVariadic,
    #[error("a variadic parameter cannot be optional")]
    OptionalVariadic,
    #[error("display-only verification requires a non-empty reason")]
    MissingDisplayReason,
    #[error("executable verification cannot carry a reason")]
    ExecutableReason,
    #[error("duplicate rendered signature `{0}`")]
    DuplicateSignature(String),
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawCatalog {
    kind: String,
    #[serde(rename = "$schema")]
    schema: serde_yaml_ng::Value,
    functions: Vec<RawFunction>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawFunction {
    name: String,
    category: String,
    order: i64,
    description: String,
    overloads: Vec<RawOverload>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawOverload {
    #[serde(default)]
    parameters: Vec<RawParam>,
    returns: RawReturn,
    example: RawExample,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawParam {
    name: String,
    #[serde(rename = "type")]
    ty: String,
    #[serde(default)]
    array: bool,
    #[serde(default)]
    optional: bool,
    #[serde(default)]
    variadic: bool,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawReturn {
    #[serde(rename = "type")]
    ty: String,
    #[serde(default)]
    array: bool,
    #[serde(default)]
    fallible: bool,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawExample {
    expression: String,
    result: String,
    verification: RawVerification,
    reason: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "kebab-case")]
enum RawVerification {
    Executable,
    DisplayOnly,
}

pub(crate) fn parse_expression_function_catalog(
    yaml: &str,
) -> Result<ExpressionFunctionCatalog, CatalogParseError> {
    let value: serde_yaml_ng::Value = serde_yaml_ng::from_str(yaml)
        .map_err(|err| error(None, None, None, CatalogErrorKind::InvalidYaml(err.to_string())))?;
    let raw: RawCatalog = serde_yaml_ng::from_value(value.clone())
        .map_err(|err| error(None, None, None, CatalogErrorKind::InvalidYaml(err.to_string())))?;

    // Structural validation: the catalog document must conform to its own
    // declared SimplifiedSchema before dedicated function-domain checks run.
    // This makes the authored `$schema` the structural authority in the
    // loading path (requirement 3).
    let schema = parse_yaml_schema(&raw.schema)
        .map_err(|err| error(None, None, Some("$schema"), CatalogErrorKind::SchemaDeclaration(err.to_string())))?;
    let json_schema = to_json_schema(&schema)
        .map_err(|err| error(None, None, Some("$schema"), CatalogErrorKind::SchemaDeclaration(err.to_string())))?;
    let validator = jsonschema::validator_for(&json_schema)
        .map_err(|err| error(None, None, Some("$schema"), CatalogErrorKind::SchemaDeclaration(err.to_string())))?;
    let instance = serde_json::to_value(&value)
        .map_err(|err| error(None, None, None, CatalogErrorKind::InvalidYaml(err.to_string())))?;
    validator.validate(&instance)
        .map_err(|err| error(None, None, Some("$schema"), CatalogErrorKind::SchemaInstance(err.to_string())))?;

    if raw.kind != "expression-function-catalog" {
        return Err(error(None, None, Some("kind"), CatalogErrorKind::InvalidKind));
    }
    if raw.functions.is_empty() {
        return Err(error(None, None, Some("functions"), CatalogErrorKind::EmptyCatalog));
    }

    let mut names = HashSet::new();
    let mut orders = HashSet::new();
    let mut signatures = HashSet::new();
    let mut functions = Vec::with_capacity(raw.functions.len());
    for function in raw.functions {
        let name = function.name.clone();
        if !valid_identifier(&name) {
            return Err(error(Some(&name), None, Some("name"), CatalogErrorKind::InvalidIdentifier(name.clone())));
        }
        if !names.insert(function.name.clone()) {
            return Err(error(Some(&name), None, Some("name"), CatalogErrorKind::DuplicateFunction(name.clone())));
        }
        if function.category.trim().is_empty() {
            return Err(error(Some(&name), None, Some("category"), CatalogErrorKind::EmptyValue));
        }
        if function.description.trim().is_empty() {
            return Err(error(Some(&name), None, Some("description"), CatalogErrorKind::EmptyValue));
        }
        let order = usize::try_from(function.order).ok()
            .ok_or_else(|| error(Some(&name), None, Some("order"), CatalogErrorKind::InvalidOrder))?;
        if !orders.insert(order) {
            return Err(error(Some(&name), None, Some("order"), CatalogErrorKind::DuplicateOrder(order)));
        }
        if function.overloads.is_empty() {
            return Err(error(Some(&name), None, Some("overloads"), CatalogErrorKind::EmptyOverloads));
        }

        let mut overloads = Vec::with_capacity(function.overloads.len());
        for (index, overload) in function.overloads.into_iter().enumerate() {
            let mut parameter_names = HashSet::new();
            let mut optional_seen = false;
            let parameter_count = overload.parameters.len();
            let mut parameters = Vec::with_capacity(parameter_count);
            for (parameter_index, parameter) in overload.parameters.into_iter().enumerate() {
                if !valid_identifier(&parameter.name) {
                    return Err(error(Some(&name), Some(index), Some("parameters.name"), CatalogErrorKind::InvalidIdentifier(parameter.name)));
                }
                if !parameter_names.insert(parameter.name.clone()) {
                    return Err(error(Some(&name), Some(index), Some("parameters.name"), CatalogErrorKind::DuplicateParameter(parameter.name)));
                }
                if parameter.variadic && parameter.optional {
                    return Err(error(Some(&name), Some(index), Some("parameters"), CatalogErrorKind::OptionalVariadic));
                }
                if parameter.variadic && parameter_index + 1 != parameter_count {
                    return Err(error(Some(&name), Some(index), Some("parameters"), CatalogErrorKind::NonFinalVariadic));
                }
                if !parameter.optional && optional_seen {
                    return Err(error(Some(&name), Some(index), Some("parameters"), CatalogErrorKind::RequiredAfterOptional));
                }
                optional_seen |= parameter.optional;
                let ty = parse_type(&name, index, "parameters.type", &parameter.ty)?;
                parameters.push(CatalogParam { name: parameter.name, ty, array: parameter.array, optional: parameter.optional, variadic: parameter.variadic });
            }
            let signature = format!("{}({})", name, parameters.iter().map(render_parameter).collect::<Vec<_>>().join(", "));
            if !signatures.insert(signature.clone()) {
                return Err(error(Some(&name), Some(index), Some("parameters"), CatalogErrorKind::DuplicateSignature(signature)));
            }
            let return_ty = parse_type(&name, index, "returns.type", &overload.returns.ty)?;
            let example = validate_example(&name, index, overload.example)?;
            overloads.push(CatalogOverload {
                parameters,
                returns: CatalogReturn { ty: return_ty, array: overload.returns.array, fallible: overload.returns.fallible },
                example,
            });
        }
        functions.push(CatalogFunction { name, category: function.category, order, description: function.description, overloads });
    }
    Ok(ExpressionFunctionCatalog { functions })
}

fn parse_type(function: &str, overload: usize, field: &str, keyword: &str) -> Result<DataType, CatalogParseError> {
    DataType::from_keyword(keyword).ok_or_else(|| error(Some(function), Some(overload), Some(field), CatalogErrorKind::UnknownType(keyword.to_string())))
}

fn validate_example(function: &str, overload: usize, raw: RawExample) -> Result<CatalogExample, CatalogParseError> {
    if raw.expression.trim().is_empty() {
        return Err(error(Some(function), Some(overload), Some("example.expression"), CatalogErrorKind::EmptyValue));
    }
    let verification = match raw.verification {
        RawVerification::Executable => {
            if raw.reason.is_some() {
                return Err(error(Some(function), Some(overload), Some("example.reason"), CatalogErrorKind::ExecutableReason));
            }
            CatalogVerification::Executable
        }
        RawVerification::DisplayOnly => {
            let reason = raw.reason.as_deref().filter(|reason| !reason.trim().is_empty())
                .ok_or_else(|| error(Some(function), Some(overload), Some("example.reason"), CatalogErrorKind::MissingDisplayReason))?;
            CatalogVerification::DisplayOnly(reason.to_string())
        }
    };
    Ok(CatalogExample { expression: raw.expression, result: raw.result, verification, reason: raw.reason })
}

fn valid_identifier(value: &str) -> bool {
    let mut chars = value.chars();
    matches!(chars.next(), Some('a'..='z')) && chars.all(|character| character.is_ascii_lowercase() || character.is_ascii_digit() || character == '_')
}

fn render_parameter(parameter: &CatalogParam) -> String {
    if parameter.variadic { "...".to_string() }
    else if parameter.optional { format!("[{}]", parameter.name) }
    else { parameter.name.clone() }
}

fn error(function: Option<&str>, overload: Option<usize>, field: Option<&str>, kind: CatalogErrorKind) -> CatalogParseError {
    CatalogParseError { function: function.map(str::to_string), overload, field: field.map(str::to_string), kind }
}

fn format_location(function: Option<&str>, overload: Option<usize>, field: Option<&str>) -> String {
    let mut parts = Vec::new();
    if let Some(function) = function { parts.push(format!(" function `{function}`")); }
    if let Some(overload) = overload { parts.push(format!(" overload {overload}")); }
    if let Some(field) = field { parts.push(format!(" field `{field}`")); }
    parts.concat()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::catalog::ExampleVerification;
    use crate::markdown::compose::expression::catalog::expression_function_descriptors;
    use crate::markdown::schemas::{SimplifiedType, parse_yaml_schema, to_json_schema};

    const AUTHORED_CATALOG: &str =
        include_str!("../../../../../../docs/schemas/expression-functions.yaml");

    const SCHEMA: &str = r#"
kind: expression-function-catalog
$schema:
  kind: enum(expression-function-catalog; required)
  functions: "{ name: string(not-empty; required), category: string(not-empty; required), order: number(integer; required), description: string(not-empty; required), overloads: { parameters: { name: string(not-empty; required), type: string(not-empty; required), array: boolean, optional: boolean, variadic: boolean }[], returns: { type: string(not-empty; required), array: boolean, fallible: boolean }, example: { expression: string(not-empty; required), result: string(required), verification: enum(executable, display-only; required), reason: string(not-empty) } }[](min(1); required) }[](min(1); required)"
functions:
  - name: sample
    category: Test
    order: 1
    description: Exercises catalog shapes.
    overloads:
      - parameters:
          - { name: value, type: string }
          - { name: items, type: any, array: true }
          - { name: suffix, type: string, optional: true }
        returns: { type: string }
        example: { expression: 'sample("x", [])', result: x, verification: executable }
      - parameters:
          - { name: values, type: number, variadic: true }
        returns: { type: number, array: true, fallible: true }
        example: { expression: 'sample(1, 2)', result: '[1, 2]', verification: display-only, reason: illustrative }
  - name: later
    category: Test
    order: 2
    description: Preserves declaration order.
    overloads:
      - parameters: []
        returns: { type: boolean }
        example: { expression: later(), result: 'true', verification: executable }
"#;

    fn replace(source: &str, from: &str, to: &str) -> String {
        let output = source.replacen(from, to, 1);
        assert_ne!(output, source, "fixture replacement must match");
        output
    }

    fn kind(yaml: &str) -> CatalogErrorKind {
        parse_expression_function_catalog(yaml).unwrap_err().kind
    }

    #[test]
    fn parses_all_supported_shapes_and_preserves_order() {
        let catalog = parse_expression_function_catalog(SCHEMA).unwrap();
        assert_eq!(catalog.functions.iter().map(|function| function.name.as_str()).collect::<Vec<_>>(), ["sample", "later"]);
        let overloads = &catalog.functions[0].overloads;
        assert_eq!(overloads.len(), 2);
        assert_eq!(overloads[0].parameters[0].ty, DataType::String);
        assert!(overloads[0].parameters[1].array);
        assert!(overloads[0].parameters[2].optional);
        assert!(overloads[1].parameters[0].variadic);
        assert_eq!(overloads[0].returns, CatalogReturn { ty: DataType::String, array: false, fallible: false });
        assert_eq!(overloads[1].returns, CatalogReturn { ty: DataType::Number, array: true, fallible: true });
        assert!(matches!(overloads[0].example.verification, CatalogVerification::Executable));
        assert!(matches!(overloads[1].example.verification, CatalogVerification::DisplayOnly(ref reason) if reason == "illustrative"));
    }

    #[test]
    fn catalog_document_passes_its_simplified_schema() {
        let value: serde_yaml_ng::Value = serde_yaml_ng::from_str(SCHEMA).unwrap();
        let schema = parse_yaml_schema(value.get("$schema").unwrap()).unwrap();
        let json_schema = to_json_schema(&schema).unwrap();
        let validator = jsonschema::validator_for(&json_schema).unwrap();
        let instance = serde_json::to_value(value).unwrap();
        assert!(validator.is_valid(&instance));
    }

    #[test]
    fn authored_catalog_matches_registration_baseline() {
        let catalog = parse_expression_function_catalog(AUTHORED_CATALOG).unwrap();
        assert_eq!(catalog.functions.len(), 85);
        assert_eq!(catalog.functions.iter().map(|function| function.overloads.len()).sum::<usize>(), 88);

        let mut functions: Vec<_> = catalog.functions.iter().collect();
        functions.sort_by_key(|function| function.order);
        let authored: Vec<_> = functions.iter().flat_map(|function| {
            function.overloads.iter().enumerate().map(move |(index, overload)| (function, index, overload))
        }).collect();
        let descriptors = expression_function_descriptors();
        assert_eq!(authored.len(), descriptors.len());

        for ((function, overload_index, overload), descriptor) in authored.into_iter().zip(descriptors) {
            let signature = format!("{}({})", function.name, overload.parameters.iter()
                .map(render_parameter).collect::<Vec<_>>().join(", "));
            assert_eq!(signature, descriptor.signature);
            assert_eq!(function.category, descriptor.category);
            if overload_index == 0 {
                assert_eq!(function.description, descriptor.description);
            }
            assert_eq!(overload.parameters.iter().map(|parameter| (parameter.ty, parameter.array, parameter.optional, parameter.variadic)).collect::<Vec<_>>(),
                descriptor.parameters.iter().map(|parameter| (parameter.ty, parameter.array, parameter.optional, parameter.variadic)).collect::<Vec<_>>());
            assert_eq!((overload.returns.ty, overload.returns.array, overload.returns.fallible),
                (descriptor.returns.ty, descriptor.returns.array, descriptor.returns.fallible));

            match descriptor.example {
                Some(example) => {
                    assert_eq!(overload.example.expression, example.invocation);
                    assert_eq!(overload.example.result, example.result);
                    match (&overload.example.verification, example.verification) {
                        (CatalogVerification::Executable, ExampleVerification::Executable) => {}
                        (CatalogVerification::DisplayOnly(actual), ExampleVerification::DisplayOnly(expected)) => assert_eq!(actual, expected),
                        pair => panic!("verification mismatch for {}: {pair:?}", function.name),
                    }
                }
                None => {
                    assert_eq!(function.name, "has_command");
                    assert!(matches!(overload.example.verification, CatalogVerification::DisplayOnly(ref reason) if reason == "result is host-dependent"));
                }
            }
        }
    }

    #[test]
    fn authored_catalog_passes_its_simplified_schema() {
        let value: serde_yaml_ng::Value = serde_yaml_ng::from_str(AUTHORED_CATALOG).unwrap();
        let schema = parse_yaml_schema(value.get("$schema").unwrap()).unwrap();
        let validator = jsonschema::validator_for(&to_json_schema(&schema).unwrap()).unwrap();
        assert!(validator.is_valid(&serde_json::to_value(value).unwrap()));
    }

    #[test]
    fn production_parser_rejects_a_missing_schema_declaration() {
        // `$schema` is now a required Serde field, so a document omitting it
        // fails deserialization with `InvalidYaml`.
        let fixture = r#"
kind: expression-function-catalog
functions:
  - name: sample
    category: Test
    order: 1
    description: Exercises catalog shapes.
    overloads:
      - parameters: []
        returns: { type: string }
        example: { expression: 'sample()', result: x, verification: executable }
"#;
        assert!(matches!(kind(fixture), CatalogErrorKind::InvalidYaml(_)));
    }

    #[test]
    fn production_parser_rejects_a_malformed_schema_declaration() {
        // `$schema` is a bare scalar instead of a mapping, so `parse_yaml_schema`
        // rejects it with `SchemaDeclaration`.
        let fixture = r#"
kind: expression-function-catalog
$schema: not-a-mapping
functions:
  - name: sample
    category: Test
    order: 1
    description: Exercises catalog shapes.
    overloads:
      - parameters: []
        returns: { type: string }
        example: { expression: 'sample()', result: x, verification: executable }
"#;
        assert!(matches!(kind(fixture), CatalogErrorKind::SchemaDeclaration(_)));
    }

    #[test]
    fn production_parser_rejects_a_structurally_incomplete_instance() {
        // Tighten the declared functions-array arity from `min(1)` to `min(5)`
        // while the data still carries only two functions. The schema parses
        // and the document deserializes into `RawCatalog`, but the instance no
        // longer conforms — proving the production parser runs the validator.
        let fixture = replace(
            SCHEMA,
            "}[](min(1); required) }[](min(1); required)",
            "}[](min(1); required) }[](min(5); required)",
        );
        assert!(matches!(kind(&fixture), CatalogErrorKind::SchemaInstance(_)));
    }

    #[test]
    fn production_parser_validates_the_checked_in_authored_catalog() {
        // The real embedded catalog must pass the full production validation
        // path (structural + semantic), not just the independent schema check.
        assert!(parse_expression_function_catalog(AUTHORED_CATALOG).is_ok());
    }

    #[test]
    fn rejects_identity_order_and_value_errors() {
        // Structural validation runs before the defense-in-depth `kind` check:
        // the declared `enum(expression-function-catalog)` now rejects this.
        assert!(matches!(kind(&replace(SCHEMA, "kind: expression-function-catalog", "kind: other")), CatalogErrorKind::SchemaInstance(_)));
        // `$schema` is now a required Serde field, so a bare document without
        // one fails deserialization before any semantic check runs.
        assert!(matches!(kind("kind: expression-function-catalog\nfunctions: []\n"), CatalogErrorKind::InvalidYaml(_)));
        assert!(matches!(kind(&replace(SCHEMA, "name: sample", "name: Sample")), CatalogErrorKind::InvalidIdentifier(_)));
        assert!(matches!(kind(&replace(SCHEMA, "name: value", "name: Value")), CatalogErrorKind::InvalidIdentifier(_)));
        // `not-empty` compiles to `pattern: \S`, so whitespace-only/empty values
        // are rejected at structural validation before the semantic empty check.
        assert!(matches!(kind(&replace(SCHEMA, "category: Test", "category: '  '")), CatalogErrorKind::SchemaInstance(_)));
        assert!(matches!(kind(&replace(SCHEMA, "description: Exercises catalog shapes.", "description: ''")), CatalogErrorKind::SchemaInstance(_)));
        assert!(parse_expression_function_catalog(&replace(SCHEMA, "order: 1", "order: 0")).is_ok());
        assert!(matches!(kind(&replace(SCHEMA, "order: 1", "order: -1")), CatalogErrorKind::InvalidOrder));
        assert!(matches!(kind(&replace(SCHEMA, "order: 2", "order: 1")), CatalogErrorKind::DuplicateOrder(1)));
        assert!(matches!(kind(&replace(SCHEMA, "name: later", "name: sample")), CatalogErrorKind::DuplicateFunction(_)));
    }

    #[test]
    fn rejects_parameter_and_signature_errors() {
        assert!(matches!(kind(&replace(SCHEMA, "type: string }", "type: error }")), CatalogErrorKind::UnknownType(_)));
        assert!(matches!(kind(&replace(SCHEMA, "type: string }", "type: function }")), CatalogErrorKind::UnknownType(_)));
        assert!(matches!(kind(&replace(SCHEMA, "type: string }", "type: mystery }")), CatalogErrorKind::UnknownType(_)));
        assert!(matches!(kind(&replace(SCHEMA, "name: items", "name: value")), CatalogErrorKind::DuplicateParameter(_)));
        let required_after = replace(SCHEMA, "name: value, type: string", "name: value, type: string, optional: true");
        assert!(matches!(kind(&required_after), CatalogErrorKind::RequiredAfterOptional));
        let non_final = replace(SCHEMA, "name: value, type: string", "name: value, type: string, variadic: true");
        assert!(matches!(kind(&non_final), CatalogErrorKind::NonFinalVariadic));
        let optional_variadic = replace(SCHEMA, "name: values, type: number, variadic: true", "name: values, type: number, variadic: true, optional: true");
        assert!(matches!(kind(&optional_variadic), CatalogErrorKind::OptionalVariadic));

        let duplicate = replace(SCHEMA, "      - parameters:\n          - { name: values, type: number, variadic: true }", "      - parameters:\n          - { name: value, type: string }\n          - { name: items, type: any, array: true }\n          - { name: suffix, type: string, optional: true }");
        assert!(matches!(kind(&duplicate), CatalogErrorKind::DuplicateSignature(_)));
    }

    #[test]
    fn rejects_return_example_and_unknown_field_errors_without_panicking() {
        assert!(matches!(kind(&replace(SCHEMA, "returns: { type: string }", "returns: { type: error }")), CatalogErrorKind::UnknownType(_)));
        assert!(matches!(kind(&replace(SCHEMA, "returns: { type: string }", "returns: { type: 'string | error' }")), CatalogErrorKind::UnknownType(_)));
        assert!(matches!(kind(&replace(SCHEMA, "returns: { type: string }", "returns: { type: 'string | number' }")), CatalogErrorKind::UnknownType(_)));
        assert!(matches!(kind(&replace(SCHEMA, "verification: display-only, reason: illustrative", "verification: display-only")), CatalogErrorKind::MissingDisplayReason));
        assert!(matches!(kind(&replace(SCHEMA, "verification: executable }", "verification: executable, reason: no }")), CatalogErrorKind::ExecutableReason));
        assert!(matches!(kind(&replace(SCHEMA, "        example: { expression: 'sample(\"x\", [])', result: x, verification: executable }", "")), CatalogErrorKind::InvalidYaml(_)));
        assert!(matches!(kind(&replace(SCHEMA, "category: Test", "category: Test\n    surprise: true")), CatalogErrorKind::InvalidYaml(_)));
        assert!(matches!(kind(&replace(SCHEMA, "      - parameters:", "      - surprise: true\n        parameters:")), CatalogErrorKind::InvalidYaml(_)));
        assert!(matches!(kind(&replace(SCHEMA, "name: value, type: string", "name: value, type: string, surprise: true")), CatalogErrorKind::InvalidYaml(_)));
        assert!(matches!(kind(&replace(SCHEMA, "returns: { type: string }", "returns: { type: string, surprise: true }")), CatalogErrorKind::InvalidYaml(_)));
        assert!(matches!(kind(&replace(SCHEMA, "result: x, verification", "result: x, surprise: true, verification")), CatalogErrorKind::InvalidYaml(_)));
        assert!(matches!(kind(&replace(SCHEMA, "functions:", "surprise: true\nfunctions:")), CatalogErrorKind::InvalidYaml(_)));
        for malformed in ["", "kind: [", "kind: expression-function-catalog\nfunctions: nope"] {
            assert!(std::panic::catch_unwind(|| parse_expression_function_catalog(malformed)).is_ok());
        }
    }

    #[test]
    fn type_keyword_domains_remain_separate() {
        assert_eq!(DataType::from_keyword("number(integer)"), Some(DataType::Integer));
        assert_eq!(DataType::from_keyword("any"), Some(DataType::Any));
        assert_eq!(SimplifiedType::from_keyword("error"), None);
        assert_eq!(SimplifiedType::from_keyword("function"), None);
    }

    #[test]
    fn semantic_errors_carry_available_catalog_location() {
        let error = parse_expression_function_catalog(&replace(SCHEMA, "type: string }", "type: error }")).unwrap_err();
        assert_eq!(error.function.as_deref(), Some("sample"));
        assert_eq!(error.overload, Some(0));
        assert_eq!(error.field.as_deref(), Some("parameters.type"));
    }
}
