//! Value constraints through the public authoring, validation, and bundle
//! paths: every field shape once per field validator, the bundle loader's
//! document shapes, and the documented pattern-admission table.

use schematic_define::constraints::{Constraints, PatternAdmission, admit_pattern, constraint_problems};
use schematic_define::{
    ApiRequest, BundleError, DefinitionBundle, DefinitionContext, DefinitionError, Endpoint, FormField, ParamDef,
    PrimitiveType, QueryParamType, RestApi, TypeRef, definition_type,
};
use serde_json::{Value, json};

definition_type!(#[derive(serde::Serialize, schemars::JsonSchema)] pub struct Item { pub id: u64 });

fn int_param(rules: Constraints) -> ParamDef { ParamDef::new("limit", QueryParamType::Integer).constraints(rules) }
fn string_param(rules: Constraints) -> ParamDef { ParamDef::new("name", QueryParamType::String).constraints(rules) }
fn tags_param(rules: Constraints) -> ParamDef {
    ParamDef::new("tags", QueryParamType::Array(Box::new(QueryParamType::String))).constraints(rules)
}
fn json_param(rules: Constraints) -> ParamDef { ParamDef::new("filter", QueryParamType::Json).constraints(rules) }

/// A one-endpoint API whose endpoint `edit` finishes; built through the
/// public builders so every check runs as an author meets it.
fn build(edit: impl FnOnce(schematic_define::EndpointBuilder) -> schematic_define::EndpointBuilder) -> Result<RestApi, DefinitionError> {
    let ctx = DefinitionContext::types_in("::models");
    // Build the endpoint without validating it, so the API build reports.
    let endpoint = edit(Endpoint::get("List", "/items")).describes("List items").returns_empty();
    let endpoint = endpoint.build()?;
    RestApi::builder(&ctx, "Items", "https://api.example.com").describes("Items").no_auth().endpoints([endpoint]).build()
}

fn post(edit: impl FnOnce(schematic_define::EndpointBuilder, &mut DefinitionContext) -> schematic_define::EndpointBuilder)
    -> Result<RestApi, DefinitionError> {
    let mut ctx = DefinitionContext::types_in("::models");
    let endpoint = edit(Endpoint::post("Create", "/items"), &mut ctx).describes("Create").returns_empty().build()?;
    RestApi::builder(&ctx, "Items", "https://api.example.com").describes("Items").no_auth().endpoints([endpoint]).build()
}

/// The located failure of `result`, as `(path, keyword, reason)`.
fn located(result: Result<RestApi, DefinitionError>) -> (String, &'static str, String) {
    match result {
        Err(DefinitionError::InvalidConstraint { problem, .. }) => {
            assert!(problem.endpoint == "List" || problem.endpoint == "Create", "{}", problem.endpoint);
            (problem.path, problem.keyword, problem.reason)
        }
        other => panic!("expected an invalid constraint, got {other:?}"),
    }
}

#[test]
fn valid_rules_on_every_input_build_and_are_stored() {
    let api = build(|endpoint| endpoint
        .query(int_param(Constraints::new().minimum(1).maximum(100).multiple_of(1)))
        .query(tags_param(Constraints::new().min_items(1).max_items(10).unique_items().items(Constraints::new().min_length(1).pattern("^[a-z]+$"))))
        .query(string_param(Constraints::new().min_length(0).max_length(0).format("uuid").allowed_values([""]))))
        .unwrap();
    assert!(constraint_problems(&api).is_empty());
    let query = &api.endpoints[0].params.as_ref().unwrap().query;
    assert_eq!(query[0].constraints.as_ref().unwrap().maximum.as_ref().unwrap().as_str(), "100");
    assert_eq!(query[1].constraints.as_ref().unwrap().items.as_ref().unwrap().pattern.as_deref(), Some("^[a-z]+$"));

    let typed_path = Endpoint::get("Get", "/items/{id}").describes("Get").returns_empty()
        .path_param(ParamDef::path("id", QueryParamType::String).constraints(Constraints::new().pattern(r"^\d+$")))
        .build().unwrap();
    assert!(typed_path.params.unwrap().path[0].constraints.is_some());
}

#[test]
fn bodies_and_form_fields_take_rules_only_where_a_value_is_constrainable() {
    // Controls.
    post(|e, _| e.request_text().request_constraints(Constraints::new().max_length(1000))).unwrap();
    post(|e, ctx| e.request_json::<Vec<Item>>(ctx).request_constraints(Constraints::new().min_items(1).max_items(50))).unwrap();
    post(|e, ctx| e.request_json::<Vec<String>>(ctx).request_constraints(Constraints::new().items(Constraints::new().max_length(5)))).unwrap();
    post(|e, _| e.request_form(vec![FormField::text("title").constraints(Constraints::new().min_length(1)),
        FormField::json("labels", TypeRef::Array(Box::new(TypeRef::Primitive(PrimitiveType::String))))
            .constraints(Constraints::new().unique_items())])).unwrap();

    let refused = [
        ("body", "maxLength", post(|e, _| e.request_binary().request_constraints(Constraints::new().max_length(1))), "binary"),
        ("body", "minItems", post(|e, ctx| e.request_json::<Item>(ctx).request_constraints(Constraints::new().min_items(1))), "schema component"),
        ("body[]", "maximum", post(|e, ctx| e.request_json::<Vec<Item>>(ctx).request_constraints(Constraints::new().items(Constraints::new().maximum(1)))), "schema component"),
        ("body", "minLength", post(|e, _| e.request_form(vec![FormField::text("a")]).request_constraints(Constraints::new().min_length(1))), "form body"),
        ("body", "minLength", post(|e, _| e.request_constraints(Constraints::new().min_length(1))), "no request body"),
        ("form.upload", "maxLength", post(|e, _| e.request_form(vec![FormField::file("upload").constraints(Constraints::new().max_length(1))])), "file"),
        ("form.many", "minItems", post(|e, _| e.request_form(vec![FormField::files("many").constraints(Constraints::new().min_items(1))])), "file"),
        ("form.title", "minimum", post(|e, _| e.request_urlencoded(vec![FormField::text("title").constraints(Constraints::new().minimum(1))])), "a string"),
    ];
    for (path, keyword, result, reason) in refused {
        let (found_path, found_keyword, found_reason) = located(result);
        assert_eq!((found_path.as_str(), found_keyword), (path, keyword));
        assert!(found_reason.contains(reason), "{path}: {found_reason}");
    }
}

#[test]
fn each_semantic_definition_error_is_located_by_api_endpoint_path_and_keyword() {
    let cases: Vec<(ParamDef, &str, &str, &str)> = vec![
        (int_param(Constraints::new().min_length(1)), "query.limit", "minLength", "does not apply to an integer"),
        (string_param(Constraints::new().maximum(1)), "query.name", "maximum", "does not apply to a string"),
        (ParamDef::new("on", QueryParamType::Boolean).constraints(Constraints::new().format("x")), "query.on", "format", "does not apply to a boolean"),
        (string_param(Constraints::new().items(Constraints::new())), "query.name", "items", "does not apply to a string"),
        (string_param(Constraints::new().min_length(5).max_length(2)), "query.name", "minLength", "5 is above the maximum 2"),
        (tags_param(Constraints::new().min_items(3).max_items(1)), "query.tags", "minItems", "3 is above the maximum 1"),
        (int_param(Constraints::new().minimum(10).maximum(1)), "query.limit", "minimum", "no number"),
        (int_param(Constraints::new().exclusive_minimum(1).exclusive_maximum(1)), "query.limit", "exclusiveMinimum", "no number"),
        (int_param(Constraints::new().minimum(1).exclusive_maximum(1)), "query.limit", "minimum", "no number"),
        (int_param(Constraints::new().multiple_of(0)), "query.limit", "multipleOf", "greater than zero"),
        (int_param(Constraints::new().multiple_of("-0.5")), "query.limit", "multipleOf", "greater than zero"),
        (int_param(Constraints::new().maximum("ten")), "query.limit", "maximum", "not JSON number text"),
        (int_param(Constraints::new().maximum(f64::NAN)), "query.limit", "maximum", "not JSON number text"),
        (int_param(Constraints::new().minimum(f64::INFINITY)), "query.limit", "minimum", "not JSON number text"),
        (int_param(Constraints::new().maximum("1e10001")), "query.limit", "maximum", "exponent"),
        (string_param(Constraints::new().allowed_values(Vec::<String>::new())), "query.name", "enum", "at least one"),
        (string_param(Constraints::new().allowed_values([json!("a"), json!(1)])), "query.name", "enum", "member 1 is not a string"),
        (int_param(Constraints::new().allowed_values([1.5])), "query.limit", "enum", "member 0 is not an integer"),
        (string_param(Constraints::new().allowed_values([Value::Null])), "query.name", "enum", "member 0 is not a string"),
        (string_param(Constraints::new().constant(1)), "query.name", "const", "is not a string"),
        (string_param(Constraints::new().pattern(r"^\s+$")), "query.name", "pattern", "outside the enforced pattern subset"),
        (string_param(Constraints::new().pattern("(a")), "query.name", "pattern", "not a valid regular expression"),
        (string_param(Constraints::new().format("")), "query.name", "format", "names no format"),
        (tags_param(Constraints::new().items(Constraints::new().maximum(1))), "query.tags[]", "maximum", "does not apply to a string"),
    ];
    for (param, path, keyword, reason) in cases {
        let result = build(|endpoint| endpoint.query(param));
        let text = result.as_ref().unwrap_err().to_string();
        let (found_path, found_keyword, found_reason) = located(result);
        assert_eq!((found_path.as_str(), found_keyword), (path, keyword), "{text}");
        assert!(found_reason.contains(reason), "{path} {keyword}: {found_reason}");
        // Endpoint build reports first, without an API name.
        assert!(text.starts_with("<endpoint> endpoint List: "), "{text}");
        assert!(text.contains(&format!("{path} `{keyword}`")), "{text}");
    }
}

#[test]
fn exceptions_to_the_default_outcome_are_accepted() {
    let api = build(|endpoint| endpoint
        // An empty pattern matches everything; an unknown format is kept.
        .query(string_param(Constraints::new().pattern("").format("x-vendor-thing")))
        // A Rust-incompatible pattern is kept when marked unenforced.
        .query(ParamDef::new("code", QueryParamType::String).constraints(Constraints::new().unenforced_pattern(r"^\p{Lu}+$")))
        // `enum`/`const` on free-form JSON accept any value, `null` included.
        .query(json_param(Constraints::new().allowed_values([Value::Null, json!({"a": 1})]).constant(Value::Null)))
        // Equal inclusive bounds admit exactly one value.
        .query(int_param(Constraints::new().minimum(5).maximum(5))))
        .unwrap();
    let query = &api.endpoints[0].params.as_ref().unwrap().query;
    assert_eq!(query[0].constraints.as_ref().unwrap().format.as_deref(), Some("x-vendor-thing"));
    assert!(query[1].constraints.as_ref().unwrap().pattern_unenforced);
    assert_eq!(query[2].constraints.as_ref().unwrap().constant, Some(Value::Null));

    // A later `.pattern(..)` replaces the unenforced mark: it is authored again.
    let error = located(build(|endpoint| endpoint.query(string_param(Constraints::new().unenforced_pattern("a.c").pattern("a.c")))));
    assert_eq!((error.0.as_str(), error.1), ("query.name", "pattern"));
}

#[test]
fn rules_on_a_json_body_admit_null_only_through_an_optional_root() {
    let optional = TypeRef::Optional(Box::new(TypeRef::Primitive(PrimitiveType::String)));
    let mut api = post(|e, _| e.request_text()).unwrap();
    api.endpoints[0].request = Some(ApiRequest::Json(optional));
    api.endpoints[0].request_constraints = Some(Box::new(Constraints::new().allowed_values([Value::Null, json!("a")])));
    assert!(constraint_problems(&api).is_empty());
    api.endpoints[0].request = Some(ApiRequest::Json(TypeRef::Primitive(PrimitiveType::String)));
    let problem = &constraint_problems(&api)[0];
    assert_eq!((problem.path.as_str(), problem.keyword, problem.reason.as_str()), ("body", "enum", "member 0 is not a string"));
}

// ---- The bundle loader and serialized field shapes ----

fn bundle_api() -> RestApi {
    build(|endpoint| endpoint
        .query(int_param(Constraints::new().minimum(1).maximum("18446744073709551616").multiple_of("0.1")))
        .query(tags_param(Constraints::new().max_items(3).unique_items().items(Constraints::new().min_length(1).pattern("^[a-z]+$"))))
        .query(json_param(Constraints::new().allowed_values([Value::Null, json!(1), json!({"a": null})]).constant(Value::Null))))
        .unwrap()
}

fn canonical() -> String { DefinitionBundle::new(bundle_api()).unwrap().to_json() }

/// The canonical bundle with the `limit` parameter's constraints replaced.
fn with_limit_rules(rules: Value) -> String {
    let mut document: Value = serde_json::from_str(&canonical()).unwrap();
    document["api"]["endpoints"][0]["params"]["query"][0]["constraints"] = rules;
    serde_json::to_string(&document).unwrap()
}

fn with_tags_rules(rules: Value) -> String {
    let mut document: Value = serde_json::from_str(&canonical()).unwrap();
    document["api"]["endpoints"][0]["params"]["query"][1]["constraints"] = rules;
    serde_json::to_string(&document).unwrap()
}

fn load(text: &str) -> Result<DefinitionBundle, BundleError> { DefinitionBundle::from_json(text) }

#[test]
fn bundle_control_round_trips_the_rules_exactly_through_repeated_reads_and_writes() {
    let first = canonical();
    assert!(first.contains(r#""maximum": "18446744073709551616""#), "{first}");
    assert!(first.contains(r#""multiple_of": "0.1""#), "{first}");
    let loaded = load(&first).unwrap();
    assert_eq!(loaded.api(), &bundle_api());
    let second = loaded.to_json();
    assert_eq!(second, first);
    assert_eq!(load(&second).unwrap().to_json(), first);
}

#[test]
fn bundle_without_constraint_metadata_reads_as_absent() {
    let mut document: Value = serde_json::from_str(&canonical()).unwrap();
    for param in document["api"]["endpoints"][0]["params"]["query"].as_array_mut().unwrap() {
        param.as_object_mut().unwrap().remove("constraints");
    }
    let loaded = load(&serde_json::to_string(&document).unwrap()).unwrap();
    assert!(loaded.api().endpoints[0].params.as_ref().unwrap().query.iter().all(|param| param.constraints.is_none()));
    // The committed fixture predates constraints and still loads.
    let fixture = load(include_str!("fixtures/bundles/fixture.json")).unwrap();
    assert!(fixture.api().endpoints.iter().all(|endpoint| endpoint.request_constraints.is_none()));
}

#[test]
fn bundle_document_shapes_duplicate_key_and_trailing_content_fail() {
    let text = canonical();
    let duplicated = text.replacen(r#""max_items": 3,"#, r#""max_items": 3, "max_items": 4,"#, 1);
    assert_ne!(duplicated, text);
    assert!(matches!(load(&duplicated), Err(BundleError::Json { .. })), "duplicate key");
    assert!(matches!(load(&format!("{text} {{}}")), Err(BundleError::Json { .. })), "trailing content");
    // An explicit default the writer omits is accepted input.
    let explicit = with_limit_rules(json!({"minimum": "1", "unique_items": false, "pattern_unenforced": false}));
    let loaded = load(&explicit).unwrap();
    let rules = loaded.api().endpoints[0].params.as_ref().unwrap().query[0].constraints.clone().unwrap();
    assert!(!rules.unique_items && !rules.pattern_unenforced);
}

/// Every field-shape cell, as `(label, edited bundle, expected outcome)`.
/// `Ok(())` loads; `Err(fragment)` fails with `fragment` in its message.
#[test]
fn serialized_field_shapes_per_validator() {
    let cells: Vec<(&str, String, Result<(), &str>)> = vec![
        // The constraints object itself.
        ("constraints absent", { let mut d: Value = serde_json::from_str(&canonical()).unwrap();
            d["api"]["endpoints"][0]["params"]["query"][0].as_object_mut().unwrap().remove("constraints"); d.to_string() }, Ok(())),
        ("constraints null", with_limit_rules(Value::Null), Err("null")),
        ("constraints wrong type", with_limit_rules(json!([])), Err("expected an object")),
        ("constraints empty", with_limit_rules(json!({})), Ok(())),
        ("constraints unknown key", with_limit_rules(json!({"maxLen": 1})), Err("unknown field")),
        // Counts and lengths (one validator: a non-null u64).
        ("count control", with_tags_rules(json!({"min_items": 0, "max_items": 2})), Ok(())),
        ("count null", with_tags_rules(json!({"max_items": null})), Err("null")),
        ("count as text", with_tags_rules(json!({"max_items": "5"})), Err("invalid type")),
        ("count negative", with_tags_rules(json!({"min_items": -1})), Err("invalid value")),
        ("count fractional", with_tags_rules(json!({"min_items": 1.5})), Err("invalid type")),
        ("min_length wired", with_tags_rules(json!({"items": {"min_length": "1"}})), Err("invalid type")),
        ("max_length wired", with_tags_rules(json!({"items": {"max_length": null}})), Err("null")),
        // Numeric limits (one validator: JSON number text in a string).
        ("limit control", with_limit_rules(json!({"maximum": "1e3", "minimum": "-0.5"})), Ok(())),
        ("limit null", with_limit_rules(json!({"maximum": null})), Err("null")),
        ("limit as a JSON number", with_limit_rules(json!({"maximum": 100})), Err("invalid type")),
        ("limit malformed", with_limit_rules(json!({"maximum": "1.0.0"})), Err("not JSON number text")),
        ("limit exponent", with_limit_rules(json!({"maximum": "1e10001"})), Err("exponent")),
        ("minimum wired", with_limit_rules(json!({"minimum": 1})), Err("invalid type")),
        ("exclusive_minimum wired", with_limit_rules(json!({"exclusive_minimum": "x"})), Err("not JSON number text")),
        ("exclusive_maximum wired", with_limit_rules(json!({"exclusive_maximum": true})), Err("invalid type")),
        ("multiple_of wired", with_limit_rules(json!({"multiple_of": "0"})), Err("greater than zero")),
        ("interval semantic", with_limit_rules(json!({"minimum": "2", "maximum": "1"})), Err("no number")),
        // Pattern and format (strings).
        ("pattern empty", with_tags_rules(json!({"items": {"pattern": ""}})), Ok(())),
        ("pattern null", with_tags_rules(json!({"items": {"pattern": null}})), Err("null")),
        ("pattern wrong type", with_tags_rules(json!({"items": {"pattern": 5}})), Err("invalid type")),
        ("pattern unsupported", with_tags_rules(json!({"items": {"pattern": "a.c"}})), Err("enforced pattern subset")),
        ("pattern unenforced", with_tags_rules(json!({"items": {"pattern": "a.c", "pattern_unenforced": true}})), Ok(())),
        ("unenforced without pattern", with_tags_rules(json!({"items": {"pattern_unenforced": true}})), Err("no pattern")),
        ("format unknown name", with_tags_rules(json!({"items": {"format": "x-anything"}})), Ok(())),
        ("format empty", with_tags_rules(json!({"items": {"format": ""}})), Err("names no format")),
        ("format null", with_tags_rules(json!({"items": {"format": null}})), Err("null")),
        ("format wrong type", with_tags_rules(json!({"items": {"format": 5}})), Err("invalid type")),
        // uniqueItems (a Boolean whose false is absent).
        ("unique true", with_tags_rules(json!({"unique_items": true})), Ok(())),
        ("unique null", with_tags_rules(json!({"unique_items": null})), Err("null")),
        ("unique wrong type", with_tags_rules(json!({"unique_items": "yes"})), Err("invalid type")),
        // items (an object of rules).
        ("items null", with_tags_rules(json!({"items": null})), Err("null")),
        ("items wrong type", with_tags_rules(json!({"items": []})), Err("expected an object")),
        ("items empty", with_tags_rules(json!({"items": {}})), Ok(())),
        // enum (a list of whole values checked against the parameter type).
        ("enum control", with_tags_rules(json!({"allowed_values": [["a"], []]})), Ok(())),
        ("enum null", with_tags_rules(json!({"allowed_values": null})), Err("null")),
        ("enum wrong type", with_tags_rules(json!({"allowed_values": "a"})), Err("invalid type")),
        ("enum empty", with_tags_rules(json!({"allowed_values": []})), Err("at least one")),
        ("enum one wrong element", with_tags_rules(json!({"allowed_values": [["a"], "b"]})), Err("member 1 is not an array")),
        ("enum every wrong element", with_tags_rules(json!({"allowed_values": [1, 2]})), Err("member 0 is not an array")),
        ("enum element of wrong item type", with_tags_rules(json!({"allowed_values": [[1]]})), Err("member 0 is not an array")),
        // const (one whole value; null is data where the type admits it).
        ("const control", with_tags_rules(json!({"constant": ["a"]})), Ok(())),
        ("const wrong type", with_tags_rules(json!({"constant": "a"})), Err("is not an array")),
        ("const null on a non-nullable type", with_tags_rules(json!({"constant": null})), Err("is not an array")),
    ];
    for (label, text, expected) in cells {
        match (load(&text), expected) {
            (Ok(_), Ok(())) => {}
            (Err(error), Err(fragment)) => assert!(error.to_string().contains(fragment), "{label}: {error}"),
            (outcome, expected) => panic!("{label}: expected {expected:?}, got {outcome:?}"),
        }
    }
}

#[test]
fn null_in_enum_and_const_is_preserved_as_data_through_a_bundle() {
    let loaded = load(&canonical()).unwrap();
    let rules = loaded.api().endpoints[0].params.as_ref().unwrap().query[2].constraints.clone().unwrap();
    assert_eq!(rules.allowed_values, Some(vec![Value::Null, json!(1), json!({"a": null})]));
    assert_eq!(rules.constant, Some(Value::Null));
    // Outside constraint data, an explicit null is still refused.
    let mut document: Value = serde_json::from_str(&canonical()).unwrap();
    document["api"]["endpoints"][0]["params"]["query"][2]["rust_name"] = Value::Null;
    let error = load(&document.to_string()).unwrap_err();
    assert!(error.to_string().contains("rust_name"), "{error}");
}

#[test]
fn a_loaded_rule_located_error_names_the_api_endpoint_path_and_keyword() {
    let error = load(&with_limit_rules(json!({"min_length": 1}))).unwrap_err();
    assert_eq!(error.to_string(), "Items endpoint List: query.limit `minLength` does not apply to an integer");
}

// ---- Pattern admission, against the documented table ----

#[test]
fn pattern_admission_matches_the_documented_table() {
    let documentation = include_str!("../../docs/value-constraints.md");
    let table = documentation.split("<!-- pattern-admission:start -->").nth(1).unwrap()
        .split("<!-- pattern-admission:end -->").next().unwrap();
    let cell = |text: &str| {
        let text = text.trim().replace("\\|", "|");
        if text == "(empty)" { String::new() } else { text.trim_matches('`').to_owned() }
    };
    let mut rows = 0;
    for line in table.lines().filter(|line| line.starts_with('|')).skip(2) {
        // Split on pipes not escaped with a backslash.
        let mut cells = Vec::new();
        let mut current = String::new();
        let mut previous = ' ';
        for c in line.trim().trim_matches('|').chars() {
            if c == '|' && previous != '\\' { cells.push(std::mem::take(&mut current)); } else { current.push(c); }
            previous = c;
        }
        cells.push(current);
        let (source, outcome, detail) = (cell(&cells[0]), cells[1].trim(), cell(&cells[2]));
        let admission = admit_pattern(&source);
        match (outcome, &admission) {
            ("enforced", PatternAdmission::Enforced { translated }) => {
                assert_eq!(translated, &detail, "{source:?}");
                // What is enforced compiles and keeps ECMAScript's ASCII digits.
                regex::Regex::new(translated).unwrap();
            }
            ("unsupported", PatternAdmission::Unsupported { .. }) | ("invalid", PatternAdmission::Invalid { .. }) => {}
            _ => panic!("{source:?}: documented {outcome}, admitted as {admission:?}"),
        }
        rows += 1;
    }
    assert!(rows >= 30, "{rows} rows read");
}

#[test]
fn translated_digit_and_word_classes_are_ascii_only() {
    let PatternAdmission::Enforced { translated } = admit_pattern(r"^\d+\w$") else { panic!() };
    let regex = regex::Regex::new(&translated).unwrap();
    assert!(regex.is_match("12a"));
    // Arabic-Indic digits and an accented letter are \d / \w in Rust's
    // Unicode default but not in ECMAScript.
    assert!(!regex.is_match("١٢a"));
    assert!(!regex.is_match("12é"));
}
