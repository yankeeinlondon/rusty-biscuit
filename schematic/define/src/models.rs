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

/// A struct (object) definition.
///
/// Represents a structured data type with named fields. Optionally supports
/// additional properties for map-like structures.
///
/// ## Examples
///
/// ```
/// use schematic_define::models::{StructDef, FieldDef, TypeRef, PrimitiveType};
///
/// let user_struct = StructDef {
///     name: "User".to_string(),
///     description: Some("A user account".to_string()),
///     fields: vec![
///         FieldDef {
///             name: "email".to_string(),
///             serde_rename: None,
///             description: None,
///             required: true,
///             field_type: TypeRef::Primitive(PrimitiveType::String),
///         },
///     ],
///     additional_properties: None,
/// };
///
/// assert_eq!(user_struct.name, "User");
/// assert_eq!(user_struct.fields.len(), 1);
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StructDef {
    /// Name of the struct (PascalCase recommended).
    pub name: String,
    /// Human-readable description.
    pub description: Option<String>,
    /// List of fields in this struct.
    pub fields: Vec<FieldDef>,
    /// Type of additional properties (for map-like objects).
    ///
    /// When set, the struct accepts arbitrary additional key-value pairs
    /// beyond the defined fields, where values are of the specified type.
    pub additional_properties: Option<TypeRef>,
}

/// An enum definition.
///
/// Represents an enumeration with named variants. Variants can have optional
/// string values for serde serialization.
///
/// ## Examples
///
/// ```
/// use schematic_define::models::{EnumDef, EnumVariant};
///
/// let status_enum = EnumDef {
///     name: "Status".to_string(),
///     description: Some("Order status".to_string()),
///     variants: vec![
///         EnumVariant {
///             name: "Pending".to_string(),
///             value: Some("pending".to_string()),
///             description: Some("Order is pending".to_string()),
///         },
///         EnumVariant {
///             name: "Shipped".to_string(),
///             value: Some("shipped".to_string()),
///             description: None,
///         },
///     ],
///     untagged: false,
/// };
///
/// assert_eq!(status_enum.variants.len(), 2);
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EnumDef {
    /// Name of the enum (PascalCase recommended).
    pub name: String,
    /// Human-readable description.
    pub description: Option<String>,
    /// List of enum variants.
    pub variants: Vec<EnumVariant>,
    /// Whether this is an untagged enum (serde `#[serde(untagged)]`).
    ///
    /// Untagged enums are deserialized by trying each variant in order
    /// until one succeeds.
    pub untagged: bool,
}

/// A type alias definition.
///
/// Represents a type alias that gives a new name to an existing type.
///
/// ## Examples
///
/// ```
/// use schematic_define::models::{TypeAlias, TypeRef, PrimitiveType};
///
/// let user_id_alias = TypeAlias {
///     name: "UserId".to_string(),
///     description: Some("Unique user identifier".to_string()),
///     target: TypeRef::Primitive(PrimitiveType::String),
/// };
///
/// assert_eq!(user_id_alias.name, "UserId");
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TypeAlias {
    /// Name of the alias (PascalCase recommended).
    pub name: String,
    /// Human-readable description.
    pub description: Option<String>,
    /// The target type this alias refers to.
    pub target: TypeRef,
}

/// An enum variant.
///
/// Represents a single variant in an enum definition.
///
/// ## Examples
///
/// ```
/// use schematic_define::models::EnumVariant;
///
/// // Variant with explicit string value
/// let variant = EnumVariant {
///     name: "InProgress".to_string(),
///     value: Some("in_progress".to_string()),
///     description: Some("Work is ongoing".to_string()),
/// };
///
/// // Variant using name as value
/// let simple_variant = EnumVariant {
///     name: "Active".to_string(),
///     value: None,
///     description: None,
/// };
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EnumVariant {
    /// Variant name (PascalCase recommended).
    pub name: String,
    /// Optional string value for serde serialization.
    ///
    /// If `None`, the variant name is used as-is.
    /// If `Some`, this value is used for `#[serde(rename = "...")]`.
    pub value: Option<String>,
    /// Human-readable description.
    pub description: Option<String>,
}

