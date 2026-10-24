#![cfg(feature = "openapi")]
//! OpenAPI export of value constraints: parameters, request bodies, form
//! fields, adopted page sizes, and component schemas carry their rules in
//! OpenAPI 3.0 form, numbers stay exact in serialized text, and the rules
//! survive a re-import.

use schemars::JsonSchema;
use schematic_define::constraints::{ConstraintNumber, Constraints};
use schematic_define::openapi::{ExportFormat, ExportOptions, OpenApiImport, OpenApiSource, export, serialize};
use schematic_define::pagination::{MoreSignal, PageSize, Strategy};
use schematic_define::{
    ApiRequest, DefinitionContext, Endpoint, FormField, ParamDef, PrimitiveType, QueryParamType, RestApi, SchemaComponent, TypeRef,
    definition_type,
};
use serde_json::{Value, json};

definition_type!(#[derive(JsonSchema)] pub struct Counter {
    #[schemars(range(max = 18446744073709551615u64))]
    pub total: u64,
    #[schemars(range(min = 1, max = 10))]
    pub retries: Option<u32>,
});

definition_type!(#[derive(JsonSchema)] #[schemars(rename = "item record")] pub struct ItemRecord { pub id: String });

fn api(ctx: &DefinitionContext, endpoints: impl IntoIterator<Item = Endpoint>) -> RestApi {
    RestApi::builder(ctx, "Rules", "https://example.test").describes("Rules").no_auth().endpoints(endpoints).build().unwrap()
}

/// The exported document as JSON, without `x-schematic` extensions.
fn exported(api: &RestApi) -> Value {
    let doc = export(api, &ExportOptions::new().skip_extensions()).unwrap();
    serde_json::from_str(&serialize(&doc, ExportFormat::Json).unwrap()).unwrap()
}

fn text(api: &RestApi, format: ExportFormat) -> String {
    serialize(&export(api, &ExportOptions::new()).unwrap(), format).unwrap()
}

fn param_schema<'a>(operation: &'a Value, location: &str, name: &str) -> &'a Value {
    let params = operation["parameters"].as_array().expect("operation parameters");
    &params.iter().find(|param| param["in"] == location && param["name"] == name).unwrap_or_else(|| panic!("no {location} {name}"))["schema"]
}

/// `text`'s exact decimal value as `(negative, digits, exponent)`, with no
/// leading or trailing zero digits; zero has no digits.
fn decimal(text: &str) -> (bool, String, i64) {
    let (negative, unsigned) = match text.strip_prefix('-') {
        Some(rest) => (true, rest),
        None => (false, text.strip_prefix('+').unwrap_or(text)),
    };
    let (mantissa, exponent) = match unsigned.split_once(['e', 'E']) {
        Some((mantissa, exponent)) => (mantissa, exponent.parse::<i64>().unwrap()),
        None => (unsigned, 0),
    };
    let (whole, fraction) = mantissa.split_once('.').unwrap_or((mantissa, ""));
    let mut digits = format!("{whole}{fraction}").trim_start_matches('0').to_owned();
    let mut exponent = exponent - fraction.len() as i64;
    while digits.ends_with('0') {
        digits.pop();
        exponent += 1;
    }
    if digits.is_empty() {
        return (false, digits, 0);
    }
    (negative, digits, exponent)
}

fn same_number(left: &Option<ConstraintNumber>, right: &Option<ConstraintNumber>) -> bool {
    match (left, right) {
        (None, None) => true,
        (Some(left), Some(right)) => decimal(left.as_str()) == decimal(right.as_str()),
        _ => false,
    }
}

/// Whether two rule sets mean the same: numbers by exact decimal value,
/// everything else by equality.
fn same_meaning(left: &Constraints, right: &Constraints) -> bool {
    let numbers = |rules: &Constraints| {
        [rules.minimum.clone(), rules.maximum.clone(), rules.exclusive_minimum.clone(), rules.exclusive_maximum.clone(), rules.multiple_of.clone()]
    };
    let strip = |rules: &Constraints| {
        let mut rest = rules.clone();
        (rest.minimum, rest.maximum, rest.exclusive_minimum, rest.exclusive_maximum, rest.multiple_of, rest.items) =
            (None, None, None, None, None, None);
        rest
    };
    let items = match (&left.items, &right.items) {
        (None, None) => true,
        (Some(left), Some(right)) => same_meaning(left, right),
        _ => false,
    };
    numbers(left).iter().zip(numbers(right).iter()).all(|(left, right)| same_number(left, right)) && strip(left) == strip(right) && items
}

