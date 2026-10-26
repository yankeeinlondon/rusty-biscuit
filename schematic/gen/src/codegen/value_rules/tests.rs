use schematic_define::constraints::Constraints;
use schematic_define::{
    ApiRequest, Endpoint, EndpointParams, ParamDef, ParamStyle, QueryParamType, RestApi, RestMethod, SchemaComponent,
    SuccessShape, TypeRef,
};
use serde_json::json;

use super::*;
use crate::codegen::request_structs::format_generated_code;
use crate::output::assemble::assemble_api_module;

fn endpoint(id: &str, method: RestMethod, path: &str) -> Endpoint {
    let mut endpoint = Endpoint::get("Fixture", "/").describes("Fixture").returns_empty().build().unwrap();
    endpoint.id = id.to_string();
    endpoint.method = method;
    endpoint.path = path.to_string();
    endpoint.response = vec![SuccessShape::empty()];
    endpoint
}

fn param(name: &str, param_type: QueryParamType, rules: Option<Constraints>) -> ParamDef {
    ParamDef {
        constraints: rules.map(Box::new),
        deprecated: false,
        rust_name: None,
        secret: false,
        pager_managed: false,
        name: name.to_string(),
        required: false,
        description: None,
        param_type,
        explode: false,
        style: ParamStyle::Form,
    }
}

fn api(endpoints: Vec<Endpoint>, components: Vec<(&str, serde_json::Value)>) -> RestApi {
    let mut api = RestApi::builder(&schematic_define::DefinitionContext::types_in("::fixture"), "RulesApi", "https://example.test")
        .describes("Rules")
        .no_auth()
        .build()
        .unwrap();
    api.endpoints = endpoints;
    api.schemas = components
        .into_iter()
        .map(|(name, schema)| SchemaComponent { name: name.to_string(), rust_path: Some(name.to_string()), schema })
        .collect();
    api
}

fn json_body(id: &str, component: &str) -> Endpoint {
    let mut create = endpoint(id, RestMethod::Post, "/items");
    create.request = Some(ApiRequest::json_type(component));
    create
}

fn module_code(api: &RestApi) -> String {
    format_generated_code(&assemble_api_module(api)).expect("generated code parses")
}

fn squash(code: &str) -> String {
    code.split_whitespace().collect()
}

#[test]
fn a_request_without_rules_keeps_its_direct_serialization_and_gets_no_validate() {
    let mut list = endpoint("ListItems", RestMethod::Get, "/items");
    list.params = Some(EndpointParams { query: vec![param("q", QueryParamType::String, None)], ..EndpointParams::default() });
    let api = api(
        vec![list, json_body("CreateItem", "Item")],
        // A width bound and a unit enum's value set are the Rust type's own.
        vec![("Item", json!({"type": "object", "properties": {
            "count": {"type": "integer", "format": "uint32", "minimum": 0},
            "kind": {"$ref": "#/components/schemas/Kind"},
        }})), ("Kind", json!({"type": "string", "enum": ["a", "b"]}))],
    );
    let code = module_code(&api);
    assert!(!code.contains("fn validate"), "{code}");
    assert!(!code.contains("_value_rules"), "{code}");
    assert!(squash(&code).contains(&squash("serde_json::to_string(&self.body)")), "{code}");
    assert!(api_findings(&api).is_empty());
}

