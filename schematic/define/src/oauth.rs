//! OAuth2 configuration types for REST APIs.
//!
//! This module defines declarative OAuth2 metadata for API definitions.
//! [`OAuth2Config`] selects one grant, and each grant payload carries only the
//! fields that grant uses, so an authorization-code grant cannot omit its
//! authorization endpoint and a client-credentials grant cannot carry one.
//! Actual token lifecycle management is handled by the `schematic-oauth`
//! runtime crate.
//!
//! The serialized form is a single object tagged by `grant_type` whose other
//! members are exactly the selected grant's fields. Deserialization rejects
//! unknown members, a missing or null required field, an explicit null
//! `revocation_url`, and any non-null value for a field another grant owns,
//! then runs [`OAuth2Config::validate`].
//!
//! A grant payload cannot omit a field its grant requires:
//!
//! ```compile_fail
//! use schematic_define::{DeviceCodeGrant, OAuth2ClientAuthMethod};
//! let _ = DeviceCodeGrant {
//!     token_url: "https://example.com/token".into(),
//!     revocation_url: None,
//!     default_scopes: vec![],
//!     client_auth: OAuth2ClientAuthMethod::None,
//! };
//! ```
//!
//! nor carry a field that belongs to another grant:
//!
//! ```compile_fail
//! use schematic_define::{ClientCredentialsGrant, OAuth2ClientAuthMethod};
//! let _ = ClientCredentialsGrant {
//!     authorization_url: "https://example.com/authorize".into(),
//!     token_url: "https://example.com/token".into(),
//!     revocation_url: None,
//!     default_scopes: vec![],
//!     client_auth: OAuth2ClientAuthMethod::ClientSecretBasic,
//! };
//! ```
//!
//! and PKCE can no longer be switched off:
//!
//! ```compile_fail
//! let _ = schematic_define::PkceRequirement::NotUsed;
//! ```

use serde::{Deserialize, Deserializer, Serialize};
use thiserror::Error;

/// OAuth2 provider configuration for an API, one variant per grant.
///
/// Serialized internally tagged by `grant_type` with the variant name as the
/// tag value. Inapplicable fields are never serialized.
///
/// ## Examples
///
/// ```
/// use schematic_define::{
///     AuthorizationCodeGrant, ClientCredentialsGrant, DeviceCodeGrant, OAuth2ClientAuthMethod,
///     OAuth2Config, OAuth2GrantType, PkceRequirement,
/// };
///
/// let github = OAuth2Config::AuthorizationCodePkce(AuthorizationCodeGrant {
///     authorization_url: "https://github.com/login/oauth/authorize".into(),
///     token_url: "https://github.com/login/oauth/access_token".into(),
///     revocation_url: None,
///     default_scopes: vec!["repo".into(), "read:user".into()],
///     pkce: PkceRequirement::Required,
///     client_auth: OAuth2ClientAuthMethod::ClientSecretPost,
/// });
/// assert_eq!(github.grant_type(), OAuth2GrantType::AuthorizationCodePkce);
/// assert!(github.validate().is_ok());
///
/// let service = OAuth2Config::ClientCredentials(ClientCredentialsGrant {
///     token_url: "https://auth.example.com/token".into(),
///     revocation_url: None,
///     default_scopes: vec!["api".into()],
///     client_auth: OAuth2ClientAuthMethod::ClientSecretBasic,
/// });
/// assert_eq!(service.pkce(), None);
///
/// let device = OAuth2Config::DeviceCode(DeviceCodeGrant {
///     device_authorization_url: "https://auth.example.com/device".into(),
///     token_url: "https://auth.example.com/token".into(),
///     revocation_url: None,
///     default_scopes: vec![],
///     client_auth: OAuth2ClientAuthMethod::None,
/// });
/// assert_eq!(device.device_authorization_url(), Some("https://auth.example.com/device"));
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "grant_type")]
#[non_exhaustive]
pub enum OAuth2Config {
    /// Authorization Code with PKCE (recommended for most use cases).
    AuthorizationCodePkce(AuthorizationCodeGrant),
    /// Client Credentials (service-to-service).
    ClientCredentials(ClientCredentialsGrant),
    /// Device Code (for input-constrained devices).
    DeviceCode(DeviceCodeGrant),
}

