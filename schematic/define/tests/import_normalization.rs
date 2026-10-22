#![cfg(feature = "openapi")]

use biscuit_file::serde_yaml_ng;
use schematic_define::openapi::{OpenApiImport, OpenApiSource};
use serde_json::{Value, json};

fn fixture() -> Value {
    json!({"openapi":"3.0.0","info":{"title":"QueryFixture","version":"1"},"paths":{
        "/items?beta=true":{"post":{"operationId":"createItems","parameters":[
            {"name":"limit","in":"query","schema":{"type":"integer"}}
        ],"responses":{"200":{"description":"ok"}}}}
    }})
}

fn import(value: &Value, yaml: bool) -> Result<schematic_define::openapi::OpenApiImportResult, schematic_define::openapi::OpenApiError> {
    let source = if yaml { OpenApiSource::yaml(serde_yaml_ng::to_string(value).unwrap()) }
        else { OpenApiSource::json(value.to_string()) };
    OpenApiImport::new(source).build()
}

fn matrix(yaml: bool, provider_source: &str) {
    let (source, _) = schematic_define::openapi::import::clamp_numeric_bounds(provider_source);
    let provider: Value = serde_yaml_ng::from_str(&source).unwrap();
    let path = "/responses/{response_id}/input_items?beta=true";
    // The response names this component; an import whose references dangle fails.
    let response = "BetaResponseItemList";
    let input = json!({"openapi":"3.0.0", "info":provider["info"], "paths":{path:provider["paths"][path]},
        "components":{"schemas":{response:provider["components"]["schemas"][response]}}});
    let result = import(&input, yaml).unwrap();
    let endpoint = &result.api.endpoints[0];
    assert_eq!(endpoint.path, "/responses/{response_id}/input_items");
    let params = &endpoint.params.as_ref().unwrap().query;
    assert_eq!(params.len(), 5);
    assert!(params.iter().any(|p| p.name == "beta" && !p.required));
    for field in ["paths", "parameters", "name", "in", "required", "schema", "type", "requestBody"] {
        for shape in ["absent", "null", "wrong", "mixed", "all_wrong", "empty"] {
            let mut edited = input.clone();
            let operation = "/paths/~1responses~1{response_id}~1input_items?beta=true/get";
            let (parent, key) = match field {
                "paths" => ("".to_string(), "paths"),
                "parameters" => (operation.to_string(), "parameters"),
                "requestBody" => (operation.to_string(), "requestBody"),
                "type" => (format!("{operation}/parameters/1/schema"), field),
                _ => (format!("{operation}/parameters/1"), field),
            };
            let parent = edited.pointer_mut(&parent).unwrap().as_object_mut().unwrap();
            if shape == "absent" { parent.remove(key); }
            else {
                let value = match (field, shape) {
                    (_, "null") => Value::Null,
                    (_, "wrong") => json!(123),
                    ("paths", "mixed") => json!({"/valid":{"get":{"responses":{"200":{"description":"ok"}}}},"/bad":123}),
                    ("paths", "all_wrong") => json!({"/bad":123}),
                    ("paths", "empty") => json!({}),
                    ("parameters", "mixed") => json!([{"name":"limit","in":"query","schema":{"type":"integer"}},123]),
                    ("parameters", "all_wrong") => json!([123]),
                    ("parameters", "empty") => json!([]),
                    ("schema", "empty") | ("requestBody", "empty") => json!({}),
                    (_, "mixed") => json!(["query",123]),
                    (_, "all_wrong") => json!([123]),
                    (_, "empty") => json!(""),
                    _ => unreachable!(),
                };
                parent.insert(key.to_string(), value);
            }
            let result = import(&edited, yaml);
            match (field, shape) {
                ("parameters", "absent" | "empty") => {
                    let result = result.unwrap();
                    assert_eq!(result.api.endpoints[0].params.as_ref().unwrap().query.len(), 1);
                }
                ("requestBody", "absent") => assert_eq!(result.unwrap().api, import(&input, yaml).unwrap().api),
                ("required", "absent") => {
                    let result = result.unwrap();
                    assert!(!result.api.endpoints[0].params.as_ref().unwrap().query.iter().find(|p| p.name == "limit").unwrap().required);
                }
                ("type", "absent") | ("schema", "empty") => {
                    let result = result.unwrap();
                    assert_eq!(result.api.endpoints[0].params.as_ref().unwrap().query.iter().find(|p| p.name == "limit").unwrap().param_type, schematic_define::params::QueryParamType::Json);
                }
                ("paths", "empty") => {
                    // Empty imports retain the existing foundation contract; no endpoint is invented.
                    assert!(result.unwrap().api.endpoints.is_empty());
                }
                _ => assert!(result.is_err(), "{field} {shape} accepted ({yaml})"),
            }
        }
    }
    let mut required = input.clone();
    required["paths"][path]["get"]["parameters"][1]["required"] = json!(true);
    assert!(import(&required, yaml).unwrap().api.endpoints[0].params.as_ref().unwrap().query.iter().find(|p| p.name == "limit").unwrap().required);
    required["paths"][path]["get"]["parameters"][1]["required"] = json!("false");
    assert!(import(&required, yaml).is_err());
    let text = if yaml { serde_yaml_ng::to_string(&input).unwrap() } else { input.to_string() };
    if yaml {
        let quoted_type = text.replacen("type: integer", "type: 'integer'", 1);
        assert_eq!(OpenApiImport::new(OpenApiSource::yaml(quoted_type)).build().unwrap().api, result.api);
    }
    let bad = if yaml { format!("{text}\npaths: {{}}\n") } else { text.replacen("\"paths\":", "\"paths\":{},\"paths\":", 1) };
    let source = if yaml { OpenApiSource::yaml(bad) } else { OpenApiSource::json(bad) };
    assert!(OpenApiImport::new(source).build().is_err());
    for (field, value) in [("parameters", "[]"), ("name", "limit"), ("in", "query"), ("required", "false"), ("schema", "{}"), ("type", "integer"), ("requestBody", "{}")] {
        // JSON flow mappings are also native YAML syntax, preserving duplicate keys.
        let duplicate_text = if field == "requestBody" {
            let mut with_body = input.clone();
            with_body["paths"][path]["get"]["requestBody"] = json!({});
            with_body.to_string()
        } else { input.to_string() };
        let value = if ["parameters", "required", "schema", "requestBody"].contains(&field) { value.to_string() } else { format!("\"{value}\"") };
        let duplicated = duplicate_text.replacen(&format!("\"{field}\":"), &format!("\"{field}\":{value},\"{field}\":"), 1);
        let source = if yaml { OpenApiSource::yaml(duplicated) } else { OpenApiSource::json(duplicated) };
        let error = OpenApiImport::new(source).build().unwrap_err().to_string();
        assert!(error.contains("Duplicate key"), "duplicate {field} {yaml}: {error}");
    }
    let source = if yaml { OpenApiSource::yaml(format!("{text}\n---\ninvalid")) } else { OpenApiSource::json(format!("{text} garbage")) };
    assert!(OpenApiImport::new(source).build().is_err());
}