/// A field in a struct.
///
/// Represents a single field with its name, type, and metadata.
///
/// ## Examples
///
/// ```
/// use schematic_define::models::{FieldDef, TypeRef, PrimitiveType};
///
/// let field = FieldDef {
///     name: "user_name".to_string(),
///     serde_rename: Some("userName".to_string()),
///     description: Some("The user's display name".to_string()),
///     required: true,
///     field_type: TypeRef::Primitive(PrimitiveType::String),
/// };
///
/// assert!(field.required);
/// assert_eq!(field.serde_rename, Some("userName".to_string()));
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FieldDef {
    /// Field name (snake_case for Rust conventions).
    pub name: String,
    /// Optional rename for serde serialization.
    ///
    /// Used when the API field name differs from the Rust field name.
    pub serde_rename: Option<String>,
    /// Human-readable description.
    pub description: Option<String>,
    /// Whether this field is required (non-optional).
    pub required: bool,
    /// The type of this field.
    pub field_type: TypeRef,
}

/// A reference to a type.
///
/// Represents various ways to reference types: primitives, arrays, maps,
/// named types, and schema combinators (oneOf, anyOf, allOf).
///
/// ## Examples
///
/// ```
/// use schematic_define::models::{TypeRef, PrimitiveType};
///
/// // Primitive type
/// let string_ref = TypeRef::Primitive(PrimitiveType::String);
///
/// // Array of strings
/// let string_array = TypeRef::Array(Box::new(TypeRef::Primitive(PrimitiveType::String)));
///
/// // Map with string values
/// let string_map = TypeRef::Map(Box::new(TypeRef::Primitive(PrimitiveType::String)));
///
/// // Named type reference
/// let user_ref = TypeRef::Named("User".to_string());
///
/// // Optional string
/// let optional_string = TypeRef::Optional(Box::new(TypeRef::Primitive(PrimitiveType::String)));
///
/// // OneOf combinator
/// let one_of = TypeRef::OneOf(vec![
///     TypeRef::Named("Cat".to_string()),
///     TypeRef::Named("Dog".to_string()),
/// ]);
/// ```
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TypeRef {
    /// A primitive type.
    Primitive(PrimitiveType),
    /// An array of items of the inner type.
    Array(Box<TypeRef>),
    /// A map (object) with string keys and values of the inner type.
    Map(Box<TypeRef>),
    /// A reference to a named type (struct, enum, or alias).
    Named(String),
    /// OpenAPI `oneOf` - exactly one of the listed types.
    OneOf(Vec<TypeRef>),
    /// OpenAPI `anyOf` - one or more of the listed types.
    AnyOf(Vec<TypeRef>),
    /// OpenAPI `allOf` - combines all listed types (intersection).
    AllOf(Vec<TypeRef>),
    /// An optional wrapper around another type.
    Optional(Box<TypeRef>),
    /// Unknown or unsupported type.
    Unknown,
}

/// Primitive types.
///
/// Represents the basic building-block types that map to Rust primitives.
///
/// ## Examples
///
/// ```
/// use schematic_define::models::PrimitiveType;
///
/// let primitives = vec![
///     PrimitiveType::String,
///     PrimitiveType::Integer,
///     PrimitiveType::Number,
///     PrimitiveType::Boolean,
///     PrimitiveType::Bytes,
///     PrimitiveType::Json,
/// ];
///
/// assert_eq!(primitives.len(), 6);
/// ```
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PrimitiveType {
    /// UTF-8 string.
    String,
    /// 64-bit signed integer.
    Integer,
    /// 64-bit floating-point number.
    Number,
    /// Boolean true/false.
    Boolean,
    /// Raw bytes (binary data).
    Bytes,
    /// Arbitrary JSON value (serde_json::Value).
    Json,
}

#[cfg(test)]
mod tests {
    use super::*;

    // ========== PrimitiveType Tests ==========

    #[test]
    fn primitive_type_debug_clone_eq() {
        let prim = PrimitiveType::String;
        // Explicitly exercise the Clone impl on a Copy type.
        #[allow(clippy::clone_on_copy)]
        let cloned = prim.clone();
        assert_eq!(prim, cloned);
        assert_eq!(format!("{:?}", prim), "String");
    }

    #[test]
    fn primitive_type_copy() {
        let original = PrimitiveType::Integer;
        let copied = original; // Copy, not move
        assert_eq!(original, copied);
    }

    #[test]
    fn primitive_type_hash() {
        use std::collections::HashSet;
        let mut set = HashSet::new();
        set.insert(PrimitiveType::String);
        set.insert(PrimitiveType::Integer);
        assert_eq!(set.len(), 2);
        assert!(set.contains(&PrimitiveType::String));
    }

