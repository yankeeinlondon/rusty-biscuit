//! Endpoint parameter definitions for imported API schemas.
//!
//! This module provides types for representing query, header, and cookie parameters
//! that are imported from external API specifications like OpenAPI. These types are
//! general-purpose and not gated behind any feature flag.
//!
//! ## Core Types
//!
//! - [`EndpointParams`] - Collection of parameters for an endpoint (query, header, cookie)
//! - [`ParamDef`] - Definition of a single parameter
//! - [`QueryParamType`] - Type of a parameter value
//! - [`ParamStyle`] - Serialization style for parameter values
//!
//! ## Examples
//!
//! Define endpoint parameters:
//!
//! ```
//! use schematic_define::params::{EndpointParams, ParamDef, QueryParamType, ParamStyle};
//!
//! let params = EndpointParams {
//!     query: vec![
//!         ParamDef {
//!             name: "page".to_string(),
//!             required: false,
//!             description: Some("Page number".to_string()),
//!             param_type: QueryParamType::Integer,
//!             explode: false,
//!             style: ParamStyle::Form,
//!         },
//!         ParamDef {
//!             name: "limit".to_string(),
//!             required: false,
//!             description: Some("Items per page".to_string()),
//!             param_type: QueryParamType::Integer,
//!             explode: false,
//!             style: ParamStyle::Form,
//!         },
//!     ],
//!     header: vec![],
//!     cookie: vec![],
//! };
//!
//! assert_eq!(params.query.len(), 2);
//! ```