/// Authorization-code grant payload; the only grant that uses PKCE.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct AuthorizationCodeGrant {
    /// Authorization endpoint URL the user is sent to.
    pub authorization_url: String,
    /// Token endpoint URL.
    pub token_url: String,
    /// Token revocation endpoint URL.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub revocation_url: Option<String>,
    /// Default OAuth2 scopes requested by this API.
    pub default_scopes: Vec<String>,
    /// PKCE requirement for this provider.
    pub pkce: PkceRequirement,
    /// How client credentials are sent to the token endpoint.
    pub client_auth: OAuth2ClientAuthMethod,
}

/// Client-credentials grant payload.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ClientCredentialsGrant {
    /// Token endpoint URL.
    pub token_url: String,
    /// Token revocation endpoint URL.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub revocation_url: Option<String>,
    /// Default OAuth2 scopes requested by this API.
    pub default_scopes: Vec<String>,
    /// How client credentials are sent to the token endpoint.
    pub client_auth: OAuth2ClientAuthMethod,
}

/// Device-code grant payload.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct DeviceCodeGrant {
    /// Device authorization endpoint URL.
    pub device_authorization_url: String,
    /// Token endpoint URL.
    pub token_url: String,
    /// Token revocation endpoint URL.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub revocation_url: Option<String>,
    /// Default OAuth2 scopes requested by this API.
    pub default_scopes: Vec<String>,
    /// How client credentials are sent to the token endpoint.
    pub client_auth: OAuth2ClientAuthMethod,
}

/// OAuth2 grant type; the `grant_type` tag of a serialized [`OAuth2Config`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub enum OAuth2GrantType {
    /// Authorization Code with PKCE (recommended for most use cases).
    AuthorizationCodePkce,
    /// Client Credentials (service-to-service).
    ClientCredentials,
    /// Device Code (for input-constrained devices).
    DeviceCode,
}

/// PKCE (Proof Key for Code Exchange) requirement of an authorization-code grant.
///
/// The `schematic-oauth` runtime sends a PKCE challenge in both modes; the
/// distinction records whether the provider would reject a request without one.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub enum PkceRequirement {
    /// PKCE is required by the provider.
    #[default]
    Required,
    /// PKCE is supported but not required.
    Supported,
}

/// How client credentials are sent to the OAuth2 token endpoint.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub enum OAuth2ClientAuthMethod {
    /// HTTP Basic authentication with client_id:client_secret.
    #[default]
    ClientSecretBasic,
    /// Client credentials in the POST body.
    ClientSecretPost,
    /// No client authentication (public clients).
    None,
}

/// Why an [`OAuth2Config`] is invalid.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
#[non_exhaustive]
pub enum OAuth2ConfigError {
    /// An endpoint is not an absolute `http`/`https` URL with a host.
    #[error("OAuth2 `{field}` must be an absolute http or https URL with a host, got {value:?}: {reason}")]
    InvalidUrl {
        /// Serialized name of the offending field.
        field: &'static str,
        /// The rejected value.
        value: String,
        /// What is wrong with it.
        reason: String,
    },
    /// The selected grant requires a field that is absent or null.
    #[error("OAuth2 grant `{grant_type:?}` requires `{field}`")]
    MissingField {
        /// The selected grant.
        grant_type: OAuth2GrantType,
        /// Serialized name of the missing field.
        field: &'static str,
    },
    /// A field that belongs to another grant carries a value.
    #[error("OAuth2 grant `{grant_type:?}` does not use `{field}`")]
    InapplicableField {
        /// The selected grant.
        grant_type: OAuth2GrantType,
        /// Serialized name of the inapplicable field.
        field: &'static str,
    },
}

impl OAuth2Config {
    /// The grant this configuration selects.
    pub fn grant_type(&self) -> OAuth2GrantType {
        match self {
            Self::AuthorizationCodePkce(_) => OAuth2GrantType::AuthorizationCodePkce,
            Self::ClientCredentials(_) => OAuth2GrantType::ClientCredentials,
            Self::DeviceCode(_) => OAuth2GrantType::DeviceCode,
        }
    }

    /// Token endpoint URL.
    pub fn token_url(&self) -> &str {
        match self {
            Self::AuthorizationCodePkce(grant) => &grant.token_url,
            Self::ClientCredentials(grant) => &grant.token_url,
            Self::DeviceCode(grant) => &grant.token_url,
        }
    }

