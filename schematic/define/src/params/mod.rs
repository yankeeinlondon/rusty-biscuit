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
//! For pagination types, see [`crate::pagination`].
//!
//! ## Examples
//!
//! Define endpoint parameters using the builder pattern (recommended):
//!
//! ```
//! use schematic_define::params::{EndpointParams, QueryParamType};
//!
//! let params = EndpointParams::default()
//!     .with_query_param("state", QueryParamType::String, false, Some("Filter by state"));
//!
//! assert_eq!(params.query.len(), 1);
//! ```
//!
//! Or construct directly with all fields:
//!
//! ```
//! use schematic_define::params::{EndpointParams, ParamDef, QueryParamType, ParamStyle};
//!
//! let params = EndpointParams {
//!     query: vec![
//!         ParamDef { rust_name: None, secret: false, pager_managed: false,
//!             name: "page".to_string(),
//!             required: false,
//!             description: Some("Page number".to_string()),
//!             param_type: QueryParamType::Integer,
//!             explode: false,
//!             style: ParamStyle::Form,
//!         },
//!         ParamDef { rust_name: None, secret: false, pager_managed: false,
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
//!
//! };
//!
//! assert_eq!(params.query.len(), 2);
//! ```

mod param_def;

pub use param_def::{ParamDef, ParamStyle, QueryParamType};

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
#[derive(Debug, Clone, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
pub struct EndpointParams {
    /// Query string parameters.
    pub query: Vec<ParamDef>,
    /// HTTP header parameters.
    pub header: Vec<ParamDef>,
    /// Cookie parameters.
    pub cookie: Vec<ParamDef>,

}

impl EndpointParams {
    /// An endpoint with no declared parameters.
    pub fn new() -> Self { Self::default() }

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
        self.query.push(ParamDef { rust_name: None, secret: false, pager_managed: false,
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

}

#[cfg(test)]
mod tests {
    use super::*;

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
            query: vec![ParamDef { rust_name: None, secret: false, pager_managed: false,
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
            query: vec![ParamDef { rust_name: None, secret: false, pager_managed: false,
                name: "q".to_string(),
                required: true,
                description: Some("Search query".to_string()),
                param_type: QueryParamType::String,
                explode: false,
                style: ParamStyle::Form,
            }],
            header: vec![ParamDef { rust_name: None, secret: false, pager_managed: false,
                name: "X-Request-Id".to_string(),
                required: false,
                description: None,
                param_type: QueryParamType::String,
                explode: false,
                style: ParamStyle::Simple,
            }],
            cookie: vec![ParamDef { rust_name: None, secret: false, pager_managed: false,
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

    #[test]
    fn endpoint_params_pagination_example() {
        let params = EndpointParams {
            query: vec![
                ParamDef { rust_name: None, secret: false, pager_managed: false,
                    name: "page".to_string(),
                    required: false,
                    description: Some("Page number (1-indexed)".to_string()),
                    param_type: QueryParamType::Integer,
                    explode: false,
                    style: ParamStyle::Form,
                },
                ParamDef { rust_name: None, secret: false, pager_managed: false,
                    name: "per_page".to_string(),
                    required: false,
                    description: Some("Items per page (default: 20)".to_string()),
                    param_type: QueryParamType::Integer,
                    explode: false,
                    style: ParamStyle::Form,
                },
                ParamDef { rust_name: None, secret: false, pager_managed: false,
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
            query: vec![ParamDef { rust_name: None, secret: false, pager_managed: false,
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




















}
