//! Programmatic credentials and how an API accepts them.
//!
//! A credential is supplied per call or at client level; when neither is
//! set, the client reads its configured environment variables, and when
//! nothing resolves the request is sent anonymously. There is no
//! missing-credential error and no retry with another credential. A
//! credential's `Debug` and `Display` output never shows its value.
//!
//! [`AuthConfig`] is the runtime's form of a definition's effective
//! authentication policy; it is separate from the definition types in
//! `schematic-define`, which this crate does not depend on.

use std::fmt;

use crate::error::InvalidArgument;
use crate::meta::AuthMethod;
use crate::options::CallAuth;

/// A credential supplied by the caller.
#[derive(Clone, PartialEq, Eq)]
pub struct Credential(Secret);

#[derive(Clone, PartialEq, Eq)]
enum Secret {
    AccessToken(String),
    ApiKey(String),
    Basic { username: String, password: String },
}

/// The authentication method a [`Credential`] is for.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CredentialKind {
    /// A bearer or OAuth access token.
    AccessToken,
    /// An API key, sent in a header, query parameter, or cookie.
    ApiKey,
    /// HTTP Basic username and password.
    Basic,
}

impl Credential {
    /// A bearer or OAuth access token.
    pub fn access_token(token: impl Into<String>) -> Self {
        Self(Secret::AccessToken(token.into()))
    }

    /// An API key.
    pub fn api_key(key: impl Into<String>) -> Self {
        Self(Secret::ApiKey(key.into()))
    }

    /// HTTP Basic credentials; both fields are always supplied together.
    pub fn basic(username: impl Into<String>, password: impl Into<String>) -> Self {
        Self(Secret::Basic {
            username: username.into(),
            password: password.into(),
        })
    }

    /// The method this credential is for.
    pub fn kind(&self) -> CredentialKind {
        match self.0 {
            Secret::AccessToken(_) => CredentialKind::AccessToken,
            Secret::ApiKey(_) => CredentialKind::ApiKey,
            Secret::Basic { .. } => CredentialKind::Basic,
        }
    }
}

impl fmt::Debug for Credential {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Credential({:?}, {})", self.kind(), crate::redact::REDACTED)
    }
}

impl fmt::Display for Credential {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} {}", self.kind(), crate::redact::REDACTED)
    }
}

impl CredentialKind {
    pub(crate) fn name(self) -> &'static str {
        match self {
            Self::AccessToken => "access token",
            Self::ApiKey => "API key",
            Self::Basic => "Basic",
        }
    }
}

impl fmt::Display for CredentialKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.name())
    }
}

impl Credential {
    /// The secret as it is placed on the wire by `placement`, which must
    /// accept this credential's kind.
    fn wire_value(&self, placement: &CredentialPlacement) -> String {
        match (&self.0, placement) {
            (Secret::Basic { username, password }, _) => basic_value(username, password),
            (Secret::AccessToken(token) | Secret::ApiKey(token), CredentialPlacement::Bearer { .. }) => {
                format!("Bearer {token}")
            }
            (Secret::AccessToken(token) | Secret::ApiKey(token), CredentialPlacement::ApiKeyHeader { prefix, .. }) => {
                format!("{}{token}", prefix.as_deref().unwrap_or_default())
            }
            (Secret::AccessToken(token) | Secret::ApiKey(token), _) => token.clone(),
        }
    }
}

fn basic_value(username: &str, password: &str) -> String {
    use base64::Engine;
    format!(
        "Basic {}",
        base64::engine::general_purpose::STANDARD.encode(format!("{username}:{password}"))
    )
}

/// Where and how an API sends a credential.
///
/// This is the runtime's translation of a definition's authentication
/// methods; generated code builds it from the definition's effective policy.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum CredentialPlacement {
    /// An access token sent as `Bearer <token>` in `Authorization`, or in
    /// `header` when the API names another header.
    Bearer {
        /// A header other than `Authorization`.
        header: Option<String>,
    },
    /// An API key sent in `header`, after `prefix` when one is set.
    ApiKeyHeader {
        /// Header name.
        header: String,
        /// Text placed before the key, such as `token `.
        prefix: Option<String>,
    },
    /// An API key sent as the query parameter `name`.
    ApiKeyQuery {
        /// Query parameter name.
        name: String,
    },
    /// An API key sent as the cookie `name`.
    ApiKeyCookie {
        /// Cookie name.
        name: String,
    },
    /// HTTP Basic username and password in `Authorization`.
    Basic,
}

impl CredentialPlacement {
    /// The kind of [`Credential`] this placement sends.
    pub fn kind(&self) -> CredentialKind {
        match self {
            Self::Bearer { .. } => CredentialKind::AccessToken,
            Self::ApiKeyHeader { .. } | Self::ApiKeyQuery { .. } | Self::ApiKeyCookie { .. } => CredentialKind::ApiKey,
            Self::Basic => CredentialKind::Basic,
        }
    }

    /// The header this placement writes, if it writes one.
    pub(crate) fn header_name(&self) -> Option<&str> {
        match self {
            Self::Bearer { header } => Some(header.as_deref().unwrap_or("Authorization")),
            Self::ApiKeyHeader { header, .. } => Some(header),
            Self::Basic => Some("Authorization"),
            Self::ApiKeyQuery { .. } | Self::ApiKeyCookie { .. } => None,
        }
    }

