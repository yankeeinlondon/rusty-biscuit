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
//! - [`PaginationStyle`] - Common pagination patterns with builder support
//!
//! ## Pagination Helpers
//!
//! Use [`PaginationStyle`] to add pagination parameters with sensible defaults:
//!
//! ```
//! use schematic_define::params::{EndpointParams, PaginationStyle};
//!
//! // Bitbucket-style pagination (page + pagelen)
//! let params = EndpointParams::default().with_pagination(PaginationStyle::PageNumber {
//!     page_param: "page".to_string(),
//!     per_page_param: "pagelen".to_string(),
//!     default_per_page: 50,
//!     max_per_page: 100,
//! });
//!
//! // GitHub/GitLab-style pagination (page + per_page)
//! let params = EndpointParams::default().with_pagination(PaginationStyle::PageNumber {
//!     page_param: "page".to_string(),
//!     per_page_param: "per_page".to_string(),
//!     default_per_page: 100,
//!     max_per_page: 100,
//! });
//!
//! // Cursor-based pagination
//! let params = EndpointParams::default().with_pagination(PaginationStyle::Cursor {
//!     cursor_param: "after".to_string(),
//!     limit_param: Some("limit".to_string()),
//!     default_limit: 20,
//! });
//! ```
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

/// Common pagination patterns for API endpoints.
///
/// Provides standardized pagination parameter definitions for common API patterns.
/// Use with [`EndpointParams::with_pagination`] to add pagination to list endpoints.
///
/// ## Examples
///
/// ```
/// use schematic_define::params::{EndpointParams, PaginationStyle};
///
/// // Bitbucket-style (page + pagelen)
/// let params = EndpointParams::default()
///     .with_pagination(PaginationStyle::bitbucket());
///
/// // GitHub/GitLab-style (page + per_page)
/// let params = EndpointParams::default()
///     .with_pagination(PaginationStyle::github());
///
/// // Cursor-based (after + limit)
/// let params = EndpointParams::default()
///     .with_pagination(PaginationStyle::cursor("after", Some("limit"), 20));
/// ```
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PaginationStyle {
    /// Page number-based pagination (page + per_page).
    ///
    /// Used by GitHub, GitLab, and many REST APIs.
    /// Page numbers are typically 1-indexed.
    PageNumber {
        /// Query parameter name for page number (e.g., "page").
        page_param: String,
        /// Query parameter name for items per page (e.g., "per_page", "pagelen").
        per_page_param: String,
        /// Default number of items per page.
        default_per_page: u32,
        /// Maximum allowed items per page.
        max_per_page: u32,
    },

    /// Offset/limit-based pagination.
    ///
    /// Used by APIs that prefer absolute positioning over page numbers.
    OffsetLimit {
        /// Query parameter name for offset (e.g., "offset", "skip").
        offset_param: String,
        /// Query parameter name for limit (e.g., "limit", "count").
        limit_param: String,
        /// Default limit value.
        default_limit: u32,
        /// Maximum allowed limit.
        max_limit: u32,
    },

    /// Cursor-based pagination.
    ///
    /// Used by APIs for stable pagination through large or changing datasets.
    /// The cursor is typically an opaque token from the previous response.
    Cursor {
        /// Query parameter name for cursor (e.g., "after", "cursor", "next_token").
        cursor_param: String,
        /// Optional query parameter for limit/page size.
        limit_param: Option<String>,
        /// Default limit value.
        default_limit: u32,
    },

    /// Bitbucket-specific pagination (page + pagelen).
    ///
    /// Bitbucket uses `pagelen` instead of `per_page` and supports `page`
    /// for 1-indexed page navigation.
    Bitbucket {
        /// Default page size (typically 50).
        default_pagelen: u32,
    },
}

impl PaginationStyle {
    /// Creates GitHub-style pagination (page + per_page, default 100).
    ///
    /// ## Example
    ///
    /// ```
    /// use schematic_define::params::PaginationStyle;
    ///
    /// let style = PaginationStyle::github();
    /// ```
    pub fn github() -> Self {
        Self::PageNumber {
            page_param: "page".to_string(),
            per_page_param: "per_page".to_string(),
            default_per_page: 100,
            max_per_page: 100,
        }
    }

