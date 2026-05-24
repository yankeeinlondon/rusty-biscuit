//! Enum and enum variant definitions.

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

#[cfg(test)]
mod tests {
    use super::*;

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
}
