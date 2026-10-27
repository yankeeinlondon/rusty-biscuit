//! The `$ref` closure of component schemas (`RestApi::validate_schema_closure`):
//! every reference inside a component names a component, or API build,
//! bundle loading, and export fail naming the parent, the path, and the
//! missing name.

#[cfg(feature = "openapi")]
use schematic_define::openapi::{ExportOptions, OpenApiError, export};
use schematic_define::{
    BundleError, DefinitionBundle, DefinitionContext, DefinitionError, Endpoint, Registration, RestApi, SchemaComponent, SchemaReference, definition_type,
};
use schemars::JsonSchema;
use serde_json::{Value, json};

pub mod models {
    use super::*;
    definition_type!(#[derive(JsonSchema)] #[allow(dead_code)] pub struct Inner { pub id: u32 });
    definition_type!(#[derive(JsonSchema)] #[allow(dead_code)] pub struct Outer { pub inner: Inner, pub maybe: Option<Inner> });
    definition_type!(#[derive(JsonSchema)] #[allow(dead_code)] pub struct Node { pub name: String, pub children: Vec<Node>, pub parent: Option<Box<Node>> });
}

/// A built API whose only registration is `T`, returned by one endpoint.
fn api_returning<T: Registration>() -> RestApi {
    let mut context = DefinitionContext::types_in("::schema_closure::models");
    let endpoint = Endpoint::get("Get", "/thing").describes("Get the thing").returns_json::<T>(&mut context).build().unwrap();
    RestApi::builder(&context, "Fixture", "https://fixture.test").describes("Fixture").no_auth().endpoints([endpoint]).build().unwrap()
}

/// `api` with an extra nested-only component holding `schema`.
fn with_component(mut api: RestApi, name: &str, schema: Value) -> RestApi {
    api.schemas.push(SchemaComponent { name: name.into(), rust_path: None, schema });
    api
}

fn reference(parent: &str, path: &str, reference: &str, missing: Option<&str>) -> SchemaReference {
    SchemaReference { parent: parent.into(), path: path.into(), reference: reference.into(), missing: missing.map(Into::into) }
}

#[test]
fn an_unresolved_nested_reference_names_its_parent_path_and_missing_name() {
    let api = with_component(api_returning::<models::Outer>(), "Holder", json!({
        "type": "object",
        "properties": {"owner": {"anyOf": [{"$ref": "#/components/schemas/Missing"}, {"type": "null"}]}},
    }));

    let expected = DefinitionError::UnresolvedSchemaReferences {
        api: "Fixture".into(),
        references: vec![reference("Holder", "/properties/owner/anyOf/0", "#/components/schemas/Missing", Some("Missing"))],
    };
    assert_eq!(api.validate_schema_closure(), Err(expected.clone()));
    assert_eq!(expected.to_string(), "Fixture schema Holder at /properties/owner/anyOf/0: unresolved schema reference \"Missing\"");
    // The endpoint references all resolve; only the closure fails.
    assert_eq!(api.validate_references(), Ok(()));
    // Export and the builder's validation (which bundles run) block on it.
    #[cfg(feature = "openapi")]
    assert_eq!(export(&api, &ExportOptions::new()).unwrap_err(), OpenApiError::Definition(expected));
    assert_eq!(
        DefinitionBundle::new(api).unwrap_err(),
        BundleError::SchemaReference {
            api: "Fixture".into(),
            component: "Holder".into(),
            reference: "#/components/schemas/Missing".into(),
            problem: "names no component in the bundle",
        }
    );
}

#[test]
fn api_build_fails_on_a_dangling_reference_in_a_deserialized_definition() {
    // A definition read back from JSON carries its components verbatim.
    let mut api = api_returning::<models::Outer>();
    api.schemas.iter_mut().find(|c| c.name == "Outer").unwrap().schema["properties"]["inner"] = json!({"$ref": "#/components/schemas/Gone"});
    let text = serde_json::to_string(&api).unwrap();
    let read: RestApi = serde_json::from_str(&text).unwrap();
    assert_eq!(
        read.validate_schema_closure(),
        Err(DefinitionError::UnresolvedSchemaReferences {
            api: "Fixture".into(),
            references: vec![reference("Outer", "/properties/inner", "#/components/schemas/Gone", Some("Gone"))],
        })
    );
}

#[test]
fn every_unresolved_reference_is_reported_in_component_then_schema_order() {
    let api = with_component(api_returning::<models::Inner>(), "Many", json!({
        "type": "object",
        "properties": {
            "a": {"$ref": "#/components/schemas/First"},
            "b": {"type": "array", "items": {"$ref": "https://example.com/schemas/b.json"}},
            "c": {"$ref": "#/components/schemas/Inner/properties/id"},
            "d": {"type": "object", "additionalProperties": {"$ref": "#/$defs/Inner"}},
            "e": {"$ref": null},
            "f": {"$ref": 5},
            "g": {"$ref": ""},
            "h": {"$ref": "#/components/schemas/"},
        },
    }));

    let Err(DefinitionError::UnresolvedSchemaReferences { references, .. }) = api.validate_schema_closure() else { panic!("closure holds") };
    assert_eq!(references, [
        reference("Many", "/properties/a", "#/components/schemas/First", Some("First")),
        // Never fetched: another document, a pointer into a schema, and a
        // namespace outside `components.schemas`.
        reference("Many", "/properties/b/items", "https://example.com/schemas/b.json", None),
        reference("Many", "/properties/c", "#/components/schemas/Inner/properties/id", None),
        reference("Many", "/properties/d/additionalProperties", "#/$defs/Inner", None),
        // Wrong types and empty references are reported, not read as absent.
        reference("Many", "/properties/e", "null", None),
        reference("Many", "/properties/f", "5", None),
        reference("Many", "/properties/g", "", None),
        reference("Many", "/properties/h", "#/components/schemas/", None),
    ]);
    assert!(
        DefinitionError::UnresolvedSchemaReferences { api: "Fixture".into(), references: references[1..2].to_vec() }
            .to_string()
            .ends_with("unsupported reference \"https://example.com/schemas/b.json\"; only local component references resolve, and none is fetched")
    );
}

#[test]
fn escaped_names_resolve_and_instance_data_is_not_a_reference() {
    let api = with_component(api_returning::<models::Inner>(), "a/b~c<d>", json!({"type": "string"}));
    let api = with_component(api, "Escapes", json!({
        "type": "object",
        "properties": {
            // JSON-pointer escapes and percent-encoding both decode.
            "pointer": {"$ref": "#/components/schemas/a~1b~0c<d>"},
            "percent": {"$ref": "#/components/schemas/a~1b~0c%3Cd%3E"},
            // Data keywords hold values, never references.
            "data": {"type": "object", "default": {"$ref": "#/components/schemas/Nope"}, "examples": [{"$ref": "x"}],
                "enum": [{"$ref": "y"}], "const": {"$ref": "z"}},
            // A property named like a data keyword is still a schema.
            "default": {"$ref": "#/components/schemas/Inner"},
        },
    }));
    assert_eq!(api.validate_schema_closure(), Ok(()));

    let api = with_component(api, "Named", json!({"type": "object", "properties": {"enum": {"$ref": "#/components/schemas/Absent"}}}));
    let Err(DefinitionError::UnresolvedSchemaReferences { references, .. }) = api.validate_schema_closure() else { panic!("closure holds") };
    assert_eq!(references, [reference("Named", "/properties/enum", "#/components/schemas/Absent", Some("Absent"))]);
}

#[test]
fn extracted_nested_definitions_close_without_their_own_registration() {
    let api = api_returning::<models::Outer>();
    let names: Vec<_> = api.schemas.iter().map(|c| (c.name.as_str(), c.rust_path.is_some())).collect();
    // `Inner` was extracted from `Outer`'s definitions; nobody registered it.
    assert_eq!(names, [("Outer", true), ("Inner", false)]);
    assert_eq!(api.validate_schema_closure(), Ok(()));
    #[cfg(feature = "openapi")]
    export(&api, &ExportOptions::new()).unwrap();
}

#[test]
fn a_recursive_model_closes_and_exports() {
    let api = api_returning::<models::Node>();
    assert_eq!(api.schemas.len(), 1);
    let node = serde_json::to_string(&api.schemas[0].schema).unwrap();
    assert!(node.contains("\"#/components/schemas/Node\""), "Node refers to itself: {node}");
    assert_eq!(api.validate_schema_closure(), Ok(()));
    #[cfg(feature = "openapi")]
    export(&api, &ExportOptions::new()).unwrap();
}
