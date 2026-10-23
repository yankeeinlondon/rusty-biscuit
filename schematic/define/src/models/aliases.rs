//! Type alias definition.

use super::type_ref::TypeRef;

/// A type alias definition.
///
/// Represents a type alias that gives a new name to an existing type. A
/// `oneOf`/`anyOf` target is generated as an enum with one variant per
/// alternative, selected by [`TypeAlias::discriminator`] when it is set.
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
///     discriminator: None,
/// };
///
/// assert_eq!(user_id_alias.name, "UserId");
/// ```
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TypeAlias {
    /// Name of the alias (PascalCase recommended).
    pub name: String,
    /// Human-readable description.
    #[serde(default, skip_serializing_if = "Option::is_none", deserialize_with = "crate::serde_presence::non_null")]
    pub description: Option<String>,
    /// The target type this alias refers to.
    pub target: TypeRef,
    /// How a `oneOf`/`anyOf` target selects its alternative; `None` for any
    /// other target, and for a union whose alternatives cannot all be told
    /// apart by one property (it decodes as the first alternative that fits).
    #[serde(default, skip_serializing_if = "Option::is_none", deserialize_with = "crate::serde_presence::non_null")]
    pub discriminator: Option<UnionDiscriminator>,
}

/// The property whose string value selects a union's alternative.
///
/// `tags[i]` is the set of values that select alternative `i` of the alias's
/// `oneOf`/`anyOf` target, so it has one entry per alternative, in order. A
/// value may select several alternatives (OpenAI's `Item` gives `message` to
/// both input and output messages); they are then tried in order. A decoded
/// value whose property is missing, not a string, or not listed is rejected
/// rather than given to the first alternative that fits.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UnionDiscriminator {
    /// Wire name of the discriminator property.
    pub property: String,
    /// Accepted values per alternative, aligned with the target's alternatives.
    pub tags: Vec<Vec<String>>,
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
            discriminator: None,
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
            discriminator: None,
        };
        assert!(matches!(alias.target, TypeRef::Array(_)));
    }
}