    /// Token revocation endpoint URL, when the provider declares one.
    pub fn revocation_url(&self) -> Option<&str> {
        match self {
            Self::AuthorizationCodePkce(grant) => grant.revocation_url.as_deref(),
            Self::ClientCredentials(grant) => grant.revocation_url.as_deref(),
            Self::DeviceCode(grant) => grant.revocation_url.as_deref(),
        }
    }

    /// Default OAuth2 scopes requested by this API.
    pub fn default_scopes(&self) -> &[String] {
        match self {
            Self::AuthorizationCodePkce(grant) => &grant.default_scopes,
            Self::ClientCredentials(grant) => &grant.default_scopes,
            Self::DeviceCode(grant) => &grant.default_scopes,
        }
    }

    /// How client credentials are sent to the token endpoint.
    pub fn client_auth(&self) -> &OAuth2ClientAuthMethod {
        match self {
            Self::AuthorizationCodePkce(grant) => &grant.client_auth,
            Self::ClientCredentials(grant) => &grant.client_auth,
            Self::DeviceCode(grant) => &grant.client_auth,
        }
    }

    /// Authorization endpoint URL; `Some` exactly for the authorization-code grant.
    pub fn authorization_url(&self) -> Option<&str> {
        match self {
            Self::AuthorizationCodePkce(grant) => Some(&grant.authorization_url),
            Self::ClientCredentials(_) | Self::DeviceCode(_) => None,
        }
    }

    /// Device authorization endpoint URL; `Some` exactly for the device-code grant.
    pub fn device_authorization_url(&self) -> Option<&str> {
        match self {
            Self::DeviceCode(grant) => Some(&grant.device_authorization_url),
            Self::AuthorizationCodePkce(_) | Self::ClientCredentials(_) => None,
        }
    }

    /// PKCE requirement; `Some` exactly for the authorization-code grant.
    pub fn pkce(&self) -> Option<PkceRequirement> {
        match self {
            Self::AuthorizationCodePkce(grant) => Some(grant.pkce),
            Self::ClientCredentials(_) | Self::DeviceCode(_) => None,
        }
    }

    /// Checks every endpoint URL of the selected grant.
    ///
    /// Scopes are provider-defined strings and are not checked.
    ///
    /// ## Errors
    ///
    /// Returns [`OAuth2ConfigError::InvalidUrl`] for the first endpoint that is
    /// empty, unparsable, not `http`/`https`, or has no host.
    pub fn validate(&self) -> Result<(), OAuth2ConfigError> {
        if let Some(url) = self.authorization_url() {
            check_url("authorization_url", url)?;
        }
        if let Some(url) = self.device_authorization_url() {
            check_url("device_authorization_url", url)?;
        }
        check_url("token_url", self.token_url())?;
        if let Some(url) = self.revocation_url() {
            check_url("revocation_url", url)?;
        }
        Ok(())
    }
}

fn check_url(field: &'static str, value: &str) -> Result<(), OAuth2ConfigError> {
    let invalid = |reason: String| OAuth2ConfigError::InvalidUrl {
        field,
        value: value.to_string(),
        reason,
    };
    if value.is_empty() {
        return Err(invalid("empty".to_string()));
    }
    let parsed = url::Url::parse(value).map_err(|error| invalid(error.to_string()))?;
    if !matches!(parsed.scheme(), "http" | "https") {
        return Err(invalid(format!("unsupported scheme `{}`", parsed.scheme())));
    }
    if parsed.host_str().is_none_or(str::is_empty) {
        return Err(invalid("missing host".to_string()));
    }
    Ok(())
}

/// Wire mirror of [`OAuth2Config`] that keeps absent, null, and present apart.
///
/// Grant-owned fields are `Option<Option<T>>`: outer `None` is absent, inner
/// `None` is an explicit null.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawOAuth2Config {
    grant_type: OAuth2GrantType,
    #[serde(default, deserialize_with = "nullable")]
    authorization_url: Option<Option<String>>,
    token_url: String,
    #[serde(default, deserialize_with = "crate::serde_presence::non_null")]
    revocation_url: Option<String>,
    #[serde(default, deserialize_with = "nullable")]
    device_authorization_url: Option<Option<String>>,
    default_scopes: Vec<String>,
    #[serde(default, deserialize_with = "nullable")]
    pkce: Option<Option<PkceRequirement>>,
    client_auth: OAuth2ClientAuthMethod,
}

fn nullable<'de, D, T>(deserializer: D) -> Result<Option<Option<T>>, D::Error>
where
    D: Deserializer<'de>,
    T: Deserialize<'de>,
{
    Option::<T>::deserialize(deserializer).map(Some)
}

