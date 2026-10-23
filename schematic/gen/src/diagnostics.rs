//! Validation diagnostics shared by every generator output.
//!
//! A [`ValidationDiagnostic`] names the API (or every API of a module group),
//! the endpoint when one applies, the [`Feature`] at fault, and the [`Output`]
//! it concerns. [`Severity::Error`] blocks that output; [`Severity::Warning`]
//! is reported and never blocks. This is the one diagnostic format for
//! generation: WebSocket lowering maps onto the same types
//! ([`Feature::WebSocketDefinition`], [`Output::Rust`]) rather than inventing
//! another.
//!
//! See `schematic/docs/generation/validation.md` for the rules each output
//! applies.
//!
//! ## Examples
//!
//! ```
//! use schematic_gen::diagnostics::{Feature, Output, Severity, ValidationDiagnostic};
//!
//! let diagnostic = ValidationDiagnostic::new(
//!     Severity::Error,
//!     "FixtureApi",
//!     Some("Tree"),
//!     Feature::QueryInPath,
//!     Output::OpenApi,
//!     "path template embeds a query string",
//! );
//!
//! assert_eq!(
//!     diagnostic.to_string(),
//!     "severity=error api=FixtureApi endpoint=Tree feature=query-in-path \
//!      output=openapi: path template embeds a query string"
//! );
//! ```

use std::fmt;

use serde::Serialize;

/// Whether a diagnostic blocks its output.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Severity {
    /// Reported; the output is still written.
    Warning,
    /// The output is not written.
    Error,
}

impl Severity {
    /// The lowercase name used in rendered and serialized diagnostics.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Warning => "warning",
            Self::Error => "error",
        }
    }
}

impl fmt::Display for Severity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// A generated artifact that is validated and published on its own.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Output {
    /// The generated Rust client crate.
    Rust,
    /// OpenAPI documents.
    #[serde(rename = "openapi")]
    OpenApi,
    /// Postman collections.
    Postman,
    /// AsyncAPI documents of WebSocket APIs.
    #[serde(rename = "asyncapi")]
    AsyncApi,
}

impl Output {
    /// The lowercase name used in rendered and serialized diagnostics.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Rust => "rust",
            Self::OpenApi => "openapi",
            Self::Postman => "postman",
            Self::AsyncApi => "asyncapi",
        }
    }
}

impl fmt::Display for Output {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// The definition feature or rule a diagnostic concerns.
///
/// Rendered and serialized as a stable kebab-case id.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Feature {
    /// `ApiRequest::Text` request bodies.
    TextRequestBody,
    /// `ApiRequest::Binary` request bodies.
    BinaryRequestBody,
    /// An API key sent as a query parameter or cookie.
    ApiKeyParam,
    /// `EndpointParams.query` entries.
    QueryParameter,
    /// `EndpointParams.header` entries.
    HeaderParameter,
    /// `EndpointParams.cookie` entries.
    CookieParameter,
    /// A fixed or explicit header an OpenAPI operation does not carry as a
    /// header parameter (a credential, `Accept`, `Content-Type`, or
    /// `Authorization` header, or a conflicting declaration).
    FixedHeader,
    /// Two endpoints of one API share an id.
    DuplicateEndpointId,
    /// Two endpoints of one API share a method and normalized path.
    DuplicateMethodPath,
    /// A type name that is not a Rust type.
    InvalidTypeSyntax,
    /// Two operations of one OpenAPI document share an `operationId`.
    DuplicateOperationId,
    /// An OpenAPI link names an `operationId` that no operation of its
    /// document declares.
    UnresolvedLinkOperation,
    /// A path template that embeds a query string (`?`).
    QueryInPath,
    /// A name that cannot become the Rust identifier the emitters build from it.
    InvalidIdentifier,
    /// A `request_suffix` that is empty or not alphanumeric.
    InvalidRequestSuffix,
    /// Two generated Rust items, fields, or methods would share a name.
    GeneratedNameCollision,
    /// An endpoint method or call-type setter was given a name other than the
    /// one its rule gives, to avoid a method every API struct or call type
    /// has.
    EndpointMethodRename,
    /// A module path the generated crate cannot declare.
    InvalidModulePath,
    /// An output file name that is not one safe path component.
    InvalidOutputFilename,
    /// A JSON body reference that does not resolve in the API's own
    /// [`RestApi::schemas`](schematic_define::RestApi::schemas), or a
    /// malformed schema component.
    InvalidSchemaReference,
    /// The shape of a pagination strategy, alone or on its endpoint.
    PaginationStrategy,
    /// An endpoint collection: invalid item selectors or item types.
    /// Retains the existing serialized diagnostic category.
    PaginationException,
    /// A pagination strategy input against the endpoint's declared parameters.
    PaginationParameter,
    /// An endpoint's own authentication (public, or a replacement policy).
    EndpointAuth,
    /// An OAuth2 grant an output has no form for.
    #[serde(rename = "oauth2-grant")]
    OAuth2Grant,
    /// A typed path parameter (`EndpointParams.path`).
    PathParameter,
    /// A GraphQL operation an output does not carry, or cannot address.
    #[serde(rename = "graphql-operation")]
    GraphQlOperation,
    /// A request body, saved response, or GraphQL `variables` for which no
    /// value validating against its registered schema could be built.
    ExampleValue,
    /// A WebSocket API definition, found while lowering it into its
    /// generated module. The message starts with the definition path.
    #[serde(rename = "websocket-definition")]
    WebSocketDefinition,
    /// A value constraint on a parameter, form field, or request body root
    /// that contradicts itself or its value's type.
    TypeConstraint,
}

