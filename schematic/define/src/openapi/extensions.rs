//! The `x-schematic` vendor extensions an exported OpenAPI document carries.
//!
//! The structs here describe what the exporter writes: Schematic metadata
//! that standard OpenAPI cannot express, at the document root
//! ([`SchematicDocExtension`]) and on each operation
//! ([`SchematicOpExtension`]). Standard fields stay authoritative; the
//! extension never repeats the auth method, credential location or name,
//! scopes, deprecation, or a JSON/text/binary request body.
//!
//! The importer reads the same members back, each decoded and validated on
//! its own (see `import/extension.rs`), so one malformed member never
//! discards the rest. Only the subset documented in
//! `schematic/docs/io/openapi-extensions.md` round-trips; pagination, GraphQL,
//! OAuth2 details beyond the standard flows, and schema Rust paths do not.

use indexmap::IndexMap;
use serde::Serialize;

use crate::cross_origin::CrossOrigin;
use crate::error_table::ErrorEntry;
use crate::headers::EnvMapping;
use crate::metadata::{RateLimit, RetryGuidance};
use crate::request::ApiRequest;
use crate::response::{ResponseShape, SuccessShape};

/// The `x-schematic.version` this crate writes and the only major it reads.
pub const EXTENSION_VERSION: u32 = 1;

/// Document-level vendor extension for Schematic metadata.
///
/// Serialized to `x-schematic` at the OpenAPI document root. Every optional
/// member is omitted when unset; `env_mapping`'s four legacy members are the
/// only nulls it writes.
///
/// ## Examples
///
/// ```
/// use schematic_define::openapi::SchematicDocExtension;
///
/// let ext = SchematicDocExtension {
///     module_path: Some("openai".to_string()),
///     ..Default::default()
/// };
///
/// let json: serde_json::Value = ext.into();
/// assert_eq!(json, serde_json::json!({"version": 1, "module_path": "openai"}));
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SchematicDocExtension {
    /// The extension's major version, always [`EXTENSION_VERSION`].
    pub version: u32,

    /// Custom module path for generated code.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub module_path: Option<String>,

    /// Custom suffix for generated request structs.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub request_suffix: Option<String>,

    /// Environment variable mapping for authentication.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub env_mapping: Option<EnvMapping>,

    /// Environment variable names for token and key credentials (names, not
    /// credentials).
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub env_auth: Vec<String>,

    /// Default headers for all endpoints, as `[name, value]` pairs.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub headers: Vec<(String, String)>,

    /// Auth details `securitySchemes` cannot express, keyed by the component
    /// name of a scheme the root `security` applies.
    #[serde(skip_serializing_if = "IndexMap::is_empty")]
    pub auth: IndexMap<String, AuthDetail>,

    /// The API's rate-limit headers; descriptive only.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rate_limit: Option<RateLimit>,

    /// The API's retry guidance; descriptive only.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub retry_guidance: Option<RetryGuidance>,

    /// The API's default error entries, as declared (R26).
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub errors: Vec<ErrorEntry>,

    /// The API's cross-origin policy; absent means the same-origin default.
    #[serde(skip_serializing_if = "is_same_origin")]
    pub cross_origin: CrossOrigin,

    /// The response header carrying the provider's request id.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub request_id_header: Option<String>,

    /// Extra header and query names whose values are never shown.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub secret_names: Vec<String>,
}

impl Default for SchematicDocExtension {
    fn default() -> Self {
        Self {
            version: EXTENSION_VERSION,
            module_path: None,
            request_suffix: None,
            env_mapping: None,
            env_auth: vec![],
            headers: vec![],
            auth: IndexMap::new(),
            rate_limit: None,
            retry_guidance: None,
            errors: vec![],
            cross_origin: CrossOrigin::SameOrigin,
            request_id_header: None,
            secret_names: vec![],
        }
    }
}

fn is_same_origin(policy: &CrossOrigin) -> bool {
    *policy == CrossOrigin::SameOrigin
}

impl From<SchematicDocExtension> for serde_json::Value {
    fn from(ext: SchematicDocExtension) -> Self {
        serde_json::to_value(ext).expect("x-schematic extension is always serializable")
    }
}

/// One auth detail OpenAPI cannot express (R54): a bearer token sent in a
/// header other than `Authorization`, or an API-key value prefix.
///
/// The scheme it names is an `apiKey` scheme in a header; the header name
/// stays in the scheme's standard `name`. Never holds a credential.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct AuthDetail {
    /// The `apiKey` header scheme carries a bearer token.
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub bearer: bool,

    /// Prepended to the API key before it is sent (Gitea: `"token "`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value_prefix: Option<String>,
}

