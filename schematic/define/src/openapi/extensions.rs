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
use crate::request::ApiRequest;
use crate::response::ApiResponse;

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
///     oauth_scopes: None,
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

    /// OAuth2 scopes required for this operation.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub oauth_scopes: Option<Vec<String>>,
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

    /// Environment variable names for auth credentials (legacy format).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub env_auth: Vec<String>,

    /// Environment variable for Basic auth username.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub env_username: Option<String>,
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
        let ext = SchematicOpExtension {
            request: Some(ApiRequest::json_type("CreateBody")),
            response: ApiResponse::json_type("Response"),
            headers: vec![],
            oauth_scopes: None,
        };
        assert!(ext.request.is_some());
    }

    #[test]
    fn op_extension_with_form_data_request() {
        let ext = SchematicOpExtension {
            request: Some(ApiRequest::form_data(vec![
                FormField::file("document"),
                FormField::text("name").optional(),
            ])),
            response: ApiResponse::json_type("UploadResponse"),
            headers: vec![],
            oauth_scopes: None,
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
        let ext = SchematicOpExtension {
            request: None,
            response: ApiResponse::Binary,
            headers: vec![],
            oauth_scopes: None,
        };
        assert!(ext.response.is_binary());
    }

    #[test]
    fn op_extension_with_endpoint_headers() {
        let ext = SchematicOpExtension {
            request: None,
            response: ApiResponse::Empty,
            headers: vec![("X-Custom-Header".to_string(), "custom-value".to_string())],
            oauth_scopes: None,
        };
        assert_eq!(ext.headers.len(), 1);
    }

    #[test]
    fn op_extension_into_json() {
        let ext = SchematicOpExtension {
            request: Some(ApiRequest::json_type("TestBody")),
            response: ApiResponse::Text,
            headers: vec![],
            oauth_scopes: None,
        };

        let json: serde_json::Value = ext.into();
        assert!(json.is_object());
        assert!(json.get("request").is_some());
        assert!(json.get("response").is_some());
    }

    #[test]
    fn op_extension_serde_roundtrip() {
        let ext = SchematicOpExtension {
            request: Some(ApiRequest::form_data(vec![FormField::file("file")])),
            response: ApiResponse::json_type("Result"),
            headers: vec![("X-Header".to_string(), "value".to_string())],
            oauth_scopes: None,
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
            env_username: None,
        };

        assert!(matches!(ext.strategy, AuthStrategy::BearerToken { .. }));
        assert_eq!(ext.env_auth.len(), 1);
    }

    #[test]
    fn auth_extension_basic_auth() {
        let ext = AuthExtension {
            strategy: AuthStrategy::Basic,
            env_auth: vec!["API_PASSWORD".to_string()],
            env_username: Some("API_USERNAME".to_string()),
        };

        assert!(matches!(ext.strategy, AuthStrategy::Basic));
        assert!(ext.env_username.is_some());
    }

    #[test]
    fn auth_extension_api_key() {
        let ext = AuthExtension {
            strategy: AuthStrategy::ApiKey {
                header: "X-API-Key".to_string(),
                value_prefix: None,
            },
            env_auth: vec!["MY_API_KEY".to_string()],
            env_username: None,
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
            env_username: None,
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
            env_username: None,
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
        let ext = SchematicOpExtension {
            request: Some(ApiRequest::json_type("Body")),
            response: ApiResponse::json_type("Response"),
            headers: vec![],
            oauth_scopes: None,
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
            env_username: None,
        };
        let json = serde_json::to_string(&ext).unwrap();
        assert!(!json.contains("env_auth"));
        let parsed: AuthExtension = serde_json::from_str(&json).unwrap();
        assert_eq!(ext, parsed);
    }
}