#[test]
fn json_import_normalization_matrix() { matrix(false, include_str!("../../specs/openai/openapi.yaml")); }
#[test]
fn yaml_import_normalization_matrix() { matrix(true, include_str!("../../specs/openai/openapi.yaml")); }

#[test]
fn query_expressions_reject_ambiguity_and_get_bodies() {
    for yaml in [false, true] {
        for path in ["/items?", "/items?beta", "/items?beta=true&beta=false", "/items?=true", "/items?ref={unknown}", "/items?ref=x{partial}", "/items?beta=%GG", "/items?beta=true#fragment"] {
            let mut input = fixture();
            let item = input["paths"].as_object_mut().unwrap().remove("/items?beta=true").unwrap();
            input["paths"].as_object_mut().unwrap().insert(path.to_string(), item);
            let error = import(&input, yaml).unwrap_err().to_string();
            assert!(error.contains(path), "{error}");
            assert!(error.contains("createItems"), "{error}");
        }
        let mut input = fixture();
        let mut operation = input["paths"]["/items?beta=true"]["post"].take();
        operation["requestBody"] = json!({"content":{"application/json":{"schema":{"type":"string"}}}});
        input["paths"]["/items?beta=true"].as_object_mut().unwrap().remove("post");
        input["paths"]["/items?beta=true"]["get"] = operation;
        let error = import(&input, yaml).unwrap_err().to_string();
        assert!(error.contains("GET") && error.contains("createItems") && error.contains("QueryFixture"), "{error}");
    }
}

