use schematic_define::{DefinitionContext, DefinitionError, TypeDesc, definition_type, rust_path};
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
        c.validate_reference("Fixture", "Get", &nested).unwrap();
        for _ in 0..2 {
            let data = serde_json::to_string(&nested).unwrap();
            let read: TypeDesc = serde_json::from_str(&data).unwrap();
            c.validate_reference("Fixture", "Get", &read).unwrap();
            assert_eq!(read, nested);
        }
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
        for d in [c.register::<HashMap<i64, Value>>().unwrap(), c.register::<HashMap<u64, Vec<Value>>>().unwrap()] { c.validate_reference("Fixture", "Get", &d).unwrap(); }
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
    #[test]
    fn destination_checks_named_paths_wrappers_inline_and_reserved_structure() {
        let mut source = DefinitionContext::types_in("::typed_registration::models");
        let original = source.register::<Option<Vec<models::Model>>>().unwrap();
        let mut destination = DefinitionContext::types_in("::typed_registration::models");
        assert!(destination.validate_reference("Fixture", "Get", &original).is_err());
        destination.register::<Option<Vec<models::Model>>>().unwrap();
        destination.validate_reference("Fixture", "Get", &original).unwrap();
        let data = serde_json::to_value(&original).unwrap();
        let mut wrong_path = data.clone();
        wrong_path["shape"]["Option"]["Vec"]["Named"]["rust_path"] = json!("unrelated::Model");
        let mut wrong_name = data.clone();
        wrong_name["shape"]["Option"]["Vec"]["Named"]["schema_name"] = json!("Missing");
        let mut structure = data;
        structure["structure"] = json!({"Named":"unsupported"});
        for forged in [wrong_path, wrong_name, structure] {
            let d: TypeDesc = serde_json::from_value(forged).unwrap();
            assert!(destination.validate_reference("Fixture", "Get", &d).is_err());
        }
        let inline = source.register::<String>().unwrap();
        destination.register::<String>().unwrap();
        destination.validate_reference("Fixture", "Get", &inline).unwrap();
        let mut forged_inline = serde_json::to_value(&inline).unwrap();
        forged_inline["shape"]["Inline"]["schema"] = json!({"type":"integer"});
        assert!(destination.validate_reference("Fixture", "Get", &serde_json::from_value(forged_inline).unwrap()).is_err());
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
            for desc in [&primitive, &array, &optional, &map] { ctx.validate_reference("Inline", "Get", desc).unwrap(); }
        )* };
    }
    check!(String, bool, char, i8, i16, i32, i64, i128, isize, u8, u16, u32, u64, u128, usize, f32, f64, (), Value);
    macro_rules! keys {
        ($($t:ty),*) => { $(
            let map = ctx.register::<HashMap<$t, Option<Vec<Value>>>>().unwrap();
            assert_eq!(map.wire_schema()["additionalProperties"]["anyOf"][0], json!({"type":"array","items":{}}));
            ctx.validate_reference("Inline", "Map", &map).unwrap();
        )* };
    }
    keys!(i8, i16, i32, i64, i128, isize, u8, u16, u32, u64, u128, usize);
    assert!(ctx.registrations().is_empty());
    assert!(matches!(ctx.register_for::<Vec<HashMap<f64, models::Model>>>("Inline", "BadMap"), Err(DefinitionError::UnsupportedKey { api, endpoint, rust_type: "f64" }) if api == "Inline" && endpoint == "BadMap"));
    assert!(ctx.registrations().is_empty());
}

#[test]
fn destination_rejects_changed_wrapper_shape_even_with_matching_leaf() {
    let mut source = DefinitionContext::types_in("::models");
    let desc = source.register::<Vec<models::Model>>().unwrap();
    let mut destination = DefinitionContext::types_in("::models");
    destination.register::<models::Model>().unwrap();
    let error = destination.validate_reference("Example", "List", &desc).unwrap_err();
    assert!(matches!(&error, DefinitionError::InvalidDescriptor { api, endpoint, rust_type } if api == "Example" && endpoint == "List" && rust_type.contains("Vec")));
    assert!(error.to_string().contains("Example endpoint List"));
    destination.register::<Vec<models::Model>>().unwrap();
    destination.validate_reference("Example", "List", &desc).unwrap();
    let mut other = DefinitionContext::types_in("::other_models");
    other.register::<Vec<models::Model>>().unwrap();
    assert!(other.validate_reference("Example", "List", &desc).is_err());
}

