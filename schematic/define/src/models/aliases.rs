//! Type alias definition.

use super::type_ref::TypeRef;

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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::PrimitiveType;

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
}
