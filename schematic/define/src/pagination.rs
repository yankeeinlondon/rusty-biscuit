//! Pagination types for API endpoints.
//!
//! This module provides [`PaginationStyle`] for common pagination request patterns
//! and [`PaginationResponse`] for describing how APIs signal pagination state in responses.
//!
//! ## Examples
//!
//! ```
//! use schematic_define::pagination::PaginationStyle;
//!
//! // GitHub-style pagination
//! let style = PaginationStyle::github();
//!
//! // Cursor-based pagination
//! let style = PaginationStyle::cursor("after", Some("limit"), 20);
//! ```

use serde::{Deserialize, Serialize};

use crate::params::{ParamDef, ParamStyle, QueryParamType};

/// Common pagination patterns for API endpoints.
///
/// Provides standardized pagination parameter definitions for common API patterns.
///
/// ## Examples
///
/// ```
/// use schematic_define::pagination::PaginationStyle;
///
/// let style = PaginationStyle::github();
/// let params = style.to_query_params();
/// assert_eq!(params.len(), 2);
/// ```
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PaginationStyle {
    /// Page number-based pagination (page + per_page).
    PageNumber {
        page_param: String,
        per_page_param: String,
        default_per_page: u32,
        max_per_page: u32,
    },

    /// Offset/limit-based pagination.
    OffsetLimit {
        offset_param: String,
        limit_param: String,
        default_limit: u32,
        max_limit: u32,
    },

    /// Cursor-based pagination.
    Cursor {
        cursor_param: String,
        limit_param: Option<String>,
        default_limit: u32,
    },
}

impl PaginationStyle {
    /// Creates GitHub-style pagination (page + per_page, default 100).
    ///
    /// ## Example
    ///
    /// ```
    /// use schematic_define::pagination::PaginationStyle;
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
    /// use schematic_define::pagination::PaginationStyle;
    ///
    /// let style = PaginationStyle::gitlab();
    /// ```
    pub fn gitlab() -> Self {
        Self::github()
    }

    /// Creates Bitbucket-style pagination (page + pagelen, default 50, max 100).
    ///
    /// ## Example
    ///
    /// ```
    /// use schematic_define::pagination::PaginationStyle;
    ///
    /// let style = PaginationStyle::bitbucket();
    /// ```
    pub fn bitbucket() -> Self {
        Self::PageNumber {
            page_param: "page".to_string(),
            per_page_param: "pagelen".to_string(),
            default_per_page: 50,
            max_per_page: 100,
        }
    }

    /// Creates Gitea-style pagination (page + limit, default 50).
    ///
    /// ## Example
    ///
    /// ```
    /// use schematic_define::pagination::PaginationStyle;
    ///
    /// let style = PaginationStyle::gitea();
    /// ```
    pub fn gitea() -> Self {
        Self::PageNumber {
            page_param: "page".to_string(),
            per_page_param: "limit".to_string(),
            default_per_page: 50,
            max_per_page: 100,
        }
    }

    /// Creates cursor-based pagination.
    ///
    /// ## Example
    ///
    /// ```
    /// use schematic_define::pagination::PaginationStyle;
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
    /// ## Example
    ///
    /// ```
    /// use schematic_define::pagination::PaginationStyle;
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
                default_limit,
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
                        description: Some(format!(
                            "Maximum items to return (default: {})",
                            default_limit
                        )),
                        param_type: QueryParamType::Integer,
                        explode: false,
                        style: ParamStyle::Form,
                    });
                }

                params
            }
        }
    }
}