/// Parameter metadata OpenAPI cannot express, keyed under
/// `params.<location>.<wire name>`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ParamDetail {
    /// The value is a secret and is never shown.
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub secret: bool,

    /// The generated Rust field name, when it differs from the wire name.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rust_name: Option<String>,
}

/// Operation-level vendor extension for Schematic metadata.
///
/// Serialized to `x-schematic` on each OpenAPI operation. Members the
/// standard operation already states are omitted; `response` is always
/// written for compatibility with earlier artifacts.
///
/// ## Examples
///
/// ```
/// use schematic_define::openapi::SchematicOpExtension;
/// use schematic_define::{ResponseShape, TypeRef};
///
/// let ext = SchematicOpExtension {
///     headers: vec![("X-Custom".to_string(), "value".to_string())],
///     ..SchematicOpExtension::new(ResponseShape::Json(TypeRef::Named("User".into())))
/// };
///
/// let json: serde_json::Value = ext.into();
/// assert_eq!(json["headers"], serde_json::json!([["X-Custom", "value"]]));
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SchematicOpExtension {
    /// A form body's field details (multipart or URL-encoded); JSON, text,
    /// and binary bodies are carried by `requestBody` alone.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub request: Option<ApiRequest>,

    /// The first success shape's kind, in its historical serialized form
    /// (`{"Json": …}`, `"Text"`, `"Binary"`, `"Empty"`, or a stream).
    pub response: ResponseShape,

    /// Endpoint-specific headers, as `[name, value]` pairs.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub headers: Vec<(String, String)>,

    /// The endpoint's own rate-limit headers, which replace the API's whole;
    /// absent when the endpoint inherits.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rate_limit: Option<RateLimit>,

    /// The endpoint's own retry guidance, which replaces the API's whole;
    /// absent when the endpoint inherits.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub retry_guidance: Option<RetryGuidance>,

    /// Every success shape, written only when `response` cannot carry them:
    /// several shapes, a stream, or any selection fact, variant name, or note
    /// (R26).
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub success: Vec<SuccessShape>,

    /// The endpoint's own error entries, which overlay the API's `errors`.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub errors: Vec<ErrorEntry>,

    /// The endpoint opts out of the API's default errors.
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub no_declared_errors: bool,

    /// The endpoint's own cross-origin policy, which replaces the API's;
    /// absent when the endpoint inherits.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cross_origin: Option<CrossOrigin>,

    /// Auth details of the schemes this operation's own `security` names.
    /// Document-level details never apply to an operation override.
    #[serde(skip_serializing_if = "IndexMap::is_empty")]
    pub auth: IndexMap<String, AuthDetail>,

    /// The environment mapping of an endpoint auth policy override.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub env_mapping: Option<EnvMapping>,

    /// The endpoint's path when it holds reserved-expansion markers
    /// (`{+name}`), which the OpenAPI path key cannot.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path_template: Option<String>,

    /// Parameter details keyed by location (`query`, `header`, `cookie`,
    /// `path`) and then wire name.
    #[serde(skip_serializing_if = "IndexMap::is_empty")]
    pub params: IndexMap<String, IndexMap<String, ParamDetail>>,
}

impl SchematicOpExtension {
    /// An extension holding only `response`.
    pub fn new(response: ResponseShape) -> Self {
        Self {
            request: None,
            response,
            headers: vec![],
            rate_limit: None,
            retry_guidance: None,
            success: vec![],
            errors: vec![],
            no_declared_errors: false,
            cross_origin: None,
            auth: IndexMap::new(),
            env_mapping: None,
            path_template: None,
            params: IndexMap::new(),
        }
    }
}

