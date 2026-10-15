//! Typed registrations for one generated module group.
//!
//! Deserialization creates untrusted transfer data. Call
//! [`DefinitionContext::validate_reference`] before using it in a definition.
//! The first registration of a canonical Rust type fixes its name and path.
//!
//! ```compile_fail
//! use schematic_define::TypeDesc;
//! let descriptor = TypeDesc::new("Model");
//! ```
//! ```compile_fail
//! use schematic_define::{DefinitionContext, TypeRef};
//! let mut ctx = DefinitionContext::types_in("::models");
//! ctx.register::<String>().unwrap().structure(TypeRef::Named("Model".into()));
//! ```
//! ```compile_fail
//! let path = schematic_define::rust_path!(::std::string::Misspelled);
//! ```
//! ```compile_fail
//! use schematic_define::{DefinitionContext, rust_path};
//! let mut ctx = DefinitionContext::types_in("::models");
//! ctx.register_as::<std::time::Duration>(rust_path!(::std::ops::Range<u64>));
//! ```

use std::{any::{TypeId, type_name}, collections::HashMap, marker::PhantomData};
use schemars::{JsonSchema, Schema, generate::SchemaSettings};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use crate::TypeRef;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
enum Shape {
    Named { schema_name: String, rust_path: String },
    Inline { rust_path: String, schema: Value },
    Vec(Box<Shape>),
    Option(Box<Shape>),
    Map { key: String, value: Box<Shape> },
}

/// Transfer representation of a registered type, including its Rust wrappers.
///
/// Fields are private: registration is the authoring constructor. Data received
/// through Serde must pass destination-context validation before use.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TypeDesc {
    shape: Shape,
    structure: Option<TypeRef>,
}
impl TypeDesc {
    /// Fully qualified Rust spelling, preserving aliases and container shape.
    pub fn rust_path(&self) -> String { self.shape.rust_path() }
    pub(crate) fn root_array_item(&self) -> Option<String> {
        match &self.shape { Shape::Vec(item) => Some(item.rust_path()), _ => None }
    }
    /// Wire schema with container shape and nominal component references.
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
            Self::Option(t) => {
                let mut schema = schemars::json_schema!({"anyOf":[t.wire_schema(), {"type":"null"}]});
                for mut transform in SchemaSettings::openapi3().transforms {
                    transform.transform(&mut schema);
                }
                schema.to_value()
            },
            Self::Map { value, .. } => json!({"type":"object", "additionalProperties":value.wire_schema()}),
        }
    }
}