fn every_param_endpoint() -> Endpoint {
    Endpoint::get("Search", "/items/{id}")
        .describes("Search")
        .returns_empty()
        .path_param(ParamDef::path("id", QueryParamType::String).constraints(Constraints::new().min_length(2).max_length(64)))
        .query(ParamDef::new("limit", QueryParamType::Integer).constraints(Constraints::new().minimum(1).exclusive_maximum(100).multiple_of(5)))
        .query(ParamDef::new("score", QueryParamType::Number).constraints(Constraints::new().exclusive_minimum(0).maximum(1.5)))
        .query(ParamDef::new("name", QueryParamType::String).constraints(Constraints::new().pattern("^[a-z]+$").format("slug")))
        .query(ParamDef::new("tags", QueryParamType::Array(Box::new(QueryParamType::String)))
            .constraints(Constraints::new().min_items(1).max_items(3).unique_items().items(Constraints::new().max_length(5))))
        .query(ParamDef::new("mode", QueryParamType::String).constraints(Constraints::new().constant("fast")))
        .query(ParamDef::new("level", QueryParamType::Integer).constraints(Constraints::new().allowed_values([1, 2, 3])))
        .query(ParamDef::new("order", QueryParamType::Enum(vec!["asc".into(), "desc".into()])).constraints(Constraints::new().max_length(4)))
        .header_param(ParamDef::new("X-Trace", QueryParamType::String).constraints(Constraints::new().format("uuid")))
        .cookie(ParamDef::new("session", QueryParamType::String).constraints(Constraints::new().min_length(16)))
        .build()
        .unwrap()
}

#[test]
fn every_parameter_location_exports_its_rules_in_openapi_3_0_form() {
    let ctx = DefinitionContext::types_in("::models");
    let doc = exported(&api(&ctx, [every_param_endpoint()]));
    let item = &doc["paths"]["/items/{id}"];
    let get = &item["get"];

    assert_eq!(param_schema(item, "path", "id"), &json!({"type": "string", "minLength": 2, "maxLength": 64}));
    assert_eq!(
        param_schema(get, "query", "limit"),
        &json!({"type": "integer", "minimum": 1, "maximum": 100, "exclusiveMaximum": true, "multipleOf": 5})
    );
    let score = param_schema(get, "query", "score");
    assert_eq!((&score["type"], &score["exclusiveMinimum"], &score["maximum"]), (&json!("number"), &json!(true), &json!(1.5)));
    assert_eq!(score["minimum"].as_f64(), Some(0.0));
    assert_eq!(param_schema(get, "query", "name"), &json!({"type": "string", "pattern": "^[a-z]+$", "format": "slug"}));
    assert_eq!(
        param_schema(get, "query", "tags"),
        &json!({"type": "array", "items": {"type": "string", "maxLength": 5}, "minItems": 1, "maxItems": 3, "uniqueItems": true})
    );
    assert_eq!(param_schema(get, "query", "mode"), &json!({"type": "string", "enum": ["fast"]}));
    assert_eq!(param_schema(get, "query", "level"), &json!({"type": "integer", "enum": [1, 2, 3]}));
    // The type's own enum is the value domain; the rules join it.
    assert_eq!(param_schema(get, "query", "order"), &json!({"type": "string", "enum": ["asc", "desc"], "maxLength": 4}));
    assert_eq!(param_schema(get, "header", "X-Trace"), &json!({"type": "string", "format": "uuid"}));
    assert_eq!(param_schema(get, "cookie", "session"), &json!({"type": "string", "minLength": 16}));
}