    /// The name this placement makes secret in stored metadata.
    pub(crate) fn secret_name(&self) -> &str {
        match self {
            Self::ApiKeyQuery { name } | Self::ApiKeyCookie { name } => name,
            _ => self.header_name().unwrap_or("Authorization"),
        }
    }
}

/// The environment fallback of an API: where to read a credential when the
/// caller supplied none.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EnvFallback {
    placement: CredentialPlacement,
    vars: Vec<String>,
    username_vars: Vec<String>,
}

impl EnvFallback {
    /// Reads the first non-empty variable of `vars` and sends it with
    /// `placement`. For [`CredentialPlacement::Basic`], `vars` hold the
    /// password; set the username variables with
    /// [`username_vars`](Self::username_vars).
    pub fn new<I, S>(placement: CredentialPlacement, vars: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        Self {
            placement,
            vars: vars.into_iter().map(Into::into).collect(),
            username_vars: Vec::new(),
        }
    }

    /// The variables holding the Basic username, read in order.
    #[must_use]
    pub fn username_vars<I, S>(mut self, vars: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.username_vars = vars.into_iter().map(Into::into).collect();
        self
    }
}

/// How an API accepts credentials: the placements a caller-supplied
/// credential may use, and an optional environment fallback.
///
/// A caller-supplied credential uses the first accepted placement of its
/// kind. The fallback is read only when the caller supplied none.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct AuthConfig {
    accepted: Vec<CredentialPlacement>,
    env: Option<EnvFallback>,
}

impl AuthConfig {
    /// An API that accepts no credential.
    pub fn none() -> Self {
        Self::default()
    }

    /// Accepts caller-supplied credentials sent with `placement`.
    #[must_use]
    pub fn accept(mut self, placement: CredentialPlacement) -> Self {
        self.accepted.push(placement);
        self
    }

    /// Sets the environment fallback.
    #[must_use]
    pub fn env_fallback(mut self, fallback: EnvFallback) -> Self {
        self.env = Some(fallback);
        self
    }

    /// The placement for a caller-supplied credential of `kind`, if any.
    pub(crate) fn placement_for(&self, kind: CredentialKind) -> Option<&CredentialPlacement> {
        self.accepted.iter().find(|placement| placement.kind() == kind)
    }

    /// Replaces the order of the fallback's (password, key, or token)
    /// variables; does nothing when there is no fallback.
    pub(crate) fn replace_env_order(&mut self, vars: Vec<String>) {
        if let Some(env) = &mut self.env {
            env.vars = vars;
        }
    }

    /// Every placement this API can send a credential with.
    pub(crate) fn placements(&self) -> impl Iterator<Item = &CredentialPlacement> {
        self.accepted.iter().chain(self.env.iter().map(|env| &env.placement))
    }

    /// Resolves the credential for one send, reading the environment at most
    /// once.
    pub(crate) fn resolve(&self, call: &CallAuth, client: Option<&Credential>) -> Result<Resolved, InvalidArgument> {
        match call {
            CallAuth::Anonymous => Ok(Resolved::anonymous()),
            CallAuth::PerCall(credential) => {
                let placement = self.placement_for(credential.kind()).ok_or_else(|| {
                    InvalidArgument::new(format!("the API does not accept a per-call {} credential", credential.kind()))
                })?;
                Ok(Resolved::sent(placement, credential.wire_value(placement), AuthMethod::PerCall))
            }
            CallAuth::Inherit => {
                if let Some(credential) = client {
                    // Validated against the accepted placements when the client was built.
                    if let Some(placement) = self.placement_for(credential.kind()) {
                        return Ok(Resolved::sent(placement, credential.wire_value(placement), AuthMethod::ClientLevel));
                    }
                }
                Ok(self.resolve_env().unwrap_or_else(Resolved::anonymous))
            }
        }
    }

    fn resolve_env(&self) -> Option<Resolved> {
        let env = self.env.as_ref()?;
        let (password_var, password) = first_non_empty(&env.vars)?;
        let value = match &env.placement {
            CredentialPlacement::Basic => {
                let (_, username) = first_non_empty(&env.username_vars)?;
                basic_value(&username, &password)
            }
            placement => Credential::api_key(password).wire_value(placement),
        };
        Some(Resolved::sent(&env.placement, value, AuthMethod::EnvVar(password_var)))
    }
}

fn first_non_empty(vars: &[String]) -> Option<(String, String)> {
    vars.iter()
        .find_map(|name| std::env::var(name).ok().filter(|value| !value.is_empty()).map(|value| (name.clone(), value)))
}

/// The credential chosen for one send.
pub(crate) struct Resolved {
    /// The placement and the value to send with it; `None` when anonymous.
    pub(crate) sent: Option<(CredentialPlacement, String)>,
    /// Where the credential came from.
    pub(crate) auth: AuthMethod,
}

impl Resolved {
    fn anonymous() -> Self {
        Self {
            sent: None,
            auth: AuthMethod::Anonymous,
        }
    }

    fn sent(placement: &CredentialPlacement, value: String, auth: AuthMethod) -> Self {
        Self {
            sent: Some((placement.clone(), value)),
            auth,
        }
    }
}

impl fmt::Debug for Resolved {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Resolved")
            .field("placement", &self.sent.as_ref().map(|(placement, _)| placement))
            .field("auth", &self.auth)
            .finish()
    }
}
