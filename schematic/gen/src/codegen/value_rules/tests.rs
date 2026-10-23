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
    assert_eq!(findings, [
        at("ListItems", Severity::Warning, "query.id", "format"),
        at("ListItems", Severity::Warning, "query.code", "pattern"),
        at("CreateTool", Severity::Warning, "body (component `Search` at `#/properties/depth`)", "maximum"),
        at("CreateTool", Severity::Warning, "body (component `Search` at `#/properties/type`)", "enum"),
        at("CreateTool", Severity::Warning, "body (component `Code` at `#/properties/type`)", "enum"),
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
