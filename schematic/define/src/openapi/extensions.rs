//! Vendor extensions for OpenAPI documents.
//!
//! This module provides structs for `x-schematic` vendor extensions that preserve
//! Schematic-specific metadata in exported OpenAPI documents. These extensions
//! enable round-trip fidelity when importing and re-exporting API definitions.
//!
//! ## Extension Types
//!
//! - [`SchematicDocExtension`] - Document-level extension (API metadata)
//! - [`SchematicOpExtension`] - Operation-level extension (endpoint metadata)
//! - [`SchematicSchemaExtension`] - Schema-level extension (type metadata)

use serde::{Deserialize, Serialize};

use crate::auth::AuthStrategy;
use crate::headers::EnvMapping;
use crate::metadata::{RateLimit, RetryGuidance};
use crate::request::ApiRequest;
use crate::error_table::ErrorEntry;
use crate::response::{ApiResponse, SuccessShape};

/// Document-level vendor extension for Schematic metadata.
///
/// This extension is serialized to `x-schematic` at the OpenAPI document root level.
/// It preserves API-wide configuration that doesn't have native OpenAPI equivalents.
///
/// ## Examples
///
/// ```
/// use schematic_define::openapi::SchematicDocExtension;
///
/// let ext = SchematicDocExtension {
///     module_path: Some("openai".to_string()),
///     request_suffix: None,
///     env_mapping: None,
///     headers: vec![],
///     rate_limit: None,
///     retry_guidance: None,
///     errors: vec![],
/// };
///
/// let json: serde_json::Value = ext.into();
/// assert!(json.get("module_path").is_some());
/// ```
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct SchematicDocExtension {
    /// Custom module path for generated code.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub module_path: Option<String>,

    /// Custom suffix for generated request structs.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub request_suffix: Option<String>,

    /// Environment variable mapping for authentication.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub env_mapping: Option<EnvMapping>,

    /// Default headers for all endpoints.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub headers: Vec<(String, String)>,

    /// The API's rate-limit headers; descriptive only.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rate_limit: Option<RateLimit>,

    /// The API's retry guidance; descriptive only.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub retry_guidance: Option<RetryGuidance>,

    /// The API's default error entries, as declared (R26).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub errors: Vec<ErrorEntry>,
}

impl From<SchematicDocExtension> for serde_json::Value {
    fn from(ext: SchematicDocExtension) -> Self {
        serde_json::to_value(ext).expect("x-schematic extension is always serializable")
    }
}

/// Operation-level vendor extension for Schematic metadata.
///
/// This extension is serialized to `x-schematic` at the OpenAPI operation level.
/// It preserves endpoint-specific configuration that extends OpenAPI capabilities.
///
/// ## Examples
///
/// ```
/// use schematic_define::openapi::SchematicOpExtension;
/// use schematic_define::{ApiRequest, ApiResponse};
///
/// let ext = SchematicOpExtension {
///     request: Some(ApiRequest::json_type("CreateUserBody")),
///     response: ApiResponse::json_type("User"),
///     headers: vec![("X-Custom".to_string(), "value".to_string())],
///     rate_limit: None,
///     retry_guidance: None,
///     success: vec![],
///     errors: vec![],
///     no_declared_errors: false,
/// };
///
/// let json: serde_json::Value = ext.into();
/// assert!(json.get("request").is_some());
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SchematicOpExtension {
    /// Request body specification (preserves FormData field details, etc.).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub request: Option<ApiRequest>,

    /// Response specification.
    pub response: ApiResponse,

    /// Endpoint-specific headers.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub headers: Vec<(String, String)>,

    /// The endpoint's own rate-limit headers, which replace the API's whole;
    /// absent when the endpoint inherits.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rate_limit: Option<RateLimit>,

    /// The endpoint's own retry guidance, which replaces the API's whole;
    /// absent when the endpoint inherits.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub retry_guidance: Option<RetryGuidance>,

    /// Every success shape, written only when `response` cannot carry them:
    /// several shapes, a stream, or any selection fact, variant name, or note
    /// (R26).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub success: Vec<SuccessShape>,

    /// The endpoint's own error entries, which overlay the API's `errors`.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub errors: Vec<ErrorEntry>,

    /// The endpoint opts out of the API's default errors.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub no_declared_errors: bool,
}