impl TryFrom<RawOAuth2Config> for OAuth2Config {
    type Error = OAuth2ConfigError;

    fn try_from(raw: RawOAuth2Config) -> Result<Self, Self::Error> {
        let grant_type = raw.grant_type;
        let required = |field: &'static str| OAuth2ConfigError::MissingField { grant_type, field };
        let inapplicable = |field: &'static str, present: bool| {
            if present {
                Err(OAuth2ConfigError::InapplicableField { grant_type, field })
            } else {
                Ok(())
            }
        };
        let has_authorization_url = matches!(raw.authorization_url, Some(Some(_)));
        let has_device_url = matches!(raw.device_authorization_url, Some(Some(_)));
        let has_pkce = matches!(raw.pkce, Some(Some(_)));

        let config = match grant_type {
            OAuth2GrantType::AuthorizationCodePkce => {
                inapplicable("device_authorization_url", has_device_url)?;
                Self::AuthorizationCodePkce(AuthorizationCodeGrant {
                    authorization_url: raw
                        .authorization_url
                        .flatten()
                        .ok_or_else(|| required("authorization_url"))?,
                    token_url: raw.token_url,
                    revocation_url: raw.revocation_url,
                    default_scopes: raw.default_scopes,
                    pkce: raw.pkce.flatten().ok_or_else(|| required("pkce"))?,
                    client_auth: raw.client_auth,
                })
            }
            OAuth2GrantType::ClientCredentials => {
                inapplicable("authorization_url", has_authorization_url)?;
                inapplicable("device_authorization_url", has_device_url)?;
                inapplicable("pkce", has_pkce)?;
                Self::ClientCredentials(ClientCredentialsGrant {
                    token_url: raw.token_url,
                    revocation_url: raw.revocation_url,
                    default_scopes: raw.default_scopes,
                    client_auth: raw.client_auth,
                })
            }
            OAuth2GrantType::DeviceCode => {
                inapplicable("authorization_url", has_authorization_url)?;
                inapplicable("pkce", has_pkce)?;
                Self::DeviceCode(DeviceCodeGrant {
                    device_authorization_url: raw
                        .device_authorization_url
                        .flatten()
                        .ok_or_else(|| required("device_authorization_url"))?,
                    token_url: raw.token_url,
                    revocation_url: raw.revocation_url,
                    default_scopes: raw.default_scopes,
                    client_auth: raw.client_auth,
                })
            }
        };
        config.validate()?;
        Ok(config)
    }
}

impl<'de> Deserialize<'de> for OAuth2Config {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let raw = RawOAuth2Config::deserialize(deserializer)?;
        Self::try_from(raw).map_err(serde::de::Error::custom)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::auth::AuthStrategy;
    use serde_json::{json, Value};

    fn authorization_code() -> OAuth2Config {
        OAuth2Config::AuthorizationCodePkce(AuthorizationCodeGrant {
            authorization_url: "https://example.com/authorize".into(),
            token_url: "https://example.com/token".into(),
            revocation_url: Some("https://example.com/revoke".into()),
            default_scopes: vec!["read".into(), "write".into()],
            pkce: PkceRequirement::Required,
            client_auth: OAuth2ClientAuthMethod::ClientSecretPost,
        })
    }

    fn client_credentials() -> OAuth2Config {
        OAuth2Config::ClientCredentials(ClientCredentialsGrant {
            token_url: "https://auth.example.com/token".into(),
            revocation_url: None,
            default_scopes: vec!["api".into()],
            client_auth: OAuth2ClientAuthMethod::ClientSecretBasic,
        })
    }

    fn device_code() -> OAuth2Config {
        OAuth2Config::DeviceCode(DeviceCodeGrant {
            device_authorization_url: "https://auth.example.com/device".into(),
            token_url: "https://auth.example.com/token".into(),
            revocation_url: None,
            default_scopes: vec![],
            client_auth: OAuth2ClientAuthMethod::None,
        })
    }