impl From<SchematicOpExtension> for serde_json::Value {
    fn from(ext: SchematicOpExtension) -> Self {
        serde_json::to_value(ext).expect("x-schematic extension is always serializable")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::headers::{EnvList, EnvMapping};
    use crate::request::FormField;
    use serde_json::json;

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
    fn defaults_write_only_the_version_and_response() {
        assert_eq!(serde_json::Value::from(SchematicDocExtension::default()), json!({"version": 1}));
        assert_eq!(serde_json::Value::from(SchematicOpExtension::new(ResponseShape::Empty)), json!({"response": "Empty"}));
    }

    #[test]
    fn env_mapping_writes_its_four_legacy_members_as_null() {
        let doc = SchematicDocExtension {
            env_mapping: Some(EnvMapping { bearer_token: Some(EnvList::single("TOKEN")), ..Default::default() }),
            ..Default::default()
        };
        assert_eq!(serde_json::Value::from(doc)["env_mapping"], json!({
            "bearer_token": {"names": ["TOKEN"]}, "basic_user": null, "basic_pass": null, "api_key": null,
        }));
    }

    #[test]
    fn auth_details_and_param_details_omit_unset_members() {
        let doc = SchematicDocExtension {
            env_auth: vec!["GITEA_TOKEN".into()],
            auth: IndexMap::from([
                ("apiKeyAuth".to_string(), AuthDetail { bearer: false, value_prefix: Some("token ".into()) }),
                ("bearerAuth".to_string(), AuthDetail { bearer: true, value_prefix: None }),
            ]),
            secret_names: vec!["x-session".into()],
            ..Default::default()
        };
        let json = serde_json::Value::from(doc);
        assert_eq!(json["auth"], json!({"apiKeyAuth": {"value_prefix": "token "}, "bearerAuth": {"bearer": true}}));
        assert_eq!(json["env_auth"], json!(["GITEA_TOKEN"]));
        assert_eq!(json["secret_names"], json!(["x-session"]));

        let op = SchematicOpExtension {
            path_template: Some("/repos/{+path}".into()),
            params: IndexMap::from([("query".to_string(), IndexMap::from([
                ("api-key".to_string(), ParamDetail { secret: true, rust_name: None }),
                ("type".to_string(), ParamDetail { secret: false, rust_name: Some("kind".into()) }),
            ]))]),
            ..SchematicOpExtension::new(ResponseShape::Empty)
        };
        let json = serde_json::Value::from(op);
        assert_eq!(json["params"], json!({"query": {"api-key": {"secret": true}, "type": {"rust_name": "kind"}}}));
        assert_eq!(json["path_template"], "/repos/{+path}");
    }

    #[test]
    fn form_requests_keep_their_writer_nulls() {
        let op = SchematicOpExtension {
            request: Some(ApiRequest::form_data(vec![FormField::files("samples")])),
            ..SchematicOpExtension::new(ResponseShape::Empty)
        };
        assert_eq!(serde_json::Value::from(op)["request"], json!({"FormData": {"fields": [{
            "name": "samples", "kind": {"Files": {"accept": [], "min": null, "max": null}}, "required": true, "description": null,
        }]}}));
    }

    #[test]
    fn metadata_keeps_every_field_in_both_extensions() {
        let (rate_limit, retry) = metadata();
        let expected_rate = json!({
            "limit_header": "x-limit", "remaining_header": "x-remaining", "reset_header": "x-reset",
            "retry_after_header": "retry-after", "reset_unit": "http_date",
        });
        let expected_retry = json!({
            "retry_on": [{"code": 429}, {"range": {"start": 500, "end": 503}}],
            "honor_retry_after": true, "max_attempts": 4, "backoff": {"fixed": {"delay_ms": 750}},
        });

        let doc = SchematicDocExtension {
            rate_limit: Some(rate_limit.clone()), retry_guidance: Some(retry.clone()), ..Default::default()
        };
        let doc_json: serde_json::Value = doc.into();
        assert_eq!(doc_json["rate_limit"], expected_rate);
        assert_eq!(doc_json["retry_guidance"], expected_retry);

        let op = SchematicOpExtension {
            rate_limit: Some(rate_limit), retry_guidance: Some(retry),
            ..SchematicOpExtension::new(ResponseShape::Empty)
        };
        let op_json: serde_json::Value = op.into();
        assert_eq!(op_json["rate_limit"], expected_rate);
        assert_eq!(op_json["retry_guidance"], expected_retry);
    }

    #[test]
    fn cross_origin_and_request_id_are_written_only_when_declared() {
        let default_doc = serde_json::Value::from(SchematicDocExtension::default());
        assert!(default_doc.get("cross_origin").is_none() && default_doc.get("request_id_header").is_none());

        let doc = SchematicDocExtension {
            cross_origin: CrossOrigin::allow_origins(["https://uploads.example.com"]),
            request_id_header: Some("x-request-id".into()),
            ..Default::default()
        };
        let doc_json: serde_json::Value = doc.into();
        assert_eq!(doc_json["cross_origin"], json!({"AllowOrigins": ["https://uploads.example.com"]}));
        assert_eq!(doc_json["request_id_header"], "x-request-id");

        let op = SchematicOpExtension { cross_origin: Some(CrossOrigin::Deny), ..SchematicOpExtension::new(ResponseShape::Empty) };
        assert_eq!(serde_json::Value::from(op)["cross_origin"], "Deny");
    }
}
