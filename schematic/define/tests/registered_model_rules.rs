//! Value rules on registered models: Schemars attributes land in the stored
//! components once, and `unenforceable_rules` locates the ones beneath a
//! composition or an unresolved reference.
use schematic_define::constraints::{UnenforceableRule, unenforceable_rules};
use schematic_define::{ApiRequest, DefinitionContext, Endpoint, RestApi, TypeRef, definition_type};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

pub mod models {
    use super::*;
    definition_type!(#[derive(JsonSchema, Serialize, Deserialize)] pub struct Address {
        #[schemars(length(min = 1, max = 80))] pub street: String,
        #[schemars(regex(pattern = r"^[0-9]{5}$"))] pub zip: String,
    });
    definition_type!(#[derive(JsonSchema, Serialize, Deserialize)] pub struct Signup {
        #[schemars(length(min = 1, max = 40))] pub name: String,
        #[serde(rename = "displayName")] #[schemars(length(min = 2))] pub display_name: String,
        #[schemars(length(min = 1, max = 5), inner(length(max = 12)))] pub tags: Vec<String>,
        #[schemars(range(min = 1, max = 10))] pub seats: Option<i32>,
        #[schemars(extend("multipleOf" = 0.5))] pub price: f64,
        #[schemars(extend("exclusiveMinimum" = 0))] pub ratio: f64,
        pub age: u8,
        pub address: Address,
        pub billing: Option<Address>,
    });
    definition_type!(#[derive(JsonSchema, Serialize, Deserialize)] pub struct Tree {
        #[schemars(range(min = 0))] pub value: i32,
        pub parent: Option<Box<Tree>>,
        pub children: Vec<Tree>,
    });
    definition_type!(#[derive(JsonSchema, Serialize, Deserialize)] pub struct Forest { pub trunk: Tree, pub saplings: Option<Vec<Tree>> });
    definition_type!(#[derive(JsonSchema, Serialize, Deserialize)] #[serde(untagged)] pub enum Choice {
        Text(#[schemars(length(max = 3))] String),
        Number(#[schemars(range(min = 1))] i64),
        Small(u8),
    });
    definition_type!(#[derive(JsonSchema, Serialize, Deserialize)] pub struct Pair(#[schemars(length(max = 3))] pub String, pub u32););
    definition_type!(#[derive(JsonSchema, Serialize, Deserialize)] pub struct Placement { pub pair: Pair });
    definition_type!(#[derive(JsonSchema, Serialize, Deserialize)] pub struct Order {
        pub choice: Choice,
        pub fallback: Option<Choice>,
        #[schemars(range(min = 1))] pub quantity: u32,
    });
}
use models::*;

const MODULE: &str = "::registered_model_rules::models";

fn named(name: &str) -> TypeRef { TypeRef::Named(name.into()) }

/// Builds one API from `endpoints`, each built from a fresh endpoint builder
/// and the shared context.
fn api(build: impl FnOnce(&mut DefinitionContext) -> Vec<Endpoint>) -> RestApi {
    let mut ctx = DefinitionContext::types_in(MODULE);
    let endpoints = build(&mut ctx);
    RestApi::builder(&ctx, "Rules", "https://example.test").describes("Rules").no_auth().endpoints(endpoints).build().unwrap()
}

fn post<T: schematic_define::Registration>(id: &str, ctx: &mut DefinitionContext) -> Endpoint {
    Endpoint::post(id, "/").describes(id).request_json::<T>(ctx).returns_empty().build().unwrap()
}

fn schema<'a>(api: &'a RestApi, name: &str) -> &'a Value { &api.component(name).unwrap().schema }

fn located(rules: &[UnenforceableRule]) -> Vec<(Option<&str>, &str, &str)> {
    rules.iter().map(|rule| (rule.component.as_deref(), rule.pointer.as_str(), rule.keyword)).collect()
}

#[test]
fn rules_on_root_and_nested_models_land_in_stored_components_once() {
    let api = api(|ctx| vec![post::<Signup>("CreateSignup", ctx), post::<Signup>("UpdateSignup", ctx)]);
    assert_eq!(api.schemas.iter().map(|c| c.name.as_str()).collect::<Vec<_>>(), ["Signup", "Address"]);

    let signup = &schema(&api, "Signup")["properties"];
    assert_eq!(signup["name"], json!({"type": "string", "minLength": 1, "maxLength": 40}));
    assert_eq!(signup["displayName"], json!({"type": "string", "minLength": 2}));
    assert!(signup.get("display_name").is_none());
    assert_eq!(signup["tags"], json!({"type": "array", "items": {"type": "string", "maxLength": 12}, "minItems": 1, "maxItems": 5}));
    assert_eq!(signup["seats"], json!({"type": "integer", "format": "int32", "minimum": 1, "maximum": 10, "nullable": true}));
    assert_eq!(signup["price"], json!({"type": "number", "format": "double", "multipleOf": 0.5}));
    assert_eq!(signup["ratio"], json!({"type": "number", "format": "double", "exclusiveMinimum": 0}));
    assert_eq!(signup["billing"], json!({"anyOf": [{"$ref": "#/components/schemas/Address"}, {"enum": [null], "nullable": true}]}));
    let address = &schema(&api, "Address")["properties"];
    assert_eq!(address["street"], json!({"type": "string", "minLength": 1, "maxLength": 80}));
    assert_eq!(address["zip"], json!({"type": "string", "pattern": "^[0-9]{5}$"}));

    // The component is the only place a rule is stored: endpoints reference it
    // by name and carry no copy.
    for endpoint in &api.endpoints {
        assert_eq!(endpoint.request, Some(ApiRequest::Json(named("Signup"))));
        assert_eq!(endpoint.request_constraints, None);
    }
    let serialized = serde_json::to_string(&api).unwrap();
    assert_eq!(serialized.matches(r#""pattern":"^[0-9]{5}$""#).count(), 1);
    assert_eq!(serialized.matches(r#""multipleOf":0.5"#).count(), 1);
    assert_eq!(serialized.matches(r#""exclusiveMinimum":0"#).count(), 1);

    // Sharing the model across endpoints stores and changes nothing extra.
    let single = api_with_one_signup();
    assert_eq!(api.schemas, single.schemas);
    for body in [named("Signup"), TypeRef::Optional(Box::new(named("Signup"))), TypeRef::Array(Box::new(named("Address")))] {
        assert_eq!(unenforceable_rules(&api, &body), [], "{body:?}");
    }
}

fn api_with_one_signup() -> RestApi { api(|ctx| vec![post::<Signup>("CreateSignup", ctx)]) }

#[test]
fn nullable_and_recursive_shapes_are_enforceable() {
    let mut rooted = api(|ctx| vec![post::<Tree>("PlantTree", ctx)]);
    // Registration stores the recursive root's `"$ref": "#"` by name.
    assert_eq!(schema(&rooted, "Tree")["properties"]["parent"], json!({"anyOf": [{"$ref": "#/components/schemas/Tree"}, {"enum": [null], "nullable": true}]}));
    assert_eq!(schema(&rooted, "Tree")["properties"]["value"]["minimum"], 0);
    assert_eq!(unenforceable_rules(&rooted, &named("Tree")), []);

    // A self reference beneath a composition is walked once more, then stops.
    let tree = &mut rooted.schemas[0].schema["properties"];
    tree["parent"] = json!({"oneOf": [{"$ref": "#"}, {"type": "integer"}]});
    assert_eq!(located(&unenforceable_rules(&rooted, &named("Tree"))), [(Some("Tree"), "/properties/value", "minimum")]);

    let nested = api(|ctx| vec![post::<Option<Forest>>("PlantForest", ctx)]);
    assert_eq!(nested.schemas.iter().map(|c| c.name.as_str()).collect::<Vec<_>>(), ["Forest", "Tree"]);
    let parent = &schema(&nested, "Tree")["properties"]["parent"];
    assert_eq!(parent, &json!({"anyOf": [{"$ref": "#/components/schemas/Tree"}, {"type": "null"}]}));
    let body = TypeRef::Optional(Box::new(named("Forest")));
    assert_eq!(unenforceable_rules(&nested, &body), []);
}

#[test]
fn rules_beneath_an_untagged_enum_are_located() {
    let api = api(|ctx| vec![post::<Order>("PlaceOrder", ctx)]);
    assert_eq!(schema(&api, "Choice"), &json!({"anyOf": [
        {"type": "string", "maxLength": 3},
        {"type": "integer", "format": "int64", "minimum": 1},
        {"type": "integer", "format": "uint8", "minimum": 0, "maximum": 255},
    ]}));
    let rules = unenforceable_rules(&api, &named("Order"));
    assert_eq!(located(&rules), [(Some("Choice"), "/anyOf/0", "maxLength"), (Some("Choice"), "/anyOf/1", "minimum")]);
    assert_eq!(rules[0].reason, "beneath the `anyOf` at `#` of component `Choice`");
    // `Order.quantity`'s own `minimum: 1` sits outside the composition, and
    // `u8`'s bounds are its type's, not authored rules.
    assert!(rules.iter().all(|rule| rule.component.as_deref() == Some("Choice")));
}

#[test]
fn rules_beneath_one_of_all_of_and_not_are_located_but_a_one_member_all_of_is_walked_through() {
    let mut api = api_with_one_signup();
    let signup = &mut api.schemas.iter_mut().find(|c| c.name == "Signup").unwrap().schema["properties"];
    let name = signup["name"].take();
    signup["name"] = json!({"oneOf": [name, {"type": "null"}]});
    signup["address"] = json!({"allOf": [{"$ref": "#/components/schemas/Address"}, {"type": "object"}]});
    signup["seats"] = json!({"not": {"type": "integer", "maximum": 3}});
    // Schemars' OpenAPI 3.0 form of a `$ref` with sibling keywords is a
    // one-member `allOf`; it constrains only as its member does, so its
    // member's rules are enforceable.
    signup["billing"] = json!({"allOf": [{"type": "string", "maxLength": 5}], "enum": ["Main"]});

    let rules = unenforceable_rules(&api, &named("Signup"));
    assert_eq!(located(&rules), [
        (Some("Address"), "/properties/street", "minLength"),
        (Some("Address"), "/properties/street", "maxLength"),
        (Some("Address"), "/properties/zip", "pattern"),
        (Some("Signup"), "/properties/name/oneOf/0", "minLength"),
        (Some("Signup"), "/properties/name/oneOf/0", "maxLength"),
        (Some("Signup"), "/properties/seats/not", "maximum"),
    ]);
    let reasons: Vec<_> = rules.iter().map(|rule| rule.reason.as_str()).collect();
    assert_eq!(reasons[0], "beneath the `allOf` at `#/properties/address` of component `Signup`");
    assert_eq!(reasons[3], "beneath the `oneOf` at `#/properties/name` of component `Signup`");
    assert_eq!(reasons[5], "beneath the `not` at `#/properties/seats` of component `Signup`");
}

#[test]
fn rules_beside_an_unresolved_reference_are_located() {
    let mut api = api_with_one_signup();
    let signup = &mut api.schemas.iter_mut().find(|c| c.name == "Signup").unwrap().schema["properties"];
    signup["name"] = json!({"$ref": "#/components/schemas/Gone", "maxLength": 40});
    signup["displayName"] = json!({"$ref": "#/$defs/Address", "minLength": 2});

    let rules = unenforceable_rules(&api, &named("Signup"));
    assert_eq!(located(&rules), [(Some("Signup"), "/properties/name", "maxLength")]);
    assert_eq!(rules[0].reason, r##"beside the unresolved reference "#/components/schemas/Gone""##);
}

#[test]
fn rules_in_positional_items_are_located() {
    let mut api = api(|ctx| vec![post::<Placement>("Place", ctx)]);
    // Schemars writes a tuple's members as `prefixItems`, whatever the
    // OpenAPI 3.0 settings.
    assert_eq!(schema(&api, "Pair")["prefixItems"][0], json!({"type": "string", "maxLength": 3}));

    let rules = unenforceable_rules(&api, &named("Placement"));
    assert_eq!(located(&rules), [(Some("Pair"), "/prefixItems/0", "maxLength")]);
    assert_eq!(rules[0].reason, "beneath the `prefixItems` at `#` of component `Pair`, which the client does not traverse");
    assert_eq!(rules[0].composition, None);

    // The draft-07 spelling: an `items` list is positional too.
    let pair = &mut api.schemas.iter_mut().find(|c| c.name == "Pair").unwrap().schema;
    let members = pair["prefixItems"].take();
    pair.as_object_mut().unwrap().remove("prefixItems");
    pair["items"] = members;
    let rules = unenforceable_rules(&api, &named("Placement"));
    assert_eq!(located(&rules), [(Some("Pair"), "/items/0", "maxLength")]);
    assert_eq!(rules[0].reason, "beneath the `items` at `#` of component `Pair`, which the client does not traverse");

    // An `items` object stays the enforceable item schema.
    pair_items(&mut api, json!({"type": "string", "maxLength": 3}));
    assert_eq!(unenforceable_rules(&api, &named("Placement")), []);
}

fn pair_items(api: &mut RestApi, items: Value) {
    api.schemas.iter_mut().find(|c| c.name == "Pair").unwrap().schema["items"] = items;
}

#[test]
fn rules_beneath_untraversed_keywords_are_located() {
    let mut api = api_with_one_signup();
    let signup = &mut api.schemas.iter_mut().find(|c| c.name == "Signup").unwrap().schema["properties"];
    let rule = json!({"type": "string", "maxLength": 3});
    signup["name"] = json!({"type": "array", "prefixItems": [rule], "additionalItems": rule, "contains": rule});
    signup["displayName"] = json!({"type": "object", "patternProperties": {"^x-": rule}, "propertyNames": rule});
    signup["tags"] = json!({"if": rule, "then": rule, "else": rule, "dependentSchemas": {"a": rule}, "dependencies": {"b": rule, "c": ["d"]}});
    signup["price"] = json!({"unevaluatedItems": rule, "unevaluatedProperties": rule});

    let rules = unenforceable_rules(&api, &named("Signup"));
    let pointers: Vec<&str> = rules.iter().filter(|rule| rule.component.as_deref() == Some("Signup")).map(|rule| rule.pointer.as_str()).collect();
    assert_eq!(pointers, [
        "/properties/displayName/patternProperties/^x-",
        "/properties/displayName/propertyNames",
        "/properties/name/prefixItems/0",
        "/properties/name/additionalItems",
        "/properties/name/contains",
        "/properties/price/unevaluatedItems",
        "/properties/price/unevaluatedProperties",
        "/properties/tags/dependentSchemas/a",
        "/properties/tags/dependencies/b",
        "/properties/tags/if",
        "/properties/tags/then",
        "/properties/tags/else",
    ]);
    assert!(rules.iter().all(|rule| rule.keyword == "maxLength" && rule.composition.is_none()), "{rules:#?}");
    assert_eq!(rules[0].reason, "beneath the `patternProperties` at `#/properties/displayName` of component `Signup`, which the client does not traverse");
}
