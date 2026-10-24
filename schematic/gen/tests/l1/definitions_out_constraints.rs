//! `import --definitions-out` rebuilds the imported value rules: models carry
//! them as `#[schemars(extend(..))]` field attributes, and parameters, form
//! fields, and alias-typed bodies carry them as `Constraints` builder calls.

use schematic_gen::import_pipeline::{ImportOptions, run_import};

/// Every rule shape a model field or endpoint input can carry, including a
/// renamed field, an optional enum, a primitive alias used by a field and a
/// body, a bound beyond `i64`, and a decimal no `f64` holds.
const SPEC: &str = r#"
openapi: '3.0.3'
info: {title: Widgets, version: '1'}
paths:
  /widgets:
    post:
      operationId: createWidget
      parameters:
        - name: limit
          in: query
          schema: {type: integer, minimum: 1, maximum: 100}
      requestBody:
        content:
          application/json:
            schema: {$ref: '#/components/schemas/Widget'}
      responses:
        '200':
          description: ok
          content:
            application/json:
              schema: {$ref: '#/components/schemas/Plain'}
  /rename:
    post:
      operationId: rename
      requestBody:
        content:
          application/json:
            schema: {$ref: '#/components/schemas/Slug'}
      responses:
        '204': {description: done}
components:
  schemas:
    Widget:
      type: object
      required: [name, big]
      properties:
        name: {type: string, minLength: 1, maxLength: 64}
        big: {type: integer, minimum: 0, maximum: 9223372036854775808}
        displayName: {type: string, minLength: 2}
        mode: {type: string, enum: [fast, slow]}
        tags:
          type: array
          maxItems: 3
          items: {type: string, maxLength: 5}
        slug: {$ref: '#/components/schemas/Slug'}
        ratio: {type: number, minimum: 0, exclusiveMinimum: true, multipleOf: 0.10000000000000000001}
    Slug: {type: string, pattern: '^[a-z]+$'}
    Plain:
      type: object
      properties:
        label: {type: string}
"#;

/// Imports [`SPEC`] with `--definitions-out` and returns `(mod.rs, types.rs)`.
fn import_definitions() -> (String, String) {
    let temp_dir = tempfile::TempDir::new().unwrap();
    let input = temp_dir.path().join("widgets.yaml");
    std::fs::write(&input, SPEC).unwrap();
    let definitions = temp_dir.path().join("widgets");
    run_import(&ImportOptions {
        input: input.display().to_string(),
        api_name: Some("Widgets".to_string()),
        module_path: None,
        output: String::new(),
        definitions_out: Some(definitions.display().to_string()),
        overlay: None,
        exclude_paths: vec![],
        request_id_header: None,
        dry_run: false,
        strict: false,
        verbose: false,
    })
    .expect("import succeeds");
    let read = |name: &str| std::fs::read_to_string(definitions.join(name)).unwrap();
    (read("mod.rs"), read("types.rs"))
}

/// `source` without whitespace or the formatter's trailing commas.
fn compact(source: &str) -> String {
    source.split_whitespace().collect::<String>().replace(",)", ")")
}

#[test]
fn definitions_out_models_carry_each_field_rule_as_a_schemars_extend() {
    let (_, types) = import_definitions();
    syn::parse_file(&types).expect("types.rs parses");
    let types = compact(&types);

    for expected in [
        r#"#[schemars(extend("maxLength"=64u64,"minLength"=1u64))]pubname:String"#,
        r#"#[schemars(extend("maximum"=9223372036854775808u64,"minimum"=0u64))]pubbig:i64"#,
        r#"#[serde(rename="displayName")]#[serde(skip_serializing_if="Option::is_none")]#[schemars(extend("minLength"=2u64))]pubdisplay_name:Option<String>"#,
        r#"#[schemars(extend("enum"=["fast","slow"]))]pubmode:Option<String>"#,
        r#"#[schemars(extend("items"={"maxLength":5u64,"type":"string"},"maxItems"=3u64))]pubtags:Option<Vec<String>>"#,
        r#"#[schemars(extend("pattern"="^[a-z]+$"))]pubslug:Option<Slug>"#,
        r#"#[schemars(extend("exclusiveMinimum"=0u64,"multipleOf"=0.1f64,"x-schematic-exact"={"multipleOf":"0.10000000000000000001"}))]pubratio:Option<f64>"#,
    ] {
        assert!(types.contains(expected), "missing {expected}\n{types}");
    }
    let plain = &types[types.find("pubstructPlain{").expect("Plain is emitted")..];
    let plain = &plain[..plain.find('}').unwrap()];
    assert!(!plain.contains("schemars("), "an unconstrained model has no attribute: {plain}");
}

#[test]
fn definitions_out_endpoints_carry_parameter_and_alias_body_rules() {
    let (module, _) = import_definitions();
    syn::parse_file(&module).expect("mod.rs parses");
    let module = compact(&module);

    for expected in [
        r#"constraints:Some(Box::new(schematic_define::constraints::Constraints::new().minimum("1").maximum("100"))),"#,
        r#".request_json::<Slug>(ctx).request_constraints(schematic_define::constraints::Constraints::new().pattern("^[a-z]+$"))"#,
    ] {
        assert!(module.contains(expected), "missing {expected}\n{module}");
    }
    assert!(!module.contains(".request_json::<Widget>(ctx).request_constraints"), "a component body keeps its rules on its model");
}

/// A JSON body typed as a plain array alias whose inline item objects hold
/// a rule: the endpoint's request constraints cannot carry it.
const FOREST_SPEC: &str = r#"
openapi: '3.0.3'
info: {title: Forests, version: '1'}
paths:
  /forests:
    post:
      operationId: plantForest
      requestBody:
        content:
          application/json:
            schema: {$ref: '#/components/schemas/Forest'}
      responses:
        '204': {description: planted}
components:
  schemas:
    Forest:
      type: array
      maxItems: 3
      items:
        type: object
        properties:
          label: {type: string, maxLength: 11}
"#;

#[test]
fn definitions_out_rejects_an_alias_body_rule_it_cannot_rebuild_and_writes_nothing() {
    let temp_dir = tempfile::TempDir::new().unwrap();
    let input = temp_dir.path().join("forests.yaml");
    std::fs::write(&input, FOREST_SPEC).unwrap();
    let definitions = temp_dir.path().join("forests");

    let error = run_import(&ImportOptions {
        input: input.display().to_string(),
        api_name: Some("Forests".to_string()),
        module_path: None,
        output: String::new(),
        definitions_out: Some(definitions.display().to_string()),
        overlay: None,
        exclude_paths: vec![],
        request_id_header: None,
        dry_run: false,
        strict: false,
        verbose: false,
    })
    .expect_err("the label rule cannot be rebuilt")
    .to_string();

    for expected in ["endpoint `PlantForest`", "plain alias `Forest`", "`#/components/schemas/Forest/items/properties/label/maxLength`", "component name"] {
        assert!(error.contains(expected), "missing {expected}: {error}");
    }
    assert!(!definitions.join("types.rs").exists() && !definitions.join("mod.rs").exists(), "nothing is written");
}
