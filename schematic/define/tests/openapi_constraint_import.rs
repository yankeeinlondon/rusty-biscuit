#![cfg(feature = "openapi")]
//! Import of value constraints from OpenAPI documents: parameter, body-root,
//! and form-field rules, component rules kept in stored schemas, pattern
//! admission, rules a definition cannot carry, and the loader input matrix.

use schematic_define::constraints::Constraints;
use schematic_define::openapi::{DiagnosticSeverity, OpenApiDiagnostic, OpenApiError, OpenApiImport, OpenApiImportResult, OpenApiSource};
use schematic_define::params::{ParamDef, QueryParamType};
use schematic_define::{ApiRequest, Endpoint, FormField};
use serde_json::{Value, json};

fn import(source: OpenApiSource) -> OpenApiImportResult {
    OpenApiImport::new(source).build().expect("import succeeds")
}

fn import_yaml(yaml: &str) -> OpenApiImportResult {
    import(OpenApiSource::yaml(yaml))
}

fn endpoint<'a>(result: &'a OpenApiImportResult, id: &str) -> &'a Endpoint {
    result.api.endpoints.iter().find(|endpoint| endpoint.id == id).expect("endpoint imported")
}

fn query<'a>(endpoint: &'a Endpoint, name: &str) -> &'a ParamDef {
    endpoint.params.as_ref().expect("params").query.iter().find(|param| param.name == name).expect("query parameter")
}

fn rules(param: &ParamDef) -> Constraints {
    *param.constraints.clone().expect("constraints")
}

fn warnings(result: &OpenApiImportResult) -> Vec<&OpenApiDiagnostic> {
    result.diagnostics.iter().filter(|diagnostic| diagnostic.severity == DiagnosticSeverity::Warn).collect()
}

fn component<'a>(result: &'a OpenApiImportResult, name: &str) -> &'a Value {
    &result.api.schemas.iter().find(|component| component.name == name).expect("component stored").schema
}

/// One `GET /items` operation with the given YAML parameter list (indented
/// for `parameters:`).
fn with_query_schema(schema: &str) -> String {
    format!(
        r#"
openapi: "3.0.3"
info: {{ title: Test, version: "1" }}
servers: [{{ url: "https://api.test" }}]
paths:
  /items:
    get:
      operationId: listItems
      parameters:
        - name: value
          in: query
          schema: {schema}
      responses: {{ '204': {{ description: none }} }}
"#
    )
}

const PARAMETERS: &str = r#"
openapi: "3.0.3"
info: { title: Test, version: "1" }
servers: [{ url: "https://api.test" }]
paths:
  /items/{id}:
    get:
      operationId: getItem
      parameters:
        - name: id
          in: path
          required: true
          schema: { type: string, minLength: 3, maxLength: 12, pattern: "^[a-z0-9]+$" }
        - name: when
          in: query
          schema: { type: string, format: date-time }
        - name: score
          in: query
          schema: { type: number, minimum: 0, exclusiveMinimum: true, maximum: 10, multipleOf: 0.1 }
        - name: page
          in: query
          schema: { type: integer, minimum: 1, maximum: 100, exclusiveMaximum: true }
        - name: level
          in: query
          schema: { type: integer, enum: [1, 2, 3] }
        - name: sort
          in: query
          schema: { type: string, enum: [asc, desc] }
        - name: tags
          in: query
          schema:
            type: array
            minItems: 1
            maxItems: 5
            uniqueItems: true
            items: { type: string, minLength: 2, maxLength: 8 }
        - name: x-trace
          in: header
          schema: { type: string, maxLength: 64 }
        - name: session
          in: cookie
          schema: { type: string, minLength: 16 }
        - $ref: '#/components/parameters/Limit'
      responses: { '204': { description: none } }
components:
  parameters:
    Limit:
      name: limit
      in: query
      schema: { type: integer, minimum: 1, maximum: 50 }
"#;

