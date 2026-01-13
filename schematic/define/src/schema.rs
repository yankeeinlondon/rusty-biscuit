//! Schema definitions for request/response types.
//!
//! This module provides types for describing the structure of API
//! request and response bodies. These schemas are used during code
//! generation to create strongly-typed Rust structs.

use serde::de::DeserializeOwned;
use serde::Serialize;
use std::fmt::Debug;

/// Trait bound for types that can be used in API schemas.
///
/// This trait provides the necessary bounds for serialization,
/// deserialization, and thread-safe usage in async contexts.
///
/// The trait is automatically implemented for any type that satisfies
/// all the required bounds.
///
/// ## Required Bounds
///
/// - `Serialize` - Can be serialized to JSON
/// - `DeserializeOwned` - Can be deserialized from JSON
/// - `Debug` - Supports debug formatting
/// - `Clone` - Can be cloned
/// - `Send + Sync` - Thread-safe for async usage
/// - `'static` - No borrowed references
///
/// ## Examples
///
/// Any struct with the right derives automatically implements `SchemaObject`:
///
/// ```
/// use serde::{Deserialize, Serialize};
/// use schematic_define::SchemaObject;
///
/// #[derive(Debug, Clone, Serialize, Deserialize)]
/// struct MyRequest {
///     name: String,
///     count: u32,
/// }
///
/// // MyRequest automatically implements SchemaObject
/// fn accepts_schema<T: SchemaObject>(_: T) {}
///
/// let req = MyRequest { name: "test".into(), count: 42 };
/// accepts_schema(req);
/// ```
pub trait SchemaObject: Serialize + DeserializeOwned + Debug + Clone + Send + Sync + 'static {}

// Blanket implementation for all qualifying types
impl<T> SchemaObject for T where T: Serialize + DeserializeOwned + Debug + Clone + Send + Sync + 'static
{}

/// A schema descriptor for code generation.
///
/// This struct captures type information needed to generate strongly-typed
/// request and response structs. It stores the type name and optional
/// module path for proper imports in generated code.
///
/// ## Examples
///
/// Create a schema with just a type name:
///
/// ```
/// use schematic_define::Schema;
///
/// let schema = Schema::new("ListModelsResponse");
/// assert_eq!(schema.type_name, "ListModelsResponse");
/// assert_eq!(schema.full_path(), "ListModelsResponse");
/// ```
///
/// Create a schema with a module path:
///
/// ```
/// use schematic_define::Schema;
///
/// let schema = Schema::with_path("Model", "crate::types");
/// assert_eq!(schema.full_path(), "crate::types::Model");
/// ```
#[derive(Debug, Clone)]
pub struct Schema {
    /// The Rust type name (e.g., "ListModelsResponse").
    pub type_name: String,
    /// Module path where this type is defined (e.g., "crate::models").
    pub module_path: Option<String>,
}

impl Schema {
    /// Creates a new schema with just a type name.
    ///
    /// Use this when the type is in the current module or will be
    /// imported separately.
    ///
    /// ## Examples
    ///
    /// ```
    /// use schematic_define::Schema;
    ///
    /// let schema = Schema::new("CreateUserRequest");
    /// assert_eq!(schema.type_name, "CreateUserRequest");
    /// assert!(schema.module_path.is_none());
    /// ```
    pub fn new(type_name: impl Into<String>) -> Self {
        Self {
            type_name: type_name.into(),
            module_path: None,
        }
    }

    /// Creates a schema with a module path.
    ///
    /// Use this when the type needs to be referenced with its full path.
    ///
    /// ## Examples
    ///
    /// ```
    /// use schematic_define::Schema;
    ///
    /// let schema = Schema::with_path("User", "crate::models::user");
    /// assert_eq!(schema.type_name, "User");
    /// assert_eq!(schema.module_path, Some("crate::models::user".to_string()));
    /// ```
    pub fn with_path(type_name: impl Into<String>, module_path: impl Into<String>) -> Self {
        Self {
            type_name: type_name.into(),
            module_path: Some(module_path.into()),
        }
    }

    /// Returns the fully qualified type path.
    ///
    /// If a module path is set, returns `module_path::type_name`.
    /// Otherwise, returns just the type name.
    ///
    /// ## Examples
    ///
    /// ```
    /// use schematic_define::Schema;
    ///
    /// // Without module path
    /// let schema1 = Schema::new("Response");
    /// assert_eq!(schema1.full_path(), "Response");
    ///
    /// // With module path
    /// let schema2 = Schema::with_path("Response", "api::types");
    /// assert_eq!(schema2.full_path(), "api::types::Response");
    /// ```
    pub fn full_path(&self) -> String {
        match &self.module_path {
            Some(path) => format!("{}::{}", path, self.type_name),
            None => self.type_name.clone(),
        }
    }
}
