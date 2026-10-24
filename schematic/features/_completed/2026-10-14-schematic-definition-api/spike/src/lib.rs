//! Isolated stable-Rust design proof; no production registration API.
extern crate self as schematic_registration_spike;
use std::{any::{TypeId, type_name}, collections::HashMap, marker::PhantomData};
use schemars::{JsonSchema, generate::SchemaSettings};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
enum Shape {
    Named { schema_name: String, rust_path: String },
    Inline { rust_path: String, schema: Value },
    Vec(Box<Shape>),
    Option(Box<Shape>),
    Map { key: String, value: Box<Shape> },
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TypeDesc {
    shape: Shape,
    // Reserved structural carrier is deliberately independent of wrapper metadata.
    structure: Option<Value>,
}
impl TypeDesc {
    pub fn rust_path(&self) -> String { self.shape.rust_path() }
    pub fn wire_schema(&self) -> Value { self.shape.wire_schema() }
}
impl Shape {
    fn rust_path(&self) -> String {
        match self {
            Self::Named { rust_path, .. } | Self::Inline { rust_path, .. } => rust_path.clone(),
            Self::Vec(t) => format!("::std::vec::Vec<{}>", t.rust_path()),
            Self::Option(t) => format!("::std::option::Option<{}>", t.rust_path()),
            Self::Map { key, value } => format!("::std::collections::HashMap<{key}, {}>", value.rust_path()),
        }
    }
    fn wire_schema(&self) -> Value {
        match self {
            Self::Named { schema_name, .. } => json!({"$ref": format!("#/components/schemas/{schema_name}")}),
            Self::Inline { schema, .. } => schema.clone(),
            Self::Vec(t) => json!({"type":"array", "items":t.wire_schema()}),
            Self::Option(t) => json!({"anyOf":[t.wire_schema(), {"type":"null"}]}),
            Self::Map { value, .. } => json!({"type":"object", "additionalProperties":value.wire_schema()}),
        }
    }
}

pub struct RustPath<T> { path: &'static str, schema_name: Option<&'static str>, marker: PhantomData<fn() -> T> }
impl<T> RustPath<T> {
    #[doc(hidden)]
    pub const fn checked(path: &'static str, _: fn() -> PhantomData<T>) -> Self {
        Self { path, schema_name: None, marker: PhantomData }
    }
    pub fn schema_name(mut self, name: &'static str) -> Self { self.schema_name = Some(name); self }
}
#[macro_export]
macro_rules! rust_path {
    ($path:path) => { $crate::RustPath::<$path>::checked(stringify!($path), || ::std::marker::PhantomData::<$path>) };
}

#[derive(Debug, PartialEq)]
pub enum DefinitionError {
    Collision { schema_name: String, first: &'static str, second: &'static str },
    InvalidDescriptor,
    NotLeaf,
    UnsupportedKey { api: String, endpoint: String, rust_type: &'static str },
}
struct Entry { identity: TypeId, diagnostic: &'static str, name: String, path: String, schema: Value }
pub struct DefinitionContext { entries: Vec<Entry>, errors: Vec<DefinitionError>, module_path: String }
impl Default for DefinitionContext {
    fn default() -> Self { Self::types_in("::schematic_registration_spike::models") }
}
impl DefinitionContext {
    pub fn types_in(module_path: &str) -> Self { Self { entries: Vec::new(), errors: Vec::new(), module_path: module_path.into() } }
    #[doc(hidden)]
    pub fn register_nominal<T: Registration>(&mut self) -> Result<TypeDesc, DefinitionError> {
        let identifier = T::RUST_IDENTIFIER.ok_or(DefinitionError::NotLeaf)?;
        let path = format!("{}::{identifier}", self.module_path);
        self.leaf::<T>(&path, None)
    }
    pub fn register<T: Registration>(&mut self) -> Result<TypeDesc, DefinitionError> { T::register(self) }
    pub fn register_as<T: Registration>(&mut self, path: RustPath<T>) -> Result<TypeDesc, DefinitionError> {
        if !T::IS_LEAF { return Err(DefinitionError::NotLeaf); }
        self.leaf::<T>(path.path, path.schema_name)
    }
    fn leaf<T: JsonSchema + 'static>(&mut self, path: &str, name: Option<&str>) -> Result<TypeDesc, DefinitionError> {
        if let Some(entry) = self.entries.iter().find(|e| e.identity == TypeId::of::<T>()) {
            return Ok(TypeDesc { shape: Shape::Named { schema_name: entry.name.clone(), rust_path: entry.path.clone() }, structure: None });
        }
        let name = name.map(str::to_owned).unwrap_or_else(|| T::schema_name().into_owned());
        if let Some(entry) = self.entries.iter().find(|e| e.name == name) {
            self.errors.push(DefinitionError::Collision { schema_name: name.clone(), first: entry.diagnostic, second: type_name::<T>() });
        } else {
            self.entries.push(Entry { identity: TypeId::of::<T>(), diagnostic: type_name::<T>(), name: name.clone(), path: path.into(), schema: serde_json::to_value(SchemaSettings::openapi3().into_generator().into_root_schema_for::<T>()).unwrap() });
        }
        Ok(TypeDesc { shape: Shape::Named { schema_name: name, rust_path: path.into() }, structure: None })
    }
    pub fn finish(&self) -> Result<(), &[DefinitionError]> { if self.errors.is_empty() { Ok(()) } else { Err(&self.errors) } }
    pub fn names(&self) -> Vec<&str> { self.entries.iter().map(|e| e.name.as_str()).collect() }
    pub fn schemas(&self) -> Vec<&Value> { self.entries.iter().map(|e| &e.schema).collect() }
    pub fn validate(&self, desc: &TypeDesc) -> Result<(), DefinitionError> {
        fn valid(ctx: &DefinitionContext, shape: &Shape) -> bool {
            match shape {
                Shape::Named { schema_name, rust_path } => ctx.entries.iter().any(|e| e.name == *schema_name && e.path == *rust_path),
                Shape::Vec(t) | Shape::Option(t) => valid(ctx, t),
                Shape::Map { key, value } => matches!(key.as_str(), "::std::string::String" | "i64" | "u64") && valid(ctx, value),
                Shape::Inline { rust_path, schema } => inline_schema(rust_path).as_ref() == Some(schema),
            }
        }
        if desc.structure.is_none() && valid(self, &desc.shape) { Ok(()) } else { Err(DefinitionError::InvalidDescriptor) }
    }
}

pub trait Registration: JsonSchema + 'static {
    const IS_LEAF: bool = true;
    const RUST_IDENTIFIER: Option<&'static str> = None;
    fn register(ctx: &mut DefinitionContext) -> Result<TypeDesc, DefinitionError>;
}
// Emit alongside each nominal definition, never as a second registration inventory.
#[macro_export]
macro_rules! nominal {
    ($(#[$attr:meta])* $vis:vis struct $name:ident $body:tt) => {
        $(#[$attr])* $vis struct $name $body
        impl $crate::Registration for $name {
            const RUST_IDENTIFIER: Option<&'static str> = Some(stringify!($name));
            fn register(ctx: &mut $crate::DefinitionContext) -> Result<$crate::TypeDesc, $crate::DefinitionError> {
                ctx.register_nominal::<Self>()
            }
        }
    };
}
impl<T: Registration> Registration for Vec<T> {
    const IS_LEAF: bool = false;
    fn register(c: &mut DefinitionContext) -> Result<TypeDesc, DefinitionError> { Ok(TypeDesc { shape: Shape::Vec(Box::new(T::register(c)?.shape)), structure: None }) }
}
impl<T: Registration> Registration for Option<T> {
    const IS_LEAF: bool = false;
    fn register(c: &mut DefinitionContext) -> Result<TypeDesc, DefinitionError> { Ok(TypeDesc { shape: Shape::Option(Box::new(T::register(c)?.shape)), structure: None }) }
}
// The same trait can carry an explicit string-like key contract in production.
pub trait MapKey: JsonSchema + 'static { fn path() -> Option<&'static str>; }
impl MapKey for String { fn path() -> Option<&'static str> { Some("::std::string::String") } }
impl MapKey for i64 { fn path() -> Option<&'static str> { Some("i64") } }
impl MapKey for u64 { fn path() -> Option<&'static str> { Some("u64") } }
impl MapKey for bool { fn path() -> Option<&'static str> { None } }
impl<K: MapKey, V: Registration> Registration for HashMap<K, V> {
    const IS_LEAF: bool = false;
    fn register(c: &mut DefinitionContext) -> Result<TypeDesc, DefinitionError> {
        let key = K::path().ok_or_else(|| DefinitionError::UnsupportedKey { api: "Spike".into(), endpoint: "Map".into(), rust_type: type_name::<K>() })?;
        Ok(TypeDesc { shape: Shape::Map { key: key.into(), value: Box::new(V::register(c)?.shape) }, structure: None })
    }
}
fn inline_schema(path: &str) -> Option<Value> {
    Some(match path {
        "::std::string::String" => json!({"type":"string"}), "bool" => json!({"type":"boolean"}),
        "i64" | "u64" => json!({"type":"integer"}), "f64" => json!({"type":"number"}),
        "::serde_json::Value" => json!({}), _ => return None,
    })
}
macro_rules! inline {
    ($t:ty, $path:literal) => {
        impl Registration for $t {
            const IS_LEAF: bool = false;
            fn register(_: &mut DefinitionContext) -> Result<TypeDesc, DefinitionError> {
                Ok(TypeDesc { shape: Shape::Inline { rust_path: $path.into(), schema: inline_schema($path).unwrap() }, structure: None })
            }
        }
    };
}
inline!(String, "::std::string::String"); inline!(bool, "bool"); inline!(i64, "i64"); inline!(u64, "u64"); inline!(f64, "f64"); inline!(Value, "::serde_json::Value");
// A foreign leaf adapter must be owned by the registration-trait crate.
impl Registration for std::time::Duration {
    fn register(c: &mut DefinitionContext) -> Result<TypeDesc, DefinitionError> { c.register_as::<Self>(rust_path!(::std::time::Duration)) }
}

pub mod models {
    use super::*;
    nominal!(#[derive(JsonSchema)] pub struct Model { pub id: String });
    nominal!(#[derive(JsonSchema, Serialize, Deserialize)] #[serde(rename = "WireRenamed")] pub struct Renamed { pub value: bool });
    pub type Alias = Model;
    pub use Renamed as Reexport;
    nominal!(#[derive(JsonSchema)] #[schemars(rename = "Model")] pub struct ClashA { pub a: bool });
    nominal!(#[derive(JsonSchema)] #[schemars(rename = "Model")] pub struct ClashB { pub b: i64 });
}
impl Registration for std::ops::Range<u64> {
    fn register(c: &mut DefinitionContext) -> Result<TypeDesc, DefinitionError> { c.register_as::<Self>(rust_path!(::std::ops::Range<u64>)) }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn nominal_renamed_alias_reexport_foreign_and_generic_paths() {
        let mut c = DefinitionContext::default();
        let alias = c.register_as::<models::Model>(rust_path!(::schematic_registration_spike::models::Alias).schema_name("Historical")).unwrap();
        assert_eq!(alias.rust_path(), "::schematic_registration_spike::models::Alias");
        assert_eq!(alias.wire_schema(), json!({"$ref":"#/components/schemas/Historical"}));
        assert_eq!(c.register::<models::Model>().unwrap(), alias);
        let renamed = c.register_as::<models::Renamed>(rust_path!(::schematic_registration_spike::models::Reexport)).unwrap();
        assert_eq!(renamed.wire_schema(), json!({"$ref":"#/components/schemas/WireRenamed"}));
        assert!(renamed.rust_path().ends_with("::Reexport"));
        let foreign = c.register::<std::time::Duration>().unwrap();
        assert_eq!(foreign.rust_path(), "::std::time::Duration");
        assert_eq!(c.register::<std::ops::Range<u64>>().unwrap().rust_path(), "::std::ops::Range<u64>");
        assert_eq!(c.names(), ["Historical", "WireRenamed", "Duration", "Range_of_uint64"]);
        assert_eq!(c.schemas().len(), 4);
        assert!(c.schemas()[0].get("properties").is_some());
        c.finish().unwrap();
    }
    #[test]
    fn wrappers_inline_shape_nullability_and_order_reuse_leaf() {
        let mut c = DefinitionContext::default();
        let leaf = c.register_as::<models::Model>(rust_path!(::schematic_registration_spike::models::Alias)).unwrap();
        let nested = c.register::<Option<Vec<HashMap<String, models::Model>>>>().unwrap();
        assert_eq!(nested.rust_path(), "::std::option::Option<::std::vec::Vec<::std::collections::HashMap<::std::string::String, ::schematic_registration_spike::models::Alias>>>");
        assert_eq!(nested.wire_schema(), json!({"anyOf":[{"type":"array", "items":{"type":"object", "additionalProperties":leaf.wire_schema()}}, {"type":"null"}]}));
        assert_eq!(c.names(), ["Model"]);
        c.validate(&nested).unwrap();
        for _ in 0..2 {
            let data = serde_json::to_string(&nested).unwrap();
            let read: TypeDesc = serde_json::from_str(&data).unwrap();
            c.validate(&read).unwrap();
            assert_eq!(read, nested);
        }
        assert_eq!(c.register_as::<Vec<models::Model>>(rust_path!(Vec<models::Model>)), Err(DefinitionError::NotLeaf));
    }
    #[test]
    fn primitives_value_and_integer_map_keys_create_no_components() {
        let mut c = DefinitionContext::default();
        assert_eq!(c.register::<Vec<String>>().unwrap().wire_schema(), json!({"type":"array","items":{"type":"string"}}));
        assert_eq!(c.register::<Option<bool>>().unwrap().wire_schema(), json!({"anyOf":[{"type":"boolean"},{"type":"null"}]}));
        assert_eq!(c.register::<i64>().unwrap().wire_schema(), json!({"type":"integer"}));
        assert_eq!(c.register::<u64>().unwrap().wire_schema(), json!({"type":"integer"}));
        assert_eq!(c.register::<f64>().unwrap().wire_schema(), json!({"type":"number"}));
        type FreeForm = Value;
        assert_eq!(c.register::<FreeForm>().unwrap().wire_schema(), json!({}));
        for d in [c.register::<HashMap<i64, Value>>().unwrap(), c.register::<HashMap<u64, Vec<Value>>>().unwrap()] { c.validate(&d).unwrap(); }
        assert!(c.names().is_empty());
        assert_eq!(c.register::<HashMap<bool, models::Model>>(), Err(DefinitionError::UnsupportedKey { api:"Spike".into(), endpoint:"Map".into(), rust_type:"bool" }));
        assert!(c.names().is_empty());
    }
    #[test]
    fn collision_aggregation_is_ordered_and_never_merges_distinct_types() {
        let mut c = DefinitionContext::default();
        c.register::<models::Model>().unwrap();
        c.register::<models::ClashA>().unwrap();
        c.register::<models::ClashB>().unwrap();
        assert_eq!(c.names(), ["Model"]);
        let errors = c.finish().unwrap_err();
        assert_eq!(errors.len(), 2);
        assert!(matches!(&errors[0], DefinitionError::Collision { schema_name, first, second } if schema_name == "Model" && first.ends_with("::Model") && second.ends_with("::ClashA")));
        assert!(matches!(&errors[1], DefinitionError::Collision { second, .. } if second.ends_with("::ClashB")));
    }
    #[test]
    fn destination_checks_named_paths_wrappers_inline_and_reserved_structure() {
        let mut source = DefinitionContext::default();
        let original = source.register::<Option<Vec<models::Model>>>().unwrap();
        let mut destination = DefinitionContext::default();
        assert_eq!(destination.validate(&original), Err(DefinitionError::InvalidDescriptor));
        destination.register::<models::Model>().unwrap();
        destination.validate(&original).unwrap();
        let data = serde_json::to_value(&original).unwrap();
        let mut wrong_path = data.clone();
        wrong_path["shape"]["Option"]["Vec"]["Named"]["rust_path"] = json!("unrelated::Model");
        let mut wrong_name = data.clone();
        wrong_name["shape"]["Option"]["Vec"]["Named"]["schema_name"] = json!("Missing");
        let mut structure = data;
        structure["structure"] = json!({"reserved":"unsupported"});
        for forged in [wrong_path, wrong_name, structure] {
            let d: TypeDesc = serde_json::from_value(forged).unwrap();
            assert_eq!(destination.validate(&d), Err(DefinitionError::InvalidDescriptor));
        }
        let mut forged_inline = serde_json::to_value(source.register::<String>().unwrap()).unwrap();
        forged_inline["shape"]["Inline"]["schema"] = json!({"type":"integer"});
        assert_eq!(destination.validate(&serde_json::from_value(forged_inline).unwrap()), Err(DefinitionError::InvalidDescriptor));
    }
}