    /// Creates GitLab-style pagination (same as GitHub).
    ///
    /// ## Example
    ///
    /// ```
    /// use schematic_define::params::PaginationStyle;
    ///
    /// let style = PaginationStyle::gitlab();
    /// ```
    pub fn gitlab() -> Self {
        Self::github()
    }

    /// Creates Bitbucket-style pagination (page + pagelen, default 50).
    ///
    /// ## Example
    ///
    /// ```
    /// use schematic_define::params::PaginationStyle;
    ///
    /// let style = PaginationStyle::bitbucket();
    /// ```
    pub fn bitbucket() -> Self {
        Self::Bitbucket {
            default_pagelen: 50,
        }
    }

    /// Creates cursor-based pagination.
    ///
    /// ## Arguments
    ///
    /// * `cursor_param` - Name of the cursor parameter (e.g., "after", "cursor")
    /// * `limit_param` - Optional name of the limit parameter
    /// * `default_limit` - Default page size
    ///
    /// ## Example
    ///
    /// ```
    /// use schematic_define::params::PaginationStyle;
    ///
    /// let style = PaginationStyle::cursor("after", Some("limit"), 20);
    /// ```
    pub fn cursor(cursor_param: &str, limit_param: Option<&str>, default_limit: u32) -> Self {
        Self::Cursor {
            cursor_param: cursor_param.to_string(),
            limit_param: limit_param.map(|s| s.to_string()),
            default_limit,
        }
    }

    /// Creates offset/limit-based pagination.
    ///
    /// ## Arguments
    ///
    /// * `offset_param` - Name of the offset parameter (e.g., "offset", "skip")
    /// * `limit_param` - Name of the limit parameter
    /// * `default_limit` - Default limit value
    /// * `max_limit` - Maximum allowed limit
    ///
    /// ## Example
    ///
    /// ```
    /// use schematic_define::params::PaginationStyle;
    ///
    /// let style = PaginationStyle::offset_limit("offset", "limit", 20, 100);
    /// ```
    pub fn offset_limit(
        offset_param: &str,
        limit_param: &str,
        default_limit: u32,
        max_limit: u32,
    ) -> Self {
        Self::OffsetLimit {
            offset_param: offset_param.to_string(),
            limit_param: limit_param.to_string(),
            default_limit,
            max_limit,
        }
    }

    /// Converts the pagination style into query parameters.
    ///
    /// Returns a vector of [`ParamDef`] instances suitable for adding to
    /// [`EndpointParams::query`].
    pub fn to_query_params(&self) -> Vec<ParamDef> {
        match self {
            Self::PageNumber {
                page_param,
                per_page_param,
                default_per_page,
                max_per_page,
            } => {
                vec![
                    ParamDef {
                        name: page_param.clone(),
                        required: false,
                        description: Some("Page number (1-indexed, default: 1)".to_string()),
                        param_type: QueryParamType::Integer,
                        explode: false,
                        style: ParamStyle::Form,
                    },
                    ParamDef {
                        name: per_page_param.clone(),
                        required: false,
                        description: Some(format!(
                            "Items per page (default: {}, max: {})",
                            default_per_page, max_per_page
                        )),
                        param_type: QueryParamType::Integer,
                        explode: false,
                        style: ParamStyle::Form,
                    },
                ]
            }
            Self::OffsetLimit {
                offset_param,
                limit_param,
                default_limit,
                max_limit,
            } => {
                vec![
                    ParamDef {
                        name: offset_param.clone(),
                        required: false,
                        description: Some("Number of items to skip".to_string()),
                        param_type: QueryParamType::Integer,
                        explode: false,
                        style: ParamStyle::Form,
                    },
                    ParamDef {
                        name: limit_param.clone(),
                        required: false,
                        description: Some(format!(
                            "Maximum items to return (default: {}, max: {})",
                            default_limit, max_limit
                        )),
                        param_type: QueryParamType::Integer,
                        explode: false,
                        style: ParamStyle::Form,
                    },
                ]
            }
            Self::Cursor {
                cursor_param,
                limit_param,
                default_limit: _,
            } => {
                let mut params = vec![ParamDef {
                    name: cursor_param.clone(),
                    required: false,
                    description: Some("Pagination cursor from previous response".to_string()),
                    param_type: QueryParamType::String,
                    explode: false,
                    style: ParamStyle::Form,
                }];

                if let Some(limit_name) = limit_param {
                    params.push(ParamDef {
                        name: limit_name.clone(),
                        required: false,
                        description: Some("Maximum items to return".to_string()),
                        param_type: QueryParamType::Integer,
                        explode: false,
                        style: ParamStyle::Form,
                    });
                }

                params
            }
            Self::Bitbucket { default_pagelen } => {
                vec![
                    ParamDef {
                        name: "page".to_string(),
                        required: false,
                        description: Some("Page number (1-indexed)".to_string()),
                        param_type: QueryParamType::Integer,
                        explode: false,
                        style: ParamStyle::Form,
                    },
                    ParamDef {
                        name: "pagelen".to_string(),
                        required: false,
                        description: Some(format!(
                            "Items per page (default: {}, max: 100)",
                            default_pagelen
                        )),
                        param_type: QueryParamType::Integer,
                        explode: false,
                        style: ParamStyle::Form,
                    },
                ]
            }
        }
    }
}