impl From<SchematicOpExtension> for serde_json::Value {
    fn from(ext: SchematicOpExtension) -> Self {
        serde_json::to_value(ext).expect("x-schematic extension is always serializable")
    }
}

/// Schema-level vendor extension for Schematic metadata.
///
/// This extension is serialized to `x-schematic` at the OpenAPI schema level.
/// It preserves type information that enables code generation.
///
/// ## Examples
///
/// ```
/// use schematic_define::openapi::SchematicSchemaExtension;
///
/// let ext = SchematicSchemaExtension {
///     rust_type: Some("crate::models::User".to_string()),
/// };
///
/// let json: serde_json::Value = ext.into();
/// assert!(json.get("rust_type").is_some());
/// ```
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct SchematicSchemaExtension {
    /// Full Rust type path for this schema.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rust_type: Option<String>,
}

impl From<SchematicSchemaExtension> for serde_json::Value {
    fn from(ext: SchematicSchemaExtension) -> Self {
        serde_json::to_value(ext).expect("x-schematic extension is always serializable")
    }
}

/// Authentication extension for preserving AuthStrategy details.
///
/// This is embedded within [`SchematicDocExtension`] to preserve authentication
/// configuration beyond what OpenAPI security schemes support.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AuthExtension {
    /// The original AuthStrategy.
    pub strategy: AuthStrategy,

    /// Environment variable names for token and key credentials (legacy
    /// format); Basic carries its own lists inside `strategy`.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub env_auth: Vec<String>,
}