#[test]
fn parameter_keywords_import_as_constraints() {
    let result = import_yaml(PARAMETERS);
    let get = endpoint(&result, "GetItem");
    let params = get.params.as_ref().expect("params");

    // A string path parameter is kept as typed because it is constrained.
    assert_eq!(params.path, vec![ParamDef::path("id", QueryParamType::String)
        .constraints(Constraints::new().min_length(3).max_length(12).pattern("^[a-z0-9]+$"))]);
    assert_eq!(rules(query(get, "when")), Constraints::new().format("date-time"));
    // A 3.0 Boolean exclusive flag becomes the numeric bound.
    assert_eq!(rules(query(get, "score")), Constraints::new().exclusive_minimum(0).maximum(10).multiple_of("0.1"));
    assert_eq!(rules(query(get, "page")), Constraints::new().minimum(1).exclusive_maximum(100));
    assert_eq!(rules(query(get, "level")), Constraints::new().allowed_values([1, 2, 3]));
    // The enum type already lists the values, so no rule repeats them.
    assert_eq!(query(get, "sort").param_type, QueryParamType::Enum(vec!["asc".into(), "desc".into()]));
    assert_eq!(query(get, "sort").constraints, None);
    assert_eq!(
        rules(query(get, "tags")),
        Constraints::new().min_items(1).max_items(5).unique_items().items(Constraints::new().min_length(2).max_length(8))
    );
    assert_eq!(rules(&params.header[0]), Constraints::new().max_length(64));
    assert_eq!(rules(&params.cookie[0]), Constraints::new().min_length(16));
    // A `$ref` parameter's rules come from the component parameter.
    assert_eq!(rules(query(get, "limit")), Constraints::new().minimum(1).maximum(50));
    assert!(warnings(&result).is_empty(), "{:?}", warnings(&result));
}

#[test]
fn unconstrained_parameters_carry_no_constraints() {
    let result = import_yaml(&with_query_schema("{ type: integer }"));
    let get = endpoint(&result, "ListItems");
    assert_eq!(query(get, "value").constraints, None);
}

#[test]
fn parameter_schema_reference_carries_no_constraints() {
    let result = import_yaml(&format!(
        "{}{}",
        with_query_schema("{ $ref: '#/components/schemas/Code' }"),
        "components:\n  schemas:\n    Code: { type: string, maxLength: 4 }\n"
    ));
    assert_eq!(query(endpoint(&result, "ListItems"), "value").constraints, None);
    assert_eq!(component(&result, "Code")["maxLength"], json!(4));
}

const EXACT_YAML: &str = r#"
openapi: "3.0.3"
info: { title: Test, version: "1" }
servers: [{ url: "https://api.test" }]
paths:
  /items:
    get:
      operationId: listItems
      parameters:
        - name: big
          in: query
          schema:
            type: integer
            maximum: 18446744073709551616
        - name: fine
          in: query
          schema: { type: number, multipleOf: 0.10000000000000000001 }
        - name: seed
          in: query
          schema:
            type: integer
            minimum: -9223372036854776000
            maximum: 9223372036854776000
      responses: { '204': { description: none } }
"#;

const EXACT_JSON: &str = r#"{
  "openapi": "3.0.3",
  "info": { "title": "Test", "version": "1" },
  "servers": [{ "url": "https://api.test" }],
  "paths": { "/items": { "get": {
    "operationId": "listItems",
    "parameters": [
      { "name": "big", "in": "query", "schema": { "type": "integer", "maximum": 18446744073709551616 } },
      { "name": "fine", "in": "query", "schema": { "type": "number", "multipleOf": 0.10000000000000000001 } }
    ],
    "responses": { "204": { "description": "none" } }
  } } }
}"#;

#[test]
fn exact_parameter_bounds_survive_yaml_and_json() {
    for result in [import(OpenApiSource::yaml(EXACT_YAML)), import(OpenApiSource::json(EXACT_JSON))] {
        let get = endpoint(&result, "ListItems");
        assert_eq!(rules(query(get, "big")).maximum.expect("maximum").as_str(), "18446744073709551616");
        assert_eq!(rules(query(get, "fine")).multiple_of.expect("multipleOf").as_str(), "0.10000000000000000001");
    }
}

/// The seed bounds lie just outside `i64`, so they need the parse stand-in.
/// An `f64` holds their decimal value exactly, so the restored schema keeps
/// them as that `f64` and the rule's text is its shortest spelling
/// (`-9.223372036854776e+18`): the same decimal, not the source spelling.
#[test]
fn openai_style_seed_bounds_parse_from_yaml() {
    let result = import_yaml(EXACT_YAML);
    let seed = rules(query(endpoint(&result, "ListItems"), "seed"));
    let minimum = seed.minimum.expect("minimum");
    let maximum = seed.maximum.expect("maximum");
    assert_eq!(minimum.as_str().parse::<f64>(), Ok(-9223372036854776000.0), "{minimum}");
    assert_eq!(maximum.as_str().parse::<f64>(), Ok(9223372036854776000.0), "{maximum}");
}