impl EndpointParams {
    /// Adds pagination parameters to this endpoint.
    ///
    /// Appends pagination query parameters to the existing query params.
    /// Use this for list endpoints that support pagination.
    ///
    /// ## Examples
    ///
    /// ```
    /// use schematic_define::params::{EndpointParams, PaginationStyle};
    ///
    /// // Add Bitbucket-style pagination
    /// let params = EndpointParams::default()
    ///     .with_pagination(PaginationStyle::bitbucket());
    ///
    /// assert_eq!(params.query.len(), 2);
    /// assert!(params.query.iter().any(|p| p.name == "page"));
    /// assert!(params.query.iter().any(|p| p.name == "pagelen"));
    /// ```
    pub fn with_pagination(mut self, style: PaginationStyle) -> Self {
        self.query.extend(style.to_query_params());
        self
    }

    /// Adds a single query parameter.
    ///
    /// ## Examples
    ///
    /// ```
    /// use schematic_define::params::{EndpointParams, QueryParamType, ParamStyle};
    ///
    /// let params = EndpointParams::default()
    ///     .with_query_param("state", QueryParamType::Enum(vec![
    ///         "open".to_string(),
    ///         "closed".to_string(),
    ///         "all".to_string(),
    ///     ]), false, Some("Filter by state"));
    ///
    /// assert_eq!(params.query.len(), 1);
    /// assert_eq!(params.query[0].name, "state");
    /// ```
    pub fn with_query_param(
        mut self,
        name: &str,
        param_type: QueryParamType,
        required: bool,
        description: Option<&str>,
    ) -> Self {
        self.query.push(ParamDef {
            name: name.to_string(),
            required,
            description: description.map(|s| s.to_string()),
            param_type,
            explode: false,
            style: ParamStyle::Form,
        });
        self
    }

    /// Adds multiple query parameters.
    ///
    /// ## Examples
    ///
    /// ```
    /// use schematic_define::params::{EndpointParams, QueryParamType};
    ///
    /// let params = EndpointParams::default()
    ///     .with_query_params(vec![
    ///         ("sort", QueryParamType::String, false, Some("Sort field")),
    ///         ("order", QueryParamType::Enum(vec!["asc".into(), "desc".into()]), false, Some("Sort order")),
    ///     ]);
    ///
    /// assert_eq!(params.query.len(), 2);
    /// ```
    pub fn with_query_params(
        mut self,
        params: Vec<(&str, QueryParamType, bool, Option<&str>)>,
    ) -> Self {
        for (name, param_type, required, description) in params {
            self = self.with_query_param(name, param_type, required, description);
        }
        self
    }

    /// Checks if this endpoint has pagination parameters.
    ///
    /// Returns `true` if the endpoint has common pagination parameter names:
    /// `page`, `per_page`, `pagelen`, `offset`, `limit`, `cursor`, `after`, `before`.
    pub fn has_pagination(&self) -> bool {
        let pagination_params = [
            "page", "per_page", "pagelen", "offset", "limit", "cursor", "after", "before",
        ];
        self.query
            .iter()
            .any(|p| pagination_params.contains(&p.name.as_str()))
    }
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
                    param_type: QueryParamType::Enum(vec!["asc".to_string(), "desc".to_string()]),
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

    // ========== PaginationStyle Tests ==========