/// Describes how an API signals pagination state in responses.
///
/// ## Examples
///
/// ```
/// use schematic_define::pagination::PaginationResponse;
///
/// let response = PaginationResponse::LinkHeader;
///
/// let response = PaginationResponse::BodyField {
///     next_field: "next_page_url".to_string(),
/// };
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum PaginationResponse {
    /// Next page URL is in a response body field.
    BodyField {
        next_field: String,
    },

    /// Pagination via RFC 5988 Link header.
    LinkHeader,

    /// Total count with current page tracking.
    TotalCount {
        total_field: String,
        page_field: Option<String>,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

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

        match style {
            PaginationStyle::PageNumber {
                page_param,
                per_page_param,
                default_per_page,
                max_per_page,
            } => {
                assert_eq!(page_param, "page");
                assert_eq!(per_page_param, "pagelen");
                assert_eq!(default_per_page, 50);
                assert_eq!(max_per_page, 100);
            }
            _ => panic!("bitbucket() should return PageNumber variant"),
        }
    }

    #[test]
    fn pagination_style_gitea() {
        let style = PaginationStyle::gitea();
        let params = style.to_query_params();

        assert_eq!(params.len(), 2);
        assert!(params.iter().any(|p| p.name == "page"));
        assert!(params.iter().any(|p| p.name == "limit"));

        match style {
            PaginationStyle::PageNumber {
                page_param,
                per_page_param,
                default_per_page,
                max_per_page,
            } => {
                assert_eq!(page_param, "page");
                assert_eq!(per_page_param, "limit");
                assert_eq!(default_per_page, 50);
                assert_eq!(max_per_page, 100);
            }
            _ => panic!("gitea() should return PageNumber variant"),
        }
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

    #[test]
    fn pagination_response_body_field_construction() {
        let response = PaginationResponse::BodyField {
            next_field: "next_page_url".to_string(),
        };

        match response {
            PaginationResponse::BodyField { next_field } => {
                assert_eq!(next_field, "next_page_url");
            }
            _ => panic!("Expected BodyField variant"),
        }
    }

    #[test]
    fn pagination_response_link_header_construction() {
        let response = PaginationResponse::LinkHeader;
        assert!(matches!(response, PaginationResponse::LinkHeader));
    }

    #[test]
    fn pagination_response_total_count_construction() {
        let response = PaginationResponse::TotalCount {
            total_field: "total".to_string(),
            page_field: Some("page".to_string()),
        };

        match response {
            PaginationResponse::TotalCount {
                total_field,
                page_field,
            } => {
                assert_eq!(total_field, "total");
                assert_eq!(page_field, Some("page".to_string()));
            }
            _ => panic!("Expected TotalCount variant"),
        }
    }

    #[test]
    fn pagination_response_total_count_without_page_field() {
        let response = PaginationResponse::TotalCount {
            total_field: "count".to_string(),
            page_field: None,
        };

        match response {
            PaginationResponse::TotalCount {
                total_field,
                page_field,
            } => {
                assert_eq!(total_field, "count");
                assert!(page_field.is_none());
            }
            _ => panic!("Expected TotalCount variant"),
        }
    }

    #[test]
    fn pagination_response_debug_clone_eq() {
        let response = PaginationResponse::LinkHeader;
        let cloned = response.clone();
        assert_eq!(response, cloned);
        assert!(format!("{:?}", response).contains("LinkHeader"));
    }

    #[test]
    fn pagination_response_serialization_body_field() {
        let response = PaginationResponse::BodyField {
            next_field: "next".to_string(),
        };
        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("body_field"));
        assert!(json.contains("next_field"));
        assert!(json.contains("\"next\""));

        let deserialized: PaginationResponse = serde_json::from_str(&json).unwrap();
        assert_eq!(response, deserialized);
    }

    #[test]
    fn pagination_response_serialization_link_header() {
        let response = PaginationResponse::LinkHeader;
        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("link_header"));

        let deserialized: PaginationResponse = serde_json::from_str(&json).unwrap();
        assert_eq!(response, deserialized);
    }

    #[test]
    fn pagination_response_serialization_total_count() {
        let response = PaginationResponse::TotalCount {
            total_field: "total".to_string(),
            page_field: Some("current_page".to_string()),
        };
        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("total_count"));
        assert!(json.contains("total_field"));
        assert!(json.contains("page_field"));

        let deserialized: PaginationResponse = serde_json::from_str(&json).unwrap();
        assert_eq!(response, deserialized);
    }

    #[test]
    fn pagination_response_serialization_total_count_null_page() {
        let response = PaginationResponse::TotalCount {
            total_field: "total".to_string(),
            page_field: None,
        };
        let json = serde_json::to_string(&response).unwrap();

        let deserialized: PaginationResponse = serde_json::from_str(&json).unwrap();
        assert_eq!(response, deserialized);
    }
}
