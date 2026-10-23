//! Parameter definition types.

/// Definition of a single parameter.
///
/// Represents a parameter that can be passed to an endpoint, including
/// its type, serialization style, and whether it's required.
///
/// ## Examples
///
/// ```
/// use schematic_define::params::{ParamDef, QueryParamType, ParamStyle};
///
/// let param = ParamDef { rust_name: None, secret: false, pager_managed: false, deprecated: false,
///     name: "tags".to_string(),
///     required: false,
///     description: Some("Filter by tags".to_string()),
///     param_type: QueryParamType::Array(Box::new(QueryParamType::String)),
///     explode: true,
///     style: ParamStyle::Form,
/// };
///
/// assert_eq!(param.name, "tags");
/// assert!(!param.required);
/// ```
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct ParamDef {
    /// Parameter name as used in the API.
    pub name: String,
    /// Optional Rust identifier; the wire name remains `name`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rust_name: Option<String>,
    /// Adds the wire name to the runtime redaction metadata.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub secret: bool,
    /// Requires adoption by a resolved pagination strategy at this location.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub pager_managed: bool,
    /// The provider marks this parameter deprecated. Documentation only: no
    /// Rust `#[deprecated]` attribute is generated.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub deprecated: bool,
    /// Whether this parameter is required.
    pub required: bool,
    /// Human-readable description.
    pub description: Option<String>,
    /// Type of the parameter value.
    pub param_type: QueryParamType,
    /// Whether array/object values should be exploded into separate parameters.
    ///
    /// When `true`, arrays are serialized as `tags=a&tags=b`.
    /// When `false`, arrays are serialized as `tags=a,b` (depending on style).
    pub explode: bool,
    /// Serialization style for the parameter.
    pub style: ParamStyle,
}

/// Type of a parameter value.
///
/// Represents the possible types for query, header, cookie, and typed path
/// parameters of REST endpoints and WebSocket handshakes alike.
///
/// ## Examples
///
/// ```
/// use schematic_define::params::QueryParamType;
///
/// // Simple string parameter
/// let string_type = QueryParamType::String;
///
/// // Array of strings (e.g., for tags)
/// let array_type = QueryParamType::Array(Box::new(QueryParamType::String));
///
/// // Enum with allowed values
/// let enum_type = QueryParamType::Enum(vec![
///     "asc".to_string(),
///     "desc".to_string(),
/// ]);
/// ```
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum QueryParamType {
    /// UTF-8 string parameter.
    String,
    /// Integer parameter.
    Integer,
    /// Floating-point number parameter.
    Number,
    /// Boolean parameter.
    Boolean,
    /// Array of parameters of the inner type.
    Array(Box<QueryParamType>),
    /// Enum with a fixed set of allowed string values.
    Enum(Vec<String>),
    /// Arbitrary JSON value.
    Json,
}

/// Serialization style for parameter values.
///
/// Defines how parameter values (especially arrays and objects) are serialized
/// in the request. Based on OpenAPI 3.x parameter styles.
///
/// ## Examples
///
/// ```
/// use schematic_define::params::ParamStyle;
///
/// // Form style (default for query): tags=a,b or tags=a&tags=b
/// let form = ParamStyle::Form;
///
/// // Pipe-delimited: tags=a|b
/// let pipe = ParamStyle::PipeDelimited;
/// ```
///
/// ## Serialization Examples
///
/// Given an array `["a", "b", "c"]`:
///
/// | Style | explode=false | explode=true |
/// |-------|---------------|--------------|
/// | Form | `tags=a,b,c` | `tags=a&tags=b&tags=c` |
/// | SpaceDelimited | `tags=a%20b%20c` | `tags=a&tags=b&tags=c` |
/// | PipeDelimited | `tags=a\|b\|c` | `tags=a&tags=b&tags=c` |
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum ParamStyle {
    /// Form style (default for query parameters).
    ///
    /// - Without explode: `tags=a,b,c`
    /// - With explode: `tags=a&tags=b&tags=c`
    Form,
    /// Simple style (default for path and header parameters).
    ///
    /// Values are comma-separated: `a,b,c`
    Simple,
    /// Space-delimited style.
    ///
    /// Values are separated by spaces: `a%20b%20c`
    SpaceDelimited,
    /// Pipe-delimited style.
    ///
    /// Values are separated by pipes: `a|b|c`
    PipeDelimited,
    /// Deep object style (for nested objects in query).
    ///
    /// Objects are serialized as: `filter[name]=value&filter[age]=30`
    DeepObject,
}