    #[test]
    fn primitive_type_all_variants() {
        let variants = [
            PrimitiveType::String,
            PrimitiveType::Integer,
            PrimitiveType::Number,
            PrimitiveType::Boolean,
            PrimitiveType::Bytes,
            PrimitiveType::Json,
        ];
        assert_eq!(variants.len(), 6);
    }

    // ========== TypeRef Tests ==========

    #[test]
    fn type_ref_debug_clone_eq() {
        let type_ref = TypeRef::Primitive(PrimitiveType::String);
        let cloned = type_ref.clone();
        assert_eq!(type_ref, cloned);
        assert!(format!("{:?}", type_ref).contains("Primitive"));
    }

    #[test]
    fn type_ref_primitive() {
        let type_ref = TypeRef::Primitive(PrimitiveType::Boolean);
        assert!(matches!(
            type_ref,
            TypeRef::Primitive(PrimitiveType::Boolean)
        ));
    }

    #[test]
    fn type_ref_array() {
        let inner = TypeRef::Primitive(PrimitiveType::String);
        let array = TypeRef::Array(Box::new(inner.clone()));
        if let TypeRef::Array(boxed) = array {
            assert_eq!(*boxed, inner);
        } else {
            panic!("Expected Array variant");
        }
    }

    #[test]
    fn type_ref_map() {
        let inner = TypeRef::Primitive(PrimitiveType::Integer);
        let map = TypeRef::Map(Box::new(inner.clone()));
        if let TypeRef::Map(boxed) = map {
            assert_eq!(*boxed, inner);
        } else {
            panic!("Expected Map variant");
        }
    }

    #[test]
    fn type_ref_named() {
        let named = TypeRef::Named("User".to_string());
        if let TypeRef::Named(name) = named {
            assert_eq!(name, "User");
        } else {
            panic!("Expected Named variant");
        }
    }

    #[test]
    fn type_ref_one_of() {
        let one_of = TypeRef::OneOf(vec![
            TypeRef::Named("Cat".to_string()),
            TypeRef::Named("Dog".to_string()),
        ]);
        if let TypeRef::OneOf(types) = one_of {
            assert_eq!(types.len(), 2);
        } else {
            panic!("Expected OneOf variant");
        }
    }

    #[test]
    fn type_ref_any_of() {
        let any_of = TypeRef::AnyOf(vec![
            TypeRef::Primitive(PrimitiveType::String),
            TypeRef::Primitive(PrimitiveType::Integer),
        ]);
        if let TypeRef::AnyOf(types) = any_of {
            assert_eq!(types.len(), 2);
        } else {
            panic!("Expected AnyOf variant");
        }
    }

    #[test]
    fn type_ref_all_of() {
        let all_of = TypeRef::AllOf(vec![TypeRef::Named("Base".to_string())]);
        if let TypeRef::AllOf(types) = all_of {
            assert_eq!(types.len(), 1);
        } else {
            panic!("Expected AllOf variant");
        }
    }

    #[test]
    fn type_ref_optional() {
        let inner = TypeRef::Primitive(PrimitiveType::String);
        let optional = TypeRef::Optional(Box::new(inner.clone()));
        if let TypeRef::Optional(boxed) = optional {
            assert_eq!(*boxed, inner);
        } else {
            panic!("Expected Optional variant");
        }
    }

    #[test]
    fn type_ref_unknown() {
        let unknown = TypeRef::Unknown;
        assert!(matches!(unknown, TypeRef::Unknown));
    }

    #[test]
    fn type_ref_nested() {
        // Array of optional named types
        let nested = TypeRef::Array(Box::new(TypeRef::Optional(Box::new(TypeRef::Named(
            "User".to_string(),
        )))));
        assert!(matches!(nested, TypeRef::Array(_)));
    }

    // ========== EnumVariant Tests ==========

    #[test]
    fn enum_variant_debug_clone_eq() {
        let variant = EnumVariant {
            name: "Active".to_string(),
            value: Some("active".to_string()),
            description: Some("The item is active".to_string()),
        };
        let cloned = variant.clone();
        assert_eq!(variant, cloned);
        assert!(format!("{:?}", variant).contains("Active"));
    }

    #[test]
    fn enum_variant_minimal() {
        let variant = EnumVariant {
            name: "Pending".to_string(),
            value: None,
            description: None,
        };
        assert_eq!(variant.name, "Pending");
        assert!(variant.value.is_none());
    }

    // ========== FieldDef Tests ==========

