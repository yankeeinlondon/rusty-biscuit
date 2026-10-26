#![cfg(feature = "openapi")]
//! Import of path-item parameters, parameter `$ref`s, typed path parameters,
//! and the standard `deprecated` flags.

use schematic_define::Endpoint;
use schematic_define::openapi::{
    DiagnosticSeverity, ExportFormat, ExportOptions, OpenApiImport, OpenApiImportResult,
    OpenApiSource, export, serialize,
};
use schematic_define::params::{ParamDef, ParamStyle, QueryParamType};

fn import(yaml: &str) -> OpenApiImportResult {
    OpenApiImport::new(OpenApiSource::yaml(yaml)).build().expect("import succeeds")
}

fn endpoint<'a>(result: &'a OpenApiImportResult, id: &str) -> &'a Endpoint {
    result.api.endpoints.iter().find(|endpoint| endpoint.id == id).expect("endpoint imported")
}

fn names(params: &[ParamDef]) -> Vec<&str> {
    params.iter().map(|param| param.name.as_str()).collect()
}

fn errors(result: &OpenApiImportResult) -> Vec<(&str, &str)> {
    result
        .diagnostics
        .iter()
        .filter(|diagnostic| diagnostic.severity == DiagnosticSeverity::Error)
        .map(|diagnostic| (diagnostic.location.as_str(), diagnostic.message.as_str()))
        .collect()
}

#[test]
fn path_item_parameter_is_inherited() {
    let result = import(
        r#"
openapi: "3.0.0"
info: { title: Test, version: "1" }
servers: [{ url: "https://api.test" }]
paths:
  /items:
    parameters:
      - { name: limit, in: query, schema: { type: integer } }
    get:
      operationId: listItems
      responses: { '204': { description: none } }
"#,
    );

    let params = endpoint(&result, "ListItems").params.as_ref().expect("params");
    assert_eq!(names(&params.query), ["limit"]);
    assert_eq!(params.query[0].param_type, QueryParamType::Integer);
    assert!(result.diagnostics.is_empty(), "{:?}", result.diagnostics);
}

#[test]
fn operation_parameter_overrides_inherited_in_place() {
    let result = import(
        r#"
openapi: "3.0.0"
info: { title: Test, version: "1" }
servers: [{ url: "https://api.test" }]
paths:
  /items:
    parameters:
      - { name: limit, in: query, schema: { type: string } }
      - { name: cursor, in: query, schema: { type: string } }
    get:
      operationId: listItems
      parameters:
        - { name: tag, in: query, schema: { type: string } }
        - { name: limit, in: query, required: true, schema: { type: integer } }
      responses: { '204': { description: none } }
"#,
    );

    let query = &endpoint(&result, "ListItems").params.as_ref().expect("params").query;
    assert_eq!(names(query), ["limit", "cursor", "tag"]);
    assert_eq!(query[0].param_type, QueryParamType::Integer);
    assert!(query[0].required);
}

#[test]
fn header_override_compares_names_case_insensitively() {
    let result = import(
        r#"
openapi: "3.0.0"
info: { title: Test, version: "1" }
servers: [{ url: "https://api.test" }]
paths:
  /items:
    parameters:
      - { name: X-Trace, in: header, description: inherited, schema: { type: string } }
    get:
      operationId: listItems
      parameters:
        - { name: x-trace, in: header, description: operation, schema: { type: string } }
      responses: { '204': { description: none } }
"#,
    );

    let header = &endpoint(&result, "ListItems").params.as_ref().expect("params").header;
    assert_eq!(names(header), ["x-trace"]);
    assert_eq!(header[0].description.as_deref(), Some("operation"));
}

#[test]
fn same_name_in_different_locations_is_kept_twice() {
    let result = import(
        r#"
openapi: "3.0.0"
info: { title: Test, version: "1" }
servers: [{ url: "https://api.test" }]
paths:
  /items:
    parameters:
      - { name: token, in: query, schema: { type: string } }
    get:
      operationId: listItems
      parameters:
        - { name: token, in: header, schema: { type: string } }
        - { name: Token, in: query, schema: { type: string } }
      responses: { '204': { description: none } }
"#,
    );

    let params = endpoint(&result, "ListItems").params.as_ref().expect("params");
    assert_eq!(names(&params.query), ["token", "Token"]);
    assert_eq!(names(&params.header), ["token"]);
}

#[test]
fn local_parameter_refs_resolve_in_path_item_and_operation() {
    let result = import(
        r#"
openapi: "3.0.0"
info: { title: Test, version: "1" }
servers: [{ url: "https://api.test" }]
paths:
  /items:
    parameters:
      - $ref: '#/components/parameters/Limit'
    get:
      operationId: listItems
      parameters:
        - $ref: '#/components/parameters/StrictLimit'
        - $ref: '#/components/parameters/Trace'
      responses: { '204': { description: none } }
components:
  parameters:
    Limit: { name: limit, in: query, schema: { type: string } }
    StrictLimit: { name: limit, in: query, required: true, schema: { type: integer } }
    Trace: { name: X-Trace, in: header, schema: { type: string } }
"#,
    );

    let params = endpoint(&result, "ListItems").params.as_ref().expect("params");
    assert_eq!(names(&params.query), ["limit"]);
    assert_eq!(params.query[0].param_type, QueryParamType::Integer);
    assert_eq!(names(&params.header), ["X-Trace"]);
    assert!(result.diagnostics.is_empty(), "{:?}", result.diagnostics);
}

