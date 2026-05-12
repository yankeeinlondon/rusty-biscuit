//! Model definitions for imported API schemas.
//!
//! This module provides types for representing models (structs, enums, type aliases)
//! that are imported from external API specifications like OpenAPI. These types are
//! general-purpose and not gated behind any feature flag.
//!
//! ## Core Types
//!
//! - [`ModelCatalog`] - Collection of model definitions with optional module path
//! - [`ModelDef`] - Union of model types: struct, enum, or type alias
//! - [`StructDef`] - Structure definition with fields
//! - [`EnumDef`] - Enumeration definition with variants
//! - [`TypeAlias`] - Type alias definition
//! - [`FieldDef`] - Field definition for structs
//! - [`EnumVariant`] - Variant definition for enums
//! - [`TypeRef`] - Type reference (primitives, arrays, named types, combinators)
//! - [`PrimitiveType`] - Basic primitive types
//!
//! ## Examples
//!
//! Define a simple struct model:
//!
//! ```
//! use schematic_define::models::{ModelCatalog, ModelDef, StructDef, FieldDef, TypeRef, PrimitiveType};
//!
//! let catalog = ModelCatalog {
//!     module_path: Some("my_api".to_string()),
//!     types: vec![
//!         ModelDef::Struct(StructDef {
//!             name: "User".to_string(),
//!             description: Some("A user in the system".to_string()),
//!             fields: vec![
//!                 FieldDef {
//!                     name: "id".to_string(),
//!                     serde_rename: None,
//!                     description: Some("Unique identifier".to_string()),
//!                     required: true,
//!                     field_type: TypeRef::Primitive(PrimitiveType::Integer),
//!                 },
//!                 FieldDef {
//!                     name: "name".to_string(),
//!                     serde_rename: None,
//!                     description: None,
//!                     required: true,
//!                     field_type: TypeRef::Primitive(PrimitiveType::String),
//!                 },
//!             ],
//!             additional_properties: None,
//!         }),
//!     ],
//! };
//!
//! assert_eq!(catalog.types.len(), 1);
//! ```

mod aliases;
mod enums;
mod structs;
mod type_ref;

pub use aliases::TypeAlias;
pub use enums::{EnumDef, EnumVariant};
pub use structs::{FieldDef, StructDef};
pub use type_ref::{PrimitiveType, TypeRef};

/// A catalog of model definitions for an API.
///
/// This represents a collection of types (structs, enums, aliases) that can be
/// generated as Rust code. The optional `module_path` specifies where the
/// generated types should be placed.
///
/// ## Examples
///
/// ```
/// use schematic_define::models::{ModelCatalog, ModelDef, StructDef};
///
/// let catalog = ModelCatalog {
///     module_path: Some("my_api::types".to_string()),
///     types: vec![],
/// };
///
/// assert!(catalog.types.is_empty());
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModelCatalog {
    /// Optional module path for generated types (e.g., "my_api::types").
    pub module_path: Option<String>,
    /// Collection of model definitions.
    pub types: Vec<ModelDef>,
}