impl From<AuthExtension> for serde_json::Value {
    fn from(ext: AuthExtension) -> Self {
        serde_json::to_value(ext).expect("x-schematic extension is always serializable")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::headers::{EnvList, EnvMapping};
    use crate::request::FormField;

    // =============================================
    // SchematicDocExtension tests
    // =============================================

    #[test]
    fn doc_extension_default() {
        let ext = SchematicDocExtension::default();
        assert!(ext.module_path.is_none());
        assert!(ext.request_suffix.is_none());
        assert!(ext.env_mapping.is_none());
        assert!(ext.headers.is_empty());
    }

    #[test]
    fn doc_extension_with_module_path() {
        let ext = SchematicDocExtension {
            module_path: Some("openai".to_string()),
            ..Default::default()
        };
        assert_eq!(ext.module_path, Some("openai".to_string()));
    }

    #[test]
    fn doc_extension_with_headers() {
        let ext = SchematicDocExtension {
            headers: vec![("X-Custom".to_string(), "value".to_string())],
            ..Default::default()
        };
        assert_eq!(ext.headers.len(), 1);
    }

    #[test]
    fn doc_extension_with_env_mapping() {
        let mapping = EnvMapping {
            bearer_token: Some(EnvList::single("API_KEY")),
            ..Default::default()
        };
        let ext = SchematicDocExtension {
            env_mapping: Some(mapping.clone()),
            ..Default::default()
        };
        assert!(ext.env_mapping.is_some());
    }

    #[test]
    fn doc_extension_into_json() {
        let ext = SchematicDocExtension {
            module_path: Some("test".to_string()),
            request_suffix: Some("Params".to_string()),
            env_mapping: None,
            headers: vec![],
            ..Default::default()
        };

        let json: serde_json::Value = ext.into();
        assert!(json.is_object());
        assert_eq!(
            json.get("module_path").and_then(|v| v.as_str()),
            Some("test")
        );
        assert_eq!(
            json.get("request_suffix").and_then(|v| v.as_str()),
            Some("Params")
        );
    }

    #[test]
    fn doc_extension_serde_roundtrip() {
        let ext = SchematicDocExtension {
            module_path: Some("mymodule".to_string()),
            request_suffix: Some("Body".to_string()),
            env_mapping: Some(EnvMapping {
                bearer_token: Some(EnvList::single("TOKEN")),
                ..Default::default()
            }),
            headers: vec![("Accept".to_string(), "application/json".to_string())],
            ..Default::default()
        };

        let json = serde_json::to_string(&ext).unwrap();
        let parsed: SchematicDocExtension = serde_json::from_str(&json).unwrap();
        assert_eq!(ext, parsed);
    }

    #[test]
    fn doc_extension_skips_empty_fields_in_json() {
        let ext = SchematicDocExtension::default();
        let json = serde_json::to_string(&ext).unwrap();

        // Empty fields should be skipped
        assert!(!json.contains("module_path"));
        assert!(!json.contains("headers"));
    }

    // =============================================
    // SchematicOpExtension tests
    // =============================================

    #[test]
    fn op_extension_with_json_request() {
        let ext = SchematicOpExtension { rate_limit: None, retry_guidance: None, success: vec![], errors: vec![], no_declared_errors: false,
            request: Some(ApiRequest::json_type("CreateBody")),
            response: ApiResponse::json_type("Response"),
            headers: vec![],
            };
        assert!(ext.request.is_some());
    }

    #[test]
    fn op_extension_with_form_data_request() {
        let ext = SchematicOpExtension { rate_limit: None, retry_guidance: None, success: vec![], errors: vec![], no_declared_errors: false,
            request: Some(ApiRequest::form_data(vec![
                FormField::file("document"),
                FormField::text("name").optional(),
            ])),
            response: ApiResponse::json_type("UploadResponse"),
            headers: vec![],
            };

        if let Some(ApiRequest::FormData { fields }) = &ext.request {
            assert_eq!(fields.len(), 2);
            assert_eq!(fields[0].name, "document");
            assert!(!fields[1].required);
        } else {
            panic!("Expected FormData request");
        }
    }

    #[test]
    fn op_extension_with_binary_response() {
        let ext = SchematicOpExtension { rate_limit: None, retry_guidance: None, success: vec![], errors: vec![], no_declared_errors: false,
            request: None,
            response: ApiResponse::Binary,
            headers: vec![],
            };
        assert!(ext.response.is_binary());
    }

    #[test]
    fn op_extension_with_endpoint_headers() {
        let ext = SchematicOpExtension { rate_limit: None, retry_guidance: None, success: vec![], errors: vec![], no_declared_errors: false,
            request: None,
            response: ApiResponse::Empty,
            headers: vec![("X-Custom-Header".to_string(), "custom-value".to_string())],
            };
        assert_eq!(ext.headers.len(), 1);
    }

    #[test]
    fn op_extension_into_json() {
        let ext = SchematicOpExtension { rate_limit: None, retry_guidance: None, success: vec![], errors: vec![], no_declared_errors: false,
            request: Some(ApiRequest::json_type("TestBody")),
            response: ApiResponse::Text,
            headers: vec![],
            };

        let json: serde_json::Value = ext.into();
        assert!(json.is_object());
        assert!(json.get("request").is_some());
        assert!(json.get("response").is_some());
    }

    #[test]
    fn op_extension_serde_roundtrip() {
        let ext = SchematicOpExtension { rate_limit: None, retry_guidance: None, success: vec![], errors: vec![], no_declared_errors: false,
            request: Some(ApiRequest::form_data(vec![FormField::file("file")])),
            response: ApiResponse::json_type("Result"),
            headers: vec![("X-Header".to_string(), "value".to_string())],
            };

        let json = serde_json::to_string(&ext).unwrap();
        let parsed: SchematicOpExtension = serde_json::from_str(&json).unwrap();
        assert_eq!(ext, parsed);
    }

    // =============================================
    // SchematicSchemaExtension tests
    // =============================================

    #[test]
    fn schema_extension_default() {
        let ext = SchematicSchemaExtension::default();
        assert!(ext.rust_type.is_none());
    }

    #[test]
    fn schema_extension_with_rust_type() {
        let ext = SchematicSchemaExtension {
            rust_type: Some("crate::models::User".to_string()),
        };
        assert_eq!(ext.rust_type, Some("crate::models::User".to_string()));
    }

    #[test]
    fn schema_extension_into_json() {
        let ext = SchematicSchemaExtension {
            rust_type: Some("MyType".to_string()),
        };

        let json: serde_json::Value = ext.into();
        assert_eq!(
            json.get("rust_type").and_then(|v| v.as_str()),
            Some("MyType")
        );
    }

    #[test]
    fn schema_extension_serde_roundtrip() {
        let ext = SchematicSchemaExtension {
            rust_type: Some("crate::types::Response".to_string()),
        };

        let json = serde_json::to_string(&ext).unwrap();
        let parsed: SchematicSchemaExtension = serde_json::from_str(&json).unwrap();
        assert_eq!(ext, parsed);
    }

    #[test]
    fn schema_extension_skips_none_rust_type() {
        let ext = SchematicSchemaExtension::default();
        let json = serde_json::to_string(&ext).unwrap();
        assert!(!json.contains("rust_type"));
    }

    // =============================================
    // AuthExtension tests
    // =============================================

    #[test]
    fn auth_extension_bearer_token() {
        let ext = AuthExtension {
            strategy: AuthStrategy::BearerToken { header: None },
            env_auth: vec!["OPENAI_API_KEY".to_string()],
        };

        assert!(matches!(ext.strategy, AuthStrategy::BearerToken { .. }));
        assert_eq!(ext.env_auth.len(), 1);
    }

    #[test]
    fn auth_extension_basic_auth() {
        let ext = AuthExtension {
            strategy: AuthStrategy::Basic {
                username_env: EnvList::single("API_USERNAME"),
                password_env: EnvList::single("API_PASSWORD"),
            },
            env_auth: vec![],
        };

        let json = serde_json::to_string(&ext).unwrap();
        assert!(json.contains(r#""username_env":{"names":["API_USERNAME"]}"#));
        assert_eq!(serde_json::from_str::<AuthExtension>(&json).unwrap(), ext);
    }

    #[test]
    fn auth_extension_api_key() {
        let ext = AuthExtension {
            strategy: AuthStrategy::ApiKey {
                header: "X-API-Key".to_string(),
                value_prefix: None,
            },
            env_auth: vec!["MY_API_KEY".to_string()],
        };

        if let AuthStrategy::ApiKey { header, .. } = &ext.strategy {
            assert_eq!(header, "X-API-Key");
        } else {
            panic!("Expected ApiKey strategy");
        }
    }

    #[test]
    fn auth_extension_into_json() {
        let ext = AuthExtension {
            strategy: AuthStrategy::BearerToken { header: None },
            env_auth: vec!["TOKEN".to_string()],
        };

        let json: serde_json::Value = ext.into();
        assert!(json.is_object());
        assert!(json.get("strategy").is_some());
        assert!(json.get("env_auth").is_some());
    }

    #[test]
    fn auth_extension_serde_roundtrip() {
        let ext = AuthExtension {
            strategy: AuthStrategy::ApiKey {
                header: "Authorization".to_string(),
                value_prefix: None,
            },
            env_auth: vec!["KEY1".to_string(), "KEY2".to_string()],
        };

        let json = serde_json::to_string(&ext).unwrap();
        let parsed: AuthExtension = serde_json::from_str(&json).unwrap();
        assert_eq!(ext, parsed);
    }

    // =============================================
    // Empty-collection round-trip (re-import own export)
    // =============================================

    #[test]
    fn doc_extension_default_roundtrips_without_headers_field() {
        let ext = SchematicDocExtension::default();
        let json = serde_json::to_string(&ext).unwrap();
        assert!(!json.contains("headers"));
        let parsed: SchematicDocExtension = serde_json::from_str(&json).unwrap();
        assert_eq!(ext, parsed);
    }

    #[test]
    fn op_extension_empty_headers_roundtrips() {
        let ext = SchematicOpExtension { rate_limit: None, retry_guidance: None, success: vec![], errors: vec![], no_declared_errors: false,
            request: Some(ApiRequest::json_type("Body")),
            response: ApiResponse::json_type("Response"),
            headers: vec![],
            };
        let json = serde_json::to_string(&ext).unwrap();
        assert!(!json.contains("headers"));
        let parsed: SchematicOpExtension = serde_json::from_str(&json).unwrap();
        assert_eq!(ext, parsed);
    }

    #[test]
    fn auth_extension_empty_env_auth_roundtrips() {
        let ext = AuthExtension {
            strategy: AuthStrategy::BearerToken { header: None },
            env_auth: vec![],
        };
        let json = serde_json::to_string(&ext).unwrap();
        assert!(!json.contains("env_auth"));
        let parsed: AuthExtension = serde_json::from_str(&json).unwrap();
        assert_eq!(ext, parsed);
    }

    // =============================================
    // Rate-limit and retry metadata
    // =============================================

    fn metadata() -> (RateLimit, RetryGuidance) {
        use crate::metadata::{Backoff, ResetUnit, StatusSelector};
        let rate_limit = RateLimit::new("x-limit", "x-remaining", "x-reset", ResetUnit::HttpDate)
            .unwrap()
            .with_retry_after_header("retry-after")
            .unwrap();
        let retry = RetryGuidance::new(
            [StatusSelector::Code(429), StatusSelector::Range { start: 500, end: 503 }],
            true,
            4,
            Backoff::Fixed { delay_ms: 750 },
        )
        .unwrap();
        (rate_limit, retry)
    }

    #[test]
    fn absent_metadata_adds_no_extension_members() {
        let doc = serde_json::to_value(SchematicDocExtension::default()).unwrap();
        assert!(doc.get("rate_limit").is_none() && doc.get("retry_guidance").is_none());
        let op = serde_json::to_value(SchematicOpExtension {
            rate_limit: None, retry_guidance: None, request: None, response: ApiResponse::Empty, headers: vec![],
            success: vec![], errors: vec![], no_declared_errors: false,
        }).unwrap();
        assert_eq!(op, serde_json::json!({"response": "Empty"}));
    }

    #[test]
    fn metadata_keeps_every_field_through_both_extensions() {
        let (rate_limit, retry) = metadata();
        let expected_rate = serde_json::json!({
            "limit_header": "x-limit", "remaining_header": "x-remaining", "reset_header": "x-reset",
            "retry_after_header": "retry-after", "reset_unit": "http_date",
        });
        let expected_retry = serde_json::json!({
            "retry_on": [{"code": 429}, {"range": {"start": 500, "end": 503}}],
            "honor_retry_after": true, "max_attempts": 4, "backoff": {"fixed": {"delay_ms": 750}},
        });

        let doc = SchematicDocExtension {
            rate_limit: Some(rate_limit.clone()), retry_guidance: Some(retry.clone()), ..Default::default()
        };
        let doc_json: serde_json::Value = doc.clone().into();
        assert_eq!(doc_json["rate_limit"], expected_rate);
        assert_eq!(doc_json["retry_guidance"], expected_retry);
        assert_eq!(serde_json::from_value::<SchematicDocExtension>(doc_json).unwrap(), doc);

        let op = SchematicOpExtension {
            rate_limit: Some(rate_limit), retry_guidance: Some(retry), request: None,
            response: ApiResponse::Empty, headers: vec![], success: vec![], errors: vec![], no_declared_errors: false,
        };
        let op_json: serde_json::Value = op.clone().into();
        assert_eq!(op_json["rate_limit"], expected_rate);
        assert_eq!(op_json["retry_guidance"], expected_retry);
        assert_eq!(serde_json::from_value::<SchematicOpExtension>(op_json).unwrap(), op);
    }

    #[test]
    fn response_vocabulary_round_trips_through_both_extensions() {
        use crate::error_table::StatusMatch;
        use crate::response::{JsonTag, ResponseShape, StreamEnd, StreamFraming};
        let mut tagged = SuccessShape::new(ResponseShape::Json(crate::TypeRef::Named("File".into())));
        tagged.json_tag = Some(JsonTag::Field { name: "type".into(), value: serde_json::json!("file") });
        tagged.variant = Some("File".into());
        tagged.statuses = vec![200];
        let mut fallback = SuccessShape::new(ResponseShape::Binary);
        fallback.fallback = true;
        fallback.note = Some("anything else".into());
        let stream = SuccessShape::new(ResponseShape::Stream {
            framing: StreamFraming::Ndjson(crate::TypeRef::Named("Chunk".into())),
            end: StreamEnd::FieldEquals { field: "done".into(), value: serde_json::json!(true) },
            keep: true,
        });
        let errors = vec![ErrorEntry { content_type: Some("text/html".into()), ..ErrorEntry::new(StatusMatch::exact(404), ResponseShape::Text) }];
        let op = SchematicOpExtension {
            rate_limit: None, retry_guidance: None, request: None, response: ApiResponse::Empty, headers: vec![],
            success: vec![tagged, fallback, stream], errors: errors.clone(), no_declared_errors: false,
        };
        let doc = SchematicDocExtension { errors, ..Default::default() };

        let op_json: serde_json::Value = op.clone().into();
        assert_eq!(op_json["success"][2]["shape"]["Stream"], serde_json::json!({"framing": {"Ndjson": {"Named": "Chunk"}}, "end": {"FieldEquals": {"field": "done", "value": true}}, "keep": true}));
        assert_eq!(serde_json::from_value::<SchematicOpExtension>(op_json).unwrap(), op);
        assert_eq!(serde_json::from_value::<SchematicDocExtension>(doc.clone().into()).unwrap(), doc);
    }
}