    /// A valid serialized object for `grant`, to be mutated by matrix rows.
    fn base(grant: &str) -> Value {
        match grant {
            "AuthorizationCodePkce" => json!({
                "grant_type": grant,
                "authorization_url": "https://example.com/authorize",
                "token_url": "https://example.com/token",
                "default_scopes": ["read"],
                "pkce": "Required",
                "client_auth": "ClientSecretBasic",
            }),
            "ClientCredentials" => json!({
                "grant_type": grant,
                "token_url": "https://example.com/token",
                "default_scopes": ["read"],
                "client_auth": "ClientSecretBasic",
            }),
            "DeviceCode" => json!({
                "grant_type": grant,
                "device_authorization_url": "https://example.com/device",
                "token_url": "https://example.com/token",
                "default_scopes": ["read"],
                "client_auth": "ClientSecretBasic",
            }),
            other => panic!("unknown grant {other}"),
        }
    }

    fn read(value: &Value) -> Result<OAuth2Config, serde_json::Error> {
        serde_json::from_value(value.clone())
    }

    fn with(grant: &str, field: &str, value: Option<Value>) -> Value {
        let mut object = base(grant);
        let members = object.as_object_mut().unwrap();
        match value {
            Some(value) => members.insert(field.to_string(), value),
            None => members.remove(field),
        };
        object
    }

    const GRANTS: [&str; 3] = ["AuthorizationCodePkce", "ClientCredentials", "DeviceCode"];

    #[test]
    fn every_grant_constructs_validates_and_round_trips_byte_stable() {
        for config in [authorization_code(), client_credentials(), device_code()] {
            config.validate().unwrap();
            let first = serde_json::to_string(&config).unwrap();
            let parsed: OAuth2Config = serde_json::from_str(&first).unwrap();
            assert_eq!(parsed, config);
            let second = serde_json::to_string(&parsed).unwrap();
            assert_eq!(first, second);
            let third = serde_json::to_string(&serde_json::from_str::<OAuth2Config>(&second).unwrap()).unwrap();
            assert_eq!(second, third);
        }
    }

    #[test]
    fn serialization_pins_tag_spelling_and_emits_only_applicable_fields() {
        assert_eq!(
            serde_json::to_string(&authorization_code()).unwrap(),
            r#"{"grant_type":"AuthorizationCodePkce","authorization_url":"https://example.com/authorize","token_url":"https://example.com/token","revocation_url":"https://example.com/revoke","default_scopes":["read","write"],"pkce":"Required","client_auth":"ClientSecretPost"}"#
        );
        assert_eq!(
            serde_json::to_string(&client_credentials()).unwrap(),
            r#"{"grant_type":"ClientCredentials","token_url":"https://auth.example.com/token","default_scopes":["api"],"client_auth":"ClientSecretBasic"}"#
        );
        assert_eq!(
            serde_json::to_string(&device_code()).unwrap(),
            r#"{"grant_type":"DeviceCode","device_authorization_url":"https://auth.example.com/device","token_url":"https://auth.example.com/token","default_scopes":[],"client_auth":"None"}"#
        );
        for (grant, spelling) in [
            (OAuth2GrantType::AuthorizationCodePkce, "\"AuthorizationCodePkce\""),
            (OAuth2GrantType::ClientCredentials, "\"ClientCredentials\""),
            (OAuth2GrantType::DeviceCode, "\"DeviceCode\""),
        ] {
            assert_eq!(serde_json::to_string(&grant).unwrap(), spelling);
        }
    }

    #[test]
    fn grant_type_absent_null_or_unknown_is_rejected() {
        for grant in GRANTS {
            for value in [None, Some(Value::Null), Some(json!("Implicit")), Some(json!("authorization_code_pkce"))] {
                let object = with(grant, "grant_type", value.clone());
                assert!(read(&object).is_err(), "{grant} grant_type {value:?} accepted");
            }
        }
    }

    #[test]
    fn shared_required_fields_absent_or_null_are_rejected() {
        for grant in GRANTS {
            for field in ["token_url", "default_scopes", "client_auth"] {
                assert!(read(&with(grant, field, None)).is_err(), "{grant} without {field} accepted");
                assert!(read(&with(grant, field, Some(Value::Null))).is_err(), "{grant} null {field} accepted");
            }
        }
    }

    #[test]
    fn unknown_member_is_rejected() {
        for grant in GRANTS {
            assert!(read(&with(grant, "refresh_url", Some(json!("https://example.com/r")))).is_err());
        }
    }

    #[test]
    fn revocation_url_absent_is_none_and_null_or_empty_is_rejected() {
        for grant in GRANTS {
            assert_eq!(read(&base(grant)).unwrap().revocation_url(), None);
            let present = read(&with(grant, "revocation_url", Some(json!("https://example.com/revoke")))).unwrap();
            assert_eq!(present.revocation_url(), Some("https://example.com/revoke"));
            assert!(read(&with(grant, "revocation_url", Some(Value::Null))).is_err());
            assert!(read(&with(grant, "revocation_url", Some(json!("")))).is_err());
        }
    }