#[test]
fn placeholder_moves_to_query_and_preserves_rust_name() {
    let input = json!({"openapi":"3.0.0","info":{"title":"GitLab","version":"1"},"paths":{
        "/projects/{id}/repository/files/{file_path}?ref={git_ref}":{"get":{"operationId":"getFile","parameters":[
            {"name":"git_ref","in":"path","required":true,"schema":{"type":"string"}}
        ],"responses":{"200":{"description":"ok"}}}}
    }});
    for yaml in [false, true] {
        let result = import(&input, yaml).unwrap();
        let endpoint = &result.api.endpoints[0];
        assert_eq!(endpoint.path, "/projects/{id}/repository/files/{file_path}");
        let param = &endpoint.params.as_ref().unwrap().query[0];
        assert_eq!(param.name, "ref");
        assert_eq!(param.rust_name.as_deref(), Some("git_ref"));
        assert!(param.required);
    }
}

#[test]
fn twins_merge_only_with_compatible_schemas_and_parameters() {
    for yaml in [false, true] {
        let mut input = fixture();
        let mut twin = input["paths"]["/items?beta=true"].clone();
        twin["post"]["operationId"] = json!("ordinaryItems");
        input["paths"]["/items"] = twin;
        assert_eq!(import(&input, yaml).unwrap().api.endpoints.len(), 1);
        input["paths"]["/items"]["post"]["parameters"][0]["schema"]["type"] = json!("string");
        assert!(import(&input, yaml).is_err());
    }
}

#[test]
fn shipped_openai_query_schema_gate_and_reimports() {
    let source = include_str!("../../specs/openai/openapi.yaml");
    let first = OpenApiImport::new(OpenApiSource::yaml(source)).build().unwrap();
    let second = OpenApiImport::new(OpenApiSource::yaml(source)).build().unwrap();
    assert_eq!(first.api, second.api);
    assert_eq!(first.models, second.models);
    let call = first.api.endpoints.iter().find(|endpoint| endpoint.id == "CreateRealtimeCall").unwrap();
    let Some(schematic_define::ApiRequest::FormData { fields }) = &call.request else {
        panic!("shipped Realtime call must preserve multipart/form-data preference");
    };
    assert_eq!(fields.iter().map(|field| field.name.as_str()).collect::<Vec<_>>(), ["sdp", "session"]);
    let model = first.models.types.iter().find_map(|model| match model {
        schematic_define::models::ModelDef::Struct(model) if model.name == "Model" => Some(model), _ => None,
    }).unwrap();
    assert_eq!(model.fields.iter().map(|field| field.name.as_str()).collect::<Vec<_>>(), ["id", "created", "object", "owned_by"]);
    for diagnostic in first.diagnostics.iter().filter(|d| d.message.contains("flattened schema:")) {
        println!("{}: {}", diagnostic.location, diagnostic.message);
    }
    assert_eq!(first.diagnostics.iter().filter(|d| d.message.contains("flattened schema:")).count(), 14);
    let merged: Vec<_> = first.api.endpoints.iter().filter(|endpoint| endpoint.params.as_ref().is_some_and(|params| params.query.iter().any(|p| p.name == "beta"))).collect();
    assert_eq!(merged.len(), 7);
    assert!(first.api.endpoints.iter().all(|endpoint| !endpoint.path.contains('?')));
}

