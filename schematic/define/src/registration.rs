//! Typed registrations for one generated module group.
//!
//! A [`TypeDesc`] is an authoring handle: builders lower it to a
//! [`TypeRef`] naming components, and API build stores the context's
//! components on the API (see [`crate::SchemaComponent`]).
//! The first registration of a canonical Rust type fixes its name and path,
//! and, for a composition ([`DefinitionContext::one_of`] and its siblings),
//! the structure its component exports.
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
use crate::{ModelCatalog, ModelDef, PrimitiveType, SchemaComponent, TypeAlias, TypeRef};

/// A registered type: the reference builders store, and its Rust spelling.
///
/// Fields are private: registration is the only constructor. Equality
/// includes the structure, so a descriptor equals only what its own context
/// returns for that component.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TypeDesc {
    reference: TypeRef,
    rust_path: String,
    structure: Option<TypeRef>,
}
impl TypeDesc {
    fn new(reference: TypeRef, rust_path: String) -> Self { Self { reference, rust_path, structure: None } }
    /// The `oneOf`/`anyOf`/`allOf` a composed component exports in place of its
    /// Rust type's own schema. `Some` only on the component's own descriptor;
    /// a container of it, such as `Vec<T>`, is not itself a composition.
    pub fn structure(&self) -> Option<&TypeRef> { self.structure.as_ref() }
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
    /// Invalid GraphQL operation facts, including an unresolved variables,
    /// data, or error reference, with assembly context.
    #[error("{api} operation {operation}: {message}")]
    InvalidOperation { api: String, operation: String, message: String },
    /// Containers, primitives, and free-form JSON do not define components.
    #[error("register_as requires a nominal leaf type")]
    NotLeaf,
    /// JSON object keys need a supported Serde string representation.
    #[error("{api} endpoint {endpoint}: unsupported map key {rust_type}")]
    UnsupportedKey { api: String, endpoint: String, rust_type: &'static str },
    /// A composition must name at least one member.
    #[error("composition for {target} has no members")]
    EmptyComposition { target: &'static str },
    /// Members are component references; containers and primitives have no
    /// component to reference.
    #[error("composition for {target}: member {member} is not a nominal component")]
    NonNominalMember { target: &'static str, member: String },
    /// A member must be the descriptor this context holds for its component
    /// name. Descriptors carry no context identity, so a foreign descriptor is
    /// detected only when its name, Rust path, or structure differs from this
    /// context's entry, or the name is absent here.
    #[error("composition for {target}: member \"{name}\" ({rust_path}) is unregistered or from a foreign context")]
    ForeignMember { target: &'static str, name: String, rust_path: String },
    /// A member may appear once.
    #[error("composition for {target}: member \"{name}\" is listed more than once")]
    DuplicateMember { target: &'static str, name: String },
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
    structure: Option<TypeRef>,
}
impl TypeRegistration {
    /// Component name, independent of the Rust path.
    pub fn schema_name(&self) -> &str { &self.name }
    /// Fully qualified Rust leaf path.
    pub fn rust_path(&self) -> &str { &self.path }
    /// Schema generated with the context's OpenAPI-compatible settings. For a
    /// composition this is the Rust type's own schema; the exported component
    /// is [`Self::structure`] instead.
    pub fn schema(&self) -> &Schema { &self.schema }
    /// The composition fixed by the first registration, if any.
    pub fn structure(&self) -> Option<&TypeRef> { self.structure.as_ref() }
    fn descriptor(&self) -> TypeDesc {
        TypeDesc { reference: TypeRef::Named(self.name.clone()), rust_path: self.path.clone(), structure: self.structure.clone() }
    }
}

/// Owns canonical registrations and collision diagnostics for one module group.
#[derive(Debug)]
pub struct DefinitionContext {
    entries: Vec<TypeRegistration>,
    errors: Vec<DefinitionError>,
    module_path: String,
    models: ModelCatalog,
}
impl DefinitionContext {
    /// State the generated types module once; it must be publicly accessible
    /// from the generated crate. Generated-client compilation proves visibility.
    pub fn types_in(module_path: impl Into<String>) -> Self {
        Self { entries: Vec::new(), errors: Vec::new(), module_path: module_path.into(), models: ModelCatalog { module_path: None, types: Vec::new() } }
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
            return Ok(entry.descriptor());
        }
        let name = name.map(str::to_owned).unwrap_or_else(|| T::schema_name().into_owned());
        if let Some(entry) = self.entries.iter().find(|e| e.name == name) {
            let error = DefinitionError::Collision { schema_name: name, first: entry.diagnostic, second: type_name::<T>() };
            return Err(self.record(error));
        }
        self.entries.push(TypeRegistration { identity: TypeId::of::<T>(), diagnostic: type_name::<T>(), name: name.clone(), path: path.into(), schema: SchemaSettings::openapi3().into_generator().into_root_schema_for::<T>(), structure: None });
        Ok(TypeDesc::new(TypeRef::Named(name), path.into()))
    }
    fn record(&mut self, error: DefinitionError) -> DefinitionError {
        if !self.errors.contains(&error) { self.errors.push(error.clone()); }
        error
    }
    /// Bind the hand-written nominal type `T` to `oneOf` its `variants`, which
    /// this context registered. `T`'s component exports the composition
    /// instead of its own schema, and the descriptor still names `T`, so
    /// generated code uses `T`. The author asserts `T`'s Serde representation
    /// matches; an untagged enum does not prove `oneOf` exclusivity.
    ///
    /// The first registration of `T` fixes its entry: repeating the same
    /// composition returns the same descriptor, and a later `register::<T>()`
    /// returns the composed one.
    ///
    /// ## Examples
    ///
    /// ```
    /// use schematic_define::{DefinitionContext, TypeRef, definition_type};
    /// definition_type!(#[derive(schemars::JsonSchema)] pub struct Cat { pub meows: bool });
    /// definition_type!(#[derive(schemars::JsonSchema)] pub struct Dog { pub barks: u8 });
    /// definition_type!(#[derive(schemars::JsonSchema)] #[serde(untagged)] pub enum Pet { Cat(Cat), Dog(Dog) });
    ///
    /// let mut ctx = DefinitionContext::types_in("::models");
    /// let (cat, dog) = (ctx.register::<Cat>()?, ctx.register::<Dog>()?);
    /// let pet = ctx.one_of::<Pet>([cat, dog])?;
    /// assert_eq!(pet.rust_path(), "::models::Pet");
    /// assert!(matches!(pet.structure(), Some(TypeRef::OneOf(_))));
    /// # Ok::<(), schematic_define::DefinitionError>(())
    /// ```
    ///
    /// ## Errors
    ///
    /// [`DefinitionError::NotLeaf`] when `T` is a container or primitive.
    /// Recorded for [`Self::finish`] as well as returned:
    /// [`DefinitionError::EmptyComposition`],
    /// [`DefinitionError::NonNominalMember`],
    /// [`DefinitionError::ForeignMember`],
    /// [`DefinitionError::DuplicateMember`], a
    /// [`DefinitionError::ComponentConflict`] when `T` is already registered
    /// without this structure, and `T`'s own registration collisions.
    pub fn one_of<T: Registration>(&mut self, variants: impl IntoIterator<Item = TypeDesc>) -> Result<TypeDesc, DefinitionError> {
        self.compose::<T>(TypeRef::OneOf, variants)
    }
    /// [`Self::one_of`] exporting `anyOf`.
    ///
    /// ## Errors
    ///
    /// As for [`Self::one_of`].
    pub fn any_of<T: Registration>(&mut self, variants: impl IntoIterator<Item = TypeDesc>) -> Result<TypeDesc, DefinitionError> {
        self.compose::<T>(TypeRef::AnyOf, variants)
    }
    /// [`Self::one_of`] exporting `allOf`: `T` satisfies every member at once.
    ///
    /// ## Errors
    ///
    /// As for [`Self::one_of`].
    pub fn all_of<T: Registration>(&mut self, members: impl IntoIterator<Item = TypeDesc>) -> Result<TypeDesc, DefinitionError> {
        self.compose::<T>(TypeRef::AllOf, members)
    }
    fn compose<T: Registration>(&mut self, kind: fn(Vec<TypeRef>) -> TypeRef, members: impl IntoIterator<Item = TypeDesc>) -> Result<TypeDesc, DefinitionError> {
        if !T::IS_LEAF { return Err(DefinitionError::NotLeaf); }
        let target = type_name::<T>();
        let members: Vec<TypeDesc> = members.into_iter().collect();
        if members.is_empty() { return Err(self.record(DefinitionError::EmptyComposition { target })); }
        for (index, member) in members.iter().enumerate() {
            let TypeRef::Named(name) = &member.reference else {
                return Err(self.record(DefinitionError::NonNominalMember { target, member: member.rust_path.clone() }));
            };
            if !self.entries.iter().any(|e| &e.name == name && e.descriptor() == *member) {
                return Err(self.record(DefinitionError::ForeignMember { target, name: name.clone(), rust_path: member.rust_path.clone() }));
            }
            if members[..index].iter().any(|m| m.reference == member.reference) {
                return Err(self.record(DefinitionError::DuplicateMember { target, name: name.clone() }));
            }
        }
        let references = members.into_iter().map(TypeDesc::into_type_ref).collect();
        let structure = kind(references);
        if let Some(entry) = self.entries.iter().find(|e| e.identity == TypeId::of::<T>()) {
            if entry.structure.as_ref() == Some(&structure) { return Ok(entry.descriptor()); }
            let first = match &entry.structure { Some(existing) => format!("{target} as {}", existing.wire_schema()), None => format!("{target} without a composition") };
            let error = DefinitionError::ComponentConflict { schema_name: entry.name.clone(), first, second: format!("{target} as {}", structure.wire_schema()) };
            return Err(self.record(error));
        }
        T::register(self)?;
        let entry = self.entries.iter_mut().find(|e| e.identity == TypeId::of::<T>()).ok_or(DefinitionError::NotLeaf)?;
        entry.structure = Some(structure.clone());
        self.models.types.push(ModelDef::Alias(TypeAlias { name: entry.name.clone(), description: None, target: structure }));
        Ok(entry.descriptor())
    }
    /// Fail assembly with every distinct collision seen so far.
    pub fn finish(&self) -> Result<(), DefinitionError> {
        if self.errors.is_empty() { Ok(()) } else { Err(DefinitionError::Collisions(self.errors.clone())) }
    }
    /// Ordered nominal entries, including explicit types unused by endpoints.
    pub fn registrations(&self) -> &[TypeRegistration] { &self.entries }
    /// Each composition as an alias from its component name to its structure,
    /// in registration order; empty when nothing is composed.
    pub fn models(&self) -> &ModelCatalog { &self.models }
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
        crate::components::flatten(self.entries.iter().map(|e| (e.name.as_str(), e.path.as_str(), e.diagnostic, e.schema.as_value(), e.structure.as_ref().map(TypeRef::wire_schema))))
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