const BODIES: &str = r#"
openapi: "3.0.3"
info: { title: Test, version: "1" }
servers: [{ url: "https://api.test" }]
paths:
  /notes:
    post:
      operationId: postNote
      requestBody:
        content:
          text/plain:
            schema: { type: string, minLength: 1, maxLength: 280 }
      responses: { '204': { description: none } }
  /ids:
    post:
      operationId: postIds
      requestBody:
        content:
          application/json:
            schema:
              type: array
              maxItems: 3
              items: { type: integer, minimum: 1 }
      responses: { '204': { description: none } }
  /widgets:
    post:
      operationId: postWidget
      requestBody:
        content:
          application/json:
            schema: { $ref: '#/components/schemas/Widget' }
      responses: { '204': { description: none } }
components:
  schemas:
    Widget:
      type: object
      properties:
        name: { type: string, maxLength: 10 }
"#;

#[test]
fn body_root_rules_import_for_inline_bodies_only() {
    let result = import_yaml(BODIES);

    let note = endpoint(&result, "PostNote");
    assert_eq!(note.request, Some(ApiRequest::text("text/plain")));
    assert_eq!(note.request_constraints.as_deref(), Some(&Constraints::new().min_length(1).max_length(280)));

    let ids = endpoint(&result, "PostIds");
    assert_eq!(ids.request_constraints.as_deref(), Some(&Constraints::new().max_items(3).items(Constraints::new().minimum(1))));

    // A component body keeps its rules in the stored component.
    let widget = endpoint(&result, "PostWidget");
    assert_eq!(widget.request_constraints, None);
    assert_eq!(component(&result, "Widget")["properties"]["name"]["maxLength"], json!(10));
    assert!(warnings(&result).is_empty(), "{:?}", warnings(&result));
}

const FORMS: &str = r#"
openapi: "3.0.3"
info: { title: Test, version: "1" }
servers: [{ url: "https://api.test" }]
paths:
  /posts:
    post:
      operationId: createPost
      requestBody:
        content:
          application/x-www-form-urlencoded:
            schema:
              type: object
              required: [title]
              properties:
                title: { type: string, maxLength: 80 }
                count: { type: integer, maximum: 9 }
      responses: { '204': { description: none } }
  /uploads:
    post:
      operationId: upload
      requestBody:
        content:
          multipart/form-data:
            schema:
              type: object
              properties:
                caption: { type: string, minLength: 1 }
                file: { type: string, format: binary, maxLength: 1000 }
      responses: { '204': { description: none } }
"#;

#[test]
fn form_field_rules_import_and_unrepresentable_ones_are_dropped() {
    let result = import_yaml(FORMS);

    let Some(ApiRequest::UrlEncoded { fields }) = &endpoint(&result, "CreatePost").request else { panic!("url-encoded body") };
    assert_eq!(fields[0], FormField::text("title").constraints(Constraints::new().max_length(80)));
    // A URL-encoded field is text, so a numeric rule cannot apply to it.
    assert_eq!(fields[1], FormField::text("count").optional());
    let dropped: Vec<_> = warnings(&result).into_iter().filter(|warning| warning.message.contains("form.count")).collect();
    assert_eq!(dropped.len(), 1, "{:?}", result.diagnostics);
    assert_eq!(dropped[0].location, "#/paths/~1posts/post");
    assert!(dropped[0].message.contains("maximum") && dropped[0].message.contains("not preserved"), "{}", dropped[0].message);

    let Some(ApiRequest::FormData { fields }) = &endpoint(&result, "Upload").request else { panic!("multipart body") };
    assert_eq!(fields[0], FormField::text("caption").optional().constraints(Constraints::new().min_length(1)));
    assert_eq!(fields[1].name, "file");
    assert_eq!(fields[1].constraints, None);
}

const COMPONENTS: &str = r#"
openapi: "3.0.3"
info: { title: Test, version: "1" }
servers: [{ url: "https://api.test" }]
paths:
  /a:
    post:
      operationId: postA
      requestBody:
        content:
          application/json:
            schema: { $ref: '#/components/schemas/Shared' }
      responses: { '204': { description: none } }
  /b:
    put:
      operationId: putB
      requestBody:
        content:
          application/json:
            schema: { $ref: '#/components/schemas/Shared' }
      responses:
        '200':
          description: ok
          content:
            application/json:
              schema: { $ref: '#/components/schemas/Receipt' }