#[allow(missing_docs)]
impl ParamDef {
    pub fn new(name: impl Into<String>, param_type: QueryParamType) -> Self {
        Self { name: name.into(), param_type, required: false, description: None,
            explode: false, style: ParamStyle::Form, rust_name: None, secret: false, pager_managed: false, deprecated: false }
    }
    /// A typed path parameter: required, `Simple` style, not exploded, as
    /// [`crate::EndpointBuilder::path_param`] also enforces.
    pub fn path(name: impl Into<String>, param_type: QueryParamType) -> Self {
        Self { required: true, style: ParamStyle::Simple, ..Self::new(name, param_type) }
    }
    pub fn required(mut self) -> Self { self.required = true; self }
    pub fn describes(mut self, description: impl Into<String>) -> Self { self.description = Some(description.into()); self }
    pub fn rust_name(mut self, name: impl Into<String>) -> Self { self.rust_name = Some(name.into()); self }
    pub fn secret(mut self) -> Self { self.secret = true; self }
    pub fn pager_managed(mut self) -> Self { self.pager_managed = true; self }
    pub fn deprecated(mut self) -> Self { self.deprecated = true; self }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn param_def_debug_clone_eq() {
        let param = ParamDef { rust_name: None, secret: false, pager_managed: false, deprecated: false,
            name: "limit".to_string(),
            required: false,
            description: Some("Max results".to_string()),
            param_type: QueryParamType::Integer,
            explode: false,
            style: ParamStyle::Form,
        };
        let cloned = param.clone();
        assert_eq!(param, cloned);
        assert!(format!("{:?}", param).contains("limit"));
    }

    #[test]
    fn param_def_required() {
        let param = ParamDef { rust_name: None, secret: false, pager_managed: false, deprecated: false,
            name: "id".to_string(),
            required: true,
            description: None,
            param_type: QueryParamType::String,
            explode: false,
            style: ParamStyle::Simple,
        };
        assert!(param.required);
    }

    #[test]
    fn param_def_with_explode() {
        let param = ParamDef { rust_name: None, secret: false, pager_managed: false, deprecated: false,
            name: "tags".to_string(),
            required: false,
            description: None,
            param_type: QueryParamType::Array(Box::new(QueryParamType::String)),
            explode: true,
            style: ParamStyle::Form,
        };
        assert!(param.explode);
    }

    #[test]
    fn query_param_type_debug_clone_eq() {
        let param_type = QueryParamType::String;
        let cloned = param_type.clone();
        assert_eq!(param_type, cloned);
        assert!(format!("{:?}", param_type).contains("String"));
    }

    #[test]
    fn query_param_type_string() {
        let t = QueryParamType::String;
        assert!(matches!(t, QueryParamType::String));
    }

    #[test]
    fn query_param_type_integer() {
        let t = QueryParamType::Integer;
        assert!(matches!(t, QueryParamType::Integer));
    }

    #[test]
    fn query_param_type_number() {
        let t = QueryParamType::Number;
        assert!(matches!(t, QueryParamType::Number));
    }

    #[test]
    fn query_param_type_boolean() {
        let t = QueryParamType::Boolean;
        assert!(matches!(t, QueryParamType::Boolean));
    }

    #[test]
    fn query_param_type_array() {
        let t = QueryParamType::Array(Box::new(QueryParamType::String));
        if let QueryParamType::Array(inner) = t {
            assert!(matches!(*inner, QueryParamType::String));
        } else {
            panic!("Expected Array variant");
        }
    }

    #[test]
    fn query_param_type_enum() {
        let t = QueryParamType::Enum(vec!["asc".to_string(), "desc".to_string()]);
        if let QueryParamType::Enum(values) = t {
            assert_eq!(values.len(), 2);
            assert_eq!(values[0], "asc");
        } else {
            panic!("Expected Enum variant");
        }
    }

    #[test]
    fn query_param_type_json() {
        let t = QueryParamType::Json;
        assert!(matches!(t, QueryParamType::Json));
    }

    #[test]
    fn query_param_type_nested_array() {
        let t = QueryParamType::Array(Box::new(QueryParamType::Array(Box::new(
            QueryParamType::String,
        ))));
        if let QueryParamType::Array(outer) = t {
            if let QueryParamType::Array(inner) = *outer {
                assert!(matches!(*inner, QueryParamType::String));
            } else {
                panic!("Expected inner Array");
            }
        } else {
            panic!("Expected outer Array");
        }
    }

    #[test]
    fn param_style_debug_clone_eq() {
        let style = ParamStyle::Form;
        #[allow(clippy::clone_on_copy)]
        let cloned = style.clone();
        assert_eq!(style, cloned);
        assert!(format!("{:?}", style).contains("Form"));
    }

    #[test]
    fn param_style_copy() {
        let original = ParamStyle::Simple;
        let copied = original;
        assert_eq!(original, copied);
    }

    #[test]
    fn param_style_hash() {
        use std::collections::HashSet;
        let mut set = HashSet::new();
        set.insert(ParamStyle::Form);
        set.insert(ParamStyle::Simple);
        set.insert(ParamStyle::PipeDelimited);
        assert_eq!(set.len(), 3);
        assert!(set.contains(&ParamStyle::Form));
    }

    #[test]
    fn param_style_all_variants() {
        let variants = [
            ParamStyle::Form,
            ParamStyle::Simple,
            ParamStyle::SpaceDelimited,
            ParamStyle::PipeDelimited,
            ParamStyle::DeepObject,
        ];
        assert_eq!(variants.len(), 5);
    }
}