    #[test]
    fn pagination_style_github() {
        let style = PaginationStyle::github();
        let params = style.to_query_params();

        assert_eq!(params.len(), 2);
        assert!(params.iter().any(|p| p.name == "page"));
        assert!(params.iter().any(|p| p.name == "per_page"));
    }

    #[test]
    fn pagination_style_gitlab() {
        let style = PaginationStyle::gitlab();
        let params = style.to_query_params();

        assert_eq!(params.len(), 2);
        assert!(params.iter().any(|p| p.name == "page"));
        assert!(params.iter().any(|p| p.name == "per_page"));
    }

    #[test]
    fn pagination_style_bitbucket() {
        let style = PaginationStyle::bitbucket();
        let params = style.to_query_params();

        assert_eq!(params.len(), 2);
        assert!(params.iter().any(|p| p.name == "page"));
        assert!(params.iter().any(|p| p.name == "pagelen"));
    }

    #[test]
    fn pagination_style_cursor() {
        let style = PaginationStyle::cursor("after", Some("limit"), 20);
        let params = style.to_query_params();

        assert_eq!(params.len(), 2);
        assert!(params.iter().any(|p| p.name == "after"));
        assert!(params.iter().any(|p| p.name == "limit"));
    }

    #[test]
    fn pagination_style_cursor_without_limit() {
        let style = PaginationStyle::cursor("cursor", None, 20);
        let params = style.to_query_params();

        assert_eq!(params.len(), 1);
        assert_eq!(params[0].name, "cursor");
    }

    #[test]
    fn pagination_style_offset_limit() {
        let style = PaginationStyle::offset_limit("offset", "limit", 20, 100);
        let params = style.to_query_params();

        assert_eq!(params.len(), 2);
        assert!(params.iter().any(|p| p.name == "offset"));
        assert!(params.iter().any(|p| p.name == "limit"));
    }

    // ========== EndpointParams Builder Tests ==========

    #[test]
    fn endpoint_params_with_pagination_bitbucket() {
        let params = EndpointParams::default().with_pagination(PaginationStyle::bitbucket());

        assert_eq!(params.query.len(), 2);
        assert!(params.has_pagination());
    }

    #[test]
    fn endpoint_params_with_pagination_github() {
        let params = EndpointParams::default().with_pagination(PaginationStyle::github());

        assert_eq!(params.query.len(), 2);
        assert!(params.has_pagination());
    }

    #[test]
    fn endpoint_params_with_query_param() {
        let params = EndpointParams::default().with_query_param(
            "state",
            QueryParamType::String,
            false,
            Some("Filter state"),
        );

        assert_eq!(params.query.len(), 1);
        assert_eq!(params.query[0].name, "state");
        assert!(!params.query[0].required);
    }

    #[test]
    fn endpoint_params_with_query_params() {
        let params = EndpointParams::default().with_query_params(vec![
            ("sort", QueryParamType::String, false, Some("Sort field")),
            (
                "order",
                QueryParamType::Enum(vec!["asc".into(), "desc".into()]),
                false,
                Some("Sort order"),
            ),
        ]);

        assert_eq!(params.query.len(), 2);
    }

    #[test]
    fn endpoint_params_has_pagination_detects_page() {
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

        assert!(params.has_pagination());
    }

    #[test]
    fn endpoint_params_has_pagination_detects_cursor() {
        let params = EndpointParams {
            query: vec![ParamDef {
                name: "after".to_string(),
                required: false,
                description: None,
                param_type: QueryParamType::String,
                explode: false,
                style: ParamStyle::Form,
            }],
            header: vec![],
            cookie: vec![],
        };

        assert!(params.has_pagination());
    }

    #[test]
    fn endpoint_params_has_pagination_returns_false_without_pagination() {
        let params = EndpointParams {
            query: vec![ParamDef {
                name: "state".to_string(),
                required: false,
                description: None,
                param_type: QueryParamType::String,
                explode: false,
                style: ParamStyle::Form,
            }],
            header: vec![],
            cookie: vec![],
        };

        assert!(!params.has_pagination());
    }

    #[test]
    fn endpoint_params_chained_builders() {
        let params = EndpointParams::default()
            .with_query_param("state", QueryParamType::String, false, Some("Filter state"))
            .with_pagination(PaginationStyle::github());

        assert_eq!(params.query.len(), 3);
        assert!(params.has_pagination());
    }
}
