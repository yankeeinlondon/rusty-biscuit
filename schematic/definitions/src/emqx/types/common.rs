use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Pagination metadata in EMQX list responses.
///
/// ## Example
///
/// ```json
/// {
///   "count": 100,
///   "limit": 100,
///   "page": 1,
///   "hasnext": true
/// }
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct PaginationMeta {
    /// Total count of items.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub count: Option<u64>,

    /// Maximum items per page.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub limit: Option<u32>,

    /// Current page number.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub page: Option<u32>,

    /// Whether more pages exist.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hasnext: Option<bool>,
}

/// Standard error response from EMQX API.
///
/// ## Example
///
/// ```json
/// {
///   "code": "RESOURCE_NOT_FOUND",
///   "reason": "Client id not found"
/// }
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ErrorResponse {
    /// Error code (e.g., "RESOURCE_NOT_FOUND", "BAD_REQUEST").
    pub code: String,

    /// Human-readable error description.
    pub reason: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn error_response_deserialization() {
        let json = r#"{"code": "RESOURCE_NOT_FOUND", "reason": "Client id not found"}"#;
        let err: ErrorResponse = serde_json::from_str(json).unwrap();
        assert_eq!(err.code, "RESOURCE_NOT_FOUND");
        assert_eq!(err.reason, "Client id not found");
    }

    #[test]
    fn pagination_meta_deserialization() {
        let json = r#"{"count": 100, "limit": 50, "page": 1, "hasnext": true}"#;
        let meta: PaginationMeta = serde_json::from_str(json).unwrap();
        assert_eq!(meta.count, Some(100));
        assert_eq!(meta.hasnext, Some(true));
    }
}