impl Feature {
    /// The kebab-case id used in rendered and serialized diagnostics.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::TextRequestBody => "text-request-body",
            Self::BinaryRequestBody => "binary-request-body",
            Self::ApiKeyParam => "api-key-param",
            Self::QueryParameter => "query-parameter",
            Self::HeaderParameter => "header-parameter",
            Self::CookieParameter => "cookie-parameter",
            Self::FixedHeader => "fixed-header",
            Self::DuplicateEndpointId => "duplicate-endpoint-id",
            Self::DuplicateMethodPath => "duplicate-method-path",
            Self::InvalidTypeSyntax => "invalid-type-syntax",
            Self::DuplicateOperationId => "duplicate-operation-id",
            Self::UnresolvedLinkOperation => "unresolved-link-operation",
            Self::QueryInPath => "query-in-path",
            Self::InvalidIdentifier => "invalid-identifier",
            Self::InvalidRequestSuffix => "invalid-request-suffix",
            Self::GeneratedNameCollision => "generated-name-collision",
            Self::EndpointMethodRename => "endpoint-method-rename",
            Self::InvalidModulePath => "invalid-module-path",
            Self::InvalidOutputFilename => "invalid-output-filename",
            Self::InvalidSchemaReference => "invalid-schema-reference",
            Self::PaginationStrategy => "pagination-strategy",
            Self::PaginationException => "pagination-exception",
            Self::PaginationParameter => "pagination-parameter",
            Self::EndpointAuth => "endpoint-auth",
            Self::OAuth2Grant => "oauth2-grant",
            Self::PathParameter => "path-parameter",
            Self::GraphQlOperation => "graphql-operation",
            Self::ExampleValue => "example-value",
            Self::WebSocketDefinition => "websocket-definition",
            Self::TypeConstraint => "type-constraint",
        }
    }
}

impl fmt::Display for Feature {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// One validation finding for one output.
///
/// `api` holds every contributing API: one name for an API-level finding,
/// all members for a module-group collision. `endpoint` is `None` for an
/// API- or group-level finding and renders as `endpoint=<none>`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ValidationDiagnostic {
    pub severity: Severity,
    pub api: Vec<String>,
    pub endpoint: Option<String>,
    pub feature: Feature,
    pub output: Output,
    pub message: String,
}

impl ValidationDiagnostic {
    /// Builds a diagnostic for one API.
    pub fn new(
        severity: Severity,
        api: impl Into<String>,
        endpoint: Option<&str>,
        feature: Feature,
        output: Output,
        message: impl Into<String>,
    ) -> Self {
        Self {
            severity,
            api: vec![api.into()],
            endpoint: endpoint.map(str::to_string),
            feature,
            output,
            message: message.into(),
        }
    }

    /// Builds a diagnostic naming every API of a module group.
    pub fn for_group<I, S>(
        severity: Severity,
        apis: I,
        feature: Feature,
        output: Output,
        message: impl Into<String>,
    ) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        Self {
            severity,
            api: apis.into_iter().map(Into::into).collect(),
            endpoint: None,
            feature,
            output,
            message: message.into(),
        }
    }

    /// Whether this diagnostic blocks its output.
    pub fn is_error(&self) -> bool {
        self.severity == Severity::Error
    }
}