#[test]
fn parameter_rules_are_checked_as_typed_values_in_declaration_order() {
    let mut list = endpoint("ListItems", RestMethod::Get, "/items/{id}");
    list.params = Some(EndpointParams {
        path: vec![ParamDef { required: true, ..param("id", QueryParamType::Integer, Some(Constraints::new().minimum(1))) }],
        query: vec![
            param("name", QueryParamType::String, Some(Constraints::new().max_length(5))),
            param(
                "ids",
                QueryParamType::Array(Box::new(QueryParamType::Integer)),
                Some(Constraints::new().max_items(3).items(Constraints::new().multiple_of("0.5"))),
            ),
        ],
        header: vec![param("X-Tag", QueryParamType::String, Some(Constraints::new().pattern(r"^\d+$")))],
        cookie: vec![param("theme", QueryParamType::String, Some(Constraints::new().allowed_values(["dark"])))],
    });
    let code = squash(&module_code(&api(vec![list], vec![])));
    let order = ["PathSegment::Name(\"path\"),", "PathSegment::Name(\"query\"),schematic_runtime::validate::PathSegment::Name(\"name\")",
        "check_array_len(value.len()", "PathSegment::Index(index)", "PathSegment::Name(\"header\")", "PathSegment::Name(\"cookie\")"];
    let validate = &code[code.find("pubfnvalidate(").expect("validate is generated")..];
    let mut at = 0;
    for needle in order {
        let found = validate[at..].find(needle).unwrap_or_else(|| panic!("{needle} after {at}: {validate}"));
        at += found + needle.len();
    }
    assert!(code.contains("check_number(*value,&rules_api_value_rules::LIST_ITEMS_REQUEST_PATH_ID"), "{code}");
    // The ASCII meaning of `\d` is spelled out, and numbers stay text.
    assert!(code.contains(r#"Pattern=Pattern::new("^[0-9]+$")"#), "{code}");
    assert!(code.contains(r#".multiple_of("0.5")"#), "{code}");
    // `into_parts` runs the same checks before it builds the path.
    let into_parts = &code[code.find("fninto_parts(").unwrap()..];
    assert!(into_parts.find("violations.into_result()").unwrap() < into_parts.find("letpath=").unwrap(), "{into_parts}");
}

#[test]
fn a_shared_recursive_component_is_emitted_once_and_refers_to_its_own_plan() {
    let node = json!({"type": "object", "properties": {
        "name": {"type": "string", "minLength": 1},
        "children": {"type": "array", "items": {"$ref": "#/components/schemas/Node"}},
        "parent": {"anyOf": [{"$ref": "#"}, {"type": "null"}]},
        "big": {"type": "integer", "maximum": 1.8446744073709552e19, "x-schematic-exact": {"maximum": "18446744073709551616"}},
    }});
    let api = api(vec![json_body("CreateNode", "Node"), json_body("UpdateNode", "Node")], vec![("Node", node)]);
    let code = squash(&module_code(&api));
    assert_eq!(code.matches("staticCOMPONENT_0_NODE:Plan").count(), 1, "{code}");
    let component = &code[code.find("staticCOMPONENT_0_NODE:Plan").unwrap()..];
    assert!(component.contains("Plan::reference(&COMPONENT_0_NODE)"), "{component}");
    assert!(component.contains("Plan::reference(&Plan::reference(&COMPONENT_0_NODE)).nullable()"), "{component}");
    assert!(component.contains(r#".maximum("18446744073709551616")"#), "{component}");
    assert_eq!(code.matches("pubfnvalidate(").count(), 2);
    assert!(code.contains("serde_json::to_string(&body)"), "the checked value is the one sent: {code}");
}

#[test]
fn a_nullable_type_keeps_its_enum_on_the_plan_null_is_checked_against() {
    // A nullable type (2020-12 `type` array or OpenAPI 3.0 `nullable`) puts
    // the enum on the nullable plan itself, where the runtime checks null
    // against it; a null alternative puts the member's enum behind the
    // reference that null skips.
    let item = json!({"type": "object", "properties": {
        "status": {"type": ["string", "null"], "enum": ["ok"]},
        "legacy": {"type": "string", "nullable": true, "enum": ["ok"]},
        "option": {"anyOf": [{"type": "string", "enum": ["ok"]}, {"type": "null"}]},
    }});
    let api = api(vec![json_body("CreateItem", "Item")], vec![("Item", item)]);
    let code = squash(&module_code(&api));
    let own = r#"Plan::new().nullable().allowed_values(&[Literal::String("ok")])"#;
    for property in ["status", "legacy"] {
        assert!(code.contains(&format!(r#"Property::new("{property}",&{own}"#)), "{property}: {code}");
    }
    assert!(
        code.contains(r#"Property::new("option",&Plan::reference(&Plan::new().allowed_values(&[Literal::String("ok")]),).nullable()"#),
        "{code}"
    );
}

#[test]
fn a_one_member_all_of_is_its_reference_and_keeps_the_rule_beside_it() {
    // Schemars' OpenAPI 3.0 form of `extend("enum" = [..])` on a field typed
    // by a model; a two-member `allOf` stays a composition.
    let owner = json!({"type": "object", "properties": {"name": {"type": "string", "maxLength": 10}}});
    let item = json!({"type": "object", "properties": {
        "owner": {"allOf": [{"$ref": "#/components/schemas/Owner"}], "enum": [{"name": "boss"}]},
    }});
    let api = api(vec![json_body("CreateItem", "Item")], vec![("Item", item), ("Owner", owner.clone())]);
    let code = squash(&module_code(&api));
    assert!(api_findings(&api).is_empty(), "{:?}", api_findings(&api).iter().map(|(_, finding)| finding.message()).collect::<Vec<_>>());
    let property = &code[code.find(r#"Property::new("owner""#).expect("owner is planned")..];
    assert!(property.starts_with(r#"Property::new("owner",&Plan::reference(&Plan::reference(&COMPONENT_1_OWNER)).allowed_values("#), "{property}");
    assert!(code.contains(r#"Property::new("name",&Plan::new().max_length(10u64))"#), "{code}");

    let composed = json!({"type": "object", "properties": {
        "owner": {"allOf": [{"$ref": "#/components/schemas/Owner"}, {"type": "object"}]},
    }});
    let findings = api_findings(&self::api(vec![json_body("CreateItem", "Item")], vec![("Item", composed), ("Owner", owner)]));
    assert!(findings.iter().any(|(_, finding)| finding.severity == Severity::Error && finding.keyword == "maxLength"), "{:?}", findings.len());
}

#[test]
fn unenforced_rules_are_findings_with_their_severity() {
    let mut list = endpoint("ListItems", RestMethod::Get, "/items");
    list.params = Some(EndpointParams {
        query: vec![
            param("id", QueryParamType::String, Some(Constraints::new().format("uuid"))),
            param("code", QueryParamType::String, Some(Constraints::new().unenforced_pattern(r"^\s+$"))),
            param("count", QueryParamType::Integer, Some(Constraints::new().format("int64"))),
        ],
        ..EndpointParams::default()
    });
    let mut wrapped = endpoint("Wrap", RestMethod::Post, "/wrap");
    wrapped.request = Some(ApiRequest::Json(TypeRef::Named("Wrapper".into())));
    let api = api(
        vec![list, json_body("CreateTool", "Tools"), wrapped],
        vec![
            // A union component: a Rust enum of request structs.
            ("Tools", json!({"anyOf": [{"$ref": "#/components/schemas/Search"}, {"$ref": "#/components/schemas/Code"}]})),
            ("Search", json!({"type": "object", "properties": {"type": {"type": "string", "enum": ["search"]}, "depth": {"type": "integer", "maximum": 3}}})),
            ("Code", json!({"type": "object", "properties": {"type": {"type": "string", "enum": ["code"]}}})),
            // A composition inside a property.
            ("Wrapper", json!({"type": "object", "properties": {"value": {"oneOf": [{"type": "string", "maxLength": 3}, {"type": "integer"}]}}})),
            // A tagged enum: its tags are the Rust type's own.
            ("Shape", json!({"oneOf": [{"type": "object", "properties": {"kind": {"type": "string", "const": "circle"}}}]})),
        ],
    );
    let findings: Vec<(Option<String>, Severity, String, String)> = api_findings(&api)
        .into_iter()
        .map(|(endpoint, finding)| (endpoint, finding.severity, finding.place, finding.keyword))
        .collect();
    let at = |endpoint: &str, severity, place: &str, keyword: &str| {
        (Some(endpoint.to_string()), severity, place.to_string(), keyword.to_string())
    };
    // Without the API's opt-in, a rule inside a union component's member is
    // an error like any other composition rule.
    assert_eq!(findings, [
        at("ListItems", Severity::Warning, "query.id", "format"),
        at("ListItems", Severity::Warning, "query.code", "pattern"),
        at("CreateTool", Severity::Error, "body (component `Search` at `#/properties/depth`)", "maximum"),
        at("CreateTool", Severity::Error, "body (component `Search` at `#/properties/type`)", "enum"),
        at("CreateTool", Severity::Error, "body (component `Code` at `#/properties/type`)", "enum"),
        at("Wrap", Severity::Error, "body (component `Wrapper` at `#/properties/value/oneOf/0`)", "maxLength"),
    ]);

    // The request docs label what is not checked.
    let code = module_code(&api);
    assert!(code.contains("Not checked: `format` `uuid` (an annotation only)."), "{code}");
    assert!(code.contains("`pattern` (outside the enforced subset)"), "{code}");
    assert!(code.contains("/// ## Value rules"), "{code}");
}

#[test]
fn a_component_pattern_outside_the_subset_is_reported_once_for_the_api() {
    let item = json!({"type": "object", "properties": {"code": {"type": "string", "pattern": r"^\s+$"}, "id": {"type": "string", "pattern": r"^\d+$"}}});
    let api = api(vec![json_body("CreateItem", "Item"), json_body("UpdateItem", "Item")], vec![("Item", item)]);
    let findings = api_findings(&api);
    assert_eq!(findings.len(), 1, "{findings:?}");
    assert_eq!(findings[0].0, None);
    assert_eq!((findings[0].1.place.as_str(), findings[0].1.keyword.as_str()), ("component `Item`.code", "pattern"));
    let code = module_code(&api);
    assert!(squash(&code).contains(r#"Pattern=Pattern::new("^[0-9]+$")"#), "the admitted pattern is still enforced: {code}");
}

#[test]
fn snake_case_keeps_acronyms_together() {
    assert_eq!(snake_case("OpenAI"), "open_ai");
    assert_eq!(snake_case("GitHub"), "git_hub");
    assert_eq!(snake_case("HTTPServer2Api"), "http_server2_api");
    assert_eq!(snake_case("x-tag"), "x_tag");
    assert_eq!(screaming_case("ListItemsRequest"), "LIST_ITEMS_REQUEST");
}

#[test]
fn pruning_keeps_declared_names_while_an_extra_value_plan_survives() {
    let limited = Node { max_length: Some(3), ..Node::default() };
    let unchecked_label = ("label".to_string(), Node { nullable: true, ..Node::default() });
    let mixed = Node { properties: vec![unchecked_label.clone()], values: Some(Box::new(limited.clone())), ..Node::default() };
    let pruned = prune(&mixed, &[]).expect("the extra-value plan checks something");
    assert_eq!(pruned.properties, [("label".to_string(), Node::default())]);
    assert_eq!(pruned.values.as_deref(), Some(&limited));

    // Without an extra-value plan, membership decides nothing.
    let declared = Node { properties: vec![unchecked_label, ("code".to_string(), limited.clone())], ..Node::default() };
    let pruned = prune(&declared, &[]).expect("`code` checks something");
    assert_eq!(pruned.properties, [("code".to_string(), limited)]);
}

#[test]
fn an_unchecked_declared_property_stays_out_of_the_extra_value_plan_at_every_level() {
    let mixed = json!({
        "type": "object",
        "properties": {"label": {"type": "string"}},
        "additionalProperties": {"type": "string", "maxLength": 3},
    });
    let outer = json!({"type": "object", "properties": {
        "inner": {"$ref": "#/components/schemas/Mixed"},
        "note": {"type": "string"},
    }});
    let mut upload = endpoint("Upload", RestMethod::Post, "/upload");
    upload.request = Some(ApiRequest::FormData {
        fields: vec![schematic_define::FormField::json("meta", TypeRef::Named("Mixed".into()))],
    });
    let api = api(vec![json_body("CreateOuter", "Outer"), upload], vec![("Mixed", mixed), ("Outer", outer)]);
    let code = squash(&module_code(&api));
    let component = &code[code.find("staticCOMPONENT_").expect("a component plan is emitted")..];
    assert!(
        component.contains(r#".properties(&[Property::new("label",&Plan::new())]).values(&Plan::new().max_length(3u64))"#),
        "{component}"
    );
    // `Outer` has no extra-value plan, so its unchecked `note` is pruned.
    assert!(!code.contains(r#"Property::new("note""#), "{code}");
    assert!(code.contains("UPLOAD_REQUEST_FORM_META"), "the multipart JSON field is checked: {code}");
}

/// The `(endpoint, severity, place, keyword)` of every finding in `api`.
fn located_findings(api: &RestApi) -> Vec<(Option<String>, Severity, String, String)> {
    api_findings(api).into_iter().map(|(endpoint, finding)| (endpoint, finding.severity, finding.place, finding.keyword)).collect()
}

/// `api` whose `CreateItem` posts the component `Item` as its JSON body and
/// whose `Upload` sends it as the multipart JSON field `meta`.
fn body_and_form_field(item: serde_json::Value) -> RestApi {
    let mut upload = endpoint("Upload", RestMethod::Post, "/upload");
    upload.request = Some(ApiRequest::FormData { fields: vec![schematic_define::FormField::json("meta", TypeRef::Named("Item".into()))] });
    api(vec![json_body("CreateItem", "Item"), upload], vec![("Item", item)])
}

#[test]
fn a_rule_in_positional_items_is_a_generation_error_for_body_and_form_field_alike() {
    for keyword in ["items", "prefixItems"] {
        let item = json!({"type": "object", "properties": {"pair": {"type": "array", keyword: [{"type": "string", "maxLength": 3}, {"type": "integer"}]}}});
        let api = body_and_form_field(item);
        let place = format!("component `Item` at `#/properties/pair/{keyword}/0`");
        assert_eq!(located_findings(&api), [
            (Some("CreateItem".to_string()), Severity::Error, format!("body ({place})"), "maxLength".to_string()),
            (Some("Upload".to_string()), Severity::Error, format!("body.meta ({place})"), "maxLength".to_string()),
        ], "{keyword}");
        let finding = &api_findings(&api)[0].1;
        assert_eq!(
            finding.message(),
            format!(
                "body ({place}) `maxLength` is not enforced: beneath the `{keyword}` at `#/properties/pair` of component `Item`, \
                 which the client does not traverse; the client cannot check it, so remove the rule or restructure the schema"
            )
        );
    }
}

#[test]
fn a_rule_beneath_a_nested_composition_is_an_error_for_body_and_form_field_alike() {
    let item = json!({"type": "object", "properties": {"value": {"anyOf": [{"type": "string", "maxLength": 3}, {"type": "integer"}]}}});
    let place = "component `Item` at `#/properties/value/anyOf/0`";
    assert_eq!(located_findings(&body_and_form_field(item)), [
        (Some("CreateItem".to_string()), Severity::Error, format!("body ({place})"), "maxLength".to_string()),
        (Some("Upload".to_string()), Severity::Error, format!("body.meta ({place})"), "maxLength".to_string()),
    ]);
}

#[test]
fn item_objects_and_nullable_any_of_stay_enforced_without_findings() {
    let item = json!({"type": "object", "properties": {
        "tags": {"type": "array", "items": {"type": "string", "maxLength": 3}},
        "note": {"anyOf": [{"type": "string", "maxLength": 5}, {"enum": [null], "nullable": true}]},
    }});
    let api = body_and_form_field(item);
    assert_eq!(located_findings(&api), []);
    let code = squash(&module_code(&api));
    assert!(code.contains(".items(&Plan::new().max_length(3u64))"), "{code}");
    assert!(code.contains(".max_length(5u64)"), "{code}");
    assert!(code.contains("UPLOAD_REQUEST_FORM_META"), "the multipart JSON field is checked: {code}");
}

/// `api` with the author's opt-in for unchecked compositions.
fn opted_in(mut api: RestApi) -> RestApi {
    api.unchecked_compositions = true;
    api
}

/// Asserts that `item`'s one unenforceable rule, `keyword` at `pointer` of
/// the component `Item`, is reported for the JSON body and the multipart
/// JSON field alike, with `without` and then `with` the API's opt-in.
fn assert_both_entry_points(case: &str, item: serde_json::Value, pointer: &str, keyword: &str, without: Severity, with: Severity) {
    for (api, severity) in [(body_and_form_field(item.clone()), without), (opted_in(body_and_form_field(item)), with)] {
        let place = format!("component `Item` at `#{pointer}`");
        assert_eq!(located_findings(&api), [
            (Some("CreateItem".to_string()), severity, format!("body ({place})"), keyword.to_string()),
            (Some("Upload".to_string()), severity, format!("body.meta ({place})"), keyword.to_string()),
        ], "{case}, opted in: {}", api.unchecked_compositions);
    }
}

fn limited() -> serde_json::Value {
    json!({"type": "string", "maxLength": 3})
}

fn property(value: serde_json::Value) -> serde_json::Value {
    json!({"type": "object", "properties": {"value": value}})
}

#[test]
fn a_composition_rule_is_an_error_without_the_opt_in_and_a_warning_with_it_wherever_it_sits() {
    let union = |keyword: &str| json!({keyword: [limited(), {"type": "integer"}]});
    let cases = [
        ("root anyOf", union("anyOf"), "/anyOf/0"),
        ("root oneOf", union("oneOf"), "/oneOf/0"),
        ("nested anyOf", property(union("anyOf")), "/properties/value/anyOf/0"),
        ("nested oneOf", property(union("oneOf")), "/properties/value/oneOf/0"),
        ("two-member allOf", property(json!({"allOf": [limited(), {"type": "string"}]})), "/properties/value/allOf/0"),
        ("not", property(json!({"not": limited()})), "/properties/value/not"),
        ("map-value union", json!({"type": "object", "additionalProperties": union("anyOf")}), "/additionalProperties/anyOf/0"),
    ];
    for (case, item, pointer) in cases {
        assert_both_entry_points(case, item, pointer, "maxLength", Severity::Error, Severity::Warning);
    }
}

#[test]
fn a_rule_beneath_positional_items_stays_an_error_with_the_opt_in_even_inside_a_composition() {
    let positional = |keyword: &str| json!({"type": "array", keyword: [limited(), {"type": "integer"}]});
    let cases = [
        ("positional items", property(positional("items")), "/properties/value/items/0"),
        ("prefixItems", property(positional("prefixItems")), "/properties/value/prefixItems/0"),
        ("positional items inside anyOf", property(json!({"anyOf": [positional("items"), {"type": "integer"}]})), "/properties/value/anyOf/0/items/0"),
        ("prefixItems inside oneOf", json!({"oneOf": [positional("prefixItems"), {"type": "integer"}]}), "/oneOf/0/prefixItems/0"),
        ("anyOf inside positional items", property(json!({"type": "array", "items": [{"anyOf": [limited(), {"type": "integer"}]}]})), "/properties/value/items/0/anyOf/0"),
    ];
    for (case, item, pointer) in cases {
        assert_both_entry_points(case, item, pointer, "maxLength", Severity::Error, Severity::Error);
    }
}

#[test]
fn a_rule_beneath_another_untraversed_keyword_stays_an_error_with_the_opt_in() {
    for keyword in ["additionalItems", "contains", "unevaluatedItems", "propertyNames", "unevaluatedProperties", "if", "then", "else"] {
        let item = property(json!({keyword: limited()}));
        assert_both_entry_points(keyword, item, &format!("/properties/value/{keyword}"), "maxLength", Severity::Error, Severity::Error);
    }
    for keyword in ["patternProperties", "dependentSchemas", "dependencies"] {
        let item = property(json!({keyword: {"x": limited()}}));
        assert_both_entry_points(keyword, item, &format!("/properties/value/{keyword}/x"), "maxLength", Severity::Error, Severity::Error);
    }
}

#[test]
fn a_rule_beside_an_unresolved_reference_stays_an_error_with_the_opt_in_even_inside_a_composition() {
    let beside = json!({"$ref": "#/components/schemas/Gone", "maxLength": 3});
    for (case, item, pointer) in [
        ("plain", property(beside.clone()), "/properties/value"),
        ("inside anyOf", property(json!({"anyOf": [beside, {"type": "integer"}]})), "/properties/value/anyOf/0"),
    ] {
        assert_both_entry_points(case, item.clone(), pointer, "maxLength", Severity::Error, Severity::Error);
        let finding = api_findings(&opted_in(body_and_form_field(item))).remove(0).1;
        assert!(finding.reason.starts_with("beside the unresolved reference \"#/components/schemas/Gone\""), "{case}: {finding:?}");
    }
}

#[test]
fn a_component_reached_beneath_a_composition_and_positional_items_is_reported_once_as_an_error() {
    let item = json!({"type": "object", "properties": {
        "choice": {"anyOf": [{"$ref": "#/components/schemas/Code"}, {"type": "integer"}]},
        "pair": {"type": "array", "prefixItems": [{"$ref": "#/components/schemas/Code"}]},
    }});
    let api = opted_in(api(vec![json_body("CreateItem", "Item")], vec![("Item", item), ("Code", limited())]));
    assert_eq!(located_findings(&api), [
        (Some("CreateItem".to_string()), Severity::Error, "body (component `Code` at `#`)".to_string(), "maxLength".to_string()),
    ]);
    assert!(api_findings(&api)[0].1.reason.contains("`prefixItems`"), "{:?}", api_findings(&api));
}

#[test]
fn supported_traversal_stays_enforced_without_findings_with_or_without_the_opt_in() {
    let item = json!({"type": "object", "properties": {
        "tags": {"type": "array", "items": {"type": "object", "properties": {"name": limited()}}},
        "note": {"anyOf": [{"type": "string", "maxLength": 5}, {"type": "null"}]},
        "code": {"allOf": [{"$ref": "#/components/schemas/Code"}], "minLength": 1},
    }});
    for opt_in in [false, true] {
        let mut api = api(vec![json_body("CreateItem", "Item")], vec![("Item", item.clone()), ("Code", json!({"type": "string", "maxLength": 7}))]);
        let mut upload = endpoint("Upload", RestMethod::Post, "/upload");
        upload.request = Some(ApiRequest::FormData { fields: vec![schematic_define::FormField::json("meta", TypeRef::Named("Item".into()))] });
        api.endpoints.push(upload);
        api.unchecked_compositions = opt_in;
        assert_eq!(located_findings(&api), [], "opted in: {opt_in}");
        let code = squash(&module_code(&api));
        for plan in [".max_length(3u64)", ".max_length(5u64)", ".max_length(7u64)", ".min_length(1u64)"] {
            assert!(code.contains(plan), "{plan}, opted in: {opt_in}: {code}");
        }
        assert!(code.contains("UPLOAD_REQUEST_FORM_META"), "the multipart JSON field is checked: {code}");
    }
}

#[test]
fn the_composition_messages_name_the_remedies_and_the_opt_in_discloses_the_rule_in_rustdoc() {
    let item = property(json!({"anyOf": [limited(), {"type": "integer"}]}));
    let place = "body (component `Item` at `#/properties/value/anyOf/0`) `maxLength`";
    let refused = api_findings(&body_and_form_field(item.clone())).remove(0).1;
    assert_eq!(
        refused.message(),
        format!(
            "{place} is not enforced: beneath the `anyOf` at `#/properties/value` of component `Item`; the client cannot \
             check it, so remove the rule or the composition, or accept unchecked compositions for the API with \
             `accept_unchecked_compositions`"
        )
    );

    let api = opted_in(body_and_form_field(item));
    let accepted = api_findings(&api).remove(0).1;
    let reason = "beneath the `anyOf` at `#/properties/value` of component `Item`, a composition whose members are not \
                  checked; the API accepts unchecked compositions";
    assert_eq!(accepted.message(), format!("{place} is not enforced: {reason}"));
    let code = module_code(&api);
    let create = &code[code.find("pub struct CreateItemRequest").expect("request struct")..];
    let docs = &code[..code.find("pub struct CreateItemRequest").unwrap()];
    let docs = &docs[docs.rfind("## Value rules").expect("the request documents its value rules")..];
    assert!(docs.contains("This request declares value rules that the client does not check."), "{docs}");
    assert!(docs.contains(&format!("/// - body (component `Item` at `#/properties/value/anyOf/0`) `maxLength`: {reason}")), "{docs}");
    assert!(!squash(create).contains("fnvalidate("), "no check is emitted for the rule: {create}");
    let upload = &code[..code.find("pub struct UploadRequest").expect("form request struct")];
    assert!(upload[upload.rfind("## Value rules").unwrap()..].contains("/// - body.meta (component `Item`"), "{upload}");
}