#[test]
fn descriptor_json_matrix_uses_public_destination_validation() {
    let mut ctx = DefinitionContext::types_in("::models");
    let nominal = ctx.register::<models::Model>().unwrap();
    let inline = ctx.register::<String>().unwrap();
    let map = ctx.register::<HashMap<String, Vec<models::Model>>>().unwrap();
    let array = ctx.register::<Vec<models::Model>>().unwrap();
    let optional = ctx.register::<Option<models::Model>>().unwrap();
    let free_form = ctx.register::<Value>().unwrap();
    let accepts = |text: &str| serde_json::from_str::<TypeDesc>(text).is_ok_and(|d| ctx.validate_reference("Matrix", "Get", &d).is_ok());
    for desc in [&nominal, &inline, &map, &array, &optional, &free_form] {
        let mut text = serde_json::to_string(desc).unwrap();
        assert!(accepts(&text));
        for _ in 0..2 {
            let read: TypeDesc = serde_json::from_str(&text).unwrap();
            ctx.validate_reference("Matrix", "Get", &read).unwrap();
            assert_eq!(read.rust_path(), desc.rust_path());
            assert_eq!(read.wire_schema(), desc.wire_schema());
            text = serde_json::to_string(&read).unwrap();
        }
        assert!(!accepts(&(text.clone()+" garbage")));
        assert!(!accepts(&text.replacen("\"shape\":", "\"shape\":null,\"shape\":", 1)));
        assert!(!accepts(&text.replacen("\"structure\":null", "\"structure\":null,\"structure\":null", 1)));
        let mut value = serde_json::to_value(desc).unwrap();
        value.as_object_mut().unwrap().remove("structure");
        assert!(accepts(&value.to_string())); // Optional transfer field: absent and null both mean unsupported data is absent.
        for invalid in [json!(false), json!(123), json!("null"), json!([]), json!({}), json!({"Named":"Extra"}), json!({"OneOf":[]}), json!({"OneOf":[{"Named":"A"},123]}), json!({"OneOf":[123]})] {
            value["structure"] = invalid;
            assert!(!accepts(&value.to_string()), "{value}");
        }
        for invalid in [Value::Null, json!(false), json!(123), json!("Named"), json!([]), json!({}), json!({"Unknown":{}})] {
            value = serde_json::to_value(desc).unwrap();
            value["shape"] = invalid;
            assert!(!accepts(&value.to_string()), "{value}");
        }
        value.as_object_mut().unwrap().remove("shape");
        assert!(!accepts(&value.to_string()));
    }
    for (original, variant, fields) in [
        (&nominal, "Named", vec!["schema_name", "rust_path"]),
        (&inline, "Inline", vec!["rust_path", "schema"]),
        (&map, "Map", vec!["key", "value"]),
    ] {
        for field in fields {
            for invalid in [Value::Null, json!(123), json!("123"), json!(false), json!("false"), json!(""), json!([]), json!(["a",123]), json!([123]), json!({})] {
                let mut data = serde_json::to_value(original).unwrap();
                data["shape"][variant][field] = invalid;
                assert!(!accepts(&data.to_string()), "{data}");
            }
            let mut data = serde_json::to_value(original).unwrap();
            data["shape"][variant].as_object_mut().unwrap().remove(field);
            assert!(!accepts(&data.to_string()));
            let text = serde_json::to_string(original).unwrap();
            let key = format!("\"{field}\":");
            assert!(!accepts(&text.replacen(&key, &format!("{key}null,{key}"), 1)));
        }
    }
    for (desc, variant) in [(&array, "Vec"), (&optional, "Option")] {
        for invalid in [Value::Null, json!(123), json!("Named"), json!({}), json!([]), json!([{"Named":{"schema_name":"Model","rust_path":"::models::Model"}},123]), json!([123])] {
            let mut value = serde_json::to_value(desc).unwrap();
            value["shape"][variant] = invalid;
            assert!(!accepts(&value.to_string()));
        }
    }
    assert_eq!(ctx.registrations().len(), 1);
    ctx.finish().unwrap();
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
    ctx.validate_reference("Fixture", "Map", &desc).unwrap();
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