impl fmt::Display for ValidationDiagnostic {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "severity={} api={} endpoint={} feature={} output={}: {}",
            self.severity,
            self.api.join(","),
            self.endpoint.as_deref().unwrap_or("<none>"),
            self.feature,
            self.output,
            self.message
        )
    }
}

/// Whether any diagnostic in `diagnostics` blocks its output.
pub fn has_errors(diagnostics: &[ValidationDiagnostic]) -> bool {
    diagnostics.iter().any(ValidationDiagnostic::is_error)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn endpoint_diagnostic_renders_every_scope_field() {
        let diagnostic = ValidationDiagnostic::new(
            Severity::Error,
            "Foo",
            Some("UploadFile"),
            Feature::BinaryRequestBody,
            Output::OpenApi,
            "not supported",
        );

        assert_eq!(
            diagnostic.to_string(),
            "severity=error api=Foo endpoint=UploadFile feature=binary-request-body \
             output=openapi: not supported"
        );
    }

    #[test]
    fn group_diagnostic_names_every_api_and_no_endpoint() {
        let diagnostic = ValidationDiagnostic::for_group(
            Severity::Error,
            ["OllamaNative", "OllamaOpenAI"],
            Feature::GeneratedNameCollision,
            Output::Rust,
            "`ChatRequest` is emitted twice",
        );

        assert_eq!(
            diagnostic.to_string(),
            "severity=error api=OllamaNative,OllamaOpenAI endpoint=<none> \
             feature=generated-name-collision output=rust: `ChatRequest` is emitted twice"
        );
    }

    #[test]
    fn serialized_form_carries_the_severity_and_kebab_case_ids() {
        let warning = ValidationDiagnostic::new(
            Severity::Warning,
            "Gitea",
            Some("GetGitTreeRecursive"),
            Feature::QueryInPath,
            Output::Rust,
            "embedded query",
        );
        let error = ValidationDiagnostic::for_group(
            Severity::Error,
            ["A"],
            Feature::InvalidSchemaReference,
            Output::Postman,
            "missing",
        );

        assert_eq!(
            serde_json::to_value(&warning).unwrap(),
            serde_json::json!({
                "severity": "warning",
                "api": ["Gitea"],
                "endpoint": "GetGitTreeRecursive",
                "feature": "query-in-path",
                "output": "rust",
                "message": "embedded query",
            })
        );
        let error_json = serde_json::to_value(&error).unwrap();
        assert_eq!(error_json["severity"], "error");
        assert_eq!(error_json["endpoint"], serde_json::Value::Null);
        assert_eq!(error_json["feature"], "invalid-schema-reference");
        assert_eq!(error_json["output"], "postman");
    }

    #[test]
    fn display_and_serialized_ids_agree_for_every_variant() {
        let features = [
            Feature::TextRequestBody,
            Feature::BinaryRequestBody,
            Feature::ApiKeyParam,
            Feature::QueryParameter,
            Feature::HeaderParameter,
            Feature::CookieParameter,
            Feature::DuplicateEndpointId,
            Feature::DuplicateMethodPath,
            Feature::InvalidTypeSyntax,
            Feature::DuplicateOperationId,
            Feature::UnresolvedLinkOperation,
            Feature::QueryInPath,
            Feature::InvalidIdentifier,
            Feature::InvalidRequestSuffix,
            Feature::GeneratedNameCollision,
            Feature::InvalidModulePath,
            Feature::InvalidOutputFilename,
            Feature::InvalidSchemaReference,
            Feature::GraphQlOperation,
            Feature::ExampleValue,
            Feature::WebSocketDefinition,
            Feature::TypeConstraint,
        ];
        for feature in features {
            assert_eq!(
                serde_json::to_value(feature).unwrap(),
                feature.to_string(),
                "{feature:?}"
            );
        }
        for output in [Output::Rust, Output::OpenApi, Output::Postman, Output::AsyncApi] {
            assert_eq!(serde_json::to_value(output).unwrap(), output.to_string());
        }
        for severity in [Severity::Warning, Severity::Error] {
            assert_eq!(
                serde_json::to_value(severity).unwrap(),
                severity.to_string()
            );
        }
    }

    #[test]
    fn only_errors_block() {
        let warning = ValidationDiagnostic::new(
            Severity::Warning,
            "A",
            None,
            Feature::QueryInPath,
            Output::Rust,
            "w",
        );
        let error = ValidationDiagnostic {
            severity: Severity::Error,
            ..warning.clone()
        };

        assert!(!has_errors(std::slice::from_ref(&warning)));
        assert!(has_errors(&[warning, error]));
    }
}