components:
  schemas:
    Shared:
      type: object
      properties:
        big:
          type: integer
          maximum: 18446744073709551616
        ratio: { type: number, minimum: 0, exclusiveMinimum: true }
        version: { type: string, const: v2 }
        note: { type: string, nullable: true, maxLength: 5 }
    Receipt:
      type: object
      properties:
        code: { type: string, pattern: "^[A-Z]{3}$" }
"#;

#[test]
fn component_rules_are_kept_in_stored_schemas() {
    let result = import_yaml(COMPONENTS);
    let shared = component(&result, "Shared");
    let properties = &shared["properties"];

    assert_eq!(properties["big"]["x-schematic-exact"], json!({"maximum": "18446744073709551616"}));
    assert_eq!(properties["ratio"]["exclusiveMinimum"], json!(0));
    assert_eq!(properties["ratio"].get("minimum"), None);
    assert_eq!(properties["version"]["const"], json!("v2"));
    assert_eq!(properties["note"]["maxLength"], json!(5));
    assert_eq!(properties["note"]["nullable"], json!(true));

    // Two endpoints share it; it is stored once.
    assert_eq!(result.api.schemas.iter().filter(|component| component.name == "Shared").count(), 1);
    // A response-only component keeps its rules too.
    assert_eq!(component(&result, "Receipt")["properties"]["code"]["pattern"], json!("^[A-Z]{3}$"));
}