/// A model definition that can be a struct, enum, or type alias.
///
/// This enum represents the three fundamental kinds of type definitions
/// that can be imported from an API specification.
///
/// ## Examples
///
/// ```
/// use schematic_define::models::{ModelDef, StructDef, EnumDef, TypeAlias, TypeRef, PrimitiveType};
///
/// // A struct model
/// let struct_model = ModelDef::Struct(StructDef {
///     name: "User".to_string(),
///     description: None,
///     fields: vec![],
///     additional_properties: None,
/// });
///
/// // An enum model
/// let enum_model = ModelDef::Enum(EnumDef {
///     name: "Status".to_string(),
///     description: None,
///     variants: vec![],
///     untagged: false,
/// });
///
/// // A type alias
/// let alias_model = ModelDef::Alias(TypeAlias {
///     name: "UserId".to_string(),
///     description: None,
///     target: TypeRef::Primitive(PrimitiveType::String),
/// });
/// ```
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ModelDef {
    /// A struct (object) definition.
    Struct(StructDef),
    /// An enum definition.
    Enum(EnumDef),
    /// A type alias.
    Alias(TypeAlias),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn model_def_debug_clone_eq() {
        let model = ModelDef::Struct(StructDef {
            name: "Test".to_string(),
            description: None,
            fields: vec![],
            additional_properties: None,
        });
        let cloned = model.clone();
        assert_eq!(model, cloned);
        assert!(format!("{:?}", model).contains("Struct"));
    }

    #[test]
    fn model_def_struct_variant() {
        let model = ModelDef::Struct(StructDef {
            name: "User".to_string(),
            description: None,
            fields: vec![],
            additional_properties: None,
        });
        assert!(matches!(model, ModelDef::Struct(_)));
    }

    #[test]
    fn model_def_enum_variant() {
        let model = ModelDef::Enum(EnumDef {
            name: "Status".to_string(),
            description: None,
            variants: vec![],
            untagged: false,
        });
        assert!(matches!(model, ModelDef::Enum(_)));
    }

    #[test]
    fn model_def_alias_variant() {
        let model = ModelDef::Alias(TypeAlias {
            name: "Id".to_string(),
            description: None,
            target: TypeRef::Primitive(PrimitiveType::String),
        });
        assert!(matches!(model, ModelDef::Alias(_)));
    }

    #[test]
    fn model_catalog_debug_clone_eq() {
        let catalog = ModelCatalog {
            module_path: Some("my_api".to_string()),
            types: vec![],
        };
        let cloned = catalog.clone();
        assert_eq!(catalog, cloned);
        assert!(format!("{:?}", catalog).contains("my_api"));
    }

    #[test]
    fn model_catalog_empty() {
        let catalog = ModelCatalog {
            module_path: None,
            types: vec![],
        };
        assert!(catalog.types.is_empty());
        assert!(catalog.module_path.is_none());
    }

    #[test]
    fn model_catalog_with_types() {
        let catalog = ModelCatalog {
            module_path: Some("api".to_string()),
            types: vec![
                ModelDef::Struct(StructDef {
                    name: "User".to_string(),
                    description: None,
                    fields: vec![],
                    additional_properties: None,
                }),
                ModelDef::Enum(EnumDef {
                    name: "Status".to_string(),
                    description: None,
                    variants: vec![],
                    untagged: false,
                }),
                ModelDef::Alias(TypeAlias {
                    name: "UserId".to_string(),
                    description: None,
                    target: TypeRef::Primitive(PrimitiveType::String),
                }),
            ],
        };
        assert_eq!(catalog.types.len(), 3);
    }

    #[test]
    fn model_catalog_represents_simple_struct() {
        let catalog = ModelCatalog {
            module_path: None,
            types: vec![ModelDef::Struct(StructDef {
                name: "Point".to_string(),
                description: Some("A 2D point".to_string()),
                fields: vec![
                    FieldDef {
                        name: "x".to_string(),
                        serde_rename: None,
                        description: None,
                        required: true,
                        field_type: TypeRef::Primitive(PrimitiveType::Number),
                    },
                    FieldDef {
                        name: "y".to_string(),
                        serde_rename: None,
                        description: None,
                        required: true,
                        field_type: TypeRef::Primitive(PrimitiveType::Number),
                    },
                ],
                additional_properties: None,
            })],
        };

        let ModelDef::Struct(point) = &catalog.types[0] else {
            panic!("Expected Struct");
        };
        assert_eq!(point.name, "Point");
        assert_eq!(point.fields.len(), 2);
    }

    #[test]
    fn model_catalog_represents_enum_with_string_variants() {
        let catalog = ModelCatalog {
            module_path: None,
            types: vec![ModelDef::Enum(EnumDef {
                name: "Priority".to_string(),
                description: None,
                variants: vec![
                    EnumVariant {
                        name: "Low".to_string(),
                        value: Some("low".to_string()),
                        description: None,
                    },
                    EnumVariant {
                        name: "Medium".to_string(),
                        value: Some("medium".to_string()),
                        description: None,
                    },
                    EnumVariant {
                        name: "High".to_string(),
                        value: Some("high".to_string()),
                        description: None,
                    },
                ],
                untagged: false,
            })],
        };

        let ModelDef::Enum(priority) = &catalog.types[0] else {
            panic!("Expected Enum");
        };
        assert_eq!(priority.name, "Priority");
        assert_eq!(priority.variants.len(), 3);
        assert_eq!(priority.variants[0].value, Some("low".to_string()));
    }

    #[test]
    fn model_catalog_represents_nested_type_refs() {
        let nested_type = TypeRef::Map(Box::new(TypeRef::Array(Box::new(TypeRef::Optional(
            Box::new(TypeRef::Named("User".to_string())),
        )))));

        let catalog = ModelCatalog {
            module_path: None,
            types: vec![ModelDef::Alias(TypeAlias {
                name: "UserGroups".to_string(),
                description: Some("Groups of optional users".to_string()),
                target: nested_type.clone(),
            })],
        };

        let ModelDef::Alias(alias) = &catalog.types[0] else {
            panic!("Expected Alias");
        };
        assert_eq!(alias.name, "UserGroups");

        let TypeRef::Map(map_inner) = &alias.target else {
            panic!("Expected Map");
        };
        let TypeRef::Array(array_inner) = map_inner.as_ref() else {
            panic!("Expected Array");
        };
        let TypeRef::Optional(opt_inner) = array_inner.as_ref() else {
            panic!("Expected Optional");
        };
        let TypeRef::Named(name) = opt_inner.as_ref() else {
            panic!("Expected Named");
        };
        assert_eq!(name, "User");
    }
}