/// A type-checked authored path produced by [`crate::rust_path!`].
pub struct RustPath<T> { path: &'static str, schema_name: Option<&'static str>, marker: PhantomData<fn() -> T> }
impl<T> RustPath<T> {
    #[doc(hidden)]
    pub const fn checked(path: &'static str, _: fn() -> PhantomData<T>) -> Self {
        Self { path, schema_name: None, marker: PhantomData }
    }
    /// Override the component name without changing the Rust spelling.
    pub fn schema_name(mut self, name: &'static str) -> Self { self.schema_name = Some(name); self }
}
/// Prove an authored path resolves to the type required by `register_as`.
#[macro_export]
macro_rules! rust_path {
    ($path:path) => { $crate::RustPath::<$path>::checked(stringify!($path), || ::std::marker::PhantomData::<$path>) };
}

/// Registration or destination validation failure.
#[derive(Clone, Debug, PartialEq, thiserror::Error)]
pub enum DefinitionError {
    /// Distinct canonical types cannot share a component name.
    #[error("schema name \"{schema_name}\" is claimed by {first} and {second}; register an alias or rename one")]
    Collision { schema_name: String, first: &'static str, second: &'static str },
    /// All collected collisions, in encounter order.
    #[error("{}", .0.iter().map(ToString::to_string).collect::<Vec<_>>().join("\n"))]
    Collisions(Vec<DefinitionError>),
    /// A transferred descriptor does not match a destination registration.
    #[error("{api} endpoint {endpoint}: invalid type descriptor {rust_type}")]
    InvalidDescriptor { api: String, endpoint: String, rust_type: String },
    /// Invalid authored facts, with assembly context.
    #[error("{api} endpoint {endpoint}: {message}")]
    InvalidDefinition { api: String, endpoint: String, message: String },
    /// Containers, primitives, and free-form JSON do not define components.
    #[error("register_as requires a nominal leaf type")]
    NotLeaf,
    /// JSON object keys need a supported Serde string representation.
    #[error("{api} endpoint {endpoint}: unsupported map key {rust_type}")]
    UnsupportedKey { api: String, endpoint: String, rust_type: &'static str },
}

impl DefinitionError {
    /// Render registration errors with the API being assembled, one collision
    /// per line. Reference and key errors already carry their endpoint context.
    pub fn diagnostics(&self, api: &str) -> String {
        match self {
            Self::Collisions(errors) => errors.iter().map(|e| e.diagnostics(api)).collect::<Vec<_>>().join("\n"),
            Self::InvalidDescriptor { .. } | Self::UnsupportedKey { .. } => self.to_string(),
            _ => format!("API {api}: {self}"),
        }
    }
}

/// Provider-neutral component; order is owned by its context.
#[derive(Debug)]
pub struct TypeRegistration {
    identity: TypeId,
    diagnostic: &'static str,
    name: String,
    path: String,
    schema: Schema,
}
impl TypeRegistration {
    /// Component name, independent of the Rust path.
    pub fn schema_name(&self) -> &str { &self.name }
    /// Fully qualified Rust leaf path.
    pub fn rust_path(&self) -> &str { &self.path }
    /// Schema generated with the registry's OpenAPI-compatible settings.
    pub fn schema(&self) -> &Schema { &self.schema }
}

/// Owns canonical registrations and collision diagnostics for one module group.
#[derive(Debug)]
pub struct DefinitionContext {
    entries: Vec<TypeRegistration>,
    errors: Vec<DefinitionError>,
    descriptors: Vec<TypeDesc>,
    module_path: String,
}
impl DefinitionContext {
    /// State the generated types module once; it must be publicly accessible
    /// from the generated crate. Generated-client compilation proves visibility.
    pub fn types_in(module_path: impl Into<String>) -> Self {
        Self { entries: Vec::new(), errors: Vec::new(), descriptors: Vec::new(), module_path: module_path.into() }
    }
    #[doc(hidden)]
    pub fn register_nominal<T: Registration>(&mut self) -> Result<TypeDesc, DefinitionError> {
        let identifier = T::RUST_IDENTIFIER.ok_or(DefinitionError::NotLeaf)?;
        let path = format!("{}::{identifier}", self.module_path);
        self.leaf::<T>(&path, None)
    }
    /// Register a nominal type or recursively register a container's leaves.
    pub fn register<T: Registration>(&mut self) -> Result<TypeDesc, DefinitionError> {
        self.register_for::<T>("<module group>", "<registration>")
    }
    /// Registration with diagnostic context, used by endpoint authoring.
    pub fn register_for<T: Registration>(&mut self, api: &str, endpoint: &str) -> Result<TypeDesc, DefinitionError> {
        let desc = T::register(self).map_err(|e| match e {
            DefinitionError::UnsupportedKey { rust_type, .. } => DefinitionError::UnsupportedKey { api: api.into(), endpoint: endpoint.into(), rust_type },
            other => other,
        })?;
        self.remember(desc)
    }
    /// Exceptional naming must precede the first implicit registration of T.
    /// Repeated registration reuses the original name, path, and position.
    pub fn register_as<T: Registration>(&mut self, path: RustPath<T>) -> Result<TypeDesc, DefinitionError> {
        if !T::IS_LEAF { return Err(DefinitionError::NotLeaf); }
        let desc = self.leaf::<T>(path.path, path.schema_name)?;
        self.remember(desc)
    }
    fn remember(&mut self, desc: TypeDesc) -> Result<TypeDesc, DefinitionError> {
        if !self.descriptors.contains(&desc) { self.descriptors.push(desc.clone()); }
        Ok(desc)
    }
    fn leaf<T: JsonSchema + 'static>(&mut self, path: &str, name: Option<&str>) -> Result<TypeDesc, DefinitionError> {
        if let Some(entry) = self.entries.iter().find(|e| e.identity == TypeId::of::<T>()) {
            return Ok(TypeDesc { shape: Shape::Named { schema_name: entry.name.clone(), rust_path: entry.path.clone() }, structure: None });
        }
        let name = name.map(str::to_owned).unwrap_or_else(|| T::schema_name().into_owned());
        if let Some(entry) = self.entries.iter().find(|e| e.name == name) {
            let error = DefinitionError::Collision { schema_name: name, first: entry.diagnostic, second: type_name::<T>() };
            if !self.errors.contains(&error) { self.errors.push(error.clone()); }
            return Err(error);
        }
        self.entries.push(TypeRegistration { identity: TypeId::of::<T>(), diagnostic: type_name::<T>(), name: name.clone(), path: path.into(), schema: SchemaSettings::openapi3().into_generator().into_root_schema_for::<T>() });
        Ok(TypeDesc { shape: Shape::Named { schema_name: name, rust_path: path.into() }, structure: None })
    }
    /// Fail assembly with every distinct collision seen so far.
    pub fn finish(&self) -> Result<(), DefinitionError> {
        if self.errors.is_empty() { Ok(()) } else { Err(DefinitionError::Collisions(self.errors.clone())) }
    }
    /// Ordered nominal entries, including explicit types unused by endpoints.
    pub fn registrations(&self) -> &[TypeRegistration] { &self.entries }
    /// Accept transferred data only when the destination registered the same
    /// component, Rust path, and wrapper shape. Reserved structure is unsupported.
    pub fn validate_reference(&self, api: &str, endpoint: &str, desc: &TypeDesc) -> Result<(), DefinitionError> {
        if desc.structure.is_none() && self.descriptors.contains(desc) {
            Ok(())
        } else {
            Err(DefinitionError::InvalidDescriptor { api: api.into(), endpoint: endpoint.into(), rust_type: desc.rust_path() })
        }
    }
}

/// Disjoint dispatch: nominal implementations are emitted beside declarations;
/// containers recurse without a blanket `T: JsonSchema` implementation.
pub trait Registration: JsonSchema + 'static {
    /// Only nominal leaves may be explicitly named.
    const IS_LEAF: bool = true;
    /// Rust identifier proved by the nominal declaration, never a schema name.
    const RUST_IDENTIFIER: Option<&'static str> = None;
    /// Implementations should use the context's registration entry points.
    fn register(ctx: &mut DefinitionContext) -> Result<TypeDesc, DefinitionError>;
}
/// Declare a nominal model and its registration implementation together.
#[macro_export]
macro_rules! definition_type {
    ($(#[$attr:meta])* $vis:vis struct $name:ident ($($fields:tt)*);) => {
        $(#[$attr])* $vis struct $name ($($fields)*);
        $crate::definition_type!(@impl $name);
    };
    ($(#[$attr:meta])* $vis:vis struct $name:ident;) => {
        $(#[$attr])* $vis struct $name;
        $crate::definition_type!(@impl $name);
    };
    ($(#[$attr:meta])* $vis:vis struct $name:ident $body:tt) => {
        $(#[$attr])* $vis struct $name $body
        $crate::definition_type!(@impl $name);
    };
    ($(#[$attr:meta])* $vis:vis enum $name:ident $body:tt) => {
        $(#[$attr])* $vis enum $name $body
        $crate::definition_type!(@impl $name);
    };
    (@impl $name:ident) => {
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
    fn register(c: &mut DefinitionContext) -> Result<TypeDesc, DefinitionError> { Ok(TypeDesc { shape: Shape::Vec(Box::new(c.register::<T>()?.shape)), structure: None }) }
}
impl<T: Registration> Registration for Option<T> {
    const IS_LEAF: bool = false;
    fn register(c: &mut DefinitionContext) -> Result<TypeDesc, DefinitionError> { Ok(TypeDesc { shape: Shape::Option(Box::new(c.register::<T>()?.shape)), structure: None }) }
}
/// Explicit Serde object-key contract. String-like newtypes may implement this
/// only when Serde encodes them as strings; `path` must name their public type.
pub trait MapKey: JsonSchema + 'static { fn path() -> Option<&'static str>; }
impl MapKey for String { fn path() -> Option<&'static str> { Some("::std::string::String") } }
impl MapKey for bool { fn path() -> Option<&'static str> { None } }
impl MapKey for f32 { fn path() -> Option<&'static str> { None } }
impl MapKey for f64 { fn path() -> Option<&'static str> { None } }
impl<K: MapKey, V: Registration> Registration for HashMap<K, V> {
    const IS_LEAF: bool = false;
    fn register(c: &mut DefinitionContext) -> Result<TypeDesc, DefinitionError> {
        let key = K::path().ok_or(DefinitionError::UnsupportedKey { api: "<module group>".into(), endpoint: "<registration>".into(), rust_type: type_name::<K>() })?;
        Ok(TypeDesc { shape: Shape::Map { key: key.into(), value: Box::new(c.register::<V>()?.shape) }, structure: None })
    }
}
macro_rules! inline {
    ($t:ty, $path:expr) => {
        impl Registration for $t {
            const IS_LEAF: bool = false;
            fn register(_: &mut DefinitionContext) -> Result<TypeDesc, DefinitionError> {
                let schema = SchemaSettings::openapi3().into_generator().into_root_schema_for::<Self>();
                let mut schema = schema.to_value();
                if let Some(object) = schema.as_object_mut() { object.remove("$schema"); object.remove("title"); }
                Ok(TypeDesc { shape: Shape::Inline { rust_path: $path.into(), schema }, structure: None })
            }
        }
    };
}
macro_rules! integer {
    ($($t:ty),*) => { $(
        inline!($t, stringify!($t));
        impl MapKey for $t { fn path() -> Option<&'static str> { Some(stringify!($t)) } }
    )* };
}
integer!(i8, i16, i32, i64, i128, isize, u8, u16, u32, u64, u128, usize);
inline!(String, "::std::string::String");
inline!(bool, "bool"); inline!(f32, "f32"); inline!(f64, "f64"); inline!(char, "char");
inline!(Value, "::serde_json::Value"); inline!((), "()");
// Foreign leaf implementations belong here: downstream crates cannot implement
// this trait for types they do not own.
impl Registration for std::time::Duration {
    fn register(c: &mut DefinitionContext) -> Result<TypeDesc, DefinitionError> { c.register_as::<Self>(rust_path!(::std::time::Duration)) }
}
impl Registration for std::ops::Range<u64> {
    fn register(c: &mut DefinitionContext) -> Result<TypeDesc, DefinitionError> { c.register_as::<Self>(rust_path!(::std::ops::Range<u64>)) }
}