#[test]
fn admitted_pattern_draws_no_diagnostic() {
    let result = import_yaml(&with_query_schema(r#"{ type: string, pattern: "^\\d{3}$" }"#));
    let value = rules(query(endpoint(&result, "ListItems"), "value"));
    assert_eq!(value, Constraints::new().pattern(r"^\d{3}$"));
    assert!(warnings(&result).is_empty(), "{:?}", warnings(&result));
}

#[test]
fn unsupported_pattern_is_kept_unenforced_with_a_warning() {
    let result = import_yaml(&with_query_schema(r#"{ type: string, pattern: "^\\p{Lu}" }"#));
    let value = rules(query(endpoint(&result, "ListItems"), "value"));
    assert_eq!(value, Constraints::new().unenforced_pattern(r"^\p{Lu}"));

    let warnings = warnings(&result);
    assert_eq!(warnings.len(), 1, "{warnings:?}");
    assert_eq!(warnings[0].location, "#/paths/~1items/get/parameters/0/schema/pattern");
    assert!(warnings[0].message.contains("subset") && warnings[0].message.contains("not enforced"), "{}", warnings[0].message);
}

/// The import error of `source`, as `(path, message)`.
fn rejected(source: OpenApiSource) -> (String, String) {
    match OpenApiImport::new(source).build() {
        Err(OpenApiError::Validation { path, message }) => (path, message),
        other => panic!("expected a located validation error, got {other:?}"),
    }
}

#[test]
fn invalid_pattern_fails_the_import_at_its_pointer() {
    for source in ["(", "[a"] {
        let schema = format!(r#"{{ type: array, items: {{ type: string, pattern: "{source}" }} }}"#);
        let (path, message) = rejected(OpenApiSource::yaml(with_query_schema(&schema)));
        assert_eq!(path, "#/paths/~1items/get/parameters/0/schema/items/pattern", "{source}");
        assert!(message.contains("not a valid regular expression"), "{message}");
    }
}

#[test]
fn contradictory_source_bounds_fail_the_import_at_their_pointer() {
    let (path, message) = rejected(OpenApiSource::yaml(with_query_schema("{ type: integer, minimum: 10, maximum: 1 }")));
    assert_eq!(path, "#/paths/~1items/get/parameters/0/schema/minimum");
    assert!(message.contains("leaves no number"), "{message}");
}

const PRESERVATION: &str = include_str!("fixtures/constraint_preservation.yaml");

/// The real fixture's `tags` items with `minLength` raised above their
/// `maxLength`: the import fails at that keyword in either mode, rather than
/// succeeding without `tags`' valid count, uniqueness, and pattern rules.
#[test]
fn contradictory_item_lengths_fail_the_import_in_either_mode() {
    let valid = "              minLength: 2\n              maxLength: 12\n";
    assert!(PRESERVATION.contains(valid));
    let contradictory = PRESERVATION.replace(valid, "              minLength: 20\n              maxLength: 12\n");

    let (path, message) = rejected(OpenApiSource::yaml(contradictory.clone()));
    assert_eq!(path, "#/paths/~1items/get/parameters/3/schema/items/minLength");
    assert!(message.contains("20 is above the maximum 12"), "{message}");
    let strict = OpenApiImport::new(OpenApiSource::yaml(contradictory)).strict().build();
    assert!(matches!(&strict, Err(OpenApiError::Validation { path: located, .. }) if *located == path), "{strict:?}");

    // Control: the unedited fixture keeps every `tags` rule.
    let result = import_yaml(PRESERVATION);
    let tags = rules(query(endpoint(&result, "ListItems"), "tags"));
    assert_eq!(tags, Constraints::new().min_items(1).max_items(5).unique_items()
        .items(Constraints::new().min_length(2).max_length(12).pattern("^[a-z][a-z0-9-]*$")));
}

/// Contradictory rules fail the same way wherever the importer reads them.
#[test]
fn contradictory_rules_fail_the_import_at_every_input_and_component() {
    let cases = [
        ("header", "      parameters:\n        - { name: X-Trace, in: header, schema: { type: string, minLength: 37, maxLength: 36 } }\n",
            "#/paths/~1items/get/parameters/0/schema/minLength"),
        ("path", "      parameters:\n        - { name: id, in: path, required: true, schema: { type: string, minLength: 9, maxLength: 1 } }\n",
            "#/paths/~1items/get/parameters/0/schema/minLength"),
        ("cookie", "      parameters:\n        - { name: session, in: cookie, schema: { type: number, multipleOf: 0 } }\n",
            "#/paths/~1items/get/parameters/0/schema/multipleOf"),
    ];
    for (location, parameters, pointer) in cases {
        let path = if location == "path" { "/items/{id}" } else { "/items" };
        let document = format!("openapi: \"3.0.3\"\ninfo: {{ title: T, version: \"1\" }}\npaths:\n  {path}:\n    get:\n      operationId: listItems\n{parameters}      responses: {{ '204': {{ description: none }} }}\n");
        let pointer = if location == "path" { pointer.replace("~1items/", "~1items~1{id}/") } else { pointer.to_owned() };
        assert_eq!(rejected(OpenApiSource::yaml(document)).0, pointer, "{location}");
    }

    let edits = [
        (BODIES, "{ type: string, minLength: 1, maxLength: 280 }", "{ type: string, minLength: 300, maxLength: 280 }",
            "#/paths/~1notes/post/requestBody/content/text~1plain/schema/minLength"),
        (BODIES, "name: { type: string, maxLength: 10 }", "name: { type: string, minLength: 11, maxLength: 10 }",
            "#/components/schemas/Widget/properties/name/minLength"),
        (FORMS, "title: { type: string, maxLength: 80 }", "title: { type: string, minLength: 81, maxLength: 80 }",
            "#/paths/~1posts/post/requestBody/content/application~1x-www-form-urlencoded/schema/properties/title/minLength"),
    ];
    for (document, valid, contradictory, pointer) in edits {
        assert!(document.contains(valid), "{valid}");
        assert_eq!(rejected(OpenApiSource::yaml(document.replace(valid, contradictory))).0, pointer, "{contradictory}");
    }
}

/// A rule that does not fit its input's type is removed alone, with a
/// warning; its valid siblings stay.
#[test]
fn an_inapplicable_rule_is_removed_without_its_siblings() {
    let valid = "count: { type: integer, maximum: 9 }";
    let result = import_yaml(&FORMS.replace(valid, "count: { type: integer, maximum: 9, minLength: 1 }"));
    let Some(ApiRequest::UrlEncoded { fields }) = &endpoint(&result, "CreatePost").request else { panic!("url-encoded body") };
    assert_eq!(fields[1], FormField::text("count").optional().constraints(Constraints::new().min_length(1)));
    let dropped: Vec<_> = warnings(&result).into_iter().filter(|warning| warning.message.contains("form.count")).collect();
    assert_eq!(dropped.len(), 1, "{:?}", result.diagnostics);
    assert!(dropped[0].message.contains("`maximum`"), "{}", dropped[0].message);
}

/// An `items` list (positional items) is not an OpenAPI 3.0 schema, so the
/// document does not parse: its rules are never silently dropped.
#[test]
fn positional_items_on_a_parameter_are_a_parse_error() {
    let outcome = OpenApiImport::new(OpenApiSource::yaml(with_query_schema("{ type: array, items: [{ type: string, maxLength: 3 }] }"))).build();
    assert!(matches!(outcome, Err(OpenApiError::Parse { .. })), "{outcome:?}");
}

/// JSON Schema keywords OpenAPI 3.0 lacks are dropped by the parser; each
/// one under a parameter schema is still reported where it was written.
#[test]
fn dropped_subschema_keywords_on_a_parameter_are_reported_not_preserved() {
    let schema = "{ type: array, items: { type: string, if: { maxLength: 1 }, then: { minLength: 1 }, else: { maxLength: 9 } }, \
        prefixItems: [{ maxLength: 3 }], additionalItems: { maxLength: 3 }, contains: { maxLength: 3 }, \
        unevaluatedItems: { maxLength: 3 }, unevaluatedProperties: false, dependentSchemas: { a: { maxLength: 3 } }, \
        patternProperties: { a: { maxLength: 3 } }, propertyNames: { maxLength: 3 }, dependencies: { a: [b] } }";
    let result = import_yaml(&with_query_schema(schema));
    let root = "#/paths/~1items/get/parameters/0/schema";
    let mut located: Vec<String> = warnings(&result).iter().map(|warning| warning.location.clone()).collect();
    located.sort();
    let mut expected: Vec<String> = ["prefixItems", "additionalItems", "contains", "unevaluatedItems", "patternProperties",
        "propertyNames", "unevaluatedProperties", "dependentSchemas", "dependencies", "items/if", "items/then", "items/else"]
        .iter().map(|keyword| format!("{root}/{keyword}")).collect();
    expected.sort();
    assert_eq!(located, expected);
    assert!(warnings(&result).iter().all(|warning| warning.message.contains("not preserved")), "{:?}", result.diagnostics);
}

#[test]
fn object_count_keywords_on_a_parameter_are_reported_not_preserved() {
    let result = import_yaml(&with_query_schema("{ type: object, minProperties: 1 }"));
    let warnings = warnings(&result);
    assert_eq!(warnings.len(), 1, "{warnings:?}");
    assert_eq!(warnings[0].location, "#/paths/~1items/get/parameters/0/schema/minProperties");
    assert!(warnings[0].message.contains("not preserved"), "{}", warnings[0].message);
}

#[test]
fn strict_mode_fails_on_constraint_warnings() {
    let error = OpenApiImport::new(OpenApiSource::yaml(with_query_schema(r#"{ type: string, pattern: "^\\p{Lu}" }"#)))
        .strict()
        .build()
        .expect_err("strict import fails");
    let OpenApiError::Validation { path, .. } = error else { panic!("validation error: {error}") };
    assert_eq!(path, "#/paths/~1items/get/parameters/0/schema/pattern");
}

// ---- OpenAPI loader input matrix -------------------------------------

#[test]
fn loader_control_imports_rules() {
    let result = import_yaml(&with_query_schema("{ type: integer, maximum: 5 }"));
    assert_eq!(rules(query(endpoint(&result, "ListItems"), "value")), Constraints::new().maximum(5));
}

#[test]
fn loader_absent_keyword_imposes_nothing() {
    let result = import_yaml(&with_query_schema("{ type: string }"));
    assert_eq!(query(endpoint(&result, "ListItems"), "value").constraints, None);
}

#[test]
fn loader_boolean_exclusive_flags_become_numeric_bounds() {
    let result = import_yaml(&with_query_schema(
        "{ type: number, minimum: 1, exclusiveMinimum: true, maximum: 2, exclusiveMaximum: false }",
    ));
    assert_eq!(rules(query(endpoint(&result, "ListItems"), "value")), Constraints::new().exclusive_minimum(1).maximum(2));
}

/// A repeated keyword is a parse error in either format, never a
/// last-wins (or first-wins) bound.
#[test]
fn loader_duplicate_keyword_is_rejected_in_yaml() {
    let source = with_query_schema("{ type: integer, maximum: 5, maximum: 7 }");
    let outcome = OpenApiImport::new(OpenApiSource::yaml(source)).build();
    assert!(matches!(outcome, Err(OpenApiError::Parse { .. })), "{outcome:?}");
}

#[test]
fn loader_duplicate_keyword_in_json() {
    let source = r#"{"openapi": "3.0.3", "info": {"title": "T", "version": "1"},
      "paths": {"/items": {"get": {"operationId": "listItems",
        "parameters": [{"name": "value", "in": "query", "schema": {"type": "integer", "maximum": 5, "maximum": 7}}],
        "responses": {"204": {"description": "none"}}}}}}"#;
    let outcome = OpenApiImport::new(OpenApiSource::json(source)).build();
    assert!(matches!(outcome, Err(OpenApiError::Parse { .. })), "{outcome:?}");
}

#[test]
fn loader_infinite_bound_fails_with_its_path() {
    let outcome = OpenApiImport::new(OpenApiSource::yaml(with_query_schema("{ type: number, maximum: .inf }"))).build();
    let Err(OpenApiError::Validation { path, .. }) = outcome else { panic!("validation error: {outcome:?}") };
    assert!(path.ends_with("/maximum"), "{path}");
}

#[test]
fn loader_huge_exponent_fails_with_its_path() {
    let outcome = OpenApiImport::new(OpenApiSource::yaml(with_query_schema("{ type: number, maximum: 1e10001 }"))).build();
    let Err(OpenApiError::Validation { path, .. }) = outcome else { panic!("validation error: {outcome:?}") };
    assert_eq!(path, "#/paths/~1items/get/parameters/0/schema/maximum");
}

/// `openapiv3` reads `maxLength` as an optional count, so an explicit
/// `null` would deserialize as absent; the source capture refuses it first,
/// located at the keyword, in both formats.
#[test]
fn loader_null_value_keyword_fails_with_its_path() {
    let yaml = OpenApiImport::new(OpenApiSource::yaml(with_query_schema("{ type: string, maxLength: null }"))).build();
    let Err(OpenApiError::Validation { path, message }) = yaml else { panic!("validation error: {yaml:?}") };
    assert!(path.ends_with("/parameters/0/schema/maxLength"), "{path}");
    assert!(message.contains("null"), "{message}");

    let json = r#"{"openapi":"3.0.3","info":{"title":"T","version":"1"},"paths":{"/items":{"get":{"operationId":"listItems",
        "parameters":[{"name":"value","in":"query","schema":{"type":"integer","maximum":null}}],"responses":{"204":{"description":"ok"}}}}}}"#;
    let json = OpenApiImport::new(OpenApiSource::json(json)).build();
    let Err(OpenApiError::Validation { path, .. }) = json else { panic!("validation error: {json:?}") };
    assert_eq!(path, "#/paths/~1items/get/parameters/0/schema/maximum");
}

/// `openapiv3` writes no `enum` for an empty list, so the capture refuses
/// one before it could read as "any value".
#[test]
fn loader_empty_enum_fails_with_its_path() {
    let (path, message) = rejected(OpenApiSource::yaml(with_query_schema("{ type: string, enum: [] }")));
    assert_eq!(path, "#/paths/~1items/get/parameters/0/schema/enum");
    assert!(message.contains("at least one value"), "{message}");
}

/// A YAML flow mapping puts several keys on one line, and the parse
/// stand-in rewrites only `key: value` lines, so a bound beyond `i64` there
/// is a parse error: located, and never a silently clamped bound.
#[test]
fn loader_flow_style_bound_beyond_i64_is_a_parse_error() {
    let outcome = OpenApiImport::new(OpenApiSource::yaml(with_query_schema("{ type: integer, maximum: 18446744073709551616 }"))).build();
    let Err(OpenApiError::Parse { message, .. }) = outcome else { panic!("parse error: {outcome:?}") };
    assert!(message.contains("schema.maximum"), "{message}");
}

/// `openapiv3` rejects a string where it reads a count, so the document
/// does not parse: a wrong type is never mistaken for an absent keyword.
#[test]
fn loader_wrong_type_length_is_a_parse_error() {
    let outcome = OpenApiImport::new(OpenApiSource::yaml(with_query_schema(r#"{ type: string, maxLength: "5" }"#))).build();
    let Err(OpenApiError::Parse { message, .. }) = outcome else { panic!("parse error: {outcome:?}") };
    assert!(message.contains(r#"string "5", expected usize"#), "{message}");
}

/// The same keywords inside a component, a JSON or form body, or a
/// response are dropped by the parser too, so each is reported at its own
/// pointer; the stored component holds none of them.
#[test]
fn dropped_subschema_keywords_in_components_bodies_and_forms_are_reported_not_preserved() {
    let yaml = r#"
openapi: "3.0.3"
info: { title: Test, version: "1" }
servers: [{ url: "https://api.test" }]
paths:
  /items:
    post:
      operationId: createItem
      requestBody:
        content:
          application/json:
            schema: { type: object, properties: { tags: { type: array, contains: { maxLength: 3 } } } }
      responses:
        '200':
          description: ok
          content:
            application/json:
              schema: { $ref: '#/components/schemas/Item' }
  /uploads:
    post:
      operationId: upload
      requestBody:
        content:
          multipart/form-data:
            schema:
              type: object
              propertyNames: { maxLength: 8 }
              properties:
                file: { type: string, format: binary, if: { maxLength: 1 } }
                note: { type: string, else: { maxLength: 2 } }
      responses: { '204': { description: none } }
components:
  schemas:
    Item:
      type: object
      patternProperties: { "^x-": { type: string } }
      properties:
        code: { type: string, if: { minLength: 1 }, then: { maxLength: 3 } }
        list: { type: array, items: { type: integer }, prefixItems: [{ type: integer }], unevaluatedItems: false }
"#;
    let result = import_yaml(yaml);
    let mut located: Vec<String> = warnings(&result)
        .iter()
        .filter(|warning| warning.message.contains("not preserved"))
        .map(|warning| warning.location.clone())
        .collect();
    located.sort();
    let mut expected: Vec<String> = [
        "#/components/schemas/Item/patternProperties",
        "#/components/schemas/Item/properties/code/if",
        "#/components/schemas/Item/properties/code/then",
        "#/components/schemas/Item/properties/list/prefixItems",
        "#/components/schemas/Item/properties/list/unevaluatedItems",
        "#/paths/~1items/post/requestBody/content/application~1json/schema/properties/tags/contains",
        "#/paths/~1uploads/post/requestBody/content/multipart~1form-data/schema/propertyNames",
        "#/paths/~1uploads/post/requestBody/content/multipart~1form-data/schema/properties/file/if",
        "#/paths/~1uploads/post/requestBody/content/multipart~1form-data/schema/properties/note/else",
    ]
    .map(str::to_owned)
    .to_vec();
    expected.sort();
    assert_eq!(located, expected, "{:?}", result.diagnostics);

    let item = component(&result, "Item").to_string();
    for keyword in ["patternProperties", "\"if\"", "\"then\"", "prefixItems", "unevaluatedItems"] {
        assert!(!item.contains(keyword), "{keyword} stored in {item}");
    }
    let strict = OpenApiImport::new(OpenApiSource::yaml(yaml)).strict().build();
    assert!(matches!(strict, Err(OpenApiError::Validation { .. })), "{strict:?}");
}

/// A component rule its declared `type` cannot carry is removed with a
/// warning at the keyword, as an input's is; its valid siblings stay, and
/// the stored component passes the definition check.
#[test]
fn a_component_rule_inapplicable_to_its_type_is_removed_with_a_warning() {
    let yaml = r#"
openapi: "3.0.3"
info: { title: Test, version: "1" }
servers: [{ url: "https://api.test" }]
paths:
  /items:
    post:
      operationId: createItem
      requestBody:
        content:
          application/json:
            schema: { $ref: '#/components/schemas/Item' }
      responses: { '204': { description: none } }
components:
  schemas:
    Item:
      type: object
      properties:
        count: { type: integer, minimum: 1, maxLength: 3 }
        code: { type: string, maxLength: 3, minimum: 1, minItems: 1 }
        free: { maxLength: 3, minimum: 1 }
"#;
    let result = import_yaml(yaml);
    let located: Vec<(&str, &str)> =
        warnings(&result).iter().map(|warning| (warning.location.as_str(), warning.message.as_str())).collect();
    assert_eq!(located, [
        ("#/components/schemas/Item/properties/code/minimum", "Rule `minimum` not preserved: does not apply to the declared type, a string"),
        ("#/components/schemas/Item/properties/code/minItems", "Rule `minItems` not preserved: does not apply to the declared type, a string"),
        ("#/components/schemas/Item/properties/count/maxLength", "Rule `maxLength` not preserved: does not apply to the declared type, an integer"),
    ]);
    let item = component(&result, "Item");
    assert_eq!(item["properties"]["count"], json!({"type": "integer", "minimum": 1}));
    assert_eq!(item["properties"]["code"], json!({"type": "string", "maxLength": 3}));
    assert_eq!(item["properties"]["free"], json!({"maxLength": 3, "minimum": 1}));
    assert!(schematic_define::constraints::constraint_problems(&result.api).is_empty());
    let strict = OpenApiImport::new(OpenApiSource::yaml(yaml)).strict().build();
    assert!(matches!(strict, Err(OpenApiError::Validation { .. })), "{strict:?}");
}