#[test]
fn request_bodies_and_form_fields_export_their_rules() {
    let mut ctx = DefinitionContext::types_in("::models");
    let note = Endpoint::post("CreateNote", "/notes").describes("Note").returns_empty().request_text()
        .request_constraints(Constraints::new().min_length(1).max_length(280)).build().unwrap();
    let batch = Endpoint::post("CreateBatch", "/batches").describes("Batch").returns_empty().request_json::<Vec<String>>(&mut ctx)
        .request_constraints(Constraints::new().max_items(100).items(Constraints::new().pattern("^[a-z]+$"))).build().unwrap();
    let count = Endpoint::post("SetCount", "/count").describes("Count").returns_empty().request_json::<u32>(&mut ctx)
        .request_constraints(Constraints::new().exclusive_minimum(0)).build().unwrap();
    let fields = || vec![
        FormField::text("query").constraints(Constraints::new().min_length(1)),
        FormField::json("filter", TypeRef::Array(Box::new(TypeRef::Primitive(PrimitiveType::String)))).constraints(Constraints::new().max_items(2)),
        FormField::file("upload"),
    ];
    let search = Endpoint::post("Search", "/search").describes("Search").returns_empty().request_urlencoded(fields()).build().unwrap();
    let upload = Endpoint::post("Upload", "/upload").describes("Upload").returns_empty().request_form(fields()).build().unwrap();
    let doc = exported(&api(&ctx, [note, batch, count, search, upload]));
    let body = |path: &str, media: &str| doc["paths"][path]["post"]["requestBody"]["content"][media]["schema"].clone();

    assert_eq!(body("/notes", "text/plain"), json!({"type": "string", "minLength": 1, "maxLength": 280}));
    assert_eq!(body("/batches", "application/json"), json!({"type": "array", "items": {"type": "string", "pattern": "^[a-z]+$"}, "maxItems": 100}));
    // The type's own `minimum: 0` and the exclusive rule both apply.
    let count = body("/count", "application/json");
    let members = count["allOf"].as_array().expect("overlapping bounds export side by side");
    assert_eq!(members[0]["minimum"], json!(0));
    assert_eq!((members[1]["minimum"].as_f64(), &members[1]["exclusiveMinimum"]), (Some(0.0), &json!(true)));

    for (path, media) in [("/search", "application/x-www-form-urlencoded"), ("/upload", "multipart/form-data")] {
        let properties = body(path, media)["properties"].clone();
        assert_eq!(properties["query"], json!({"type": "string", "minLength": 1}), "{media}");
        assert_eq!(
            properties["filter"]["allOf"][0],
            json!({"type": "array", "items": {"type": "string"}, "maxItems": 2}),
            "{media}"
        );
        assert_eq!(properties["upload"], json!({"type": "string", "format": "binary"}), "{media}");
    }
}

