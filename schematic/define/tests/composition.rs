//! Same-context structural composition bound to hand-written Rust types.
use schematic_define::{ApiRequest, DefinitionContext, DefinitionError, Endpoint, ModelCatalog, ModelDef, RestApi, TypeAlias, TypeDesc, TypeRef, definition_type};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::any::type_name;

pub mod models {
    use super::*;
    definition_type!(#[derive(Debug, PartialEq, JsonSchema, Serialize, Deserialize)] #[serde(deny_unknown_fields)] pub struct Cat { pub meows: bool });
    definition_type!(#[derive(Debug, PartialEq, JsonSchema, Serialize, Deserialize)] #[serde(deny_unknown_fields)] pub struct Dog { pub barks: u8 });
    // Untagged: Serde tries Cat, then Dog. Cat and Dog reject each other's
    // members, so at most one matches; that exclusivity is the author's
    // assertion, which neither serde nor the schema proves.
    definition_type!(#[derive(Debug, PartialEq, JsonSchema, Serialize, Deserialize)] #[serde(untagged)] pub enum Pet { Cat(Cat), Dog(Dog) });
    definition_type!(#[derive(Debug, PartialEq, JsonSchema, Serialize, Deserialize)] #[serde(deny_unknown_fields)] pub struct Email { pub email: String });
    definition_type!(#[derive(Debug, PartialEq, JsonSchema, Serialize, Deserialize)] #[serde(deny_unknown_fields)] pub struct Phone { pub phone: String });
    definition_type!(#[derive(Debug, PartialEq, JsonSchema, Serialize, Deserialize)] #[serde(untagged)] pub enum Contact { Email(Email), Phone(Phone) });
    definition_type!(#[derive(Debug, PartialEq, JsonSchema, Serialize, Deserialize)] pub struct Named { pub name: String });
    definition_type!(#[derive(Debug, PartialEq, JsonSchema, Serialize, Deserialize)] pub struct Aged { pub age: u32 });
    definition_type!(#[derive(Debug, PartialEq, JsonSchema, Serialize, Deserialize)] #[serde(deny_unknown_fields)] pub struct Person { pub name: String, pub age: u32 });
    definition_type!(#[derive(JsonSchema)] pub struct Owner { pub pet: Pet });
    definition_type!(#[derive(JsonSchema)] #[schemars(rename = "Pet")] pub struct FakePet { pub scales: bool });
    definition_type!(#[derive(JsonSchema)] pub struct Terrarium { pub pet: FakePet });
}
use models::*;

const MODULE: &str = "::composition::models";
fn named(name: &str) -> TypeRef { TypeRef::Named(name.into()) }
fn reference(name: &str) -> Value { json!({"$ref": format!("#/components/schemas/{name}")}) }
fn context() -> (DefinitionContext, TypeDesc, TypeDesc) {
    let mut ctx = DefinitionContext::types_in(MODULE);
    let cat = ctx.register::<Cat>().unwrap();
    let dog = ctx.register::<Dog>().unwrap();
    (ctx, cat, dog)
}
fn schema_of(ctx: &DefinitionContext, name: &str) -> Value {
    ctx.components().unwrap().into_iter().find(|c| c.name == name).unwrap().schema
}
fn round_trip<T: Serialize + for<'de> Deserialize<'de> + PartialEq + std::fmt::Debug>(value: T, wire: Value) {
    assert_eq!(serde_json::to_value(&value).unwrap(), wire);
    assert_eq!(serde_json::from_value::<T>(wire).unwrap(), value);
}

#[test]
fn composition_one_of_binds_untagged_enum() {
    let (mut ctx, cat, dog) = context();
    let pet = ctx.one_of::<Pet>([cat.clone(), dog.clone()]).unwrap();
    assert_eq!(pet.rust_path(), format!("{MODULE}::Pet"));
    assert_eq!(pet.type_ref(), &named("Pet"));
    assert_eq!(pet.wire_schema(), reference("Pet"));
    assert_eq!(pet.structure(), Some(&TypeRef::OneOf(vec![named("Cat"), named("Dog")])));
    assert_eq!(cat.structure(), None);
    assert_eq!(schema_of(&ctx, "Pet"), json!({"oneOf": [reference("Cat"), reference("Dog")]}));
    let component = ctx.components().unwrap();
    assert_eq!(component.iter().map(|c| c.name.as_str()).collect::<Vec<_>>(), ["Cat", "Dog", "Pet"]);
    assert_eq!(component[2].rust_path.as_deref(), Some(pet.rust_path()));
    assert_eq!(ctx.registrations()[2].structure(), pet.structure());
    // schemars' own schema for an untagged enum is `anyOf`; the explicit
    // structure replaces it and is not a conflict.
    assert!(ctx.registrations()[2].schema().as_value().get("anyOf").is_some());
    round_trip(Pet::Cat(Cat { meows: true }), json!({"meows": true}));
    round_trip(Pet::Dog(Dog { barks: 3 }), json!({"barks": 3}));
    assert!(serde_json::from_value::<Pet>(json!({"hisses": true})).is_err());
    assert!(serde_json::from_value::<Pet>(json!({"meows": true, "barks": 3})).is_err());
    ctx.finish().unwrap();
}

#[test]
fn composition_repeats_and_later_plain_registration_return_composed_descriptor() {
    let (mut ctx, cat, dog) = context();
    let pet = ctx.one_of::<Pet>([cat.clone(), dog.clone()]).unwrap();
    assert_eq!(ctx.one_of::<Pet>([cat, dog]).unwrap(), pet);
    assert_eq!(ctx.register::<Pet>().unwrap(), pet);
    assert_eq!(ctx.register::<Vec<Pet>>().unwrap().structure(), None);
    assert_eq!(ctx.registrations().len(), 3);
    assert_eq!(ctx.models().types.len(), 1);
    ctx.finish().unwrap();
}

#[test]
fn composition_any_of_binds_distinguishable_pair() {
    let mut ctx = DefinitionContext::types_in(MODULE);
    let email = ctx.register::<Email>().unwrap();
    let phone = ctx.register::<Phone>().unwrap();
    let contact = ctx.any_of::<Contact>([email, phone]).unwrap();
    assert_eq!(contact.rust_path(), format!("{MODULE}::Contact"));
    assert_eq!(contact.structure(), Some(&TypeRef::AnyOf(vec![named("Email"), named("Phone")])));
    assert_eq!(schema_of(&ctx, "Contact"), json!({"anyOf": [reference("Email"), reference("Phone")]}));
    round_trip(Contact::Email(Email { email: "a@example.test".into() }), json!({"email": "a@example.test"}));
    round_trip(Contact::Phone(Phone { phone: "555".into() }), json!({"phone": "555"}));
    assert!(serde_json::from_value::<Contact>(json!({"fax": "1"})).is_err());
}

#[test]
fn composition_all_of_binds_struct_satisfying_every_member() {
    let mut ctx = DefinitionContext::types_in(MODULE);
    let name = ctx.register::<Named>().unwrap();
    let age = ctx.register::<Aged>().unwrap();
    let person = ctx.all_of::<Person>([name, age]).unwrap();
    assert_eq!(person.structure(), Some(&TypeRef::AllOf(vec![named("Named"), named("Aged")])));
    assert_eq!(schema_of(&ctx, "Person"), json!({"allOf": [reference("Named"), reference("Aged")]}));
    let wire = json!({"name": "Ada", "age": 36});
    round_trip(Person { name: "Ada".into(), age: 36 }, wire.clone());
    assert_eq!(serde_json::from_value::<Named>(wire.clone()).unwrap(), Named { name: "Ada".into() });
    assert_eq!(serde_json::from_value::<Aged>(wire).unwrap(), Aged { age: 36 });
}

#[test]
fn composition_rejects_empty_member_list() {
    let mut ctx = DefinitionContext::types_in(MODULE);
    let error = ctx.one_of::<Pet>(Vec::new()).unwrap_err();
    assert_eq!(error, DefinitionError::EmptyComposition { target: type_name::<Pet>() });
    assert_eq!(error.to_string(), format!("composition for {} has no members", type_name::<Pet>()));
    assert!(ctx.registrations().is_empty() && ctx.models().types.is_empty());
    assert_eq!(ctx.finish(), Err(DefinitionError::Collisions(vec![error])));
}

#[test]
fn composition_rejects_member_unregistered_in_this_context() {
    let mut foreign = DefinitionContext::types_in(MODULE);
    let cat = foreign.register::<Cat>().unwrap();
    let mut ctx = DefinitionContext::types_in(MODULE);
    let dog = ctx.register::<Dog>().unwrap();
    let error = ctx.one_of::<Pet>([cat, dog]).unwrap_err();
    assert_eq!(error, DefinitionError::ForeignMember { target: type_name::<Pet>(), name: "Cat".into(), rust_path: format!("{MODULE}::Cat") });
    assert!(error.to_string().ends_with(&format!("member \"Cat\" ({MODULE}::Cat) is unregistered or from a foreign context")), "{error}");
    assert_eq!(ctx.finish(), Err(DefinitionError::Collisions(vec![error])));
    assert_eq!(ctx.registrations().len(), 1);
}

#[test]
fn composition_rejects_foreign_member_with_colliding_name_and_other_path() {
    let mut foreign = DefinitionContext::types_in("::elsewhere");
    let foreign_cat = foreign.register::<Cat>().unwrap();
    let (mut ctx, _, dog) = context();
    let error = ctx.one_of::<Pet>([foreign_cat, dog]).unwrap_err();
    assert_eq!(error, DefinitionError::ForeignMember { target: type_name::<Pet>(), name: "Cat".into(), rust_path: "::elsewhere::Cat".into() });
}

#[test]
fn composition_rejects_plain_registration_first() {
    let (mut ctx, cat, dog) = context();
    let plain = ctx.register::<Pet>().unwrap();
    let error = ctx.one_of::<Pet>([cat, dog]).unwrap_err();
    let target = type_name::<Pet>();
    assert_eq!(error, DefinitionError::ComponentConflict { schema_name: "Pet".into(), first: format!("{target} without a composition"),
        second: format!("{target} as {}", json!({"oneOf": [reference("Cat"), reference("Dog")]})) });
    assert!(error.to_string().starts_with("schema component \"Pet\" has different definitions from "), "{error}");
    assert_eq!(ctx.register::<Pet>().unwrap(), plain);
    assert!(ctx.models().types.is_empty());
    assert!(ctx.components().is_err());
}

#[test]
fn composition_rejects_second_different_composition() {
    let (mut ctx, cat, dog) = context();
    let pet = ctx.one_of::<Pet>([cat.clone(), dog.clone()]).unwrap();
    let target = type_name::<Pet>();
    let one_of = format!("{target} as {}", json!({"oneOf": [reference("Cat"), reference("Dog")]}));
    let error = ctx.any_of::<Pet>([cat.clone(), dog.clone()]).unwrap_err();
    assert_eq!(error, DefinitionError::ComponentConflict { schema_name: "Pet".into(), first: one_of.clone(),
        second: format!("{target} as {}", json!({"anyOf": [reference("Cat"), reference("Dog")]})) });
    let reordered = ctx.one_of::<Pet>([dog, cat]).unwrap_err();
    assert!(matches!(&reordered, DefinitionError::ComponentConflict { first, .. } if first == &one_of));
    assert_eq!(ctx.register::<Pet>().unwrap(), pet);
    assert_eq!(ctx.finish(), Err(DefinitionError::Collisions(vec![error, reordered])));
}

#[test]
fn composition_rejects_duplicate_non_nominal_and_container_targets() {
    let (mut ctx, cat, _) = context();
    let error = ctx.one_of::<Pet>([cat.clone(), cat.clone()]).unwrap_err();
    assert_eq!(error, DefinitionError::DuplicateMember { target: type_name::<Pet>(), name: "Cat".into() });
    assert!(error.to_string().ends_with("member \"Cat\" is listed more than once"));
    let cats = ctx.register::<Vec<Cat>>().unwrap();
    let error = ctx.one_of::<Pet>([cat.clone(), cats]).unwrap_err();
    assert_eq!(error, DefinitionError::NonNominalMember { target: type_name::<Pet>(), member: format!("::std::vec::Vec<{MODULE}::Cat>") });
    let text = ctx.register::<String>().unwrap();
    assert!(matches!(ctx.any_of::<Pet>([text]), Err(DefinitionError::NonNominalMember { .. })));
    assert_eq!(ctx.one_of::<Vec<Pet>>([cat]), Err(DefinitionError::NotLeaf));
    assert!(ctx.models().types.is_empty());
}

#[test]
fn composition_agrees_with_nested_copy_of_bound_type_in_either_order() {
    let (mut ctx, cat, dog) = context();
    ctx.register::<Owner>().unwrap();
    ctx.one_of::<Pet>([cat, dog]).unwrap();
    let components = ctx.components().unwrap();
    // Owner's nested copy of Pet came first; the composition takes it over in place.
    assert_eq!(components.iter().map(|c| (c.name.as_str(), c.rust_path.is_some())).collect::<Vec<_>>(), [("Cat", true), ("Dog", true), ("Owner", true), ("Pet", true)]);
    assert_eq!(schema_of(&ctx, "Pet"), json!({"oneOf": [reference("Cat"), reference("Dog")]}));
    let (mut later, cat, dog) = context();
    later.one_of::<Pet>([cat, dog]).unwrap();
    later.register::<Owner>().unwrap();
    assert_eq!(schema_of(&later, "Pet"), json!({"oneOf": [reference("Cat"), reference("Dog")]}));
}

#[test]
fn composition_conflicts_with_differently_nested_same_name() {
    let (mut ctx, cat, dog) = context();
    ctx.one_of::<Pet>([cat, dog]).unwrap();
    ctx.register::<Terrarium>().unwrap();
    let Err(DefinitionError::Collisions(errors)) = ctx.components() else { panic!("expected conflict") };
    assert!(matches!(&errors[..], [DefinitionError::ComponentConflict { schema_name, first, second }]
        if schema_name == "Pet" && first == type_name::<Pet>() && second == &format!("a schema nested in {}", type_name::<Terrarium>())), "{errors:?}");
}

#[test]
fn models_catalog_records_compositions_in_order() {
    let (mut ctx, cat, dog) = context();
    assert_eq!(ctx.models(), &ModelCatalog { module_path: None, types: vec![] });
    ctx.one_of::<Pet>([cat, dog]).unwrap();
    let name = ctx.register::<Named>().unwrap();
    let age = ctx.register::<Aged>().unwrap();
    ctx.all_of::<Person>([name, age]).unwrap();
    assert_eq!(ctx.models().types, [
        ModelDef::Alias(TypeAlias { name: "Pet".into(), description: None, target: TypeRef::OneOf(vec![named("Cat"), named("Dog")]) }),
        ModelDef::Alias(TypeAlias { name: "Person".into(), description: None, target: TypeRef::AllOf(vec![named("Named"), named("Aged")]) }),
    ]);
    let mut bytes = serde_json::to_vec(ctx.models()).unwrap();
    for _ in 0..2 {
        let reread: ModelCatalog = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(&reread, ctx.models());
        let again = serde_json::to_vec(&reread).unwrap();
        assert_eq!(again, bytes);
        bytes = again;
    }
    assert!(!String::from_utf8(bytes).unwrap().contains("null"));
}

#[test]
fn models_catalog_serde_presence_rules() {
    let read = |value: Value| serde_json::from_value::<ModelCatalog>(value);
    let catalog = read(json!({"types": [
        {"Alias": {"name": "Id", "target": "Unknown"}},
        {"Struct": {"name": "User", "fields": [{"name": "id", "required": true, "field_type": {"Primitive": "String"}}]}},
        {"Enum": {"name": "Status", "variants": [{"name": "Active"}], "untagged": false}},
    ]})).unwrap();
    assert_eq!(catalog.module_path, None);
    let [ModelDef::Alias(alias), ModelDef::Struct(user), ModelDef::Enum(status)] = &catalog.types[..] else { panic!("{catalog:?}") };
    assert_eq!((alias.description.as_ref(), user.description.as_ref(), user.additional_properties.as_ref()), (None, None, None));
    assert_eq!((user.fields[0].serde_rename.as_ref(), status.variants[0].value.as_ref()), (None, None));
    for (value, message) in [
        (json!({"module_path": null, "types": []}), "invalid type: null"),
        (json!({}), "missing field `types`"),
        (json!({"types": [], "extra": 1}), "unknown field `extra`"),
        (json!({"types": [{"Alias": {"name": "Id", "description": null, "target": "Unknown"}}]}), "invalid type: null"),
        (json!({"types": [{"Struct": {"name": "U", "fields": [], "additional_properties": null}}]}), "invalid type: null"),
        (json!({"types": [{"Struct": {"name": "U", "fields": [], "extra": 1}}]}), "unknown field `extra`"),
        (json!({"types": [{"Struct": {"name": "U", "fields": [{"name": "id", "field_type": "Unknown"}]}}]}), "missing field `required`"),
        (json!({"types": [{"Enum": {"name": "S", "variants": [], "untagged": false, "extra": 1}}]}), "unknown field `extra`"),
        (json!({"types": [{"Enum": {"name": "S", "variants": []}}]}), "missing field `untagged`"),
        (json!({"types": [{"Enum": {"name": "S", "variants": [{"name": "A", "value": null}], "untagged": false}}]}), "invalid type: null"),
        (json!({"types": [{"Enum": {"name": "S", "variants": [{"name": "A", "extra": 1}], "untagged": false}}]}), "unknown field `extra`"),
    ] {
        let error = read(value.clone()).unwrap_err().to_string();
        assert!(error.contains(message), "{value}: {error}");
    }
}

#[test]
fn composition_builds_into_rest_api() {
    let (mut ctx, cat, dog) = context();
    let pet = ctx.one_of::<Pet>([cat, dog]).unwrap();
    let endpoint = Endpoint::post("AdoptPet", "/pets").describes("Adopt a pet").request_json::<Pet>(&mut ctx).returns_empty().build().unwrap();
    assert_eq!(endpoint.request, Some(ApiRequest::Json(named("Pet"))));
    let api = RestApi::builder(&ctx, "PetApi", "https://example.test").describes("Pets").no_auth().endpoints([endpoint]).build().unwrap();
    assert_eq!(api.component("Pet").unwrap().schema, json!({"oneOf": [reference("Cat"), reference("Dog")]}));
    assert_eq!(api.component("Pet").unwrap().rust_path.as_deref(), Some(pet.rust_path()));
    assert_eq!(api.rust_type(&named("Pet")).as_deref(), Some(pet.rust_path()));
    api.validate_references().unwrap();
}
