//! A union alias with a discriminator generates a `Deserialize` that selects
//! its alternative by tag, never by first structural match.
//!
//! `tests/fixtures/discriminated_union/models.rs` is the generator's output
//! for [`catalog`]; the first test keeps it current and the rest compile and
//! decode through it.

use schematic_define::models::{
    FieldDef, ModelCatalog, ModelDef, PrimitiveType, StructDef, TypeAlias, TypeRef, UnionDiscriminator,
};
use schematic_gen::model_gen::assemble_model_tokens;
use schematic_gen::output::{format_code, validate_code};

// The generator always imports `HashMap`; these models do not use it.
#[allow(unused_imports)]
#[path = "../fixtures/discriminated_union/models.rs"]
mod generated;

use generated::{Cat, Dog, Pet, Puppy};

fn field(name: &str, field_type: PrimitiveType) -> FieldDef {
    FieldDef { name: name.into(), serde_rename: None, description: None, required: true, field_type: TypeRef::Primitive(field_type) }
}

fn object(name: &str, own: (&str, PrimitiveType)) -> ModelDef {
    ModelDef::Struct(StructDef {
        name: name.into(),
        description: None,
        fields: vec![field("kind", PrimitiveType::String), field(own.0, own.1)],
        additional_properties: None,
    })
}

/// `Pet` over `Cat`, `Dog`, and `Puppy`, where `Dog` and `Puppy` share the
/// `puppy` tag and `Cat` would structurally accept every `Dog` value that
/// carries `meow`.
fn catalog() -> ModelCatalog {
    let tags = |values: &[&str]| values.iter().map(|value| value.to_string()).collect::<Vec<_>>();
    ModelCatalog {
        module_path: None,
        types: vec![
            object("Cat", ("meow", PrimitiveType::Boolean)),
            object("Dog", ("bark", PrimitiveType::Boolean)),
            object("Puppy", ("yip", PrimitiveType::Integer)),
            ModelDef::Alias(TypeAlias {
                name: "Pet".into(),
                description: None,
                target: TypeRef::OneOf(vec![TypeRef::Named("Cat".into()), TypeRef::Named("Dog".into()), TypeRef::Named("Puppy".into())]),
                discriminator: Some(UnionDiscriminator {
                    property: "kind".into(),
                    tags: vec![tags(&["cat"]), tags(&["dog", "puppy"]), tags(&["puppy"])],
                }),
            }),
        ],
    }
}

#[test]
fn discriminated_union_fixture_is_the_generator_output() {
    let generated = format_code(&validate_code(&assemble_model_tokens(&catalog())).expect("generated models parse"));

    assert_eq!(generated, include_str!("../fixtures/discriminated_union/models.rs"));
}

#[test]
fn discriminated_union_decodes_the_alternative_its_tag_names() {
    // `Cat` comes first and this value fits it too; untagged would pick it.
    let pet: Pet = serde_json::from_str(r#"{"kind":"dog","bark":true,"meow":false}"#).expect("a dog decodes");

    let Pet::Dog(Dog { kind, bark }) = pet else { panic!("expected Dog, got {pet:?}") };
    assert_eq!((kind.as_str(), bark), ("dog", true));
}

#[test]
fn discriminated_union_tries_alternatives_sharing_a_tag_in_order() {
    let dog: Pet = serde_json::from_str(r#"{"kind":"puppy","bark":false}"#).expect("a barking puppy decodes");
    let puppy: Pet = serde_json::from_str(r#"{"kind":"puppy","yip":3}"#).expect("a yipping puppy decodes");

    assert!(matches!(dog, Pet::Dog(Dog { bark: false, .. })), "{dog:?}");
    assert!(matches!(puppy, Pet::Puppy(Puppy { yip: 3, .. })), "{puppy:?}");
}

#[test]
fn discriminated_union_serializes_its_alternative_unchanged() {
    let cat = Pet::Cat(Cat { kind: "cat".into(), meow: true });

    let json = serde_json::to_value(&cat).expect("serializes");

    assert_eq!(json, serde_json::json!({"kind": "cat", "meow": true}));
    assert!(matches!(serde_json::from_value::<Pet>(json), Ok(Pet::Cat(_))));
}

#[test]
fn discriminated_union_rejects_a_missing_non_string_or_unknown_tag() {
    let cases = [
        (r#"{"meow":true}"#, "missing field `kind`"),
        (r#"{"kind":7,"meow":true}"#, "`Pet` discriminator `kind` is not a string"),
        (r#"{"kind":"lizard","meow":true}"#, "unknown `kind` value `lizard` for `Pet`"),
    ];
    for (input, expected) in cases {
        let error = serde_json::from_str::<Pet>(input).expect_err(input).to_string();

        assert!(error.contains(expected), "{input}: {error}");
    }
}

#[test]
fn discriminated_union_reports_the_tagged_alternative_failure() {
    // The tag selects `Cat`; its missing field is the error, not a fallback to `Dog`.
    let error = serde_json::from_str::<Pet>(r#"{"kind":"cat","bark":true}"#).expect_err("a cat needs `meow`").to_string();

    assert!(error.contains("`Pet` with `kind` `cat`: missing field `meow`"), "{error}");
}