#[test]
fn flattened_schema_gate_selects_superset_and_blocks_incompatible_twins() {
    for yaml in [false, true] {
        let mut input = fixture();
        input["components"] = json!({"schemas":{
            "Ordinary":{"type":"object","required":["value"],"properties":{"value":{"type":"string"}}},
            "Beta":{"allOf":[{"$ref":"#/components/schemas/Ordinary"},{"type":"object","properties":{"extra":{"type":"integer"}}}]}
        }});
        let mut twin = input["paths"]["/items?beta=true"].clone();
        twin["post"]["operationId"] = json!("ordinaryItems");
        input["paths"]["/items"] = twin;
        for (path, schema) in [("/items", "Ordinary"), ("/items?beta=true", "Beta")] {
            input["paths"][path]["post"]["requestBody"] = json!({"content":{"application/json":{"schema":{"$ref":format!("#/components/schemas/{schema}")}}}});
            input["paths"][path]["post"]["responses"]["200"]["content"] = json!({"application/json":{"schema":{"$ref":format!("#/components/schemas/{schema}")}}});
        }
        let result = import(&input, yaml).unwrap();
        assert_eq!(result.api.endpoints.len(), 1);
        let serialized = serde_json::to_value(&result.api.endpoints[0]).unwrap();
        assert!(serialized["request"].to_string().contains("Beta"));
        assert!(serialized["response"].to_string().contains("Beta"));
        input["components"]["schemas"]["Beta"] = json!({"type":"object","properties":{"value":{"type":"integer"}}});
        let error = import(&input, yaml).unwrap_err().to_string();
        assert!(error.contains("schema is not a superset"), "{error}");
    }
}

#[test]
fn declaration_duplicates_null_operations_and_query_conflicts_are_rejected() {
    for yaml in [false, true] {
        for mutation in ["duplicate", "conflict", "null_operation", "empty_path", "null_path"] {
            let mut input = fixture();
            match mutation {
                "duplicate" => {
                    let parameter = input["paths"]["/items?beta=true"]["post"]["parameters"][0].clone();
                    input["paths"]["/items?beta=true"]["post"]["parameters"].as_array_mut().unwrap().push(parameter);
                }
                "conflict" => input["paths"]["/items?beta=true"]["post"]["parameters"].as_array_mut().unwrap().push(json!({"name":"beta","in":"query","schema":{"type":"string"}})),
                "null_operation" => input["paths"]["/items?beta=true"]["post"] = Value::Null,
                "null_path" => input["paths"]["/items?beta=true"] = Value::Null,
                "empty_path" => { let item = input["paths"].as_object_mut().unwrap().remove("/items?beta=true").unwrap(); input["paths"][""] = item; }
                _ => unreachable!(),
            }
            assert!(import(&input, yaml).is_err(), "{mutation} {yaml}");
        }
        let mut input = fixture();
        input["paths"]["/items?beta=true"]["post"]["parameters"].as_array_mut().unwrap().push(json!({"name":"limit","in":"header","schema":{"type":"integer"}}));
        let result = import(&input, yaml).unwrap();
        assert_eq!(result.api.endpoints[0].params.as_ref().unwrap().header.len(), 1);
    }
}

#[test]
fn shipped_unfolded_circle_import_corpus() {
    let source = include_str!("../../specs/unfolded-circle/UCR-core-openapi.yaml");
    let result = OpenApiImport::new(OpenApiSource::yaml(source)).build().unwrap();
    assert!(!result.api.endpoints.is_empty());
    assert!(result.api.endpoints.iter().all(|endpoint| !endpoint.path.contains('?')));
}

#[test]
fn gitea_recursive_twin_becomes_optional_caller_control() {
    let input = json!({"openapi":"3.0.0","info":{"title":"Gitea","version":"1"},"paths":{
        "/repos/{owner}/{repo}/git/trees/{sha}":{"get":{"operationId":"GetGitTree","responses":{"200":{"description":"ok","content":{"application/json":{"schema":{"type":"object"}}}}}}},
        "/repos/{owner}/{repo}/git/trees/{sha}?recursive=true":{"get":{"operationId":"GetGitTreeRecursive","responses":{"200":{"description":"ok","content":{"application/json":{"schema":{"type":"object"}}}}}}}
    }});
    for yaml in [false, true] {
        let result = import(&input, yaml).unwrap();
        assert_eq!(result.api.endpoints.len(), 1);
        assert_eq!(result.api.endpoints[0].id, "GetGitTree");
        assert_eq!(result.api.endpoints[0].path, "/repos/{owner}/{repo}/git/trees/{sha}");
        let param = &result.api.endpoints[0].params.as_ref().unwrap().query[0];
        assert_eq!(param.name, "recursive");
        assert!(!param.required);
        assert_eq!(param.param_type, schematic_define::params::QueryParamType::Boolean);
    }
}

