use schematic_define::{DefinitionContext, DefinitionError, PrimitiveType, TypeRef, definition_type, rust_path};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::collections::HashMap;
pub mod models {
    use super::*;
    definition_type!(#[derive(JsonSchema)] pub struct Model { pub id: String });
    definition_type!(#[derive(JsonSchema, Serialize, Deserialize)] #[serde(rename = "WireRenamed")] pub struct Renamed { pub value: bool });
    pub type Alias = Model;
    pub use Renamed as Reexport;
    definition_type!(#[derive(JsonSchema)] #[schemars(rename = "Model")] pub struct ClashA { pub a: bool });
    definition_type!(#[derive(JsonSchema)] #[schemars(rename = "Model")] pub struct ClashB { pub b: i64 });
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn nominal_renamed_alias_reexport_foreign_and_generic_paths() {
        let mut c = DefinitionContext::types_in("::typed_registration::models");
        let alias = c.register_as::<models::Model>(rust_path!(crate::models::Alias).schema_name("Historical")).unwrap();
        assert_eq!(alias.rust_path(), "crate::models::Alias");
        assert_eq!(alias.wire_schema(), json!({"$ref":"#/components/schemas/Historical"}));
        assert_eq!(c.register::<models::Model>().unwrap(), alias);
        let renamed = c.register_as::<models::Renamed>(rust_path!(crate::models::Reexport)).unwrap();
        assert_eq!(renamed.wire_schema(), json!({"$ref":"#/components/schemas/WireRenamed"}));
        assert!(renamed.rust_path().ends_with("::Reexport"));
        let foreign = c.register::<std::time::Duration>().unwrap();
        assert_eq!(foreign.rust_path(), "::std::time::Duration");
        assert_eq!(c.register::<std::ops::Range<u64>>().unwrap().rust_path(), "::std::ops::Range<u64>");
        assert_eq!(c.registrations().iter().map(|r| r.schema_name()).collect::<Vec<_>>(), ["Historical", "WireRenamed", "Duration", "Range_of_uint64"]);
        assert_eq!(c.registrations().len(), 4);
        assert!(c.registrations()[0].schema().as_value().get("properties").is_some());
        c.finish().unwrap();
    }
    #[test]
    fn wrappers_inline_shape_nullability_and_order_reuse_leaf() {
        let mut c = DefinitionContext::types_in("::typed_registration::models");
        let leaf = c.register_as::<models::Model>(rust_path!(crate::models::Alias)).unwrap();
        let nested = c.register::<Option<Vec<HashMap<String, models::Model>>>>().unwrap();
        assert_eq!(nested.rust_path(), "::std::option::Option<::std::vec::Vec<::std::collections::HashMap<::std::string::String, crate::models::Alias>>>");
        assert_eq!(nested.wire_schema(), json!({"anyOf":[{"type":"array", "items":{"type":"object", "additionalProperties":leaf.wire_schema()}}, {"nullable":true,"enum":[null]}]}));
        assert_eq!(c.registrations().iter().map(|r| r.schema_name()).collect::<Vec<_>>(), ["Model"]);
        let model = || Box::new(TypeRef::Named("Model".into()));
        assert_eq!(nested.type_ref(), &TypeRef::Optional(Box::new(TypeRef::Array(Box::new(TypeRef::Map(model()))))));
        assert_eq!(c.register_as::<Vec<models::Model>>(rust_path!(Vec<models::Model>)), Err(DefinitionError::NotLeaf));
    }
    #[test]
    fn primitives_value_and_integer_map_keys_create_no_components() {
        let mut c = DefinitionContext::types_in("::typed_registration::models");
        assert_eq!(c.register::<Vec<String>>().unwrap().wire_schema(), json!({"type":"array","items":{"type":"string"}}));
        assert_eq!(c.register::<Option<bool>>().unwrap().wire_schema(), json!({"anyOf":[{"type":"boolean"},{"nullable":true,"enum":[null]}]}));
        assert_eq!(c.register::<i64>().unwrap().wire_schema()["type"], "integer");
        assert_eq!(c.register::<u64>().unwrap().wire_schema()["type"], "integer");
        assert_eq!(c.register::<f64>().unwrap().wire_schema()["type"], "number");
        type FreeForm = Value;
        assert_eq!(c.register::<FreeForm>().unwrap().wire_schema(), json!({}));
        let json = || Box::new(TypeRef::Primitive(PrimitiveType::Json));
        assert_eq!(c.register::<HashMap<i64, Value>>().unwrap().type_ref(), &TypeRef::KeyedMap { key: "i64".into(), value: json() });
        assert_eq!(c.register::<HashMap<u64, Vec<Value>>>().unwrap().type_ref(), &TypeRef::KeyedMap { key: "u64".into(), value: Box::new(TypeRef::Array(json())) });
        assert!(c.registrations().iter().map(|r| r.schema_name()).collect::<Vec<_>>().is_empty());
        assert_eq!(c.register::<HashMap<bool, models::Model>>(), Err(DefinitionError::UnsupportedKey { api:"<module group>".into(), endpoint:"<registration>".into(), rust_type:"bool" }));
        assert!(c.registrations().iter().map(|r| r.schema_name()).collect::<Vec<_>>().is_empty());
    }
    #[test]
    fn collision_aggregation_is_ordered_and_never_merges_distinct_types() {
        let mut c = DefinitionContext::types_in("::typed_registration::models");
        c.register::<models::Model>().unwrap();
        assert!(c.register::<models::ClashA>().is_err());
        assert!(c.register::<models::ClashB>().is_err());
        assert_eq!(c.registrations().iter().map(|r| r.schema_name()).collect::<Vec<_>>(), ["Model"]);
        let error = c.finish().unwrap_err();
        let diagnostic = error.diagnostics("Fixture");
        let lines: Vec<_> = diagnostic.lines().collect();
        assert_eq!(lines.len(), 2);
        assert!(lines.iter().all(|line| line.starts_with("API Fixture: schema name \"Model\" is claimed by ") && line.ends_with("; register an alias or rename one")));
        let DefinitionError::Collisions(errors) = error else { panic!("expected collisions") };
        assert_eq!(errors.len(), 2);
        assert!(matches!(&errors[0], DefinitionError::Collision { schema_name, first, second } if schema_name == "Model" && first.ends_with("::Model") && second.ends_with("::ClashA")));
        assert!(matches!(&errors[1], DefinitionError::Collision { second, .. } if second.ends_with("::ClashB")));
    }
}

#[test]
fn every_primitive_and_map_integer_key_stays_inline() {
    let mut ctx = DefinitionContext::types_in("::unused");
    macro_rules! check {
        ($($t:ty),*) => { $(
            let primitive = ctx.register::<$t>().unwrap();
            let array = ctx.register::<Vec<$t>>().unwrap();
            let optional = ctx.register::<Option<$t>>().unwrap();
            let map = ctx.register::<HashMap<String, $t>>().unwrap();
            assert_eq!(array.wire_schema()["items"], primitive.wire_schema());
            assert_eq!(optional.wire_schema()["anyOf"][0], primitive.wire_schema());
            assert_eq!(optional.wire_schema()["anyOf"][1]["nullable"], true);
            assert_eq!(map.wire_schema()["additionalProperties"], primitive.wire_schema());
            let TypeRef::Primitive(stored) = primitive.type_ref() else { panic!("{} is not primitive", primitive.rust_path()) };
            assert_eq!((stored.rust_path(), stored.wire_schema()), (primitive.rust_path(), primitive.wire_schema()));
        )* };
    }
    check!(String, bool, char, i8, i16, i32, i64, i128, isize, u8, u16, u32, u64, u128, usize, f32, f64, (), Value);
    macro_rules! keys {
        ($($t:ty),*) => { $(
            let map = ctx.register::<HashMap<$t, Option<Vec<Value>>>>().unwrap();
            assert_eq!(map.wire_schema()["additionalProperties"]["anyOf"][0], json!({"type":"array","items":{}}));
            assert!(matches!(map.type_ref(), TypeRef::KeyedMap { key, .. } if key == stringify!($t)));
        )* };
    }
    keys!(i8, i16, i32, i64, i128, isize, u8, u16, u32, u64, u128, usize);
    assert!(ctx.registrations().is_empty());
    assert!(matches!(ctx.register_for::<Vec<HashMap<f64, models::Model>>>("Inline", "BadMap"), Err(DefinitionError::UnsupportedKey { api, endpoint, rust_type: "f64" }) if api == "Inline" && endpoint == "BadMap"));
    assert!(ctx.registrations().is_empty());
}

#[derive(JsonSchema, Serialize)]
#[allow(dead_code)]
struct StringKey(String);
impl schematic_define::MapKey for StringKey {
    fn path() -> Option<&'static str> { Some("crate::StringKey") }
}
#[test]
fn string_newtype_key_requires_explicit_contract() {
    let mut ctx = DefinitionContext::types_in("::models");
    let desc = ctx.register::<HashMap<StringKey, Value>>().unwrap();
    assert_eq!(desc.rust_path(), "::std::collections::HashMap<crate::StringKey, ::serde_json::Value>");
    assert_eq!(desc.wire_schema(), json!({"type":"object","additionalProperties":{}}));
    assert_eq!(desc.type_ref(), &TypeRef::KeyedMap { key: "crate::StringKey".into(), value: Box::new(TypeRef::Primitive(PrimitiveType::Json)) });
    assert!(ctx.registrations().is_empty());
}

mod left {
    use super::*;
    definition_type!(#[derive(JsonSchema)] #[allow(dead_code)] pub struct Identical { value: bool });
}
mod right {
    use super::*;
    definition_type!(#[derive(JsonSchema)] #[allow(dead_code)] pub struct Identical { value: String });
}
definition_type!(#[derive(JsonSchema)] #[allow(dead_code)] enum Choice { Text(String), Number(i64) });
definition_type!(#[derive(JsonSchema)] #[allow(dead_code)] struct NominalCollection(Vec<String>););
definition_type!(#[derive(JsonSchema)] struct UnitModel;);

#[test]
fn same_names_in_different_modules_collide_and_nominal_containers_stay_components() {
    let mut ctx = DefinitionContext::types_in("::models");
    ctx.register_as::<left::Identical>(rust_path!(crate::left::Identical)).unwrap();
    assert!(matches!(ctx.register_as::<right::Identical>(rust_path!(crate::right::Identical)), Err(DefinitionError::Collision { schema_name, .. }) if schema_name == "Identical"));
    assert!(ctx.finish().unwrap_err().to_string().contains("left::Identical"));
    assert!(ctx.finish().unwrap_err().to_string().contains("right::Identical"));
    let choice = ctx.register::<Choice>().unwrap();
    let collection = ctx.register::<NominalCollection>().unwrap();
    let unit = ctx.register::<UnitModel>().unwrap();
    assert_eq!(unit.wire_schema(), json!({"$ref":"#/components/schemas/UnitModel"}));
    assert_eq!(choice.wire_schema(), json!({"$ref":"#/components/schemas/Choice"}));
    assert_eq!(collection.wire_schema(), json!({"$ref":"#/components/schemas/NominalCollection"}));
    assert_eq!(ctx.registrations().iter().map(|e| e.schema_name()).collect::<Vec<_>>(), ["Identical", "Choice", "NominalCollection", "UnitModel"]);
}