/// Collection of parameters for an endpoint.
///
/// Groups parameters by their location: query string, headers, or cookies.
/// Implements `Default` to provide empty parameter collections.
///
/// ## Examples
///
/// ```
/// use schematic_define::params::EndpointParams;
///
/// // Default creates empty collections
/// let params = EndpointParams::default();
/// assert!(params.query.is_empty());
/// assert!(params.header.is_empty());
/// assert!(params.cookie.is_empty());
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct EndpointParams {
    /// Query string parameters.
    pub query: Vec<ParamDef>,
    /// HTTP header parameters.
    pub header: Vec<ParamDef>,
    /// Cookie parameters.
    pub cookie: Vec<ParamDef>,
}

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
/// let param = ParamDef {
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
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParamDef {
    /// Parameter name as used in the API.
    pub name: String,
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
/// Represents the possible types for query, header, or cookie parameters.
/// Note: This is named `QueryParamType` to avoid conflict with
/// [`crate::websocket::ParamType`] which is used for WebSocket connection parameters.
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
#[derive(Debug, Clone, PartialEq, Eq)]
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
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
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

#[cfg(test)]
mod tests {
    use super::*;

    // ========== EndpointParams Tests ==========

    #[test]
    fn endpoint_params_default_empty() {
        let params = EndpointParams::default();
        assert!(params.query.is_empty());
        assert!(params.header.is_empty());
        assert!(params.cookie.is_empty());
    }

    #[test]
    fn endpoint_params_debug_clone_eq() {
        let params = EndpointParams {
            query: vec![],
            header: vec![],
            cookie: vec![],
        };
        let cloned = params.clone();
        assert_eq!(params, cloned);
        assert!(format!("{:?}", params).contains("EndpointParams"));
    }

    #[test]
    fn endpoint_params_with_query() {
        let params = EndpointParams {
            query: vec![ParamDef {
                name: "page".to_string(),
                required: false,
                description: None,
                param_type: QueryParamType::Integer,
                explode: false,
                style: ParamStyle::Form,
            }],
            header: vec![],
            cookie: vec![],
        };
        assert_eq!(params.query.len(), 1);
        assert!(params.header.is_empty());
    }

    #[test]
    fn endpoint_params_with_all_locations() {
        let params = EndpointParams {
            query: vec![ParamDef {
                name: "q".to_string(),
                required: true,
                description: Some("Search query".to_string()),
                param_type: QueryParamType::String,
                explode: false,
                style: ParamStyle::Form,
            }],
            header: vec![ParamDef {
                name: "X-Request-Id".to_string(),
                required: false,
                description: None,
                param_type: QueryParamType::String,
                explode: false,
                style: ParamStyle::Simple,
            }],
            cookie: vec![ParamDef {
                name: "session".to_string(),
                required: true,
                description: None,
                param_type: QueryParamType::String,
                explode: false,
                style: ParamStyle::Form,
            }],
        };
        assert_eq!(params.query.len(), 1);
        assert_eq!(params.header.len(), 1);
        assert_eq!(params.cookie.len(), 1);
    }

    // ========== ParamDef Tests ==========

    #[test]
    fn param_def_debug_clone_eq() {
        let param = ParamDef {
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
        let param = ParamDef {
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
        let param = ParamDef {
            name: "tags".to_string(),
            required: false,
            description: None,
            param_type: QueryParamType::Array(Box::new(QueryParamType::String)),
            explode: true,
            style: ParamStyle::Form,
        };
        assert!(param.explode);
    }

    // ========== QueryParamType Tests ==========

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
        // Array of arrays of strings
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

    // ========== ParamStyle Tests ==========

    #[test]
    fn param_style_debug_clone_eq() {
        let style = ParamStyle::Form;
        let cloned = style.clone();
        assert_eq!(style, cloned);
        assert!(format!("{:?}", style).contains("Form"));
    }

    #[test]
    fn param_style_copy() {
        let original = ParamStyle::Simple;
        let copied = original; // Copy, not move
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
        let variants = vec![
            ParamStyle::Form,
            ParamStyle::Simple,
            ParamStyle::SpaceDelimited,
            ParamStyle::PipeDelimited,
            ParamStyle::DeepObject,
        ];
        assert_eq!(variants.len(), 5);
    }

    // ========== Integration Tests ==========

    #[test]
    fn endpoint_params_pagination_example() {
        let params = EndpointParams {
            query: vec![
                ParamDef {
                    name: "page".to_string(),
                    required: false,
                    description: Some("Page number (1-indexed)".to_string()),
                    param_type: QueryParamType::Integer,
                    explode: false,
                    style: ParamStyle::Form,
                },
                ParamDef {
                    name: "per_page".to_string(),
                    required: false,
                    description: Some("Items per page (default: 20)".to_string()),
                    param_type: QueryParamType::Integer,
                    explode: false,
                    style: ParamStyle::Form,
                },
                ParamDef {
                    name: "sort".to_string(),
                    required: false,
                    description: Some("Sort order".to_string()),
                    param_type: QueryParamType::Enum(vec![
                        "asc".to_string(),
                        "desc".to_string(),
                    ]),
                    explode: false,
                    style: ParamStyle::Form,
                },
            ],
            header: vec![],
            cookie: vec![],
        };

        assert_eq!(params.query.len(), 3);
        assert!(params.query.iter().all(|p| !p.required));
    }

    #[test]
    fn endpoint_params_filter_with_arrays() {
        let params = EndpointParams {
            query: vec![ParamDef {
                name: "status".to_string(),
                required: false,
                description: Some("Filter by status (multiple allowed)".to_string()),
                param_type: QueryParamType::Array(Box::new(QueryParamType::Enum(vec![
                    "pending".to_string(),
                    "active".to_string(),
                    "completed".to_string(),
                ]))),
                explode: true,
                style: ParamStyle::Form,
            }],
            header: vec![],
            cookie: vec![],
        };

        let status_param = &params.query[0];
        assert!(status_param.explode);
        if let QueryParamType::Array(inner) = &status_param.param_type {
            if let QueryParamType::Enum(values) = inner.as_ref() {
                assert_eq!(values.len(), 3);
            } else {
                panic!("Expected inner Enum");
            }
        } else {
            panic!("Expected Array");
        }
    }
}