#[test]
fn schema_gate_rejects_new_restrictions() {
    for (keyword, value) in [("required", json!(["value"])), ("minimum", json!(1)), ("maximum", json!(1)), ("minLength", json!(1)), ("maxLength", json!(1)), ("additionalProperties", json!(false)), ("pattern", json!("x"))] {
        let mut input = fixture();
        let mut twin = input["paths"]["/items?beta=true"].clone();
        twin["post"]["operationId"] = json!("ordinaryItems");
        input["paths"]["/items"] = twin;
        let ordinary = json!({"type":"object","properties":{"value":{"type":"string"}}});
        let mut candidate = ordinary.clone();
        candidate[keyword] = value;
        input["paths"]["/items"]["post"]["responses"]["200"]["content"] = json!({"application/json":{"schema":ordinary}});
        input["paths"]["/items?beta=true"]["post"]["responses"]["200"]["content"] = json!({"application/json":{"schema":candidate}});
        assert!(import(&input, false).is_err(), "accepted restriction {keyword}");
    }
}

#[test]
fn referenced_query_declarations_require_resolvable_parameter_objects() {
    for yaml in [false, true] {
        for reference in [Value::Null, json!(123), json!("#/components/parameters/Missing")] {
            let mut input = fixture();
            input["paths"]["/items?beta=true"]["post"]["parameters"] = json!([{"$ref":reference}]);
            assert!(import(&input, yaml).is_err(), "accepted reference {reference}");
        }
        let mut input = fixture();
        input["components"]["parameters"]["Limit"] = json!({"name":"limit","in":"query","schema":{"type":"integer"}});
        input["paths"]["/items?beta=true"]["post"]["parameters"] = json!([{"$ref":"#/components/parameters/Limit"}]);
        assert_eq!(import(&input, yaml).unwrap().api.endpoints[0].params.as_ref().unwrap().query.len(), 2);
    }
}

#[test]
fn absent_explode_takes_the_openapi_default_for_the_style() {
    let array = json!({"type":"array","items":{"type":"string"}});
    let input = json!({"openapi":"3.0.0","info":{"title":"Explode","version":"1"},"paths":{
        "/items":{"get":{"operationId":"listItems","parameters":[
            {"name":"form_default","in":"query","schema":array},
            {"name":"form_explicit_false","in":"query","explode":false,"schema":array},
            {"name":"form_explicit_true","in":"query","style":"form","explode":true,"schema":array},
            {"name":"pipes","in":"query","style":"pipeDelimited","schema":array},
            {"name":"spaces","in":"query","style":"spaceDelimited","schema":array},
            {"name":"filter","in":"query","style":"deepObject","schema":{"type":"object"}},
            {"name":"x-tags","in":"header","schema":array},
            {"name":"session","in":"cookie","schema":array}
        ],"responses":{"200":{"description":"ok"}}}}
    }});
    for yaml in [false, true] {
        let result = import(&input, yaml).unwrap();
        let params = result.api.endpoints[0].params.as_ref().unwrap();
        let explode = |list: &[schematic_define::ParamDef]| {
            list.iter().map(|param| (param.name.clone(), param.explode)).collect::<Vec<_>>()
        };
        assert_eq!(
            explode(&params.query),
            [
                ("form_default".to_string(), true),
                ("form_explicit_false".to_string(), false),
                ("form_explicit_true".to_string(), true),
                ("pipes".to_string(), false),
                ("spaces".to_string(), false),
                ("filter".to_string(), false),
            ]
        );
        assert_eq!(explode(&params.header), [("x-tags".to_string(), false)]);
        assert_eq!(explode(&params.cookie), [("session".to_string(), true)]);
    }
}
