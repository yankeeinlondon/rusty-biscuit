//! Maps fixed headers ([`RestApi::headers`], [`Endpoint::headers`]) onto
//! OpenAPI header parameters.
//!
//! See the [OpenAPI parameter contract](https://spec.openapis.org/oas/v3.0.3#parameter-object):
//! a header parameter named `Accept`, `Content-Type`, or `Authorization` is
//! ignored, because content and security describe those headers.

use indexmap::IndexMap;
use openapiv3::{
    HeaderStyle, Operation, Parameter, ParameterData, ParameterSchemaOrContent, ReferenceOr,
    Schema, SchemaKind, SecurityRequirement, SecurityScheme, StatusCode, StringType, Type,
};

use crate::types::{Endpoint, RestApi};

/// A header an exported operation does not carry as a header parameter.
///
/// The fixed value stays in the `x-schematic` extension's `headers`; the
/// generator reports each gap as a `fixed-header` warning.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HeaderGap {
    /// The endpoint whose operation lacks the parameter.
    pub endpoint_id: String,
    /// The header name as declared.
    pub name: String,
    /// The fixed value; `None` for an explicit header parameter.
    pub value: Option<String>,
    /// Why the operation has no parameter for it.
    pub reason: HeaderGapReason,
}

/// Why a [`HeaderGap`] has no header parameter.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HeaderGapReason {
    /// The operation's effective security sends this header through the named
    /// `securitySchemes` entry.
    Credential {
        /// The `components.securitySchemes` key.
        scheme: String,
    },
    /// A fixed `Accept` that names no success response media type, or an
    /// explicit `Accept` parameter.
    Accept,
    /// A fixed `Content-Type` that is not the request body's media type, or an
    /// explicit `Content-Type` parameter.
    ContentType,
    /// An `Authorization` header no security scheme of the operation carries.
    Authorization,
    /// An explicit header parameter of the same name whose schema rejects the
    /// fixed value; the explicit parameter is exported unchanged.
    IncompatibleParameter,
}

impl std::fmt::Display for HeaderGap {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let subject = match &self.value {
            Some(value) => format!("fixed header `{}: {value}`", self.name),
            None => format!("header parameter `{}`", self.name),
        };
        let consequence = match &self.reason {
            HeaderGapReason::Credential { scheme } => {
                format!("is the credential header of security scheme `{scheme}`, so it is not a parameter")
            }
            HeaderGapReason::Accept => {
                "is not a parameter (OpenAPI ignores `Accept`) and no success response declares that media type".to_string()
            }
            HeaderGapReason::ContentType => {
                "is not a parameter (OpenAPI ignores `Content-Type`) and the request body does not declare that media type".to_string()
            }
            HeaderGapReason::Authorization => {
                "is not a parameter (OpenAPI ignores `Authorization`) and no security scheme of the operation sends it".to_string()
            }
            HeaderGapReason::IncompatibleParameter => {
                "conflicts with the endpoint's header parameter of the same name, whose schema rejects the value; the declared parameter is exported unchanged".to_string()
            }
        };
        write!(f, "{subject} {consequence}")
    }
}

/// Header names (lower case) the operation's credentials occupy, each with
/// the scheme that sends it.
///
/// `security` is the operation's effective requirement list: its own, else
/// the document's. Any scheme named by any requirement counts.
pub(super) fn credential_headers(
    security: &[SecurityRequirement],
    schemes: &IndexMap<String, ReferenceOr<SecurityScheme>>,
) -> Vec<(String, String)> {
    let mut headers: Vec<(String, String)> = Vec::new();
    for scheme_name in security.iter().flat_map(IndexMap::keys) {
        let Some(ReferenceOr::Item(scheme)) = schemes.get(scheme_name) else {
            continue;
        };
        let header = match scheme {
            SecurityScheme::APIKey { location: openapiv3::APIKeyLocation::Header, name, .. } => name.to_ascii_lowercase(),
            SecurityScheme::APIKey { .. } => continue,
            SecurityScheme::HTTP { .. } | SecurityScheme::OAuth2 { .. } | SecurityScheme::OpenIDConnect { .. } => {
                "authorization".to_string()
            }
        };
        if !headers.iter().any(|(existing, _)| *existing == header) {
            headers.push((header, scheme_name.clone()));
        }
    }
    headers
}