    #[test]
    fn grant_required_fields_absent_or_null_are_rejected() {
        for (grant, field) in [
            ("AuthorizationCodePkce", "authorization_url"),
            ("AuthorizationCodePkce", "pkce"),
            ("DeviceCode", "device_authorization_url"),
        ] {
            let absent = read(&with(grant, field, None)).unwrap_err().to_string();
            assert!(absent.contains(field), "{absent}");
            assert!(read(&with(grant, field, Some(Value::Null))).is_err(), "{grant} null {field} accepted");
        }
    }

    #[test]
    fn inapplicable_fields_accept_absent_or_null_and_reject_any_value() {
        for (grant, field, value) in [
            ("AuthorizationCodePkce", "device_authorization_url", json!("https://example.com/device")),
            ("ClientCredentials", "authorization_url", json!("https://example.com/authorize")),
            ("ClientCredentials", "device_authorization_url", json!("https://example.com/device")),
            ("ClientCredentials", "pkce", json!("Required")),
            ("DeviceCode", "authorization_url", json!("https://example.com/authorize")),
            ("DeviceCode", "pkce", json!("Supported")),
        ] {
            let expected = read(&base(grant)).unwrap();
            assert_eq!(read(&with(grant, field, Some(Value::Null))).unwrap(), expected);
            let error = read(&with(grant, field, Some(value.clone()))).unwrap_err().to_string();
            assert!(error.contains(field), "{grant} {field}={value}: {error}");
            assert!(read(&with(grant, field, Some(json!("")))).is_err(), "{grant} empty {field} accepted");
        }
    }

    #[test]
    fn default_scopes_accept_empty_and_reject_non_strings() {
        for grant in GRANTS {
            let empty = read(&with(grant, "default_scopes", Some(json!([])))).unwrap();
            assert!(empty.default_scopes().is_empty());
            assert!(read(&with(grant, "default_scopes", Some(json!(["read", 1])))).is_err());
            assert!(read(&with(grant, "default_scopes", Some(json!("read")))).is_err());
        }
    }

