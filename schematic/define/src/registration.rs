//! Typed registrations for one generated module group.
//!
//! A [`TypeDesc`] is an authoring handle: builders lower it to a
//! [`TypeRef`] naming components, and API build stores the context's
//! components on the API (see [`crate::SchemaComponent`]).
//! The first registration of a canonical Rust type fixes its name and path.
//!
//! ```compile_fail
//! use schematic_define::TypeDesc;
//! let descriptor = TypeDesc::new("Model");
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
use serde_json::Value;
use crate::{PrimitiveType, SchemaComponent, TypeRef};

/// A registered type: the reference builders store, and its Rust spelling.
///
/// Fields are private: registration is the only constructor.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TypeDesc {
    reference: TypeRef,
    rust_path: String,
}
impl TypeDesc {
    fn new(reference: TypeRef, rust_path: String) -> Self { Self { reference, rust_path } }
    /// Fully qualified Rust spelling, preserving aliases and container shape.
    pub fn rust_path(&self) -> &str { &self.rust_path }
    /// Wire schema with container shape and nominal component references.
    pub fn wire_schema(&self) -> Value { self.reference.wire_schema() }
    /// The stored reference: component names plus wrapper shape.
    pub fn type_ref(&self) -> &TypeRef { &self.reference }
    /// Lower to the stored reference.
    pub fn into_type_ref(self) -> TypeRef { self.reference }
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
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub enum DefinitionError {
    /// Distinct canonical types cannot share a component name.
    #[error("schema name \"{schema_name}\" is claimed by {first} and {second}; register an alias or rename one")]
    Collision { schema_name: String, first: &'static str, second: &'static str },
    /// Two sources define one component name with different wire schemas.
    /// A source is a registered Rust type or a schema nested in one.
    #[error("schema component \"{schema_name}\" has different definitions from {first} and {second}; rename one")]
    ComponentConflict { schema_name: String, first: String, second: String },
    /// All collected collisions, in encounter order.
    #[error("{}", .0.iter().map(ToString::to_string).collect::<Vec<_>>().join("\n"))]
    Collisions(Vec<DefinitionError>),
    /// A reference names a component the API does not carry.
    #[error("{api} endpoint {endpoint}: unresolved schema reference \"{name}\"")]
    UnresolvedReference { api: String, endpoint: String, name: String },
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
            Self::UnresolvedReference { .. } | Self::UnsupportedKey { .. } => self.to_string(),
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
    /// Schema generated with the context's OpenAPI-compatible settings.
    pub fn schema(&self) -> &Schema { &self.schema }
}

/// Owns canonical registrations and collision diagnostics for one module group.
#[derive(Debug)]
pub struct DefinitionContext {
    entries: Vec<TypeRegistration>,
    errors: Vec<DefinitionError>,
    module_path: String,
}
impl DefinitionContext {
    /// State the generated types module once; it must be publicly accessible
    /// from the generated crate. Generated-client compilation proves visibility.
    pub fn types_in(module_path: impl Into<String>) -> Self {
        Self { entries: Vec::new(), errors: Vec::new(), module_path: module_path.into() }
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
        T::register(self).map_err(|e| match e {
            DefinitionError::UnsupportedKey { rust_type, .. } => DefinitionError::UnsupportedKey { api: api.into(), endpoint: endpoint.into(), rust_type },
            other => other,
        })
    }
    /// Exceptional naming must precede the first implicit registration of T.
    /// Repeated registration reuses the original name, path, and position.
    pub fn register_as<T: Registration>(&mut self, path: RustPath<T>) -> Result<TypeDesc, DefinitionError> {
        if !T::IS_LEAF { return Err(DefinitionError::NotLeaf); }
        self.leaf::<T>(path.path, path.schema_name)
    }
    fn leaf<T: JsonSchema + 'static>(&mut self, path: &str, name: Option<&str>) -> Result<TypeDesc, DefinitionError> {
        if let Some(entry) = self.entries.iter().find(|e| e.identity == TypeId::of::<T>()) {
            return Ok(TypeDesc::new(TypeRef::Named(entry.name.clone()), entry.path.clone()));
        }
        let name = name.map(str::to_owned).unwrap_or_else(|| T::schema_name().into_owned());
        if let Some(entry) = self.entries.iter().find(|e| e.name == name) {
            let error = DefinitionError::Collision { schema_name: name, first: entry.diagnostic, second: type_name::<T>() };
            if !self.errors.contains(&error) { self.errors.push(error.clone()); }
            return Err(error);
        }
        self.entries.push(TypeRegistration { identity: TypeId::of::<T>(), diagnostic: type_name::<T>(), name: name.clone(), path: path.into(), schema: SchemaSettings::openapi3().into_generator().into_root_schema_for::<T>() });
        Ok(TypeDesc::new(TypeRef::Named(name), path.into()))
    }
    /// Fail assembly with every distinct collision seen so far.
    pub fn finish(&self) -> Result<(), DefinitionError> {
        if self.errors.is_empty() { Ok(()) } else { Err(DefinitionError::Collisions(self.errors.clone())) }
    }
    /// Ordered nominal entries, including explicit types unused by endpoints.
    pub fn registrations(&self) -> &[TypeRegistration] { &self.entries }
    /// Every component this context defines, in registration order, with
    /// nested `$defs` flattened into their own entries.
    ///
    /// ## Errors
    ///
    /// The collected registration collisions of [`Self::finish`]; otherwise
    /// [`DefinitionError::Collisions`] of every
    /// [`DefinitionError::ComponentConflict`], where registered or nested
    /// schemas give one name different wire schemas.
    pub fn components(&self) -> Result<Vec<SchemaComponent>, DefinitionError> {
        self.finish()?;
        crate::components::flatten(self.entries.iter().map(|e| (e.name.as_str(), e.path.as_str(), e.diagnostic, e.schema.as_value())))
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
    fn register(c: &mut DefinitionContext) -> Result<TypeDesc, DefinitionError> {
        let item = c.register::<T>()?;
        Ok(TypeDesc::new(TypeRef::Array(Box::new(item.reference)), format!("::std::vec::Vec<{}>", item.rust_path)))
    }
}
impl<T: Registration> Registration for Option<T> {
    const IS_LEAF: bool = false;
    fn register(c: &mut DefinitionContext) -> Result<TypeDesc, DefinitionError> {
        let inner = c.register::<T>()?;
        Ok(TypeDesc::new(TypeRef::Optional(Box::new(inner.reference)), format!("::std::option::Option<{}>", inner.rust_path)))
    }
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
        let value = c.register::<V>()?;
        let rust_path = format!("::std::collections::HashMap<{key}, {}>", value.rust_path);
        let reference = if key == PrimitiveType::String.rust_path() {
            TypeRef::Map(Box::new(value.reference))
        } else {
            TypeRef::KeyedMap { key: key.into(), value: Box::new(value.reference) }
        };
        Ok(TypeDesc::new(reference, rust_path))
    }
}
macro_rules! inline {
    ($t:ty, $primitive:ident) => {
        impl Registration for $t {
            const IS_LEAF: bool = false;
            fn register(_: &mut DefinitionContext) -> Result<TypeDesc, DefinitionError> {
                let primitive = PrimitiveType::$primitive;
                Ok(TypeDesc::new(TypeRef::Primitive(primitive), primitive.rust_path().into()))
            }
        }
    };
}
macro_rules! integer {
    ($($t:ty => $primitive:ident),*) => { $(
        inline!($t, $primitive);
        impl MapKey for $t { fn path() -> Option<&'static str> { Some(stringify!($t)) } }
    )* };
}
integer!(i8 => I8, i16 => I16, i32 => I32, i64 => Integer, i128 => I128, isize => Isize,
    u8 => U8, u16 => U16, u32 => U32, u64 => U64, u128 => U128, usize => Usize);
inline!(String, String);
inline!(bool, Boolean); inline!(f32, F32); inline!(f64, Number); inline!(char, Char);
inline!(Value, Json); inline!((), Unit);
// Foreign leaf implementations belong here: downstream crates cannot implement
// this trait for types they do not own.
impl Registration for std::time::Duration {
    fn register(c: &mut DefinitionContext) -> Result<TypeDesc, DefinitionError> { c.register_as::<Self>(rust_path!(::std::time::Duration)) }
}
impl Registration for std::ops::Range<u64> {
    fn register(c: &mut DefinitionContext) -> Result<TypeDesc, DefinitionError> { c.register_as::<Self>(rust_path!(::std::ops::Range<u64>)) }
}