/// Adds `endpoint`'s effective fixed headers to `operation` as required
/// single-value header parameters, and removes explicit header parameters
/// OpenAPI cannot carry; returns what was left out.
///
/// The effective set is the API's headers overlaid by the endpoint's, names
/// compared case-insensitively, the endpoint winning (as the runtime sends
/// them). A fixed header that matches an explicit header parameter whose
/// schema accepts the value merges into it: the parameter becomes required
/// with the value as its default.
pub(super) fn apply_fixed_headers(
    api: &RestApi,
    endpoint: &Endpoint,
    operation: &mut Operation,
    credentials: &[(String, String)],
) -> Vec<HeaderGap> {
    let mut gaps = Vec::new();
    let gap = |name: &str, value: Option<&str>, reason: HeaderGapReason| HeaderGap {
        endpoint_id: endpoint.id.clone(),
        name: name.to_string(),
        value: value.map(str::to_string),
        reason,
    };

    let mut fixed: IndexMap<String, (&str, &str)> = IndexMap::new();
    for (name, value) in api.headers.iter().chain(&endpoint.headers) {
        fixed.insert(name.to_ascii_lowercase(), (name.as_str(), value.as_str()));
    }

    // Explicit parameters are checked first so a merged parameter is judged
    // once, by its declared name.
    operation.parameters.retain(|parameter| {
        let ReferenceOr::Item(Parameter::Header { parameter_data, .. }) = parameter else {
            return true;
        };
        match unexpressible(&parameter_data.name.to_ascii_lowercase(), credentials) {
            Some(reason) => {
                gaps.push(gap(&parameter_data.name, None, reason));
                false
            }
            None => true,
        }
    });

    for (lower, (name, value)) in fixed {
        if let Some(reason) = unexpressible(&lower, credentials) {
            let expressed = match reason {
                HeaderGapReason::Accept => success_media_types(operation).any(|media| media.eq_ignore_ascii_case(value)),
                HeaderGapReason::ContentType => request_media_types(operation).any(|media| media.eq_ignore_ascii_case(value)),
                _ => false,
            };
            if !expressed {
                gaps.push(gap(name, Some(value), reason));
            }
            continue;
        }

        let explicit = operation.parameters.iter_mut().find_map(|parameter| match parameter {
            ReferenceOr::Item(Parameter::Header { parameter_data, .. })
                if parameter_data.name.eq_ignore_ascii_case(name) => Some(parameter_data),
            _ => None,
        });
        match explicit {
            Some(parameter_data) => match &mut parameter_data.format {
                ParameterSchemaOrContent::Schema(ReferenceOr::Item(schema)) if accepts(schema, value) => {
                    parameter_data.required = true;
                    schema.schema_data.default = Some(serde_json::Value::String(value.to_string()));
                }
                _ => gaps.push(gap(name, Some(value), HeaderGapReason::IncompatibleParameter)),
            },
            None => operation.parameters.push(ReferenceOr::Item(fixed_parameter(name, value))),
        }
    }
    gaps
}

/// Why a header of this lower-case name cannot be a parameter, if it cannot.
fn unexpressible(lower: &str, credentials: &[(String, String)]) -> Option<HeaderGapReason> {
    if let Some((_, scheme)) = credentials.iter().find(|(header, _)| header == lower) {
        return Some(HeaderGapReason::Credential { scheme: scheme.clone() });
    }
    match lower {
        "accept" => Some(HeaderGapReason::Accept),
        "content-type" => Some(HeaderGapReason::ContentType),
        "authorization" => Some(HeaderGapReason::Authorization),
        _ => None,
    }
}

/// Whether an explicit parameter's schema admits `value`: an unconstrained
/// string, or a string enum listing it.
fn accepts(schema: &Schema, value: &str) -> bool {
    match &schema.schema_kind {
        SchemaKind::Type(Type::String(string)) => {
            string.enumeration.is_empty() || string.enumeration.iter().any(|allowed| allowed.as_deref() == Some(value))
        }
        _ => false,
    }
}

fn fixed_parameter(name: &str, value: &str) -> Parameter {
    let mut schema = Schema {
        schema_data: Default::default(),
        schema_kind: SchemaKind::Type(Type::String(StringType {
            enumeration: vec![Some(value.to_string())],
            ..Default::default()
        })),
    };
    schema.schema_data.default = Some(serde_json::Value::String(value.to_string()));
    Parameter::Header {
        parameter_data: ParameterData {
            name: name.to_string(),
            description: None,
            required: true,
            deprecated: None,
            format: ParameterSchemaOrContent::Schema(ReferenceOr::Item(schema)),
            example: None,
            examples: IndexMap::new(),
            explode: None,
            extensions: IndexMap::new(),
        },
        style: HeaderStyle::Simple,
    }
}

fn success_media_types(operation: &Operation) -> impl Iterator<Item = &str> {
    operation
        .responses
        .responses
        .iter()
        .filter(|(status, _)| match status {
            StatusCode::Code(code) => (200..300).contains(code),
            StatusCode::Range(class) => *class == 2,
        })
        .filter_map(|(_, response)| response.as_item())
        .flat_map(|response| response.content.keys())
        .map(String::as_str)
}

fn request_media_types(operation: &Operation) -> impl Iterator<Item = &str> {
    operation
        .request_body
        .iter()
        .filter_map(ReferenceOr::as_item)
        .flat_map(|body| body.content.keys())
        .map(String::as_str)
}
