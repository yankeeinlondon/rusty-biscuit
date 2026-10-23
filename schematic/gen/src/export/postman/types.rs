//! Postman Collection v2.1.0 type definitions.

use serde::Serialize;

/// Postman Collection v2.1.0 format.
#[derive(Debug, Clone, Serialize)]
pub struct PostmanCollection {
    /// Collection metadata.
    pub info: PostmanInfo,
    /// Collection-level variables.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub variable: Vec<PostmanVariable>,
    /// Collection-level authentication.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub auth: Option<PostmanAuth>,
    /// Collection items (folders and requests).
    pub item: Vec<PostmanItem>,
}

/// Collection information metadata.
#[derive(Debug, Clone, Serialize)]
pub struct PostmanInfo {
    /// Collection name.
    pub name: String,
    /// Collection description.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Postman schema version URL.
    pub schema: String,
}

/// Collection item (folder or request).
#[derive(Debug, Clone, Serialize)]
#[serde(untagged)]
pub enum PostmanItem {
    /// A folder containing nested items.
    Folder {
        /// Folder name.
        name: String,
        /// Nested items.
        item: Vec<PostmanItem>,
    },
    /// A single request.
    Request {
        /// Request name.
        name: String,
        /// Request details.
        request: Box<PostmanRequest>,
        /// Saved example responses: one per success shape, then one per
        /// declared error entry.
        #[serde(skip_serializing_if = "Vec::is_empty")]
        response: Vec<PostmanResponse>,
    },
}

/// A saved example response.
#[derive(Debug, Clone, Serialize)]
pub struct PostmanResponse {
    /// Display name: the status, its reason, and the media type.
    pub name: String,
    /// The status's reason phrase.
    pub status: String,
    /// The HTTP status code.
    pub code: u16,
    /// Response headers; a body-bearing response declares its
    /// `Content-Type`.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub header: Vec<PostmanHeader>,
    /// The example body; absent for a response without one.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub body: Option<String>,
}

/// A Postman environment: the variables a collection uses, with values.
///
/// Serialized as Postman's environment export format.
#[derive(Debug, Clone, Serialize)]
pub struct PostmanEnvironment {
    /// Environment name.
    pub name: String,
    /// One entry per collection variable, in collection order.
    pub values: Vec<PostmanEnvironmentValue>,
    /// Always `environment`.
    #[serde(rename = "_postman_variable_scope")]
    pub scope: String,
}

/// One environment variable.
#[derive(Debug, Clone, Serialize)]
pub struct PostmanEnvironmentValue {
    /// Variable key, as the collection references it.
    pub key: String,
    /// Initial value; empty for credentials and parameters.
    pub value: String,
    /// `secret` for a credential or secret parameter, otherwise `default`.
    #[serde(rename = "type")]
    pub type_field: String,
    /// Always `true`.
    pub enabled: bool,
}

/// HTTP request details.
#[derive(Debug, Clone, Serialize)]
pub struct PostmanRequest {
    /// HTTP method (GET, POST, etc.).
    pub method: String,
    /// Request URL.
    pub url: PostmanUrl,
    /// Request-level authentication override.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub auth: Option<PostmanAuth>,
    /// Request headers.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub header: Vec<PostmanHeader>,
    /// Request body.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub body: Option<PostmanBody>,
    /// Request description.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

/// URL components.
#[derive(Debug, Clone, Serialize)]
pub struct PostmanUrl {
    /// Full URL string.
    pub raw: String,
    /// Host parts (including {{baseUrl}} variable).
    pub host: Vec<String>,
    /// Path segments.
    pub path: Vec<String>,
    /// Query parameters.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub query: Vec<PostmanQuery>,
    /// Path variables.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub variable: Vec<PostmanVariable>,
}

/// Authentication configuration.
#[derive(Debug, Clone, Serialize)]
pub struct PostmanAuth {
    /// Authentication type.
    #[serde(rename = "type")]
    pub type_field: String,
    /// Bearer token configuration.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bearer: Option<Vec<PostmanVariable>>,
    /// API key configuration.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub apikey: Option<Vec<PostmanVariable>>,
    /// Basic auth configuration.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub basic: Option<Vec<PostmanVariable>>,
    /// OAuth2 configuration.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub oauth2: Option<Vec<PostmanVariable>>,
}

/// Variable definition.
#[derive(Debug, Clone, Serialize)]
pub struct PostmanVariable {
    /// Variable key.
    pub key: String,
    /// Variable value.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value: Option<String>,
    /// Variable description.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

/// HTTP header.
#[derive(Debug, Clone, Serialize)]
pub struct PostmanHeader {
    /// Header key.
    pub key: String,
    /// Header value.
    pub value: String,
    /// Whether Postman leaves the header out of the request until the user
    /// enables it.
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub disabled: bool,
    /// Header description.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

/// Request body.
#[derive(Debug, Clone, Serialize)]
pub struct PostmanBody {
    /// Body mode (raw, formdata, urlencoded, file).
    pub mode: String,
    /// Raw body content.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub raw: Option<String>,
    /// Form data fields.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub formdata: Option<Vec<PostmanFormParam>>,
    /// URL-encoded fields.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub urlencoded: Option<Vec<PostmanFormParam>>,
    /// Body options (for raw mode).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub options: Option<PostmanBodyOptions>,
}

/// Form parameter (for formdata or urlencoded).
#[derive(Debug, Clone, Serialize)]
pub struct PostmanFormParam {
    /// Field key.
    pub key: String,
    /// Field value.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value: Option<String>,
    /// Field description.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Field type (text or file).
    #[serde(rename = "type")]
    pub type_field: String,
}

/// Body options.
#[derive(Debug, Clone, Serialize)]
pub struct PostmanBodyOptions {
    /// Raw body options.
    pub raw: PostmanRawOptions,
}

/// Raw body language options.
#[derive(Debug, Clone, Serialize)]
pub struct PostmanRawOptions {
    /// Language for syntax highlighting.
    pub language: String,
}

/// Query parameter.
#[derive(Debug, Clone, Serialize)]
pub struct PostmanQuery {
    /// Query parameter key.
    pub key: String,
    /// Query parameter value.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value: Option<String>,
    /// Whether Postman leaves the parameter out of the URL until the user
    /// enables it.
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub disabled: bool,
    /// Query parameter description.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}