#[test]
fn numbers_no_json_number_holds_serialize_exactly_in_json_and_yaml() {
    let ctx = DefinitionContext::types_in("::models");
    let endpoint = Endpoint::get("Exact", "/exact").describes("Exact").returns_empty()
        .query(ParamDef::new("big", QueryParamType::Integer).constraints(Constraints::new().maximum("18446744073709551616")))
        .query(ParamDef::new("step", QueryParamType::Number).constraints(Constraints::new().multiple_of("0.1")))
        .query(ParamDef::new("fine", QueryParamType::Number).constraints(Constraints::new().multiple_of("0.10000000000000000001")))
        .build().unwrap();
    let api = api(&ctx, [endpoint]);

    let json_text = text(&api, ExportFormat::Json);
    assert!(json_text.contains(r#""maximum": 18446744073709551616"#), "{json_text}");
    assert!(json_text.contains(r#""multipleOf": 0.1"#), "{json_text}");
    assert!(json_text.contains(r#""multipleOf": 0.10000000000000000001"#), "{json_text}");
    let yaml_text = text(&api, ExportFormat::Yaml);
    assert!(yaml_text.contains("maximum: 18446744073709551616"), "{yaml_text}");
    assert!(yaml_text.contains("multipleOf: 0.1\n"), "{yaml_text}");
    assert!(yaml_text.contains("multipleOf: 0.10000000000000000001"), "{yaml_text}");
    for output in [&json_text, &yaml_text] {
        assert!(!output.contains("x-schematic-exact"), "{output}");
    }
}

#[test]
fn component_bounds_above_i64_serialize_exactly_and_nullable_components_keep_their_rules() {
    let mut ctx = DefinitionContext::types_in("::models");
    let endpoint = Endpoint::get("Count", "/count").describes("Count").returns_json::<Counter>(&mut ctx).build().unwrap();
    let mut api = api(&ctx, [endpoint]);
    api.schemas.push(SchemaComponent {
        name: "Code".into(),
        rust_path: Some("::models::Code".into()),
        schema: json!({"type": ["string", "null"], "maxLength": 3, "pattern": "^[A-Z]+$"}),
    });

    let json_text = text(&api, ExportFormat::Json);
    assert!(json_text.contains(r#""maximum": 18446744073709551615"#), "{json_text}");
    assert!(text(&api, ExportFormat::Yaml).contains("maximum: 18446744073709551615"));
    assert!(!json_text.contains("x-schematic-exact"));

    let doc = exported(&api);
    let schemas = &doc["components"]["schemas"];
    assert_eq!(schemas["Code"], json!({"type": "string", "nullable": true, "maxLength": 3, "pattern": "^[A-Z]+$"}));
    let retries = &schemas["Counter"]["properties"]["retries"];
    assert_eq!((&retries["nullable"], &retries["minimum"], &retries["maximum"]), (&json!(true), &json!(1), &json!(10)));
}

#[test]
fn the_adopted_page_size_parameter_exports_the_strategy_bounds() {
    let mut ctx = DefinitionContext::types_in("::models");
    let endpoint = Endpoint::get("ListItems", "/items").describes("Items").returns_json::<Vec<Value>>(&mut ctx)
        .query(ParamDef::new("per_page", QueryParamType::Integer))
        .query(ParamDef::new("page", QueryParamType::Integer))
        .build().unwrap();
    let api = RestApi::builder(&ctx, "Items", "https://example.test").describes("Items").no_auth()
        .default_pagination(Strategy::page_number("page", MoreSignal::NonEmptyPage).page_size(PageSize::new("per_page").max(100)).build())
        .endpoints([endpoint]).build().unwrap();

    let doc = exported(&api);
    let get = &doc["paths"]["/items"]["get"];
    assert_eq!(param_schema(get, "query", "per_page"), &json!({"type": "integer", "minimum": 1, "maximum": 100}));
    assert_eq!(param_schema(get, "query", "page"), &json!({"type": "integer"}));
}

#[test]
fn a_constraint_free_api_serializes_as_its_plain_document() {
    let mut ctx = DefinitionContext::types_in("::models");
    let endpoint = Endpoint::post("Create", "/items/{id}").describes("Create").returns_empty()
        .path_param(ParamDef::path("id", QueryParamType::Integer))
        .query(ParamDef::new("tags", QueryParamType::Array(Box::new(QueryParamType::String))))
        .header_param(ParamDef::new("X-Mode", QueryParamType::Enum(vec!["a".into()])))
        .request_form(vec![FormField::text("title"), FormField::json("meta", TypeRef::Primitive(PrimitiveType::Json)), FormField::file("file")])
        .build().unwrap();
    let note = Endpoint::post("Note", "/notes").describes("Note").returns_empty().request_text().build().unwrap();
    let batch = Endpoint::post("Batch", "/batch").describes("Batch").returns_empty().request_json::<Vec<String>>(&mut ctx).build().unwrap();
    let api = api(&ctx, [endpoint, note, batch]);
    let doc = export(&api, &ExportOptions::new()).unwrap();

    assert_eq!(serialize(&doc, ExportFormat::Json).unwrap(), serde_json::to_string_pretty(&doc).unwrap());
    let value = serde_json::to_value(&doc).unwrap();
    assert_eq!(value["paths"]["/items/{id}"]["parameters"][0]["schema"], json!({"type": "integer"}));
    assert_eq!(value["paths"]["/items/{id}"]["post"]["parameters"][0]["schema"], json!({"type": "array", "items": {"type": "string"}}));
    assert_eq!(value["paths"]["/notes"]["post"]["requestBody"]["content"]["text/plain"]["schema"], json!({"type": "string"}));
}

#[test]
fn exported_parameter_rules_survive_a_reimport() {
    let mut ctx = DefinitionContext::types_in("::models");
    let mut search = every_param_endpoint();
    let params = search.params.as_mut().unwrap();
    params.query.push(ParamDef::new("big", QueryParamType::Integer).constraints(Constraints::new().maximum("18446744073709551616")));
    params.query.push(ParamDef::new("fine", QueryParamType::Number).constraints(Constraints::new().multiple_of("0.10000000000000000001")));
    let record = Endpoint::post("CreateRecord", "/records").describes("Record").returns_empty().request_json::<ItemRecord>(&mut ctx)
        .build().unwrap();
    let original = api(&ctx, [search, record]);

    for format in [ExportFormat::Json, ExportFormat::Yaml] {
        let source = match format {
            ExportFormat::Json => OpenApiSource::json(text(&original, format)),
            ExportFormat::Yaml => OpenApiSource::yaml(text(&original, format)),
        };
        let imported = OpenApiImport::new(source).api_name("Rules").build().expect("export re-imports").api;

        let find = |api: &RestApi, id: &str| api.endpoints.iter().find(|endpoint| endpoint.id == id).cloned().unwrap();
        let (before, after) = (find(&original, "Search"), find(&imported, "Search"));
        let (before, after) = (before.params.unwrap(), after.params.unwrap());
        for (location, before, after) in
            [("path", &before.path, &after.path), ("query", &before.query, &after.query), ("header", &before.header, &after.header), ("cookie", &before.cookie, &after.cookie)]
        {
            for param in before {
                let reimported = after.iter().find(|candidate| candidate.name == param.name).unwrap_or_else(|| panic!("{location} {} lost", param.name));
                let mut left = param.constraints.as_deref().unwrap().clone();
                // A string value domain re-imports as the parameter's enum type.
                if let QueryParamType::Enum(values) = &reimported.param_type && param.param_type == QueryParamType::String {
                    let domain = left.constant.take().map(|value| vec![value]).or(left.allowed_values.take());
                    assert_eq!(domain, Some(values.iter().map(|value| json!(value)).collect()), "{format:?}: {location} {}", param.name);
                }
                let empty = Constraints::new();
                let right = reimported.constraints.as_deref().unwrap_or(&empty);
                let left = &left;
                assert!(same_meaning(left, right), "{format:?}: {location} {}: {left:?} became {right:?}", param.name);
            }
        }

        // The renamed component's reference resolves in the re-imported API.
        let Some(ApiRequest::Json(TypeRef::Named(name))) = find(&imported, "CreateRecord").request else {
            panic!("{format:?}: the record body is no longer a component reference");
        };
        assert!(imported.schemas.iter().any(|schema| schema.name == name), "{format:?}: {name} does not resolve");
    }
}

#[test]
fn a_folded_null_alternative_keeps_admitting_null_and_a_rejecting_sibling_keeps_rejecting_it() {
    let mut ctx = DefinitionContext::types_in("::models");
    let endpoint = Endpoint::get("Count", "/count").describes("Count").returns_json::<Counter>(&mut ctx).build().unwrap();
    let mut api = api(&ctx, [endpoint]);
    api.schemas.push(SchemaComponent {
        name: "Grade".into(),
        rust_path: Some("::models::Grade".into()),
        schema: json!({"maxLength": 5, "anyOf": [{"type": "string", "maxLength": 3, "enum": ["a"]}, {"type": "null"}]}),
    });
    api.schemas.push(SchemaComponent {
        name: "Label".into(),
        rust_path: Some("::models::Label".into()),
        schema: json!({"type": "string", "anyOf": [{"maxLength": 3}, {"type": "null"}]}),
    });

    let doc = exported(&api);
    let only_null = json!({"type": "object", "nullable": true, "enum": [null]});
    let schemas = &doc["components"]["schemas"];
    // The member overlaps the body's `maxLength`, so it stays an `allOf`
    // member; it still admits null, as the source's null alternative did.
    assert_eq!(
        schemas["Grade"],
        json!({"anyOf": [
            {"maxLength": 5, "allOf": [{"anyOf": [{"type": "string", "maxLength": 3, "enum": ["a"]}, only_null]}]},
            only_null,
        ]})
    );
    // `type: string` rejects null whatever the alternative admits.
    assert_eq!(schemas["Label"], json!({"type": "string", "maxLength": 3}));

    let imported = OpenApiImport::new(OpenApiSource::json(text(&api, ExportFormat::Json))).api_name("Rules").build().expect("export re-imports").api;
    let stored = |name: &str| imported.schemas.iter().find(|schema| schema.name == name).unwrap_or_else(|| panic!("{name} lost")).schema.clone();
    // The re-import stores both exported forms unchanged, so the null each
    // admits or rejects survives.
    for name in ["Grade", "Label"] {
        assert_eq!(stored(name), schemas[name], "{name}");
    }
}