#[test]
fn unresolved_local_refs_are_errors_at_their_pointers() {
    let result = import(
        r#"
openapi: "3.0.0"
info: { title: Test, version: "1" }
servers: [{ url: "https://api.test" }]
paths:
  /a~b/{id}:
    parameters:
      - { name: keep, in: query, schema: { type: string } }
      - $ref: '#/components/parameters/MissingShared'
    get:
      operationId: getItem
      parameters:
        - $ref: '#/components/parameters/MissingOwn'
      responses: { '204': { description: none } }
"#,
    );

    let errors = errors(&result);
    assert_eq!(errors.len(), 2, "{errors:?}");
    assert_eq!(errors[0].0, "#/paths/~1a~0b~1{id}/parameters/1");
    assert!(errors[0].1.contains("'#/components/parameters/MissingShared'"), "{}", errors[0].1);
    assert_eq!(errors[1].0, "#/paths/~1a~0b~1{id}/get/parameters/0");
    assert!(errors[1].1.contains("'#/components/parameters/MissingOwn'"), "{}", errors[1].1);
    // The unresolved parameters are skipped; the rest still import.
    assert_eq!(names(&endpoint(&result, "GetItem").params.as_ref().expect("params").query), ["keep"]);
}

#[test]
fn remote_parameter_ref_is_an_error_and_never_fetched() {
    let result = import(
        r#"
openapi: "3.0.0"
info: { title: Test, version: "1" }
servers: [{ url: "https://api.test" }]
paths:
  /items:
    get:
      operationId: listItems
      parameters:
        - $ref: 'https://example.invalid/params.yaml#/Limit'
      responses: { '204': { description: none } }
"#,
    );

    let errors = errors(&result);
    assert_eq!(errors.len(), 1, "{errors:?}");
    assert_eq!(errors[0].0, "#/paths/~1items/get/parameters/0");
    assert!(errors[0].1.contains("Remote parameter reference 'https://example.invalid/params.yaml#/Limit'"));
    assert!(endpoint(&result, "ListItems").params.is_none());
}

const TYPED_PATH_SPEC: &str = r#"
openapi: "3.0.0"
info: { title: Test, version: "1" }
servers: [{ url: "https://api.test" }]
paths:
  /orgs/{org}/items/{item_id}:
    parameters:
      - { name: item_id, in: path, required: true, description: Item number, schema: { type: integer } }
      - { name: org, in: path, required: true, schema: { type: string } }
    get:
      operationId: getItem
      responses: { '204': { description: none } }
    delete:
      operationId: deleteItem
      parameters:
        - { name: item_id, in: path, required: true, description: Item to delete, deprecated: true, schema: { type: integer } }
      responses: { '204': { description: none } }
"#;

#[test]
fn typed_path_parameters_import_in_placeholder_order() {
    let result = import(TYPED_PATH_SPEC);

    let get = &endpoint(&result, "GetItem").params.as_ref().expect("params").path;
    assert_eq!(get, &[
        ParamDef::path("org", QueryParamType::String),
        ParamDef::path("item_id", QueryParamType::Integer).describes("Item number"),
    ]);
    assert_eq!(get[0].style, ParamStyle::Simple);
    let delete = &endpoint(&result, "DeleteItem").params.as_ref().expect("params").path;
    assert_eq!(delete[1], ParamDef::path("item_id", QueryParamType::Integer).describes("Item to delete").deprecated());
    assert!(result.diagnostics.is_empty(), "{:?}", result.diagnostics);
}

#[test]
fn typed_path_parameters_round_trip_through_export() {
    let options = ExportOptions::new().with_version("1");
    let first = import(TYPED_PATH_SPEC);
    let first_yaml = serialize(&export(&first.api, &options).unwrap(), ExportFormat::Yaml).unwrap();

    let second = import(&first_yaml);
    let second_yaml = serialize(&export(&second.api, &options).unwrap(), ExportFormat::Yaml).unwrap();

    assert_eq!(second_yaml, first_yaml);
    for id in ["GetItem", "DeleteItem"] {
        assert_eq!(endpoint(&second, id).params, endpoint(&first, id).params, "{id}");
    }
}

#[test]
fn plain_string_path_parameters_stay_untyped() {
    let result = import(
        r#"
openapi: "3.0.0"
info: { title: Test, version: "1" }
servers: [{ url: "https://api.test" }]
paths:
  /items/{item_id}:
    parameters:
      - { name: item_id, in: path, required: true, schema: { type: string } }
    get:
      operationId: getItem
      responses: { '204': { description: none } }
"#,
    );

    assert!(endpoint(&result, "GetItem").params.is_none());
}

#[test]
fn deprecated_parameter_and_operation_are_read() {
    let result = import(
        r#"
openapi: "3.0.0"
info: { title: Test, version: "1" }
servers: [{ url: "https://api.test" }]
paths:
  /items:
    get:
      operationId: listItems
      deprecated: true
      parameters:
        - { name: page, in: query, deprecated: true, schema: { type: integer } }
        - { name: limit, in: query, schema: { type: integer } }
      responses: { '204': { description: none } }
    post:
      operationId: createItem
      responses: { '204': { description: none } }
"#,
    );

    let list = endpoint(&result, "ListItems");
    assert!(list.deprecated);
    let query = &list.params.as_ref().expect("params").query;
    assert!(query[0].deprecated);
    assert!(!query[1].deprecated);
    assert!(!endpoint(&result, "CreateItem").deprecated);
}
