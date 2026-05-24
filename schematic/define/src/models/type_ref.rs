//! Type reference and primitive type definitions.

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn primitive_type_debug_clone_eq() {
        let prim = PrimitiveType::String;
        #[allow(clippy::clone_on_copy)]
        let cloned = prim.clone();
        assert_eq!(prim, cloned);
        assert_eq!(format!("{:?}", prim), "String");
    }

    #[test]
    fn primitive_type_copy() {
        let original = PrimitiveType::Integer;
        let copied = original;
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
        let nested = TypeRef::Array(Box::new(TypeRef::Optional(Box::new(TypeRef::Named(
            "User".to_string(),
        )))));
        assert!(matches!(nested, TypeRef::Array(_)));
    }
}