    #[test]
    fn field_def_debug_clone_eq() {
        let field = FieldDef {
            name: "user_id".to_string(),
            serde_rename: Some("userId".to_string()),
            description: Some("The user identifier".to_string()),
            required: true,
            field_type: TypeRef::Primitive(PrimitiveType::String),
        };
        let cloned = field.clone();
        assert_eq!(field, cloned);
        assert!(format!("{:?}", field).contains("user_id"));
    }

    #[test]
    fn field_def_optional_field() {
        let field = FieldDef {
            name: "nickname".to_string(),
            serde_rename: None,
            description: None,
            required: false,
            field_type: TypeRef::Primitive(PrimitiveType::String),
        };
        assert!(!field.required);
    }

    // ========== StructDef Tests ==========

    #[test]
    fn struct_def_debug_clone_eq() {
        let struct_def = StructDef {
            name: "User".to_string(),
            description: Some("A user".to_string()),
            fields: vec![],
            additional_properties: None,
        };
        let cloned = struct_def.clone();
        assert_eq!(struct_def, cloned);
        assert!(format!("{:?}", struct_def).contains("User"));
    }

    #[test]
    fn struct_def_with_fields() {
        let struct_def = StructDef {
            name: "User".to_string(),
            description: None,
            fields: vec![
                FieldDef {
                    name: "id".to_string(),
                    serde_rename: None,
                    description: None,
                    required: true,
                    field_type: TypeRef::Primitive(PrimitiveType::Integer),
                },
                FieldDef {
                    name: "name".to_string(),
                    serde_rename: None,
                    description: None,
                    required: true,
                    field_type: TypeRef::Primitive(PrimitiveType::String),
                },
            ],
            additional_properties: None,
        };
        assert_eq!(struct_def.fields.len(), 2);
    }

    #[test]
    fn struct_def_with_additional_properties() {
        let struct_def = StructDef {
            name: "Metadata".to_string(),
            description: None,
            fields: vec![],
            additional_properties: Some(TypeRef::Primitive(PrimitiveType::Json)),
        };
        assert!(struct_def.additional_properties.is_some());
    }

    // ========== EnumDef Tests ==========

    #[test]
    fn enum_def_debug_clone_eq() {
        let enum_def = EnumDef {
            name: "Status".to_string(),
            description: Some("Order status".to_string()),
            variants: vec![],
            untagged: false,
        };
        let cloned = enum_def.clone();
        assert_eq!(enum_def, cloned);
        assert!(format!("{:?}", enum_def).contains("Status"));
    }

    #[test]
    fn enum_def_with_string_variants() {
        let enum_def = EnumDef {
            name: "Color".to_string(),
            description: None,
            variants: vec![
                EnumVariant {
                    name: "Red".to_string(),
                    value: Some("red".to_string()),
                    description: None,
                },
                EnumVariant {
                    name: "Green".to_string(),
                    value: Some("green".to_string()),
                    description: None,
                },
                EnumVariant {
                    name: "Blue".to_string(),
                    value: Some("blue".to_string()),
                    description: None,
                },
            ],
            untagged: false,
        };
        assert_eq!(enum_def.variants.len(), 3);
        assert!(!enum_def.untagged);
    }

    #[test]
    fn enum_def_untagged() {
        let enum_def = EnumDef {
            name: "Animal".to_string(),
            description: None,
            variants: vec![],
            untagged: true,
        };
        assert!(enum_def.untagged);
    }

    // ========== TypeAlias Tests ==========

    #[test]
    fn type_alias_debug_clone_eq() {
        let alias = TypeAlias {
            name: "UserId".to_string(),
            description: Some("Unique user ID".to_string()),
            target: TypeRef::Primitive(PrimitiveType::String),
        };
        let cloned = alias.clone();
        assert_eq!(alias, cloned);
        assert!(format!("{:?}", alias).contains("UserId"));
    }

    #[test]
    fn type_alias_to_array() {
        let alias = TypeAlias {
            name: "Tags".to_string(),
            description: None,
            target: TypeRef::Array(Box::new(TypeRef::Primitive(PrimitiveType::String))),
        };
        assert!(matches!(alias.target, TypeRef::Array(_)));
    }

    // ========== ModelDef Tests ==========

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

    // ========== ModelCatalog Tests ==========

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

    // ========== Complex Nested Type Tests ==========

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
        // Represents: Map<String, Array<Optional<User>>>
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

        // Verify nesting structure
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