    #[test]
    fn pkce_accepts_required_and_supported_only() {
        for (spelling, expected) in [("Required", PkceRequirement::Required), ("Supported", PkceRequirement::Supported)] {
            let config = read(&with("AuthorizationCodePkce", "pkce", Some(json!(spelling)))).unwrap();
            assert_eq!(config.pkce(), Some(expected));
            let bytes = serde_json::to_string(&config).unwrap();
            assert!(bytes.contains(&format!(r#""pkce":"{spelling}""#)), "{bytes}");
        }
        assert!(read(&with("AuthorizationCodePkce", "pkce", Some(json!("NotUsed")))).is_err());
        assert_eq!(PkceRequirement::default(), PkceRequirement::Required);
    }

    #[test]
    fn malformed_urls_are_rejected_by_validate_and_deserialization() {
        let url_fields = [
            ("AuthorizationCodePkce", "authorization_url"),
            ("AuthorizationCodePkce", "token_url"),
            ("ClientCredentials", "token_url"),
            ("ClientCredentials", "revocation_url"),
            ("DeviceCode", "device_authorization_url"),
            ("DeviceCode", "token_url"),
        ];
        for bad in ["", "not a url", "ftp://example.com/token", "https://", "mailto:someone@example.com", "/token"] {
            for (grant, field) in url_fields {
                let error = read(&with(grant, field, Some(json!(bad)))).unwrap_err().to_string();
                assert!(error.contains(field), "{grant} {field}={bad:?}: {error}");
            }
            let mut config = authorization_code();
            if let OAuth2Config::AuthorizationCodePkce(grant) = &mut config {
                grant.token_url = bad.to_string();
            }
            assert!(
                matches!(config.validate(), Err(OAuth2ConfigError::InvalidUrl { field: "token_url", .. })),
                "{bad:?}"
            );
        }
        let mut config = device_code();
        if let OAuth2Config::DeviceCode(grant) = &mut config {
            grant.device_authorization_url = "ftp://example.com/device".into();
        }
        assert!(matches!(
            config.validate(),
            Err(OAuth2ConfigError::InvalidUrl { field: "device_authorization_url", .. })
        ));
        assert!(read(&with("ClientCredentials", "token_url", Some(json!("http://localhost:8080/token")))).is_ok());
    }

    #[test]
    fn accessors_report_each_grant() {
        let code = authorization_code();
        assert_eq!(code.grant_type(), OAuth2GrantType::AuthorizationCodePkce);
        assert_eq!(code.authorization_url(), Some("https://example.com/authorize"));
        assert_eq!(code.device_authorization_url(), None);
        assert_eq!(code.token_url(), "https://example.com/token");
        assert_eq!(code.revocation_url(), Some("https://example.com/revoke"));
        assert_eq!(code.default_scopes(), ["read".to_string(), "write".to_string()]);
        assert_eq!(code.pkce(), Some(PkceRequirement::Required));
        assert_eq!(code.client_auth(), &OAuth2ClientAuthMethod::ClientSecretPost);

        let service = client_credentials();
        assert_eq!(service.grant_type(), OAuth2GrantType::ClientCredentials);
        assert_eq!(service.authorization_url(), None);
        assert_eq!(service.device_authorization_url(), None);
        assert_eq!(service.token_url(), "https://auth.example.com/token");
        assert_eq!(service.revocation_url(), None);
        assert_eq!(service.default_scopes(), ["api".to_string()]);
        assert_eq!(service.pkce(), None);
        assert_eq!(service.client_auth(), &OAuth2ClientAuthMethod::ClientSecretBasic);

        let device = device_code();
        assert_eq!(device.grant_type(), OAuth2GrantType::DeviceCode);
        assert_eq!(device.authorization_url(), None);
        assert_eq!(device.device_authorization_url(), Some("https://auth.example.com/device"));
        assert_eq!(device.token_url(), "https://auth.example.com/token");
        assert!(device.default_scopes().is_empty());
        assert_eq!(device.pkce(), None);
        assert_eq!(device.client_auth(), &OAuth2ClientAuthMethod::None);
    }

    #[test]
    fn client_auth_method_default_is_basic() {
        assert_eq!(
            OAuth2ClientAuthMethod::default(),
            OAuth2ClientAuthMethod::ClientSecretBasic
        );
    }

    #[test]
    fn client_auth_method_serde_roundtrip() {
        for variant in [
            OAuth2ClientAuthMethod::ClientSecretBasic,
            OAuth2ClientAuthMethod::ClientSecretPost,
            OAuth2ClientAuthMethod::None,
        ] {
            let json = serde_json::to_string(&variant).unwrap();
            let parsed: OAuth2ClientAuthMethod = serde_json::from_str(&json).unwrap();
            assert_eq!(variant, parsed);
        }
    }

    #[test]
    fn auth_strategy_oauth2_serde_roundtrip() {
        for config in [authorization_code(), client_credentials(), device_code()] {
            let strategy = AuthStrategy::OAuth2(config);
            let json = serde_json::to_string(&strategy).unwrap();
            let parsed: AuthStrategy = serde_json::from_str(&json).unwrap();
            assert_eq!(strategy, parsed);
        }
    }

    #[test]
    fn env_mapping_with_oauth_fields_serde_roundtrip() {
        use crate::headers::{EnvList, EnvMapping};

        let mapping = EnvMapping {
            bearer_token: None,
            basic_user: None,
            basic_pass: None,
            api_key: None,
            oauth_client_id: Some(EnvList::single("GITHUB_CLIENT_ID")),
            oauth_client_secret: Some(EnvList::single("GITHUB_CLIENT_SECRET")),
            oauth_redirect_uri: Some(EnvList::single("GITHUB_REDIRECT_URI")),
        };

        let json = serde_json::to_string(&mapping).unwrap();
        let parsed: EnvMapping = serde_json::from_str(&json).unwrap();
        assert_eq!(mapping, parsed);
    }

    #[test]
    fn env_mapping_backward_compat_without_oauth_fields() {
        use crate::headers::EnvMapping;

        // JSON without OAuth fields should deserialize correctly
        let json = r#"{"bearer_token":{"names":["API_KEY"]}}"#;
        let parsed: EnvMapping = serde_json::from_str(json).unwrap();
        assert!(parsed.bearer_token.is_some());
        assert!(parsed.oauth_client_id.is_none());
        assert!(parsed.oauth_client_secret.is_none());
        assert!(parsed.oauth_redirect_uri.is_none());
    }
}
