//! Struct and field definitions.

use super::type_ref::TypeRef;

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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::PrimitiveType;

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
}
